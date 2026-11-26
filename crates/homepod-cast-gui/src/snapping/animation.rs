use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::*;
pub(crate) static IS_WORKSTATION_ANIMATING: AtomicBool = AtomicBool::new(false);
pub(crate) static IS_CLUSTER_CLOSING_ANIMATING: AtomicBool = AtomicBool::new(false);


pub fn is_workstation_animating() -> bool {
    IS_WORKSTATION_ANIMATING.load(Ordering::Relaxed)
}

pub fn is_cluster_closing_animating() -> bool {
    IS_CLUSTER_CLOSING_ANIMATING.load(Ordering::Relaxed)
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


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterState {
    Normal,
    Minimizing(std::time::Instant),
    Minimized,
    Restoring(std::time::Instant),
}

pub(crate) static CLUSTER_FSM: Mutex<Option<HashMap<WindowKind, ClusterState>>> = Mutex::new(None);

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
    let members = get_cluster_members(root);
    crate::window_manager::WindowManager::with_manager(|mgr| {
        for m in members {
            mgr.mark_window_minimized(m);
        }
    });
}

pub fn finish_cluster_restore(root: WindowKind) {
    set_cluster_state(root, ClusterState::Normal);
    let members = get_cluster_members(root);
    crate::window_manager::WindowManager::with_manager(|mgr| {
        for m in members {
            mgr.mark_window_restored(m);
        }
    });
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

pub(crate) const GWLP_HWNDPARENT: i32 = -8;

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

pub(crate) unsafe fn delete_taskbar_tab(hwnd: HWND) {
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

pub(crate) unsafe fn add_taskbar_tab(hwnd: HWND) {
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
        let normal_r = get_window_normal_rect(kind);
        update_known_rect(kind, normal_r);
        set_cluster_state(kind, ClusterState::Minimized);
        crate::genie_effect::disable_window_dwm_transitions(hwnd);
        let (hbmp, rect) = crate::genie_effect::capture_window_snapshot(hwnd);
        if hbmp != 0 {
            crate::genie_effect::play_genie_s_curve_effect_with_finish(
                hbmp,
                hwnd as usize,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                crate::genie_effect::GenieTarget::Taskbar,
                move || {
                    finish_cluster_minimize(kind);
                },
            );
        } else {
            ShowWindow(hwnd, SW_MINIMIZE);
            finish_cluster_minimize(kind);
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
                    let nr = get_window_normal_rect(root);
                    SetWindowPos(
                        h,
                        HWND_NOTOPMOST,
                        nr.left,
                        nr.top,
                        nr.right - nr.left,
                        nr.bottom - nr.top,
                        SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    );
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
            crate::window_manager::WindowManager::with_manager(|mgr| {
                mgr.on_workstation_aligned(
                    main_r,
                    if has_mini { Some(mini_r) } else { None },
                    if has_eq { Some(eq_r) } else { None },
                    if has_vis { Some(vis_r) } else { None },
                );
            });
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


