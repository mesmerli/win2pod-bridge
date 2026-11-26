use std::collections::HashMap;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, RECT};

use crate::snapping::WindowKind;
use crate::window_entity::{ManagedWindow, MinimizeTarget, WindowLifecycleState, WindowRect};

static WINDOW_MANAGER: Mutex<Option<WindowManager>> = Mutex::new(None);

pub struct WindowManager {
    windows: HashMap<WindowKind, ManagedWindow>,
}

impl WindowManager {
    pub fn new() -> Self {
        let mut windows = HashMap::new();
        windows.insert(WindowKind::Main, ManagedWindow::new(WindowKind::Main));
        windows.insert(WindowKind::MiniPlayer, ManagedWindow::new(WindowKind::MiniPlayer));
        windows.insert(WindowKind::Equalizer, ManagedWindow::new(WindowKind::Equalizer));
        windows.insert(WindowKind::Visualizer, ManagedWindow::new(WindowKind::Visualizer));
        Self { windows }
    }

    /// 取得全域單例管理者（延遲初始化）
    pub fn global() -> &'static Mutex<Option<WindowManager>> {
        &WINDOW_MANAGER
    }

    /// 安全調用 WindowManager
    pub fn with_manager<F, R>(f: F) -> R
    where
        F: FnOnce(&mut WindowManager) -> R,
    {
        let mut guard = WINDOW_MANAGER.lock().unwrap_or_else(|e| e.into_inner());
        let mgr = guard.get_or_insert_with(WindowManager::new);
        f(mgr)
    }

    /// 註冊視窗 HWND
    pub fn register_window(&mut self, kind: WindowKind, hwnd: HWND) {
        let entry = self.windows.entry(kind).or_insert_with(|| ManagedWindow::new(kind));
        entry.set_hwnd(hwnd);
    }

    /// 註銷視窗（銷毀關閉時）
    pub fn unregister_window(&mut self, kind: WindowKind) {
        if let Some(w) = self.windows.get_mut(&kind) {
            w.clear_hwnd();
        }
    }

    /// 查詢視窗實體
    pub fn get_window(&self, kind: WindowKind) -> Option<&ManagedWindow> {
        self.windows.get(&kind)
    }

    /// 變更視窗實體
    pub fn get_window_mut(&mut self, kind: WindowKind) -> Option<&mut ManagedWindow> {
        self.windows.get_mut(&kind)
    }

    /// 查詢視窗目前狀態
    pub fn get_window_state(&self, kind: WindowKind) -> WindowLifecycleState {
        self.windows
            .get(&kind)
            .map(|w| w.state.clone())
            .unwrap_or(WindowLifecycleState::Uninitialized)
    }

    /// 判斷視窗是否處於最小化
    pub fn is_window_minimized(&self, kind: WindowKind) -> bool {
        self.windows
            .get(&kind)
            .map(|w| w.state.is_minimized())
            .unwrap_or(false)
    }

    /// 使用者手動邊框縮放回報（更新權威尺寸並記憶使用者自訂長寬）
    pub fn on_user_resize(&mut self, kind: WindowKind, new_rect: RECT) {
        if let Some(w) = self.windows.get_mut(&kind) {
            w.on_user_resize(WindowRect::from(new_rect));
        }
    }

    /// 使用者手動移動視窗回報
    pub fn on_user_move(&mut self, kind: WindowKind, new_rect: RECT) {
        if let Some(w) = self.windows.get_mut(&kind) {
            w.on_user_move(WindowRect::from(new_rect));
        }
    }

    /// 僅標記視窗最小化（由外部如神燈動畫完成時呼叫，保持狀態機同步）
    pub fn mark_window_minimized(&mut self, kind: WindowKind) {
        if let Some(w) = self.windows.get_mut(&kind) {
            w.mark_minimized(MinimizeTarget::Taskbar);
        }
    }

    /// 僅標記視窗已還原
    pub fn mark_window_restored(&mut self, kind: WindowKind) {
        if let Some(w) = self.windows.get_mut(&kind) {
            w.mark_restored();
        }
    }

    /// 工作站排列完成同步各視窗之狀態與幾何
    pub fn on_workstation_aligned(
        &mut self,
        main_r: RECT,
        mini_r: Option<RECT>,
        eq_r: Option<RECT>,
        vis_r: Option<RECT>,
    ) {
        if let Some(w) = self.windows.get_mut(&WindowKind::Main) {
            w.canonical_rect = WindowRect::from(main_r);
            w.state = WindowLifecycleState::WorkstationDocked {
                root: WindowKind::Main,
                rect: WindowRect::from(main_r),
            };
        }
        if let (Some(r), Some(w)) = (mini_r, self.windows.get_mut(&WindowKind::MiniPlayer)) {
            w.canonical_rect = WindowRect::from(r);
            w.state = WindowLifecycleState::WorkstationDocked {
                root: WindowKind::Main,
                rect: WindowRect::from(r),
            };
        }
        if let (Some(r), Some(w)) = (eq_r, self.windows.get_mut(&WindowKind::Equalizer)) {
            w.canonical_rect = WindowRect::from(r);
            w.state = WindowLifecycleState::WorkstationDocked {
                root: WindowKind::Main,
                rect: WindowRect::from(r),
            };
        }
        if let (Some(r), Some(w)) = (vis_r, self.windows.get_mut(&WindowKind::Visualizer)) {
            w.canonical_rect = WindowRect::from(r);
            w.state = WindowLifecycleState::WorkstationDocked {
                root: WindowKind::Main,
                rect: WindowRect::from(r),
            };
        }
    }

    /// 取得視覺視窗的權威出生與排版幾何（解決「忽矮忽長」的核心入口）
    /// 規則：高度統一動態錨定在主視窗實際高度，寬度若曾被使用者縮放過則尊重自訂寬度，否則採標準 720px。
    pub fn get_visualizer_spawn_rect(&self) -> (i32, i32, i32, i32) {
        let main_hwnd = self.windows.get(&WindowKind::Main).map(|w| w.hwnd()).unwrap_or(std::ptr::null_mut());
        let main_r = if !main_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(main_hwnd) != 0 } {
            let mut r: RECT = unsafe { std::mem::zeroed() };
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(main_hwnd, &mut r); }
            r
        } else {
            RECT { left: 100, top: 100, right: 560, bottom: 780 }
        };

        let main_h = (main_r.bottom - main_r.top).max(500);
        let (def_w, def_h) = if let Some(vis) = self.windows.get(&WindowKind::Visualizer) {
            let w = vis.user_custom_size.map(|(w, _)| w).unwrap_or(720);
            (w, main_h)
        } else {
            (720, main_h)
        };

        // 預設誕生在主視窗右緣貼合處
        (def_w, def_h, main_r.right, main_r.top)
    }

    /// 全螢幕狀態切換事件
    pub fn on_visualizer_fullscreen_toggled(&mut self, is_fullscreen: bool) {
        if is_fullscreen {
            if let Some(vis) = self.windows.get_mut(&WindowKind::Visualizer) {
                vis.enter_fullscreen();
            }
        } else {
            // 退出全螢幕：純粹還原視覺特效視窗的幾何與樣式，其餘所有視窗保持其真實螢幕位置與生命週期
            if let Some(vis) = self.windows.get_mut(&WindowKind::Visualizer) {
                vis.exit_fullscreen();
            }
        }
    }

    /// 全螢幕 HUD 懸浮層開關事件同步
    pub fn on_hud_toggled(&mut self, _is_shown: bool) {
        // 全螢幕 HUD 顯隱純粹為圖層層級 (Topmost / 沉降) 與釘選狀態管理，
        // 各視窗自主維持其真實常態生命週期（Floating 或 WorkstationDocked），
        // 狀態機無需介入偽裝或備份，徹底杜絕幽靈位移與座標倒退。
    }
}
