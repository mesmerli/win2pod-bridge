//! Window Snapping (Magnetic Windows) & Group Movement
//! Inspired by Winamp: automatically snaps windows to screen edges / taskbar
//! and to each other when close, with linked movement when bonded.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
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

/// Dock side relationship of Visualizer relative to Main Window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    None,
    Right,  // Visualizer is docked to the right edge of Main Window
    Left,   // Visualizer is docked to the left edge of Main Window
    Top,    // Visualizer is docked above Main Window
    Bottom, // Visualizer is docked below Main Window
}

/// Dock bond state tracking relative offset and docked edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockBond {
    pub is_bonded: bool,
    pub side: DockSide,
    pub offset_x: i32,
    pub offset_y: i32,
}

static DOCK_BOND: Mutex<DockBond> = Mutex::new(DockBond {
    is_bonded: false,
    side: DockSide::None,
    offset_x: 0,
    offset_y: 0,
});

/// Determine which edge the Visualizer is docked to relative to Main Window.
pub fn determine_dock_side(vis_rect: &RECT, main_rect: &RECT, tolerance: i32) -> DockSide {
    let d_right = (vis_rect.left - main_rect.right).abs();
    let d_left = (vis_rect.right - main_rect.left).abs();
    let d_bottom = (vis_rect.top - main_rect.bottom).abs();
    let d_top = (vis_rect.bottom - main_rect.top).abs();

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

/// Calculate bonded Visualizer position based on docked side and Main Window bounds.
pub fn calc_bonded_visualizer_pos(bond: &DockBond, vis_rect: &RECT, main_rect: &RECT) -> (i32, i32) {
    let vis_w = vis_rect.right - vis_rect.left;
    let vis_h = vis_rect.bottom - vis_rect.top;
    match bond.side {
        DockSide::Right => (main_rect.right, main_rect.top + bond.offset_y),
        DockSide::Left => (main_rect.left - vis_w, main_rect.top + bond.offset_y),
        DockSide::Bottom => (main_rect.left + bond.offset_x, main_rect.bottom),
        DockSide::Top => (main_rect.left + bond.offset_x, main_rect.top - vis_h),
        DockSide::None => (main_rect.left + bond.offset_x, main_rect.top + bond.offset_y),
    }
}

/// Calculate bonded Main Window position based on docked side and Visualizer bounds.
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

/// Snap moving rectangle to monitor work area edges (left, top, right, bottom).
pub fn snap_rect_to_work_area(r: &mut RECT, work: &RECT, dist: i32) {
    let w = r.right - r.left;
    let h = r.bottom - r.top;

    // Horizontal snap: Left edge or Right edge
    if (r.left - work.left).abs() <= dist {
        r.left = work.left;
        r.right = r.left + w;
    } else if (r.right - work.right).abs() <= dist {
        r.right = work.right;
        r.left = r.right - w;
    }

    // Vertical snap: Top edge or Bottom edge
    if (r.top - work.top).abs() <= dist {
        r.top = work.top;
        r.bottom = r.top + h;
    } else if (r.bottom - work.bottom).abs() <= dist {
        r.bottom = work.bottom;
        r.top = r.bottom - h;
    }
}

/// Snap moving rectangle (A) to stationary target rectangle (B).
/// Supports edge-to-edge docking and corner alignment.
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
            // A's left edge touches B's right edge
            r_a.left = r_b.right;
            r_a.right = r_a.left + w_a;
            snapped_x = true;
        } else if (r_a.right - r_b.left).abs() <= dist {
            // A's right edge touches B's left edge
            r_a.right = r_b.left;
            r_a.left = r_a.right - w_a;
            snapped_x = true;
        }

        if snapped_x {
            // Align corners without altering window height or width
            if (r_a.top - r_b.top).abs() <= dist * 2 {
                r_a.top = r_b.top;
                r_a.bottom = r_a.top + h_a;
            } else if (r_a.bottom - r_b.bottom).abs() <= dist * 2 {
                r_a.bottom = r_b.bottom;
                r_a.top = r_a.bottom - h_a;
            }
        }
    }

    // Vertical edge snap
    if h_overlap {
        if (r_a.top - r_b.bottom).abs() <= dist {
            // A's top edge touches B's bottom edge
            r_a.top = r_b.bottom;
            r_a.bottom = r_a.top + h_a;
            snapped_y = true;
        } else if (r_a.bottom - r_b.top).abs() <= dist {
            // A's bottom edge touches B's top edge
            r_a.bottom = r_b.top;
            r_a.top = r_a.bottom - h_a;
            snapped_y = true;
        }

        if snapped_y {
            // Align corners if near
            if (r_a.left - r_b.left).abs() <= dist {
                r_a.left = r_b.left;
                r_a.right = r_a.left + w_a;
            } else if (r_a.right - r_b.right).abs() <= dist {
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

pub fn set_dock_bond(vis_rect: &RECT, main_rect: &RECT) {
    let side = determine_dock_side(vis_rect, main_rect, 20);
    let offset_x = vis_rect.left - main_rect.left;
    let offset_y = vis_rect.top - main_rect.top;
    let mut became_bonded = false;
    if let Ok(mut bond) = DOCK_BOND.lock() {
        if !bond.is_bonded || bond.side != side || bond.offset_x != offset_x || bond.offset_y != offset_y {
            if !bond.is_bonded {
                became_bonded = true;
            }
            bond.is_bonded = true;
            bond.side = side;
            bond.offset_x = offset_x;
            bond.offset_y = offset_y;
        }
    }
    if became_bonded {
        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
        if !vis_hwnd.is_null() {
            unsafe {
                crate::visualizer::update_visualizer_dock_buttons(vis_hwnd, true);
            }
        }
    }
}

pub fn break_dock_bond() {
    let mut became_unbonded = false;
    if let Ok(mut bond) = DOCK_BOND.lock() {
        if bond.is_bonded {
            bond.is_bonded = false;
            bond.side = DockSide::None;
            became_unbonded = true;
        }
    }
    if became_unbonded {
        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
        if !vis_hwnd.is_null() {
            unsafe {
                crate::visualizer::update_visualizer_dock_buttons(vis_hwnd, false);
            }
        }
    }
}

pub fn get_dock_bond() -> DockBond {
    DOCK_BOND.lock().map(|b| *b).unwrap_or(DockBond {
        is_bonded: false,
        side: DockSide::None,
        offset_x: 0,
        offset_y: 0,
    })
}

/// Installs a Win32 window subclass on the Main Window to handle snapping and linked movement.
pub unsafe fn install_main_window_subclass(hwnd: HWND) {
    SetWindowSubclass(hwnd, Some(main_window_subclass_proc), MAIN_WINDOW_SUBCLASS_ID, 0);
}

unsafe extern "system" fn main_window_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uid_subclass: usize,
    _ref_data: usize,
) -> LRESULT {
    // When main window is hidden in tray (e.g. moved off-screen to -32000), ignore all snapping
    // and movement events so we never accidentally pull or restore docked windows.
    if crate::tray::is_window_hidden_to_tray() {
        if msg == WM_NCDESTROY {
            RemoveWindowSubclass(hwnd, Some(main_window_subclass_proc), MAIN_WINDOW_SUBCLASS_ID);
        }
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_MOVING => {
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                let rect_ptr = lparam as *mut RECT;
                if !rect_ptr.is_null() {
                    let dist = SNAP_DISTANCE.load(Ordering::Relaxed);
                    let work = get_monitor_work_area(hwnd);
                    snap_rect_to_work_area(&mut *rect_ptr, &work, dist);

                    // Check if Visualizer window is open and active
                    let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
                    if !vis_hwnd.is_null()
                        && IsWindow(vis_hwnd) != 0
                        && IsWindowVisible(vis_hwnd) != 0
                        && !crate::visualizer::is_visualizer_fullscreen()
                    {
                        let bond = get_dock_bond();
                        if bond.is_bonded {
                            // Winamp linked movement: drag attached Visualizer together!
                            let mut vis_rect: RECT = std::mem::zeroed();
                            GetWindowRect(vis_hwnd, &mut vis_rect);
                            let (new_vis_x, new_vis_y) = calc_bonded_visualizer_pos(&bond, &vis_rect, &*rect_ptr);
                            SetWindowPos(
                                vis_hwnd,
                                std::ptr::null_mut(),
                                new_vis_x,
                                new_vis_y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                            );
                        } else {
                            // Snap Main Window to Visualizer if dragged close
                            let mut vis_rect: RECT = std::mem::zeroed();
                            GetWindowRect(vis_hwnd, &mut vis_rect);
                            if snap_rect_to_rect(&mut *rect_ptr, &vis_rect, dist) {
                                set_dock_bond(&vis_rect, &*rect_ptr);
                            }
                        }
                    }
                    return 1;
                }
            }
        }
        WM_WINDOWPOSCHANGED => {
            if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                let wp = lparam as *const WINDOWPOS;
                if !wp.is_null() && ((*wp).flags & SWP_NOSIZE == 0) {
                    let bond = get_dock_bond();
                    if bond.is_bonded {
                        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
                        if !vis_hwnd.is_null()
                            && IsWindow(vis_hwnd) != 0
                            && IsWindowVisible(vis_hwnd) != 0
                            && !crate::visualizer::is_visualizer_fullscreen()
                        {
                            let mut main_rect: RECT = std::mem::zeroed();
                            GetWindowRect(hwnd, &mut main_rect);
                            let mut vis_rect: RECT = std::mem::zeroed();
                            GetWindowRect(vis_hwnd, &mut vis_rect);
                            let (new_vis_x, new_vis_y) = calc_bonded_visualizer_pos(&bond, &vis_rect, &main_rect);
                            if vis_rect.left != new_vis_x || vis_rect.top != new_vis_y {
                                SetWindowPos(
                                    vis_hwnd,
                                    std::ptr::null_mut(),
                                    new_vis_x,
                                    new_vis_y,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                                );
                            }
                        }
                    }
                }
            }
        }
        WM_EXITSIZEMOVE => {
            // Verify final touch status on release
            let mut main_rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut main_rect);
            let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
            if !vis_hwnd.is_null()
                && IsWindow(vis_hwnd) != 0
                && IsWindowVisible(vis_hwnd) != 0
                && !crate::visualizer::is_visualizer_fullscreen()
            {
                let mut vis_rect: RECT = std::mem::zeroed();
                GetWindowRect(vis_hwnd, &mut vis_rect);
                if are_rects_docked(&main_rect, &vis_rect, 4) {
                    set_dock_bond(&vis_rect, &main_rect);
                }
            }
        }
        WM_SIZE => {
            let resize_type = wparam as u32;
            if resize_type == SIZE_MINIMIZED {
                if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                    let bond = get_dock_bond();
                    if bond.is_bonded {
                        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
                        if !vis_hwnd.is_null()
                            && IsWindow(vis_hwnd) != 0
                            && !crate::visualizer::is_visualizer_fullscreen()
                        {
                            // Minimize visualizer window together with main window
                            ShowWindow(vis_hwnd, SW_MINIMIZE);
                        }
                    }
                }
            } else if resize_type == SIZE_RESTORED {
                unsafe {
                    crate::tray::ensure_window_on_screen(hwnd);
                }
                if SNAPPING_ENABLED.load(Ordering::Relaxed) {
                    let bond = get_dock_bond();
                    if bond.is_bonded {
                        let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
                        if !vis_hwnd.is_null()
                            && IsWindow(vis_hwnd) != 0
                            && !crate::visualizer::is_visualizer_fullscreen()
                        {
                            let mut main_rect: RECT = std::mem::zeroed();
                            GetWindowRect(hwnd, &mut main_rect);
                            let mut vis_rect: RECT = std::mem::zeroed();
                            GetWindowRect(vis_hwnd, &mut vis_rect);
                            // Restore visualizer window if minimized, and restore position next to main window
                            if IsIconic(vis_hwnd) != 0 {
                                ShowWindow(vis_hwnd, SW_SHOWNOACTIVATE);
                            }
                            let (new_vis_x, new_vis_y) = calc_bonded_visualizer_pos(&bond, &vis_rect, &main_rect);
                            SetWindowPos(
                                vis_hwnd,
                                hwnd,
                                new_vis_x,
                                new_vis_y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOACTIVATE,
                            );
                        }
                    }
                }
            }
        }
        WM_ACTIVATE => {
            let active_type = (wparam & 0xFFFF) as u32;
            if active_type != WA_INACTIVE as u32 && SNAPPING_ENABLED.load(Ordering::Relaxed) {
                let bond = get_dock_bond();
                if bond.is_bonded {
                    let vis_hwnd = crate::visualizer::get_visualizer_hwnd();
                    if !vis_hwnd.is_null()
                        && IsWindow(vis_hwnd) != 0
                        && !crate::visualizer::is_visualizer_fullscreen()
                    {
                        if IsIconic(vis_hwnd) != 0 {
                            ShowWindow(vis_hwnd, SW_SHOWNOACTIVATE);
                        }
                        let mut main_rect: RECT = std::mem::zeroed();
                        GetWindowRect(hwnd, &mut main_rect);
                        let mut vis_rect: RECT = std::mem::zeroed();
                        GetWindowRect(vis_hwnd, &mut vis_rect);
                        let (new_vis_x, new_vis_y) = calc_bonded_visualizer_pos(&bond, &vis_rect, &main_rect);
                        // Bring Visualizer alongside Main Window so other apps don't sandwich between them
                        SetWindowPos(
                            vis_hwnd,
                            hwnd,
                            new_vis_x,
                            new_vis_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
            }
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(main_window_subclass_proc), MAIN_WINDOW_SUBCLASS_ID);
        }
        _ => {}
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}
