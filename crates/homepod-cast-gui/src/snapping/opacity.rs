use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::*;
pub(crate) const WM_MOUSELEAVE: u32 = 0x02A3;

// Per-window Always-On-Top pinned state storage
pub(crate) static MAIN_PINNED: AtomicBool = AtomicBool::new(false);
pub(crate) static MINI_PLAYER_PINNED: AtomicBool = AtomicBool::new(false);
pub(crate) static EQUALIZER_PINNED: AtomicBool = AtomicBool::new(false);
pub(crate) static VISUALIZER_PINNED: AtomicBool = AtomicBool::new(false);

// Window Opacity & Hover Peek settings storage (opacity stored as percent 20..=100)
pub(crate) static MAIN_WINDOW_OPACITY: AtomicU32 = AtomicU32::new(100);
pub(crate) static MINI_PLAYER_OPACITY: AtomicU32 = AtomicU32::new(100);
pub(crate) static EQUALIZER_OPACITY: AtomicU32 = AtomicU32::new(100);
pub(crate) static VISUALIZER_OPACITY: AtomicU32 = AtomicU32::new(100);
pub(crate) static LAST_CUSTOM_OPACITY_MAIN: AtomicU32 = AtomicU32::new(30);
pub(crate) static LAST_CUSTOM_OPACITY_MINI: AtomicU32 = AtomicU32::new(30);
pub(crate) static LAST_CUSTOM_OPACITY_EQ: AtomicU32 = AtomicU32::new(30);
pub(crate) static LAST_CUSTOM_OPACITY_VIS: AtomicU32 = AtomicU32::new(30);
pub(crate) static HOVER_PEEK_ENABLED: AtomicBool = AtomicBool::new(true);

pub(crate) static GHOST_EXIT_TIME_MS: AtomicU64 = AtomicU64::new(0);

pub(crate) static PEEK_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
pub(crate) fn peek_now_ms() -> u64 {
    let epoch = PEEK_EPOCH.get_or_init(std::time::Instant::now);
    epoch.elapsed().as_millis() as u64
}

pub fn record_ghost_exit() {
    GHOST_EXIT_TIME_MS.store(peek_now_ms(), Ordering::SeqCst);
}

pub fn is_recently_exited_ghost() -> bool {
    let last = GHOST_EXIT_TIME_MS.load(Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    peek_now_ms().saturating_sub(last) < 450
}

pub fn clear_ghost_exit_record() {
    GHOST_EXIT_TIME_MS.store(0, Ordering::SeqCst);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpacityState {
    /// 待命常態：HUD 隱藏、Hover Peek 正常啟用、若透明度 <= 40% 則穿透生效
    Normal,
    /// 主動微調中：滾輪旋轉或滑桿拖曳中（850ms）。
    /// 中央 HUD 顯示、Hover Peek 暫時抑制（實體化預覽暫停，確保即時觀察真實透明度）
    Tuning(std::time::Instant),
    /// 控制面板開啟中：右鍵點擊圖釘開啟選單。
    /// 暫時豁免穿透模式（允許點擊滑桿）、Hover Peek 抑制
    MenuOpen,
    /// 微調或選單關閉後的平滑淡出過渡態 (200ms)
    Fading(std::time::Instant),
}

pub(crate) static OPACITY_FSM: Mutex<Option<HashMap<WindowKind, OpacityState>>> = Mutex::new(None);

pub(crate) fn with_opacity_fsm<R>(f: impl FnOnce(&mut HashMap<WindowKind, OpacityState>) -> R) -> R {
    let mut guard = OPACITY_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

pub fn get_opacity_state(kind: WindowKind) -> OpacityState {
    with_opacity_fsm(|map| {
        let state = map.entry(kind).or_insert(OpacityState::Normal);
        match *state {
            OpacityState::Tuning(start) => {
                if start.elapsed() > std::time::Duration::from_millis(850) {
                    *state = OpacityState::Fading(std::time::Instant::now());
                }
            }
            OpacityState::Fading(start) => {
                if start.elapsed() > std::time::Duration::from_millis(200) {
                    *state = OpacityState::Normal;
                }
            }
            _ => {}
        }
        *state
    })
}

pub fn set_opacity_state(kind: WindowKind, new_state: OpacityState) {
    with_opacity_fsm(|map| {
        map.insert(kind, new_state);
    });
    crate::gui::request_ui_repaint();
}

pub fn clear_all_opacity_fsm() {
    with_opacity_fsm(|map| {
        map.clear();
    });
}

pub fn trigger_opacity_tuning(kind: WindowKind) {
    set_opacity_state(kind, OpacityState::Tuning(std::time::Instant::now()));
}

pub fn trigger_cluster_opacity_tuning(origin: WindowKind) {
    let members = get_cluster_members(origin);
    for m in members {
        trigger_opacity_tuning(m);
    }
}

pub fn is_opacity_hud_active(kind: WindowKind) -> bool {
    matches!(get_opacity_state(kind), OpacityState::Tuning(_))
}

pub fn is_peek_suppressed(kind: WindowKind) -> bool {
    matches!(
        get_opacity_state(kind),
        OpacityState::Tuning(_) | OpacityState::MenuOpen | OpacityState::Fading(_)
    )
}

pub fn is_opacity_menu_open(kind: WindowKind) -> bool {
    matches!(get_opacity_state(kind), OpacityState::MenuOpen)
}

pub fn set_opacity_menu_open(kind: WindowKind, open: bool) {
    if open {
        set_opacity_state(kind, OpacityState::MenuOpen);
    } else {
        if matches!(get_opacity_state(kind), OpacityState::MenuOpen) {
            set_opacity_state(kind, OpacityState::Fading(std::time::Instant::now()));
        }
    }
}

pub fn set_single_window_opacity_tuning(kind: WindowKind, opacity: f32) {
    set_window_opacity(kind, opacity);
    trigger_opacity_tuning(kind);
    if opacity < 1.0 && !is_window_pinned(kind) {
        // 進入半透明態：自動鎖定置頂，跳過「半透明卻未置頂」的無效死態！
        set_window_pinned(kind, true);
        crate::controller::save_window_pinned(kind, true);
        unsafe {
            apply_window_topmost(kind, true);
        }
    }
    if kind == WindowKind::Visualizer {
        let pct = (opacity.clamp(0.2, 1.0) * 100.0).round() as u32;
        crate::visualizer::sync_visualizer_opacity(pct, true);
    }
}

pub fn get_last_custom_opacity(kind: WindowKind) -> u32 {
    match kind {
        WindowKind::Main => LAST_CUSTOM_OPACITY_MAIN.load(Ordering::Relaxed),
        WindowKind::MiniPlayer => LAST_CUSTOM_OPACITY_MINI.load(Ordering::Relaxed),
        WindowKind::Equalizer => LAST_CUSTOM_OPACITY_EQ.load(Ordering::Relaxed),
        WindowKind::Visualizer => LAST_CUSTOM_OPACITY_VIS.load(Ordering::Relaxed),
    }
}

pub fn set_last_custom_opacity(kind: WindowKind, val: u32) {
    let clamped = val.clamp(20, 95);
    match kind {
        WindowKind::Main => LAST_CUSTOM_OPACITY_MAIN.store(clamped, Ordering::SeqCst),
        WindowKind::MiniPlayer => LAST_CUSTOM_OPACITY_MINI.store(clamped, Ordering::SeqCst),
        WindowKind::Equalizer => LAST_CUSTOM_OPACITY_EQ.store(clamped, Ordering::SeqCst),
        WindowKind::Visualizer => LAST_CUSTOM_OPACITY_VIS.store(clamped, Ordering::SeqCst),
    }
}

/// 快速切換單獨視窗透明度（State 2 釘選實體 100% ↔ State 3 幽靈穿透 30%）
pub fn toggle_single_window_opacity_quick(kind: WindowKind) -> u32 {
    let cur = (get_window_opacity(kind) * 100.0).round() as u32;
    let next_pct = if cur < 100 {
        100
    } else {
        30
    };
    set_single_window_opacity_tuning(kind, next_pct as f32 / 100.0);
    set_window_pinned(kind, true);
    crate::controller::save_window_pinned(kind, true);
    unsafe {
        apply_window_topmost(kind, true);
    }
    next_pct
}

/// 快速切換群組透明度（State 2 釘選實體 100% ↔ State 3 幽靈穿透 30%）
pub fn toggle_cluster_opacity_quick(origin: WindowKind) -> u32 {
    let cur = (get_window_opacity(origin) * 100.0).round() as u32;
    let next_pct = if cur < 100 {
        100
    } else {
        30
    };
    set_cluster_opacity(origin, next_pct as f32 / 100.0);
    let members = get_cluster_members(origin);
    for &m in &members {
        set_window_pinned(m, true);
        crate::controller::save_window_pinned(m, true);
        unsafe {
            apply_window_topmost(m, true);
        }
    }
    next_pct
}

/// 喚醒單一視窗：從穿透幽靈態（State 3）喚醒至釘選實體態（State 2：100% 實體 + 保持置頂）
pub fn wake_single_window_ghost(kind: WindowKind) {
    set_single_window_opacity_tuning(kind, 1.0);
    set_window_pinned(kind, true);
    crate::controller::save_window_pinned(kind, true);
    unsafe {
        apply_window_topmost(kind, true);
    }
}

/// 喚醒群組視窗：從穿透幽靈態（State 3）喚醒至釘選實體態（State 2：100% 實體 + 保持置頂）
pub fn wake_cluster_ghost(origin: WindowKind) {
    set_cluster_opacity(origin, 1.0);
    let members = get_cluster_members(origin);
    for &m in &members {
        set_window_pinned(m, true);
        crate::controller::save_window_pinned(m, true);
        unsafe {
            apply_window_topmost(m, true);
        }
    }
}

pub fn set_cluster_opacity(origin: WindowKind, opacity: f32) {
    let members = get_cluster_members(origin);
    let pct = (opacity.clamp(0.2, 1.0) * 100.0).round() as u32;
    for &m in &members {
        set_window_opacity(m, opacity);
        if opacity < 1.0 && !is_window_pinned(m) {
            set_window_pinned(m, true);
            crate::controller::save_window_pinned(m, true);
            unsafe {
                apply_window_topmost(m, true);
            }
        }
    }
    trigger_cluster_opacity_tuning(origin);
    if members.contains(&WindowKind::Visualizer) {
        crate::visualizer::sync_visualizer_opacity(pct, true);
    }
}


pub fn init_window_pinned_states() {
    let main_pin = crate::controller::load_always_on_top();
    let mini_pin = crate::controller::load_mini_player_pinned();
    let eq_pin = crate::controller::load_equalizer_pinned();
    let vis_pin = crate::controller::load_visualizer_pinned();

    MAIN_PINNED.store(main_pin, Ordering::SeqCst);
    crate::tray::IS_ALWAYS_ON_TOP.store(main_pin, Ordering::SeqCst);
    MINI_PLAYER_PINNED.store(mini_pin, Ordering::SeqCst);
    EQUALIZER_PINNED.store(eq_pin, Ordering::SeqCst);
    VISUALIZER_PINNED.store(vis_pin, Ordering::SeqCst);

    let main_op = crate::controller::load_main_window_opacity();
    MAIN_WINDOW_OPACITY.store((main_op * 100.0).round() as u32, Ordering::SeqCst);
    let mini_op = crate::controller::load_mini_player_opacity();
    MINI_PLAYER_OPACITY.store((mini_op * 100.0).round() as u32, Ordering::SeqCst);
    let eq_op = crate::controller::load_equalizer_opacity();
    EQUALIZER_OPACITY.store((eq_op * 100.0).round() as u32, Ordering::SeqCst);
    let vis_op = crate::controller::load_visualizer_opacity();
    VISUALIZER_OPACITY.store((vis_op * 100.0).round() as u32, Ordering::SeqCst);
    let peek = crate::controller::load_hover_peek_enabled();
    HOVER_PEEK_ENABLED.store(peek, Ordering::SeqCst);

    reset_animated_window_opacity(WindowKind::Main, main_op);
    reset_animated_window_opacity(WindowKind::MiniPlayer, mini_op);
    reset_animated_window_opacity(WindowKind::Equalizer, eq_op);
    reset_animated_window_opacity(WindowKind::Visualizer, vis_op);
}

// 平滑淡入淡出 (Smooth Hover Peek Lerp)
pub(crate) static ANIMATED_OPACITY_MAIN: AtomicU32 = AtomicU32::new(1000);
pub(crate) static ANIMATED_OPACITY_MINI: AtomicU32 = AtomicU32::new(1000);
pub(crate) static ANIMATED_OPACITY_EQ: AtomicU32 = AtomicU32::new(1000);
pub(crate) static ANIMATED_OPACITY_VIS: AtomicU32 = AtomicU32::new(1000);

pub(crate) static LAST_ANIM_TIME_MAIN: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_ANIM_TIME_MINI: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_ANIM_TIME_EQ: AtomicU64 = AtomicU64::new(0);
pub(crate) static LAST_ANIM_TIME_VIS: AtomicU64 = AtomicU64::new(0);

pub fn reset_animated_window_opacity(kind: WindowKind, opacity: f32) {
    let val = (opacity.clamp(0.2, 1.0) * 1000.0).round() as u32;
    let now = peek_now_ms();
    match kind {
        WindowKind::Main => {
            ANIMATED_OPACITY_MAIN.store(val, Ordering::Relaxed);
            LAST_ANIM_TIME_MAIN.store(now, Ordering::Relaxed);
        }
        WindowKind::MiniPlayer => {
            ANIMATED_OPACITY_MINI.store(val, Ordering::Relaxed);
            LAST_ANIM_TIME_MINI.store(now, Ordering::Relaxed);
        }
        WindowKind::Equalizer => {
            ANIMATED_OPACITY_EQ.store(val, Ordering::Relaxed);
            LAST_ANIM_TIME_EQ.store(now, Ordering::Relaxed);
        }
        WindowKind::Visualizer => {
            ANIMATED_OPACITY_VIS.store(val, Ordering::Relaxed);
            LAST_ANIM_TIME_VIS.store(now, Ordering::Relaxed);
        }
    }
}

pub fn step_animated_window_opacity(kind: WindowKind, target_op: f32) -> (f32, bool) {
    let target = target_op.clamp(0.2, 1.0);
    let (anim_atomic, time_atomic) = match kind {
        WindowKind::Main => (&ANIMATED_OPACITY_MAIN, &LAST_ANIM_TIME_MAIN),
        WindowKind::MiniPlayer => (&ANIMATED_OPACITY_MINI, &LAST_ANIM_TIME_MINI),
        WindowKind::Equalizer => (&ANIMATED_OPACITY_EQ, &LAST_ANIM_TIME_EQ),
        WindowKind::Visualizer => (&ANIMATED_OPACITY_VIS, &LAST_ANIM_TIME_VIS),
    };

    // 若處於即時微調（Peek 抑制期間），零延遲直接到位
    if is_peek_suppressed(kind) {
        let val = (target * 1000.0).round() as u32;
        anim_atomic.store(val, Ordering::Relaxed);
        time_atomic.store(peek_now_ms(), Ordering::Relaxed);
        return (target, false);
    }

    let now = peek_now_ms();
    let last = time_atomic.swap(now, Ordering::Relaxed);
    let dt = if last == 0 || now <= last {
        0.016
    } else {
        ((now - last) as f32 / 1000.0).clamp(0.001, 0.08)
    };

    let current = anim_atomic.load(Ordering::Relaxed) as f32 / 1000.0;
    let diff = target - current;

    if diff.abs() <= 0.008 {
        let val = (target * 1000.0).round() as u32;
        anim_atomic.store(val, Ordering::Relaxed);
        return (target, false);
    }

    // ~160ms 平滑線性過渡 (速度 ~6.25/s)
    let step = dt * 6.25;
    let next = if diff > 0.0 {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    };

    let next_u32 = (next * 1000.0).round() as u32;
    anim_atomic.store(next_u32, Ordering::Relaxed);
    let still_animating = (next - target).abs() > 0.008;
    (next, still_animating)
}

pub fn get_window_opacity(kind: WindowKind) -> f32 {
    match kind {
        WindowKind::Main => MAIN_WINDOW_OPACITY.load(Ordering::Relaxed) as f32 / 100.0,
        WindowKind::MiniPlayer => MINI_PLAYER_OPACITY.load(Ordering::Relaxed) as f32 / 100.0,
        WindowKind::Equalizer => EQUALIZER_OPACITY.load(Ordering::Relaxed) as f32 / 100.0,
        WindowKind::Visualizer => VISUALIZER_OPACITY.load(Ordering::Relaxed) as f32 / 100.0,
    }
}

pub fn set_window_opacity(kind: WindowKind, opacity: f32) {
    let clamped = opacity.clamp(0.2, 1.0);
    let val = (clamped * 100.0).round() as u32;
    match kind {
        WindowKind::Main => MAIN_WINDOW_OPACITY.store(val, Ordering::SeqCst),
        WindowKind::MiniPlayer => MINI_PLAYER_OPACITY.store(val, Ordering::SeqCst),
        WindowKind::Equalizer => EQUALIZER_OPACITY.store(val, Ordering::SeqCst),
        WindowKind::Visualizer => VISUALIZER_OPACITY.store(val, Ordering::SeqCst),
    }
    if val < 100 {
        set_last_custom_opacity(kind, val);
    }
    reset_animated_window_opacity(kind, clamped);
    crate::controller::save_window_opacity(kind, clamped);
    if kind == WindowKind::Visualizer {
        crate::visualizer::sync_visualizer_opacity(val, false);
    }
    let hwnd = get_window_hwnd(kind);
    if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
        unsafe {
            apply_native_window_opacity(hwnd, clamped);
        }
    }
    crate::gui::request_ui_repaint();
}

pub fn is_hover_peek_enabled() -> bool {
    HOVER_PEEK_ENABLED.load(Ordering::Relaxed)
}

/// 判斷視窗是否處於自動點擊穿透模式（透明度 <= 40%）
pub fn is_click_through_active(kind: WindowKind) -> bool {
    get_window_opacity(kind) <= 0.40 + f32::EPSILON
}

pub fn set_hover_peek_enabled(enabled: bool) {
    HOVER_PEEK_ENABLED.store(enabled, Ordering::SeqCst);
    crate::controller::save_hover_peek_enabled(enabled);
    crate::gui::request_ui_repaint();
}

pub unsafe fn is_cursor_over_window(hwnd: HWND) -> bool {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return false;
    }
    let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
    windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
    let mut rect = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
    pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom
}

/// 判斷滑鼠游標是否位於視窗頂部的標題列保護區內（預設高度 42px）
pub unsafe fn is_cursor_over_window_header(hwnd: HWND, header_height: i32) -> bool {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return false;
    }
    let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
    windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
    let mut rect = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
    pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.top + header_height
}

pub unsafe fn update_window_click_through(kind: WindowKind, hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    let should_be_transparent = is_click_through_active(kind)
        && !is_opacity_menu_open(kind)
        && !is_cursor_over_window_header(hwnd, 42);

    if should_be_transparent {
        if (ex_style & WS_EX_TRANSPARENT) == 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED | WS_EX_TRANSPARENT) as i32);
        }
    } else if (ex_style & WS_EX_TRANSPARENT) != 0 {
        SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style & !WS_EX_TRANSPARENT) as i32);
    }
}

/// 套用 Windows 11 DWM Acrylic（亞克力高斯毛玻璃）特效
pub unsafe fn apply_dwm_acrylic(hwnd: HWND) {
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
            let m = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
            func(hwnd, &m);
        }

        let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(dwmapi, b"DwmSetWindowAttribute\0".as_ptr());
        if let Some(dwm_set_attr) = proc {
            let func: DwmSetWindowAttributeFn = std::mem::transmute(dwm_set_attr);

            // 1. DWMWA_USE_IMMERSIVE_DARK_MODE (20 on Win10 20H1+ and Win11, 19 on older Win10)
            let dark_mode: i32 = 1;
            func(hwnd, 20, &dark_mode as *const _ as _, std::mem::size_of::<i32>() as u32);
            func(hwnd, 19, &dark_mode as *const _ as _, std::mem::size_of::<i32>() as u32);

            // 2. DWMWA_SYSTEMBACKDROP_TYPE = 38 (Windows 11 22H2+ / Build 22621+)
            // DWMSBT_TRANSIENTWINDOW = 3 (Acrylic 亞克力高斯模糊)
            let backdrop_type: u32 = 3;
            func(hwnd, 38, &backdrop_type as *const _ as _, std::mem::size_of::<u32>() as u32);
        }
    }
}

pub unsafe fn apply_native_window_opacity(hwnd: HWND, opacity: f32) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let alpha = (opacity.clamp(0.2, 1.0) * 255.0).round() as u8;

    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if (ex_style & WS_EX_LAYERED) == 0 {
        if alpha == 255 {
            // 預設 100% 實體不透明且非 Layered 視窗，完全不更動 Win32 風格
            return;
        }
        SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED) as i32);
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_FRAMECHANGED,
        );
    }
    // 注意：達到 100% (alpha == 255) 時僅設定 alpha=255，絕不呼叫 SetWindowLongW 剝除 WS_EX_LAYERED！
    // 在執行時期動態剝除 WS_EX_LAYERED 會重建 DWM 重定向表面並破壞 DirectX SwapChain，引發視窗死鎖！
    SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
    windows_sys::Win32::Graphics::Gdi::RedrawWindow(
        hwnd,
        std::ptr::null(),
        std::ptr::null_mut(),
        windows_sys::Win32::Graphics::Gdi::RDW_INVALIDATE
            | windows_sys::Win32::Graphics::Gdi::RDW_UPDATENOW
            | windows_sys::Win32::Graphics::Gdi::RDW_ALLCHILDREN,
    );
}

pub fn is_window_pinned(kind: WindowKind) -> bool {
    match kind {
        WindowKind::Main => MAIN_PINNED.load(Ordering::SeqCst),
        WindowKind::MiniPlayer => MINI_PLAYER_PINNED.load(Ordering::SeqCst),
        WindowKind::Equalizer => EQUALIZER_PINNED.load(Ordering::SeqCst),
        WindowKind::Visualizer => VISUALIZER_PINNED.load(Ordering::SeqCst),
    }
}

pub fn set_window_pinned(kind: WindowKind, pinned: bool) {
    match kind {
        WindowKind::Main => {
            MAIN_PINNED.store(pinned, Ordering::SeqCst);
            crate::tray::IS_ALWAYS_ON_TOP.store(pinned, Ordering::SeqCst);
        }
        WindowKind::MiniPlayer => MINI_PLAYER_PINNED.store(pinned, Ordering::SeqCst),
        WindowKind::Equalizer => EQUALIZER_PINNED.store(pinned, Ordering::SeqCst),
        WindowKind::Visualizer => VISUALIZER_PINNED.store(pinned, Ordering::SeqCst),
    }
}

pub unsafe fn apply_window_topmost(kind: WindowKind, pinned: bool) {
    if kind == WindowKind::Visualizer {
        crate::visualizer::set_visualizer_pinned(pinned);
    } else {
        let hwnd = get_window_hwnd(kind);
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            // 防護：若視窗正處於最小化（Iconic），嚴禁調用 SetWindowPos 調整 Z-order，避免干擾 DWM 縮圖與視窗位置！
            if IsIconic(hwnd) != 0 {
                return;
            }
            let is_fullscreen_hud = crate::visualizer::is_visualizer_fullscreen() && crate::visualizer::is_hud_shown();
            let effective_pinned = pinned || is_fullscreen_hud;
            let target_z = if effective_pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
            SetWindowPos(hwnd, target_z, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
}

pub unsafe fn toggle_window_pin(origin: WindowKind) {
    let current = is_window_pinned(origin);
    let current_op = (get_window_opacity(origin) * 100.0).round() as u32;
    let role = get_window_role(origin);

    // 狀態機防呆：若當前處於半透明態 (cur < 100%) 且已置頂，單擊取消釘選代表退出幽靈模式：
    // 自動恢復 100% 實體並取消置頂（回到 State 1 常規視窗），徹底消滅「半透明卻未置頂」的無效死態！
    if current && current_op < 100 {
        record_ghost_exit();
        if role == WindowRole::Standalone {
            set_single_window_opacity_tuning(origin, 1.0);
            set_window_pinned(origin, false);
            crate::controller::save_window_pinned(origin, false);
            apply_window_topmost(origin, false);
        } else {
            set_cluster_opacity(origin, 1.0);
            let members = get_cluster_members(origin);
            for &m in &members {
                set_window_pinned(m, false);
                crate::controller::save_window_pinned(m, false);
                apply_window_topmost(m, false);
            }
        }
        return;
    }

    let new_pinned = !current;

    if role == WindowRole::Standalone {
        set_window_pinned(origin, new_pinned);
        crate::controller::save_window_pinned(origin, new_pinned);
        apply_window_topmost(origin, new_pinned);
    } else {
        let members = get_cluster_members(origin);
        for &m in &members {
            set_window_pinned(m, new_pinned);
            crate::controller::save_window_pinned(m, new_pinned);
            apply_window_topmost(m, new_pinned);
        }
    }
}

/// 錨點同化（Anchor Precedence）：
/// 當 moving 視窗吸附至 anchor 錨點時，被吸附的目標錨點之置頂狀態居主導地位，
/// moving 及其所屬的整個連通叢集成員自動同步為該狀態，並持久化與套用 HWND topmost。
pub unsafe fn assimilate_cluster_to_anchor(moving: WindowKind, anchor: WindowKind) {
    let is_fullscreen_hud = crate::visualizer::is_visualizer_fullscreen() && crate::visualizer::is_hud_shown();
    let anchor_pinned = is_window_pinned(anchor) || is_fullscreen_hud;

    // 1. 同化 moving 視窗自身及其原連通成員之置頂與透明度
    let anchor_op = get_window_opacity(anchor);
    let moving_members = get_cluster_members(moving);
    for m in moving_members {
        set_window_pinned(m, anchor_pinned);
        crate::controller::save_window_pinned(m, anchor_pinned);
        apply_window_topmost(m, anchor_pinned);
        set_window_opacity(m, anchor_op);
    }

    // 2. 同化包含 anchor 與 moving 的所有成員（保持連通叢集全體同態一致）
    let all_members = get_cluster_members(anchor);
    for m in all_members {
        set_window_pinned(m, anchor_pinned);
        crate::controller::save_window_pinned(m, anchor_pinned);
        apply_window_topmost(m, anchor_pinned);
    }

    // 3. 請求 UI 立即重繪，更新 📌 金色外觀與 Tooltip
    crate::gui::request_ui_repaint();
}


