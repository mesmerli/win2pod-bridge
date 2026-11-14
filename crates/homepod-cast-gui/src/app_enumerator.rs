use std::collections::HashMap;
use std::path::Path;
use windows::core::Interface;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;
use windows_sys::Win32::Foundation::BOOL;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Foundation::LPARAM;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioAppInfo {
    pub pid: u32,
    pub exe_name: String,
    pub display_name: String,
    pub is_active_audio: bool,
}

/// Enumerate all potential audio applications on the system.
/// Prioritizes processes currently having active WASAPI audio sessions,
/// followed by running GUI applications with visible windows.
pub fn get_audio_applications() -> Vec<AudioAppInfo> {
    let current_pid = std::process::id();
    let mut app_map: HashMap<u32, AudioAppInfo> = HashMap::new();

    // 1. Scan WASAPI Audio Sessions across all active audio render devices
    if let Ok(sessions) = scan_audio_sessions(current_pid) {
        for s in sessions {
            app_map.insert(s.pid, s);
        }
    }

    // 2. Scan visible top-level windows for running GUI apps not yet having an audio session
    let window_apps = scan_top_level_windows(current_pid);
    for app in window_apps {
        // If not already detected by WASAPI, add it
        app_map.entry(app.pid).or_insert(app);
    }

    // 3. Convert to Vec and sort:
    // Active audio first, then alphabetical by display name
    let mut list: Vec<AudioAppInfo> = app_map.into_values().collect();
    list.sort_by(|a, b| {
        match (a.is_active_audio, b.is_active_audio) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()),
        }
    });

    list
}

/// Helper to find a running PID by its executable name.
pub fn find_pid_by_exe(exe_name: &str) -> Option<u32> {
    let apps = get_audio_applications();
    apps.iter()
        .find(|a| a.exe_name.eq_ignore_ascii_case(exe_name))
        .map(|a| a.pid)
}

/// Scan WASAPI audio sessions across all active render endpoints.
fn scan_audio_sessions(current_pid: u32) -> windows::core::Result<Vec<AudioAppInfo>> {
    let mut results = Vec::new();

    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let collection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        for dev_idx in 0..count {
            let dev = match collection.Item(dev_idx) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let session_manager: IAudioSessionManager2 = match dev.Activate(CLSCTX_ALL, None) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let session_enumerator = match session_manager.GetSessionEnumerator() {
                Ok(e) => e,
                Err(_) => continue,
            };

            let session_count = match session_enumerator.GetCount() {
                Ok(c) => c,
                Err(_) => continue,
            };

            for sess_idx in 0..session_count {
                let session = match session_enumerator.GetSession(sess_idx) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let session2: IAudioSessionControl2 = match session.cast() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let pid = match session2.GetProcessId() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                // Ignore system sounds, PID 0, or our own app
                if pid == 0 || pid == current_pid {
                    continue;
                }

                let is_system = session2.IsSystemSoundsSession() == S_OK;
                if is_system {
                    continue;
                }

                let state = session.GetState().unwrap_or(AudioSessionStateInactive);
                if state == AudioSessionStateExpired {
                    continue;
                }

                let is_active = state == AudioSessionStateActive;

                let (exe_name, window_title) = get_process_details(pid);
                if exe_name.is_empty() {
                    continue;
                }

                let display_name = if !window_title.is_empty() {
                    window_title
                } else {
                    format_friendly_exe_name(&exe_name)
                };

                results.push(AudioAppInfo {
                    pid,
                    exe_name,
                    display_name,
                    is_active_audio: is_active,
                });
            }
        }
    }

    Ok(results)
}

/// Retrieve process executable name and main window title (if any).
fn get_process_details(pid: u32) -> (String, String) {
    use windows::Win32::System::Threading::*;
    let mut exe_name = String::new();

    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buffer = [0u16; 512];
            let mut size = buffer.len() as u32;
            let res = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(handle);

            if res.is_ok() && size > 0 {
                let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
                if let Some(file_name) = Path::new(&full_path).file_name() {
                    exe_name = file_name.to_string_lossy().to_string();
                }
            }
        }
    }

    let window_title = find_window_title_for_pid(pid);
    (exe_name, window_title)
}

/// Scan visible top-level windows using EnumWindows.
fn scan_top_level_windows(current_pid: u32) -> Vec<AudioAppInfo> {
    struct WindowScanContext {
        current_pid: u32,
        apps: Vec<AudioAppInfo>,
    }

    unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut WindowScanContext);

        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);

        if pid == 0 || pid == ctx.current_pid {
            return 1;
        }

        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return 1;
        }

        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        if copied > 0 {
            let title = String::from_utf16_lossy(&buf[..copied as usize]).trim().to_string();
            // Filter out system or unwanted utility window titles
            if !title.is_empty()
                && title != "Settings"
                && title != "Windows Input Experience"
                && title != "Program Manager"
            {
                let (exe_name, _) = get_process_details(pid);
                if !exe_name.is_empty()
                    && !exe_name.eq_ignore_ascii_case("explorer.exe")
                    && !exe_name.eq_ignore_ascii_case("dwm.exe")
                    && !exe_name.eq_ignore_ascii_case("SearchHost.exe")
                {
                    ctx.apps.push(AudioAppInfo {
                        pid,
                        exe_name,
                        display_name: title,
                        is_active_audio: false,
                    });
                }
            }
        }

        1
    }

    let mut context = WindowScanContext {
        current_pid,
        apps: Vec::new(),
    };

    unsafe {
        EnumWindows(Some(enum_window_callback), &mut context as *mut _ as LPARAM);
    }

    context.apps
}

/// Find the main visible window title for a given PID.
fn find_window_title_for_pid(target_pid: u32) -> String {
    struct TitleSearchContext {
        target_pid: u32,
        best_title: String,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut TitleSearchContext);

        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);

        if pid == ctx.target_pid {
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                if copied > 0 {
                    let title = String::from_utf16_lossy(&buf[..copied as usize]).trim().to_string();
                    if !title.is_empty() {
                        ctx.best_title = title;
                        return 0; // stop searching
                    }
                }
            }
        }

        1
    }

    let mut ctx = TitleSearchContext {
        target_pid,
        best_title: String::new(),
    };

    unsafe {
        EnumWindows(Some(enum_callback), &mut ctx as *mut _ as LPARAM);
    }

    ctx.best_title
}

/// Turn "Spotify.exe" into "Spotify", "chrome.exe" into "Google Chrome" etc.
fn format_friendly_exe_name(exe_name: &str) -> String {
    let lower = exe_name.to_lowercase();
    match lower.as_str() {
        "spotify.exe" => "Spotify".to_string(),
        "chrome.exe" => "Google Chrome".to_string(),
        "msedge.exe" => "Microsoft Edge".to_string(),
        "firefox.exe" => "Mozilla Firefox".to_string(),
        "applemusic.exe" => "Apple Music".to_string(),
        "itunes.exe" => "iTunes".to_string(),
        "foobar2000.exe" => "foobar2000".to_string(),
        "vlc.exe" => "VLC Media Player".to_string(),
        "discord.exe" => "Discord".to_string(),
        "steam.exe" => "Steam".to_string(),
        _ => {
            // Strip .exe extension and capitalize
            let stem = exe_name.strip_suffix(".exe").unwrap_or(exe_name);
            let mut chars = stem.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => stem.to_string(),
            }
        }
    }
}
