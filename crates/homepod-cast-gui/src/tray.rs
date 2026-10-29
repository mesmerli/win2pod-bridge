//! Windows system tray integration using tray-icon & muda.
//! Runs on a dedicated background thread with its own Win32 message pump
//! for 100% responsiveness independent of window visibility or eframe frame rate.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use eframe::egui;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW,
    RegisterClassExW, ShowWindow, TranslateMessage, MSG, WNDCLASSEXW, WS_OVERLAPPED,
};

use crate::controller::{ControllerState, DeviceStatus, GuiCmd};
use crate::i18n::{Language, NoticeKind};

pub static TRAY_THREAD_ID: AtomicU32 = AtomicU32::new(0);
pub const WM_USER: u32 = 0x0400;
pub const WM_TRAY_STATE_UPDATE: u32 = WM_USER + 101;
pub const WM_TRAY_RELOAD_HOTKEY: u32 = WM_USER + 102;
pub const WM_HOTKEY: u32 = 0x0312;
pub const HOTKEY_ID: i32 = 1001;

static POWER_CMD_TX: std::sync::OnceLock<async_channel::Sender<GuiCmd>> = std::sync::OnceLock::new();
static TRAY_EGUI_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

unsafe extern "system" fn power_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    const WM_POWERBROADCAST: u32 = 0x0218;
    const PBT_APMSUSPEND: usize = 0x0004;
    const PBT_APMRESUMESUSPEND: usize = 0x0007;
    const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
    const WM_DEVICECHANGE: u32 = 0x0219;

    if msg == WM_POWERBROADCAST {
        if let Some(cmd_tx) = POWER_CMD_TX.get() {
            if wparam == PBT_APMSUSPEND {
                tracing::info!("Power event: System entering suspend (PBT_APMSUSPEND) via WndProc");
                let _ = cmd_tx.send_blocking(GuiCmd::SystemSuspend);
            } else if wparam == PBT_APMRESUMESUSPEND || wparam == PBT_APMRESUMEAUTOMATIC {
                tracing::info!("Power event: System resuming from suspend (0x{:X}) via WndProc", wparam);
                let _ = cmd_tx.send_blocking(GuiCmd::SystemResume);
            }
        }
        return 1;
    } else if msg == WM_DEVICECHANGE {
        tracing::info!("Hardware PnP event: WM_DEVICECHANGE via WndProc, securing audio render shields");
        crate::controller::on_hardware_device_change();
        return 1;
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Notify the tray thread to re-register global hotkey with new modifiers and virtual key.
pub fn update_registered_hotkey(_mods: u32, _vk: u32) {
    let tid = TRAY_THREAD_ID.load(Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                tid,
                WM_TRAY_RELOAD_HOTKEY,
                0,
                0,
            );
        }
    }
}

/// Notify the tray thread's Win32 message queue that a new ControllerState is available.
pub fn notify_tray_state() {
    let tid = TRAY_THREAD_ID.load(Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                tid,
                WM_TRAY_STATE_UPDATE,
                0,
                0,
            );
        }
    }
}

pub static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn set_main_hwnd(hwnd: isize) {
    MAIN_HWND.store(hwnd, Ordering::SeqCst);
}

pub fn get_main_hwnd() -> HWND {
    let stored = MAIN_HWND.load(Ordering::SeqCst);
    if stored != 0 {
        return stored as HWND;
    }

    // Fallback: dynamically find the window by title
    unsafe {
        let title: Vec<u16> = "Win2Pod Bridge\0".encode_utf16().collect();
        FindWindowW(std::ptr::null(), title.as_ptr())
    }
}

/// Trims the process working set, instructing the Windows Memory Manager to flush inactive
/// driver pages (e.g. OpenGL/NVIDIA driver caches) and font memory into standby pool.
/// This reduces physical RAM usage from ~120MB+ down to ~8-15MB.
pub fn trim_working_set() {
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

#[link(name = "user32")]
extern "system" {
    fn AttachThreadInput(idattach: u32, idattachto: u32, fattach: i32) -> i32;
}

pub unsafe fn force_foreground(hwnd: HWND) {
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
        SetForegroundWindow, SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        SW_RESTORE, SW_SHOW,
    };

    ShowWindow(hwnd, SW_SHOW);
    ShowWindow(hwnd, SW_RESTORE);

    let fg = GetForegroundWindow();
    let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
    let cur_thread = GetCurrentThreadId();

    if fg_thread != 0 && fg_thread != cur_thread {
        AttachThreadInput(cur_thread, fg_thread, 1);
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        AttachThreadInput(cur_thread, fg_thread, 0);
    } else {
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
    }

    // Force Z-order placement to topmost and back so Windows DWM brings it above other windows
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    SetForegroundWindow(hwnd);
}

static START_INSTANT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
static LAST_SHOWN_MS: AtomicU64 = AtomicU64::new(0);

pub fn mark_just_shown() {
    let start = START_INSTANT.get_or_init(std::time::Instant::now);
    let ms = start.elapsed().as_millis() as u64;
    LAST_SHOWN_MS.store(ms, Ordering::SeqCst);
}

pub fn was_just_shown(threshold_ms: u64) -> bool {
    let start = START_INSTANT.get_or_init(std::time::Instant::now);
    let now_ms = start.elapsed().as_millis() as u64;
    let last = LAST_SHOWN_MS.load(Ordering::SeqCst);
    now_ms.saturating_sub(last) < threshold_ms
}

pub fn toggle_main_window(ctx: &egui::Context) {
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() {
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
                IsWindowVisible, ShowWindow, GA_ROOT, SW_HIDE,
            };

            let is_visible = IsWindowVisible(hwnd) != 0;
            let is_iconic = IsIconic(hwnd) != 0;

            let fg = GetForegroundWindow();
            let mut fg_pid = 0;
            let _ = GetWindowThreadProcessId(fg, &mut fg_pid);
            let current_pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
            let is_our_process = fg_pid == current_pid;

            let root_hwnd = GetAncestor(hwnd, GA_ROOT);
            let fg_root = if !fg.is_null() { GetAncestor(fg, GA_ROOT) } else { fg };
            let is_foreground = is_our_process
                || fg == hwnd
                || fg == root_hwnd
                || (!fg.is_null() && (fg_root == hwnd || fg_root == root_hwnd));

            tracing::info!(
                "toggle_main_window: hwnd={:?}, is_visible={}, is_iconic={}, fg={:?}, is_foreground={}",
                hwnd, is_visible, is_iconic, fg, is_foreground
            );

            if is_visible && !is_iconic && is_foreground {
                tracing::info!("toggle_main_window: Hiding main window to tray");
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ShowWindow(hwnd, SW_HIDE);
                trim_working_set();
            } else {
                tracing::info!("toggle_main_window: Showing and bringing main window to foreground");
                mark_just_shown();
                force_foreground(hwnd);
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.request_repaint();
            }
        }
    } else {
        tracing::info!("toggle_main_window: hwnd is null, sending Visible(true) to viewport");
        mark_just_shown();
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

pub fn show_and_focus_main_window(ctx: &egui::Context) {
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() {
        unsafe {
            force_foreground(hwnd);
        }
    }
    mark_just_shown();
    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

pub struct TrayManager {
    pub tray: TrayIcon,
    pub _menu: Menu,
    pub quit_id: tray_icon::menu::MenuId,
    pub show_id: tray_icon::menu::MenuId,
    pub mute_id: tray_icon::menu::MenuId,
    pub show_item: MenuItem,
    pub quit_item: MenuItem,
    pub mute_item: CheckMenuItem,
    last_status: Option<DeviceStatus>,
    last_muted: Option<bool>,
    last_name: Option<String>,
    last_volume: Option<f32>,
    last_notice: Option<NoticeKind>,
    last_language: Option<Language>,
}

impl TrayManager {
    pub fn new() -> anyhow::Result<Self> {
        let menu = Menu::new();
        let lang = crate::controller::load_language();

        let show_item = MenuItem::new(lang.tray_open_panel(), true, None);
        let show_id = show_item.id().clone();
        menu.append(&show_item)?;

        let mute_item = CheckMenuItem::new(lang.tray_mute(), true, false, None);
        let mute_id = mute_item.id().clone();
        menu.append(&mute_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let quit_item = MenuItem::new(lang.tray_quit(), true, None);
        let quit_id = quit_item.id().clone();
        menu.append(&quit_item)?;

        let icon = load_tray_icon(&DeviceStatus::Idle);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_menu_on_left_click(false)
            .with_tooltip("Win2Pod Bridge")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            tray,
            _menu: menu,
            quit_id,
            show_id,
            mute_id,
            show_item,
            quit_item,
            mute_item,
            last_status: None,
            last_muted: None,
            last_name: None,
            last_volume: None,
            last_notice: None,
            last_language: Some(lang),
        })
    }

    pub fn show_notification(&self, title: &str, message: &str) {
        unsafe {
            use windows_sys::Win32::UI::Shell::{
                Shell_NotifyIconW, NOTIFYICONDATAW, NIF_INFO, NIIF_INFO, NIM_MODIFY,
            };
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = self.tray.window_handle();
            nid.uID = 1;
            nid.uFlags = NIF_INFO;
            nid.dwInfoFlags = NIIF_INFO;

            let wide_title: Vec<u16> = title.encode_utf16().collect();
            let wide_msg: Vec<u16> = message.encode_utf16().collect();

            let title_len = wide_title.len().min(63);
            nid.szInfoTitle[..title_len].copy_from_slice(&wide_title[..title_len]);

            let msg_len = wide_msg.len().min(255);
            nid.szInfo[..msg_len].copy_from_slice(&wide_msg[..msg_len]);

            Shell_NotifyIconW(NIM_MODIFY, &mut nid as _);
        }
    }

    pub fn update_status(&mut self, state: &ControllerState) {
        let lang_changed = self.last_language != Some(state.language);
        let status_changed = self.last_status.as_ref() != Some(&state.status);
        let muted_changed = self.last_muted != Some(state.is_muted);
        let name_changed = self.last_name.as_ref() != state.connected_name.as_ref();
        let notice_changed = self.last_notice.as_ref() != state.notice_message.as_ref().map(|(msg, _)| msg);
        let volume_changed = self.last_volume.map(|v| (v - state.volume).abs() > 0.005).unwrap_or(true);

        // Avoid re-creating GDI icons and updating menus if status hasn't changed
        if !status_changed && !muted_changed && !name_changed && !notice_changed && !volume_changed && !lang_changed {
            return;
        }

        if lang_changed {
            self.last_language = Some(state.language);
            self.show_item.set_text(state.language.tray_open_panel());
            self.quit_item.set_text(state.language.tray_quit());
        }

        if let Some((ref notice, time)) = state.notice_message {
            if time.elapsed().as_secs() < 30 && self.last_notice.as_ref() != Some(notice) {
                self.last_notice = Some(notice.clone());
                self.show_notification(state.language.tray_notification_title(), &notice.render(state.language));
            }
        }

        if muted_changed || lang_changed {
            self.last_muted = Some(state.is_muted);
            self.mute_item.set_checked(state.is_muted);
            if state.is_muted {
                self.mute_item.set_text(state.language.tray_muted());
            } else {
                self.mute_item.set_text(state.language.tray_mute());
            }
        }

        if status_changed || name_changed || volume_changed || muted_changed || lang_changed {
            self.last_status = Some(state.status.clone());
            self.last_name = state.connected_name.clone();
            self.last_volume = Some(state.volume);
            self.last_muted = Some(state.is_muted);

            let vol_pct = (state.volume * 100.0).round() as i32;
            let vol_suffix = if state.is_muted {
                state.language.slider_muted_suffix().to_string()
            } else {
                format!(" [{vol_pct}%]")
            };

            let mut tooltip = match &state.status {
                DeviceStatus::Idle => state.language.tray_idle_tooltip().to_string(),
                DeviceStatus::Connecting => {
                    let name = state.connected_name.as_deref().unwrap_or("");
                    state.language.tray_connecting_tooltip(name)
                }
                DeviceStatus::Streaming => {
                    let name = state.connected_name.as_deref().unwrap_or("");
                    state.language.tray_streaming_tooltip(name, &vol_suffix)
                }
                DeviceStatus::Error(e) => state.language.tray_error_tooltip(e),
            };

            if let Some((ref notice, time)) = state.notice_message {
                if time.elapsed().as_secs() < 60 && state.status == DeviceStatus::Idle {
                    tooltip = format!("Win2Pod Bridge - {}（{}）", state.language.status_idle(), notice.render(state.language));
                }
            }

            let icon = load_tray_icon(&state.status);
            let _ = self.tray.set_icon(Some(icon));
            let _ = self.tray.set_tooltip(Some(tooltip));
        }
    }
}

const TRAY_IDLE_BYTES: &[u8] = include_bytes!("../assets/tray_idle.png");
const TRAY_STREAMING_BYTES: &[u8] = include_bytes!("../assets/tray_streaming.png");
const TRAY_CONNECTING_BYTES: &[u8] = include_bytes!("../assets/tray_connecting.png");
const TRAY_ERROR_BYTES: &[u8] = include_bytes!("../assets/tray_error.png");

/// Load specialized Win2Pod brand tray icon based on device connection status
pub fn load_tray_icon(status: &DeviceStatus) -> Icon {
    let bytes = match status {
        DeviceStatus::Idle => TRAY_IDLE_BYTES,
        DeviceStatus::Connecting => TRAY_CONNECTING_BYTES,
        DeviceStatus::Streaming => TRAY_STREAMING_BYTES,
        DeviceStatus::Error(_) => TRAY_ERROR_BYTES,
    };

    if let Ok(img) = image::load_from_memory(bytes) {
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();
        if let Ok(icon) = Icon::from_rgba(rgba.into_raw(), w, h) {
            return icon;
        }
    }

    create_color_icon(140, 140, 140)
}

/// Create a circular tray icon of specified color (RGBA 32x32)
pub fn create_color_icon(r: u8, g: u8, b: u8) -> Icon {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let center = SIZE as f32 / 2.0;
    let radius = (SIZE as f32 / 2.0) - 2.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= radius {
                rgba.extend_from_slice(&[r, g, b, 255]);
            } else if dist <= radius + 1.0 {
                let alpha = ((radius + 1.0 - dist) * 255.0) as u8;
                rgba.extend_from_slice(&[r, g, b, alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Icon::from_rgba(rgba, SIZE, SIZE).expect("Failed to create tray icon")
}

/// Spawn the background system tray thread with its own Win32 message pump
pub fn spawn_tray_thread(
    cmd_tx: async_channel::Sender<GuiCmd>,
    shared_state: Arc<RwLock<ControllerState>>,
    egui_ctx: egui::Context,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("system_tray".into())
        .spawn(move || {
            let mut tray_mgr = match TrayManager::new() {
                Ok(mgr) => mgr,
                Err(e) => {
                    tracing::error!("Failed to initialize system tray: {e:#}");
                    return;
                }
            };

            let is_muted = Arc::new(AtomicBool::new(false));
            let is_muted_menu = is_muted.clone();
            let cmd_tx_menu = cmd_tx.clone();
            let egui_ctx_menu = egui_ctx.clone();

            let quit_id = tray_mgr.quit_id.clone();
            let show_id = tray_mgr.show_id.clone();
            let mute_id = tray_mgr.mute_id.clone();

            // Direct callback for right-click context menu clicks
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
                if event.id == quit_id {
                    crate::controller::restore_original_speaker_mute();
                    let _ = cmd_tx_menu.send_blocking(GuiCmd::Quit);
                    std::process::exit(0);
                } else if event.id == show_id {
                    show_and_focus_main_window(&egui_ctx_menu);
                } else if event.id == mute_id {
                    let new_muted = !is_muted_menu.load(Ordering::SeqCst);
                    is_muted_menu.store(new_muted, Ordering::SeqCst);
                    let _ = cmd_tx_menu.send_blocking(GuiCmd::SetMute(new_muted));
                }
            }));

            // Direct callback for tray icon clicks (left-click toggles flyout panel)
            let egui_ctx_tray = egui_ctx.clone();
            tray_icon::TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    } => {
                        toggle_main_window(&egui_ctx_tray);
                    }
                    _ => {}
                }
            }));

            let tid = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
            TRAY_THREAD_ID.store(tid, Ordering::SeqCst);
            let _ = POWER_CMD_TX.set(cmd_tx.clone());
            let _ = TRAY_EGUI_CTX.set(egui_ctx.clone());

            // Create a dedicated top-level hidden window to reliably receive Windows broadcast messages (WM_POWERBROADCAST, WM_DEVICECHANGE)
            // as well as system-wide Global Hotkeys (WM_HOTKEY).
            // Message-only windows (HWND_MESSAGE) used internally by tray-icon never receive broadcast messages.
            unsafe {
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};

                let class_name: Vec<u16> = "Win2PodPowerNotifierClass\0".encode_utf16().collect();
                let wndclass = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: 0,
                    lpfnWndProc: Some(power_wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: 0 as _,
                    hIcon: 0 as _,
                    hCursor: 0 as _,
                    hbrBackground: 0 as _,
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                    hIconSm: 0 as _,
                };
                RegisterClassExW(&wndclass);

                let window_name: Vec<u16> = "Win2PodPowerNotifier\0".encode_utf16().collect();
                let notifier_hwnd = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    window_name.as_ptr(),
                    WS_OVERLAPPED, // Top-level window receives broadcast messages and global hotkeys
                    0,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0 as _,
                    std::ptr::null_mut(),
                );

                // Register global toggle hotkey
                let (cur_mods, cur_vk) = crate::controller::load_hotkey();
                if cur_vk != 0 {
                    let res = RegisterHotKey(notifier_hwnd, HOTKEY_ID, cur_mods, cur_vk);
                    tracing::info!(
                        "Registered global hotkey on top-level HWND (mods=0x{:X}, vk=0x{:X}, success={})",
                        cur_mods,
                        cur_vk,
                        res != 0
                    );
                }

                // Register for modern suspend/resume notifications if user32.dll provides it
                extern "system" {
                    fn GetModuleHandleA(lpModuleName: *const u8) -> isize;
                    fn GetProcAddress(hModule: isize, lpProcName: *const u8) -> Option<unsafe extern "system" fn() -> isize>;
                }
                let user32_mod = GetModuleHandleA(b"user32.dll\0".as_ptr());
                if user32_mod != 0 {
                    let proc_addr = GetProcAddress(user32_mod, b"RegisterSuspendResumeNotification\0".as_ptr());
                    if let Some(reg_fn) = proc_addr {
                        type RegFn = unsafe extern "system" fn(HWND, u32) -> isize;
                        let register_fn: RegFn = std::mem::transmute(reg_fn);
                        const DEVICE_NOTIFY_WINDOW_HANDLE: u32 = 0x00000000;
                        let _ = register_fn(notifier_hwnd, DEVICE_NOTIFY_WINDOW_HANDLE);
                        tracing::info!("Registered for Win32 Suspend/Resume notifications with top-level HWND");
                    }
                }

                let mut msg: MSG = std::mem::zeroed();

                const WM_POWERBROADCAST: u32 = 0x0218;
                const PBT_APMSUSPEND: usize = 0x0004;
                const PBT_APMRESUMESUSPEND: usize = 0x0007;
                const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
                const WM_DEVICECHANGE: u32 = 0x0219;

                // Initial state sync
                if let Ok(latest) = shared_state.read() {
                    is_muted.store(latest.is_muted, Ordering::SeqCst);
                    tray_mgr.update_status(&latest);
                }

                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    if msg.message == WM_POWERBROADCAST {
                        let wp = msg.wParam as usize;
                        if wp == PBT_APMSUSPEND {
                            tracing::info!("Power event: System entering suspend (PBT_APMSUSPEND) via message loop");
                            let _ = cmd_tx.send_blocking(GuiCmd::SystemSuspend);
                        } else if wp == PBT_APMRESUMESUSPEND || wp == PBT_APMRESUMEAUTOMATIC {
                            tracing::info!("Power event: System resuming from suspend (0x{:X}) via message loop", wp);
                            let _ = cmd_tx.send_blocking(GuiCmd::SystemResume);
                        }
                    } else if msg.message == WM_DEVICECHANGE {
                        tracing::info!("Hardware PnP event: WM_DEVICECHANGE via message loop, securing audio render shields");
                        crate::controller::on_hardware_device_change();
                    } else if msg.message == WM_HOTKEY && msg.wParam == HOTKEY_ID as usize {
                        tracing::info!("Global hotkey received via message loop, toggling main window");
                        toggle_main_window(&egui_ctx);
                        continue;
                    } else if msg.message == WM_TRAY_RELOAD_HOTKEY {
                        let (new_mods, new_vk) = crate::controller::load_hotkey();
                        UnregisterHotKey(notifier_hwnd, HOTKEY_ID);
                        if new_vk != 0 {
                            let res = RegisterHotKey(notifier_hwnd, HOTKEY_ID, new_mods, new_vk);
                            tracing::info!(
                                "Reloaded global hotkey: mods=0x{:X}, vk=0x{:X}, success={}",
                                new_mods,
                                new_vk,
                                res != 0
                            );
                        }
                    } else if msg.message == WM_TRAY_STATE_UPDATE {
                        if let Ok(latest) = shared_state.read() {
                            is_muted.store(latest.is_muted, Ordering::SeqCst);
                            tray_mgr.update_status(&latest);
                        }
                    }

                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                UnregisterHotKey(notifier_hwnd, HOTKEY_ID);
            }
        })
        .expect("Failed to spawn tray thread")
}
