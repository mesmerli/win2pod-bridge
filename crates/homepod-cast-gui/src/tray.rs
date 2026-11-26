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
pub static IS_TRAY_HIDDEN: AtomicBool = AtomicBool::new(false);
pub static IS_APP_FLEET_HIDDEN: AtomicBool = AtomicBool::new(false);
pub static IS_ALWAYS_ON_TOP: AtomicBool = AtomicBool::new(false);
pub static IS_APP_QUITTING: AtomicBool = AtomicBool::new(false);
pub static LAST_POS_X: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(i32::MIN);
pub static LAST_POS_Y: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(i32::MIN);
pub static TRAY_ICON_RECT_X: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(i32::MIN);
pub static TRAY_ICON_RECT_Y: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(i32::MIN);

pub fn is_app_fleet_hidden() -> bool {
    IS_APP_FLEET_HIDDEN.load(Ordering::SeqCst)
}

pub fn is_app_quitting() -> bool {
    IS_APP_QUITTING.load(Ordering::SeqCst)
}

pub fn trigger_app_quit(cmd_tx: &async_channel::Sender<GuiCmd>) {
    if IS_APP_QUITTING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        tracing::info!("[Shutdown] Initiating graceful application shutdown...");
        let _ = cmd_tx.try_send(GuiCmd::Quit);
        std::thread::spawn(|| {
            // Give graceful shutdown ample time (5.0s) for network RTSP TEARDOWN and audio endpoint restoration.
            // Under normal conditions, controller exits cleanly in ~0.5s - 1.5s via std::process::exit(0).
            std::thread::sleep(std::time::Duration::from_millis(5000));
            tracing::warn!("[Watchdog] Graceful exit timed out after 5000ms. Forcing exit.");
            std::process::exit(0);
        });
    } else {
        tracing::debug!("[Shutdown] Quit already in progress, ignoring repeated request.");
    }
}

pub fn is_window_hidden_to_tray() -> bool {
    IS_TRAY_HIDDEN.load(Ordering::SeqCst)
}

#[inline]
pub fn is_any_window_visible() -> bool {
    !is_window_hidden_to_tray()
        || crate::visualizer::is_visualizer_visible()
        || crate::snapping::is_window_visible_and_active(crate::snapping::WindowKind::MiniPlayer)
        || crate::snapping::is_window_visible_and_active(crate::snapping::WindowKind::Equalizer)
}

static START_INSTANT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
static LAST_SHOWN_MS: AtomicU64 = AtomicU64::new(0);

pub fn record_shown_time() {
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

/// Pure state evaluation: checks if Main Window is currently frontmost, on Page::Main,
/// unminimized, not hidden, and fully ready for magnetic docking (Stage 2).
/// Re-exported from Window Manager (snapping.rs).
pub fn is_main_window_dock_ready() -> bool {
    crate::snapping::is_main_window_dock_ready()
}

pub fn is_main_window_in_foreground() -> bool {
    let hwnd = get_main_hwnd();
    if hwnd.is_null() || is_window_hidden_to_tray() {
        return false;
    }
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsIconic, IsWindowVisible};
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }
        GetForegroundWindow() == hwnd
    }
}

pub fn set_main_hwnd(hwnd: isize) {
    MAIN_HWND.store(hwnd, Ordering::SeqCst);
}

pub fn set_main_window_pinned(pinned: bool) {
    crate::snapping::set_window_pinned(crate::snapping::WindowKind::Main, pinned);
    crate::controller::save_always_on_top(pinned);
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() {
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
            };
            let target = if pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
            SetWindowPos(hwnd, target, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        }
    }
    crate::gui::request_ui_repaint();
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

/// Hide window into system tray by moving off-screen and setting WS_EX_TOOLWINDOW.
/// We intentionally do NOT use ShowWindow(SW_HIDE) because Windows DWM stops sending
/// VSync and WM_PAINT to SW_HIDE windows, causing eframe/winit's event loop to enter
/// an unconstrained busy-poll loop consuming 100% of a CPU core (~5% total).
/// Moving off-screen keeps DWM VSync active while removing the window from the screen,
/// taskbar, and Alt+Tab switcher with 0.0% CPU usage.
/// Hide ONLY the main window to the system tray (used when closing main window / cluster via X)
pub fn hide_main_window_only(hwnd: HWND, _ctx: &egui::Context) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, GetWindowRect, SetWindowLongW, SetWindowPos,
            GWL_EXSTYLE, HWND_NOTOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE,
            SWP_NOSIZE, SWP_NOZORDER, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };

        let mut rect = windows_sys::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(hwnd, &mut rect);
        let monitor = windows_sys::Win32::Graphics::Gdi::MonitorFromWindow(
            hwnd,
            windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
        );
        let mut mi: windows_sys::Win32::Graphics::Gdi::MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32;
        if windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut mi) != 0 {
            let work = mi.rcWork;
            if rect.right >= work.left + 60
                && rect.left <= work.right - 60
                && rect.bottom >= work.top + 60
                && rect.top <= work.bottom - 60
            {
                LAST_POS_X.store(rect.left, Ordering::SeqCst);
                LAST_POS_Y.store(rect.top, Ordering::SeqCst);
            }
        }

        IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        crate::spectrum::update_spectrum_needed_state();
        let _ = crate::app_lifecycle::dispatch_app_lifecycle_event(crate::app_lifecycle::AppLifecycleEvent::HideToTray);

        // Capture window screenshot for Muelle Genie S-Curve effect before moving offscreen
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let mut capture_bmp: usize = 0;
        if w > 50 && h > 50 {
            let screen_dc = windows_sys::Win32::Graphics::Gdi::GetDC(std::ptr::null_mut());
            let mem_dc = windows_sys::Win32::Graphics::Gdi::CreateCompatibleDC(screen_dc);
            let hbmp = windows_sys::Win32::Graphics::Gdi::CreateCompatibleBitmap(screen_dc, w, h);
            let old_bmp = windows_sys::Win32::Graphics::Gdi::SelectObject(mem_dc, hbmp);
            windows_sys::Win32::Graphics::Gdi::BitBlt(
                mem_dc,
                0,
                0,
                w,
                h,
                screen_dc,
                rect.left,
                rect.top,
                windows_sys::Win32::Graphics::Gdi::SRCCOPY,
            );
            windows_sys::Win32::Graphics::Gdi::SelectObject(mem_dc, old_bmp);
            windows_sys::Win32::Graphics::Gdi::DeleteDC(mem_dc);
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), screen_dc);
            capture_bmp = hbmp as usize;
        }

        if !IS_APP_FLEET_HIDDEN.load(Ordering::SeqCst) {
            crate::snapping::handle_window_closed(crate::snapping::WindowKind::Main);
        }
        crate::snapping::sync_cluster_taskbar_ownership();

        if capture_bmp != 0 {
            // Trigger macOS-style S-curve Genie effect with seamless atomic handover.
            // The original window remains in place until frame 0 is rendered and displayed,
            // preventing any blank flash or frame gap flicker.
            crate::genie_effect::play_genie_s_curve_effect(
                capture_bmp,
                hwnd as usize,
                rect.left,
                rect.top,
                w,
                h,
                crate::genie_effect::GenieTarget::Tray,
            );
        } else {
            // Fallback: If no screenshot bitmap available, hide and move immediately
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let new_ex = (ex | WS_EX_TOOLWINDOW as i32) & !(WS_EX_APPWINDOW as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex);
            crate::snapping::delete_taskbar_tab_pub(hwnd);

            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                -32000,
                -32000,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            schedule_trim_working_set();
        }
    }
}

/// Backward compatibility alias for hide_main_window_only
pub fn hide_window_to_tray(hwnd: HWND, ctx: &egui::Context) {
    hide_main_window_only(hwnd, ctx);
}

/// Hide ALL application windows to the system tray (used by global hotkey / tray click)
pub fn hide_all_windows_to_tray(hwnd: HWND, ctx: &egui::Context) {
    IS_APP_FLEET_HIDDEN.store(true, Ordering::SeqCst);
    hide_main_window_only(hwnd, ctx);

    unsafe {
        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
        if !vis_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(vis_hwnd) != 0 {
            crate::visualizer::hide_visualizer_with_cluster();
        }

        let eq_hwnd = crate::snapping::get_equalizer_hwnd();
        if !eq_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(eq_hwnd) != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(eq_hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE);
        }

        let mini_hwnd = crate::snapping::get_mini_player_hwnd();
        if !mini_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(mini_hwnd) != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(mini_hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE);
        }
    }
}

/// Helper to finalize restoring the window and its associated cluster to screen
pub fn show_window_actual_finish(hwnd: HWND, x: i32, y: i32, ctx: &egui::Context) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SetWindowLongW, SetWindowPos,
            GWL_EXSTYLE, HWND_TOP,
            SWP_FRAMECHANGED, SWP_NOSIZE, SWP_SHOWWINDOW,
            WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
        };

        // Restore normal ex_style (removes WS_EX_TOOLWINDOW)
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let new_ex = (ex & !(WS_EX_TOOLWINDOW as i32)) | (WS_EX_APPWINDOW as i32);
        SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex);
        crate::snapping::add_taskbar_tab_pub(hwnd);

        // Position and show main window at target coordinates directly
        SetWindowPos(
            hwnd,
            HWND_TOP,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );

        // Restore any bonded or detached visualizer window that was hidden with the cluster/app
        crate::visualizer::restore_visualizer_with_cluster();

        let eq_hwnd = crate::snapping::get_equalizer_hwnd();
        if !eq_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(eq_hwnd) != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(eq_hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE);
        }

        let mini_hwnd = crate::snapping::get_mini_player_hwnd();
        if !mini_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(mini_hwnd) != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(mini_hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE);
        }

        // Elevate all fleet windows (including detached ones) and preserve clustered relations atomically
        crate::snapping::bring_fleet_to_top();
        crate::snapping::sync_cluster_taskbar_ownership();

        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

/// Restore window from system tray back onto visible desktop screen
pub fn show_window_from_tray(hwnd: HWND, ctx: &egui::Context) {
    crate::genie_effect::abort_genie_effect();
    if hwnd.is_null() {
        return;
    }
    cancel_pending_trim();
    ctx.request_repaint();
    record_shown_time();
    unsafe {
        IS_APP_FLEET_HIDDEN.store(false, Ordering::SeqCst);
        IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);
        crate::spectrum::update_spectrum_needed_state();
        let _ = crate::app_lifecycle::dispatch_app_lifecycle_event(crate::app_lifecycle::AppLifecycleEvent::ShowUI);

        let mut x = LAST_POS_X.load(Ordering::SeqCst);
        let mut y = LAST_POS_Y.load(Ordering::SeqCst);
        let work = crate::snapping::get_monitor_work_area(hwnd);
        if x < work.left || x + 60 > work.right || y < work.top || y + 60 > work.bottom {
            x = work.left + (work.right - work.left - 460).max(0) / 2;
            y = work.top + (work.bottom - work.top - 680).max(0) / 2;
            LAST_POS_X.store(x, Ordering::SeqCst);
            LAST_POS_Y.store(y, Ordering::SeqCst);
        }

        // Add taskbar tab immediately so Taskbar shows active state while expanding
        crate::snapping::add_taskbar_tab_pub(hwnd);

        let target_hwnd = hwnd as usize;
        let ctx_clone = ctx.clone();
        let restore_action = move || {
            show_window_actual_finish(target_hwnd as HWND, x, y, &ctx_clone);
        };

        if !crate::genie_effect::play_reverse_genie_s_curve_effect(x, y, hwnd as usize, crate::genie_effect::GenieTarget::Tray, restore_action) {
            // Fallback: If no cached snapshot available, restore immediately
            show_window_actual_finish(hwnd, x, y, ctx);
        }
    }
}

static PENDING_TRIM_GEN: AtomicU64 = AtomicU64::new(0);

/// Schedules a debounced working set trim after 8 seconds of inactivity in tray.
/// If the window is restored before the timer fires, the trim is cleanly canceled.
pub fn schedule_trim_working_set() {
    let gen = PENDING_TRIM_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::Builder::new()
        .name("ws-trim-debounce".to_string())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(8));
            if PENDING_TRIM_GEN.load(Ordering::SeqCst) == gen && is_window_hidden_to_tray() {
                trim_working_set();
                tracing::debug!("Working set trimmed after debounce delay");
            }
        })
        .ok();
}

/// Cancels any scheduled debounced working set trim.
pub fn cancel_pending_trim() {
    PENDING_TRIM_GEN.fetch_add(1, Ordering::SeqCst);
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

/// Ensures that the specified window is within the visible boundaries of its monitor work area.
/// If it is off-screen (e.g. less than 60px visible), it is moved back onto the screen.
pub unsafe fn ensure_window_on_screen(hwnd: HWND) -> bool {
    if hwnd.is_null() || is_window_hidden_to_tray() {
        return false;
    }

    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};

    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if GetWindowRect(hwnd, &mut rect) == 0 {
        return false;
    }

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut mi) == 0 {
        return false;
    }

    let work = mi.rcWork;
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;

    let is_off = rect.right < work.left + 60
        || rect.left > work.right - 60
        || rect.bottom < work.top + 60
        || rect.top > work.bottom - 60;

    if is_off {
        let win_w = if w < 380 { 460 } else { w };
        let win_h = if h < 320 { 680 } else { h };
        let mut new_x = LAST_POS_X.load(Ordering::SeqCst);
        let mut new_y = LAST_POS_Y.load(Ordering::SeqCst);

        if new_x < work.left || new_x + 60 > work.right || new_y < work.top || new_y + 60 > work.bottom {
            new_x = work.left + ((work.right - work.left - win_w).max(0)) / 2;
            new_y = work.top + ((work.bottom - work.top - win_h).max(0)) / 2;
        }

        LAST_POS_X.store(new_x, Ordering::SeqCst);
        LAST_POS_Y.store(new_y, Ordering::SeqCst);

        let flags = if w < 380 || h < 320 {
            SWP_NOZORDER | SWP_NOACTIVATE
        } else {
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE
        };

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            new_x,
            new_y,
            win_w,
            win_h,
            flags,
        );
        return true;
    }
    false
}

pub unsafe fn force_foreground(hwnd: HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
        SetForegroundWindow, SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOACTIVATE, SW_RESTORE, SW_SHOW,
    };

    if hwnd.is_null() {
        return;
    }

    ShowWindow(hwnd, SW_SHOW);
    if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0 {
        ShowWindow(hwnd, SW_RESTORE);
    }

    let fg = GetForegroundWindow();
    if fg == hwnd {
        return;
    }

    let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
    let fg_thread = if !fg.is_null() {
        GetWindowThreadProcessId(fg, std::ptr::null_mut())
    } else {
        0
    };

    // Attach target GUI thread to fg_thread ONLY if target_thread is valid and different from fg_thread.
    // Note: Never attach from a background worker thread, only the window's owning GUI thread!
    let attached = if fg_thread != 0 && target_thread != 0 && fg_thread != target_thread {
        AttachThreadInput(target_thread, fg_thread, 1) != 0
    } else {
        false
    };

    BringWindowToTop(hwnd);
    SetForegroundWindow(hwnd);

    if attached {
        AttachThreadInput(target_thread, fg_thread, 0);
    }

    // Single TOPMOST toggle to guarantee bringing above third-party windows if needed
    if !IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) && !crate::visualizer::is_visualizer_fullscreen() {
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    }
    SetForegroundWindow(hwnd);
}

pub fn toggle_main_window(ctx: &egui::Context) {
    crate::genie_effect::abort_genie_effect();
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() {
        let is_hidden = is_window_hidden_to_tray();
        let (is_foreground, is_minimized, is_offscreen) = unsafe {
            use windows_sys::Win32::Foundation::RECT;
            use windows_sys::Win32::System::Threading::GetCurrentProcessId;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic,
            };

            let fg = GetForegroundWindow();
            let mut fg_pid = 0u32;
            GetWindowThreadProcessId(fg, &mut fg_pid);
            let cur_pid = GetCurrentProcessId();
            let is_fg = fg == hwnd || (fg_pid != 0 && fg_pid == cur_pid);
            let is_min = IsIconic(hwnd) != 0;

            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(hwnd, &mut rect);
            let monitor = windows_sys::Win32::Graphics::Gdi::MonitorFromWindow(
                hwnd,
                windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
            );
            let mut mi: windows_sys::Win32::Graphics::Gdi::MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32;
            let is_off = if windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut mi) != 0 {
                let work = mi.rcWork;
                rect.right < work.left + 60
                    || rect.left > work.right - 60
                    || rect.bottom < work.top + 60
                    || rect.top > work.bottom - 60
            } else {
                rect.left <= -10000 || rect.top <= -10000
            };

            (is_fg, is_min, is_off)
        };

        if is_hidden || is_offscreen {
            tracing::info!("toggle_main_window: Window hidden in tray -> showing and focusing fleet");
            show_window_from_tray(hwnd, ctx);
        } else if !is_foreground || is_minimized {
            tracing::info!(
                "toggle_main_window: Fleet not in foreground (is_fg={}, min={}) -> bringing all windows to top",
                is_foreground,
                is_minimized
            );
            unsafe {
                crate::snapping::bring_fleet_to_top();
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
        } else {
            tracing::info!("toggle_main_window: Fleet is already foreground -> hiding all to tray");
            hide_all_windows_to_tray(hwnd, ctx);
        }
    }
}

pub fn show_and_focus_main_window(ctx: &egui::Context) {
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() {
        if is_window_hidden_to_tray() {
            show_window_from_tray(hwnd, ctx);
        } else {
            unsafe {
                if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0 {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                        hwnd,
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_RESTORE,
                    );
                }
                ensure_window_on_screen(hwnd);
                force_foreground(hwnd);
                crate::snapping::bring_fleet_to_top();
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
        }
    }
}

pub fn center_and_open_main_window(ctx: &egui::Context) {
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 } {
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect, IsIconic, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SW_RESTORE};

            // 立即終止任何進行中或快照的神燈縮放動畫，避免結束回呼將視窗拉回舊座標
            crate::genie_effect::abort_genie_effect();

            // 保底救回：強制重設主視窗透明度為 100% 完全不透明，解除穿透模式
            crate::snapping::set_window_opacity(crate::snapping::WindowKind::Main, 1.0);

            // 先精準計算當前螢幕工作區正中央座標，並立即更新全域記錄
            let mut pt = std::mem::zeroed();
            GetCursorPos(&mut pt);
            let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            let work = if GetMonitorInfoW(monitor, &mut mi) != 0 {
                mi.rcWork
            } else {
                crate::snapping::get_monitor_work_area(hwnd)
            };

            let cx = work.left + (work.right - work.left - 460).max(0) / 2;
            let cy = work.top + (work.bottom - work.top - 680).max(0) / 2;
            LAST_POS_X.store(cx, Ordering::SeqCst);
            LAST_POS_Y.store(cy, Ordering::SeqCst);

            if is_window_hidden_to_tray() {
                // 從系統匣復原時直接於 (cx, cy) 完成顯示，防止非同步動畫覆寫座標
                cancel_pending_trim();
                record_shown_time();
                IS_APP_FLEET_HIDDEN.store(false, Ordering::SeqCst);
                IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);
                crate::spectrum::update_spectrum_needed_state();
                let _ = crate::app_lifecycle::dispatch_app_lifecycle_event(crate::app_lifecycle::AppLifecycleEvent::ShowUI);
                show_window_actual_finish(hwnd, cx, cy, ctx);
            } else {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                let mut rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
                let w = (rect.right - rect.left).max(460);
                let h = (rect.bottom - rect.top).max(680);
                SetWindowPos(hwnd, HWND_TOPMOST, cx, cy, w, h, SWP_NOACTIVATE);
                ensure_window_on_screen(hwnd);
                force_foreground(hwnd);
                crate::snapping::bring_fleet_to_top();
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

pub unsafe fn reset_all_windows_to_screen_center() {
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 {
        // 保底救回：強制重設主視窗透明度為 100% 完全不透明，解除穿透模式
        crate::snapping::set_window_opacity(crate::snapping::WindowKind::Main, 1.0);

        if is_window_hidden_to_tray() {
            if let Some(ctx) = crate::gui::get_egui_ctx() {
                show_window_from_tray(hwnd, &ctx);
            }
        }
        let work = crate::snapping::get_monitor_work_area(hwnd);
        let cx = work.left + (work.right - work.left - 460).max(0) / 2;
        let cy = work.top + (work.bottom - work.top - 680).max(0) / 2;
        LAST_POS_X.store(cx, Ordering::SeqCst);
        LAST_POS_Y.store(cy, Ordering::SeqCst);
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
            hwnd,
            windows_sys::Win32::UI::WindowsAndMessaging::HWND_TOPMOST,
            cx,
            cy,
            0,
            0,
            windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        );
        ensure_window_on_screen(hwnd);
        force_foreground(hwnd);
    }
}

pub fn bring_main_to_front() {
    crate::genie_effect::abort_genie_effect();
    let hwnd = get_main_hwnd();
    if !hwnd.is_null() {
        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
        let was_hidden = is_window_hidden_to_tray();
        tracing::info!("bring_main_to_front called: hwnd={:?}, vis_hwnd={:?}, was_hidden={}", hwnd, vis_hwnd, was_hidden);

        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                FindWindowW, GetWindowLongW, IsIconic, SetWindowLongW,
                SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_BOTTOM,
                HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
                SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, SW_SHOWNOACTIVATE,
                WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
            };

            record_shown_time();
            IS_APP_FLEET_HIDDEN.store(false, Ordering::SeqCst);

            if was_hidden {
                IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);
                crate::spectrum::update_spectrum_needed_state();
                let _ = crate::app_lifecycle::dispatch_app_lifecycle_event(crate::app_lifecycle::AppLifecycleEvent::ShowUI);
                let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let new_ex = (ex & !(WS_EX_TOOLWINDOW as i32)) | (WS_EX_APPWINDOW as i32);
                SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex);
                crate::snapping::add_taskbar_tab_pub(hwnd);

                let anchor = if !vis_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(vis_hwnd) != 0 {
                    vis_hwnd
                } else {
                    hwnd
                };
                let work = crate::snapping::get_monitor_work_area(anchor);
                let mut x = LAST_POS_X.load(Ordering::SeqCst);
                let mut y = LAST_POS_Y.load(Ordering::SeqCst);
                if x < work.left || x + 60 > work.right || y < work.top || y + 60 > work.bottom {
                    x = work.left + (work.right - work.left - 460).max(0) / 2;
                    y = work.top + (work.bottom - work.top - 680).max(0) / 2;
                    LAST_POS_X.store(x, Ordering::SeqCst);
                    LAST_POS_Y.store(y, Ordering::SeqCst);
                }
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
            } else {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
            }
            ensure_window_on_screen(hwnd);

            // If visualizer is open, demote visualizer to HWND_NOTOPMOST so main window
            // is guaranteed by Windows DWM to stay above it, even when visualizer has keyboard focus!
            if !vis_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(vis_hwnd) != 0 {
                use windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST;
                SetWindowPos(vis_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }

            let is_top = IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) || crate::visualizer::is_visualizer_fullscreen();
            let target_tier = if is_top { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };
            SetWindowPos(hwnd, target_tier, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);

            // Elevate all fleet windows (including detached ones) and preserve clustered relations
            crate::snapping::bring_fleet_to_top();

            // If visualizer is open and in fullscreen, ensure Windows taskbar stays below it
            if crate::visualizer::is_visualizer_fullscreen() {
                let taskbar_class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
                let taskbar = FindWindowW(taskbar_class.as_ptr(), std::ptr::null());
                if !taskbar.is_null() {
                    SetWindowPos(taskbar, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
                let sec_taskbar_class: Vec<u16> = "Shell_SecondaryTrayWnd\0".encode_utf16().collect();
                let sec_taskbar = FindWindowW(sec_taskbar_class.as_ptr(), std::ptr::null());
                if !sec_taskbar.is_null() {
                    SetWindowPos(sec_taskbar, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }

            // Keep focus on the visualizer window only in fullscreen mode so keyboard shortcuts remain completely fluent
            if crate::visualizer::is_visualizer_fullscreen()
                && !vis_hwnd.is_null()
                && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(vis_hwnd) != 0
            {
                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(vis_hwnd);
            } else {
                force_foreground(hwnd);
            }

            crate::snapping::sync_cluster_taskbar_ownership();
        }
        if let Some(ctx) = crate::gui::get_egui_ctx() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.request_repaint();
        }
    }
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
    last_airpods_sig: Option<(bool, bool, Option<u8>, Option<u8>, Option<u8>, bool, bool, bool, bool, bool)>,
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

        let icon = load_tray_icon(&DeviceStatus::Idle, false);
        tracing::info!("[Tray] Initializing system tray icon with tooltip 'Win2Pod Bridge'...");

        let tray = match TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_menu_on_left_click(false)
            .with_tooltip("Win2Pod Bridge")
            .with_icon(icon)
            .build()
        {
            Ok(t) => {
                let rect = t.rect();
                if let Some(r) = rect {
                    TRAY_ICON_RECT_X.store(r.position.x as i32 + (r.size.width / 2) as i32, Ordering::SeqCst);
                    TRAY_ICON_RECT_Y.store(r.position.y as i32 + (r.size.height / 2) as i32, Ordering::SeqCst);
                }
                tracing::info!("[Tray] TrayIconBuilder::build() returned Ok(tray) - id={:?}, rect={:?}", t.id(), rect);
                t
            }
            Err(e) => {
                tracing::error!("[Tray] TrayIconBuilder::build() FAILED: {e:#}");
                return Err(e.into());
            }
        };

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
            last_airpods_sig: None,
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
        let is_airpods_connected = state.is_airpods_connected();
        let current_airpods_sig = (
            state.airpods_detected,
            is_airpods_connected,
            state.airpods_battery_left,
            state.airpods_battery_right,
            state.airpods_battery_case,
            state.airpods_charging_left,
            state.airpods_charging_right,
            state.airpods_charging_case,
            state.airpods_left_in_ear,
            state.airpods_right_in_ear,
        );
        let airpods_changed = self.last_airpods_sig.as_ref() != Some(&current_airpods_sig);

        // Avoid re-creating GDI icons and updating menus if status hasn't changed
        if !status_changed && !muted_changed && !name_changed && !notice_changed && !volume_changed && !lang_changed && !airpods_changed {
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

        if status_changed || name_changed || volume_changed || muted_changed || lang_changed || airpods_changed {
            self.last_status = Some(state.status.clone());
            self.last_name = state.connected_name.clone();
            self.last_volume = Some(state.volume);
            self.last_muted = Some(state.is_muted);
            self.last_airpods_sig = Some(current_airpods_sig);

            let vol_pct = (state.volume * 100.0).round() as i32;

            let airpods_summary = if is_airpods_connected && state.airpods_detected {
                let is_in_use = match &state.status {
                    DeviceStatus::AoMode(ref dev_name) => {
                        crate::controller::is_airpods_playback_device(dev_name)
                            || crate::controller::is_airpods_playback_device(&state.ao_output_device)
                    }
                    _ => false,
                };
                let status_tag = if is_in_use {
                    state.language.airpods_in_use_status_tag()
                } else {
                    state.language.airpods_connected_status_tag()
                };

                let mut bat = Vec::new();
                if let Some(l) = state.airpods_battery_left {
                    let chg = if state.airpods_charging_left { "⚡" } else { "" };
                    let wear = if state.airpods_left_in_ear { "● " } else { "" };
                    bat.push(format!("{wear}L {l}%{chg}"));
                }
                if let Some(r) = state.airpods_battery_right {
                    let chg = if state.airpods_charging_right { "⚡" } else { "" };
                    let wear = if state.airpods_right_in_ear { "● " } else { "" };
                    bat.push(format!("{wear}R {r}%{chg}"));
                }
                if let Some(c) = state.airpods_battery_case {
                    let chg = if state.airpods_charging_case { "⚡" } else { "" };
                    bat.push(format!("🔋{c}%{chg}"));
                }
                let model_name = state.airpods_model.as_deref().unwrap_or("AirPods");
                if bat.is_empty() {
                    Some(state.language.tray_airpods_status_only(model_name, status_tag))
                } else {
                    Some(state.language.tray_airpods_with_battery(model_name, &bat.join("  ")))
                }
            } else {
                None
            };

            let tooltip = match &state.status {
                DeviceStatus::Streaming => {
                    let name = state.connected_name.as_deref().unwrap_or("");
                    let base = state.language.tray_streaming_tooltip(state.is_muted, name, vol_pct);
                    if let Some(ref ap) = airpods_summary {
                        format!("{base}\r\n{ap}")
                    } else {
                        base
                    }
                }
                DeviceStatus::Connecting => {
                    let name = state.connected_name.as_deref().unwrap_or("");
                    let base = state.language.tray_connecting_tooltip(name);
                    if let Some(ref ap) = airpods_summary {
                        format!("{base}\r\n{ap}")
                    } else {
                        base
                    }
                }
                DeviceStatus::AoMode(_) => {
                    let base = format!("Win2Pod - {}", state.language.status_ao_active());
                    if let Some(ref ap) = airpods_summary {
                        format!("{base}\r\n{ap}")
                    } else {
                        base
                    }
                }
                DeviceStatus::Idle => {
                    let mut idle_txt = state.language.tray_idle_tooltip().to_string();
                    if let Some((ref notice, time)) = state.notice_message {
                        if time.elapsed().as_secs() < 60 {
                            idle_txt = format!("Win2Pod Bridge - {}（{}）", state.language.status_idle(), notice.render(state.language));
                        }
                    }
                    if let Some(ref ap) = airpods_summary {
                        format!("{idle_txt}\r\n{ap}")
                    } else {
                        idle_txt
                    }
                }
                DeviceStatus::Error(e) => {
                    let base = state.language.tray_error_tooltip(e);
                    if let Some(ref ap) = airpods_summary {
                        format!("{base}\r\n{ap}")
                    } else {
                        base
                    }
                }
            };

            let icon = load_tray_icon(&state.status, state.is_muted);
            if let Err(e) = self.tray.set_icon(Some(icon)) {
                tracing::warn!("[Tray] set_icon returned error: {e:#}");
            } else {
                let rect = self.tray.rect();
                if let Some(r) = rect {
                    TRAY_ICON_RECT_X.store(r.position.x as i32 + (r.size.width / 2) as i32, Ordering::SeqCst);
                    TRAY_ICON_RECT_Y.store(r.position.y as i32 + (r.size.height / 2) as i32, Ordering::SeqCst);
                }
                tracing::info!("[Tray] set_icon updated successfully (status={:?}, muted={}, rect={:?})", state.status, state.is_muted, rect);
            }
            if let Err(e) = self.tray.set_tooltip(Some(&tooltip)) {
                tracing::warn!("[Tray] set_tooltip returned error: {e:#}");
            }
        }
    }
}

const TRAY_IDLE_BYTES: &[u8] = include_bytes!("../assets/tray_idle.png");
const TRAY_STREAMING_BYTES: &[u8] = include_bytes!("../assets/tray_streaming.png");
const TRAY_CONNECTING_BYTES: &[u8] = include_bytes!("../assets/tray_connecting.png");
const TRAY_ERROR_BYTES: &[u8] = include_bytes!("../assets/tray_error.png");
const TRAY_MUTED_BYTES: &[u8] = include_bytes!("../assets/tray_muted.png");

/// Load specialized Win2Pod brand tray icon based on device connection status and mute state
pub fn load_tray_icon(status: &DeviceStatus, is_muted: bool) -> Icon {
    let (name, bytes) = match status {
        DeviceStatus::Streaming if is_muted => ("tray_muted.png", TRAY_MUTED_BYTES),
        DeviceStatus::Streaming => ("tray_streaming.png", TRAY_STREAMING_BYTES),
        DeviceStatus::AoMode(_) if is_muted => ("tray_muted.png", TRAY_MUTED_BYTES),
        DeviceStatus::AoMode(_) => ("tray_streaming.png", TRAY_STREAMING_BYTES),
        DeviceStatus::Connecting => ("tray_connecting.png", TRAY_CONNECTING_BYTES),
        DeviceStatus::Idle => ("tray_idle.png", TRAY_IDLE_BYTES),
        DeviceStatus::Error(_) => ("tray_error.png", TRAY_ERROR_BYTES),
    };

    match image::load_from_memory(bytes) {
        Ok(img) => {
            let rgba = img.into_rgba8();
            let (w, h) = rgba.dimensions();
            match Icon::from_rgba(rgba.into_raw(), w, h) {
                Ok(icon) => {
                    tracing::info!("[Tray] Loaded PNG icon '{name}' ({w}x{h}) successfully");
                    return icon;
                }
                Err(e) => {
                    tracing::warn!("[Tray] Icon::from_rgba failed for '{name}': {e:#}");
                }
            }
        }
        Err(e) => {
            tracing::warn!("[Tray] image::load_from_memory failed for '{name}': {e:#}");
        }
    }

    tracing::warn!("[Tray] Falling back to create_color_icon for status={:?}", status);
    if *status == DeviceStatus::Streaming && is_muted {
        create_color_icon(240, 140, 40)
    } else {
        create_color_icon(140, 140, 140)
    }
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
                    trigger_app_quit(&cmd_tx_menu);
                } else if event.id == show_id {
                    center_and_open_main_window(&egui_ctx_menu);
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
