use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::*;

/// Canonical representation of an undirected edge between two windows
pub fn canonical_edge(a: WindowKind, b: WindowKind) -> (WindowKind, WindowKind) {
    if (a as u32) < (b as u32) {
        (a, b)
    } else {
        (b, a)
    }
}

/// Pure Logical Data Structure for multi-window snapping topology and dock tree hierarchy.
/// Contains no Win32 calls, no Mutex locks inside, and is 100% deterministically unit-testable.
#[derive(Clone, Default)]
pub struct ClusterTopology {
    pub edges: HashSet<(WindowKind, WindowKind)>,
    pub tree: HashMap<WindowKind, DockBond>,
    pub known_rects: HashMap<WindowKind, RECT>,
}

impl std::fmt::Debug for ClusterTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterTopology")
            .field("edges", &self.edges)
            .field("tree", &self.tree)
            .finish()
    }
}

impl ClusterTopology {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_edge(&mut self, a: WindowKind, b: WindowKind) {
        self.edges.insert(canonical_edge(a, b));
    }

    pub fn remove_edge(&mut self, a: WindowKind, b: WindowKind) {
        self.edges.remove(&canonical_edge(a, b));
    }

    pub fn remove_all_edges_for(&mut self, kind: WindowKind) {
        self.edges.retain(|&(a, b)| a != kind && b != kind);
    }

    pub fn update_known_rect(&mut self, kind: WindowKind, rect: RECT) {
        self.known_rects.insert(kind, rect);
    }

    pub fn get_known_rect(&self, kind: WindowKind) -> Option<RECT> {
        self.known_rects.get(&kind).copied()
    }

    pub fn clear_known_rects(&mut self) {
        self.known_rects.clear();
    }

    pub fn is_in_cluster(&self, kind: WindowKind) -> bool {
        self.edges.iter().any(|&(a, b)| a == kind || b == kind)
    }

    pub fn get_cluster_members(&self, kind: WindowKind) -> Vec<WindowKind> {
        let mut members = HashSet::new();
        let mut queue = VecDeque::new();
        members.insert(kind);
        queue.push_back(kind);
        while let Some(curr) = queue.pop_front() {
            for &(a, b) in &self.edges {
                let neighbor = if a == curr {
                    Some(b)
                } else if b == curr {
                    Some(a)
                } else {
                    None
                };
                if let Some(n) = neighbor {
                    if members.insert(n) {
                        queue.push_back(n);
                    }
                }
            }
        }
        let mut list: Vec<_> = members.into_iter().collect();
        list.sort_by_key(|k| k.priority());
        list
    }

    pub fn get_root(&self, kind: WindowKind) -> WindowKind {
        let members = self.get_cluster_members(kind);
        members.into_iter().min_by_key(|k| k.priority()).unwrap_or(kind)
    }

    pub fn get_role(&self, kind: WindowKind) -> WindowRole {
        if !self.is_in_cluster(kind) {
            return WindowRole::Standalone;
        }
        if self.get_root(kind) == kind {
            WindowRole::ClusterMaster
        } else {
            WindowRole::ClusterSlave
        }
    }

    pub fn get_component_root(&self, kind: WindowKind) -> WindowKind {
        let mut curr = kind;
        let mut visited = HashSet::new();
        while let Some(bond) = self.tree.get(&curr) {
            if !bond.is_bonded || visited.contains(&curr) {
                break;
            }
            visited.insert(curr);
            curr = bond.parent;
        }
        curr
    }

    pub fn build_tree(
        &mut self,
        active_windows: &[WindowKind],
        get_rect: impl Fn(WindowKind) -> Option<RECT>,
    ) {
        let edge_slice: Vec<(WindowKind, WindowKind)> = self.edges.iter().copied().collect();
        self.tree = build_dock_tree_from_edges_and_rects(active_windows, &edge_slice, get_rect);
    }
}

pub(crate) static CLUSTER_TOPOLOGY: Mutex<Option<ClusterTopology>> = Mutex::new(None);

pub fn with_cluster_topology<R>(f: impl FnOnce(&mut ClusterTopology) -> R) -> R {
    let mut guard = CLUSTER_TOPOLOGY.lock().unwrap_or_else(|e| e.into_inner());
    let top = guard.get_or_insert_with(ClusterTopology::new);
    f(top)
}

pub fn with_dock_edges<R>(f: impl FnOnce(&mut HashSet<(WindowKind, WindowKind)>) -> R) -> R {
    with_cluster_topology(|top| f(&mut top.edges))
}

pub fn add_dock_edge(a: WindowKind, b: WindowKind) {
    with_cluster_topology(|top| {
        top.add_edge(a, b);
    });
}

pub fn remove_dock_edge(a: WindowKind, b: WindowKind) {
    with_cluster_topology(|top| {
        top.remove_edge(a, b);
    });
}

pub fn remove_all_edges_for(kind: WindowKind) {
    with_cluster_topology(|top| {
        top.remove_all_edges_for(kind);
    });
}

pub fn update_known_rect(kind: WindowKind, rect: RECT) {
    with_cluster_topology(|top| {
        top.update_known_rect(kind, rect);
    });
}

pub fn get_known_rect(kind: WindowKind) -> Option<RECT> {
    with_cluster_topology(|top| {
        top.get_known_rect(kind)
    })
}

pub fn get_all_known_rects() -> HashMap<WindowKind, RECT> {
    with_cluster_topology(|top| {
        top.known_rects.clone()
    })
}

pub fn clear_known_rects() {
    with_cluster_topology(|top| {
        top.clear_known_rects();
    });
}

pub fn with_dock_tree<R>(f: impl FnOnce(&mut HashMap<WindowKind, DockBond>) -> R) -> R {
    with_cluster_topology(|top| f(&mut top.tree))
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
        let map = get_all_known_rects();
        for (&k, &r) in &map {
            if !active.contains(&k) {
                rects.insert(k, r);
                active.push(k);
            }
        }
    }

    if active.is_empty() {
        with_dock_tree(|tree| {
            tree.clear();
        });
        return;
    }

    let tree_snapshot = with_dock_tree(|tree| tree.clone());
    let get_root_from_snapshot = |kind: WindowKind| -> WindowKind {
        let mut curr = kind;
        let mut visited = HashSet::new();
        while let Some(bond) = tree_snapshot.get(&curr) {
            if !bond.is_bonded || visited.contains(&curr) {
                break;
            }
            visited.insert(curr);
            curr = bond.parent;
        }
        curr
    };

    let edges: Vec<(WindowKind, WindowKind)> = with_dock_edges(|edge_set| {
        edge_set.retain(|&(a, b)| {
            if !active.contains(&a) || !active.contains(&b) {
                return false;
            }
            // 關鍵不變量保護：若視窗或其所屬叢集正處於收攏隱藏、最小化、或正在進行過渡動畫，
            // 嚴禁在重新拓撲時破壞該磁吸關係鏈！
            let is_in_collapsed_or_anim = |k: WindowKind| -> bool {
                let root = get_root_from_snapshot(k);
                matches!(get_cluster_state(root), ClusterState::Minimized)
                    || matches!(get_cluster_state(root), ClusterState::Minimizing(_))
                    || matches!(get_cluster_state(root), ClusterState::Restoring(_))
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
                let map = get_all_known_rects();
                for (&k, &r) in &map {
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


pub(crate) static BRINGING_CLUSTER_TO_TOP: AtomicBool = AtomicBool::new(false);


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


/// 根據當前 Dock Tree，以 `root` 的坐標為基準，遞迴計算出該群組所有附屬子視窗的目標螢幕停靠坐標。
pub fn compute_cluster_docked_targets(
    root: WindowKind,
    root_rect: &RECT,
) -> HashMap<WindowKind, RECT> {
    let mut targets = HashMap::new();
    targets.insert(root, *root_rect);

    let tree = with_dock_tree(|t| t.clone());
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
    let root_r = get_window_normal_rect(root);
    if !root_hwnd.is_null() && IsWindow(root_hwnd) != 0 && IsIconic(root_hwnd) != 0 {
        ShowWindow(root_hwnd, SW_RESTORE);
        SetWindowPos(
            root_hwnd,
            HWND_NOTOPMOST,
            root_r.left,
            root_r.top,
            root_r.right - root_r.left,
            root_r.bottom - root_r.top,
            SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );
    }
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
        if !effective_root_hwnd.is_null() && IsWindow(effective_root_hwnd) != 0 {
            if IsIconic(effective_root_hwnd) != 0 {
                ShowWindow(effective_root_hwnd, SW_RESTORE);
            }
            let nr = get_window_normal_rect(root);
            SetWindowPos(
                effective_root_hwnd,
                HWND_NOTOPMOST,
                nr.left,
                nr.top,
                nr.right - nr.left,
                nr.bottom - nr.top,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
            finish_cluster_restore(root);
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
    remove_all_edges_for(WindowKind::Visualizer);
    unsafe {
        rebuild_dock_tree();
    }
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

    // 迷你播放器與迷你等化器高度精準貼合主視窗的一半 (main_h / 2)，且保底不低於其 min_size
    if kind == WindowKind::MiniPlayer || kind == WindowKind::Equalizer {
        let main_hwnd = get_window_hwnd(WindowKind::Main);
        if !main_hwnd.is_null() && IsWindow(main_hwnd) != 0 {
            let mut main_r: RECT = std::mem::zeroed();
            GetWindowRect(main_hwnd, &mut main_r);
            let mh = (main_r.bottom - main_r.top).max(680);
            h = (mh / 2).max(kind.min_size()[1] as i32);
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
            let map = get_all_known_rects();
            for &m in &members {
                if m != WindowKind::Main {
                    if let Some(&r) = map.get(&m) {
                        member_rects.push((m, r));
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


