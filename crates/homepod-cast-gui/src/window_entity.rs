//! Window Entity (ManagedWindow)
//!
//! 封裝每個受控視窗的實體屬性、幾何邊界限制（Hardware Clamp）以及內生狀態機（WindowLifecycleState）。

use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SetWindowPlacement, SetWindowPos, ShowWindow, HWND_NOTOPMOST, HWND_TOPMOST,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_MINIMIZE,
    WINDOWPLACEMENT,
};

use crate::snapping::WindowKind;

/// 視窗幾何矩形封裝（具備完整的 PartialEq, Eq, Clone, Copy, Debug 特徵）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WindowRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn to_rect(&self) -> RECT {
        RECT {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

impl From<RECT> for WindowRect {
    fn from(r: RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

impl From<WindowRect> for RECT {
    fn from(w: WindowRect) -> Self {
        w.to_rect()
    }
}

/// 最小化收納目標
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MinimizeTarget {
    #[default]
    Taskbar,
    Tray,
}

/// 視窗生命週期狀態機（Window Lifecycle State Machine）
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WindowLifecycleState {
    #[default]
    Uninitialized,
    /// 獨立浮動（自由拖曳、自由調整大小）
    Floating { rect: WindowRect },
    /// 工作站磁吸附屬狀態（依附於工作站 Root 節點）
    WorkstationDocked {
        root: WindowKind,
        rect: WindowRect,
    },
    /// 最小化收納中（Taskbar 或 Tray）
    Minimized {
        target: MinimizeTarget,
        restore_rect: WindowRect,
    },
    /// 沉浸全螢幕（目前為視覺效果視窗特有）
    Fullscreen { pre_fs_rect: WindowRect },
    /// 懸浮 HUD 模式（例如全螢幕視覺效果中的主視窗 HUD 控制器）
    FullscreenHud {
        prior_state: Box<WindowLifecycleState>,
        hud_rect: WindowRect,
    },
    /// 托盤常駐隱藏
    HiddenToTray { restore_rect: WindowRect },
}

impl WindowLifecycleState {
    pub fn is_minimized(&self) -> bool {
        matches!(self, WindowLifecycleState::Minimized { .. })
    }

    pub fn is_fullscreen(&self) -> bool {
        matches!(self, WindowLifecycleState::Fullscreen { .. })
    }

    pub fn is_hud(&self) -> bool {
        matches!(self, WindowLifecycleState::FullscreenHud { .. })
    }

    pub fn is_docked(&self) -> bool {
        matches!(self, WindowLifecycleState::WorkstationDocked { .. })
    }
}

/// 受控視窗實體（Managed Window）
///
/// 封裝視窗參數、邊界守護限制、狀態機以及底層 Win32 操作
pub struct ManagedWindow {
    pub kind: WindowKind,
    pub hwnd_val: isize,

    /// 權威常態幾何（Canonical Geometry）：絕對不受最小化負坐標或動畫干擾
    pub canonical_rect: WindowRect,

    /// 視窗硬體級最小邊界尺寸限制 (min_width, min_height)
    pub min_size: (i32, i32),

    /// 使用者手動自訂縮放的尺寸 (自訂寬, 自訂高)
    pub user_custom_size: Option<(i32, i32)>,

    /// 置頂圖層釘選狀態
    pub is_pinned: bool,

    /// 視窗內生狀態機
    pub state: WindowLifecycleState,

    /// 動畫播放中保護標誌：動畫期間發生的 Win32 幾何變化嚴禁污染 canonical_rect
    pub is_animating: AtomicBool,
}

impl ManagedWindow {
    pub fn new(kind: WindowKind) -> Self {
        let min_s = kind.min_size();
        let (min_w, min_h) = (min_s[0] as i32, min_s[1] as i32);

        let def_size = kind.default_size();
        let def_w = def_size[0] as i32;
        let def_h = def_size[1] as i32;

        let canonical_rect = WindowRect::new(100, 100, 100 + def_w, 100 + def_h);

        Self {
            kind,
            hwnd_val: 0,
            canonical_rect,
            min_size: (min_w, min_h),
            user_custom_size: None,
            is_pinned: false,
            state: WindowLifecycleState::Uninitialized,
            is_animating: AtomicBool::new(false),
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd_val as HWND
    }

    pub fn set_hwnd(&mut self, hwnd: HWND) {
        self.hwnd_val = hwnd as isize;
        if self.state == WindowLifecycleState::Uninitialized {
            let mut r: RECT = unsafe { std::mem::zeroed() };
            if !hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut r) != 0 } {
                let safe = self.clamp_rect(WindowRect::from(r));
                self.canonical_rect = safe;
                self.state = WindowLifecycleState::Floating { rect: safe };
            } else {
                self.state = WindowLifecycleState::Floating {
                    rect: self.canonical_rect,
                };
            }
        }
    }

    pub fn clear_hwnd(&mut self) {
        self.hwnd_val = 0;
        self.state = WindowLifecycleState::Uninitialized;
    }

    pub fn is_alive(&self) -> bool {
        if self.hwnd_val == 0 {
            return false;
        }
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(self.hwnd()) != 0 }
    }

    /// 幾何邊界保護（Hardware Clamp）：保證寬高絕對不小於 min_size
    pub fn clamp_rect(&self, r: WindowRect) -> WindowRect {
        let w = (r.right - r.left).max(self.min_size.0);
        let h = (r.bottom - r.top).max(self.min_size.1);
        WindowRect {
            left: r.left,
            top: r.top,
            right: r.left + w,
            bottom: r.top + h,
        }
    }

    /// 使用者手動拖曳邊框伸縮完成（由 WM_EXITSIZEMOVE 觸發）
    pub fn on_user_resize(&mut self, new_rect: WindowRect) {
        if self.is_animating.load(Ordering::Relaxed) {
            return;
        }
        let safe = self.clamp_rect(new_rect);
        self.canonical_rect = safe;
        let w = safe.right - safe.left;
        let h = safe.bottom - safe.top;
        self.user_custom_size = Some((w, h));
        match &mut self.state {
            WindowLifecycleState::Floating { rect } => *rect = safe,
            WindowLifecycleState::WorkstationDocked { rect, .. } => *rect = safe,
            WindowLifecycleState::FullscreenHud { prior_state, hud_rect } => {
                *hud_rect = safe;
                match prior_state.as_mut() {
                    WindowLifecycleState::Floating { rect } => *rect = safe,
                    WindowLifecycleState::WorkstationDocked { rect, .. } => *rect = safe,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// 使用者手動拖曳位置完成（由 WM_EXITSIZEMOVE 觸發）
    pub fn on_user_move(&mut self, new_rect: WindowRect) {
        if self.is_animating.load(Ordering::Relaxed) {
            return;
        }
        let safe = self.clamp_rect(new_rect);
        self.canonical_rect = safe;
        match &mut self.state {
            WindowLifecycleState::Floating { rect } => *rect = safe,
            WindowLifecycleState::WorkstationDocked { rect, .. } => *rect = safe,
            WindowLifecycleState::FullscreenHud { prior_state, hud_rect } => {
                *hud_rect = safe;
                match prior_state.as_mut() {
                    WindowLifecycleState::Floating { rect } => *rect = safe,
                    WindowLifecycleState::WorkstationDocked { rect, .. } => *rect = safe,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// 僅標記最小化狀態（不呼叫底層 Win32 API，避免打架與時序競態）
    pub fn mark_minimized(&mut self, target: MinimizeTarget) {
        if self.state.is_minimized() {
            return;
        }
        let pre_min_rect = self.canonical_rect;
        self.state = WindowLifecycleState::Minimized {
            target,
            restore_rect: pre_min_rect,
        };
    }

    /// 僅標記還原狀態
    pub fn mark_restored(&mut self) {
        if let WindowLifecycleState::Minimized { restore_rect, .. } = self.state {
            self.state = WindowLifecycleState::Floating { rect: restore_rect };
        }
    }

    /// 最小化此視窗：封裝狀態遷移與底層 Win32 最小化動作
    pub fn minimize(&mut self, target: MinimizeTarget) {
        if !self.is_alive() || self.state.is_minimized() {
            return;
        }

        let pre_min_rect = self.canonical_rect;
        self.state = WindowLifecycleState::Minimized {
            target,
            restore_rect: pre_min_rect,
        };

        let hwnd = self.hwnd();
        unsafe {
            if target == MinimizeTarget::Taskbar {
                ShowWindow(hwnd, SW_MINIMIZE);
                let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
                wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
                if GetWindowPlacement(hwnd, &mut wp) != 0 {
                    wp.rcNormalPosition = pre_min_rect.to_rect();
                    SetWindowPlacement(hwnd, &wp);
                }
            } else {
                // Tray: 隱藏離屏
                SetWindowPos(
                    hwnd,
                    HWND_NOTOPMOST,
                    -32000,
                    -32000,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
        }
    }

    /// 依據指定矩形呈現視窗（唯一的 Win32 SetWindowPos 出口）
    pub fn apply_geometry(&self, rect: WindowRect, activate: bool) {
        if !self.is_alive() {
            return;
        }
        let safe = self.clamp_rect(rect);
        let z_order = if self.is_pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
        let mut flags = SWP_SHOWWINDOW;
        if !activate {
            flags |= SWP_NOACTIVATE;
        }
        unsafe {
            SetWindowPos(
                self.hwnd(),
                z_order,
                safe.left,
                safe.top,
                safe.right - safe.left,
                safe.bottom - safe.top,
                flags,
            );
        }
    }

    /// 設定視窗置頂層級
    pub fn set_pinned(&mut self, pinned: bool) {
        self.is_pinned = pinned;
        if self.is_alive() && !self.state.is_minimized() {
            let target_z = if pinned { HWND_TOPMOST } else { HWND_NOTOPMOST };
            unsafe {
                SetWindowPos(
                    self.hwnd(),
                    target_z,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    /// 進入懸浮 HUD 模式（例如視覺效果全螢幕時喚起的主視窗 HUD）
    pub fn enter_hud(&mut self, hud_rect: WindowRect) {
        if self.state.is_hud() {
            return;
        }
        let safe = self.clamp_rect(hud_rect);
        self.canonical_rect = safe;
        let prior = Box::new(self.state.clone());
        self.state = WindowLifecycleState::FullscreenHud {
            prior_state: prior,
            hud_rect: safe,
        };
    }

    /// 退出 HUD 模式
    pub fn exit_hud(&mut self) {
        if let WindowLifecycleState::FullscreenHud { prior_state, .. } = &self.state {
            let mut p = *prior_state.clone();
            // 退出 HUD 模式時，最新幾何絕對繼承自當前的 canonical_rect，絕不倒退回過去舊坐標
            match &mut p {
                WindowLifecycleState::Floating { rect } => *rect = self.canonical_rect,
                WindowLifecycleState::WorkstationDocked { rect, .. } => *rect = self.canonical_rect,
                _ => {}
            }
            self.state = p;
            // 關鍵防護：退出 HUD 或退出全螢幕時，絕不調用 apply_geometry 強制搬移視窗或觸發 SWP_SHOWWINDOW。
            // 視窗已在使用者放置的位置（canonical_rect），尊重其當前物理螢幕位置。
        }
    }

    /// 進入全螢幕
    pub fn enter_fullscreen(&mut self) {
        if self.state.is_fullscreen() {
            return;
        }
        self.state = WindowLifecycleState::Fullscreen {
            pre_fs_rect: self.canonical_rect,
        };
    }

    /// 退出全螢幕
    pub fn exit_fullscreen(&mut self) {
        if let WindowLifecycleState::Fullscreen { pre_fs_rect } = self.state {
            self.state = WindowLifecycleState::Floating { rect: pre_fs_rect };
            self.apply_geometry(pre_fs_rect, false);
        }
    }
}
