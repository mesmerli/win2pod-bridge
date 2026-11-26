//! Window Snapping (Magnetic Windows) & Group Movement
//! Inspired by Winamp: automatically snaps windows to screen edges / taskbar
//! and to each other when close, with linked movement when bonded.
//!
//! Uses a Generalized Directed Dock Tree (Forest) architecture:
//! - Priority: Main Window (Lv 1) > Mini Player (Lv 2) > Equalizer (Lv 3) > Visualizer (Lv 4)
//! - Each Slave window has at most ONE Master (parent) it is bonded to.
//! - Master movement automatically propagates down the entire subtree recursively.
//! - Dragging a Slave window detaches it if pulled away (> 24px) without pulling Master.
//! - Window HWND discovery uses exact Win32 window title matching to prevent viewport ID collisions.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP,
};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WM_MOUSELEAVE: u32 = 0x02A3;

pub static SNAPPING_ENABLED: AtomicBool = AtomicBool::new(true);
pub static SNAP_DISTANCE: AtomicI32 = AtomicI32::new(12);

pub const MAIN_WINDOW_SUBCLASS_ID: usize = 0x504F44; // "POD"
pub const EQUALIZER_WINDOW_SUBCLASS_ID: usize = 0x504F45; // "POE"
pub const MINI_PLAYER_WINDOW_SUBCLASS_ID: usize = 0x504F4D; // "POM"

// HWND storage for standalone viewports
static EQUALIZER_HWND: AtomicIsize = AtomicIsize::new(0);
static MINI_PLAYER_HWND: AtomicIsize = AtomicIsize::new(0);

// Per-window Always-On-Top pinned state storage
static MAIN_PINNED: AtomicBool = AtomicBool::new(false);
static MINI_PLAYER_PINNED: AtomicBool = AtomicBool::new(false);
static EQUALIZER_PINNED: AtomicBool = AtomicBool::new(false);
static VISUALIZER_PINNED: AtomicBool = AtomicBool::new(false);

// Window Opacity & Hover Peek settings storage (opacity stored as percent 20..=100)
static MAIN_WINDOW_OPACITY: AtomicU32 = AtomicU32::new(100);
static MINI_PLAYER_OPACITY: AtomicU32 = AtomicU32::new(100);
static EQUALIZER_OPACITY: AtomicU32 = AtomicU32::new(100);
static VISUALIZER_OPACITY: AtomicU32 = AtomicU32::new(100);
static LAST_CUSTOM_OPACITY_MAIN: AtomicU32 = AtomicU32::new(30);
static LAST_CUSTOM_OPACITY_MINI: AtomicU32 = AtomicU32::new(30);
static LAST_CUSTOM_OPACITY_EQ: AtomicU32 = AtomicU32::new(30);
static LAST_CUSTOM_OPACITY_VIS: AtomicU32 = AtomicU32::new(30);
static HOVER_PEEK_ENABLED: AtomicBool = AtomicBool::new(true);
static IS_WORKSTATION_ANIMATING: AtomicBool = AtomicBool::new(false);
static IS_CLUSTER_CLOSING_ANIMATING: AtomicBool = AtomicBool::new(false);

static GHOST_EXIT_TIME_MS: AtomicU64 = AtomicU64::new(0);

static PEEK_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
fn peek_now_ms() -> u64 {
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

static OPACITY_FSM: Mutex<Option<HashMap<WindowKind, OpacityState>>> = Mutex::new(None);

pub fn get_opacity_state(kind: WindowKind) -> OpacityState {
    let mut guard = OPACITY_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
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
}

pub fn set_opacity_state(kind: WindowKind, new_state: OpacityState) {
    let mut guard = OPACITY_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(kind, new_state);
    crate::gui::request_ui_repaint();
}

pub fn clear_all_opacity_fsm() {
    let mut guard = OPACITY_FSM.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.clear();
    }
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

pub fn is_workstation_animating() -> bool {
    IS_WORKSTATION_ANIMATING.load(Ordering::Relaxed)
}

pub fn is_cluster_closing_animating() -> bool {
    IS_CLUSTER_CLOSING_ANIMATING.load(Ordering::Relaxed)
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
static ANIMATED_OPACITY_MAIN: AtomicU32 = AtomicU32::new(1000);
static ANIMATED_OPACITY_MINI: AtomicU32 = AtomicU32::new(1000);
static ANIMATED_OPACITY_EQ: AtomicU32 = AtomicU32::new(1000);
static ANIMATED_OPACITY_VIS: AtomicU32 = AtomicU32::new(1000);

static LAST_ANIM_TIME_MAIN: AtomicU64 = AtomicU64::new(0);
static LAST_ANIM_TIME_MINI: AtomicU64 = AtomicU64::new(0);
static LAST_ANIM_TIME_EQ: AtomicU64 = AtomicU64::new(0);
static LAST_ANIM_TIME_VIS: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    Main = 1,
    MiniPlayer = 2,
    Equalizer = 3,
    Visualizer = 4,
}

impl WindowKind {
    pub fn priority(&self) -> u32 {
        *self as u32
    }

    /// Whether this window consumes/displays real-time audio spectrum or VU meter.
    pub fn consumes_spectrum(&self) -> bool {
        match self {
            WindowKind::Main | WindowKind::MiniPlayer | WindowKind::Visualizer => true,
            WindowKind::Equalizer => false,
        }
    }

    /// Returns the embedded page in Main Window corresponding to this standalone window (if any).
    pub fn embedded_page(&self) -> Option<crate::gui::Page> {
        match self {
            WindowKind::MiniPlayer => Some(crate::gui::Page::Player),
            WindowKind::Equalizer => Some(crate::gui::Page::Equalizer),
            WindowKind::Main | WindowKind::Visualizer => None,
        }
    }

    /// 獨立視窗唯一的 Viewport ID 字串
    pub fn viewport_id_str(&self) -> Option<&'static str> {
        match self {
            WindowKind::Equalizer => Some("equalizer_window"),
            WindowKind::MiniPlayer => Some("mini_player_window"),
            _ => None,
        }
    }

    /// 視窗預設尺寸 [寬, 高]（迷你播放器與迷你等化器預設高度皆精準為主視窗 680px 的一半：340px）
    pub fn default_size(&self) -> [f32; 2] {
        match self {
            WindowKind::Equalizer => [460.0, 340.0],
            WindowKind::MiniPlayer => [460.0, 340.0],
            WindowKind::Main => [460.0, 680.0],
            WindowKind::Visualizer => [720.0, 680.0],
        }
    }

    /// 視窗最小邊界限制 [寬, 高]
    pub fn min_size(&self) -> [f32; 2] {
        match self {
            WindowKind::Equalizer => [380.0, 240.0],
            WindowKind::MiniPlayer => [380.0, 295.0],
            WindowKind::Main => [380.0, 500.0],
            WindowKind::Visualizer => [500.0, 400.0],
        }
    }

    /// 根據視窗標題比對此視窗是否吻合（涵蓋多國語言規格）
    pub fn matches_window_title(&self, title: &str) -> bool {
        match self {
            WindowKind::Main => title == "Win2Pod",
            WindowKind::Visualizer => title.contains("Visualizer") || title.contains("視覺") || title.contains("Paint Pulse"),
            WindowKind::Equalizer => {
                crate::i18n::ALL_LANGUAGES.iter().any(|l| l.eq_window_title() == title || l.mini_equalizer_window_title() == title)
                    || title.contains("Equalizer")
                    || title.contains("等化器")
                    || title.contains("均衡器")
                    || title.contains("Égaliseur")
                    || title.contains("イコライザー")
            }
            WindowKind::MiniPlayer => {
                crate::i18n::ALL_LANGUAGES.iter().any(|l| l.mini_player_window_title() == title)
                    || title.contains("Mini Player")
                    || title.contains("迷你播放器")
                    || title.contains("Mini Lecteur")
                    || title.contains("ミニプレーヤー")
            }
        }
    }
}

pub fn set_equalizer_hwnd(hwnd: HWND) {
    EQUALIZER_HWND.store(hwnd as isize, Ordering::SeqCst);
}

pub fn get_equalizer_hwnd() -> HWND {
    get_window_hwnd(WindowKind::Equalizer)
}

pub fn set_mini_player_hwnd(hwnd: HWND) {
    MINI_PLAYER_HWND.store(hwnd as isize, Ordering::SeqCst);
    crate::spectrum::update_spectrum_needed_state();
}

pub fn get_mini_player_hwnd() -> HWND {
    get_window_hwnd(WindowKind::MiniPlayer)
}

pub fn get_window_hwnd(kind: WindowKind) -> HWND {
    match kind {
        WindowKind::Main => crate::tray::get_main_hwnd(),
        WindowKind::Visualizer => crate::visualizer::get_visualizer_hwnd(),
        WindowKind::Equalizer => {
            let hwnd = EQUALIZER_HWND.load(Ordering::SeqCst);
            if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
                hwnd as HWND
            } else {
                std::ptr::null_mut()
            }
        }
        WindowKind::MiniPlayer => {
            let hwnd = MINI_PLAYER_HWND.load(Ordering::SeqCst);
            if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
                hwnd as HWND
            } else {
                std::ptr::null_mut()
            }
        }
    }
}

/// Check if a window is currently active, valid, visible, and not minimized.
pub fn is_window_visible_and_active(kind: WindowKind) -> bool {
    if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
        return false;
    }
    if kind == WindowKind::Visualizer {
        return crate::visualizer::is_visualizer_visible();
    }
    let hwnd = get_window_hwnd(kind);
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0
            && windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0
            && windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) == 0
    }
}

/// Dock side relationship of Slave window relative to Master Window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    None,
    Right,  // Slave is docked to the right edge of Master Window
    Left,   // Slave is docked to the left edge of Master Window
    Top,    // Slave is docked above Master Window
    Bottom, // Slave is docked below Master Window
}

/// Generalized Dock Bond (Slave -> Master edge)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockBond {
    pub is_bonded: bool,
    pub parent: WindowKind,
    pub side: DockSide,
    pub offset_x: i32,
    pub offset_y: i32,
}

/// Global Dock Tree: tracks every Slave window's single Master parent
static DOCK_TREE: Mutex<Option<HashMap<WindowKind, DockBond>>> = Mutex::new(None);

pub const ALL_WINDOW_KINDS: [WindowKind; 4] = [
    WindowKind::Main,
    WindowKind::MiniPlayer,
    WindowKind::Equalizer,
    WindowKind::Visualizer,
];

pub fn canonical_edge(a: WindowKind, b: WindowKind) -> (WindowKind, WindowKind) {
    if (a as u32) < (b as u32) {
        (a, b)
    } else {
        (b, a)
    }
}

static DOCK_EDGES: Mutex<Option<HashSet<(WindowKind, WindowKind)>>> = Mutex::new(None);
static LAST_KNOWN_RECTS: Mutex<Option<HashMap<WindowKind, RECT>>> = Mutex::new(None);

pub fn with_dock_edges<R>(f: impl FnOnce(&mut HashSet<(WindowKind, WindowKind)>) -> R) -> R {
    let mut guard = DOCK_EDGES.lock().unwrap();
    let edges = guard.get_or_insert_with(HashSet::new);
    f(edges)
}

pub fn add_dock_edge(a: WindowKind, b: WindowKind) {
    with_dock_edges(|edges| {
        edges.insert(canonical_edge(a, b));
    });
}

pub fn remove_dock_edge(a: WindowKind, b: WindowKind) {
    with_dock_edges(|edges| {
        edges.remove(&canonical_edge(a, b));
    });
}

pub fn remove_all_edges_for(kind: WindowKind) {
    with_dock_edges(|edges| {
        edges.retain(|&(a, b)| a != kind && b != kind);
    });
}

pub fn update_known_rect(kind: WindowKind, rect: RECT) {
    if let Ok(mut guard) = LAST_KNOWN_RECTS.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(kind, rect);
    }
}

pub fn with_dock_tree<R>(f: impl FnOnce(&mut HashMap<WindowKind, DockBond>) -> R) -> R {
    let mut guard = DOCK_TREE.lock().unwrap();
    let tree = guard.get_or_insert_with(HashMap::new);
    f(tree)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRole {
    Standalone,
    ClusterMaster,
    ClusterSlave,
}

pub fn get_window_role(kind: WindowKind) -> WindowRole {
    with_dock_tree(|tree| {
        let is_bonded = tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false);
        if is_bonded {
            WindowRole::ClusterSlave
        } else {
            let has_bonded_children = tree.values().any(|b| b.is_bonded && b.parent == kind);
            if has_bonded_children {
                WindowRole::ClusterMaster
            } else {
                WindowRole::Standalone
            }
        }
    })
}


// --- Window Title Query & HWND Discovery ---

pub unsafe fn get_window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}

unsafe extern "system" fn enum_windows_find_by_kind_proc(
    hwnd: HWND,
    lparam: LPARAM,
) -> windows_sys::Win32::Foundation::BOOL {
    let pair = &mut *(lparam as *mut (WindowKind, HWND));
    let target_kind = pair.0;

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == windows_sys::Win32::System::Threading::GetCurrentProcessId() {
        if IsWindow(hwnd) != 0 {
            let title = get_window_title(hwnd);
            if target_kind.matches_window_title(&title) {
                pair.1 = hwnd;
                return 0; // stop enumeration
            }
        }
    }
    1 // continue enumeration
}

pub fn find_hwnd_by_kind(kind: WindowKind) -> HWND {
    let existing = get_window_hwnd(kind);
    if !existing.is_null() && unsafe { IsWindow(existing) != 0 } {
        return existing;
    }
    let mut pair = (kind, std::ptr::null_mut() as HWND);
    unsafe {
        EnumWindows(
            Some(enum_windows_find_by_kind_proc),
            &mut pair as *mut _ as LPARAM,
        );
    }
    if !pair.1.is_null() {
        match kind {
            WindowKind::Equalizer => set_equalizer_hwnd(pair.1),
            WindowKind::MiniPlayer => set_mini_player_hwnd(pair.1),
            _ => {}
        }
    }
    pair.1
}

pub fn find_equalizer_hwnd() -> HWND {
    find_hwnd_by_kind(WindowKind::Equalizer)
}

pub fn find_mini_player_hwnd() -> HWND {
    find_hwnd_by_kind(WindowKind::MiniPlayer)
}

// --- Geometry Math Helpers ---

/// Determine which edge the Slave window is docked to relative to Master Window.
pub fn determine_dock_side(slave_rect: &RECT, master_rect: &RECT, tolerance: i32) -> DockSide {
    let d_right = (slave_rect.left - master_rect.right).abs();
    let d_left = (slave_rect.right - master_rect.left).abs();
    let d_bottom = (slave_rect.top - master_rect.bottom).abs();
    let d_top = (slave_rect.bottom - master_rect.top).abs();

    let mut min_d = tolerance + 1;
    let mut side = DockSide::None;

    if d_right <= tolerance && d_right < min_d {
        min_d = d_right;
        side = DockSide::Right;
    }
    if d_left <= tolerance && d_left < min_d {
        min_d = d_left;
        side = DockSide::Left;
    }
    if d_bottom <= tolerance && d_bottom < min_d {
        min_d = d_bottom;
        side = DockSide::Bottom;
    }
    if d_top <= tolerance && d_top < min_d {
        side = DockSide::Top;
    }

    side
}

/// Calculate bonded Slave position based on docked side and Master Window bounds.
pub fn calc_bonded_visualizer_pos(bond: &DockBond, slave_rect: &RECT, master_rect: &RECT) -> (i32, i32) {
    let slave_w = slave_rect.right - slave_rect.left;
    let slave_h = slave_rect.bottom - slave_rect.top;
    match bond.side {
        DockSide::Right => (master_rect.right, master_rect.top + bond.offset_y),
        DockSide::Left => (master_rect.left - slave_w, master_rect.top + bond.offset_y),
        DockSide::Bottom => (master_rect.left + bond.offset_x, master_rect.bottom),
        DockSide::Top => (master_rect.left + bond.offset_x, master_rect.top - slave_h),
        DockSide::None => (master_rect.left + bond.offset_x, master_rect.top + bond.offset_y),
    }
}

pub fn calc_bonded_main_pos(bond: &DockBond, vis_rect: &RECT, main_rect: &RECT) -> (i32, i32) {
    let main_w = main_rect.right - main_rect.left;
    let main_h = main_rect.bottom - main_rect.top;
    match bond.side {
        DockSide::Right => (vis_rect.left - main_w, vis_rect.top - bond.offset_y),
        DockSide::Left => (vis_rect.right, vis_rect.top - bond.offset_y),
        DockSide::Bottom => (vis_rect.left - bond.offset_x, vis_rect.top - main_h),
        DockSide::Top => (vis_rect.left - bond.offset_x, vis_rect.bottom),
        DockSide::None => (vis_rect.left - bond.offset_x, vis_rect.top - bond.offset_y),
    }
}

pub fn calc_bonded_slave_pos(
    side: DockSide,
    slave_rect: &RECT,
    master_rect: &RECT,
    offset_x: i32,
    offset_y: i32,
) -> (i32, i32) {
    let bond = DockBond {
        is_bonded: true,
        parent: WindowKind::Main,
        side,
        offset_x,
        offset_y,
    };
    calc_bonded_visualizer_pos(&bond, slave_rect, master_rect)
}

/// Query the current monitor's usable work area (excluding taskbar).
pub unsafe fn get_monitor_work_area(hwnd: HWND) -> RECT {
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut mi) != 0 {
        mi.rcWork
    } else {
        RECT {
            left: 0,
            top: 0,
            right: GetSystemMetrics(SM_CXSCREEN),
            bottom: GetSystemMetrics(SM_CYSCREEN),
        }
    }
}

/// Snap moving rectangle to monitor work area edges.
pub fn snap_rect_to_work_area(r: &mut RECT, work: &RECT, dist: i32) {
    let w = r.right - r.left;
    let h = r.bottom - r.top;

    if (r.left - work.left).abs() <= dist {
        r.left = work.left;
        r.right = r.left + w;
    } else if (r.right - work.right).abs() <= dist {
        r.right = work.right;
        r.left = r.right - w;
    }

    if (r.top - work.top).abs() <= dist {
        r.top = work.top;
        r.bottom = r.top + h;
    } else if (r.bottom - work.bottom).abs() <= dist {
        r.bottom = work.bottom;
        r.top = r.bottom - h;
    }
}

/// 磁吸彈簧阻尼運動曲線：在 t ~ 0.45 時產生 ~6-8% 的細微彈性超調 (overshoot)，隨後迅速平穩收斂鎖定，
/// 營造如 MagSafe 磁吸配件吸附時的機械彈性回彈手感。
pub fn spring_overshoot_curve(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let zeta = 5.2_f32; // 阻尼系數
    let omega = 7.2_f32; // 彈簧固有振動頻率
    let raw = 1.0_f32 - (-zeta * t).exp() * (omega * t).cos();
    let norm = 1.0_f32 - (-zeta).exp() * omega.cos();
    raw / norm
}

/// 階梯式彈簧展開函數：依據延遲時間 delay (0.0..1.0) 進行局部彈簧時間映射
pub fn staggered_spring(t_global: f32, delay: f32) -> f32 {
    if t_global <= delay {
        0.0
    } else {
        let t_local = ((t_global - delay) / (1.0 - delay)).clamp(0.0, 1.0);
        spring_overshoot_curve(t_local)
    }
}

/// 逆向向心彈簧曲線（方案 A：母體歸位心智）：強加速度、末端緊緻向心吸攏鎖定
pub fn reverse_spring_curve(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3) * (std::f32::consts::FRAC_PI_2 * t).cos()
}

/// 逆向階梯式向心延遲映射：最外層葉節點優先啟動收縮，中間層依序跟進
pub fn reverse_staggered_spring(t_global: f32, delay: f32) -> f32 {
    if t_global <= delay {
        0.0
    } else {
        let t_local = ((t_global - delay) / (1.0 - delay)).clamp(0.0, 1.0);
        reverse_spring_curve(t_local)
    }
}

/// Snap moving rectangle (A) to stationary target rectangle (B).
pub fn snap_rect_to_rect(r_a: &mut RECT, r_b: &RECT, dist: i32) -> bool {
    let w_a = r_a.right - r_a.left;
    let h_a = r_a.bottom - r_a.top;

    let v_overlap = r_a.bottom >= r_b.top - dist && r_a.top <= r_b.bottom + dist;
    let h_overlap = r_a.right >= r_b.left - dist && r_a.left <= r_b.right + dist;

    let mut snapped_x = false;
    let mut snapped_y = false;

    // Horizontal edge snap
    if v_overlap {
        if (r_a.left - r_b.right).abs() <= dist {
            r_a.left = r_b.right;
            r_a.right = r_a.left + w_a;
            snapped_x = true;
        } else if (r_a.right - r_b.left).abs() <= dist {
            r_a.right = r_b.left;
            r_a.left = r_a.right - w_a;
            snapped_x = true;
        }

        if snapped_x {
            let h_b = r_b.bottom - r_b.top;
            if (r_a.top - r_b.top).abs() <= dist * 2 {
                r_a.top = r_b.top;
                r_a.bottom = r_a.top + h_a;
            } else if (r_a.bottom - r_b.bottom).abs() <= dist * 2 && h_a == h_b {
                r_a.bottom = r_b.bottom;
                r_a.top = r_a.bottom - h_a;
            }
        }
    }

    // Vertical edge snap
    if h_overlap {
        if (r_a.top - r_b.bottom).abs() <= dist {
            r_a.top = r_b.bottom;
            r_a.bottom = r_a.top + h_a;
            snapped_y = true;
        } else if (r_a.bottom - r_b.top).abs() <= dist {
            r_a.bottom = r_b.top;
            r_a.top = r_a.bottom - h_a;
            snapped_y = true;
        }

        if snapped_y {
            let w_b = r_b.right - r_b.left;
            if (r_a.left - r_b.left).abs() <= dist {
                r_a.left = r_b.left;
                r_a.right = r_a.left + w_a;
            } else if (r_a.right - r_b.right).abs() <= dist && w_a == w_b {
                r_a.right = r_b.right;
                r_a.left = r_a.right - w_a;
            }
        }
    }

    snapped_x || snapped_y
}

/// Check if two rectangles are actively touching/docked.
pub fn are_rects_docked(r_a: &RECT, r_b: &RECT, tolerance: i32) -> bool {
    let h_touch = (r_a.right - r_b.left).abs() <= tolerance || (r_a.left - r_b.right).abs() <= tolerance;
    let v_overlap = r_a.bottom > r_b.top && r_a.top < r_b.bottom;
    if h_touch && v_overlap {
        return true;
    }

    let v_touch = (r_a.bottom - r_b.top).abs() <= tolerance || (r_a.top - r_b.bottom).abs() <= tolerance;
    let h_overlap = r_a.right > r_b.left && r_a.left < r_b.right;
    if v_touch && h_overlap {
        return true;
    }

    false
}

// --- Dock Tree Operations ---

/// Updates caption buttons (minimize, close) on a native Win32 window titlebar.
/// Retains _ and X at all times per user specification, while excluding the maximize box.
pub unsafe fn set_window_caption_buttons(hwnd: HWND, _visible: bool) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    // 無原生外框視窗（Frameless / WS_CAPTION == 0）無非客戶區按鈕，直接返回避免干擾樣式
    if current_style & WS_CAPTION == 0 {
        return;
    }
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

/// Generalized BFS Tree Builder from undirected adjacency graph.
/// In each connected component, the window with highest priority (min priority number)
/// is selected as Root, and BFS discovers directed parent -> child edges.
pub fn build_dock_tree_from_edges_and_rects(
    active_windows: &[WindowKind],
    edges: &[(WindowKind, WindowKind)],
    get_rect: impl Fn(WindowKind) -> Option<RECT>,
) -> HashMap<WindowKind, DockBond> {
    let mut adj: HashMap<WindowKind, Vec<WindowKind>> = HashMap::new();
    for &k in active_windows {
        adj.entry(k).or_default();
    }
    for &(a, b) in edges {
        if active_windows.contains(&a) && active_windows.contains(&b) {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }
    }

    let mut tree = HashMap::new();
    let mut visited: HashSet<WindowKind> = HashSet::new();

    let mut sorted_windows = active_windows.to_vec();
    sorted_windows.sort_by_key(|k| k.priority());

    for root in sorted_windows {
        if visited.contains(&root) {
            continue;
        }

        let mut queue = VecDeque::new();
        queue.push_back(root);
        visited.insert(root);

        while let Some(parent) = queue.pop_front() {
            let parent_rect = match get_rect(parent) {
                Some(r) => r,
                None => continue,
            };

            if let Some(neighbors) = adj.get(&parent) {
                for &child in neighbors {
                    if !visited.contains(&child) {
                        visited.insert(child);
                        queue.push_back(child);

                        if let Some(child_rect) = get_rect(child) {
                            let side = determine_dock_side(&child_rect, &parent_rect, 24);
                            if side != DockSide::None {
                                let offset_x = child_rect.left - parent_rect.left;
                                let offset_y = child_rect.top - parent_rect.top;

                                tree.insert(
                                    child,
                                    DockBond {
                                        is_bonded: true,
                                        parent,
                                        side,
                                        offset_x,
                                        offset_y,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    tree
}

pub unsafe fn rebuild_dock_tree() {
    let mut active = Vec::new();
    let mut rects: HashMap<WindowKind, RECT> = HashMap::new();

    for &kind in &ALL_WINDOW_KINDS {
        if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
            continue;
        }
        let hwnd = get_window_hwnd(kind);
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            if kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                continue;
            }
            // 使用 get_window_normal_rect 獲取真實常態坐標：
            // 即便視窗當前處於最小化（Iconic）或叢集收攏隱藏（SW_HIDE），依然讀取其 rcNormalPosition 或快取坐標，
            // 絕不可將已最小化或收攏的視窗自拓撲叢集中剃除！
            let r = get_window_normal_rect(kind);
            if (r.right - r.left) > 50 && (r.bottom - r.top) > 50 {
                rects.insert(kind, r);
                active.push(kind);
                update_known_rect(kind, r);
            }
        }
    }

    #[cfg(test)]
    {
        if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
            if let Some(ref map) = *guard {
                for (&k, &r) in map {
                    if !active.contains(&k) {
                        rects.insert(k, r);
                        active.push(k);
                    }
                }
            }
        }
    }

    if active.is_empty() {
        with_dock_tree(|tree| {
            tree.clear();
        });
        return;
    }

    let edges: Vec<(WindowKind, WindowKind)> = with_dock_edges(|edge_set| {
        edge_set.retain(|&(a, b)| {
            if !active.contains(&a) || !active.contains(&b) {
                return false;
            }
            // 關鍵不變量保護：若視窗或其所屬叢集正處於收攏隱藏、最小化、或正在進行過渡動畫，
            // 嚴禁在重新拓撲時破壞該磁吸關係鏈！
            let is_in_collapsed_or_anim = |k: WindowKind| -> bool {
                let root = get_component_root(k);
                matches!(get_cluster_state(root), ClusterState::Minimized)
                    || is_kind_cluster_minimizing(k)
                    || is_kind_cluster_restoring(k)
                    || {
                        let h = get_window_hwnd(k);
                        !h.is_null() && unsafe { IsIconic(h) != 0 || IsWindowVisible(h) == 0 }
                    }
            };
            if is_in_collapsed_or_anim(a) || is_in_collapsed_or_anim(b) {
                return true;
            }

            if let (Some(r_a), Some(r_b)) = (rects.get(&a), rects.get(&b)) {
                are_rects_docked(r_a, r_b, 24)
            } else {
                false
            }
        });

        edge_set.iter().copied().collect()
    });

    let new_tree = build_dock_tree_from_edges_and_rects(
        &active,
        &edges,
        |k| rects.get(&k).copied(),
    );

    with_dock_tree(|tree| {
        *tree = new_tree.clone();
    });

    // Update caption buttons: Root/standalone -> visible (true), Slave -> hidden (false)
    for &kind in &active {
        let hwnd = get_window_hwnd(kind);
        if !hwnd.is_null() && IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0 {
            let is_slave = new_tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false);
            set_window_caption_buttons(hwnd, !is_slave);
        }
    }

    // Update visualizer title with magnetic dock symbol (📎 / 🧲 / none)
    let vis_hwnd = get_window_hwnd(WindowKind::Visualizer);
    if !vis_hwnd.is_null() && IsWindow(vis_hwnd) != 0 && IsWindowVisible(vis_hwnd) != 0 && IsIconic(vis_hwnd) == 0 {
        crate::visualizer::update_visualizer_title(vis_hwnd);
    }

    // 提案一：動態母子視窗工作列歸一同步（所有附屬視窗自動自工作列整併隱藏）
    sync_cluster_taskbar_ownership();
}

pub fn set_dock_relation(child: WindowKind, parent: WindowKind, child_rect: &RECT, parent_rect: &RECT) {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    update_known_rect(child, *child_rect);
    update_known_rect(parent, *parent_rect);
    add_dock_edge(child, parent);
    unsafe {
        rebuild_dock_tree();
        assimilate_cluster_to_anchor(child, parent);
    }
}

pub fn break_dock_relation(child: WindowKind) {
    let parent = with_dock_tree(|tree| {
        tree.get(&child).and_then(|b| if b.is_bonded { Some(b.parent) } else { None })
    });
    if let Some(p) = parent {
        remove_dock_edge(child, p);
    } else {
        remove_all_edges_for(child);
    }
    unsafe {
        rebuild_dock_tree();
    }
}

/// When any window is closed (or hidden to tray), cleanly removes all dock edges
/// connecting to it, checks if any remaining active windows are touching each other
/// to synthesize edges between them, and rebuilds the dock tree in-place.
pub fn handle_window_closed(closed_kind: WindowKind) {
    remove_all_edges_for(closed_kind);

    unsafe {
        // Collect current rects of all remaining active windows
        let mut active = Vec::new();
        let mut rects: HashMap<WindowKind, RECT> = HashMap::new();

        for &kind in &ALL_WINDOW_KINDS {
            if kind == closed_kind {
                continue;
            }
            if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
                continue;
            }
            let hwnd = get_window_hwnd(kind);
            if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                if kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                    continue;
                }
                let r = get_window_normal_rect(kind);
                if (r.right - r.left) > 50 && (r.bottom - r.top) > 50 {
                    rects.insert(kind, r);
                    active.push(kind);
                }
            }
        }

        #[cfg(test)]
        {
            if active.is_empty() {
                if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
                    if let Some(ref map) = *guard {
                        for (&k, &r) in map {
                            if k != closed_kind {
                                if k == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
                                    continue;
                                }
                                rects.insert(k, r);
                                active.push(k);
                            }
                        }
                    }
                }
            }
        }

        // Check if any pairs of remaining active windows are physically touching each other!
        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                let a = active[i];
                let b = active[j];
                if let (Some(r_a), Some(r_b)) = (rects.get(&a), rects.get(&b)) {
                    if are_rects_docked(r_a, r_b, 24) {
                        add_dock_edge(a, b);
                    }
                }
            }
        }

        rebuild_dock_tree();
    }
}

/// Recursively checks if `maybe_child` is a descendant of `ancestor` in the dock tree.
pub fn is_descendant(maybe_child: WindowKind, ancestor: WindowKind) -> bool {
    if maybe_child == ancestor {
        return true;
    }
    with_dock_tree(|tree| {
        let mut curr = maybe_child;
        let mut visited = HashSet::new();
        while let Some(bond) = tree.get(&curr) {
            if !bond.is_bonded || visited.contains(&curr) {
                break;
            }
            if bond.parent == ancestor {
                return true;
            }
            visited.insert(curr);
            curr = bond.parent;
        }
        false
    })
}

/// Determines the root window of the component containing `kind`.
pub fn get_component_root(kind: WindowKind) -> WindowKind {
    with_dock_tree(|tree| {
        let mut curr = kind;
        let mut visited = HashSet::new();
        while let Some(bond) = tree.get(&curr) {
            if !bond.is_bonded || visited.contains(&curr) {
                break;
            }
            visited.insert(curr);
            curr = bond.parent;
        }
        curr
    })
}

/// Returns true if `kind` is currently part of the cluster rooted at Main Window.
pub fn is_in_cluster_with_main(kind: WindowKind) -> bool {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        return false;
    }
    let is_bonded = with_dock_tree(|tree| {
        tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false)
    });
    if !is_bonded {
        return false;
    }
    get_component_root(kind) == WindowKind::Main
}

/// Retrieves all windows belonging to the same cluster (connected component) as `kind`.
/// If `kind` is detached / standalone, returns `vec![kind]`.
pub fn get_cluster_members(kind: WindowKind) -> Vec<WindowKind> {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        return vec![kind];
    }
    with_dock_tree(|tree| {
        let is_bonded = tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false);
        let has_children = tree.values().any(|b| b.is_bonded && b.parent == kind);
        if !is_bonded && !has_children {
            return vec![kind];
        }

        let root = {
            let mut curr = kind;
            let mut visited = HashSet::new();
            while let Some(bond) = tree.get(&curr) {
                if !bond.is_bonded || visited.contains(&curr) {
                    break;
                }
                visited.insert(curr);
                curr = bond.parent;
            }
            curr
        };

        let mut members = Vec::new();
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(root);
        visited.insert(root);
        members.push(root);

        while let Some(parent) = queue.pop_front() {
            for (&child, bond) in tree.iter() {
                if bond.is_bonded && bond.parent == parent && !visited.contains(&child) {
                    visited.insert(child);
                    queue.push_back(child);
                    members.push(child);
                }
            }
        }

        members
    })
}

static BRINGING_CLUSTER_TO_TOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterState {
    Normal,
    Minimizing(std::time::Instant),
    Minimized,
    Restoring(std::time::Instant),
}

static CLUSTER_FSM: Mutex<Option<HashMap<WindowKind, ClusterState>>> = Mutex::new(None);

pub fn get_cluster_state(root: WindowKind) -> ClusterState {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let state = map.entry(root).or_insert(ClusterState::Normal);
    match *state {
        ClusterState::Minimizing(start) => {
            if start.elapsed() > std::time::Duration::from_millis(450) {
                *state = ClusterState::Minimized;
            }
        }
        ClusterState::Restoring(start) => {
            if start.elapsed() > std::time::Duration::from_millis(450) {
                *state = ClusterState::Normal;
            }
        }
        _ => {}
    }
    *state
}

pub fn set_cluster_state(root: WindowKind, new_state: ClusterState) {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(root, new_state);
}

pub fn can_start_cluster_minimize(root: WindowKind) -> bool {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let state = map.entry(root).or_insert(ClusterState::Normal);
    match *state {
        ClusterState::Normal => {
            *state = ClusterState::Minimizing(std::time::Instant::now());
            true
        }
        ClusterState::Minimizing(start) => {
            if start.elapsed() > std::time::Duration::from_millis(450) {
                *state = ClusterState::Minimizing(std::time::Instant::now());
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn can_start_cluster_restore(root: WindowKind) -> bool {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let state = map.entry(root).or_insert(ClusterState::Normal);
    match *state {
        ClusterState::Minimized => {
            *state = ClusterState::Restoring(std::time::Instant::now());
            true
        }
        ClusterState::Restoring(start) => {
            if start.elapsed() > std::time::Duration::from_millis(450) {
                *state = ClusterState::Restoring(std::time::Instant::now());
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn finish_cluster_minimize(root: WindowKind) {
    set_cluster_state(root, ClusterState::Minimized);
}

pub fn finish_cluster_restore(root: WindowKind) {
    set_cluster_state(root, ClusterState::Normal);
}

pub fn is_cluster_minimizing() -> bool {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.values().any(|&s| match s {
            ClusterState::Minimizing(start) => start.elapsed() <= std::time::Duration::from_millis(450),
            _ => false,
        })
    } else {
        false
    }
}

pub fn is_cluster_restoring() -> bool {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.values().any(|&s| match s {
            ClusterState::Restoring(start) => start.elapsed() <= std::time::Duration::from_millis(450),
            _ => false,
        })
    } else {
        false
    }
}

pub fn is_kind_cluster_minimizing(kind: WindowKind) -> bool {
    let root = get_component_root(kind);
    matches!(get_cluster_state(root), ClusterState::Minimizing(_))
}

pub fn is_kind_cluster_restoring(kind: WindowKind) -> bool {
    let root = get_component_root(kind);
    matches!(get_cluster_state(root), ClusterState::Restoring(_))
}

pub fn clear_all_cluster_fsm() {
    let mut guard = CLUSTER_FSM.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(map) = guard.as_mut() {
        map.clear();
    }
}

/// Brings all windows belonging to the same cluster as `active_kind` to the top of the Z-order,
/// stacking them contiguously without allowing any third-party windows in between.
pub unsafe fn bring_cluster_to_top(active_kind: WindowKind, active_hwnd: HWND) {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) || is_cluster_minimizing() || is_cluster_restoring() {
        return;
    }
    if active_hwnd.is_null() || IsWindow(active_hwnd) == 0 || IsIconic(active_hwnd) != 0 {
        return;
    }
    if BRINGING_CLUSTER_TO_TOP.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }

    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            BRINGING_CLUSTER_TO_TOP.store(false, Ordering::SeqCst);
        }
    }
    let _guard = Guard;

    let members = get_cluster_members(active_kind);
    if members.len() <= 1 {
        return;
    }

    let is_fullscreen_hud = crate::visualizer::is_visualizer_fullscreen() && crate::visualizer::is_hud_shown();
    let is_topmost = is_window_pinned(active_kind) || is_fullscreen_hud;
    let top_tier = if is_topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };

    // Ensure active window is at the top of its Z-order tier
    SetWindowPos(
        active_hwnd,
        top_tier,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );

    let mut prev_hwnd = active_hwnd;
    for &member_kind in &members {
        if member_kind == active_kind {
            continue;
        }
        let m_hwnd = get_window_hwnd(member_kind);
        if m_hwnd.is_null()
            || IsWindow(m_hwnd) == 0
            || IsIconic(m_hwnd) != 0
            || (member_kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray())
        {
            continue;
        }
        if member_kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
            continue;
        }
        let target_pos = if is_topmost { HWND_TOPMOST } else { prev_hwnd };
        if !is_topmost {
            SetWindowPos(
                m_hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        SetWindowPos(
            m_hwnd,
            target_pos,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        prev_hwnd = m_hwnd;
    }
}

/// Recursively moves all child windows attached to `parent_kind` to maintain their relative position.
pub unsafe fn propagate_child_moves(parent_kind: WindowKind, parent_rect: &RECT) {
    let children: Vec<(WindowKind, DockBond)> = with_dock_tree(|tree| {
        tree.iter()
            .filter(|(_, bond)| bond.is_bonded && bond.parent == parent_kind)
            .map(|(&k, &b)| (k, b))
            .collect()
    });

    for (child_kind, bond) in children {
        let child_hwnd = get_window_hwnd(child_kind);
        if child_hwnd.is_null()
            || IsWindow(child_hwnd) == 0
            || IsWindowVisible(child_hwnd) == 0
            || (child_kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray())
        {
            continue;
        }
        if child_kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
            continue;
        }

        let mut child_rect: RECT = std::mem::zeroed();
        GetWindowRect(child_hwnd, &mut child_rect);
        let (new_x, new_y) = calc_bonded_visualizer_pos(&bond, &child_rect, parent_rect);
        let child_w = child_rect.right - child_rect.left;
        let child_h = child_rect.bottom - child_rect.top;

        if child_rect.left != new_x || child_rect.top != new_y {
            SetWindowPos(
                child_hwnd,
                std::ptr::null_mut(),
                new_x,
                new_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        let updated_child_rect = RECT {
            left: new_x,
            top: new_y,
            right: new_x + child_w,
            bottom: new_y + child_h,
        };

        // Recurse for downstream children
        propagate_child_moves(child_kind, &updated_child_rect);
    }
}

pub unsafe fn minimize_all_children(parent_kind: WindowKind) {
    let children: Vec<WindowKind> = with_dock_tree(|tree| {
        tree.iter()
            .filter(|(_, bond)| bond.is_bonded && bond.parent == parent_kind)
            .map(|(&k, _)| k)
            .collect()
    });

    for child in children {
        let child_hwnd = get_window_hwnd(child);
        if !child_hwnd.is_null() && IsWindow(child_hwnd) != 0 && IsWindowVisible(child_hwnd) != 0 {
            if child == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                continue;
            }
            ShowWindow(child_hwnd, SW_MINIMIZE);
            minimize_all_children(child);
        }
    }
}

pub unsafe fn restore_and_realign_all_children(parent_kind: WindowKind, parent_hwnd: HWND) {
    let mut parent_rect: RECT = std::mem::zeroed();
    GetWindowRect(parent_hwnd, &mut parent_rect);

    let children: Vec<(WindowKind, DockBond)> = with_dock_tree(|tree| {
        tree.iter()
            .filter(|(_, b)| b.is_bonded && b.parent == parent_kind)
            .map(|(&k, &b)| (k, b))
            .collect()
    });

    for (child_kind, bond) in children {
        let child_hwnd = get_window_hwnd(child_kind);
        if child_hwnd.is_null() || IsWindow(child_hwnd) == 0 {
            continue;
        }
        if child_kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
            continue;
        }
        if IsIconic(child_hwnd) != 0 || IsWindowVisible(child_hwnd) == 0 {
            ShowWindow(child_hwnd, SW_SHOWNOACTIVATE);
            if child_kind == WindowKind::Visualizer {
                crate::visualizer::restore_visualizer_with_cluster();
            }
        }
        let mut child_rect: RECT = std::mem::zeroed();
        GetWindowRect(child_hwnd, &mut child_rect);
        let (new_x, new_y) = calc_bonded_visualizer_pos(&bond, &child_rect, &parent_rect);
        SetWindowPos(
            child_hwnd,
            parent_hwnd,
            new_x,
            new_y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
        let w = child_rect.right - child_rect.left;
        let h = child_rect.bottom - child_rect.top;
        let _updated_child_rect = RECT {
            left: new_x,
            top: new_y,
            right: new_x + w,
            bottom: new_y + h,
        };
        restore_and_realign_all_children(child_kind, child_hwnd);
    }
}

/// 安全獲取指定視窗在常態（未最小化）時的螢幕坐標矩形
pub unsafe fn get_window_normal_rect(kind: WindowKind) -> RECT {
    let min_s = kind.min_size();
    let def_s = kind.default_size();
    let min_w = min_s[0] as i32;
    let min_h = min_s[1] as i32;
    let def_w = def_s[0] as i32;
    let def_h = def_s[1] as i32;

    let hwnd = get_window_hwnd(kind);
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
            if let Some(map) = guard.as_ref() {
                if let Some(kr) = map.get(&kind) {
                    let w = (kr.right - kr.left).max(min_w);
                    let h = (kr.bottom - kr.top).max(min_h);
                    return RECT {
                        left: kr.left,
                        top: kr.top,
                        right: kr.left + w,
                        bottom: kr.top + h,
                    };
                }
            }
        }
        return RECT {
            left: 100,
            top: 100,
            right: 100 + def_w,
            bottom: 100 + def_h,
        };
    }
    let is_iconic = IsIconic(hwnd) != 0;
    let mut r: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut r);

    // 若視窗處於最小化，或 GetWindowRect 回傳負坐標（如被神燈離屏移至 -32000），一律優先回退至 rcNormalPosition
    if is_iconic || r.left < -1000 || r.top < -1000 || (r.right - r.left) < min_w || (r.bottom - r.top) < min_h {
        let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
        wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        if GetWindowPlacement(hwnd, &mut wp) != 0 {
            let mut pr = wp.rcNormalPosition;
            if pr.left >= -1000 && pr.top >= -1000 {
                let pw = (pr.right - pr.left).max(min_w);
                let ph = (pr.bottom - pr.top).max(min_h);
                pr.right = pr.left + pw;
                pr.bottom = pr.top + ph;
                return pr;
            }
        }
        if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
            if let Some(map) = guard.as_ref() {
                if let Some(kr) = map.get(&kind) {
                    if kr.left >= -1000 && kr.top >= -1000 {
                        let kw = (kr.right - kr.left).max(min_w);
                        let kh = (kr.bottom - kr.top).max(min_h);
                        return RECT {
                            left: kr.left,
                            top: kr.top,
                            right: kr.left + kw,
                            bottom: kr.top + kh,
                        };
                    }
                }
            }
        }
    }

    if r.left >= -1000 && r.top >= -1000 {
        let w = (r.right - r.left).max(min_w);
        let h = (r.bottom - r.top).max(min_h);
        RECT {
            left: r.left,
            top: r.top,
            right: r.left + w,
            bottom: r.top + h,
        }
    } else {
        RECT {
            left: 100,
            top: 100,
            right: 100 + def_w,
            bottom: 100 + def_h,
        }
    }
}

const GWLP_HWNDPARENT: i32 = -8;

#[repr(C)]
#[allow(non_snake_case)]
struct ITaskbarListVtbl {
    pub QueryInterface: unsafe extern "system" fn(this: *mut std::ffi::c_void, riid: *const windows_sys::core::GUID, ppvObject: *mut *mut std::ffi::c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32,
    pub HrInit: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> i32,
    pub AddTab: unsafe extern "system" fn(this: *mut std::ffi::c_void, hwnd: HWND) -> i32,
    pub DeleteTab: unsafe extern "system" fn(this: *mut std::ffi::c_void, hwnd: HWND) -> i32,
    pub ActivateTab: unsafe extern "system" fn(this: *mut std::ffi::c_void, hwnd: HWND) -> i32,
    pub SetActiveAlt: unsafe extern "system" fn(this: *mut std::ffi::c_void, hwnd: HWND) -> i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct ITaskbarList {
    pub lpVtbl: *const ITaskbarListVtbl,
}

const CLSID_TASKBAR_LIST: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x56FDF344,
    data2: 0xFD6D,
    data3: 0x11D0,
    data4: [0x95, 0x8A, 0x00, 0x60, 0x97, 0xC9, 0xA0, 0x90],
};

const IID_ITASKBAR_LIST: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x56FDF342,
    data2: 0xFD6D,
    data3: 0x11D0,
    data4: [0x95, 0x8A, 0x00, 0x60, 0x97, 0xC9, 0xA0, 0x90],
};

pub unsafe fn delete_taskbar_tab_pub(hwnd: HWND) {
    delete_taskbar_tab(hwnd);
}

pub unsafe fn add_taskbar_tab_pub(hwnd: HWND) {
    add_taskbar_tab(hwnd);
}

unsafe fn delete_taskbar_tab(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let mut obj: *mut std::ffi::c_void = std::ptr::null_mut();
    if windows_sys::Win32::System::Com::CoCreateInstance(
        &CLSID_TASKBAR_LIST,
        std::ptr::null_mut(),
        windows_sys::Win32::System::Com::CLSCTX_INPROC_SERVER,
        &IID_ITASKBAR_LIST,
        &mut obj,
    ) == 0 && !obj.is_null() {
        let tbl = obj as *mut ITaskbarList;
        let _ = ((*(*tbl).lpVtbl).HrInit)(obj);
        let _ = ((*(*tbl).lpVtbl).DeleteTab)(obj, hwnd);
        let _ = ((*(*tbl).lpVtbl).Release)(obj);
    }
}

unsafe fn add_taskbar_tab(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let mut obj: *mut std::ffi::c_void = std::ptr::null_mut();
    if windows_sys::Win32::System::Com::CoCreateInstance(
        &CLSID_TASKBAR_LIST,
        std::ptr::null_mut(),
        windows_sys::Win32::System::Com::CLSCTX_INPROC_SERVER,
        &IID_ITASKBAR_LIST,
        &mut obj,
    ) == 0 && !obj.is_null() {
        let tbl = obj as *mut ITaskbarList;
        let _ = ((*(*tbl).lpVtbl).HrInit)(obj);
        let _ = ((*(*tbl).lpVtbl).AddTab)(obj, hwnd);
        let _ = ((*(*tbl).lpVtbl).Release)(obj);
    }
}

/// 提案一：動態母子視窗歸一 (Dynamic Cluster Taskbar Aggregation)
/// 當視窗為附屬地位時，將 Win32 擁有者指向根節點視窗，並從工作列與 Alt+Tab 中整併刪除；
/// 當視窗為獨立或主控地位時，恢復其為頂層視窗與工作列/Alt+Tab 代表卡片。
/// 動態母子視窗群組整併 (Dynamic Cluster Taskbar & Alt+Tab Grouping)
/// 當視窗為附屬地位 (is_slave) 時，加上 WS_EX_TOOLWINDOW 並從工作列與 Alt+Tab 中整併隱藏；
/// 當視窗為獨立或主控地位 (!is_slave) 時，恢復為頂層應用程式視窗 (WS_EX_APPWINDOW)，
/// 擁有工作列與 Alt+Tab 獨立代表卡片。
/// 注意：絕不修改 GWLP_HWNDPARENT，以避免 Windows 核心 spwndOwner 遭到污染導致 Alt+Tab 行為不一致。
pub unsafe fn set_window_taskbar_grouped(child_hwnd: HWND, _root_hwnd: HWND, is_slave: bool) {
    if child_hwnd.is_null() || IsWindow(child_hwnd) == 0 {
        return;
    }

    // 防護：若視窗當前不可見，嚴禁調用 SetWindowPos(SWP_FRAMECHANGED)！
    // 否則 winit Win32 後端在處理 WM_WINDOWPOSCHANGED 時執行 ClientToScreen 失敗會引發致命 panic (exit code 101)。
    if IsWindowVisible(child_hwnd) == 0 {
        return;
    }

    if is_slave {
        let ex_style = GetWindowLongW(child_hwnd, GWL_EXSTYLE) as u32;
        let new_ex = (ex_style | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
        if ex_style != new_ex {
            SetWindowLongW(child_hwnd, GWL_EXSTYLE, new_ex as i32);
        }

        delete_taskbar_tab(child_hwnd);

        SetWindowPos(
            child_hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );
    } else {
        // 清理任何殘留的 Win32 父級擁有者，還原純淨頂層無主視窗
        let cur_owner = GetWindowLongPtrW(child_hwnd, GWLP_HWNDPARENT);
        if cur_owner != 0 {
            SetWindowLongPtrW(child_hwnd, GWLP_HWNDPARENT, 0);
        }

        let ex_style = GetWindowLongW(child_hwnd, GWL_EXSTYLE) as u32;
        let new_ex = (ex_style & !WS_EX_TOOLWINDOW) | WS_EX_APPWINDOW;
        if ex_style != new_ex {
            SetWindowLongW(child_hwnd, GWL_EXSTYLE, new_ex as i32);
        }

        add_taskbar_tab(child_hwnd);

        SetWindowPos(
            child_hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );
    }
}

/// 同步全系統所有視窗的工作列與 Alt+Tab 縮圖狀態（群組獨立代表制）：
/// 1. 各獨立群組的 Root / Master / 獨立視窗：在工作列與 Alt+Tab 擁有專屬卡片 (is_slave = false)；
/// 2. 分成兩個獨立群組時，工作列清晰顯示兩個預覽視窗，各代表所屬群組；
/// 3. 磁吸群組內的附屬視窗 (Slave)：掛載於各自群組的 Root 底下，自工作列整併隱藏 (is_slave = true)。
pub unsafe fn sync_cluster_taskbar_ownership() {
    let tree = with_dock_tree(|t| t.clone());

    for &kind in &ALL_WINDOW_KINDS {
        let hwnd = get_window_hwnd(kind);
        if hwnd.is_null() || IsWindow(hwnd) == 0 {
            continue;
        }

        // 當主視窗收納至托盤時（如畫面上只留迷你播放器），徹底自工作列與 Alt+Tab 移除，絕不產生幽靈縮圖
        // 且嚴禁對隱藏視窗調用 SetWindowPos(SWP_FRAMECHANGED)，以防 winit 在 ClientToScreen 處 panic
        if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
            delete_taskbar_tab(hwnd);
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let new_ex = (ex | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
            if ex != new_ex {
                SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex as i32);
            }
            continue;
        }

        // 依據各群組的 Master / Slave 地位決定：
        // - Root / Master / 獨立視窗：在工作列與 Alt+Tab 擁有專屬卡片 (is_slave = false)
        // - 磁吸群組附屬視窗 (Slave)：掛載於各自群組的 Root 底下，自工作列整併隱藏 (is_slave = true)
        let is_slave = tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false);
        if is_slave {
            let root = get_component_root(kind);
            let root_hwnd = get_window_hwnd(root);
            if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 && root_hwnd != hwnd {
                set_window_taskbar_grouped(hwnd, root_hwnd, true);
            }
        } else {
            set_window_taskbar_grouped(hwnd, std::ptr::null_mut(), false);
        }
    }
}

/// 關閉 DWM Iconic 模式，讓 Windows 原生 GPU 引擎自動為視窗即時生成無延遲、絕不轉圈的縮圖
pub unsafe fn disable_dwm_iconic_preview(hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let val: i32 = 0;
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_FORCE_ICONIC_REPRESENTATION as u32,
        &val as *const _ as _,
        std::mem::size_of::<i32>() as u32,
    );
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_HAS_ICONIC_BITMAP as u32,
        &val as *const _ as _,
        std::mem::size_of::<i32>() as u32,
    );
}


/// 以主控視窗為向心核心（方案 A），執行逆向彈簧吸攏過場動畫並連鎖最小化群組視窗
pub unsafe fn collapse_cluster_animated(kind: WindowKind) {
    crate::visualizer::reset_hud_main_shown();
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        let hwnd = get_window_hwnd(kind);
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            minimize_window_solo(kind);
        }
        return;
    }

    let root = get_component_root(kind);
    if !can_start_cluster_minimize(root) {
        return;
    }

    let members = get_cluster_members(root);

    if members.len() <= 1 {
        finish_cluster_minimize(root);
        let hwnd = get_window_hwnd(kind);
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            minimize_window_solo(kind);
        }
        return;
    }

    // 1. 獲取根節點主視窗幾何 Rect（作為向心塌縮的終點目標）
    let root_hwnd = get_window_hwnd(root);
    if root_hwnd.is_null() || IsWindow(root_hwnd) == 0 {
        finish_cluster_minimize(root);
        return;
    }
    let root_r = get_window_normal_rect(root);
    let root_hwnd_val = root_hwnd as isize;

    // 將主視窗拉至最上層，讓子視窗向心收納時自然滑入主視窗背後
    SetWindowPos(
        root_hwnd,
        HWND_TOP,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );

    // 3. 收集子視窗起始 Rect 與原始停靠位置
    #[derive(Clone)]
    struct ChildAnimData {
        hwnd_val: isize,
        child_kind: WindowKind,
        start_r: RECT,
        delay: f32,
    }

    let mut children: Vec<ChildAnimData> = Vec::new();
    for &m in &members {
        if m == root {
            continue;
        }
        let m_hwnd = get_window_hwnd(m);
        if !m_hwnd.is_null() && IsWindow(m_hwnd) != 0 && IsWindowVisible(m_hwnd) != 0 && IsIconic(m_hwnd) == 0 {
            let start_r = get_window_normal_rect(m);
            // 階梯延遲：外層葉節點（Visualizer / Equalizer）先動，中間層（MiniPlayer）後動
            let delay = match m {
                WindowKind::Visualizer => 0.00,
                WindowKind::Equalizer => 0.02,
                WindowKind::MiniPlayer => 0.04,
                _ => 0.0,
            };
            children.push(ChildAnimData {
                hwnd_val: m_hwnd as isize,
                child_kind: m,
                start_r,
                delay,
            });
        }
    }

    if children.is_empty() {
        if members.len() <= 1 {
            finish_cluster_minimize(root);
            minimize_window_solo(root);
            return;
        } else {
            // 群組子視窗若已處於收攏或隱藏狀態，切勿呼叫 minimize_window_solo 破壞磁吸關係鏈！
            // 直接由母體安全執行最小化收納
            let m_h = root_hwnd;
            if !m_h.is_null() && IsWindow(m_h) != 0 {
                crate::genie_effect::disable_window_dwm_transitions(m_h);
                let (hbmp, rect) = crate::genie_effect::capture_window_snapshot(m_h);
                if hbmp != 0 {
                    crate::genie_effect::play_genie_s_curve_effect_with_finish(
                        hbmp,
                        m_h as usize,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        crate::genie_effect::GenieTarget::Taskbar,
                        move || {
                            finish_cluster_minimize(root);
                        },
                    );
                } else {
                    ShowWindow(m_h, SW_MINIMIZE);
                    finish_cluster_minimize(root);
                }
            } else {
                finish_cluster_minimize(root);
            }
            return;
        }
    }

    // 4. 在背景執行緒執行 8 步（約 88ms）極速逆向向心彈簧過場動畫
    std::thread::spawn(move || {
        let steps = 8;
        for step in 1..=steps {
            let t_global = step as f32 / steps as f32;

            for child in &children {
                let f = reverse_staggered_spring(t_global, child.delay);
                let cur_x = (child.start_r.left as f32 + (root_r.left - child.start_r.left) as f32 * f).round() as i32;
                let cur_y = (child.start_r.top as f32 + (root_r.top - child.start_r.top) as f32 * f).round() as i32;

                unsafe {
                    let h = child.hwnd_val as HWND;
                    if IsWindow(h) != 0 {
                        SetWindowPos(
                            h,
                            std::ptr::null_mut(),
                            cur_x,
                            cur_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(11));
        }

        // 動畫結束：子視窗已完全滑入主視窗背後，執行靜默收納與位置保護
        unsafe {
            for child in &children {
                let h = child.hwnd_val as HWND;
                if IsWindow(h) != 0 {
                    // 關鍵保護：將 rcNormalPosition 設回原始停靠坐標，確保日後還原時自動歸位
                    let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
                    wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
                    if GetWindowPlacement(h, &mut wp) != 0 {
                        wp.rcNormalPosition = child.start_r;
                        SetWindowPlacement(h, &wp);
                    }

                    // 徹底根除往左下角飛：子視窗在母體背後直接 SW_HIDE 靜默收納，不觸發 DWM 經典左下角飛行動畫！
                    if child.child_kind == WindowKind::Visualizer {
                        crate::visualizer::hide_visualizer_with_cluster();
                    } else {
                        ShowWindow(h, SW_HIDE);
                    }
                    update_known_rect(child.child_kind, child.start_r);
                }
            }

            // 最後最小化主視窗（母體）：由母體代表整個群組單一、流暢地飛向工作列圖示！
            let m_h = root_hwnd_val as HWND;
            if IsWindow(m_h) != 0 {
                crate::genie_effect::disable_window_dwm_transitions(m_h);
                let (hbmp, rect) = crate::genie_effect::capture_window_snapshot(m_h);
                if hbmp != 0 {
                    crate::genie_effect::play_genie_s_curve_effect_with_finish(
                        hbmp,
                        root_hwnd_val as usize,
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                        crate::genie_effect::GenieTarget::Taskbar,
                        move || {
                            finish_cluster_minimize(root);
                        },
                    );
                } else {
                    ShowWindow(m_h, SW_MINIMIZE);
                    finish_cluster_minimize(root);
                }
            } else {
                finish_cluster_minimize(root);
            }
        }
    });
}

/// Minimizes all windows belonging to the same magnetic cluster safely without focus-handover bouncing.
pub unsafe fn minimize_cluster(kind: WindowKind) {
    collapse_cluster_animated(kind);
}

/// Minimizes a single window independently from its cluster by breaking its dock bond first.
/// Keeps all remaining cluster members intact on screen without chain-minimizing them.
pub unsafe fn minimize_window_solo(kind: WindowKind) {
    break_dock_relation(kind);
    rebuild_dock_tree();
    let hwnd = get_window_hwnd(kind);
    if !hwnd.is_null() && IsWindow(hwnd) != 0 {
        if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
            return;
        }
        crate::genie_effect::disable_window_dwm_transitions(hwnd);
        let (hbmp, rect) = crate::genie_effect::capture_window_snapshot(hwnd);
        if hbmp != 0 {
            crate::genie_effect::play_genie_s_curve_effect(
                hbmp,
                hwnd as usize,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                crate::genie_effect::GenieTarget::Taskbar,
            );
        } else {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
    }
}

/// Restores a window and its cluster using reverse Genie S-curve transition from taskbar
pub unsafe fn restore_window_animated(kind: WindowKind, hwnd: HWND) {
    let root = get_component_root(kind);
    let root_hwnd = get_window_hwnd(root);
    let target_hwnd = if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 { root_hwnd } else { hwnd };
    let normal_r = get_window_normal_rect(root);
    let was_collapsed = can_start_cluster_restore(root);

    let target_hwnd_val = target_hwnd as usize;
    let on_finish = move || {
        let h = target_hwnd_val as HWND;
        unsafe {
            if IsWindow(h) != 0 {
                crate::genie_effect::disable_window_dwm_transitions(h);
                ShowWindow(h, SW_RESTORE);

                let members = get_cluster_members(root);
                let has_hidden_or_iconic_children = members.iter().any(|&m| {
                    if m == root {
                        return false;
                    }
                    let m_h = get_window_hwnd(m);
                    !m_h.is_null() && IsWindow(m_h) != 0 && (IsIconic(m_h) != 0 || IsWindowVisible(m_h) == 0)
                });

                if was_collapsed || has_hidden_or_iconic_children {
                    expand_cluster_animated(root, h);
                } else {
                    bring_cluster_to_top(root, h);
                    finish_cluster_restore(root);
                }
            }
        }
    };

    crate::genie_effect::play_reverse_genie_s_curve_effect(
        normal_r.left,
        normal_r.top,
        target_hwnd as usize,
        crate::genie_effect::GenieTarget::Taskbar,
        on_finish,
    );
}

/// 根據當前 Dock Tree，以 `root` 的坐標為基準，遞迴計算出該群組所有附屬子視窗的目標螢幕停靠坐標。
pub fn compute_cluster_docked_targets(
    root: WindowKind,
    root_rect: &RECT,
) -> HashMap<WindowKind, RECT> {
    let mut targets = HashMap::new();
    targets.insert(root, *root_rect);

    with_dock_tree(|tree| {
        let mut queue = VecDeque::new();
        queue.push_back(root);

        while let Some(parent) = queue.pop_front() {
            let parent_rect = match targets.get(&parent).copied() {
                Some(r) => r,
                None => continue,
            };

            for (&child, bond) in tree.iter() {
                if bond.is_bonded && bond.parent == parent && !targets.contains_key(&child) {
                    let normal_r = unsafe { get_window_normal_rect(child) };
                    let raw_w = normal_r.right - normal_r.left;
                    let raw_h = normal_r.bottom - normal_r.top;
                    let (def_w, def_h) = {
                        let s = child.default_size();
                        (s[0] as i32, s[1] as i32)
                    };
                    let w = if raw_w > 50 { raw_w } else { def_w };
                    let h = if raw_h > 50 { raw_h } else { def_h };

                    let child_dummy_rect = RECT {
                        left: 0,
                        top: 0,
                        right: w,
                        bottom: h,
                    };

                    let (x, y) = calc_bonded_visualizer_pos(bond, &child_dummy_rect, &parent_rect);
                    let child_target_rect = RECT {
                        left: x,
                        top: y,
                        right: x + w,
                        bottom: y + h,
                    };

                    targets.insert(child, child_target_rect);
                    queue.push_back(child);
                }
            }
        }
    });

    targets
}

/// 以主控視窗為發射中心，執行順向階梯彈簧過衝外擴飛回動畫並還原群組視窗（與最小化吸攏形成 100% 鏡像對稱閉環）
pub unsafe fn expand_cluster_animated(root: WindowKind, root_hwnd: HWND) {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 {
            ShowWindow(root_hwnd, SW_RESTORE);
        }
        finish_cluster_restore(root);
        return;
    }

    set_cluster_state(root, ClusterState::Restoring(std::time::Instant::now()));

    let members = get_cluster_members(root);
    if members.len() <= 1 {
        finish_cluster_restore(root);
        if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 {
            ShowWindow(root_hwnd, SW_RESTORE);
        }
        return;
    }

    // 1. 還原根節點視窗（母體）
    if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 && IsIconic(root_hwnd) != 0 {
        ShowWindow(root_hwnd, SW_RESTORE);
    }

    let root_r = get_window_normal_rect(root);
    let targets = compute_cluster_docked_targets(root, &root_r);

    // 2. 收集子視窗動畫參數
    #[derive(Clone)]
    struct ChildExpandAnimData {
        hwnd_val: isize,
        child_kind: WindowKind,
        start_x: i32,
        start_y: i32,
        target_r: RECT,
        delay: f32,
    }

    let mut children: Vec<ChildExpandAnimData> = Vec::new();
    let root_hwnd_val = root_hwnd as isize;

    for &m in &members {
        if m == root {
            continue;
        }
        let m_hwnd = get_window_hwnd(m);
        if !m_hwnd.is_null() && IsWindow(m_hwnd) != 0 {
            let target_r = match targets.get(&m) {
                Some(r) => *r,
                None => continue,
            };

            // 順向階梯延遲（內層先彈、外層後彈，與收攏鏡像倒序）
            let delay = match m {
                WindowKind::MiniPlayer => 0.00,
                WindowKind::Equalizer => 0.02,
                WindowKind::Visualizer => 0.04,
                _ => 0.0,
            };

            // 起始位置：母體身後（鏡像收攏時的終點）
            let start_x = root_r.left;
            let start_y = root_r.top;

            // 預備動作：將子視窗初始放置於母體身後，解除最小化或隱藏狀態並顯示（隱藏於母體背後準備彈射）
            SetWindowPos(
                m_hwnd,
                root_hwnd,
                start_x,
                start_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            if IsIconic(m_hwnd) != 0 || IsWindowVisible(m_hwnd) == 0 {
                ShowWindow(m_hwnd, SW_SHOWNOACTIVATE);
            }
            if m == WindowKind::Visualizer {
                crate::visualizer::restore_visualizer_with_cluster();
            }

            children.push(ChildExpandAnimData {
                hwnd_val: m_hwnd as isize,
                child_kind: m,
                start_x,
                start_y,
                target_r,
                delay,
            });
        }
    }

    if children.is_empty() {
        finish_cluster_restore(root);
        return;
    }

    // 3. 在背景執行緒執行 8 步（約 88ms）極速順向階梯彈簧外擴彈射動畫
    std::thread::spawn(move || {
        let steps = 8;
        for step in 1..=steps {
            let t_global = step as f32 / steps as f32;

            for child in &children {
                let f = staggered_spring(t_global, child.delay);
                let cur_x = (child.start_x as f32 + (child.target_r.left - child.start_x) as f32 * f).round() as i32;
                let cur_y = (child.start_y as f32 + (child.target_r.top - child.start_y) as f32 * f).round() as i32;

                unsafe {
                    let h = child.hwnd_val as HWND;
                    if IsWindow(h) != 0 {
                        SetWindowPos(
                            h,
                            std::ptr::null_mut(),
                            cur_x,
                            cur_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(11));
        }

        // 動畫結束：精準校正最終坐標並恢復全套群組狀態
        unsafe {
            for child in &children {
                let h = child.hwnd_val as HWND;
                if IsWindow(h) != 0 {
                    SetWindowPos(
                        h,
                        std::ptr::null_mut(),
                        child.target_r.left,
                        child.target_r.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );

                    let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
                    wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
                    if GetWindowPlacement(h, &mut wp) != 0 {
                        wp.rcNormalPosition = child.target_r;
                        SetWindowPlacement(h, &wp);
                    }
                    update_known_rect(child.child_kind, child.target_r);
                }
            }

            let m_h = root_hwnd_val as HWND;
            if IsWindow(m_h) != 0 {
                bring_cluster_to_top(root, m_h);
            }

            // 同步工作列歸一屬性
            sync_cluster_taskbar_ownership();

            crate::gui::request_ui_repaint();

            finish_cluster_restore(root);
        }
    });
}

/// Restores and realigns all windows in the magnetic cluster when any member is restored.
/// 當群組處於最小化時，啟動順向階梯彈簧外擴飛回動畫；若已在螢幕上則執行即時精準校正。
pub unsafe fn restore_cluster(kind: WindowKind, active_hwnd: HWND) {
    let root = get_component_root(kind);
    let root_hwnd = get_window_hwnd(root);
    let effective_root_hwnd = if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 {
        root_hwnd
    } else {
        active_hwnd
    };

    let members = get_cluster_members(root);
    if members.len() <= 1 {
        if !effective_root_hwnd.is_null() && IsWindow(effective_root_hwnd) != 0 && IsIconic(effective_root_hwnd) != 0 {
            ShowWindow(effective_root_hwnd, SW_RESTORE);
        }
        return;
    }

    // 檢查群組成員是否有處於最小化（Iconic）或收攏隱藏（Hidden）狀態
    let is_any_collapsed = members.iter().any(|&m| {
        let h = get_window_hwnd(m);
        !h.is_null() && IsWindow(h) != 0 && (IsIconic(h) != 0 || IsWindowVisible(h) == 0)
    });

    if is_any_collapsed {
        expand_cluster_animated(root, effective_root_hwnd);
    } else {
        if !effective_root_hwnd.is_null() && IsWindow(effective_root_hwnd) != 0 {
            restore_and_realign_all_children(root, effective_root_hwnd);
        }
    }
}

// --- Backward Compatibility Shims ---

pub fn set_dock_bond(vis_rect: &RECT, main_rect: &RECT) {
    set_dock_relation(WindowKind::Visualizer, WindowKind::Main, vis_rect, main_rect);
}

pub fn break_dock_bond() {
    break_dock_relation(WindowKind::Visualizer);
}

pub fn get_dock_bond() -> DockBond {
    with_dock_tree(|tree| {
        tree.get(&WindowKind::Visualizer).copied().unwrap_or(DockBond {
            is_bonded: false,
            parent: WindowKind::Main,
            side: DockSide::None,
            offset_x: 0,
            offset_y: 0,
        })
    })
}

/// 將主視窗、迷你播放器、等化器與視覺效果視窗依據 3 欄模組化工作站幾何完美對齊並建立磁吸群組。
/// 注意：此排列僅移動各視窗至最佳相對位置（使用 SWP_NOSIZE），絕不強制修改各視窗的寬高，以防內部元件排版失常。
pub unsafe fn layout_modular_workstation(
    main_hwnd: HWND,
    mini_hwnd: HWND,
    eq_hwnd: HWND,
    vis_hwnd: HWND,
) {
    if main_hwnd.is_null() || windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(main_hwnd) == 0 {
        return;
    }

    let monitor = windows_sys::Win32::Graphics::Gdi::MonitorFromWindow(
        main_hwnd,
        windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
    );
    let mut mi: windows_sys::Win32::Graphics::Gdi::MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32;
    let (screen_left, screen_top, screen_right, screen_bottom) = if !monitor.is_null()
        && windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut mi) != 0
    {
        (mi.rcWork.left, mi.rcWork.top, mi.rcWork.right, mi.rcWork.bottom)
    } else {
        (
            0,
            0,
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(windows_sys::Win32::UI::WindowsAndMessaging::SM_CXSCREEN),
            windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(windows_sys::Win32::UI::WindowsAndMessaging::SM_CYSCREEN),
        )
    };

    let is_topmost = is_window_pinned(WindowKind::Main) || crate::visualizer::is_visualizer_fullscreen();
    let top_tier = if is_topmost { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };

    let swp_flags = windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE
        | windows_sys::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW
        | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;

    // 輔助函式：取得視窗真實常態尺寸與矩形；若視窗處於最小化（Iconic），安全解除最小化並讀取 WINDOWPLACEMENT
    let get_window_normal_rect = |hwnd: HWND, min_w: i32, min_h: i32| -> (RECT, i32, i32) {
        if hwnd.is_null() || windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) == 0 {
            return (unsafe { std::mem::zeroed() }, 0, 0);
        }
        unsafe {
            if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0 {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                    hwnd,
                    windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE,
                );
                let mut wp: windows_sys::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT = std::mem::zeroed();
                wp.length = std::mem::size_of::<windows_sys::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT>() as u32;
                if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowPlacement(hwnd, &mut wp) != 0 {
                    let mut r = wp.rcNormalPosition;
                    let w = (r.right - r.left).max(min_w);
                    let h = (r.bottom - r.top).max(min_h);
                    if r.left < -1000 || r.top < -1000 {
                        r.left = 0;
                        r.top = 0;
                        r.right = w;
                        r.bottom = h;
                    }
                    return (r, w, h);
                }
            }
            let mut r: RECT = std::mem::zeroed();
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut r);
            let w = (r.right - r.left).max(min_w);
            let h = (r.bottom - r.top).max(min_h);
            (r, w, h)
        }
    };

    // 1. 取得主視窗目前的實際寬高與矩形
    let (main_rect, main_w, main_h) = get_window_normal_rect(main_hwnd, 100, 100);

    // 2. 取得迷你播放器目前的實際寬高與矩形（若有開啟）
    let (has_mini, mini_rect, mini_w, mini_h) = if !mini_hwnd.is_null()
        && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(mini_hwnd) != 0
    {
        let (r, w, h) = get_window_normal_rect(mini_hwnd, 100, 50);
        (true, r, w, h)
    } else {
        (false, std::mem::zeroed(), 0, 0)
    };

    // 3. 取得等化器目前的實際寬高與矩形（若有開啟）
    let (has_eq, eq_rect, eq_w, eq_h) = if !eq_hwnd.is_null()
        && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(eq_hwnd) != 0
    {
        let (r, w, h) = get_window_normal_rect(eq_hwnd, 100, 50);
        (true, r, w, h)
    } else {
        (false, std::mem::zeroed(), 0, 0)
    };

    // 4. 取得視覺效果視窗目前的實際寬高與矩形（若有開啟且非全螢幕）
    let (has_vis, vis_rect, vis_w, vis_h) = if !vis_hwnd.is_null()
        && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(vis_hwnd) != 0
        && !crate::visualizer::is_visualizer_fullscreen()
    {
        let (r, w, h) = get_window_normal_rect(vis_hwnd, 100, 100);
        (true, r, w, h)
    } else {
        (false, std::mem::zeroed(), 0, 0)
    };

    // 計算各欄寬度與集群整體尺寸（迷你播放器與迷你等化器高度精準分配為主視窗高度的一半：main_h / 2，總和完美契合主視窗）
    let (mini_h, eq_h) = if has_mini && has_eq {
        let half = main_h / 2;
        let m_h = half;
        let e_h = main_h - m_h;
        (m_h, e_h)
    } else if has_mini {
        (main_h / 2, eq_h)
    } else if has_eq {
        (mini_h, main_h / 2)
    } else {
        (mini_h, eq_h)
    };

    let col2_w = if has_mini && has_eq {
        mini_w.max(eq_w)
    } else if has_mini {
        mini_w
    } else if has_eq {
        eq_w
    } else {
        0
    };
    let col2_h = if has_mini && has_eq {
        mini_h + eq_h
    } else if has_mini {
        mini_h
    } else if has_eq {
        eq_h
    } else {
        0
    };

    let total_w = main_w + col2_w + vis_w;
    let total_h = main_h.max(col2_h).max(vis_h);

    let start_x = if main_rect.left + total_w > screen_right {
        (screen_right - total_w).max(screen_left)
    } else {
        main_rect.left.max(screen_left)
    };

    let start_y = if main_rect.top + total_h > screen_bottom {
        (screen_bottom - total_h).max(screen_top)
    } else {
        main_rect.top.max(screen_top)
    };

    // 第一欄：主視窗目標坐標
    let main_r = RECT {
        left: start_x,
        top: start_y,
        right: start_x + main_w,
        bottom: start_y + main_h,
    };

    // 第二欄：迷你播放器（上方）與等化器（下方）目標坐標
    let col2_x = start_x + main_w;

    let mini_r = if has_mini {
        RECT {
            left: col2_x,
            top: start_y,
            right: col2_x + mini_w,
            bottom: start_y + mini_h,
        }
    } else {
        std::mem::zeroed()
    };
    let eq_y = if has_mini { start_y + mini_h } else { start_y };
    let eq_r = if has_eq {
        RECT {
            left: col2_x,
            top: eq_y,
            right: col2_x + eq_w,
            bottom: eq_y + eq_h,
        }
    } else {
        std::mem::zeroed()
    };

    // 第三欄：視覺效果視窗（右側完整一欄）目標坐標
    let vis_x = if col2_w > 0 { col2_x + col2_w } else { col2_x };
    let vis_h = main_h;
    let vis_r = if has_vis {
        RECT {
            left: vis_x,
            top: start_y,
            right: vis_x + vis_w,
            bottom: start_y + vis_h,
        }
    } else {
        std::mem::zeroed()
    };

    let main_start_x = main_rect.left;
    let main_start_y = main_rect.top;
    let mini_start_x = mini_rect.left;
    let mini_start_y = mini_rect.top;
    let eq_start_x = eq_rect.left;
    let eq_start_y = eq_rect.top;
    let vis_start_x = vis_rect.left;
    let vis_start_y = vis_rect.top;

    let already_aligned = (main_start_x == main_r.left && main_start_y == main_r.top)
        && (!has_mini || (mini_start_x == mini_r.left && mini_start_y == mini_r.top))
        && (!has_eq || (eq_start_x == eq_r.left && eq_start_y == eq_r.top))
        && (!has_vis || (vis_start_x == vis_r.left && vis_start_y == vis_r.top));

    // 同步完成尺寸調整與初次呈現（保持原坐標，避免位移突兀）
    windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
        main_hwnd,
        top_tier,
        main_start_x,
        main_start_y,
        main_w,
        main_h,
        swp_flags,
    );

    if has_mini {
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
            mini_hwnd,
            top_tier,
            mini_start_x,
            mini_start_y,
            mini_w,
            mini_h,
            windows_sys::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        );
    }

    if has_eq {
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
            eq_hwnd,
            top_tier,
            eq_start_x,
            eq_start_y,
            eq_w,
            eq_h,
            windows_sys::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        );
    }

    if has_vis {
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
            vis_hwnd,
            top_tier,
            vis_start_x,
            vis_start_y,
            vis_w,
            vis_h,
            windows_sys::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        );
    }

    if already_aligned {
        // 已對齊狀態直接落位與綁定拓撲關係
        update_known_rect(WindowKind::Main, main_r);
        if has_mini {
            update_known_rect(WindowKind::MiniPlayer, mini_r);
            set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        }
        if has_eq {
            update_known_rect(WindowKind::Equalizer, eq_r);
            if has_mini {
                set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
            } else {
                set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);
            }
        }
        if has_vis {
            update_known_rect(WindowKind::Visualizer, vis_r);
            if has_mini {
                set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);
            } else if has_eq {
                set_dock_relation(WindowKind::Visualizer, WindowKind::Equalizer, &vis_r, &eq_r);
            } else {
                set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
            }
        }
        bring_cluster_to_top(WindowKind::Main, main_hwnd);
        rebuild_dock_tree();
        return;
    }

    // 啟動階梯式彈簧阻尼磁吸聚合動畫 (Staggered Spring transit animation)
    if IS_WORKSTATION_ANIMATING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }

    let main_hwnd_val = main_hwnd as usize;
    let mini_hwnd_val = mini_hwnd as usize;
    let eq_hwnd_val = eq_hwnd as usize;
    let vis_hwnd_val = vis_hwnd as usize;

    std::thread::spawn(move || {
        struct WorkstationGuard;
        impl Drop for WorkstationGuard {
            fn drop(&mut self) {
                IS_WORKSTATION_ANIMATING.store(false, Ordering::SeqCst);
            }
        }
        let _guard = WorkstationGuard;
        let thread_top_tier = if is_topmost { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };

        let steps = 18;
        for step in 1..=steps {
            let t_global = step as f32 / steps as f32;

            unsafe {
                // 1. 主視窗（第 1 欄，delay = 0.0）
                let f_main = staggered_spring(t_global, 0.0);
                let cur_main_x = (main_start_x as f32 + (main_r.left - main_start_x) as f32 * f_main).round() as i32;
                let cur_main_y = (main_start_y as f32 + (main_r.top - main_start_y) as f32 * f_main).round() as i32;
                let m_h = main_hwnd_val as HWND;
                if IsWindow(m_h) != 0 {
                    SetWindowPos(
                        m_h,
                        std::ptr::null_mut(),
                        cur_main_x,
                        cur_main_y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }

                // 2. 迷你播放器（第 2 欄上方，delay = 0.05）
                if has_mini {
                    let f_mini = staggered_spring(t_global, 0.05);
                    let cur_mini_x = (mini_start_x as f32 + (mini_r.left - mini_start_x) as f32 * f_mini).round() as i32;
                    let cur_mini_y = (mini_start_y as f32 + (mini_r.top - mini_start_y) as f32 * f_mini).round() as i32;
                    let mini_h = mini_hwnd_val as HWND;
                    if IsWindow(mini_h) != 0 {
                        SetWindowPos(
                            mini_h,
                            std::ptr::null_mut(),
                            cur_mini_x,
                            cur_mini_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }

                // 3. 等化器（第 2 欄下方，delay = 0.10）
                if has_eq {
                    let f_eq = staggered_spring(t_global, 0.10);
                    let cur_eq_x = (eq_start_x as f32 + (eq_r.left - eq_start_x) as f32 * f_eq).round() as i32;
                    let cur_eq_y = (eq_start_y as f32 + (eq_r.top - eq_start_y) as f32 * f_eq).round() as i32;
                    let eq_h = eq_hwnd_val as HWND;
                    if IsWindow(eq_h) != 0 {
                        SetWindowPos(
                            eq_h,
                            std::ptr::null_mut(),
                            cur_eq_x,
                            cur_eq_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }

                // 4. 視覺效果視窗（第 3 欄右側，delay = 0.15）
                if has_vis {
                    let f_vis = staggered_spring(t_global, 0.15);
                    let cur_vis_x = (vis_start_x as f32 + (vis_r.left - vis_start_x) as f32 * f_vis).round() as i32;
                    let cur_vis_y = (vis_start_y as f32 + (vis_r.top - vis_start_y) as f32 * f_vis).round() as i32;
                    let vis_h = vis_hwnd_val as HWND;
                    if IsWindow(vis_h) != 0 {
                        SetWindowPos(
                            vis_h,
                            std::ptr::null_mut(),
                            cur_vis_x,
                            cur_vis_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        // 動畫最後一步：精確定位在目標 target_r，並鎖定磁吸關係拓撲樹
        unsafe {
            let m_h = main_hwnd_val as HWND;
            if IsWindow(m_h) != 0 {
                SetWindowPos(
                    m_h,
                    thread_top_tier,
                    main_r.left,
                    main_r.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
                update_known_rect(WindowKind::Main, main_r);
            }

            if has_mini {
                let mini_h = mini_hwnd_val as HWND;
                if IsWindow(mini_h) != 0 {
                    SetWindowPos(
                        mini_h,
                        thread_top_tier,
                        mini_r.left,
                        mini_r.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                    update_known_rect(WindowKind::MiniPlayer, mini_r);
                    set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
                }
            }

            if has_eq {
                let eq_h = eq_hwnd_val as HWND;
                if IsWindow(eq_h) != 0 {
                    SetWindowPos(
                        eq_h,
                        thread_top_tier,
                        eq_r.left,
                        eq_r.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                    update_known_rect(WindowKind::Equalizer, eq_r);
                    if has_mini {
                        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
                    } else {
                        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);
                    }
                }
            }

            if has_vis {
                let vis_h = vis_hwnd_val as HWND;
                if IsWindow(vis_h) != 0 {
                    SetWindowPos(
                        vis_h,
                        thread_top_tier,
                        vis_r.left,
                        vis_r.top,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                    update_known_rect(WindowKind::Visualizer, vis_r);
                    if has_mini {
                        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);
                    } else if has_eq {
                        set_dock_relation(WindowKind::Visualizer, WindowKind::Equalizer, &vis_r, &eq_r);
                    } else {
                        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
                    }
                }
            }

            bring_cluster_to_top(WindowKind::Main, m_h);
            rebuild_dock_tree();
        }
    });
}

/// 逆向磁吸塌縮動畫 (Clustered Implosion Animation)
/// 當使用者點擊 ⛝ 關閉視窗群組時，外圍附屬視窗原地迅速向主機核心視窗聚攏吸入背後，隨後整組乾脆俐落地原地隱沒。
pub fn animate_cluster_implosion<F>(origin: WindowKind, on_complete: F)
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(test)]
    {
        let _ = origin;
        on_complete();
        return;
    }

    #[cfg(not(test))]
    {
        if IS_CLUSTER_CLOSING_ANIMATING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }

        let root = get_component_root(origin);
        let members = get_cluster_members(root);

        let mut targets = Vec::new();
        for m in &members {
            let hwnd = get_window_hwnd(*m);
            if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
                let mut r: RECT = unsafe { std::mem::zeroed() };
                if unsafe { GetWindowRect(hwnd, &mut r) } != 0 {
                    targets.push((*m, hwnd as usize, r));
                }
            }
        }

        if targets.len() <= 1 {
            IS_CLUSTER_CLOSING_ANIMATING.store(false, Ordering::SeqCst);
            on_complete();
            return;
        }

        // 選擇核心主機視窗 (優先以主視窗為中心，若無則以第 1 個視窗為中心)
        let core_idx = targets
            .iter()
            .position(|(m, _, _)| *m == WindowKind::Main)
            .unwrap_or(0);
        let (_, core_hwnd_val, core_r) = targets[core_idx];

        // 附屬視窗列表 (Slaves)
        let mut slaves = Vec::new();
        for (i, t) in targets.iter().enumerate() {
            if i != core_idx {
                slaves.push(*t);
            }
        }

        if slaves.is_empty() {
            IS_CLUSTER_CLOSING_ANIMATING.store(false, Ordering::SeqCst);
            on_complete();
            return;
        }

        let core_hwnd = core_hwnd_val as HWND;
        unsafe {
            if IsWindow(core_hwnd) != 0 {
                BringWindowToTop(core_hwnd);
            }
        }

        std::thread::spawn(move || {
            struct AnimGuard;
            impl Drop for AnimGuard {
                fn drop(&mut self) {
                    IS_CLUSTER_CLOSING_ANIMATING.store(false, Ordering::SeqCst);
                }
            }
            let _guard = AnimGuard;

            // 8 幀 × 11ms ≈ 88ms 極速磁吸折疊回核心視窗背後
            let steps = 8;
            for step in 1..=steps {
                let t = step as f32 / steps as f32;
                // Cubic Ease-In 強力磁性收吸
                let ease = t * t * t;

                for (_, hwnd_val, r) in &slaves {
                    let hwnd = *hwnd_val as HWND;
                    let cur_x = (r.left as f32 + (core_r.left - r.left) as f32 * ease).round() as i32;
                    let cur_y = (r.top as f32 + (core_r.top - r.top) as f32 * ease).round() as i32;
                    unsafe {
                        if IsWindow(hwnd) != 0 {
                            SetWindowPos(
                                hwnd,
                                std::ptr::null_mut(),
                                cur_x,
                                cur_y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                            );
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(11));
            }

            // 塌縮吸入完成：子視窗已完全隱入核心主視窗背後，立即就地隱形，絕不彈回原位閃爍
            unsafe {
                for (_, hwnd_val, _) in &slaves {
                    let hwnd = *hwnd_val as HWND;
                    if IsWindow(hwnd) != 0 {
                        ShowWindow(hwnd, SW_HIDE);
                    }
                }
            }

            on_complete();
        });
    }
}

// --- Generalized Movement & Release Handlers ---

pub unsafe fn handle_window_moving(kind: WindowKind, hwnd: HWND, rect_ptr: *mut RECT) {
    if rect_ptr.is_null() || !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let dist = SNAP_DISTANCE.load(Ordering::Relaxed);
    let work = get_monitor_work_area(hwnd);
    snap_rect_to_work_area(&mut *rect_ptr, &work, dist);

    // 1. Slave Check: if this window is currently bonded to a parent
    let parent_bond = with_dock_tree(|tree| tree.get(&kind).copied());
    if let Some(bond) = parent_bond {
        if bond.is_bonded {
            let parent_hwnd = get_window_hwnd(bond.parent);
            if !parent_hwnd.is_null() && IsWindow(parent_hwnd) != 0 && IsWindowVisible(parent_hwnd) != 0 {
                let mut parent_rect: RECT = std::mem::zeroed();
                GetWindowRect(parent_hwnd, &mut parent_rect);
                let (expected_x, expected_y) = calc_bonded_visualizer_pos(&bond, &*rect_ptr, &parent_rect);
                if ((*rect_ptr).left - expected_x).abs() > dist
                    || ((*rect_ptr).top - expected_y).abs() > dist
                {
                    // User pulled slave away: sever edge in edge set so it unbonds smoothly.
                    // DO NOT call rebuild_dock_tree() during WM_MOVING to prevent titlebar frame flicker and dragging jitter!
                    remove_dock_edge(kind, bond.parent);
                }
            } else {
                remove_dock_edge(kind, bond.parent);
            }
        }
    }

    // 2. Snapping Check: try snapping to candidates NOT in our connected component
    let my_root = get_component_root(kind);

    for &cand in &ALL_WINDOW_KINDS {
        if cand == kind {
            continue;
        }
        if get_component_root(cand) == my_root {
            continue;
        }
        let cand_hwnd = get_window_hwnd(cand);
        if !cand_hwnd.is_null() && IsWindow(cand_hwnd) != 0 && IsWindowVisible(cand_hwnd) != 0 {
            if cand == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
                continue;
            }
            if cand == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                continue;
            }
            let mut cand_rect: RECT = std::mem::zeroed();
            GetWindowRect(cand_hwnd, &mut cand_rect);
            if snap_rect_to_rect(&mut *rect_ptr, &cand_rect, dist) {
                // Snap coordinate visually for user guidance during drag.
                // Do NOT mutate dock edges or call rebuild_dock_tree() here!
                // Dock edges will be committed cleanly on WM_EXITSIZEMOVE when user releases the mouse.
                break;
            }
        }
    }

    // 3. Master Check: propagate moves to any downstream children
    propagate_child_moves(kind, &*rect_ptr);
}

/// 以彈簧阻尼超調曲線對指定視窗進行磁吸貼齊滑入動畫
pub unsafe fn animate_window_spring_snap(
    hwnd: HWND,
    start_rect: RECT,
    target_rect: RECT,
    kind: WindowKind,
    target_anchor: WindowKind,
) {
    let anchor_pinned = is_window_pinned(target_anchor)
        || (target_anchor == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen())
        || (crate::visualizer::is_visualizer_fullscreen() && crate::visualizer::is_hud_shown());

    // 動畫開始前立即同化 moving 視窗層次與置頂狀態，防止滑入過程發生 Z-order 穿透與閃爍
    set_window_pinned(kind, anchor_pinned);
    crate::controller::save_window_pinned(kind, anchor_pinned);
    apply_window_topmost(kind, anchor_pinned);
    crate::gui::request_ui_repaint();

    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        add_dock_edge(kind, target_anchor);
        rebuild_dock_tree();
        assimilate_cluster_to_anchor(kind, target_anchor);
        return;
    }

    let top_tier = if anchor_pinned { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };

    let dx = target_rect.left - start_rect.left;
    let dy = target_rect.top - start_rect.top;
    if dx.abs() <= 1 && dy.abs() <= 1 {
        SetWindowPos(
            hwnd,
            top_tier,
            target_rect.left,
            target_rect.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
        add_dock_edge(kind, target_anchor);
        rebuild_dock_tree();
        assimilate_cluster_to_anchor(kind, target_anchor);
        return;
    }

    let hwnd_val = hwnd as usize;
    std::thread::spawn(move || {
        let h = hwnd_val as HWND;
        let thread_top_tier = if anchor_pinned { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };
        let steps = 10;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let factor = spring_overshoot_curve(t);
            let cur_x = (start_rect.left as f32 + dx as f32 * factor).round() as i32;
            let cur_y = (start_rect.top as f32 + dy as f32 * factor).round() as i32;
            unsafe {
                if IsWindow(h) == 0 {
                    return;
                }
                SetWindowPos(
                    h,
                    std::ptr::null_mut(),
                    cur_x,
                    cur_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(12));
        }

        unsafe {
            if IsWindow(h) != 0 {
                SetWindowPos(
                    h,
                    thread_top_tier,
                    target_rect.left,
                    target_rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            add_dock_edge(kind, target_anchor);
            rebuild_dock_tree();
            assimilate_cluster_to_anchor(kind, target_anchor);
        }
    });
}

pub unsafe fn handle_window_exit_size_move(kind: WindowKind, hwnd: HWND) {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let mut my_rect: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut my_rect);

    // Verify all existing edges touching `kind`:
    // If rects are not touching within tolerance (24px), sever edge
    let mut edges_to_remove = Vec::new();
    with_dock_edges(|edges| {
        for &(a, b) in edges.iter() {
            if a == kind || b == kind {
                let other = if a == kind { b } else { a };
                let other_hwnd = get_window_hwnd(other);
                if other_hwnd.is_null()
                    || IsWindow(other_hwnd) == 0
                    || IsWindowVisible(other_hwnd) == 0
                    || (other == WindowKind::Main && crate::tray::is_window_hidden_to_tray())
                {
                    edges_to_remove.push((a, b));
                } else {
                    let mut other_rect: RECT = std::mem::zeroed();
                    GetWindowRect(other_hwnd, &mut other_rect);
                    if !are_rects_docked(&my_rect, &other_rect, 16) {
                        edges_to_remove.push((a, b));
                    }
                }
            }
        }
    });

    for (a, b) in edges_to_remove {
        remove_dock_edge(a, b);
    }

    // Check if `kind` is touching or near any eligible candidate not yet connected
    let mut snap_target: Option<(RECT, WindowKind)> = None;
    let mut direct_dock_anchor: Option<WindowKind> = None;
    for &cand in &ALL_WINDOW_KINDS {
        if cand == kind {
            continue;
        }
        let cand_hwnd = get_window_hwnd(cand);
        if !cand_hwnd.is_null() && IsWindow(cand_hwnd) != 0 && IsWindowVisible(cand_hwnd) != 0 {
            if cand == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
                continue;
            }
            if cand == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                continue;
            }
            let mut cand_rect: RECT = std::mem::zeroed();
            GetWindowRect(cand_hwnd, &mut cand_rect);
            if are_rects_docked(&my_rect, &cand_rect, 8) {
                add_dock_edge(kind, cand);
                if direct_dock_anchor.is_none() {
                    direct_dock_anchor = Some(cand);
                }
            } else if snap_target.is_none() {
                let mut candidate_snap = my_rect;
                if snap_rect_to_rect(&mut candidate_snap, &cand_rect, 24) {
                    if (candidate_snap.left - my_rect.left).abs() > 0 || (candidate_snap.top - my_rect.top).abs() > 0 {
                        snap_target = Some((candidate_snap, cand));
                    }
                }
            }
        }
    }

    if let Some((target_r, cand)) = snap_target {
        animate_window_spring_snap(hwnd, my_rect, target_r, kind, cand);
    } else {
        rebuild_dock_tree();
        if let Some(anchor) = direct_dock_anchor {
            assimilate_cluster_to_anchor(kind, anchor);
        }
    }
}

// --- Unified Win32 Window Subclass Procedure ---

unsafe extern "system" fn unified_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uid_subclass: usize,
    ref_data: usize,
) -> LRESULT {
    let kind = match ref_data {
        1 => WindowKind::Main,
        2 => WindowKind::MiniPlayer,
        3 => WindowKind::Equalizer,
        4 => WindowKind::Visualizer,
        _ => return DefSubclassProc(hwnd, msg, wparam, lparam),
    };

    // When main window is hidden in tray (e.g. moved off-screen to -32000), ignore all snapping
    // and movement/resizing events for Main so we never accidentally pull or restore docked windows.
    if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
        if msg == WM_NCDESTROY {
            RemoveWindowSubclass(hwnd, Some(unified_window_subclass_proc), MAIN_WINDOW_SUBCLASS_ID);
        }
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_NCHITTEST => {
            if is_click_through_active(kind) {
                // 若選單開啟中，豁免穿透判定，允許使用者在選單內點擊
                if is_opacity_menu_open(kind) {
                    return DefSubclassProc(hwnd, msg, wparam, lparam);
                }
                let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
                // 標題列保護區：頂部 42px 內維持正常互動（拖曳、完整按鈕區域、選單、滾輪）
                // 標題列下方之內容區域回傳 HTTRANSPARENT (-1)，點擊直接穿透到底層視窗
                if y > rect.top + 42 {
                    return -1; // HTTRANSPARENT
                }
            }
        }
        WM_SETCURSOR => {
            if let Some(hcursor) = crate::custom_cursor::current_custom_cursor(kind) {
                SetCursor(hcursor);
                return 1;
            }
        }
        WM_MOUSELEAVE | WM_NCMOUSELEAVE => {
            crate::custom_cursor::set_header_cursor_state(kind, crate::custom_cursor::CustomCursorState::None);
        }
        WM_MOVING => {
            crate::custom_cursor::set_window_moving(kind, true);
            let rect_ptr = lparam as *mut RECT;
            handle_window_moving(kind, hwnd, rect_ptr);
            return 1;
        }
        WM_WINDOWPOSCHANGED => {
            if SNAPPING_ENABLED.load(Ordering::Relaxed) && !is_cluster_closing_animating() {
                let wp = lparam as *const WINDOWPOS;
                if !wp.is_null() && ((*wp).flags & SWP_NOSIZE == 0) {
                    let mut r: RECT = std::mem::zeroed();
                    GetWindowRect(hwnd, &mut r);
                    propagate_child_moves(kind, &r);
                }
            }
        }
        WM_EXITSIZEMOVE => {
            crate::custom_cursor::set_window_moving(kind, false);
            handle_window_exit_size_move(kind, hwnd);
        }
        WM_CAPTURECHANGED => {
            crate::custom_cursor::set_window_moving(kind, false);
        }
        WM_SIZE => {
            let resize_type = wparam as u32;
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                if resize_type == SIZE_MINIMIZED {
                    if !is_cluster_minimizing() {
                        let role = get_window_role(kind);
                        if role == WindowRole::ClusterMaster || role == WindowRole::ClusterSlave {
                            minimize_cluster(kind);
                        }
                    }
                } else if resize_type == SIZE_RESTORED {
                    if kind == WindowKind::Main {
                        crate::tray::ensure_window_on_screen(hwnd);
                    }
                    if !is_kind_cluster_minimizing(kind) && !is_kind_cluster_restoring(kind) {
                        let root = get_component_root(kind);
                        let members = get_cluster_members(root);
                        let has_hidden_or_iconic_children = members.iter().any(|&m| {
                            if m == root {
                                return false;
                            }
                            let m_h = get_window_hwnd(m);
                            !m_h.is_null() && IsWindow(m_h) != 0 && (IsIconic(m_h) != 0 || IsWindowVisible(m_h) == 0)
                        });
                        let was_collapsed = can_start_cluster_restore(root);
                        if was_collapsed || has_hidden_or_iconic_children {
                            expand_cluster_animated(root, hwnd);
                        } else {
                            restore_cluster(kind, hwnd);
                        }
                    }
                }
            }
        }
        WM_MOUSEACTIVATE => {
            // NOTE: 絕不在 WM_MOUSEACTIVATE 處理期間呼叫 SetWindowPos 重排 Z-order！
            // 系統正在判定滑鼠點擊焦點視窗，此時調用 SetWindowPos 會重入發送 WM_WINDOWPOSCHANGING，
            // 擾亂 winit 的事件派發並導致視窗失去回應。WM_ACTIVATE 會在焦點確認後安全地提升叢集至頂層。
        }
        WM_NCLBUTTONDOWN => {}
        WM_ACTIVATE => {
            let active_type = (wparam & 0xFFFF) as u32;
            let is_minimized = ((wparam >> 16) & 0xFFFF) != 0;
            if active_type != WA_INACTIVE as u32
                && !is_minimized
                && IsIconic(hwnd) == 0
                && !is_kind_cluster_minimizing(kind)
                && !is_kind_cluster_restoring(kind)
                && SNAPPING_ENABLED.load(Ordering::Relaxed)
            {
                bring_cluster_to_top(kind, hwnd);
            }
        }
        WM_SYSCOMMAND => {
            let sc = (wparam & 0xFFF0) as u32;
            let role = get_window_role(kind);
            if sc == SC_MINIMIZE {
                if role == WindowRole::ClusterMaster || role == WindowRole::ClusterSlave {
                    minimize_cluster(kind);
                    return 0;
                } else {
                    minimize_window_solo(kind);
                    return 0;
                }
            } else if sc == SC_RESTORE {
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                if pt.x > 0 && pt.y > 0 {
                    crate::genie_effect::record_taskbar_button_coord(pt.x, pt.y);
                }
                if crate::genie_effect::has_cached_snapshot_for_hwnd(hwnd as usize) {
                    restore_window_animated(kind, hwnd);
                    return 0;
                }
            }
            let is_slave = role == WindowRole::ClusterSlave;
            if is_slave && sc == SC_MAXIMIZE {
                return 0; // lock maximize on slave
            }
            if sc == SC_CLOSE {
                // 一律攔截 Win32 原生銷毀，交由 egui update 循環在下一幀統一關閉，防止 winit 銷毀期崩潰
                crate::gui::request_close_cluster(kind);
                return 0;
            }
        }
        WM_NCDESTROY => {
            let subclass_id = match kind {
                WindowKind::Main => MAIN_WINDOW_SUBCLASS_ID,
                WindowKind::MiniPlayer => MINI_PLAYER_WINDOW_SUBCLASS_ID,
                WindowKind::Equalizer => EQUALIZER_WINDOW_SUBCLASS_ID,
                WindowKind::Visualizer => 0,
            };
            if subclass_id != 0 {
                RemoveWindowSubclass(hwnd, Some(unified_window_subclass_proc), subclass_id);
            }
            match kind {
                WindowKind::Equalizer => set_equalizer_hwnd(std::ptr::null_mut()),
                WindowKind::MiniPlayer => set_mini_player_hwnd(std::ptr::null_mut()),
                _ => {}
            }
            break_dock_relation(kind);
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn install_window_subclass(kind: WindowKind, hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    crate::genie_effect::disable_window_dwm_transitions(hwnd);
    apply_dwm_acrylic(hwnd);
    match kind {
        WindowKind::Main => {
            crate::tray::set_main_hwnd(hwnd as isize);
            disable_dwm_iconic_preview(hwnd);
            SetWindowSubclass(
                hwnd,
                Some(unified_window_subclass_proc),
                MAIN_WINDOW_SUBCLASS_ID,
                WindowKind::Main as usize,
            );
        }
        WindowKind::MiniPlayer => {
            set_mini_player_hwnd(hwnd);
            SetWindowSubclass(
                hwnd,
                Some(unified_window_subclass_proc),
                MINI_PLAYER_WINDOW_SUBCLASS_ID,
                WindowKind::MiniPlayer as usize,
            );
        }
        WindowKind::Equalizer => {
            set_equalizer_hwnd(hwnd);
            SetWindowSubclass(
                hwnd,
                Some(unified_window_subclass_proc),
                EQUALIZER_WINDOW_SUBCLASS_ID,
                WindowKind::Equalizer as usize,
            );
        }
        WindowKind::Visualizer => {}
    }
    sync_cluster_taskbar_ownership();
}

pub unsafe fn install_main_window_subclass(hwnd: HWND) {
    install_window_subclass(WindowKind::Main, hwnd);
}

pub unsafe fn install_equalizer_window_subclass(hwnd: HWND) {
    install_window_subclass(WindowKind::Equalizer, hwnd);
}

pub unsafe fn install_mini_player_window_subclass(hwnd: HWND) {
    install_window_subclass(WindowKind::MiniPlayer, hwnd);
}

/// Universally applies Always-on-Top (pin) state to a window and its docked cluster members.
pub unsafe fn set_cluster_pinned(origin: WindowKind, pinned: bool) {
    let members = get_cluster_members(origin);
    for &kind in &members {
        set_window_pinned(kind, pinned);
        crate::controller::save_window_pinned(kind, pinned);
        apply_window_topmost(kind, pinned);
    }
}

// --- Standalone Window Initial Docking Helpers & Priority Cascading Allocator ---

pub fn get_active_window_rects(exclude: WindowKind) -> Vec<(WindowKind, RECT)> {
    let mut list = Vec::new();
    #[cfg(test)]
    {
        if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
            if let Some(ref map) = *guard {
                for (&k, &r) in map {
                    if k != exclude {
                        list.push((k, r));
                    }
                }
            }
        }
    }
    #[cfg(not(test))]
    {
        for &k in &ALL_WINDOW_KINDS {
            if k == exclude {
                continue;
            }
            if k == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
                continue;
            }
            let hwnd = get_window_hwnd(k);
            if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 } {
                if k == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                    continue;
                }
                let mut r: RECT = unsafe { std::mem::zeroed() };
                unsafe {
                    GetWindowRect(hwnd, &mut r);
                }
                list.push((k, r));
            }
        }
    }
    list
}

pub fn get_reference_work_area() -> RECT {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    if !main_hwnd.is_null() && unsafe { IsWindow(main_hwnd) != 0 } {
        unsafe { get_monitor_work_area(main_hwnd) }
    } else {
        #[cfg(not(test))]
        unsafe {
            let mut work: RECT = std::mem::zeroed();
            windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                windows_sys::Win32::UI::WindowsAndMessaging::SPI_GETWORKAREA,
                0,
                &mut work as *mut _ as *mut _,
                0,
            );
            if work.right > work.left && work.bottom > work.top {
                work
            } else {
                RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
            }
        }
        #[cfg(test)]
        {
            RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
        }
    }
}

pub fn rects_overlap(a: &RECT, b: &RECT, margin: i32) -> bool {
    a.left + margin < b.right
        && a.right - margin > b.left
        && a.top + margin < b.bottom
        && a.bottom - margin > b.top
}

pub fn is_rect_inside_work_area(r: &RECT, work: &RECT) -> bool {
    r.left >= work.left && r.right <= work.right && r.top >= work.top && r.bottom <= work.bottom
}

/// Finds the best candidate spawn rect according to the priority cascade:
/// 1. Main window Right-Top (右上)
/// 2. Main window Right-Bottom (右下)
/// 3. Right-docked window Right-Top (右側已吸附視窗的右上)
/// 4. Right-docked window Right-Bottom (右側已吸附視窗的右下)
/// ...
/// Fallback: Screen Center (若畫面擠不下，生成於螢幕正中央且不吸附)
static LAST_SPAWNER: Mutex<Option<HashMap<WindowKind, WindowKind>>> = Mutex::new(None);

pub fn set_window_spawner(target: WindowKind, spawner: WindowKind) {
    if let Ok(mut guard) = LAST_SPAWNER.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(target, spawner);
    }
}

pub fn get_window_spawner(target: WindowKind) -> Option<WindowKind> {
    if let Ok(guard) = LAST_SPAWNER.lock() {
        guard.as_ref().and_then(|m| m.get(&target).copied())
    } else {
        None
    }
}

/// 尋找新生成視窗的最佳磁吸與佈局位置。
///
/// 遵循使用者定義的母視窗（被點擊按鈕所在的發射視窗）優先策略：
/// 1. 第一優先：母視窗右側（右上、右下接續、右底對齊）
/// 2. 第二優先：母視窗左側（左上、左下接續、左底對齊）
/// 3. 第三優先：母視窗下緣（下方左對齊、下方右對齊）
/// 4. 第四優先：其他次級視窗延伸（次級右側）
/// 5. Fallback：螢幕正中央
pub fn find_best_spawn_rect(
    target_kind: WindowKind,
    width: i32,
    height: i32,
    spawner_kind: Option<WindowKind>,
) -> (RECT, Option<WindowKind>) {
    let work = get_reference_work_area();
    let active_rects = get_active_window_rects(target_kind);

    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        let cx = work.left + (work.right - work.left - width).max(0) / 2;
        let cy = work.top + (work.bottom - work.top - height).max(0) / 2;
        return (
            RECT {
                left: cx,
                top: cy,
                right: cx + width,
                bottom: cy + height,
            },
            None,
        );
    }

    let is_valid_candidate = |cand: &RECT| -> bool {
        if !is_rect_inside_work_area(cand, &work) {
            return false;
        }
        for (_, existing_r) in &active_rects {
            if rects_overlap(cand, existing_r, 4) {
                return false;
            }
        }
        true
    };

    // 確定母視窗（發起視窗）。
    // 使用者規格：「母視窗不是指主視窗，是指被按下產生子視窗按鈕的那個視窗」。
    // 若明確傳入 spawner_kind 且該視窗可見，以其為第一母視窗；否則依序 fallback 到主視窗、或其他作用中視窗。
    let primary_spawner = spawner_kind
        .and_then(|k| active_rects.iter().find(|(ak, _)| *ak == k).copied())
        .or_else(|| active_rects.iter().find(|(k, _)| *k == WindowKind::Main).copied())
        .or_else(|| active_rects.first().copied());

    if let Some((spawner_k, spawner_r)) = primary_spawner {
        // --- 1. 第一優先：母視窗右緣（右上、右下接續、右底對齊） ---
        let right_x = spawner_r.right;

        // Slot 1: 母視窗右上 (Right-Top)
        let rt_cand = RECT {
            left: right_x,
            top: spawner_r.top,
            right: right_x + width,
            bottom: spawner_r.top + height,
        };
        if is_valid_candidate(&rt_cand) {
            return (rt_cand, Some(spawner_k));
        }

        // Slot 2: 母視窗右下（接續在右上已有視窗的下方）
        let touching_right: Vec<&RECT> = active_rects
            .iter()
            .filter(|(_, r)| (r.left - right_x).abs() <= 6)
            .map(|(_, r)| r)
            .collect();

        if !touching_right.is_empty() {
            let max_bottom = touching_right.iter().map(|r| r.bottom).max().unwrap();
            let rb_cand = RECT {
                left: right_x,
                top: max_bottom,
                right: right_x + width,
                bottom: max_bottom + height,
            };
            if is_valid_candidate(&rb_cand) {
                return (rb_cand, Some(spawner_k));
            }
        }

        // Slot 3: 母視窗右底對齊 (Right-Bottom-Aligned)
        let rb_align_top = spawner_r.bottom - height;
        if rb_align_top >= spawner_r.top {
            let r_align_cand = RECT {
                left: right_x,
                top: rb_align_top,
                right: right_x + width,
                bottom: spawner_r.bottom,
            };
            if is_valid_candidate(&r_align_cand) {
                return (r_align_cand, Some(spawner_k));
            }
        }

        // --- 2. 第二優先：母視窗左緣（左上、左下接續、左底對齊） ---
        let left_x = spawner_r.left - width;

        // Slot 4: 母視窗左上 (Left-Top)
        let lt_cand = RECT {
            left: left_x,
            top: spawner_r.top,
            right: spawner_r.left,
            bottom: spawner_r.top + height,
        };
        if is_valid_candidate(&lt_cand) {
            return (lt_cand, Some(spawner_k));
        }

        // Slot 5: 母視窗左下（接續在左上已有視窗的下方）
        let touching_left: Vec<&RECT> = active_rects
            .iter()
            .filter(|(_, r)| (r.right - spawner_r.left).abs() <= 6)
            .map(|(_, r)| r)
            .collect();

        if !touching_left.is_empty() {
            let max_bottom = touching_left.iter().map(|r| r.bottom).max().unwrap();
            let lb_cand = RECT {
                left: left_x,
                top: max_bottom,
                right: spawner_r.left,
                bottom: max_bottom + height,
            };
            if is_valid_candidate(&lb_cand) {
                return (lb_cand, Some(spawner_k));
            }
        }

        // Slot 6: 母視窗左底對齊 (Left-Bottom-Aligned)
        if rb_align_top >= spawner_r.top {
            let l_align_cand = RECT {
                left: left_x,
                top: rb_align_top,
                right: spawner_r.left,
                bottom: spawner_r.bottom,
            };
            if is_valid_candidate(&l_align_cand) {
                return (l_align_cand, Some(spawner_k));
            }
        }

        // --- 3. 第三優先：母視窗下緣（Bottom Edge） ---
        // Slot 7: 母視窗下方左對齊 (Bottom Left-Aligned)
        let b_left_cand = RECT {
            left: spawner_r.left,
            top: spawner_r.bottom,
            right: spawner_r.left + width,
            bottom: spawner_r.bottom + height,
        };
        if is_valid_candidate(&b_left_cand) {
            return (b_left_cand, Some(spawner_k));
        }

        // Slot 8: 母視窗下方右對齊 (Bottom Right-Aligned)
        let b_right_cand = RECT {
            left: spawner_r.right - width,
            top: spawner_r.bottom,
            right: spawner_r.right,
            bottom: spawner_r.bottom + height,
        };
        if is_valid_candidate(&b_right_cand) {
            return (b_right_cand, Some(spawner_k));
        }
    }

    // --- 4. 第四優先：其他次級視窗延伸（次級右側） ---
    let mut other_anchors: Vec<(WindowKind, RECT)> = active_rects
        .iter()
        .filter(|(k, _)| primary_spawner.map(|(pk, _)| pk != *k).unwrap_or(true))
        .copied()
        .collect();
    other_anchors.sort_by_key(|(_, r)| r.right);

    for (anchor_kind, anchor_r) in other_anchors {
        let x = anchor_r.right;
        let top_cand = RECT {
            left: x,
            top: anchor_r.top,
            right: x + width,
            bottom: anchor_r.top + height,
        };
        if is_valid_candidate(&top_cand) {
            return (top_cand, Some(anchor_kind));
        }
    }

    // --- 5. Fallback: 螢幕中央 (Screen Center) ---
    let cx = work.left + (work.right - work.left - width).max(0) / 2;
    let cy = work.top + (work.bottom - work.top - height).max(0) / 2;
    (
        RECT {
            left: cx,
            top: cy,
            right: cx + width,
            bottom: cy + height,
        },
        None,
    )
}

pub unsafe fn position_and_dock_spawned_window(kind: WindowKind, hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let mut rect: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut rect);
    let mut w = rect.right - rect.left;
    let mut h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        let def = kind.default_size();
        w = def[0] as i32;
        h = def[1] as i32;
    }

    // 迷你播放器與迷你等化器高度精準貼合主視窗的一半 (main_h / 2)
    if kind == WindowKind::MiniPlayer || kind == WindowKind::Equalizer {
        let main_hwnd = get_window_hwnd(WindowKind::Main);
        if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 {
            let mut main_r: RECT = std::mem::zeroed();
            GetWindowRect(main_hwnd, &mut main_r);
            let mh = main_r.bottom - main_r.top;
            if mh > 100 {
                h = mh / 2;
            }
        }
    }

    let spawner = get_window_spawner(kind);
    let (best_rect, anchor) = find_best_spawn_rect(kind, w, h, spawner);
    let target_w = best_rect.right - best_rect.left;
    let target_h = best_rect.bottom - best_rect.top;

    let is_topmost = is_window_pinned(WindowKind::Main) || crate::visualizer::is_visualizer_fullscreen();
    let top_tier = if is_topmost { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };

    // 直接在目標吸附坐標就位並顯示
    SetWindowPos(
        hwnd,
        top_tier,
        best_rect.left,
        best_rect.top,
        target_w,
        target_h,
        SWP_SHOWWINDOW | SWP_NOACTIVATE,
    );
    ShowWindow(hwnd, SW_SHOW);

    if let Some(anchor_kind) = anchor {
        let anchor_hwnd = get_window_hwnd(anchor_kind);
        if !anchor_hwnd.is_null() && IsWindow(anchor_hwnd) != 0 {
            let mut anchor_rect: RECT = std::mem::zeroed();
            GetWindowRect(anchor_hwnd, &mut anchor_rect);
            set_dock_relation(kind, anchor_kind, &best_rect, &anchor_rect);
            rebuild_dock_tree();
        }
    }
}

/// Universally positions and magnetically docks a newly spawned standalone window
/// using the generalized priority cascade (Equalizer -> MiniPlayer -> Visualizer -> Main).
pub unsafe fn dock_spawned_window(kind: WindowKind, hwnd: HWND) {
    if with_dock_tree(|t| t.get(&kind).map(|b| b.is_bonded).unwrap_or(false)) {
        return;
    }
    position_and_dock_spawned_window(kind, hwnd);
}

pub unsafe fn dock_equalizer_to_chain_rightmost(eq_hwnd: HWND) {
    dock_spawned_window(WindowKind::Equalizer, eq_hwnd);
}

pub unsafe fn dock_mini_player_to_equalizer_top(mini_hwnd: HWND) {
    dock_spawned_window(WindowKind::MiniPlayer, mini_hwnd);
}

pub unsafe fn dock_equalizer_to_mini_player_bottom(eq_hwnd: HWND) {
    dock_spawned_window(WindowKind::Equalizer, eq_hwnd);
}

/// 檢查主視窗是否已包含在指定視窗所屬的群組中
pub fn is_main_in_cluster(kind: WindowKind) -> bool {
    let members = get_cluster_members(kind);
    members.contains(&WindowKind::Main)
}

/// Magnetically summon and dock the Main Window to the specified anchor cluster with a smooth slide animation.
/// 群組感知磁吸召喚：計算整組視窗的聯合包圍盒 (Union Bounding Box)，將主視窗完美吸附至整組外緣，防止穿模。
pub unsafe fn summon_and_dock_main_window_animated(anchor_kind: WindowKind) {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    let anchor_hwnd = get_window_hwnd(anchor_kind);
    if main_hwnd.is_null() || anchor_hwnd.is_null() || IsWindow(main_hwnd) == 0 || IsWindow(anchor_hwnd) == 0 {
        return;
    }

    // 1. 取得當前群組全體成員並收集常態 Rect
    let members = get_cluster_members(anchor_kind);
    let mut member_rects: Vec<(WindowKind, RECT)> = Vec::new();

    for &m in &members {
        if m == WindowKind::Main {
            continue;
        }
        let h = get_window_hwnd(m);
        if !h.is_null() && IsWindow(h) != 0 && IsWindowVisible(h) != 0 && IsIconic(h) == 0 {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(h, &mut r);
            member_rects.push((m, r));
        }
    }

    #[cfg(test)]
    {
        if member_rects.is_empty() {
            if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
                if let Some(ref map) = *guard {
                    for &m in &members {
                        if m != WindowKind::Main {
                            if let Some(&r) = map.get(&m) {
                                member_rects.push((m, r));
                            }
                        }
                    }
                }
            }
        }
    }

    if member_rects.is_empty() {
        return;
    }

    // 2. 計算整組視窗的聯合包圍盒 (Union Bounding Box)
    let cluster_left = member_rects.iter().map(|(_, r)| r.left).min().unwrap();
    let cluster_top = member_rects.iter().map(|(_, r)| r.top).min().unwrap();
    let cluster_right = member_rects.iter().map(|(_, r)| r.right).max().unwrap();
    let _cluster_bottom = member_rects.iter().map(|(_, r)| r.bottom).max().unwrap();

    let work = get_monitor_work_area(anchor_hwnd);
    let mut main_rect: RECT = std::mem::zeroed();
    GetWindowRect(main_hwnd, &mut main_rect);
    let main_w = (main_rect.right - main_rect.left).max(460);
    let main_h = (main_rect.bottom - main_rect.top).max(680);

    // 3. 尋找群組外圍最佳接合坐標（優先停靠群組左側，若放不下則停靠群組右側）
    let (target_left, target_top, dock_side) = {
        let mut left = cluster_left - main_w;
        let mut side = DockSide::Right;
        if left < work.left {
            left = cluster_right;
            side = DockSide::Left;
            if left + main_w > work.right {
                left = (work.right - main_w).max(work.left);
            }
        }
        let top = cluster_top.clamp(work.top, (work.bottom - main_h).max(work.top));
        (left, top, side)
    };

    let target_rect = RECT {
        left: target_left,
        top: target_top,
        right: target_left + main_w,
        bottom: target_top + main_h,
    };

    // 4. 尋找群組中在物理上與主視窗相鄰接觸的成員視窗（Touch Window）
    let (touch_member, touch_rect) = member_rects
        .iter()
        .min_by_key(|(_, r)| {
            let h_dist = match dock_side {
                DockSide::Right => (r.left - target_rect.right).abs(),
                _ => (r.right - target_rect.left).abs(),
            };
            let v_overlap = (r.bottom.min(target_rect.bottom) - r.top.max(target_rect.top)).max(0);
            h_dist * 1000 - v_overlap
        })
        .map(|&(m, r)| (m, r))
        .unwrap_or((anchor_kind, target_rect));

    let is_top = members.iter().any(|&m| is_window_pinned(m)) || crate::visualizer::is_visualizer_fullscreen();
    let anim_tier = if is_top { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };

    let start_x = main_rect.left;
    let start_y = main_rect.top;

    let eff_start_x = if start_x < work.left - 500 || start_x > work.right + 500 {
        if target_left > work.left + 200 { target_left - 180 } else { target_left + 180 }
    } else {
        start_x
    };
    let eff_start_y = if start_y < work.top - 500 || start_y > work.bottom + 500 {
        target_top
    } else {
        start_y
    };

    let dx = target_left - eff_start_x;
    let dy = target_top - eff_start_y;

    if dx.abs() <= 1 && dy.abs() <= 1 {
        SetWindowPos(
            main_hwnd,
            anim_tier,
            target_left,
            target_top,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
        set_dock_relation(touch_member, WindowKind::Main, &touch_rect, &target_rect);
        assimilate_cluster_to_anchor(WindowKind::Main, touch_member);
        bring_cluster_to_top(WindowKind::Main, main_hwnd);
        sync_cluster_taskbar_ownership();
        crate::tray::force_foreground(main_hwnd);
        return;
    }

    let main_hwnd_val = main_hwnd as usize;
    std::thread::spawn(move || {
        let h = main_hwnd_val as HWND;
        let thread_anim_tier = if is_top { HWND_TOPMOST } else { windows_sys::Win32::UI::WindowsAndMessaging::HWND_NOTOPMOST };
        let steps = 14;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let factor = spring_overshoot_curve(t);
            let cur_x = (eff_start_x as f32 + dx as f32 * factor).round() as i32;
            let cur_y = (eff_start_y as f32 + dy as f32 * factor).round() as i32;
            unsafe {
                if IsWindow(h) == 0 {
                    return;
                }
                SetWindowPos(
                    h,
                    std::ptr::null_mut(),
                    cur_x,
                    cur_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(14));
        }

        unsafe {
            if IsWindow(h) != 0 {
                SetWindowPos(
                    h,
                    thread_anim_tier,
                    target_left,
                    target_top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            set_dock_relation(touch_member, WindowKind::Main, &touch_rect, &target_rect);
            assimilate_cluster_to_anchor(WindowKind::Main, touch_member);
            bring_cluster_to_top(WindowKind::Main, h);
            sync_cluster_taskbar_ownership();
            crate::tray::force_foreground(h);
        }
    });
}

/// Checks if the Main Window is currently in a state ready to be magnetically docked.
/// Pure state evaluation:
/// 1. Main window HWND is valid and window exists.
/// 2. Not hidden to tray.
/// 3. Visualizer is not in fullscreen mode.
/// 4. Not minimized (iconic).
/// 5. Window is visible.
/// 6. Coordinates are within the monitor work area (not off-screen).
pub fn is_main_window_dock_ready() -> bool {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    if main_hwnd.is_null() || unsafe { IsWindow(main_hwnd) } == 0 {
        return false;
    }
    if crate::tray::is_window_hidden_to_tray() {
        return false;
    }
    if crate::visualizer::is_visualizer_fullscreen() {
        return false;
    }
    unsafe {
        if IsIconic(main_hwnd) != 0 {
            return false;
        }
        if IsWindowVisible(main_hwnd) == 0 {
            return false;
        }
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(main_hwnd, &mut r);
        let work = get_monitor_work_area(main_hwnd);
        let is_off = r.right < work.left + 60
            || r.left > work.right - 60
            || r.bottom < work.top + 60
            || r.top > work.bottom - 60;
        if is_off {
            return false;
        }
    }
    true
}

/// Brings the entire application fleet (Main, MiniPlayer, Equalizer, Visualizer)
/// to the foreground top tier, ensuring that even windows separated from clusters
/// are elevated together above third-party applications.
pub unsafe fn bring_fleet_to_top() {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    let mini_hwnd = get_window_hwnd(WindowKind::MiniPlayer);
    let eq_hwnd = get_window_hwnd(WindowKind::Equalizer);
    let vis_hwnd = get_window_hwnd(WindowKind::Visualizer);

    // 1. Un-minimize main window if iconic and not in a minimized cluster state
    let main_root = get_component_root(WindowKind::Main);
    let is_main_minimized = is_cluster_minimizing() || matches!(get_cluster_state(main_root), ClusterState::Minimized);
    if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && IsIconic(main_hwnd) != 0 && !is_main_minimized {
        ShowWindow(main_hwnd, SW_RESTORE);
    }

    // 保底校正：若主視窗當前處於還原現身狀態，確保其寬高絕不低於基本底限 (380x500)
    if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && IsWindowVisible(main_hwnd) != 0 && IsIconic(main_hwnd) == 0 {
        let mut mr: RECT = std::mem::zeroed();
        GetWindowRect(main_hwnd, &mut mr);
        let mw = mr.right - mr.left;
        let mh = mr.bottom - mr.top;
        if mw < 380 || mh < 500 {
            SetWindowPos(
                main_hwnd,
                std::ptr::null_mut(),
                mr.left,
                mr.top,
                mw.max(460),
                mh.max(680),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    // 2. Elevate all active visible fleet windows in bottom-to-top order atomically
    let fleet = [
        (WindowKind::Visualizer, vis_hwnd),
        (WindowKind::Equalizer, eq_hwnd),
        (WindowKind::MiniPlayer, mini_hwnd),
        (WindowKind::Main, main_hwnd),
    ];

    let mut valid_windows = Vec::with_capacity(4);
    for &(kind, hwnd) in &fleet {
        if hwnd.is_null() || IsWindow(hwnd) == 0 {
            continue;
        }
        if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
            continue;
        }
        if IsWindowVisible(hwnd) == 0 {
            continue;
        }
        valid_windows.push((kind, hwnd));
    }

    if !valid_windows.is_empty() {
        let mut hdwp = BeginDeferWindowPos(valid_windows.len() as i32);
        for &(kind, hwnd) in &valid_windows {
            let is_top = is_window_pinned(kind)
                || (kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen());
            let insert_after = if is_top { HWND_TOPMOST } else { HWND_NOTOPMOST };
            if !hdwp.is_null() {
                hdwp = DeferWindowPos(
                    hdwp,
                    hwnd,
                    insert_after,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        if !hdwp.is_null() {
            EndDeferWindowPos(hdwp);
        }
        for &(_, hwnd) in &valid_windows {
            crate::tray::ensure_window_on_screen(hwnd);
        }
    }

    // 3. For clustered windows, restore and preserve cluster relative Z-order & bonding
    for &(kind, hwnd) in &fleet {
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            restore_and_realign_all_children(kind, hwnd);
            bring_cluster_to_top(kind, hwnd);
        }
    }

    // 4. Force foreground on Main window (if not hidden)
    if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && !crate::tray::is_window_hidden_to_tray() {
        crate::tray::force_foreground(main_hwnd);
    }
}

/// Universal handler for 🏠 (Home) summon requests across all viewports/windows.
/// 一步到位將主視窗磁吸至指定群組：
/// 1. 若主視窗已吸附於該群組中，將焦點帶給主視窗並置頂（不重複觸發移動）。
/// 2. 若主視窗未在群組中（無論在托盤、最小化或遠處），立即還原現身（保留原有頁面，絕不強制切頁），
///    並依據當前群組聯合包圍盒 (Union Bounding Box) 平滑飛至群組外緣磁吸接合！
pub unsafe fn handle_home_summon_request(source_kind: WindowKind) {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    if main_hwnd.is_null() || IsWindow(main_hwnd) == 0 {
        return;
    }

    if is_main_in_cluster(source_kind) {
        crate::tray::force_foreground(main_hwnd);
        bring_cluster_to_top(WindowKind::Main, main_hwnd);
        return;
    }

    // 一步到位：若在托盤或最小化，先確保其顯示並還原（保留現有頁面，絕不切頁！）
    if crate::tray::is_window_hidden_to_tray() {
        crate::tray::bring_main_to_front();
    }
    if IsIconic(main_hwnd) != 0 {
        ShowWindow(main_hwnd, SW_RESTORE);
    } else if IsWindowVisible(main_hwnd) == 0 {
        ShowWindow(main_hwnd, SW_SHOWNOACTIVATE);
    }

    summon_and_dock_main_window_animated(source_kind);
}

#[cfg(test)]
pub use tests::{TEST_LOCK, reset_all};

#[cfg(test)]
mod tests {
    use super::*;

    pub static TEST_LOCK: Mutex<()> = Mutex::new(());

    pub fn reset_all() {
        with_dock_edges(|e| e.clear());
        with_dock_tree(|t| t.clear());
        if let Ok(mut guard) = LAST_KNOWN_RECTS.lock() {
            if let Some(ref mut m) = *guard {
                m.clear();
            }
        }
        clear_all_cluster_fsm();
        clear_all_opacity_fsm();
        set_window_opacity(WindowKind::Main, 1.0);
        set_window_opacity(WindowKind::MiniPlayer, 1.0);
        set_window_opacity(WindowKind::Equalizer, 1.0);
        set_window_opacity(WindowKind::Visualizer, 1.0);
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);
        set_window_pinned(WindowKind::Visualizer, false);
        set_last_custom_opacity(WindowKind::Main, 30);
        set_last_custom_opacity(WindowKind::MiniPlayer, 30);
        set_last_custom_opacity(WindowKind::Equalizer, 30);
        set_last_custom_opacity(WindowKind::Visualizer, 30);
        clear_ghost_exit_record();
    }

    #[test]
    fn test_dock_side_and_bonded_pos() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();
        // Main window at (100, 100), size 400x500 -> right=500, bottom=600
        let main_rect = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        // Mini player docked to Top of Main at (100, -100), size 400x200 -> right=500, bottom=100
        let mini_rect = RECT { left: 100, top: -100, right: 500, bottom: 100 };

        assert!(are_rects_docked(&mini_rect, &main_rect, 4));
        let side = determine_dock_side(&mini_rect, &main_rect, 10);
        assert_eq!(side, DockSide::Top);

        let bond = DockBond {
            is_bonded: true,
            parent: WindowKind::Main,
            side: DockSide::Top,
            offset_x: 0,
            offset_y: -200,
        };

        // When main moves to (200, 300)
        let new_main_rect = RECT { left: 200, top: 300, right: 600, bottom: 800 };
        let (slave_x, slave_y) = calc_bonded_visualizer_pos(&bond, &mini_rect, &new_main_rect);
        assert_eq!(slave_x, 200);
        assert_eq!(slave_y, 100); // 300 - mini_h (200) = 100
    }

    #[test]
    fn test_dock_tree_hierarchy_and_cycle_prevention() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 100, top: -100, right: 500, bottom: 100 };
        let eq_r = RECT { left: 100, top: -600, right: 500, bottom: -100 };

        // MiniPlayer docks to Main
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        // Equalizer docks to MiniPlayer
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // Check descendant relationships
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::MiniPlayer));
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::Main));
        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::Main, WindowKind::MiniPlayer));
        assert!(!is_descendant(WindowKind::Main, WindowKind::Equalizer));

        // Breaking MiniPlayer detaches it from Main
        break_dock_relation(WindowKind::MiniPlayer);
        assert!(!is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        // Equalizer is still bonded to MiniPlayer
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::MiniPlayer));
        assert!(!is_descendant(WindowKind::Equalizer, WindowKind::Main));

        break_dock_relation(WindowKind::Equalizer);
    }

    #[test]
    fn test_symmetric_snapping_assignment() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 }; // directly docked on Right

        // When Main approaches MiniPlayer (Main is dragged, MiniPlayer is stationary):
        // Main (Lv 1) has higher priority than MiniPlayer (Lv 2).
        // Therefore, MiniPlayer becomes Slave, Main becomes Master!
        assert!(WindowKind::Main.priority() < WindowKind::MiniPlayer.priority());
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::Main, WindowKind::MiniPlayer));

        let bond = with_dock_tree(|tree| tree.get(&WindowKind::MiniPlayer).copied()).unwrap();
        assert_eq!(bond.parent, WindowKind::Main);
        assert_eq!(bond.side, DockSide::Right);

        // When MiniPlayer approaches Equalizer (MiniPlayer is dragged, Equalizer is stationary):
        // MiniPlayer (Lv 2) has higher priority than Equalizer (Lv 3).
        // Therefore, Equalizer becomes Slave, MiniPlayer becomes Master!
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 850 }; // docked to Bottom of MiniPlayer
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        assert!(is_descendant(WindowKind::Equalizer, WindowKind::MiniPlayer));
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::Main));

        let eq_bond = with_dock_tree(|tree| tree.get(&WindowKind::Equalizer).copied()).unwrap();
        assert_eq!(eq_bond.parent, WindowKind::MiniPlayer);
        assert_eq!(eq_bond.side, DockSide::Bottom);

        break_dock_relation(WindowKind::Equalizer);
        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_dock_tree_graph_bfs_case2_main_vis_mini() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 0, top: 0, right: 400, bottom: 600 };
        let vis_r = RECT { left: 400, top: 0, right: 800, bottom: 600 };
        let mini_r = RECT { left: 800, top: 0, right: 1200, bottom: 350 };

        // Main docks to Visualizer, and Visualizer docks to MiniPlayer
        // Physical chain: Main (Lv 1) <-> Visualizer (Lv 4) <-> MiniPlayer (Lv 2)
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Visualizer, &mini_r, &vis_r);

        // All 3 should be in the same component rooted at Main!
        assert_eq!(get_component_root(WindowKind::Main), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        // Check BFS tree directed edges
        let tree = with_dock_tree(|t| t.clone());
        let vis_bond = tree.get(&WindowKind::Visualizer).expect("Visualizer should be bonded");
        assert_eq!(vis_bond.parent, WindowKind::Main);
        assert_eq!(vis_bond.side, DockSide::Right);

        let mini_bond = tree.get(&WindowKind::MiniPlayer).expect("MiniPlayer should be bonded");
        assert_eq!(mini_bond.parent, WindowKind::Visualizer);
        assert_eq!(mini_bond.side, DockSide::Right);

        // Check descendant relationships
        assert!(is_descendant(WindowKind::Visualizer, WindowKind::Main));
        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Visualizer));
        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::Main, WindowKind::MiniPlayer));
        assert!(!is_descendant(WindowKind::Main, WindowKind::Visualizer));

        // When MiniPlayer is pulled away / detaches:
        break_dock_relation(WindowKind::MiniPlayer);

        // Visualizer should STILL be bonded to Main! (They do not separate!)
        assert!(is_descendant(WindowKind::Visualizer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);

        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_subcluster_root_selection_and_button_roles() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // Subcluster without Main: MiniPlayer (Lv 2) <-> Equalizer (Lv 3)
        let mini_r = RECT { left: 100, top: 100, right: 500, bottom: 350 };
        let eq_r = RECT { left: 100, top: 350, right: 500, bottom: 850 };

        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // Root of component is MiniPlayer
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::MiniPlayer);

        let tree = with_dock_tree(|t| t.clone());
        // MiniPlayer has no parent -> is Root -> keeps caption buttons
        let mini_is_slave = tree.get(&WindowKind::MiniPlayer).map(|b| b.is_bonded).unwrap_or(false);
        assert!(!mini_is_slave);

        // Equalizer has parent MiniPlayer -> is Slave -> caption buttons hidden
        let eq_is_slave = tree.get(&WindowKind::Equalizer).map(|b| b.is_bonded).unwrap_or(false);
        assert!(eq_is_slave);

        break_dock_relation(WindowKind::Equalizer);
    }

    #[test]
    fn test_cluster_members_retrieval_and_bring_to_top() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Single standalone window
        let single = get_cluster_members(WindowKind::Main);
        assert_eq!(single, vec![WindowKind::Main]);

        // 2. Form cluster: Main <-> Visualizer <-> Equalizer
        let main_r = RECT { left: 0, top: 0, right: 400, bottom: 600 };
        let vis_r = RECT { left: 400, top: 0, right: 800, bottom: 600 };
        let eq_r = RECT { left: 800, top: 0, right: 1200, bottom: 500 };

        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::Visualizer, &eq_r, &vis_r);

        // Querying from any member of the cluster should return all 3 members!
        let from_main = get_cluster_members(WindowKind::Main);
        let from_vis = get_cluster_members(WindowKind::Visualizer);
        let from_eq = get_cluster_members(WindowKind::Equalizer);

        assert_eq!(from_main.len(), 3);
        assert_eq!(from_vis.len(), 3);
        assert_eq!(from_eq.len(), 3);

        for member in &[WindowKind::Main, WindowKind::Visualizer, WindowKind::Equalizer] {
            assert!(from_main.contains(member));
            assert!(from_vis.contains(member));
            assert!(from_eq.contains(member));
        }

        // Standalone window (MiniPlayer) is not in this cluster
        let from_mini = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(from_mini, vec![WindowKind::MiniPlayer]);

        break_dock_relation(WindowKind::Equalizer);
        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_spawn_priority_cascade_and_center_fallback() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Main window only: next window should spawn at Main's Right-Top (Slot 1)
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 700 };
        update_known_rect(WindowKind::Main, main_r);

        let (spawn1, anchor1) = find_best_spawn_rect(WindowKind::MiniPlayer, 300, 200, Some(WindowKind::Main));
        assert_eq!(anchor1, Some(WindowKind::Main));
        assert_eq!(spawn1.left, 500);
        assert_eq!(spawn1.top, 100);
        assert_eq!(spawn1.right, 800);
        assert_eq!(spawn1.bottom, 300);

        // 2. MiniPlayer is now placed at Main's Right-Top:
        // Next window (Equalizer) should spawn at Main's Right-Bottom (Slot 2, directly below MiniPlayer)
        let mini_r = spawn1;
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        let (spawn2, anchor2) = find_best_spawn_rect(WindowKind::Equalizer, 300, 350, Some(WindowKind::Main));
        assert_eq!(anchor2, Some(WindowKind::Main));
        assert_eq!(spawn2.left, 500);
        assert_eq!(spawn2.top, 300); // below MiniPlayer (MiniPlayer's bottom is 300)
        assert_eq!(spawn2.right, 800);
        assert_eq!(spawn2.bottom, 650);

        // 3. Right edge of Main is fully occupied by a tall Visualizer:
        // When spawning from Visualizer (Visualizer is the mother window):
        // Main: (100, 100, 500, 700), Visualizer: (500, 100, 1200, 700)
        reset_all();
        update_known_rect(WindowKind::Main, main_r);
        let vis_r = RECT { left: 500, top: 100, right: 1200, bottom: 700 };
        update_known_rect(WindowKind::Visualizer, vis_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);

        // Next window spawned from Visualizer should spawn at Visualizer's Right-Top
        let (spawn3, anchor3) = find_best_spawn_rect(WindowKind::MiniPlayer, 300, 200, Some(WindowKind::Visualizer));
        assert_eq!(anchor3, Some(WindowKind::Visualizer));
        assert_eq!(spawn3.left, 1200);
        assert_eq!(spawn3.top, 100);
        assert_eq!(spawn3.right, 1500);
        assert_eq!(spawn3.bottom, 300);

        // 4. Mother window Bottom edge (Priority 3):
        // Main: (100, 100, 500, 500).
        // Right is blocked by Right-Docked tall window down to work area bottom (500, 100, 800, 1080).
        // Left is blocked by screen edge (width 300 cannot fit in 0..100).
        // Window should dock at Mother window's Bottom edge (Slot 7)
        reset_all();
        let main_small = RECT { left: 100, top: 100, right: 500, bottom: 500 };
        update_known_rect(WindowKind::Main, main_small);
        let block_r = RECT { left: 500, top: 100, right: 800, bottom: 1080 };
        update_known_rect(WindowKind::Visualizer, block_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &block_r, &main_small);

        let (spawn_bottom, anchor_bottom) = find_best_spawn_rect(WindowKind::MiniPlayer, 300, 200, Some(WindowKind::Main));
        assert_eq!(anchor_bottom, Some(WindowKind::Main));
        assert_eq!(spawn_bottom.left, 100); // Main's left
        assert_eq!(spawn_bottom.top, 500);  // Main's bottom
        assert_eq!(spawn_bottom.right, 400);
        assert_eq!(spawn_bottom.bottom, 700);

        // 5. Screen overflow fallback:
        // Reset and position Visualizer at right 1200, so a window of width 800 and height 500
        // overflows all possible sides:
        // Main Right: blocked by vis_r (500..1200)
        // Main Left: 100 - 800 = -700 (outside work area)
        // Main Bottom: 700 + 500 = 1200 > 1080 (outside work area)
        // Secondary Vis Right: 1200 + 800 = 2000 > 1920 (outside work area)
        // Secondary Vis Bottom: 700 + 500 = 1200 > 1080 (outside work area)
        // -> Falls back to Screen Center!
        reset_all();
        update_known_rect(WindowKind::Main, main_r);
        let vis_r = RECT { left: 500, top: 100, right: 1200, bottom: 700 };
        update_known_rect(WindowKind::Visualizer, vis_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);

        let (spawn_overflow, anchor_overflow) = find_best_spawn_rect(WindowKind::Equalizer, 800, 500, Some(WindowKind::Main));
        assert_eq!(anchor_overflow, None); // Fallback: standalone in center!
        // Center of work area (1920 x 1080):
        // left = (1920 - 800) / 2 = 560
        // top = (1080 - 500) / 2 = 290
        assert_eq!(spawn_overflow.left, 560);
        assert_eq!(spawn_overflow.top, 290);
        assert_eq!(spawn_overflow.right, 1360);
        assert_eq!(spawn_overflow.bottom, 790);

        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_summon_and_dock_breaks_separated_dock_edges() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initially, Main and MiniPlayer are docked together
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 }; // docked on Right of Main
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        // 2. Visualizer is independent at (1400, 100, 1800, 600)
        let vis_r = RECT { left: 1400, top: 100, right: 1800, bottom: 600 };

        // 3. Visualizer summons Main window to dock to Visualizer (Main moves to left of Visualizer: 1000..1400)
        let new_main_r = RECT { left: 1000, top: 100, right: 1400, bottom: 600 };
        // MiniPlayer remains at (500, 100, 900, 350), which is > 24px away from (1000..1400)
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &new_main_r);

        // 4. Verification:
        // Main and Visualizer are now bonded in the same component
        assert!(is_descendant(WindowKind::Visualizer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);

        // MiniPlayer is separated from Main (distance > tolerance), so dock edge was severed!
        assert!(!is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);

        let tree = with_dock_tree(|t| t.clone());
        let mini_bond = tree.get(&WindowKind::MiniPlayer).map(|b| b.is_bonded).unwrap_or(false);
        assert!(!mini_bond, "MiniPlayer must no longer be bonded to Main");

        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_window_role_detection() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // Standalone initially
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);

        // Dock Equalizer to Main
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let eq_r = RECT { left: 500, top: 100, right: 960, bottom: 540 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);

        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);

        // Break dock relation
        break_dock_relation(WindowKind::Equalizer);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);
    }

    #[test]
    fn test_cluster_minimize_guard_and_role_handling() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        assert!(!is_cluster_minimizing());

        // Dock Visualizer and MiniPlayer to Main
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let vis_r = RECT { left: 500, top: 100, right: 1200, bottom: 600 };
        let mini_r = RECT { left: 100, top: 600, right: 500, bottom: 850 };
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Visualizer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);

        // Verify cluster members retrieval
        let members = get_cluster_members(WindowKind::Visualizer);
        assert_eq!(members.len(), 3);
        assert_eq!(members[0], WindowKind::Main);

        break_dock_relation(WindowKind::Visualizer);
        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_root_close_reassembles_remaining_cluster_in_place() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Main at (100, 100, 500, 600)
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        // MiniPlayer at (500, 100, 900, 350) - docked to right of Main
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        // Equalizer at (500, 350, 900, 700) - docked below MiniPlayer (touching MiniPlayer)
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 700 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);

        // Both MiniPlayer and Equalizer dock to Main, while touching each other
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);

        // Before closing Main: Main is Master of the cluster
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::Main);

        // 2. User closes Root window (Main) -> hides to tray and reassembles remaining windows
        crate::tray::IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        handle_window_closed(WindowKind::Main);

        // 3. Verify in-place re-assembly:
        // Main is no longer in the cluster
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        // MiniPlayer (Level 2) is promoted to the new ClusterMaster!
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterMaster);
        // Equalizer (Level 3) is synthesized as ClusterSlave bonded to MiniPlayer!
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::MiniPlayer);

        break_dock_relation(WindowKind::MiniPlayer);
        break_dock_relation(WindowKind::Equalizer);
        crate::tray::IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_spring_overshoot_curve_and_magnetic_snap() {
        // 1. Boundary conditions: t=0 -> 0.0, t=1 -> 1.0
        let s0 = spring_overshoot_curve(0.0);
        let s1 = spring_overshoot_curve(1.0);
        assert!((s0 - 0.0).abs() < 1e-4, "Spring curve at t=0 must start at 0: got {s0}");
        assert!((s1 - 1.0).abs() < 1e-4, "Spring curve at t=1 must end at 1: got {s1}");

        // 2. Physical overshoot verification: in t in 0.35..0.55, curve should produce a tactile overshoot (> 1.0)
        let mut max_val = 0.0f32;
        let mut max_t = 0.0f32;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let val = spring_overshoot_curve(t);
            if val > max_val {
                max_val = val;
                max_t = t;
            }
        }
        assert!(max_val > 1.04 && max_val < 1.15, "Spring overshoot should peak between 4% and 15%: got {max_val}");
        assert!(max_t >= 0.35 && max_t <= 0.60, "Spring peak should occur in mid motion: got {max_t}");

        // 3. Smooth monotonic approach at the end: values between 0.9 and 1.0 converge to 1.0
        let s09 = spring_overshoot_curve(0.9);
        assert!((s09 - 1.0).abs() < 0.02, "Spring curve should settle close to 1.0 by t=0.9: got {s09}");
    }

    #[test]
    fn test_snap_rect_to_rect_magnetic() {
        // Target stationary rectangle B: [200, 200, 500, 500] (w=300, h=300)
        let r_b = RECT { left: 200, top: 200, right: 500, bottom: 500 };

        // Case 1: Moving A is near B's Left edge -> snaps cleanly to B's Left
        let mut r_a_left = RECT { left: 80, top: 210, right: 195, bottom: 410 };
        assert!(snap_rect_to_rect(&mut r_a_left, &r_b, 12));
        assert_eq!(r_a_left.right, 200);

        // Case 2: Moving A is near B's Right edge -> snaps cleanly to B's Right
        let mut r_a_right = RECT { left: 508, top: 210, right: 620, bottom: 410 };
        assert!(snap_rect_to_rect(&mut r_a_right, &r_b, 12));
        assert_eq!(r_a_right.left, 500);

        // Case 3: Moving A is near B's Top edge -> snaps cleanly to B's Top
        let mut r_a_top = RECT { left: 220, top: 90, right: 420, bottom: 196 };
        assert!(snap_rect_to_rect(&mut r_a_top, &r_b, 12));
        assert_eq!(r_a_top.bottom, 200);

        // Case 4: Moving A is near B's Bottom edge -> snaps cleanly to B's Bottom
        let mut r_a_bottom = RECT { left: 220, top: 506, right: 420, bottom: 610 };
        assert!(snap_rect_to_rect(&mut r_a_bottom, &r_b, 12));
        assert_eq!(r_a_bottom.top, 500);

        // Case 5: Far away -> No snap
        let mut r_a_far = RECT { left: 800, top: 800, right: 900, bottom: 900 };
        assert!(!snap_rect_to_rect(&mut r_a_far, &r_b, 12));
    }

    #[test]
    fn test_per_window_pinning_and_cluster_propagation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Reset all dock edges and pin states
        remove_all_edges_for(WindowKind::Main);
        remove_all_edges_for(WindowKind::MiniPlayer);
        remove_all_edges_for(WindowKind::Equalizer);
        remove_all_edges_for(WindowKind::Visualizer);
        unsafe { rebuild_dock_tree(); }

        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);
        set_window_pinned(WindowKind::Visualizer, false);

        // 2. Standalone autonomy: toggling MiniPlayer does NOT affect Main
        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer));

        unsafe { toggle_window_pin(WindowKind::MiniPlayer); }
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should now be pinned");
        assert!(!is_window_pinned(WindowKind::Main), "Main window must remain unpinned (no global linking)");

        // 3. Docking assimilation: When MiniPlayer docks to unpinned Main, it inherits Main's unpinned state
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should inherit anchor's unpinned state upon docking");

        // 4. Cluster consistency: Toggling on cluster member propagates to entire cluster
        unsafe { toggle_window_pin(WindowKind::MiniPlayer); }
        assert!(is_window_pinned(WindowKind::Main), "Main should be pinned with cluster");
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should be pinned with cluster");

        // 5. Detach retention: When MiniPlayer breaks away, it retains current pinned state
        break_dock_relation(WindowKind::MiniPlayer);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should retain pinned state upon detach");

        // And can now be independently unpinned without unpinning Main
        unsafe { toggle_window_pin(WindowKind::MiniPlayer); }
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should be unpinned");
        assert!(is_window_pinned(WindowKind::Main), "Main window should still remain pinned");
    }

    #[test]
    fn test_anchor_precedence_assimilation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. 初始化重設所有視窗狀態與拓撲關係
        remove_all_edges_for(WindowKind::Main);
        remove_all_edges_for(WindowKind::MiniPlayer);
        remove_all_edges_for(WindowKind::Equalizer);
        remove_all_edges_for(WindowKind::Visualizer);
        unsafe { rebuild_dock_tree(); }

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        let vis_r = RECT { left: 900, top: 100, right: 1500, bottom: 600 };

        // 情境 A：錨點置頂 (Anchor=True)，拖入的新成員未置頂 (Moving=False) -> 新成員同化為 True
        set_window_pinned(WindowKind::Main, true);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must be assimilated to anchor's pinned=true");

        // 拆解分離
        break_dock_relation(WindowKind::MiniPlayer);
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer retains pinned=true after detach");

        // 情境 B：錨點未置頂 (Anchor=False)，拖入的新成員置頂 (Moving=True) -> 新成員同化為 False
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::Equalizer, true);
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);

        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::Equalizer), "Equalizer must be assimilated to anchor's unpinned=false");

        // 拆解分離
        break_dock_relation(WindowKind::Equalizer);

        // 情境 C：非 Main 視窗相互吸附時，目標錨點居主導地位
        // MiniPlayer (Anchor, pinned=true) <- Visualizer (Moving, pinned=false)
        set_window_pinned(WindowKind::MiniPlayer, true);
        set_window_pinned(WindowKind::Visualizer, false);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        assert!(is_window_pinned(WindowKind::MiniPlayer));
        assert!(is_window_pinned(WindowKind::Visualizer), "Visualizer must assimilate to MiniPlayer anchor pinned=true");

        // 情境 D：多視窗連通叢集吸附至錨點時，全體叢集成員同態一致
        // 現有叢集 MiniPlayer + Visualizer (皆為 true)，被吸附至 Main (Anchor, pinned=false)
        set_window_pinned(WindowKind::Main, false);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must be assimilated to Main's unpinned=false");
        assert!(!is_window_pinned(WindowKind::Visualizer), "Visualizer in cluster must also be assimilated to Main's unpinned=false");
    }

    #[test]
    fn test_window_opacity_and_hover_peek() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let orig_main_op = get_window_opacity(WindowKind::Main);
        let orig_mini_op = get_window_opacity(WindowKind::MiniPlayer);
        let orig_eq_op = get_window_opacity(WindowKind::Equalizer);
        let orig_vis_op = get_window_opacity(WindowKind::Visualizer);
        let orig_peek = is_hover_peek_enabled();

        // 1. Initial / default bounds
        set_window_opacity(WindowKind::Main, 0.80);
        assert!((get_window_opacity(WindowKind::Main) - 0.80).abs() < 0.01);

        set_window_opacity(WindowKind::MiniPlayer, 0.75);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 0.75).abs() < 0.01);

        // 2. Clamp verification: min 0.2, max 1.0
        set_window_opacity(WindowKind::Main, 0.05);
        assert!((get_window_opacity(WindowKind::Main) - 0.20).abs() < 0.01);

        set_window_opacity(WindowKind::MiniPlayer, 1.50);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 1.00).abs() < 0.01);

        // 3. Equalizer & Visualizer opacity
        set_window_opacity(WindowKind::Equalizer, 0.70);
        assert!((get_window_opacity(WindowKind::Equalizer) - 0.70).abs() < 0.01);

        set_window_opacity(WindowKind::Visualizer, 0.60);
        assert!((get_window_opacity(WindowKind::Visualizer) - 0.60).abs() < 0.01);

        // 4. Suppress peek checking via active tuning
        assert!(!is_peek_suppressed(WindowKind::MiniPlayer));
        trigger_opacity_tuning(WindowKind::MiniPlayer);
        assert!(is_peek_suppressed(WindowKind::MiniPlayer));

        // 5. Hover peek toggling
        set_hover_peek_enabled(true);
        assert!(is_hover_peek_enabled());
        set_hover_peek_enabled(false);
        assert!(!is_hover_peek_enabled());
        set_hover_peek_enabled(true);
        assert!(is_hover_peek_enabled());

        // 6. Cluster opacity synchronization test
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        set_cluster_opacity(WindowKind::MiniPlayer, 0.45);
        assert!((get_window_opacity(WindowKind::Main) - 0.45).abs() < 0.01);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 0.45).abs() < 0.01);
        assert!(is_peek_suppressed(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::MiniPlayer));
        assert!(is_opacity_hud_active(WindowKind::Main));
        assert!(is_opacity_hud_active(WindowKind::MiniPlayer));
        break_dock_relation(WindowKind::MiniPlayer);

        // 7. Click-through mode threshold verification (<= 40%)
        set_window_opacity(WindowKind::Main, 0.45);
        assert!(!is_click_through_active(WindowKind::Main));
        set_window_opacity(WindowKind::Main, 0.40);
        assert!(is_click_through_active(WindowKind::Main));
        set_window_opacity(WindowKind::Main, 0.35);
        assert!(is_click_through_active(WindowKind::Main));
        set_window_opacity(WindowKind::Main, 0.20);
        assert!(is_click_through_active(WindowKind::Main));

        // Restore original user settings so unit test NEVER pollutes user's config.json
        set_window_opacity(WindowKind::Main, orig_main_op);
        set_window_opacity(WindowKind::MiniPlayer, orig_mini_op);
        set_window_opacity(WindowKind::Equalizer, orig_eq_op);
        set_window_opacity(WindowKind::Visualizer, orig_vis_op);
        set_hover_peek_enabled(orig_peek);
    }

    #[test]
    fn test_opacity_fsm_lifecycle_and_transitions() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initial state is Normal
        assert_eq!(get_opacity_state(WindowKind::Main), OpacityState::Normal);
        assert!(!is_opacity_hud_active(WindowKind::Main));
        assert!(!is_peek_suppressed(WindowKind::Main));
        assert!(!is_opacity_menu_open(WindowKind::Main));

        // 2. Trigger active tuning
        trigger_opacity_tuning(WindowKind::Main);
        assert!(matches!(get_opacity_state(WindowKind::Main), OpacityState::Tuning(_)));
        assert!(is_opacity_hud_active(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::Main));

        // 3. Open menu
        set_opacity_menu_open(WindowKind::Main, true);
        assert_eq!(get_opacity_state(WindowKind::Main), OpacityState::MenuOpen);
        assert!(is_opacity_menu_open(WindowKind::Main));
        assert!(!is_opacity_hud_active(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::Main));

        // 4. Close menu -> transitions to Fading
        set_opacity_menu_open(WindowKind::Main, false);
        assert!(matches!(get_opacity_state(WindowKind::Main), OpacityState::Fading(_)));
        assert!(!is_opacity_menu_open(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::Main));

        // 5. Cluster tuning propagation
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        trigger_cluster_opacity_tuning(WindowKind::MiniPlayer);
        assert!(is_opacity_hud_active(WindowKind::Main));
        assert!(is_opacity_hud_active(WindowKind::MiniPlayer));
        assert!(is_peek_suppressed(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::MiniPlayer));

        break_dock_relation(WindowKind::MiniPlayer);
        reset_all();
    }

    #[test]
    fn test_minimize_window_solo() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);

        // Solo minimize MiniPlayer: it should break dock relation and become standalone
        unsafe {
            minimize_window_solo(WindowKind::MiniPlayer);
        }

        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
    }

    #[test]
    fn test_reverse_spring_curve_and_staggered_collapse() {
        // Boundary values
        assert_eq!(reverse_spring_curve(0.0), 0.0);
        let end_val = reverse_spring_curve(1.0);
        assert!((end_val - 1.0).abs() < 1e-4);

        // Monotonic progression
        let mid1 = reverse_spring_curve(0.3);
        let mid2 = reverse_spring_curve(0.7);
        assert!(mid1 > 0.0 && mid1 < mid2 && mid2 < 1.0);

        // Staggered delay behavior
        assert_eq!(reverse_staggered_spring(0.04, 0.08), 0.0);
        assert!(reverse_staggered_spring(0.12, 0.08) > 0.0);
        assert!((reverse_staggered_spring(1.0, 0.08) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_cluster_taskbar_ownership_sync_simulation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Standalone initially
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);

        // 2. Form a sub-cluster of small windows (MiniPlayer + Equalizer)
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::MiniPlayer);

        // Safe call to sync (handles null HWND safely in mock/test)
        unsafe {
            sync_cluster_taskbar_ownership();
        }

        // 3. Main hidden in tray simulation: ensures no panic and proper state handling
        crate::tray::IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        unsafe {
            sync_cluster_taskbar_ownership();
        }
        assert!(crate::tray::is_window_hidden_to_tray());
        crate::tray::IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);

        // Form two separate clusters: Cluster A (Main) and Cluster B (MiniPlayer + Equalizer)
        let main_r = RECT { left: 0, top: 0, right: 460, bottom: 680 };
        update_known_rect(WindowKind::Main, main_r);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);

        // Both Cluster Masters (Main and MiniPlayer) have independent taskbar presence,
        // while Equalizer is grouped under MiniPlayer.
        unsafe {
            sync_cluster_taskbar_ownership();
        }

        break_dock_relation(WindowKind::Equalizer);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
    }

    #[test]
    fn test_compute_cluster_docked_targets() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 建立 Main(root) -> MiniPlayer(Right) -> Equalizer(Bottom) 的三級串聯結構
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 }; // 460 x 680
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 }; // 460 x 340
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 }; // 460 x 340

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // 假設 Main 還原到了新坐標 (200, 150)
        let new_main_r = RECT { left: 200, top: 150, right: 660, bottom: 830 };
        let targets = compute_cluster_docked_targets(WindowKind::Main, &new_main_r);

        let main_target = targets.get(&WindowKind::Main).expect("Main target exists");
        assert_eq!(main_target.left, new_main_r.left);
        assert_eq!(main_target.top, new_main_r.top);
        assert_eq!(main_target.right, new_main_r.right);
        assert_eq!(main_target.bottom, new_main_r.bottom);

        let mini_target = targets.get(&WindowKind::MiniPlayer).expect("MiniPlayer target exists");
        assert_eq!(mini_target.left, 660);
        assert_eq!(mini_target.top, 150);

        let eq_target = targets.get(&WindowKind::Equalizer).expect("Equalizer target exists");
        assert_eq!(eq_target.left, 660);
        assert_eq!(eq_target.top, 150 + (mini_target.bottom - mini_target.top));
    }

    #[test]
    fn test_staggered_spring_expand_behavior() {
        // 1. Initial boundary
        assert_eq!(staggered_spring(0.0, 0.0), 0.0);
        assert_eq!(staggered_spring(0.04, 0.08), 0.0); // 未達延遲前保持靜止

        // 2. 局部超調（Overshoot）特性檢驗：物理彈簧在接近收斂前會產生微幅 > 1.0 的彈性手感
        let mut has_overshoot = false;
        for step in 1..=20 {
            let t = step as f32 / 20.0;
            let val = staggered_spring(t, 0.0);
            if val > 1.0 {
                has_overshoot = true;
                break;
            }
        }
        assert!(has_overshoot, "Physical spring curve must exhibit natural overshoot for haptic feedback");

        // 3. 終端收斂精確性
        let end_val = staggered_spring(1.0, 0.08);
        assert!((end_val - 1.0).abs() < 1e-3, "Spring must converge to 1.0 at termination");
    }

    #[test]
    fn test_expand_cluster_animated_lock() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();
        assert!(!is_cluster_restoring());
        set_cluster_state(WindowKind::Main, ClusterState::Restoring(std::time::Instant::now()));
        assert!(is_cluster_restoring());
        finish_cluster_restore(WindowKind::Main);
        assert!(!is_cluster_restoring());
    }

    #[test]
    fn test_collapsed_silent_hide_simulation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        let members = get_cluster_members(WindowKind::Main);
        assert_eq!(members.len(), 2);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        // 模擬在 HWND 為 null 的環境下安全調用 restore_cluster，無崩潰
        unsafe {
            restore_cluster(WindowKind::Main, std::ptr::null_mut());
        }
    }

    #[test]
    fn test_is_main_in_cluster_and_cluster_summon() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. MiniPlayer 與 Equalizer 組成小群組
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        assert!(!is_main_in_cluster(WindowKind::MiniPlayer));
        assert!(!is_main_in_cluster(WindowKind::Equalizer));

        // 2. 模擬主視窗吸附至該小群組（停靠在整組的左側）
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_main_in_cluster(WindowKind::MiniPlayer));
        assert!(is_main_in_cluster(WindowKind::Equalizer));
        assert!(is_main_in_cluster(WindowKind::Main));

        // 3. 再次調用 handle_home_summon_request 應安全聚焦主視窗
        unsafe {
            handle_home_summon_request(WindowKind::Equalizer);
        }
    }

    #[test]
    fn test_hud_pinned_state_backup_and_restoration() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Setup cluster: Main + MiniPlayer + Equalizer
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // Initially unpinned
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);

        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer));
        assert!(!is_window_pinned(WindowKind::Equalizer));

        // 2. Simulate entering fullscreen HUD
        crate::visualizer::show_hud_cluster();

        assert!(is_window_pinned(WindowKind::Main), "Main must be pinned in HUD");
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must be pinned in HUD");
        assert!(is_window_pinned(WindowKind::Equalizer), "Equalizer must be pinned in HUD");

        // 3. Simulate dragging Main window during HUD:
        // assimilate_cluster_to_anchor must preserve pinned=true!
        unsafe {
            assimilate_cluster_to_anchor(WindowKind::Main, WindowKind::MiniPlayer);
        }
        assert!(is_window_pinned(WindowKind::Main), "Main must stay pinned after drag");
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must stay pinned after drag");
        assert!(is_window_pinned(WindowKind::Equalizer), "Equalizer must stay pinned after drag");

        // 4. Simulate hiding HUD
        crate::visualizer::hide_hud_cluster();

        assert!(!is_window_pinned(WindowKind::Main), "Main must restore to unpinned after hiding HUD");
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must restore to unpinned after hiding HUD");
        assert!(!is_window_pinned(WindowKind::Equalizer), "Equalizer must restore to unpinned after hiding HUD");

        // 5. Simulate exiting fullscreen directly while HUD is open
        crate::visualizer::show_hud_cluster();
        assert!(is_window_pinned(WindowKind::Main));
        crate::visualizer::restore_hud_pinned_states();
        assert!(!is_window_pinned(WindowKind::Main), "Main must restore to unpinned on fullscreen exit");
        assert!(!is_window_pinned(WindowKind::MiniPlayer));
        assert!(!is_window_pinned(WindowKind::Equalizer));
    }

    #[test]
    fn test_home_summon_joins_mini_cluster_with_visualizer() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. 設置工作站拆分後的殘留群組：MiniPlayer + Equalizer + Visualizer
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 };
        let vis_r = RECT { left: 1020, top: 100, right: 1480, bottom: 780 };

        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        // 驗證三者已成一組，且主視窗不在其中
        let mini_cluster = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(mini_cluster.len(), 3);
        assert!(mini_cluster.contains(&WindowKind::MiniPlayer));
        assert!(mini_cluster.contains(&WindowKind::Equalizer));
        assert!(mini_cluster.contains(&WindowKind::Visualizer));
        assert!(!mini_cluster.contains(&WindowKind::Main));

        // 2. 主視窗隱藏於托盤
        crate::tray::IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        let orig_main_r = RECT { left: -32000, top: -32000, right: -31540, bottom: -31320 };
        update_known_rect(WindowKind::Main, orig_main_r);

        // 3. 模擬使用者在 MiniPlayer 上點擊 🏠 (Home 房子圖標)
        // 模擬主視窗回到目標停靠位 (停在群組最左側即 MiniPlayer 左邊 100..560)
        let summoned_main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        update_known_rect(WindowKind::Main, summoned_main_r);
        crate::tray::IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);

        // 執行主視窗與 MiniPlayer 的 dock 關係綁定
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &summoned_main_r);

        // 4. 驗證全體 4 個視窗全部成功合成同一個群組！
        let final_cluster_from_main = get_cluster_members(WindowKind::Main);
        let final_cluster_from_mini = get_cluster_members(WindowKind::MiniPlayer);
        let final_cluster_from_vis = get_cluster_members(WindowKind::Visualizer);

        assert_eq!(final_cluster_from_main.len(), 4, "Main 必須與原先三視窗群組合併為 4 視窗大群組");
        assert_eq!(final_cluster_from_mini.len(), 4);
        assert_eq!(final_cluster_from_vis.len(), 4);

        assert!(final_cluster_from_mini.contains(&WindowKind::Main));
        assert!(final_cluster_from_mini.contains(&WindowKind::Visualizer));
        assert!(final_cluster_from_mini.contains(&WindowKind::Equalizer));
        assert!(final_cluster_from_mini.contains(&WindowKind::MiniPlayer));
    }

    #[test]
    fn test_per_cluster_collapsed_state_isolation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Group 1: Main + Equalizer
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        let eq_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::Main);

        // 2. Group 2: MiniPlayer + Visualizer
        let mini_r = RECT { left: 1100, top: 100, right: 1560, bottom: 440 };
        let vis_r = RECT { left: 1100, top: 440, right: 1560, bottom: 780 };
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::MiniPlayer);

        // 3. Both groups collapse independently
        set_cluster_state(WindowKind::Main, ClusterState::Minimized);
        set_cluster_state(WindowKind::MiniPlayer, ClusterState::Minimized);

        assert_eq!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);
        assert_eq!(get_cluster_state(WindowKind::MiniPlayer), ClusterState::Minimized);

        // 4. Restore Group 2 (MiniPlayer) first
        assert!(can_start_cluster_restore(WindowKind::MiniPlayer));
        assert_ne!(get_cluster_state(WindowKind::MiniPlayer), ClusterState::Minimized);

        // Critical check: Group 1 (Main) MUST STILL BE MINIMIZED! No state leak or overwrite!
        assert_eq!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);

        // 5. Restore Group 1 (Main)
        assert!(can_start_cluster_restore(WindowKind::Main));
        assert_ne!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);
    }

    #[test]
    fn test_cluster_fsm_lifecycle_and_timeout() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let root = WindowKind::Main;
        assert_eq!(get_cluster_state(root), ClusterState::Normal);

        // 1. Can start minimize
        assert!(can_start_cluster_minimize(root));
        assert!(is_kind_cluster_minimizing(root));
        assert!(is_cluster_minimizing());

        // While minimizing, cannot start another minimize
        assert!(!can_start_cluster_minimize(root));

        // Finish minimize
        finish_cluster_minimize(root);
        assert_eq!(get_cluster_state(root), ClusterState::Minimized);
        assert!(!is_cluster_minimizing());

        // 2. Can start restore
        assert!(can_start_cluster_restore(root));
        assert!(is_kind_cluster_restoring(root));
        assert!(is_cluster_restoring());

        // Finish restore
        finish_cluster_restore(root);
        assert_eq!(get_cluster_state(root), ClusterState::Normal);
        assert!(!is_cluster_restoring());

        // 3. Timeout self-healing simulation (> 450ms)
        let past = std::time::Instant::now() - std::time::Duration::from_millis(500);
        set_cluster_state(root, ClusterState::Minimizing(past));
        assert_eq!(get_cluster_state(root), ClusterState::Minimized, "Minimizing past 450ms must auto-resolve to Minimized");

        set_cluster_state(root, ClusterState::Restoring(past));
        assert_eq!(get_cluster_state(root), ClusterState::Normal, "Restoring past 450ms must auto-resolve to Normal");
    }

    #[test]
    fn test_rebuild_dock_tree_preserves_minimized_cluster_bonds() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Setup layout: Main alone, Group 2: MiniPlayer, Equalizer, Visualizer
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 600, top: 100, right: 1000, bottom: 350 };
        let eq_r = RECT { left: 600, top: 350, right: 1000, bottom: 600 };
        let vis_r = RECT { left: 1000, top: 100, right: 1400, bottom: 600 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        // Group 2 dock relations: Equalizer -> MiniPlayer, Visualizer -> MiniPlayer
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        let g2_members = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(g2_members.len(), 3, "Group 2 must have 3 members initially");
        assert!(g2_members.contains(&WindowKind::MiniPlayer));
        assert!(g2_members.contains(&WindowKind::Equalizer));
        assert!(g2_members.contains(&WindowKind::Visualizer));

        // 2. Step 2: Minimize Group 2
        set_cluster_state(WindowKind::MiniPlayer, ClusterState::Minimized);

        // 3. Step 3: Minimize Main Window (calls minimize_window_solo which invokes rebuild_dock_tree)
        unsafe {
            rebuild_dock_tree();
        }

        // 4. Verify that Group 2's dock relations were NOT wiped out by rebuilding while minimized!
        let g2_members_after = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(
            g2_members_after.len(),
            3,
            "Rebuilding dock tree must NOT wipe out minimized cluster members! Found: {:?}",
            g2_members_after
        );
        assert!(g2_members_after.contains(&WindowKind::Equalizer));
        assert!(g2_members_after.contains(&WindowKind::Visualizer));

        // 5. Restore Group 2
        set_cluster_state(WindowKind::MiniPlayer, ClusterState::Normal);
        let g2_members_restored = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(g2_members_restored.len(), 3);
    }

    #[test]
    fn test_cluster_fsm_transitions_and_handover() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let root = WindowKind::MiniPlayer;

        // 1. Strict Transition testing
        assert_eq!(get_cluster_state(root), ClusterState::Normal);

        // Normal -> Minimizing
        assert!(can_start_cluster_minimize(root));
        assert!(is_kind_cluster_minimizing(root));

        // Duplicate minimize rejected while active minimizing
        assert!(!can_start_cluster_minimize(root));

        // Minimizing -> Minimized
        finish_cluster_minimize(root);
        assert_eq!(get_cluster_state(root), ClusterState::Minimized);

        // Minimized -> Restoring
        assert!(can_start_cluster_restore(root));
        assert!(is_kind_cluster_restoring(root));

        // Restoring -> Normal
        finish_cluster_restore(root);
        assert_eq!(get_cluster_state(root), ClusterState::Normal);

        // 2. Topology Handover: Root change simulation
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        let main_state = get_cluster_state(WindowKind::Main);
        assert_eq!(main_state, ClusterState::Normal);

        // Detach MiniPlayer: becomes independent root
        break_dock_relation(WindowKind::MiniPlayer);

        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);
        assert_eq!(get_cluster_state(WindowKind::MiniPlayer), ClusterState::Normal);
    }

    #[test]
    fn test_quick_opacity_toggle_lifecycle() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initial State: Opacity = 100%, Unpinned (State 1: Normal)
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(!is_window_pinned(WindowKind::Main));

        // 2. Solo Quick Toggle at 100% -> should jump directly to 30% and auto-lock pinned (State 3: Ghost)
        let next1 = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next1, 30);
        assert!((get_window_opacity(WindowKind::Main) - 0.30).abs() < 0.001);
        assert!(is_window_pinned(WindowKind::Main), "Ghost mode must auto-lock pinned!");

        // 3. Solo Quick Toggle at 30% -> should jump back to 100% (State 2: Pinned Solid)
        let next2 = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next2, 100);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(is_window_pinned(WindowKind::Main));

        // 4. Test Single-click Pin on Ghost Mode (State 3 -> State 1 direct exit):
        // Re-enter 30% Ghost Mode
        let next3 = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next3, 30);
        assert!(is_window_pinned(WindowKind::Main));
        // Single-click Pin to unpin while in Ghost mode -> must restore 100% and unpin (State 1), eliminating dead state!
        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0, "Unpinning from ghost mode must restore 100% opacity");
        assert!(!is_window_pinned(WindowKind::Main), "Window must be unpinned");
        clear_ghost_exit_record();

        // 5. Cluster Quick Toggle across docked windows (100% <-> 30%)
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        // Reset cluster to 100% unpinned
        set_cluster_opacity(WindowKind::Main, 1.0);
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);

        // Quick toggle cluster -> all cluster members switch to 30% and auto-lock pinned!
        let c_next1 = toggle_cluster_opacity_quick(WindowKind::Main);
        assert_eq!(c_next1, 30);
        assert!((get_window_opacity(WindowKind::Main) - 0.30).abs() < 0.001);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 0.30).abs() < 0.001);
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));

        // Quick toggle cluster again -> all switch back to 100%
        let c_next2 = toggle_cluster_opacity_quick(WindowKind::Main);
        assert_eq!(c_next2, 100);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert_eq!(get_window_opacity(WindowKind::MiniPlayer), 1.0);

        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_solution_c_titlebar_tap_to_wake_and_pure_pin() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initial State 1: 100% Solid, Unpinned
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(!is_window_pinned(WindowKind::Main));

        // 2. Pure Pin Toggle (State 1 <-> State 2)
        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert!(is_window_pinned(WindowKind::Main), "Click Pin in State 1 must transition to State 2 (Pinned)");
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);

        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert!(!is_window_pinned(WindowKind::Main), "Click Pin in State 2 must transition back to State 1 (Unpinned)");
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);

        // 3. Titlebar Double-click: toggles State 2 <-> State 3
        let next = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next, 30, "Titlebar double-click enters State 3 (30% Ghost, Pinned)");
        assert!(is_window_pinned(WindowKind::Main));

        // 4. Titlebar Symmetric Double-Click in State 3 -> State 2 (100% Solid, Pinned = true)
        let back = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(back, 100, "Symmetric double-click toggle restores 100% opacity");
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(is_window_pinned(WindowKind::Main), "Symmetric double-click must PRESERVE pinned state (State 2)!");

        // Also verify direct wake_single_window_ghost helper
        toggle_single_window_opacity_quick(WindowKind::Main); // Re-enter State 3
        wake_single_window_ghost(WindowKind::Main);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0, "Wake must restore 100% opacity");
        assert!(is_window_pinned(WindowKind::Main), "Wake must PRESERVE pinned state (State 2)!");

        // 5. In State 3, Single-click Pin (Unpin / 拔釘) -> State 1 (100% Solid, Unpinned)
        toggle_single_window_opacity_quick(WindowKind::Main); // Re-enter State 3
        assert_eq!(get_window_opacity(WindowKind::Main), 0.3);
        assert!(is_window_pinned(WindowKind::Main));

        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0, "Unpin from State 3 must restore 100% opacity");
        assert!(!is_window_pinned(WindowKind::Main), "Unpin from State 3 must unpin to State 1");

        // 6. Cluster Tap-to-Wake across docked windows
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        // Enter State 3 for cluster
        toggle_cluster_opacity_quick(WindowKind::Main);
        assert_eq!(get_window_opacity(WindowKind::Main), 0.3);
        assert_eq!(get_window_opacity(WindowKind::MiniPlayer), 0.3);
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));

        // Cluster Tap-to-Wake -> all cluster members become 100% and stay pinned (State 2)
        wake_cluster_ghost(WindowKind::Main);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert_eq!(get_window_opacity(WindowKind::MiniPlayer), 1.0);
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));

        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_squashed_rect_clamp_and_root_priority_preservation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Abnormal squashed rect (Win32 iconic 180x27)
        let squashed_main = RECT { left: 400, top: 300, right: 580, bottom: 327 };
        update_known_rect(WindowKind::Main, squashed_main);

        // get_window_normal_rect must clamp to min_size bounds (380x500) and preserve user coordinates
        let norm_r = unsafe { get_window_normal_rect(WindowKind::Main) };
        assert!(norm_r.right - norm_r.left >= 380, "Main width must be at least min_size width (380)");
        assert!(norm_r.bottom - norm_r.top >= 500, "Main height must be at least min_size height (500)");

        // 2. Priority check: Main (1) < MiniPlayer (2) < Equalizer (3) < Visualizer (4)
        assert_eq!(WindowKind::Main.priority(), 1);
        assert_eq!(WindowKind::MiniPlayer.priority(), 2);
        assert_eq!(WindowKind::Equalizer.priority(), 3);
        assert_eq!(WindowKind::Visualizer.priority(), 4);

        // 3. Cluster topology: When all 4 windows are connected, Main must ALWAYS be root
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 };
        let vis_r = RECT { left: 1020, top: 100, right: 1740, bottom: 780 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        // Add edges: Main <-> MiniPlayer, MiniPlayer <-> Equalizer, MiniPlayer <-> Visualizer
        add_dock_edge(WindowKind::Main, WindowKind::MiniPlayer);
        add_dock_edge(WindowKind::MiniPlayer, WindowKind::Equalizer);
        add_dock_edge(WindowKind::MiniPlayer, WindowKind::Visualizer);

        let active = vec![WindowKind::Main, WindowKind::MiniPlayer, WindowKind::Equalizer, WindowKind::Visualizer];
        let edges = vec![
            (WindowKind::Main, WindowKind::MiniPlayer),
            (WindowKind::MiniPlayer, WindowKind::Equalizer),
            (WindowKind::MiniPlayer, WindowKind::Visualizer),
        ];

        let rect_map = [
            (WindowKind::Main, main_r),
            (WindowKind::MiniPlayer, mini_r),
            (WindowKind::Equalizer, eq_r),
            (WindowKind::Visualizer, vis_r),
        ];

        let tree = build_dock_tree_from_edges_and_rects(&active, &edges, |k| {
            rect_map.iter().find(|(kind, _)| *kind == k).map(|(_, r)| *r)
        });

        with_dock_tree(|t| *t = tree);

        // Root of the connected component must be Main, NEVER Visualizer
        assert_eq!(get_component_root(WindowKind::Main), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);

        // Visualizer is slave bonded, not master
        assert_eq!(get_window_role(WindowKind::Visualizer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
    }
}
