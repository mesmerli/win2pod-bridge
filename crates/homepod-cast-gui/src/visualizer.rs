use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use std::borrow::Cow;
use std::num::NonZeroIsize;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::thread;
use wry::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use wry::WebViewBuilder;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static VISUALIZER_HWND: AtomicIsize = AtomicIsize::new(0);
static VISUALIZER_FULLSCREEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(crate) static VISUALIZER_HUD_MAIN_SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static VISUALIZER_DEV_NAME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static VISUALIZER_LANG: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static APPLIED_LANG: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static HUD_SAVED_PIN_STATES: std::sync::Mutex<Vec<(crate::snapping::WindowKind, bool)>> =
    std::sync::Mutex::new(Vec::new());

pub fn restore_hud_pinned_states() {
    if let Ok(mut saved_pins) = HUD_SAVED_PIN_STATES.lock() {
        for (m, was_pinned) in saved_pins.drain(..) {
            crate::snapping::set_window_pinned(m, was_pinned);
            unsafe {
                crate::snapping::apply_window_topmost(m, was_pinned);
            }
        }
    }
    if let Some(ctx) = crate::gui::get_egui_ctx() {
        ctx.request_repaint();
    }
}

pub fn reset_hud_main_shown() {
    VISUALIZER_HUD_MAIN_SHOWN.store(false, Ordering::SeqCst);
}

pub fn is_hud_shown() -> bool {
    VISUALIZER_HUD_MAIN_SHOWN.load(Ordering::SeqCst)
}

/// 取得當前全螢幕 HUD 浮層所涵蓋的所有視窗（包括主視窗，以及任何已開啟/可見的子視窗）。
/// 不論它們是否吸附在一起，凡是在全螢幕之上運作的 HUD 浮層視窗，全體受 HUD 機制保護，
/// 絕不因使用者將子視窗拖離主視窗而在點擊視覺特效時被全螢幕蓋過。
pub fn get_all_hud_target_windows() -> Vec<crate::snapping::WindowKind> {
    let mut targets = Vec::new();
    targets.push(crate::snapping::WindowKind::Main);

    for &k in &[crate::snapping::WindowKind::MiniPlayer, crate::snapping::WindowKind::Equalizer] {
        let hwnd = crate::snapping::get_window_hwnd(k);
        if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
            targets.push(k);
        }
    }

    #[cfg(test)]
    {
        let keys = crate::snapping::with_dock_tree(|t| t.keys().cloned().collect::<Vec<_>>());
        for k in keys {
            if k != crate::snapping::WindowKind::Visualizer && !targets.contains(&k) {
                targets.push(k);
            }
        }
    }

    targets
}

pub fn hide_hud_cluster() {
    VISUALIZER_HUD_MAIN_SHOWN.store(false, Ordering::SeqCst);

    let vis_hwnd = get_visualizer_hwnd();

    unsafe {
        // 1. 全螢幕視覺視窗本身維持最高層級 HWND_TOPMOST 並取回焦點
        if !vis_hwnd.is_null() && IsWindow(vis_hwnd) != 0 {
            SetWindowPos(
                vis_hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            crate::tray::force_foreground(vis_hwnd);
            PostMessageW(vis_hwnd, WM_USER_FOCUS_WEBVIEW, 0, 0);
        }

        // 2. 還原原始釘選狀態，並將所有 HUD 浮層視窗（無論黏著還是拉開）收納至全螢幕視覺視窗背後
        let members = get_all_hud_target_windows();
        let mut restored_pins = Vec::new();
        if let Ok(mut saved_pins) = HUD_SAVED_PIN_STATES.lock() {
            for &m in &members {
                if let Some(pos) = saved_pins.iter().position(|(k, _)| *k == m) {
                    restored_pins.push(saved_pins.remove(pos));
                }
            }
        }

        for &m in &members {
            let was_pinned = restored_pins
                .iter()
                .find(|(k, _)| *k == m)
                .map(|(_, p)| *p)
                .unwrap_or_else(|| crate::snapping::is_window_pinned(m));
            crate::snapping::set_window_pinned(m, was_pinned);

            let hwnd = crate::snapping::get_window_hwnd(m);
            if !hwnd.is_null() && IsWindow(hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
                if !was_pinned {
                    SetWindowPos(
                        hwnd,
                        HWND_NOTOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                if !vis_hwnd.is_null() && IsWindow(vis_hwnd) != 0 {
                    SetWindowPos(
                        hwnd,
                        vis_hwnd,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }

    if let Some(ctx) = crate::gui::get_egui_ctx() {
        ctx.request_repaint();
    }
}

pub fn show_hud_cluster() {
    VISUALIZER_HUD_MAIN_SHOWN.store(true, Ordering::SeqCst);

    let members = get_all_hud_target_windows();

    // 備份進入全螢幕 HUD 前的原始釘選狀態，供退出 HUD 或全螢幕時精確還原
    if let Ok(mut saved_pins) = HUD_SAVED_PIN_STATES.lock() {
        for &m in &members {
            if !saved_pins.iter().any(|(k, _)| *k == m) {
                saved_pins.push((m, crate::snapping::is_window_pinned(m)));
            }
        }
    }

    // 全體設為釘選（作用中）
    for &m in &members {
        crate::snapping::set_window_pinned(m, true);
    }

    let main_hwnd = crate::tray::get_main_hwnd();
    if !main_hwnd.is_null() && unsafe { IsWindow(main_hwnd) != 0 } {
        unsafe {
            // 1. 若主視窗在托盤，先還原
            if crate::tray::is_window_hidden_to_tray() {
                crate::tray::bring_main_to_front();
            }
            if IsIconic(main_hwnd) != 0 {
                ShowWindow(main_hwnd, SW_RESTORE);
                crate::snapping::finish_cluster_restore(crate::snapping::WindowKind::Main);
            }

            // 2. 一次到位（Single-Shot）：以 SWP_NOACTIVATE 一步鎖定至 HWND_TOPMOST
            for &m in &members {
                let hwnd = crate::snapping::get_window_hwnd(m);
                if !hwnd.is_null() && IsWindow(hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
                    if IsIconic(hwnd) != 0 {
                        ShowWindow(hwnd, SW_RESTORE);
                    }
                    SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    );
                }
            }

            // 3. 維護群組內部相對幾何對齊
            crate::snapping::restore_and_realign_all_children(crate::snapping::WindowKind::Main, main_hwnd);

            // 提案一：全螢幕主控權保持制（HUD 召喚時焦點始終鎖定留在全螢幕視覺視窗上）
            // 絕不調用 SetForegroundWindow(main_hwnd) 或 egui Focus 奪走焦點，
            // 確保視覺視窗自始至終擁有鍵盤輸入焦點，按一次 W 展開、再按一次 W 即可瞬間收起。
            let vis_hwnd = get_visualizer_hwnd();
            if !vis_hwnd.is_null() && IsWindow(vis_hwnd) != 0 {
                crate::tray::force_foreground(vis_hwnd);
                PostMessageW(vis_hwnd, WM_USER_FOCUS_WEBVIEW, 0, 0);
            }
        }
    }

    if let Some(ctx) = crate::gui::get_egui_ctx() {
        ctx.request_repaint();
    }
}

pub fn exit_visualizer_fullscreen() {
    if is_visualizer_fullscreen() {
        let hwnd = get_visualizer_hwnd();
        if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
            unsafe {
                PostMessageW(hwnd, WM_USER_TOGGLE_FULLSCREEN, 0, 0);
            }
        }
    }
}

pub fn toggle_main_window_hud() {
    let main_hwnd = crate::tray::get_main_hwnd();
    let is_hidden = main_hwnd.is_null()
        || unsafe { IsWindow(main_hwnd) == 0 || IsWindowVisible(main_hwnd) == 0 }
        || crate::tray::is_window_hidden_to_tray();

    if !is_hidden && VISUALIZER_HUD_MAIN_SHOWN.load(Ordering::SeqCst) {
        hide_hud_cluster();
    } else {
        show_hud_cluster();
    }
}

pub fn set_language(lang_code: &str) {
    if let Ok(mut lock) = VISUALIZER_LANG.lock() {
        *lock = Some(lang_code.to_string());
    }
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst);
    if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
        unsafe {
            PostMessageW(hwnd as HWND, WM_USER_SET_LANGUAGE, 0, 0);
        }
    }
}

pub fn update_device_info(name: &str) {
    if let Ok(mut lock) = VISUALIZER_DEV_NAME.lock() {
        *lock = Some(name.to_string());
    }
}

pub struct HwndWrapper(pub isize);

impl HasWindowHandle for HwndWrapper {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = Win32WindowHandle::new(NonZeroIsize::new(self.0).unwrap());
        let raw = RawWindowHandle::Win32(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw)) }
    }
}

fn get_embedded_file(path: &str) -> Option<(&'static str, &'static [u8])> {
    let clean = path.trim_start_matches('/');
    match clean {
        "" | "index.html" => Some(("text/html; charset=utf-8", include_bytes!("../assets/visualizer/index.html"))),
        "packages/baked-debris/src/index.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/baked-debris/src/index.js"))),
        "packages/baked-debris/src/bake.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/baked-debris/src/bake.js"))),
        "packages/baked-debris/src/BakedDebris.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/baked-debris/src/BakedDebris.js"))),
        "packages/baked-debris/src/colliders.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/baked-debris/src/colliders.js"))),
        "packages/baked-debris/src/meshToPieces.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/baked-debris/src/meshToPieces.js"))),
        "packages/baked-debris/src/shaders.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/baked-debris/src/shaders.js"))),
        "packages/painterly-pass/src/index.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/painterly-pass/src/index.js"))),
        "packages/painterly-pass/src/PainterlyPass.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/painterly-pass/src/PainterlyPass.js"))),
        "packages/painterly-pass/src/shaders.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/packages/painterly-pass/src/shaders.js"))),
        "vendor/three.module.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/vendor/three.module.js"))),
        "vendor/controls/OrbitControls.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/vendor/controls/OrbitControls.js"))),
        "vendor/postprocessing/Pass.js" => Some(("application/javascript; charset=utf-8", include_bytes!("../assets/visualizer/vendor/postprocessing/Pass.js"))),
        _ => None,
    }
}

static VISUALIZER_HIDDEN_WITH_CLUSTER: AtomicBool = AtomicBool::new(false);
static VISUALIZER_CLOSING: AtomicBool = AtomicBool::new(false);

pub fn is_visualizer_hidden_with_cluster() -> bool {
    VISUALIZER_HIDDEN_WITH_CLUSTER.load(Ordering::SeqCst)
}

pub fn hide_visualizer_with_cluster() {
    let hwnd = get_visualizer_hwnd();
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        VISUALIZER_HIDDEN_WITH_CLUSTER.store(true, Ordering::SeqCst);
        unsafe {
            KillTimer(hwnd, TIMER_AUDIO_SPECTRUM);
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn restore_visualizer_with_cluster() {
    VISUALIZER_HIDDEN_WITH_CLUSTER.store(false, Ordering::SeqCst);
    let hwnd = get_visualizer_hwnd();
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 16, None);
        }
    }
}

pub fn is_visualizer_visible() -> bool {
    let hwnd = get_visualizer_hwnd();
    !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 }
}

pub fn is_visualizer_open() -> bool {
    if VISUALIZER_CLOSING.load(Ordering::SeqCst) {
        return false;
    }
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst);
    hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 }
}

pub fn get_visualizer_hwnd() -> HWND {
    if VISUALIZER_CLOSING.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst);
    if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
        hwnd as HWND
    } else {
        std::ptr::null_mut()
    }
}

pub fn is_visualizer_fullscreen() -> bool {
    VISUALIZER_FULLSCREEN.load(Ordering::SeqCst)
}

pub const VISUALIZER_WINDOW_STYLE: u32 = WS_POPUP
    | WS_THICKFRAME
    | WS_MINIMIZEBOX;

/// 套用 Windows DWM 沉浸式深色標題列樣式，並設定標題列底色與文字顏色，使其外觀與主視窗及其他模組保持高度一致。
pub unsafe fn apply_visualizer_theme(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    type DwmSetWindowAttributeFn = unsafe extern "system" fn(HWND, u32, *const std::ffi::c_void, u32) -> i32;
    type DwmExtendFrameIntoClientAreaFn = unsafe extern "system" fn(HWND, *const MARGINS) -> i32;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct MARGINS {
        cxLeftWidth: i32,
        cxRightWidth: i32,
        cyTopHeight: i32,
        cyBottomHeight: i32,
    }

    let dwmapi = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(b"dwmapi.dll\0".as_ptr());
    if !dwmapi.is_null() {
        let extend_proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(dwmapi, b"DwmExtendFrameIntoClientArea\0".as_ptr());
        if let Some(extend_fn) = extend_proc {
            let func: DwmExtendFrameIntoClientAreaFn = std::mem::transmute(extend_fn);
            let m = MARGINS { cxLeftWidth: 1, cxRightWidth: 1, cyTopHeight: 1, cyBottomHeight: 1 };
            func(hwnd, &m);
        }

        let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(dwmapi, b"DwmSetWindowAttribute\0".as_ptr());
        if let Some(dwm_set_attr) = proc {
            let func: DwmSetWindowAttributeFn = std::mem::transmute(dwm_set_attr);

            // 1. DWMWA_USE_IMMERSIVE_DARK_MODE (20 on Win10 20H1+ and Win11, 19 on older Win10)
            let dark_mode: i32 = 1;
            func(hwnd, 20, &dark_mode as *const _ as _, std::mem::size_of::<i32>() as u32);
            func(hwnd, 19, &dark_mode as *const _ as _, std::mem::size_of::<i32>() as u32);

            // 2. DWMWA_CAPTION_COLOR = 35 (Windows 11)
            let caption_color: u32 = 0x001D1918;
            func(hwnd, 35, &caption_color as *const _ as _, std::mem::size_of::<u32>() as u32);

            // 3. DWMWA_TEXT_COLOR = 36 (Windows 11)
            let text_color: u32 = 0x00F5DCB4;
            func(hwnd, 36, &text_color as *const _ as _, std::mem::size_of::<u32>() as u32);
        }
    }
}

/// 更新視覺效果視窗的標題列文字，依據磁吸主從角色附加身分符號（主控 🧲、附屬 📎、獨立無符號）
pub unsafe fn update_visualizer_title(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
    let (title_str, badge_str) = match role {
        crate::snapping::WindowRole::ClusterMaster => ("Win2Pod Motion - Paint Pulse 🧲\0", "🧲"),
        crate::snapping::WindowRole::ClusterSlave => ("Win2Pod Motion - Paint Pulse 📎\0", "📎"),
        crate::snapping::WindowRole::Standalone => ("Win2Pod Motion - Paint Pulse\0", ""),
    };
    let title_wide: Vec<u16> = title_str.encode_utf16().collect();
    SetWindowTextW(hwnd, title_wide.as_ptr());

    // 檢查當前執行緒是否為視覺視窗所在的 Win32 訊息迴圈執行緒
    let window_tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
    let current_tid = windows_sys::Win32::System::Threading::GetCurrentThreadId();
    if window_tid == current_tid {
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut VisualizerState;
        if !state_ptr.is_null() {
            if let Some(ref wv) = (*state_ptr).webview {
                let is_docked = crate::snapping::is_main_in_cluster(crate::snapping::WindowKind::Visualizer);
                let script = format!(
                    "if (typeof window.updateTitleBadge === 'function') window.updateTitleBadge({:?});\
                     if (typeof window.handleVisDockChanged === 'function') window.handleVisDockChanged({});",
                    badge_str,
                    if is_docked { "true" } else { "false" }
                );
                let _ = wv.evaluate_script(&script);
            }
        }
    } else {
        // 若來自其他執行緒（例如 egui 主執行緒拖曳迷你播放器完成吸附），透過視窗訊息安全非同步派送至視覺視窗執行緒執行
        PostMessageW(hwnd, WM_USER_UPDATE_TITLE, 0, 0);
    }
}

/// Updates the caption buttons (minimize, close) on the visualizer titlebar.
/// Retains _ and X at all times per user specification, while excluding the maximize box.
pub unsafe fn update_visualizer_dock_buttons(hwnd: HWND, _is_docked: bool) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let new_style = (current_style | WS_SYSMENU | WS_MINIMIZEBOX) & !WS_MAXIMIZEBOX;
    if new_style != current_style {
        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);
        GetSystemMenu(hwnd, 1);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// Temporarily demotes the visualizer window from HWND_TOPMOST to HWND_NOTOPMOST,
/// allowing external media player windows (such as Spotify) to float cleanly in front of it.
pub fn yield_visualizer_topmost() {
    let hwnd = get_visualizer_hwnd();
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

pub fn close_visualizer() {
    VISUALIZER_CLOSING.store(true, Ordering::SeqCst);
    let hwnd = VISUALIZER_HWND.swap(0, Ordering::SeqCst);
    if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
        unsafe {
            PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0);
        }
    }
    crate::spectrum::update_spectrum_needed_state();
}

pub fn set_visualizer_pinned(pinned: bool) {
    let hwnd = get_visualizer_hwnd();
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            PostMessageW(hwnd, WM_USER_SET_PIN, pinned as usize, 0);
        }
    }
}

pub fn toggle_visualizer() {
    if is_visualizer_open() {
        close_visualizer();
    } else {
        open_visualizer();
    }
}

pub fn open_visualizer() {
    let existing = get_visualizer_hwnd();
    if !existing.is_null() && unsafe { IsWindow(existing) != 0 } {
        unsafe {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
        }
        return;
    }

    VISUALIZER_CLOSING.store(false, Ordering::SeqCst);
    thread::spawn(|| {
        unsafe {
            run_visualizer_window();
        }
    });
}

const TIMER_AUDIO_SPECTRUM: usize = 1001;
const WM_USER_TOGGLE_FULLSCREEN: u32 = WM_USER + 102;
const WM_USER_TOGGLE_PIN: u32 = WM_USER + 103;
const WM_USER_SET_LANGUAGE: u32 = WM_USER + 104;
const WM_USER_SET_PIN: u32 = WM_USER + 105;
const WM_USER_FOCUS_WEBVIEW: u32 = WM_USER + 106;
const WM_USER_UPDATE_TITLE: u32 = WM_USER + 107;
const WM_USER_SYNC_OPACITY: u32 = WM_USER + 108;
const WM_USER_WAKE_ECO: u32 = WM_USER + 109;

pub fn sync_visualizer_opacity(pct: u32, show_hud: bool) {
    let hwnd = get_visualizer_hwnd();
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            PostMessageW(hwnd, WM_USER_SYNC_OPACITY, pct as WPARAM, if show_hud { 1 } else { 0 });
        }
    }
}

pub fn sync_visualizer_opacity_hud(pct: u32) {
    sync_visualizer_opacity(pct, true);
}

pub fn focus_visualizer() {
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            PostMessageW(hwnd, WM_USER_FOCUS_WEBVIEW, 0, 0);
        }
    }
}

/// Wakes the visualizer from 30s silence Eco Mode with zero latency.
pub fn wake_visualizer_from_eco() {
    let hwnd = get_visualizer_hwnd();
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            PostMessageW(hwnd, WM_USER_WAKE_ECO, 0, 0);
        }
    }
}

static VISUALIZER_ECO_MODE: AtomicBool = AtomicBool::new(false);
static VIS_HOVERED: AtomicBool = AtomicBool::new(false);

pub unsafe fn apply_visualizer_opacity(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let is_fullscreen = is_visualizer_fullscreen();
    let user_op = crate::snapping::get_window_opacity(crate::snapping::WindowKind::Visualizer);
    let peek_enabled = crate::snapping::is_hover_peek_enabled();
    let is_click_through = crate::snapping::is_click_through_active(crate::snapping::WindowKind::Visualizer);

    crate::snapping::update_window_click_through(crate::snapping::WindowKind::Visualizer, hwnd);

    let is_hovered = (!is_click_through || is_fullscreen) && VIS_HOVERED.load(Ordering::Relaxed);
    let suppress_peek = crate::snapping::is_peek_suppressed(crate::snapping::WindowKind::Visualizer);

    let target_op = if is_fullscreen || (peek_enabled && is_hovered && !suppress_peek) {
        1.0
    } else {
        user_op
    };

    let (effective_op, _animating) = crate::snapping::step_animated_window_opacity(
        crate::snapping::WindowKind::Visualizer,
        target_op,
    );
    crate::snapping::apply_native_window_opacity(hwnd, effective_op);
}

struct VisualizerState {
    webview: Option<wry::WebView>,
    is_fullscreen: bool,
    is_pinned: bool,
    saved_placement: WINDOWPLACEMENT,
}

unsafe fn set_pin(hwnd: HWND, state: &mut VisualizerState, pinned: bool) {
    state.is_pinned = pinned;
    crate::controller::save_visualizer_pinned(state.is_pinned);
    crate::snapping::set_window_pinned(crate::snapping::WindowKind::Visualizer, pinned);
    let target = if state.is_pinned || state.is_fullscreen {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    SetWindowPos(hwnd, target, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    apply_visualizer_opacity(hwnd);

    // If main window has always-on-top enabled, keep main window above visualizer
    if crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main) && !crate::tray::is_window_hidden_to_tray() {
        let main_hwnd = crate::tray::get_main_hwnd();
        if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 {
            SetWindowPos(main_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    if let Some(ref wv) = state.webview {
        let script = format!(
            "if (typeof window.handleVisPinChanged === 'function') window.handleVisPinChanged({}, true);",
            state.is_pinned
        );
        let _ = wv.evaluate_script(&script);
    }
}

unsafe fn toggle_pin(hwnd: HWND, state: &mut VisualizerState) {
    let current_op = (crate::snapping::get_window_opacity(crate::snapping::WindowKind::Visualizer) * 100.0).round() as u32;
    // 狀態機防呆：若視覺視窗處於半透明態 (current_op < 100) 且已置頂，單擊取消釘選代表退出幽靈模式：
    // 自動恢復 100% 實體並取消置頂（回到 State 1 常規視窗），徹底消滅「半透明卻未置頂」的無效死態！
    if state.is_pinned && current_op < 100 {
        crate::snapping::record_ghost_exit();
        crate::snapping::set_single_window_opacity_tuning(crate::snapping::WindowKind::Visualizer, 1.0);
        set_pin(hwnd, state, false);
        return;
    }

    let new_pinned = !state.is_pinned;
    set_pin(hwnd, state, new_pinned);

    // If magnetically docked with cluster, propagate new_pinned to cluster!
    let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
    if role != crate::snapping::WindowRole::Standalone {
        crate::snapping::set_cluster_pinned(crate::snapping::WindowKind::Visualizer, new_pinned);
    }
}

unsafe fn toggle_fullscreen(hwnd: HWND, state: &mut VisualizerState) {
    if !state.is_fullscreen {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let mut placement: WINDOWPLACEMENT = std::mem::zeroed();
        placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        GetWindowPlacement(hwnd, &mut placement);
        state.saved_placement = placement;

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut mi) != 0 {
            // IMPORTANT: Break dock bond and set fullscreen flags BEFORE SetWindowPos!
            // Otherwise, SetWindowPos triggers a synchronous WM_SIZE, which sees is_bonded=true
            // and !is_fullscreen, recalculating bonded main window position relative to the
            // fullscreen visualizer and throwing the main window completely off-screen!
            state.is_fullscreen = true;
            VISUALIZER_FULLSCREEN.store(true, Ordering::SeqCst);
            VISUALIZER_HUD_MAIN_SHOWN.store(false, Ordering::SeqCst);
            VISUALIZER_ECO_MODE.store(false, Ordering::SeqCst);
            SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 16, None);
            crate::snapping::break_dock_bond();

            SetWindowLongW(hwnd, GWL_STYLE, (style & !WS_THICKFRAME) as i32);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                mi.rcMonitor.left,
                mi.rcMonitor.top,
                mi.rcMonitor.right - mi.rcMonitor.left,
                mi.rcMonitor.bottom - mi.rcMonitor.top,
                SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            );
            apply_visualizer_opacity(hwnd);
            crate::controller::send_controller_cmd(crate::controller::GuiCmd::VisualizerFullscreenChanged(true));

            if let Some(ref wv) = state.webview {
                let _ = wv.evaluate_script("document.body.classList.add('fullscreen');");
            }

            hide_hud_cluster();

            // Keep taskbar beneath the fullscreen visualizer
            let taskbar_class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
            let taskbar = FindWindowW(taskbar_class.as_ptr(), std::ptr::null());
            if !taskbar.is_null() {
                SetWindowPos(taskbar, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    } else {
        restore_hud_pinned_states();
        state.is_fullscreen = false;
        VISUALIZER_FULLSCREEN.store(false, Ordering::SeqCst);
        VISUALIZER_HUD_MAIN_SHOWN.store(false, Ordering::SeqCst);
        VISUALIZER_ECO_MODE.store(false, Ordering::SeqCst);
        SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 16, None);
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        SetWindowLongW(hwnd, GWL_STYLE, (style | WS_THICKFRAME) as i32);
        SetWindowPlacement(hwnd, &state.saved_placement);
        let target = if state.is_pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
        SetWindowPos(
            hwnd,
            target,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
        );
        apply_visualizer_opacity(hwnd);
        apply_visualizer_theme(hwnd);
        update_visualizer_title(hwnd);
        if let Some(ref wv) = state.webview {
            let _ = wv.evaluate_script("document.body.classList.remove('fullscreen');");
        }
        crate::controller::send_controller_cmd(crate::controller::GuiCmd::VisualizerFullscreenChanged(false));

        let main_hwnd = crate::tray::get_main_hwnd();
        if !main_hwnd.is_null() {
            const GWLP_HWNDPARENT: i32 = -8;
            if SetWindowLongPtrW(main_hwnd, GWLP_HWNDPARENT, 0) != 0 {
                // Owner cleared
            }
            if !crate::tray::is_window_hidden_to_tray()
                && !crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main)
                && IsIconic(main_hwnd) == 0
            {
                SetWindowPos(main_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }

    if let Some(ref wv) = state.webview {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetClientRect(hwnd, &mut rect);
        let w = (rect.right - rect.left).max(1) as u32;
        let h = (rect.bottom - rect.top).max(1) as u32;
        let _ = wv.set_bounds(wry::Rect {
            position: Position::Physical(PhysicalPosition::new(0, 0)),
            size: Size::Physical(PhysicalSize::new(w, h)),
        });

        let script = format!(
            "if (typeof window.handleFullscreenChanged === 'function') window.handleFullscreenChanged({});",
            state.is_fullscreen
        );
        let _ = wv.evaluate_script(&script);
    }
}

unsafe fn run_visualizer_window() {
    let _ = windows::Win32::System::Com::CoInitializeEx(
        None,
        windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
    );

    let class_name: Vec<u16> = "Win2PodVisualizerWindow\0".encode_utf16().collect();
    let title: Vec<u16> = "Win2Pod Motion - Paint Pulse\0".encode_utf16().collect();
    let hinstance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
    let mut icon = LoadIconW(hinstance, 1 as *const u16);
    if icon.is_null() {
        icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
    }

    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(visualizer_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: icon,
        hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
        hbrBackground: GetStockObject(BLACK_BRUSH as i32) as HBRUSH,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: icon,
    };

    RegisterClassExW(&wnd_class);

    // Initial window size dynamically aligned with Main Window (Player benchmark)
    // Directly dock to the right (or left) of the Main Window upon creation!
    let main_hwnd = crate::tray::get_main_hwnd();
    let has_main = !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && IsWindowVisible(main_hwnd) != 0;

    let def_size = crate::snapping::WindowKind::Visualizer.default_size();
    let w = def_size[0] as i32;
    let h = def_size[1] as i32;

    let (width, height, x, y, chosen_anchor) = if has_main {
        let spawner = crate::snapping::get_window_spawner(crate::snapping::WindowKind::Visualizer);
        let (best_rect, anchor) = crate::snapping::find_best_spawn_rect(crate::snapping::WindowKind::Visualizer, w, h, spawner);
        (
            best_rect.right - best_rect.left,
            best_rect.bottom - best_rect.top,
            best_rect.left,
            best_rect.top,
            anchor,
        )
    } else {
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let init_x = (screen_w - w) / 2;
        let init_y = (screen_h - h) / 2;
        (w, h, init_x, init_y, None)
    };

    let hwnd = CreateWindowExW(
        WS_EX_APPWINDOW,
        class_name.as_ptr(),
        title.as_ptr(),
        VISUALIZER_WINDOW_STYLE | WS_VISIBLE,
        x,
        y,
        width,
        height,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        hinstance,
        std::ptr::null_mut(),
    );

    if hwnd.is_null() {
        tracing::error!("Failed to create visualizer window");
        return;
    }

    apply_visualizer_theme(hwnd);
    update_visualizer_title(hwnd);

    SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
    SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);

    VISUALIZER_HWND.store(hwnd as isize, Ordering::SeqCst);
    crate::spectrum::update_spectrum_needed_state();

    // Directly bind magnetic dock with chosen anchor upon creation!
    if let Some(anchor_kind) = chosen_anchor {
        let anchor_hwnd = crate::snapping::get_window_hwnd(anchor_kind);
        if !anchor_hwnd.is_null() && IsWindow(anchor_hwnd) != 0 {
            let mut actual_vis_rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut actual_vis_rect);
            let mut actual_anchor_rect: RECT = std::mem::zeroed();
            GetWindowRect(anchor_hwnd, &mut actual_anchor_rect);
            crate::snapping::set_dock_relation(crate::snapping::WindowKind::Visualizer, anchor_kind, &actual_vis_rect, &actual_anchor_rect);
        }
    }

    // Create state container
    let mut placement: WINDOWPLACEMENT = std::mem::zeroed();
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;

    let initial_pinned = if let Some(anchor) = chosen_anchor {
        crate::snapping::is_window_pinned(anchor)
    } else {
        crate::controller::load_visualizer_pinned()
    };
    crate::snapping::set_window_pinned(crate::snapping::WindowKind::Visualizer, initial_pinned);
    let state = Box::into_raw(Box::new(VisualizerState {
        webview: None,
        is_fullscreen: false,
        is_pinned: initial_pinned,
        saved_placement: placement,
    }));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);

    let target_z = if initial_pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
    SetWindowPos(hwnd, target_z, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    if initial_pinned {
        if crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main) && !crate::tray::is_window_hidden_to_tray() {
            let main_hwnd = crate::tray::get_main_hwnd();
            if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 {
                SetWindowPos(main_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }

    let wrapper = HwndWrapper(hwnd as isize);
    let hwnd_capture = hwnd as isize;
    let builder = WebViewBuilder::new()
        .with_devtools(true)
        .with_custom_protocol("app".into(), |_id, req| {
            let path = req.uri().path();
            if let Some((content_type, data)) = get_embedded_file(path) {
                wry::http::Response::builder()
                    .header("Content-Type", content_type)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Borrowed(data))
                    .unwrap()
            } else {
                tracing::warn!("Visualizer embedded file not found: {}", path);
                wry::http::Response::builder()
                    .status(404)
                    .body(Cow::Borrowed(b"Not Found" as &[u8]))
                    .unwrap()
            }
        })
        .with_ipc_handler(move |msg| {
            let body = msg.body();
            if body.contains("close_cluster") {
                crate::gui::request_close_cluster(crate::snapping::WindowKind::Visualizer);
            } else if body.contains("close") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_CLOSE, 0, 0);
                }
            } else if body.contains("minimize_solo") {
                unsafe {
                    crate::snapping::minimize_window_solo(crate::snapping::WindowKind::Visualizer);
                }
            } else if body.contains("minimize") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_SYSCOMMAND, SC_MINIMIZE as usize, 0);
                }
            } else if body.contains("start_drag") {
                unsafe {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                    SendMessageW(hwnd_capture as HWND, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
                }
            } else if body.contains("fullscreen") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_TOGGLE_FULLSCREEN, 0, 0);
                }
            } else if body.contains("toggle_pin") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_TOGGLE_PIN, 0, 0);
                }
            } else if body.contains("vis_ready") || body.contains("ready") {
                tracing::info!("visualizer IPC: webview ready signal received");
                unsafe {
                    update_visualizer_title(hwnd_capture as HWND);
                }
            } else if body.contains("show_main") {
                tracing::info!("visualizer IPC: show_main received");
                if crate::visualizer::is_visualizer_fullscreen() {
                    toggle_main_window_hud();
                    return;
                }
                unsafe {
                    crate::snapping::handle_home_summon_request(crate::snapping::WindowKind::Visualizer);
                }
            } else if body.contains("wake_ghost_solo") {
                crate::snapping::wake_single_window_ghost(crate::snapping::WindowKind::Visualizer);
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_SET_PIN, 1, 0);
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.contains("wake_ghost") {
                let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
                if role == crate::snapping::WindowRole::Standalone {
                    crate::snapping::wake_single_window_ghost(crate::snapping::WindowKind::Visualizer);
                } else {
                    crate::snapping::wake_cluster_ghost(crate::snapping::WindowKind::Visualizer);
                }
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_SET_PIN, 1, 0);
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.contains("toggle_opacity_solo") {
                let _next_pct = crate::snapping::toggle_single_window_opacity_quick(crate::snapping::WindowKind::Visualizer);
                let is_pinned = crate::snapping::is_window_pinned(crate::snapping::WindowKind::Visualizer);
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_SET_PIN, if is_pinned { 1 } else { 0 }, 0);
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.contains("toggle_opacity") {
                let _next_pct = crate::snapping::toggle_cluster_opacity_quick(crate::snapping::WindowKind::Visualizer);
                let is_pinned = crate::snapping::is_window_pinned(crate::snapping::WindowKind::Visualizer);
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_SET_PIN, if is_pinned { 1 } else { 0 }, 0);
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.starts_with("set_opacity_solo:") {
                let val_str = body.trim_start_matches("set_opacity_solo:");
                if let Ok(pct) = val_str.parse::<f32>() {
                    let op = (pct / 100.0).clamp(0.2, 1.0);
                    crate::snapping::set_single_window_opacity_tuning(crate::snapping::WindowKind::Visualizer, op);
                    if op < 1.0 && !crate::snapping::is_window_pinned(crate::snapping::WindowKind::Visualizer) {
                        unsafe {
                            PostMessageW(hwnd_capture as HWND, WM_USER_SET_PIN, 1, 0);
                        }
                    }
                    unsafe {
                        apply_visualizer_opacity(hwnd_capture as HWND);
                    }
                }
            } else if body.starts_with("set_opacity:") {
                let val_str = body.trim_start_matches("set_opacity:");
                if let Ok(pct) = val_str.parse::<f32>() {
                    let op = (pct / 100.0).clamp(0.2, 1.0);
                    crate::snapping::set_cluster_opacity(crate::snapping::WindowKind::Visualizer, op);
                    if op < 1.0 && !crate::snapping::is_window_pinned(crate::snapping::WindowKind::Visualizer) {
                        unsafe {
                            PostMessageW(hwnd_capture as HWND, WM_USER_SET_PIN, 1, 0);
                        }
                    }
                    unsafe {
                        apply_visualizer_opacity(hwnd_capture as HWND);
                    }
                }
            } else if body.starts_with("opacity_menu_open:") {
                let val_str = body.trim_start_matches("opacity_menu_open:");
                let is_open = val_str == "1" || val_str == "true";
                crate::snapping::set_opacity_menu_open(crate::snapping::WindowKind::Visualizer, is_open);
                unsafe {
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.starts_with("set_hover_peek:") {
                let val_str = body.trim_start_matches("set_hover_peek:");
                let enabled = val_str == "1" || val_str == "true";
                crate::snapping::set_hover_peek_enabled(enabled);
                unsafe {
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.starts_with("suppress_peek:") {
                let val_str = body.trim_start_matches("suppress_peek:");
                let suppress = val_str == "1" || val_str == "true";
                if suppress {
                    crate::snapping::trigger_cluster_opacity_tuning(crate::snapping::WindowKind::Visualizer);
                }
                unsafe {
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.contains("vis_mouse_enter") {
                VIS_HOVERED.store(true, Ordering::SeqCst);
                unsafe {
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            } else if body.contains("vis_mouse_leave") {
                VIS_HOVERED.store(false, Ordering::SeqCst);
                unsafe {
                    apply_visualizer_opacity(hwnd_capture as HWND);
                }
            }
        });
        let lang_code = VISUALIZER_LANG
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| crate::controller::load_language().code().to_string());
        let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
        let badge_query = match role {
            crate::snapping::WindowRole::ClusterMaster => "%F0%9F%A7%B2",
            crate::snapping::WindowRole::ClusterSlave => "%F0%9F%93%8E",
            crate::snapping::WindowRole::Standalone => "",
        };
        let init_opacity = crate::snapping::get_window_opacity(crate::snapping::WindowKind::Visualizer);
        let init_peek = crate::snapping::is_hover_peek_enabled();
        let init_docked = crate::snapping::is_main_in_cluster(crate::snapping::WindowKind::Visualizer);
        let initial_url = format!(
            "app://visualizer/index.html?lang={}&pinned={}&badge={}&opacity={}&peek={}&docked={}",
            lang_code,
            if initial_pinned { 1 } else { 0 },
            badge_query,
            (init_opacity * 100.0).round() as i32,
            if init_peek { 1 } else { 0 },
            if init_docked { 1 } else { 0 },
        );
        let builder = builder.with_url(&initial_url).with_transparent(true);

    match builder.build(&wrapper) {
        Ok(wv) => {
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);
            let w = (rect.right - rect.left).max(1) as u32;
            let h = (rect.bottom - rect.top).max(1) as u32;
            let _ = wv.set_bounds(wry::Rect {
                position: Position::Physical(PhysicalPosition::new(0, 0)),
                size: Size::Physical(PhysicalSize::new(w, h)),
            });

            let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
            let badge_str = match role {
                crate::snapping::WindowRole::ClusterMaster => "🧲",
                crate::snapping::WindowRole::ClusterSlave => "📎",
                crate::snapping::WindowRole::Standalone => "",
            };
            let script = format!(
                "if (typeof window.setLanguage === 'function') window.setLanguage({:?});\
                 if (typeof window.handleVisPinChanged === 'function') window.handleVisPinChanged({}, false);\
                 if (typeof window.updateTitleBadge === 'function') window.updateTitleBadge({:?});\
                 if (typeof window.handleVisDockChanged === 'function') window.handleVisDockChanged({}, false);",
                lang_code,
                if initial_pinned { "true" } else { "false" },
                badge_str,
                if init_docked { "true" } else { "false" }
            );
            let _ = wv.evaluate_script(&script);
            if let Ok(mut applied) = APPLIED_LANG.lock() {
                *applied = Some(lang_code.clone());
            }

            (*state).webview = Some(wv);

            // Start 60 FPS audio spectrum sync timer (~16ms)
            SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 16, None);
        }
        Err(e) => {
            tracing::error!("Failed to initialize WebView2 in visualizer: {:?}", e);
        }
    }

    let mut msg = MSG {
        hwnd: std::ptr::null_mut(),
        message: 0,
        wParam: 0,
        lParam: 0,
        time: 0,
        pt: POINT { x: 0, y: 0 },
    };

    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    VISUALIZER_HWND.store(0, Ordering::SeqCst);
    VISUALIZER_CLOSING.store(false, Ordering::SeqCst);
    windows::Win32::System::Com::CoUninitialize();
}

unsafe extern "system" fn visualizer_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut VisualizerState;

    match msg {
        WM_USER_TOGGLE_FULLSCREEN => {
            if !state_ptr.is_null() {
                toggle_fullscreen(hwnd, &mut *state_ptr);
            }
            0
        }
        WM_USER_TOGGLE_PIN => {
            if !state_ptr.is_null() {
                toggle_pin(hwnd, &mut *state_ptr);
            }
            0
        }
        WM_USER_SET_PIN => {
            if !state_ptr.is_null() {
                set_pin(hwnd, &mut *state_ptr, wparam != 0);
            }
            0
        }
        WM_USER_FOCUS_WEBVIEW => {
            if !state_ptr.is_null() {
                if let Some(ref wv) = (*state_ptr).webview {
                    let _ = wv.focus();
                    let _ = wv.evaluate_script("window.focus();");
                }
            }
            0
        }
        WM_USER_SET_LANGUAGE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Some(ref wv) = state.webview {
                    if let Ok(guard) = VISUALIZER_LANG.lock() {
                        if let Some(ref current) = *guard {
                            if let Ok(mut applied) = APPLIED_LANG.lock() {
                                *applied = Some(current.clone());
                            }
                            let script = format!(
                                "if (typeof window.setLanguage === 'function') window.setLanguage({:?});",
                                current
                            );
                            let _ = wv.evaluate_script(&script);
                        }
                    }
                }
            }
            0
        }
        WM_USER_UPDATE_TITLE => {
            unsafe {
                update_visualizer_title(hwnd);
            }
            0
        }
        WM_USER_SYNC_OPACITY => {
            let pct = wparam as u32;
            let show_hud = lparam != 0;
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut VisualizerState };
            if !state_ptr.is_null() {
                unsafe {
                    if let Some(ref wv) = (*state_ptr).webview {
                        let script = format!(
                            "if (typeof window.syncOpacityFromHost === 'function') window.syncOpacityFromHost({pct}, {show_hud});"
                        );
                        let _ = wv.evaluate_script(&script);
                    }
                }
            }
            0
        }
        WM_USER_WAKE_ECO => {
            if VISUALIZER_ECO_MODE.swap(false, Ordering::SeqCst) {
                SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 16, None);
                if !state_ptr.is_null() {
                    if let Some(ref wv) = (*state_ptr).webview {
                        let _ = wv.evaluate_script("if (typeof window.setEcoMode === 'function') window.setEcoMode(false);");
                    }
                }
            }
            0
        }
        WM_SETCURSOR => {
            if let Some(hcursor) = crate::custom_cursor::current_custom_cursor(crate::snapping::WindowKind::Visualizer) {
                SetCursor(hcursor);
                return 1;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOVING => {
            crate::custom_cursor::set_window_moving(crate::snapping::WindowKind::Visualizer, true);
            if !state_ptr.is_null() && !(*state_ptr).is_fullscreen {
                let rect_ptr = lparam as *mut RECT;
                unsafe {
                    crate::snapping::handle_window_moving(
                        crate::snapping::WindowKind::Visualizer,
                        hwnd,
                        rect_ptr,
                    );
                }
                return 1;
            }
            0
        }
        WM_EXITSIZEMOVE => {
            crate::custom_cursor::set_window_moving(crate::snapping::WindowKind::Visualizer, false);
            if !state_ptr.is_null() && !(*state_ptr).is_fullscreen {
                unsafe {
                    crate::snapping::handle_window_exit_size_move(
                        crate::snapping::WindowKind::Visualizer,
                        hwnd,
                    );
                }
            }
            0
        }
        WM_CAPTURECHANGED => {
            crate::custom_cursor::set_window_moving(crate::snapping::WindowKind::Visualizer, false);
            0
        }
        WM_WINDOWPOSCHANGING => {
            if !state_ptr.is_null() && (*state_ptr).is_fullscreen && is_hud_shown() {
                let wp = lparam as *mut WINDOWPOS;
                if !wp.is_null() {
                    // 全螢幕 HUD 作用中：視覺視窗作為最底層背景畫布，鎖死其在 Z-order 中的地位！
                    // 嚴禁 Windows 點擊提升將視覺視窗提升至 HUD 浮層視窗之上，直接阻斷任何 Z-order 變更
                    (*wp).flags |= SWP_NOZORDER;
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEACTIVATE => {
            if crate::snapping::SNAPPING_ENABLED.load(Ordering::Relaxed)
                && !crate::snapping::is_cluster_minimizing()
                && !crate::snapping::is_cluster_restoring()
                && IsIconic(hwnd) == 0
            {
                let is_fs = !state_ptr.is_null() && (*state_ptr).is_fullscreen;
                if is_fs && is_hud_shown() {
                    // 全螢幕 HUD 開啟中：視覺視窗作為背景畫布，絕對不搶頂
                    // 檢查滑鼠座標是否在右上角控制按鈕區（例如關閉 HUD 房子按鈕、全螢幕按鈕）：
                    // 若在背景畫布，回傳 MA_NOACTIVATEANDEAT (4) 徹底丟棄啟用與點擊事件，
                    // 杜絕 WebView2 獲取焦點或 Windows DWM 重排 Z-order，達成 100% 零閃爍！
                    let mut pt: POINT = std::mem::zeroed();
                    GetCursorPos(&mut pt);
                    ScreenToClient(hwnd, &mut pt);
                    let mut client_rect: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut client_rect);
                    let width = client_rect.right - client_rect.left;
                    let is_top_right_controls = pt.y >= 0 && pt.y <= 70 && pt.x >= (width - 160);
                    if is_top_right_controls {
                        return 3; // MA_NOACTIVATE
                    } else {
                        return 4; // MA_NOACTIVATEANDEAT
                    }
                }
                crate::snapping::bring_cluster_to_top(crate::snapping::WindowKind::Visualizer, hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_NCLBUTTONDOWN => {
            if crate::snapping::SNAPPING_ENABLED.load(Ordering::Relaxed)
                && !crate::snapping::is_cluster_minimizing()
                && !crate::snapping::is_cluster_restoring()
                && IsIconic(hwnd) == 0
            {
                let is_fs = !state_ptr.is_null() && (*state_ptr).is_fullscreen;
                if is_fs && is_hud_shown() {
                    // 全螢幕 HUD 開啟中：禁止將視覺視窗提升至最頂層
                    return 0;
                }
                crate::snapping::bring_cluster_to_top(crate::snapping::WindowKind::Visualizer, hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_ACTIVATE => {
            let active_type = (wparam & 0xFFFF) as u32;
            let is_minimized = ((wparam >> 16) & 0xFFFF) != 0;
            if active_type != 0
                && !is_minimized
                && IsIconic(hwnd) == 0
                && !crate::snapping::is_cluster_minimizing()
                && !crate::snapping::is_cluster_restoring()
            {
                let is_fs = !state_ptr.is_null() && (*state_ptr).is_fullscreen;
                if is_fs && is_hud_shown() {
                    // 全螢幕 HUD 作用中：視覺視窗獲得啟用時，加固維持所有 HUD 浮層視窗（無論黏著還是拉開）在最頂層
                    let members = get_all_hud_target_windows();
                    for &m in &members {
                        let m_hwnd = crate::snapping::get_window_hwnd(m);
                        if !m_hwnd.is_null() && IsWindow(m_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
                            SetWindowPos(m_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        }
                    }
                    return 0;
                }
                if crate::snapping::SNAPPING_ENABLED.load(Ordering::Relaxed) {
                    crate::snapping::bring_cluster_to_top(crate::snapping::WindowKind::Visualizer, hwnd);
                }
                if !state_ptr.is_null() {
                    let is_top = crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main);
                    if is_fs || ((*state_ptr).is_pinned && is_top) {
                        let main_hwnd = crate::tray::get_main_hwnd();
                        let main_active = !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray();
                        if main_active {
                            if is_fs {
                                SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                            }
                            SetWindowPos(main_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        } else {
                            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        }
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SETFOCUS => {
            if !state_ptr.is_null() {
                if let Some(ref wv) = (*state_ptr).webview {
                    let _ = wv.focus();
                    let _ = wv.evaluate_script("window.focus();");
                }
                let is_fs = (*state_ptr).is_fullscreen;
                if is_fs && is_hud_shown() {
                    // 全螢幕 HUD 作用中：視覺視窗獲得焦點時，加固維持所有 HUD 浮層視窗（無論黏著還是拉開）在最頂層
                    let members = get_all_hud_target_windows();
                    for &m in &members {
                        let m_hwnd = crate::snapping::get_window_hwnd(m);
                        if !m_hwnd.is_null() && IsWindow(m_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
                            SetWindowPos(m_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        }
                    }
                    return 0;
                }
                let is_top = crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main);
                if is_fs || ((*state_ptr).is_pinned && is_top) {
                    let main_hwnd = crate::tray::get_main_hwnd();
                    let main_active = !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray();
                    if main_active {
                        if is_fs {
                            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        }
                        SetWindowPos(main_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    } else {
                        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_NCHITTEST => {
            if !state_ptr.is_null() && (*state_ptr).is_fullscreen {
                return HTCLIENT as isize;
            }
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let mut rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);

            // 若視覺視窗透明度 <= 40%，自動啟用穿透模式：標題列保留互動，下方內容穿透
            // 若選單開啟中，豁免穿透，允許使用者自由調整選單滑桿
            if crate::snapping::is_click_through_active(crate::snapping::WindowKind::Visualizer)
                && !crate::snapping::is_opacity_menu_open(crate::snapping::WindowKind::Visualizer)
                && y > rect.top + 42
            {
                return -1; // HTTRANSPARENT
            }

            let b = 8; // 8-pixel border for mouse resizing

            let on_left = x >= rect.left && x < rect.left + b;
            let on_right = x <= rect.right && x > rect.right - b;
            let on_top = y >= rect.top && y < rect.top + b;
            let on_bottom = y <= rect.bottom && y > rect.bottom - b;

            if on_top && on_left { return HTTOPLEFT as isize; }
            if on_top && on_right { return HTTOPRIGHT as isize; }
            if on_bottom && on_left { return HTBOTTOMLEFT as isize; }
            if on_bottom && on_right { return HTBOTTOMRIGHT as isize; }
            if on_left { return HTLEFT as isize; }
            if on_right { return HTRIGHT as isize; }
            if on_top { return HTTOP as isize; }
            if on_bottom { return HTBOTTOM as isize; }

            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SYSCOMMAND => {
            let sc = (wparam & 0xFFF0) as u32;
            if sc == SC_MINIMIZE {
                let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
                if role == crate::snapping::WindowRole::ClusterMaster || role == crate::snapping::WindowRole::ClusterSlave {
                    unsafe {
                        crate::snapping::minimize_cluster(crate::snapping::WindowKind::Visualizer);
                    }
                    return 0;
                }
            }
            if sc == SC_MAXIMIZE {
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SIZE => {
            let resize_type = wparam as u32;
            if resize_type == SIZE_RESTORED
                && !crate::snapping::is_cluster_minimizing()
                && !crate::snapping::is_cluster_restoring()
            {
                let role = crate::snapping::get_window_role(crate::snapping::WindowKind::Visualizer);
                if role == crate::snapping::WindowRole::ClusterMaster || role == crate::snapping::WindowRole::ClusterSlave {
                    unsafe {
                        crate::snapping::restore_cluster(crate::snapping::WindowKind::Visualizer, hwnd);
                    }
                }
            }
            if !state_ptr.is_null() && !VISUALIZER_CLOSING.load(Ordering::SeqCst) {
                if let Some(ref wv) = (*state_ptr).webview {
                    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    GetClientRect(hwnd, &mut rect);
                    let w = (rect.right - rect.left).max(1) as u32;
                    let h = (rect.bottom - rect.top).max(1) as u32;
                    let _ = wv.set_bounds(wry::Rect {
                        position: Position::Physical(PhysicalPosition::new(0, 0)),
                        size: Size::Physical(PhysicalSize::new(w, h)),
                    });
                }
            }
            0
        }
        WM_TIMER => {
            if wparam == TIMER_AUDIO_SPECTRUM {
                let mut pt = POINT { x: 0, y: 0 };
                let mut inside = false;
                if GetCursorPos(&mut pt) != 0 {
                    let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    if GetWindowRect(hwnd, &mut rc) != 0 {
                        let in_rect = pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom;
                        inside = if in_rect {
                            let w = WindowFromPoint(pt);
                            !w.is_null() && (w == hwnd || GetAncestor(w, GA_ROOT) == hwnd)
                        } else {
                            false
                        };
                        let was_hovered = VIS_HOVERED.load(Ordering::Relaxed);
                        if inside != was_hovered {
                            VIS_HOVERED.store(inside, Ordering::SeqCst);
                        }
                        apply_visualizer_opacity(hwnd);
                    }
                }

                // Smart Eco Mode:
                // When visualizer is not fullscreen, mouse is not hovering, and audio has been silent for > 30s,
                // pause 3D WebGL rendering loop and switch timer to low frequency (250ms).
                // This stops all GPU presents and allows Windows to enter natural display/system sleep!
                let is_fullscreen = is_visualizer_fullscreen();
                let is_audio_active = crate::spectrum::is_audio_active_30s();
                let should_eco = !is_fullscreen && !inside && !is_audio_active;
                let was_eco = VISUALIZER_ECO_MODE.load(Ordering::Relaxed);

                if should_eco != was_eco {
                    VISUALIZER_ECO_MODE.store(should_eco, Ordering::SeqCst);
                    if should_eco {
                        SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 250, None);
                        if !state_ptr.is_null() {
                            if let Some(ref wv) = (*state_ptr).webview {
                                let _ = wv.evaluate_script("if (typeof window.setEcoMode === 'function') window.setEcoMode(true);");
                            }
                        }
                    } else {
                        SetTimer(hwnd, TIMER_AUDIO_SPECTRUM, 16, None);
                        if !state_ptr.is_null() {
                            if let Some(ref wv) = (*state_ptr).webview {
                                let _ = wv.evaluate_script("if (typeof window.setEcoMode === 'function') window.setEcoMode(false);");
                            }
                        }
                    }
                }

                if should_eco {
                    return 0;
                }

                if !state_ptr.is_null() && !VISUALIZER_CLOSING.load(Ordering::SeqCst) {
                    if let Some(ref wv) = (*state_ptr).webview {
                        if let Ok(guard) = VISUALIZER_LANG.lock() {
                            if let Some(ref current) = *guard {
                                if let Ok(mut applied) = APPLIED_LANG.lock() {
                                    if applied.as_deref() != Some(current.as_str()) {
                                        *applied = Some(current.clone());
                                        let script = format!(
                                            "if (typeof window.setLanguage === 'function') window.setLanguage({:?});",
                                            current
                                        );
                                        let _ = wv.evaluate_script(&script);
                                    }
                                }
                            }
                        }

                        let spectrum = crate::spectrum::get_raw_spectrum();
                        if let Ok(json_data) = serde_json::to_string(&spectrum[..]) {
                            let script = format!(
                                "if (typeof window.handleHostAudio === 'function') window.handleHostAudio({{\"type\":\"spectrum\",\"data\":{}}});",
                                json_data
                            );
                            let _ = wv.evaluate_script(&script);
                        }
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            VISUALIZER_CLOSING.store(true, Ordering::SeqCst);
            VISUALIZER_HWND.store(0, Ordering::SeqCst);
            if VISUALIZER_FULLSCREEN.swap(false, Ordering::SeqCst) {
                VISUALIZER_HUD_MAIN_SHOWN.store(false, Ordering::SeqCst);
                restore_hud_pinned_states();
            }
            crate::spectrum::update_spectrum_needed_state();
            crate::snapping::handle_window_closed(crate::snapping::WindowKind::Visualizer);
            if let Some(ctx) = crate::gui::get_egui_ctx() {
                ctx.request_repaint();
            }
            let main_hwnd = crate::tray::get_main_hwnd();
            if !main_hwnd.is_null() {
                const GWLP_HWNDPARENT: i32 = -8;
                SetWindowLongPtrW(main_hwnd, GWLP_HWNDPARENT, 0);
            }
            KillTimer(hwnd, TIMER_AUDIO_SPECTRUM);
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            VISUALIZER_HIDDEN_WITH_CLUSTER.store(false, Ordering::SeqCst);
            VISUALIZER_HWND.store(0, Ordering::SeqCst);
            let main_hwnd = crate::tray::get_main_hwnd();
            if !main_hwnd.is_null() {
                const GWLP_HWNDPARENT: i32 = -8;
                SetWindowLongPtrW(main_hwnd, GWLP_HWNDPARENT, 0);
            }
            crate::snapping::break_dock_bond();
            restore_hud_pinned_states();
            VISUALIZER_FULLSCREEN.store(false, Ordering::SeqCst);
            crate::controller::send_controller_cmd(crate::controller::GuiCmd::VisualizerFullscreenChanged(false));
            KillTimer(hwnd, TIMER_AUDIO_SPECTRUM);
            if let Ok(mut applied) = APPLIED_LANG.lock() {
                *applied = None;
            }
            if !state_ptr.is_null() {
                let mut boxed = Box::from_raw(state_ptr);
                boxed.webview.take(); // cleanly drops wry WebView & InnerWebView before clearing user data
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            let main_hwnd = crate::tray::get_main_hwnd();
            if !main_hwnd.is_null() && !crate::tray::is_window_hidden_to_tray() && !crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main) {
                SetWindowPos(main_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
