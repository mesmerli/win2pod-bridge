//! Windows System Media Transport Controls (GSMTC) integration.
//! Tracks current playing media (Title, Artist, Album, Thumbnail) and sends playback control commands.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::DataReader;

#[derive(Debug, Clone, Default)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source_app_id: String,
    pub is_playing: bool,
    pub has_session: bool,
    pub thumbnail_bytes: Option<Vec<u8>>,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub last_updated: Option<Instant>,
    pub last_playing_instant: Option<Instant>,
}

impl MediaTrackInfo {
    pub fn current_position_secs(&self) -> f64 {
        if self.duration_secs <= 0.0 {
            return 0.0;
        }
        if self.is_playing {
            if let Some(last) = self.last_updated {
                let elapsed = last.elapsed().as_secs_f64();
                return (self.position_secs + elapsed).min(self.duration_secs);
            }
        }
        self.position_secs.min(self.duration_secs)
    }

    pub fn format_time(secs: f64) -> String {
        let total_secs = secs.round() as u64;
        let m = total_secs / 60;
        let s = total_secs % 60;
        format!("{:02}:{:02}", m, s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaControlCmd {
    Play,
    Pause,
    TogglePlayPause,
    Previous,
    Next,
    Stop,
    Seek(f64),
}

static CURRENT_MEDIA_TRACK: std::sync::OnceLock<Arc<RwLock<MediaTrackInfo>>> = std::sync::OnceLock::new();
static CMD_TX: std::sync::OnceLock<tokio::sync::mpsc::UnboundedSender<MediaControlCmd>> = std::sync::OnceLock::new();

pub fn get_current_media_track() -> Arc<RwLock<MediaTrackInfo>> {
    CURRENT_MEDIA_TRACK
        .get_or_init(|| Arc::new(RwLock::new(MediaTrackInfo::default())))
        .clone()
}

/// Returns `(is_currently_playing, was_recently_playing_within_window)`.
pub fn is_playing_or_recently_playing(recent_window: Duration) -> (bool, bool) {
    let track = get_current_media_track();
    let guard = track.read().ok();
    let playing = guard.as_ref().map(|t| t.is_playing).unwrap_or(false);
    let recently_playing = guard
        .as_ref()
        .and_then(|t| t.last_playing_instant)
        .map_or(false, |t| t.elapsed() <= recent_window);
    (playing, recently_playing)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackEvent {
    PropertiesChanged,
    PlaybackChanged,
    TimelineChanged,
    SessionChanged,
}

fn attach_session_listeners(
    session: &GlobalSystemMediaTransportControlsSession,
    notify_tx: &tokio::sync::mpsc::UnboundedSender<TrackEvent>,
) {
    let tx1 = notify_tx.clone();
    let handler1 = TypedEventHandler::new(move |_sender, _args| {
        let _ = tx1.send(TrackEvent::PropertiesChanged);
        Ok(())
    });
    let _ = session.MediaPropertiesChanged(&handler1);

    let tx2 = notify_tx.clone();
    let handler2 = TypedEventHandler::new(move |_sender, _args| {
        let _ = tx2.send(TrackEvent::PlaybackChanged);
        Ok(())
    });
    let _ = session.PlaybackInfoChanged(&handler2);

    let tx3 = notify_tx.clone();
    let handler3 = TypedEventHandler::new(move |_sender, _args| {
        let _ = tx3.send(TrackEvent::TimelineChanged);
        Ok(())
    });
    let _ = session.TimelinePropertiesChanged(&handler3);
}

/// Spawns GSMTC tracking task driven purely by Windows WinRT event callbacks
pub fn spawn_media_tracker() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = CMD_TX.set(tx);

    let state = get_current_media_track();

    std::thread::Builder::new()
        .name("win2pod-media-tracker".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to create media tracker runtime: {e}");
                    return;
                }
            };

            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, async move {
                let (notify_tx, mut notify_rx) = tokio::sync::mpsc::unbounded_channel();
                let mut last_app_user_id = String::new();

                // Hook manager-level session switch events
                let mut session_manager: Option<GlobalSystemMediaTransportControlsSessionManager> = None;
                if let Ok(manager_op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                    if let Ok(mgr) = manager_op.await {
                        let tx_mgr = notify_tx.clone();
                        let session_changed_handler = TypedEventHandler::new(
                            move |_sender, _args| {
                                let _ = tx_mgr.send(TrackEvent::SessionChanged);
                                Ok(())
                            },
                        );
                        let _ = mgr.CurrentSessionChanged(&session_changed_handler);

                        let tx_sessions = notify_tx.clone();
                        let sessions_changed_handler = TypedEventHandler::new(
                            move |_sender, _args| {
                                let _ = tx_sessions.send(TrackEvent::SessionChanged);
                                Ok(())
                            },
                        );
                        let _ = mgr.SessionsChanged(&sessions_changed_handler);

                        if let Ok(session) = mgr.GetCurrentSession() {
                            last_app_user_id = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
                            attach_session_listeners(&session, &notify_tx);
                        }
                        session_manager = Some(mgr);
                    }
                }

                // Initial fetch
                if let Some(info) = fetch_media_properties().await {
                    if let Ok(mut lock) = state.write() {
                        *lock = info;
                    }
                }

                loop {
                    tokio::select! {
                        Some(cmd) = rx.recv() => {
                            execute_media_command(cmd, &state).await;
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            let session_opt = session_manager.as_ref().and_then(|mgr| mgr.GetCurrentSession().ok());
                            if let Some(ref session) = session_opt {
                                let prev_track = state.read().ok().map(|g| g.clone());
                                if let Some(info) = fetch_media_properties_from_session(session, prev_track.as_ref()).await {
                                    let is_playing = info.is_playing;
                                    if let Ok(mut lock) = state.write() {
                                        let prev_playing_instant = lock.last_playing_instant;
                                        *lock = info;
                                        if lock.is_playing {
                                            lock.last_playing_instant = Some(Instant::now());
                                        } else {
                                            lock.last_playing_instant = prev_playing_instant;
                                        }
                                    }
                                    crate::handoff::on_playback_status_changed(is_playing);
                                }
                            }
                        }
                        Some(first_ev) = notify_rx.recv() => {
                            // Accumulate burst event notifications with a short debounce window
                            let mut has_session_changed = first_ev == TrackEvent::SessionChanged;
                            let mut has_properties_changed = first_ev == TrackEvent::PropertiesChanged;
                            let mut has_playback_changed = first_ev == TrackEvent::PlaybackChanged;
                            let mut has_timeline_changed = first_ev == TrackEvent::TimelineChanged;

                            tokio::time::sleep(Duration::from_millis(40)).await;
                            while let Ok(ev) = notify_rx.try_recv() {
                                match ev {
                                    TrackEvent::SessionChanged => has_session_changed = true,
                                    TrackEvent::PropertiesChanged => has_properties_changed = true,
                                    TrackEvent::PlaybackChanged => has_playback_changed = true,
                                    TrackEvent::TimelineChanged => has_timeline_changed = true,
                                }
                            }

                            // If manager was not initialized at startup, retry requesting manager
                            if session_manager.is_none() {
                                if let Ok(manager_op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                                    if let Ok(mgr) = manager_op.await {
                                        let tx_mgr = notify_tx.clone();
                                        let session_changed_handler = TypedEventHandler::new(
                                            move |_sender, _args| {
                                                let _ = tx_mgr.send(TrackEvent::SessionChanged);
                                                Ok(())
                                            },
                                        );
                                        let _ = mgr.CurrentSessionChanged(&session_changed_handler);
                                        session_manager = Some(mgr);
                                    }
                                }
                            }

                            let session_opt = session_manager.as_ref().and_then(|mgr| mgr.GetCurrentSession().ok());

                            if let Some(ref session) = session_opt {
                                let cur_app = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
                                if has_session_changed && (last_app_user_id != cur_app || last_app_user_id.is_empty()) {
                                    last_app_user_id = cur_app;
                                    attach_session_listeners(session, &notify_tx);
                                    has_properties_changed = true;
                                }

                                if has_properties_changed {
                                    let prev_track = state.read().ok().map(|g| g.clone());
                                    if let Some(info) = fetch_media_properties_from_session(session, prev_track.as_ref()).await {
                                        let is_playing = info.is_playing;
                                        if let Ok(mut lock) = state.write() {
                                            let prev_playing_instant = lock.last_playing_instant;
                                            *lock = info;
                                            if lock.is_playing {
                                                lock.last_playing_instant = Some(Instant::now());
                                            } else {
                                                lock.last_playing_instant = prev_playing_instant;
                                            }
                                        }
                                        crate::handoff::on_playback_status_changed(is_playing);
                                    }
                                } else if has_playback_changed {
                                    // Lightweight PlaybackInfo update - ZERO cross-process COM metadata or thumbnail overhead
                                    let is_playing = session
                                        .GetPlaybackInfo()
                                        .ok()
                                        .and_then(|info| info.PlaybackStatus().ok())
                                        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                                        .unwrap_or(false);

                                    let (pos, dur) = compute_timeline_position(session, is_playing);
                                    if let Ok(mut lock) = state.write() {
                                        lock.is_playing = is_playing;
                                        lock.position_secs = pos;
                                        if dur > 0.0 {
                                            lock.duration_secs = dur;
                                        }
                                        lock.last_updated = Some(Instant::now());
                                        if is_playing {
                                            lock.last_playing_instant = Some(Instant::now());
                                        }
                                    }
                                    crate::handoff::on_playback_status_changed(is_playing);
                                } else if has_timeline_changed {
                                    // Smart Timeline Filtering:
                                    // KKBOX (and iTunes) tick TimelinePropertiesChanged multiple times per second.
                                    // Compare reported position with our smooth local extrapolation:
                                    // If difference is small (<= 1.5s), it is normal playback ticking -> IGNORE to save CPU!
                                    // Only update if difference > 1.5s, indicating genuine user seek.
                                    let is_playing = state.read().ok().map(|g| g.is_playing).unwrap_or(false);
                                    let (real_pos, dur) = compute_timeline_position(session, is_playing);
                                    let current_calc_pos = state.read().ok().map(|g| g.current_position_secs()).unwrap_or(0.0);

                                    if (real_pos - current_calc_pos).abs() > 1.5 {
                                        if let Ok(mut lock) = state.write() {
                                            lock.position_secs = real_pos;
                                            if dur > 0.0 {
                                                lock.duration_secs = dur;
                                            }
                                            lock.last_updated = Some(Instant::now());
                                        }
                                    }
                                }
                            } else {
                                // No active session
                                last_app_user_id.clear();
                                if let Ok(mut lock) = state.write() {
                                    if lock.has_session {
                                        lock.has_session = false;
                                        lock.is_playing = false;
                                    }
                                }
                                crate::handoff::on_playback_status_changed(false);
                            }
                        }
                    }
                }
            });
        })
        .expect("Failed to spawn media tracker thread");
}

fn compute_timeline_position(
    session: &GlobalSystemMediaTransportControlsSession,
    is_playing: bool,
) -> (f64, f64) {
    if let Ok(timeline) = session.GetTimelineProperties() {
        let raw_pos = timeline.Position().ok().map(|t| (t.Duration as f64 / 10_000_000.0).max(0.0)).unwrap_or(0.0);
        let dur = timeline.EndTime().ok().map(|t| (t.Duration as f64 / 10_000_000.0).max(0.0)).unwrap_or(0.0);
        let update_time = timeline.LastUpdatedTime().ok().map(|dt| dt.UniversalTime).unwrap_or(0);

        // Microsoft Windows standard GSMTC timeline offset compensation:
        // Position represents the snapshot at LastUpdatedTime.
        // True Position = Position + (CurrentTime - LastUpdatedTime).
        let real_pos = if is_playing && update_time > 0 {
            let now_filetime = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(d) => (d.as_secs() as i64 + 11_644_473_600) * 10_000_000 + (d.subsec_nanos() as i64 / 100),
                Err(_) => 0,
            };
            if now_filetime >= update_time {
                let elapsed = (now_filetime - update_time) as f64 / 10_000_000.0;
                if elapsed < 86400.0 {
                    (raw_pos + elapsed).min(dur)
                } else {
                    raw_pos
                }
            } else {
                raw_pos
            }
        } else {
            raw_pos
        };

        (real_pos, dur)
    } else {
        (0.0, 0.0)
    }
}

async fn fetch_thumbnail(
    props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> Option<Vec<u8>> {
    let thumb_ref = props.Thumbnail().ok()?;
    let stream_op = thumb_ref.OpenReadAsync().ok()?;
    let stream = stream_op.await.ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size >= 10 * 1024 * 1024 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    let load_op = reader.LoadAsync(size as u32).ok()?;
    if load_op.await.is_ok() {
        let mut buf = vec![0u8; size as usize];
        if reader.ReadBytes(&mut buf).is_ok() {
            return Some(buf);
        }
    }
    None
}

async fn fetch_media_properties_from_session(
    session: &GlobalSystemMediaTransportControlsSession,
    prev_track: Option<&MediaTrackInfo>,
) -> Option<MediaTrackInfo> {
    let source_app_id = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
    let props = session.TryGetMediaPropertiesAsync().ok()?.await.ok()?;

    let title = props.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
    let album = props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default();

    if title.is_empty() && artist.is_empty() {
        return None;
    }

    let is_playing = session
        .GetPlaybackInfo()
        .ok()
        .and_then(|info| info.PlaybackStatus().ok())
        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false);

    let (position_secs, duration_secs) = compute_timeline_position(session, is_playing);

    // Thumbnail Caching:
    // If title, artist and album match the previous track and we already have cached thumbnail bytes,
    // REUSE the cached bytes instead of re-downloading megabytes of image over cross-process COM stream!
    let thumbnail_bytes = if let Some(prev) = prev_track {
        if prev.thumbnail_bytes.is_some() && prev.title == title && prev.artist == artist && prev.album == album {
            prev.thumbnail_bytes.clone()
        } else {
            fetch_thumbnail(&props).await
        }
    } else {
        fetch_thumbnail(&props).await
    };

    Some(MediaTrackInfo {
        title,
        artist,
        album,
        source_app_id,
        is_playing,
        has_session: true,
        thumbnail_bytes,
        position_secs,
        duration_secs,
        last_updated: Some(Instant::now()),
        last_playing_instant: if is_playing { Some(Instant::now()) } else { None },
    })
}

async fn fetch_media_properties() -> Option<MediaTrackInfo> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.await.ok()?;
    let session = manager.GetCurrentSession().ok()?;
    fetch_media_properties_from_session(&session, None).await
}

/// Helper to launch a protocol URI or shell target directly without spawning any console/terminal window.
pub fn open_protocol_uri(uri: &str) {
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = format!("{uri}\0").encode_utf16().collect();
        ShellExecuteW(
            0 as _,
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// Helper function to bring an external application window to the foreground and place it
/// cleanly above the desktop and normal windows (including over Motion when it has yielded topmost).
unsafe fn bring_window_to_foreground(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
        IsIconic, SetForegroundWindow, SetWindowPos, ShowWindow,
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        SW_RESTORE, SW_SHOW,
    };
    use windows_sys::Win32::System::Threading::AttachThreadInput;

    if IsIconic(hwnd) != 0 {
        ShowWindow(hwnd, SW_RESTORE);
    } else {
        ShowWindow(hwnd, SW_SHOW);
    }

    // Classic Win32 technique: temporarily promote to HWND_TOPMOST then back to HWND_NOTOPMOST.
    // This positions the window at the very top of the non-topmost Z-order band,
    // ensuring it floats cleanly in front of Motion (which has yielded topmost).
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );
    SetWindowPos(
        hwnd,
        HWND_NOTOPMOST,
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );

    let fg_hwnd = GetForegroundWindow();
    let fg_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
    let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
    let cur_thread = windows_sys::Win32::System::Threading::GetCurrentThreadId();

    AttachThreadInput(cur_thread, target_thread, 1);
    if fg_thread != 0 && fg_thread != cur_thread {
        AttachThreadInput(cur_thread, fg_thread, 1);
    }

    BringWindowToTop(hwnd);
    SetForegroundWindow(hwnd);

    if fg_thread != 0 && fg_thread != cur_thread {
        AttachThreadInput(cur_thread, fg_thread, 0);
    }
    AttachThreadInput(cur_thread, target_thread, 0);
}

/// Attempts to bring the media player application window to the foreground.
pub fn activate_media_source_app(
    source_app_id: &str,
    app_title: &str,
    current_source: Option<&crate::controller::AudioSource>,
) {
    let app_lower = source_app_id.to_lowercase();

    // 0. If Motion visualizer is open, temporarily yield its HWND_TOPMOST
    // so the media player application window (e.g. Spotify) can display in front of it!
    crate::visualizer::yield_visualizer_topmost();

    // 1. Specialized protocol activations (fastest and most reliable for modern desktop/store apps)
    if app_lower.contains("spotify") {
        open_protocol_uri("spotify:");
    } else if app_lower.contains("applemusic") || app_lower.contains("apple.music") {
        open_protocol_uri("itms:");
    } else if app_lower.contains("tidal") {
        open_protocol_uri("tidal:");
    }

    // 2. Win32 Window activation via EnumWindows
    let mut target_keywords = Vec::new();
    if !source_app_id.is_empty() {
        // Strip UWP package family name e.g. "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"
        for part in source_app_id.split(['!', '.', '_', ' ']) {
            let p = part.trim().to_lowercase();
            if !p.is_empty() && p.len() > 2 && p != "exe" && p != "app" {
                target_keywords.push(p);
            }
        }
    }
    if let Some(crate::controller::AudioSource::Application { exe_name, .. }) = current_source {
        let e = exe_name.trim().trim_end_matches(".exe").to_lowercase();
        if !e.is_empty() {
            target_keywords.push(e);
        }
    }

    let title_match = if !app_title.trim().is_empty() {
        Some(app_title.to_lowercase())
    } else {
        None
    };

    struct WindowSearchContext {
        keywords: Vec<String>,
        title_hint: Option<String>,
        found_hwnd: isize,
    }

    unsafe extern "system" fn enum_windows_proc(
        hwnd: windows_sys::Win32::Foundation::HWND,
        lparam: windows_sys::Win32::Foundation::LPARAM,
    ) -> windows_sys::Win32::Foundation::BOOL {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        };
        use windows_sys::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        let ctx = &mut *(lparam as *mut WindowSearchContext);

        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 || pid == std::process::id() {
            return 1;
        }

        // Get window title
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        let win_title = if len > 0 {
            String::from_utf16_lossy(&title_buf[..len as usize]).to_lowercase()
        } else {
            String::new()
        };

        // Check title hint match
        if let Some(ref hint) = ctx.title_hint {
            if !win_title.is_empty() && (win_title.contains(hint) || hint.contains(&win_title)) {
                ctx.found_hwnd = hwnd as isize;
                return 0; // stop enumeration
            }
        }

        // Get process exe name
        let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !h_proc.is_null() {
            let mut img_buf = [0u16; 1024];
            let mut img_len = img_buf.len() as u32;
            let success = QueryFullProcessImageNameW(h_proc, 0, img_buf.as_mut_ptr(), &mut img_len);
            windows_sys::Win32::Foundation::CloseHandle(h_proc);

            if success != 0 && img_len > 0 {
                let full_path = String::from_utf16_lossy(&img_buf[..img_len as usize]).to_lowercase();
                for kw in &ctx.keywords {
                    if full_path.contains(kw) {
                        ctx.found_hwnd = hwnd as isize;
                        return 0; // stop enumeration
                    }
                }
            }
        }

        1
    }

    let mut ctx = WindowSearchContext {
        keywords: target_keywords,
        title_hint: title_match,
        found_hwnd: 0,
    };

    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_windows_proc),
            &mut ctx as *mut _ as windows_sys::Win32::Foundation::LPARAM,
        );
    }

    if ctx.found_hwnd != 0 {
        let hwnd = ctx.found_hwnd as windows_sys::Win32::Foundation::HWND;
        unsafe {
            bring_window_to_foreground(hwnd);
        }
    } else {
        if !source_app_id.is_empty() && !app_lower.contains("spotify") && !app_lower.contains("applemusic") && !app_lower.contains("tidal") {
            // Fallback: If it's a UWP AppUserModelId, launch shell:AppsFolder directly
            if source_app_id.contains('!') {
                open_protocol_uri(&format!("shell:AppsFolder\\{source_app_id}"));
            }
        }

        // Retry searching shortly in background in case the app was starting up via protocol
        let kw = ctx.keywords.clone();
        let th = ctx.title_hint.clone();
        std::thread::spawn(move || {
            for _ in 0..5 {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let mut retry_ctx = WindowSearchContext {
                    keywords: kw.clone(),
                    title_hint: th.clone(),
                    found_hwnd: 0,
                };
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows(
                        Some(enum_windows_proc),
                        &mut retry_ctx as *mut _ as windows_sys::Win32::Foundation::LPARAM,
                    );
                    if retry_ctx.found_hwnd != 0 {
                        bring_window_to_foreground(retry_ctx.found_hwnd as windows_sys::Win32::Foundation::HWND);
                        break;
                    }
                }
            }
        });
    }
}

pub fn send_media_control(cmd: MediaControlCmd) {
    if let Some(tx) = CMD_TX.get() {
        let _ = tx.send(cmd);
    }
}

fn simulate_media_key(vk: u8) {
    unsafe {
        // Multimedia keys (VK_MEDIA_NEXT_TRACK, VK_MEDIA_PREV_TRACK, etc.) are extended keys.
        // Windows input subsystem requires KEYEVENTF_EXTENDEDKEY to recognize them properly.
        windows_sys::Win32::UI::Input::KeyboardAndMouse::keybd_event(
            vk,
            0,
            windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_EXTENDEDKEY,
            0,
        );
        windows_sys::Win32::UI::Input::KeyboardAndMouse::keybd_event(
            vk,
            0,
            windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP
                | windows_sys::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_EXTENDEDKEY,
            0,
        );
    }
}

async fn execute_media_command(cmd: MediaControlCmd, state: &Arc<RwLock<MediaTrackInfo>>) {
    let mut session_handled = false;

    if let Ok(manager_op) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        if let Ok(manager) = manager_op.await {
            if let Ok(session) = manager.GetCurrentSession() {
                session_handled = true;
                match cmd {
                    MediaControlCmd::Play => {
                        let is_playing = session
                            .GetPlaybackInfo()
                            .ok()
                            .and_then(|info| info.PlaybackStatus().ok())
                            .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                            .unwrap_or(false);
                        if !is_playing {
                            let mut success = false;
                            if let Ok(op) = session.TryPlayAsync() {
                                if let Ok(res) = op.await {
                                    success = res;
                                }
                            }
                            if !success {
                                tracing::info!("SMTC TryPlayAsync was not handled; sending fallback VK_MEDIA_PLAY_PAUSE");
                                simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE as u8);
                            }
                        }
                    }
                    MediaControlCmd::Pause => {
                        crate::handoff::disarm();
                        let is_playing = session
                            .GetPlaybackInfo()
                            .ok()
                            .and_then(|info| info.PlaybackStatus().ok())
                            .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                            .unwrap_or(true);
                        if is_playing {
                            if let Ok(op) = session.TryPauseAsync() {
                                let _ = op.await;
                            }
                        }
                    }
                    MediaControlCmd::TogglePlayPause => {
                        // Check current playback state to send the explicit, idempotent command:
                        // If playing -> Pause. If paused -> Play.
                        // This prevents double-toggling where a failed/unsupported toggle would
                        // trigger a keyboard fallback that immediately reverses the state.
                        let is_playing = session
                            .GetPlaybackInfo()
                            .ok()
                            .and_then(|info| info.PlaybackStatus().ok())
                            .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                            .unwrap_or_else(|| {
                                state.read().map(|s| s.is_playing).unwrap_or(true)
                            });

                        if is_playing {
                            if let Ok(op) = session.TryPauseAsync() {
                                let _ = op.await;
                            }
                        } else {
                            if let Ok(op) = session.TryPlayAsync() {
                                let _ = op.await;
                            }
                        }
                    }
                    MediaControlCmd::Previous => {
                        let mut success = false;
                        if let Ok(op) = session.TrySkipPreviousAsync() {
                            if let Ok(res) = op.await {
                                success = res;
                            }
                        }
                        if !success {
                            tracing::info!("SMTC TrySkipPreviousAsync was not handled; sending VK_MEDIA_PREV_TRACK");
                            simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PREV_TRACK as u8);
                        }
                    }
                    MediaControlCmd::Next => {
                        let mut success = false;
                        if let Ok(op) = session.TrySkipNextAsync() {
                            if let Ok(res) = op.await {
                                success = res;
                            }
                        }
                        if !success {
                            tracing::info!("SMTC TrySkipNextAsync was not handled; sending VK_MEDIA_NEXT_TRACK");
                            simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_NEXT_TRACK as u8);
                        }
                    }
                    MediaControlCmd::Stop => {
                        crate::handoff::disarm();
                        if let Ok(op) = session.TryStopAsync() {
                            let _ = op.await;
                        }
                    }
                    MediaControlCmd::Seek(secs) => {
                        let ticks = (secs * 10_000_000.0) as i64;
                        if let Ok(op) = session.TryChangePlaybackPositionAsync(ticks) {
                            let _ = op.await;
                        }
                        if let Ok(mut lock) = state.write() {
                            lock.position_secs = secs;
                            lock.last_updated = Some(Instant::now());
                        }
                    }
                }
            }
        }
    }

    if !session_handled {
        tracing::info!("No SMTC session found, fallback for {:?}", cmd);
        let known_playing = state.read().map(|s| s.is_playing).unwrap_or(false);
        match cmd {
            MediaControlCmd::Play => {
                if !known_playing {
                    simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE as u8);
                    if let Ok(mut lock) = state.write() {
                        lock.is_playing = true;
                    }
                }
            }
            MediaControlCmd::Pause => {
                if known_playing {
                    simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE as u8);
                    if let Ok(mut lock) = state.write() {
                        lock.is_playing = false;
                    }
                }
            }
            MediaControlCmd::TogglePlayPause => {
                simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PLAY_PAUSE as u8);
            }
            MediaControlCmd::Next => {
                simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_NEXT_TRACK as u8);
            }
            MediaControlCmd::Previous => {
                simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_PREV_TRACK as u8);
            }
            MediaControlCmd::Stop => {
                simulate_media_key(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_MEDIA_STOP as u8);
            }
            _ => {}
        }
    }
}
