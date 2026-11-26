use std::sync::atomic::Ordering;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::*;
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
            let mut r: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut r) != 0 {
                crate::window_manager::WindowManager::with_manager(|mgr| {
                    mgr.on_user_resize(kind, r);
                    mgr.on_user_move(kind, r);
                });
            }
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
                    if !is_kind_cluster_minimizing(kind) && !is_kind_cluster_restoring(kind) {
                        if IsIconic(hwnd) != 0 || crate::tray::is_window_hidden_to_tray() {
                            return DefSubclassProc(hwnd, msg, wparam, lparam);
                        }
                        let root = get_component_root(kind);
                        let was_collapsed = can_start_cluster_restore(root);
                        if was_collapsed {
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


