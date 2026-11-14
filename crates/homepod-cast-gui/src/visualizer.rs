use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use std::borrow::Cow;
use std::num::NonZeroIsize;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;
use wry::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use wry::WebViewBuilder;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static VISUALIZER_HWND: AtomicIsize = AtomicIsize::new(0);
static VISUALIZER_FULLSCREEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static VISUALIZER_DEV_NAME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static VISUALIZER_LANG: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static APPLIED_LANG: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

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

pub fn is_visualizer_open() -> bool {
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst);
    hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 }
}

pub fn get_visualizer_hwnd() -> HWND {
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

/// Updates the caption buttons (minimize, maximize, close) on the visualizer titlebar.
/// When magnetically docked to the main window, all three buttons disappear in unison,
/// making the ensemble look like a sleek unified panel. When detached, they are restored.
pub unsafe fn update_visualizer_dock_buttons(hwnd: HWND, is_docked: bool) {
    let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let new_style = if is_docked {
        // When docked to main window, remove WS_SYSMENU, WS_MINIMIZEBOX, WS_MAXIMIZEBOX
        // so that all three titlebar buttons (_, □, X) completely disappear in unison.
        current_style & !(WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX)
    } else {
        // When detached, restore standard caption buttons.
        current_style | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX
    };
    if new_style != current_style {
        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);
    }

    if !is_docked {
        // Revert system menu back to default
        GetSystemMenu(hwnd, 1);
    }

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
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst);
    if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
        unsafe {
            PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0);
        }
    }
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
    let existing = VISUALIZER_HWND.load(Ordering::SeqCst);
    if existing != 0 && unsafe { IsWindow(existing as HWND) != 0 } {
        unsafe {
            ShowWindow(existing as HWND, SW_RESTORE);
            SetForegroundWindow(existing as HWND);
        }
        return;
    }

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

pub fn focus_visualizer() {
    let hwnd = VISUALIZER_HWND.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            PostMessageW(hwnd, WM_USER_FOCUS_WEBVIEW, 0, 0);
        }
    }
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
    let target = if state.is_pinned || state.is_fullscreen {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    SetWindowPos(hwnd, target, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);

    // If main window has always-on-top enabled, keep main window above visualizer
    if crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) && !crate::tray::is_window_hidden_to_tray() {
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
    let new_pinned = !state.is_pinned;
    set_pin(hwnd, state, new_pinned);

    // If magnetically docked with main window, propagate new_pinned to main window!
    let bond = crate::snapping::get_dock_bond();
    if bond.is_bonded {
        crate::tray::set_main_window_pinned(new_pinned);
    }
}

unsafe fn toggle_fullscreen(hwnd: HWND, state: &mut VisualizerState) {
    if !state.is_fullscreen {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if (style & WS_OVERLAPPEDWINDOW) != 0 {
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
                crate::snapping::break_dock_bond();

                SetWindowLongW(hwnd, GWL_STYLE, (style & !WS_OVERLAPPEDWINDOW) as i32);
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    mi.rcMonitor.left,
                    mi.rcMonitor.top,
                    mi.rcMonitor.right - mi.rcMonitor.left,
                    mi.rcMonitor.bottom - mi.rcMonitor.top,
                    SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                );
                windows_sys::Win32::System::Power::SetThreadExecutionState(
                    windows_sys::Win32::System::Power::ES_CONTINUOUS
                        | windows_sys::Win32::System::Power::ES_SYSTEM_REQUIRED
                        | windows_sys::Win32::System::Power::ES_DISPLAY_REQUIRED,
                );

                // If main window has always-on-top enabled, keep it above the fullscreen visualizer
                if crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) && !crate::tray::is_window_hidden_to_tray() {
                    let main_hwnd = crate::tray::get_main_hwnd();
                    if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 {
                        SetWindowPos(
                            main_hwnd,
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }

                // Keep taskbar beneath the fullscreen visualizer
                let taskbar_class: Vec<u16> = "Shell_TrayWnd\0".encode_utf16().collect();
                let taskbar = FindWindowW(taskbar_class.as_ptr(), std::ptr::null());
                if !taskbar.is_null() {
                    SetWindowPos(taskbar, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
        }
    } else {
        state.is_fullscreen = false;
        VISUALIZER_FULLSCREEN.store(false, Ordering::SeqCst);
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        SetWindowLongW(hwnd, GWL_STYLE, (style | WS_OVERLAPPEDWINDOW) as i32);
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
        windows_sys::Win32::System::Power::SetThreadExecutionState(
            windows_sys::Win32::System::Power::ES_CONTINUOUS,
        );

        let main_hwnd = crate::tray::get_main_hwnd();
        if !main_hwnd.is_null() {
            const GWLP_HWNDPARENT: i32 = -8;
            SetWindowLongPtrW(main_hwnd, GWLP_HWNDPARENT, 0);
            if !crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) {
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
    let mut main_rect: RECT = std::mem::zeroed();

    let (width, height, x, y) = if has_main {
        GetWindowRect(main_hwnd, &mut main_rect);
        let main_h = (main_rect.bottom - main_rect.top).max(400);
        let w = ((main_h as f32) * 16.0 / 9.0).round() as i32;

        let work_area = crate::snapping::get_monitor_work_area(main_hwnd);

        // Position directly docked to the right of the Main Window if space permits
        let (init_x, init_y) = if main_rect.right + w <= work_area.right {
            (main_rect.right, main_rect.top)
        } else if main_rect.left - w >= work_area.left {
            // If right edge exceeds screen, dock to the left of the Main Window
            (main_rect.left - w, main_rect.top)
        } else {
            // Shift main window left to accommodate both windows docked side-by-side
            let shift_needed = (main_rect.right + w) - work_area.right;
            let new_main_x = (main_rect.left - shift_needed).max(work_area.left);
            let main_w = main_rect.right - main_rect.left;
            SetWindowPos(
                main_hwnd,
                std::ptr::null_mut(),
                new_main_x,
                main_rect.top,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            main_rect.left = new_main_x;
            main_rect.right = new_main_x + main_w;
            (main_rect.right, main_rect.top)
        };

        (w, main_h, init_x, init_y)
    } else {
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let w = 1280;
        let h = 720;
        let init_x = (screen_w - w) / 2;
        let init_y = (screen_h - h) / 2;
        (w, h, init_x, init_y)
    };

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
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

    SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
    SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);

    VISUALIZER_HWND.store(hwnd as isize, Ordering::SeqCst);

    // Directly bind magnetic dock with Main Window upon creation!
    if has_main {
        let mut actual_vis_rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut actual_vis_rect);
        let mut actual_main_rect: RECT = std::mem::zeroed();
        GetWindowRect(main_hwnd, &mut actual_main_rect);
        crate::snapping::set_dock_bond(&actual_vis_rect, &actual_main_rect);
    }

    // Create state container
    let mut placement: WINDOWPLACEMENT = std::mem::zeroed();
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;

    let initial_pinned = crate::controller::load_visualizer_pinned();
    let state = Box::into_raw(Box::new(VisualizerState {
        webview: None,
        is_fullscreen: false,
        is_pinned: initial_pinned,
        saved_placement: placement,
    }));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);

    if initial_pinned {
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        if crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) && !crate::tray::is_window_hidden_to_tray() {
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
            if body.contains("close") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_CLOSE, 0, 0);
                }
            } else if body.contains("fullscreen") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_TOGGLE_FULLSCREEN, 0, 0);
                }
            } else if body.contains("toggle_pin") {
                unsafe {
                    PostMessageW(hwnd_capture as HWND, WM_USER_TOGGLE_PIN, 0, 0);
                }
            } else if body.contains("show_main") {
                tracing::info!("visualizer IPC: show_main received");
                crate::tray::bring_main_to_front();
                let vis = VISUALIZER_HWND.load(Ordering::SeqCst) as HWND;
                if !vis.is_null() && unsafe { IsWindow(vis) != 0 } {
                    unsafe {
                        SetForegroundWindow(vis);
                    }
                }
            }
        });
        let lang_code = VISUALIZER_LANG
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| crate::controller::load_language().code().to_string());
        let initial_url = format!(
            "app://visualizer/index.html?lang={}&pinned={}",
            lang_code,
            if initial_pinned { 1 } else { 0 }
        );
        let builder = builder.with_url(&initial_url);

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

            let script = format!(
                "if (typeof window.setLanguage === 'function') window.setLanguage({:?});\
                 if (typeof window.handleVisPinChanged === 'function') window.handleVisPinChanged({}, false);",
                lang_code,
                initial_pinned
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
        WM_MOVING => {
            if !state_ptr.is_null() && !(*state_ptr).is_fullscreen {
                if crate::snapping::SNAPPING_ENABLED.load(Ordering::Relaxed) {
                    let rect_ptr = lparam as *mut RECT;
                    if !rect_ptr.is_null() {
                        let dist = crate::snapping::SNAP_DISTANCE.load(Ordering::Relaxed);
                        let work = crate::snapping::get_monitor_work_area(hwnd);
                        crate::snapping::snap_rect_to_work_area(&mut *rect_ptr, &work, dist);

                        let main_hwnd = crate::tray::get_main_hwnd();
                        if !main_hwnd.is_null()
                            && IsWindow(main_hwnd) != 0
                            && IsWindowVisible(main_hwnd) != 0
                        {
                            let mut main_rect: RECT = std::mem::zeroed();
                            GetWindowRect(main_hwnd, &mut main_rect);
                            if crate::snapping::snap_rect_to_rect(&mut *rect_ptr, &main_rect, dist) {
                                crate::snapping::set_dock_bond(&*rect_ptr, &main_rect);
                            } else {
                                let bond = crate::snapping::get_dock_bond();
                                if bond.is_bonded {
                                    let (expected_x, expected_y) = crate::snapping::calc_bonded_visualizer_pos(&bond, &*rect_ptr, &main_rect);
                                    if ((*rect_ptr).left - expected_x).abs() > dist * 2
                                        || ((*rect_ptr).top - expected_y).abs() > dist * 2
                                    {
                                        crate::snapping::break_dock_bond();
                                    }
                                }
                            }
                        }
                        return 1;
                    }
                }
            }
            0
        }
        WM_EXITSIZEMOVE => {
            if !state_ptr.is_null() && !(*state_ptr).is_fullscreen {
                let mut vis_rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut vis_rect);
                let main_hwnd = crate::tray::get_main_hwnd();
                if !main_hwnd.is_null()
                    && IsWindow(main_hwnd) != 0
                    && IsWindowVisible(main_hwnd) != 0
                {
                    let mut main_rect: RECT = std::mem::zeroed();
                    GetWindowRect(main_hwnd, &mut main_rect);
                    if crate::snapping::are_rects_docked(&vis_rect, &main_rect, 4) {
                        crate::snapping::set_dock_bond(&vis_rect, &main_rect);
                    } else {
                        crate::snapping::break_dock_bond();
                    }
                }
            }
            0
        }
        WM_ACTIVATE => {
            let active_type = (wparam & 0xFFFF) as u32;
            if active_type != 0 {
                // WA_ACTIVE or WA_CLICKACTIVE: User focused or clicked on the visualizer.
                if !state_ptr.is_null() && ((*state_ptr).is_fullscreen || (*state_ptr).is_pinned) {
                    let main_hwnd = crate::tray::get_main_hwnd();
                    let main_active = !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray();
                    if main_active {
                        SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        SetWindowPos(main_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    } else {
                        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    }
                }

                // If visualizer is docked with main window, bring main window along!
                if crate::snapping::SNAPPING_ENABLED.load(Ordering::Relaxed) {
                    let bond = crate::snapping::get_dock_bond();
                    if bond.is_bonded && !state_ptr.is_null() && !(*state_ptr).is_fullscreen && !VISUALIZER_FULLSCREEN.load(Ordering::Relaxed) {
                        let main_hwnd = crate::tray::get_main_hwnd();
                        if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
                            if IsIconic(main_hwnd) != 0 {
                                ShowWindow(main_hwnd, SW_SHOWNOACTIVATE);
                            }
                            let mut vis_rect: RECT = std::mem::zeroed();
                            GetWindowRect(hwnd, &mut vis_rect);
                            let mut main_rect: RECT = std::mem::zeroed();
                            GetWindowRect(main_hwnd, &mut main_rect);
                            let (mut main_x, mut main_y) = crate::snapping::calc_bonded_main_pos(&bond, &vis_rect, &main_rect);
                            let work = crate::snapping::get_monitor_work_area(main_hwnd);
                            let main_w = main_rect.right - main_rect.left;
                            let main_h = main_rect.bottom - main_rect.top;
                            main_x = main_x.max(work.left).min(work.right - main_w);
                            main_y = main_y.max(work.top).min(work.bottom - main_h);
                            SetWindowPos(
                                main_hwnd,
                                hwnd,
                                main_x,
                                main_y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOACTIVATE,
                            );
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
                if (*state_ptr).is_fullscreen || (*state_ptr).is_pinned {
                    let main_hwnd = crate::tray::get_main_hwnd();
                    let main_active = !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray();
                    if main_active {
                        SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                        SetWindowPos(main_hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    } else {
                        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SYSCOMMAND => {
            let sc = (wparam & 0xFFF0) as u32;
            let bond = crate::snapping::get_dock_bond();
            if bond.is_bonded && (sc == SC_CLOSE || sc == SC_MINIMIZE || sc == SC_MAXIMIZE) {
                // When magnetically docked to main window, caption buttons are locked
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SIZE => {
            let resize_type = wparam as u32;
            if resize_type == SIZE_RESTORED && crate::snapping::SNAPPING_ENABLED.load(Ordering::Relaxed) {
                let bond = crate::snapping::get_dock_bond();
                if bond.is_bonded && !state_ptr.is_null() && !(*state_ptr).is_fullscreen && !VISUALIZER_FULLSCREEN.load(Ordering::Relaxed) {
                    let main_hwnd = crate::tray::get_main_hwnd();
                    if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
                        if IsIconic(main_hwnd) != 0 {
                            ShowWindow(main_hwnd, SW_SHOWNOACTIVATE);
                        }
                        let mut vis_rect: RECT = std::mem::zeroed();
                        GetWindowRect(hwnd, &mut vis_rect);
                        let mut main_rect: RECT = std::mem::zeroed();
                        GetWindowRect(main_hwnd, &mut main_rect);
                        let (mut main_x, mut main_y) = crate::snapping::calc_bonded_main_pos(&bond, &vis_rect, &main_rect);
                        let work = crate::snapping::get_monitor_work_area(main_hwnd);
                        let main_w = main_rect.right - main_rect.left;
                        let main_h = main_rect.bottom - main_rect.top;
                        main_x = main_x.max(work.left).min(work.right - main_w);
                        main_y = main_y.max(work.top).min(work.bottom - main_h);
                        SetWindowPos(
                            main_hwnd,
                            hwnd,
                            main_x,
                            main_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
            }
            if !state_ptr.is_null() {
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
                if !state_ptr.is_null() {
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
            let main_hwnd = crate::tray::get_main_hwnd();
            if !main_hwnd.is_null() {
                const GWLP_HWNDPARENT: i32 = -8;
                SetWindowLongPtrW(main_hwnd, GWLP_HWNDPARENT, 0);
            }
            crate::snapping::break_dock_bond();
            VISUALIZER_FULLSCREEN.store(false, Ordering::SeqCst);
            windows_sys::Win32::System::Power::SetThreadExecutionState(
                windows_sys::Win32::System::Power::ES_CONTINUOUS,
            );
            KillTimer(hwnd, TIMER_AUDIO_SPECTRUM);
            if let Ok(mut applied) = APPLIED_LANG.lock() {
                *applied = None;
            }
            if !state_ptr.is_null() {
                let _ = Box::from_raw(state_ptr);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            let main_hwnd = crate::tray::get_main_hwnd();
            if !main_hwnd.is_null() && !crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) {
                SetWindowPos(main_hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
