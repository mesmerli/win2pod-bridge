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
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub static SNAPPING_ENABLED: AtomicBool = AtomicBool::new(true);
pub static SNAP_DISTANCE: AtomicI32 = AtomicI32::new(12);

pub const MAIN_WINDOW_SUBCLASS_ID: usize = 0x504F44; // "POD"
pub const EQUALIZER_WINDOW_SUBCLASS_ID: usize = 0x504F45; // "POE"
pub const MINI_PLAYER_WINDOW_SUBCLASS_ID: usize = 0x504F4D; // "POM"

// HWND storage for standalone viewports
static EQUALIZER_HWND: AtomicIsize = AtomicIsize::new(0);
static MINI_PLAYER_HWND: AtomicIsize = AtomicIsize::new(0);

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

    /// 視窗預設尺寸 [寬, 高]
    pub fn default_size(&self) -> [f32; 2] {
        match self {
            WindowKind::Equalizer => [460.0, 440.0],
            WindowKind::MiniPlayer => [460.0, 310.0],
            WindowKind::Main => [460.0, 680.0],
            WindowKind::Visualizer => [1000.0, 600.0],
        }
    }

    /// 視窗最小邊界限制 [寬, 高]
    pub fn min_size(&self) -> [f32; 2] {
        match self {
            WindowKind::Equalizer => [380.0, 360.0],
            WindowKind::MiniPlayer => [380.0, 240.0],
            WindowKind::Main => [380.0, 500.0],
            WindowKind::Visualizer => [600.0, 400.0],
        }
    }

    /// 根據視窗標題比對此視窗是否吻合（涵蓋多國語言規格）
    pub fn matches_window_title(&self, title: &str) -> bool {
        match self {
            WindowKind::Main => title == "Win2Pod",
            WindowKind::Visualizer => title.contains("Visualizer") || title.contains("視覺"),
            WindowKind::Equalizer => {
                crate::i18n::ALL_LANGUAGES.iter().any(|l| l.eq_window_title() == title)
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

// Backward compatibility types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainAnchor {
    Main,
    Visualizer,
    MiniPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualizerDockBond {
    pub is_bonded: bool,
    pub anchor: ChainAnchor,
    pub side: DockSide,
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiniPlayerDockBond {
    pub is_bonded: bool,
    pub side: DockSide,
    pub offset_x: i32,
    pub offset_y: i32,
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
        if IsWindowVisible(hwnd) != 0 {
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

/// Updates caption buttons (minimize, maximize, close) on a native Win32 window titlebar.
/// When docked as a Slave, all three buttons disappear in unison, making the ensemble
/// look like a unified panel. When Master (Root) or detached, they are restored.
pub unsafe fn set_window_caption_buttons(hwnd: HWND, visible: bool) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
    let new_style = if visible {
        current_style | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX
    } else {
        current_style & !(WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX)
    };
    if new_style != current_style {
        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);
        if visible {
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
        if !hwnd.is_null() && IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 {
            if kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
                continue;
            }
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut r);
            rects.insert(kind, r);
            active.push(kind);
            update_known_rect(kind, r);
        }
    }

    if active.is_empty() {
        #[cfg(test)]
        {
            if let Ok(guard) = LAST_KNOWN_RECTS.lock() {
                if let Some(ref map) = *guard {
                    for (&k, &r) in map {
                        rects.insert(k, r);
                        active.push(k);
                    }
                }
            }
        }
        #[cfg(not(test))]
        {
            with_dock_tree(|tree| {
                tree.clear();
            });
            return;
        }
    }

    let edges: Vec<(WindowKind, WindowKind)> = with_dock_edges(|edge_set| {
        edge_set.retain(|&(a, b)| {
            if !active.contains(&a) || !active.contains(&b) {
                return false;
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
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            let is_slave = new_tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false);
            set_window_caption_buttons(hwnd, !is_slave);
        }
    }
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
    }
}

pub fn break_dock_relation(child: WindowKind) {
    let parent = with_dock_tree(|tree| {
        tree.get(&child).map(|b| b.parent)
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

/// Brings all windows belonging to the same cluster as `active_kind` to the top of the Z-order,
/// stacking them contiguously without allowing any third-party windows in between.
pub unsafe fn bring_cluster_to_top(active_kind: WindowKind, active_hwnd: HWND) {
    if !SNAPPING_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if active_hwnd.is_null() || IsWindow(active_hwnd) == 0 {
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

    let is_topmost = crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) || crate::visualizer::is_visualizer_fullscreen();
    let top_tier = if is_topmost { HWND_TOPMOST } else { HWND_TOP };

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
            || (member_kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray())
        {
            continue;
        }
        if member_kind == WindowKind::Visualizer && crate::visualizer::is_visualizer_fullscreen() {
            continue;
        }
        if IsIconic(m_hwnd) != 0 {
            ShowWindow(m_hwnd, SW_SHOWNOACTIVATE);
        }
        let target_pos = if is_topmost { HWND_TOPMOST } else { prev_hwnd };
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
        if IsIconic(child_hwnd) != 0 {
            ShowWindow(child_hwnd, SW_SHOWNOACTIVATE);
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

pub fn set_mini_player_dock_bond(mini_rect: &RECT, main_rect: &RECT) {
    set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, mini_rect, main_rect);
}

pub fn break_mini_player_dock_bond() {
    break_dock_relation(WindowKind::MiniPlayer);
}

pub fn get_mini_player_dock_bond() -> MiniPlayerDockBond {
    with_dock_tree(|tree| {
        if let Some(b) = tree.get(&WindowKind::MiniPlayer) {
            MiniPlayerDockBond {
                is_bonded: b.is_bonded,
                side: b.side,
                offset_x: b.offset_x,
                offset_y: b.offset_y,
            }
        } else {
            MiniPlayerDockBond {
                is_bonded: false,
                side: DockSide::None,
                offset_x: 0,
                offset_y: 0,
            }
        }
    })
}

pub fn set_equalizer_dock_bond(eq_rect: &RECT, anchor_rect: &RECT, anchor: ChainAnchor) {
    let parent = match anchor {
        ChainAnchor::MiniPlayer => WindowKind::MiniPlayer,
        ChainAnchor::Visualizer => WindowKind::Visualizer,
        ChainAnchor::Main => WindowKind::Main,
    };
    set_dock_relation(WindowKind::Equalizer, parent, eq_rect, anchor_rect);
}

pub fn break_equalizer_dock_bond() {
    break_dock_relation(WindowKind::Equalizer);
}

pub fn get_equalizer_dock_bond() -> EqualizerDockBond {
    with_dock_tree(|tree| {
        if let Some(b) = tree.get(&WindowKind::Equalizer) {
            let anchor = match b.parent {
                WindowKind::MiniPlayer => ChainAnchor::MiniPlayer,
                WindowKind::Visualizer => ChainAnchor::Visualizer,
                _ => ChainAnchor::Main,
            };
            EqualizerDockBond {
                is_bonded: b.is_bonded,
                anchor,
                side: b.side,
                offset_x: b.offset_x,
                offset_y: b.offset_y,
            }
        } else {
            EqualizerDockBond {
                is_bonded: false,
                anchor: ChainAnchor::Main,
                side: DockSide::None,
                offset_x: 0,
                offset_y: 0,
            }
        }
    })
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
                if ((*rect_ptr).left - expected_x).abs() > dist + 8
                    || ((*rect_ptr).top - expected_y).abs() > dist + 8
                {
                    // User deliberately pulled slave away: sever edge in edge set so it unbonds on drop.
                    // DO NOT call rebuild_dock_tree() during WM_MOVING to prevent titlebar frame flicker and dragging jitter!
                    remove_dock_edge(kind, bond.parent);
                } else {
                    // Keep magnetically aligned to parent
                    snap_rect_to_rect(&mut *rect_ptr, &parent_rect, dist);
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

    // Check if `kind` is touching any eligible candidate not yet connected
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
            if are_rects_docked(&my_rect, &cand_rect, 4) {
                add_dock_edge(kind, cand);
            }
        }
    }

    rebuild_dock_tree();
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
        WM_MOVING => {
            let rect_ptr = lparam as *mut RECT;
            handle_window_moving(kind, hwnd, rect_ptr);
            return 1;
        }
        WM_WINDOWPOSCHANGED => {
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                let wp = lparam as *const WINDOWPOS;
                if !wp.is_null() && ((*wp).flags & SWP_NOSIZE == 0) {
                    let mut r: RECT = std::mem::zeroed();
                    GetWindowRect(hwnd, &mut r);
                    propagate_child_moves(kind, &r);
                }
            }
        }
        WM_EXITSIZEMOVE => {
            handle_window_exit_size_move(kind, hwnd);
        }
        WM_SIZE => {
            let resize_type = wparam as u32;
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                if resize_type == SIZE_MINIMIZED {
                    minimize_all_children(kind);
                } else if resize_type == SIZE_RESTORED {
                    if kind == WindowKind::Main {
                        crate::tray::ensure_window_on_screen(hwnd);
                    }
                    restore_and_realign_all_children(kind, hwnd);
                }
            }
        }
        WM_MOUSEACTIVATE => {
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                bring_cluster_to_top(kind, hwnd);
            }
        }
        WM_NCLBUTTONDOWN => {
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                bring_cluster_to_top(kind, hwnd);
            }
        }
        WM_ACTIVATE => {
            let active_type = (wparam & 0xFFFF) as u32;
            if active_type != WA_INACTIVE as u32 && SNAPPING_ENABLED.load(Ordering::Relaxed) {
                bring_cluster_to_top(kind, hwnd);
                restore_and_realign_all_children(kind, hwnd);
            }
        }
        WM_SYSCOMMAND => {
            let sc = (wparam & 0xFFF0) as u32;
            let is_slave = with_dock_tree(|tree| {
                tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false)
            });
            if is_slave && (sc == SC_MINIMIZE || sc == SC_MAXIMIZE) {
                return 0; // lock minimize/maximize on slave
            }
            if sc == SC_CLOSE {
                let members = get_cluster_members(kind);
                if members.len() > 1 {
                    crate::gui::request_close_cluster(kind);
                    if kind == WindowKind::Main {
                        return 0;
                    }
                }
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
            break_dock_relation(kind);
            match kind {
                WindowKind::Equalizer => set_equalizer_hwnd(std::ptr::null_mut()),
                WindowKind::MiniPlayer => set_mini_player_hwnd(std::ptr::null_mut()),
                _ => {}
            }
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub unsafe fn install_window_subclass(kind: WindowKind, hwnd: HWND) {
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    match kind {
        WindowKind::Main => {
            crate::tray::set_main_hwnd(hwnd as isize);
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
    let target_z = if pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };

    for &kind in &members {
        if kind == WindowKind::Visualizer {
            crate::visualizer::set_visualizer_pinned(pinned);
        } else {
            let hwnd = get_window_hwnd(kind);
            if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                SetWindowPos(hwnd, target_z, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
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
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;

    let spawner = get_window_spawner(kind);
    let (best_rect, anchor) = find_best_spawn_rect(kind, w, h, spawner);
    SetWindowPos(
        hwnd,
        std::ptr::null_mut(),
        best_rect.left,
        best_rect.top,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    );

    if let Some(anchor_kind) = anchor {
        let anchor_hwnd = get_window_hwnd(anchor_kind);
        if !anchor_hwnd.is_null() && IsWindow(anchor_hwnd) != 0 {
            let mut anchor_rect: RECT = std::mem::zeroed();
            GetWindowRect(anchor_hwnd, &mut anchor_rect);
            set_dock_relation(kind, anchor_kind, &best_rect, &anchor_rect);
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

/// Magnetically summon and dock the Main Window to the specified anchor window with a smooth slide animation.
pub unsafe fn summon_and_dock_main_window_animated(anchor_kind: WindowKind) {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    let anchor_hwnd = get_window_hwnd(anchor_kind);
    if main_hwnd.is_null() || anchor_hwnd.is_null() || IsWindow(main_hwnd) == 0 || IsWindow(anchor_hwnd) == 0 {
        return;
    }

    let mut anchor_rect: RECT = std::mem::zeroed();
    GetWindowRect(anchor_hwnd, &mut anchor_rect);
    let mut main_rect: RECT = std::mem::zeroed();
    GetWindowRect(main_hwnd, &mut main_rect);

    let work = get_monitor_work_area(anchor_hwnd);
    let main_w = (main_rect.right - main_rect.left).max(460);
    let main_h = (main_rect.bottom - main_rect.top).max(680);

    let (target_left, target_top) = {
        // Unify rule across all anchor windows (MiniPlayer & Visualizer):
        // Main window smoothly slides to the Left of the anchor (if fits within work area), else to the Right
        let mut left = anchor_rect.left - main_w;
        if left < work.left {
            left = anchor_rect.right;
            if left + main_w > work.right {
                left = (work.right - main_w).max(work.left);
            }
        }
        let top = anchor_rect.top.clamp(work.top, (work.bottom - main_h).max(work.top));
        (left, top)
    };

    let target_rect = RECT {
        left: target_left,
        top: target_top,
        right: target_left + main_w,
        bottom: target_top + main_h,
    };

    let start_x = main_rect.left;
    let start_y = main_rect.top;

    // Check if start position is on or near this monitor
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

    // Animate smoothly in a small background thread (150ms ease-out)
    let main_hwnd_usize = main_hwnd as usize;
    let anchor_rect_copy = anchor_rect;
    let target_rect_copy = target_rect;
    std::thread::spawn(move || {
        let hwnd = main_hwnd_usize as HWND;
        let steps = 12;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let ease = 1.0 - (1.0 - t).powi(3);
            let cur_x = (eff_start_x as f32 + (target_left - eff_start_x) as f32 * ease).round() as i32;
            let cur_y = (eff_start_y as f32 + (target_top - eff_start_y) as f32 * ease).round() as i32;
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    cur_x,
                    cur_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                target_left,
                target_top,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            // Establish magnetic docking relation
            set_dock_relation(anchor_kind, WindowKind::Main, &anchor_rect_copy, &target_rect_copy);
            bring_cluster_to_top(WindowKind::Main, hwnd);
            crate::tray::force_foreground(hwnd);
        }
    });
}

/// Checks if the Main Window is currently in a state ready to be magnetically docked.
/// Pure state evaluation:
/// 1. Main window HWND is valid and window exists.
/// 2. Not hidden to tray.
/// 3. Main window is currently on root page (Page::Main).
/// 4. Visualizer is not in fullscreen mode.
/// 5. Not minimized (iconic).
/// 6. Window is visible.
/// 7. Coordinates are within the monitor work area (not off-screen).
pub fn is_main_window_dock_ready() -> bool {
    let main_hwnd = get_window_hwnd(WindowKind::Main);
    if main_hwnd.is_null() || unsafe { IsWindow(main_hwnd) } == 0 {
        return false;
    }
    if crate::tray::is_window_hidden_to_tray() {
        return false;
    }
    if !crate::gui::is_main_on_root_page() {
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

    let is_topmost = crate::tray::IS_ALWAYS_ON_TOP.load(Ordering::SeqCst) || crate::visualizer::is_visualizer_fullscreen();

    // 1. Un-minimize main window if iconic
    if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 && IsIconic(main_hwnd) != 0 {
        ShowWindow(main_hwnd, SW_RESTORE);
    }

    // 2. Collect all active visible fleet windows in bottom-to-top order
    let fleet = [
        (WindowKind::Visualizer, vis_hwnd),
        (WindowKind::Equalizer, eq_hwnd),
        (WindowKind::MiniPlayer, mini_hwnd),
        (WindowKind::Main, main_hwnd),
    ];

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

        // Elevate window above third-party applications
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        if !is_topmost {
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
        crate::tray::ensure_window_on_screen(hwnd);
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
/// Stage 1: If main window is not dock-ready (e.g. on subpage, hidden, off-screen, or minimized),
///          restore it, switch to root page (Page::Main), and bring entire fleet to top.
/// Stage 2: If main window is already dock-ready, smoothly animate and dock it to the anchor window.
pub unsafe fn handle_home_summon_request(source_kind: WindowKind) {
    if is_main_window_dock_ready() {
        // Stage 2: Magnetically summon and dock main window to anchor window with animation!
        summon_and_dock_main_window_animated(source_kind);
    } else {
        // Stage 1: Restore, navigate to root page, and bring fleet to top
        crate::gui::request_navigate_to_main();
        crate::tray::bring_main_to_front();
        bring_fleet_to_top();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset_all() {
        with_dock_edges(|e| e.clear());
        with_dock_tree(|t| t.clear());
        if let Ok(mut guard) = LAST_KNOWN_RECTS.lock() {
            if let Some(ref mut m) = *guard {
                m.clear();
            }
        }
    }

    #[test]
    fn test_dock_side_and_bonded_pos() {
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
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
}
