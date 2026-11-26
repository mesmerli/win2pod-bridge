use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::*;
pub static SNAPPING_ENABLED: AtomicBool = AtomicBool::new(true);
pub static SNAP_DISTANCE: AtomicI32 = AtomicI32::new(12);

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


