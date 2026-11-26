use std::sync::{Arc, RwLock};
use async_channel::Sender as AsyncSender;
use eframe::egui;

use crate::controller::{
    ControllerState, EqPreset, GuiCmd, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};
use crate::i18n::Language;


pub const AVAILABLE_HOTKEYS: &[(u32, u32)] = &[
    (MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, 0x48), // Ctrl + Alt + H
    (MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, 0x48), // Ctrl + Shift + H
    (MOD_ALT | MOD_NOREPEAT, 0x48), // Alt + H
    (MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, 0x41), // Ctrl + Alt + A
    (MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, 0x50), // Ctrl + Alt + P
    (MOD_WIN | MOD_ALT | MOD_NOREPEAT, 0x48), // Win + Alt + H
    (MOD_NOREPEAT, 0x78), // F9
    (MOD_NOREPEAT, 0x79), // F10
    (0, 0), // Disabled
];

pub fn hotkey_label(mods: u32, vk: u32, lang: Language) -> String {
    if mods == (MOD_CONTROL | MOD_ALT | MOD_NOREPEAT) && vk == 0x48 {
        format!("Ctrl + Alt + H {}", lang.hotkey_default_suffix())
    } else if mods == (MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT) && vk == 0x48 {
        "Ctrl + Shift + H".to_string()
    } else if mods == (MOD_ALT | MOD_NOREPEAT) && vk == 0x48 {
        "Alt + H".to_string()
    } else if mods == (MOD_CONTROL | MOD_ALT | MOD_NOREPEAT) && vk == 0x41 {
        "Ctrl + Alt + A".to_string()
    } else if mods == (MOD_CONTROL | MOD_ALT | MOD_NOREPEAT) && vk == 0x50 {
        "Ctrl + Alt + P".to_string()
    } else if mods == (MOD_WIN | MOD_ALT | MOD_NOREPEAT) && vk == 0x48 {
        "Win + Alt + H".to_string()
    } else if mods == MOD_NOREPEAT && vk == 0x78 {
        "F9".to_string()
    } else if mods == MOD_NOREPEAT && vk == 0x79 {
        "F10".to_string()
    } else if mods == 0 && vk == 0 {
        lang.hotkey_disabled().to_string()
    } else {
        lang.hotkey_custom().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Page {
    #[default]
    Main = 0,
    Settings = 1,
    About = 2,
    Equalizer = 3,
    Player = 4,
}

impl Page {
    /// Returns the standalone window kind corresponding to this embedded page (if it supports popout).
    pub fn popout_window_kind(&self) -> Option<crate::snapping::WindowKind> {
        match self {
            Page::Player => Some(crate::snapping::WindowKind::MiniPlayer),
            Page::Equalizer => Some(crate::snapping::WindowKind::Equalizer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDirection {
    Push,
    Pop,
}

#[derive(Debug, Clone, Copy)]
pub struct PageTransition {
    pub from: Page,
    pub to: Page,
    pub direction: TransitionDirection,
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackTransitionDirection {
    Next,
    Prev,
}

#[derive(Clone)]
pub struct TrackTransition {
    pub from_title: String,
    pub from_artist: String,
    pub from_album: String,
    pub from_art_texture: Option<egui::TextureHandle>,
    pub to_title: String,
    pub to_artist: String,
    pub to_album: String,
    pub to_art_texture: Option<egui::TextureHandle>,
    pub direction: TrackTransitionDirection,
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
}

pub static MAIN_CURRENT_PAGE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(Page::Main as u8);
pub static REQUEST_NAVIGATE_TO_MAIN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 判斷主視窗當前是否處於根首頁（Page::Main）
pub fn is_main_on_root_page() -> bool {
    MAIN_CURRENT_PAGE.load(std::sync::atomic::Ordering::SeqCst) == Page::Main as u8
}

/// 發送請求以將主視窗導航回根首頁（Page::Main）
pub fn request_navigate_to_main() {
    REQUEST_NAVIGATE_TO_MAIN.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Some(ctx) = get_egui_ctx() {
        ctx.request_repaint();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Audio,
    Receiver,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EqProfile {
    #[default]
    Normal,
    Receiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTheme {
    Green,
    Blue,
    Gold,
    Cyan,
}

#[derive(Debug, Clone)]
pub struct SpectrumVisualizer {
    pub current: [f32; 16],
    pub peaks: [f32; 16],
    pub peak_decay: [f32; 16],
}

impl Default for SpectrumVisualizer {
    fn default() -> Self {
        Self {
            current: [0.0; 16],
            peaks: [0.0; 16],
            peak_decay: [0.0; 16],
        }
    }
}

pub(crate) fn meter_color_at(t: f32) -> egui::Color32 {
    if t <= 0.65 {
        let f = (t / 0.65).clamp(0.0, 1.0);
        lerp_color(
            egui::Color32::from_rgb(34, 197, 94),   // Emerald Green (#22C55E)
            egui::Color32::from_rgb(234, 179, 8),   // Amber Yellow (#EAB308)
            f,
        )
    } else if t <= 0.88 {
        let f = ((t - 0.65) / (0.88 - 0.65)).clamp(0.0, 1.0);
        lerp_color(
            egui::Color32::from_rgb(234, 179, 8),   // Amber Yellow (#EAB308)
            egui::Color32::from_rgb(249, 115, 22),  // Orange (#F97316)
            f,
        )
    } else {
        let f = ((t - 0.88) / (1.0 - 0.88)).clamp(0.0, 1.0);
        lerp_color(
            egui::Color32::from_rgb(249, 115, 22),  // Orange (#F97316)
            egui::Color32::from_rgb(239, 68, 68),   // Coral Red (#EF4444)
            f,
        )
    }
}

pub(crate) fn lerp_color(c1: egui::Color32, c2: egui::Color32, t: f32) -> egui::Color32 {
    let r = c1.r() as f32 + (c2.r() as f32 - c1.r() as f32) * t;
    let g = c1.g() as f32 + (c2.g() as f32 - c1.g() as f32) * t;
    let b = c1.b() as f32 + (c2.b() as f32 - c1.b() as f32) * t;
    egui::Color32::from_rgb(r as u8, g as u8, b as u8)
}

pub struct FlyoutApp {
    pub cmd_tx: AsyncSender<GuiCmd>,
    pub shared_state: Arc<RwLock<ControllerState>>,
    pub state: ControllerState,
    pub volume_slider: f32,
    pub last_applied_height: f32,
    pub last_applied_width: f32,
    pub selected_device_ids: std::collections::HashSet<String>,
    pub is_dragging_volume: bool,
    pub current_page: Page,
    pub last_page: Page,
    pub page_history: Vec<Page>,
    pub page_transition: Option<PageTransition>,
    pub settings_tab: SettingsTab,
    pub meter_level: f32,
    pub last_meter_update: std::time::Instant,
    pub startup_time: std::time::Instant,
    pub startup_trimmed: bool,
    pub frame_count: u32,
    pub last_hidden: bool,
    pub spectrum_vis: SpectrumVisualizer,
    pub eq_profile: EqProfile,
    pub selected_preset: Option<EqPreset>,
    pub receiver_selected_preset: Option<EqPreset>,
    pub display_eq_gains: [f32; 10],
    pub display_receiver_eq_gains: [f32; 10],
    pub album_art_texture: Option<egui::TextureHandle>,
    pub last_album_art_bytes: Option<Vec<u8>>,
    pub receiver_artwork_texture: Option<egui::TextureHandle>,
    pub last_receiver_artwork_len: usize,
    pub balance: f32,
    pub is_mono: bool,
    pub seek_drag_secs: Option<f64>,
    pub is_always_on_top: bool,
    pub equalizer_window_open: bool,
    pub mini_player_window_open: bool,
    pub mini_player_needs_initial_dock: bool,
    pub equalizer_needs_initial_dock: bool,
    pub app_icon_texture: Option<egui::TextureHandle>,
    pub pending_modular_workstation_frames: u8,
    pub track_transition: Option<TrackTransition>,
    pub prev_track_title: String,
    pub prev_track_artist: String,
    pub prev_track_album: String,
    pub prev_album_art_texture: Option<egui::TextureHandle>,
    pub next_track_intent: Option<TrackTransitionDirection>,
    pub workstation_onboarding_dismissed: bool,
    pub mascot_pos: egui::Pos2,
    pub mascot_activity: u8,
    pub mascot_platform_idx: usize,
    pub mascot_dir_x: f32,
    pub mascot_dir_y: f32,
    pub mascot_vel_y: f32,
    pub mascot_action_timer: f32,
    pub mascot_squash: f32,
    pub mascot_bounce: f32,
    pub mascot_last_update: std::time::Instant,
    pub mascot_window: crate::snapping::WindowKind,
    pub mascot_spectrum_rect: Option<egui::Rect>,
    pub mascot_spectrum_privilege: bool,
    pub show_supporter_mascot_modal: bool,
    pub reset_about_scroll: bool,
}

pub(crate) static EGUI_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

pub fn register_egui_ctx(ctx: egui::Context) {
    let _ = EGUI_CTX.set(ctx);
}

pub fn get_egui_ctx() -> Option<egui::Context> {
    EGUI_CTX.get().cloned()
}

pub(crate) static PENDING_CLUSTER_CLOSE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub(crate) static PENDING_CLUSTER_CLOSE_FINISH: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn request_close_cluster(kind: crate::snapping::WindowKind) {
    PENDING_CLUSTER_CLOSE.store(kind as u8, std::sync::atomic::Ordering::SeqCst);
    if let Some(ctx) = get_egui_ctx() {
        ctx.request_repaint();
    }
}

/// Request a UI repaint, but only if the main window is currently visible.
/// When hidden in the system tray, we intentionally suppress repaints to prevent
/// waking up the render pipeline with no visible content.
pub fn request_ui_repaint() {
    if let Some(ctx) = EGUI_CTX.get() {
        if !crate::tray::is_window_hidden_to_tray() {
            ctx.request_repaint();
        }
    }
}


impl FlyoutApp {
    pub fn is_page_active(&self, page: Page) -> bool {
        if self.current_page == page {
            return true;
        }
        if let Some(kind) = page.popout_window_kind() {
            self.is_window_open(kind)
        } else {
            false
        }
    }

    /// 檢查等化器是否處於作用中（獨立視窗開啟，或主視窗位於等化器分頁）
    pub fn is_equalizer_active(&self) -> bool {
        self.is_page_active(Page::Equalizer)
    }

    /// 檢查播放器是否處於作用中（迷你播放器開啟，或主視窗位於播放器分頁）
    pub fn is_player_active(&self) -> bool {
        self.is_page_active(Page::Player)
    }

    /// 檢查特定視窗是否處於開啟狀態
    pub fn is_window_open(&self, kind: crate::snapping::WindowKind) -> bool {
        match kind {
            crate::snapping::WindowKind::MiniPlayer => self.mini_player_window_open,
            crate::snapping::WindowKind::Equalizer => self.equalizer_window_open,
            crate::snapping::WindowKind::Visualizer => crate::visualizer::is_visualizer_open(),
            crate::snapping::WindowKind::Main => !crate::tray::is_window_hidden_to_tray(),
        }
    }

    /// 開啟特定獨立彈出視窗（Pop-out）
    pub fn open_popout_window(&mut self, kind: crate::snapping::WindowKind, spawner: crate::snapping::WindowKind) {
        crate::snapping::set_window_spawner(kind, spawner);
        match kind {
            crate::snapping::WindowKind::MiniPlayer => {
                let was_open = self.mini_player_window_open;
                self.mini_player_window_open = true;
                if !was_open {
                    self.mini_player_needs_initial_dock = true;
                }
            }
            crate::snapping::WindowKind::Equalizer => {
                let was_open = self.equalizer_window_open;
                self.equalizer_window_open = true;
                if !was_open {
                    self.equalizer_needs_initial_dock = true;
                }
            }
            crate::snapping::WindowKind::Visualizer => {
                crate::visualizer::open_visualizer();
            }
            crate::snapping::WindowKind::Main => {
                let hwnd = crate::tray::get_main_hwnd();
                if let Some(ctx) = get_egui_ctx() {
                    crate::tray::show_window_from_tray(hwnd, &ctx);
                }
            }
        }
        crate::spectrum::update_spectrum_needed_state();
    }

    /// 關閉特定獨立彈出視窗（Pop-in / Close）
    pub fn close_popout_window(&mut self, kind: crate::snapping::WindowKind) {
        match kind {
            crate::snapping::WindowKind::MiniPlayer => {
                self.mini_player_window_open = false;
                crate::snapping::set_mini_player_hwnd(std::ptr::null_mut());
                crate::snapping::handle_window_closed(crate::snapping::WindowKind::MiniPlayer);
            }
            crate::snapping::WindowKind::Equalizer => {
                self.equalizer_window_open = false;
                crate::snapping::set_equalizer_hwnd(std::ptr::null_mut());
                crate::snapping::handle_window_closed(crate::snapping::WindowKind::Equalizer);
            }
            crate::snapping::WindowKind::Visualizer => {
                crate::visualizer::close_visualizer();
                crate::snapping::handle_window_closed(crate::snapping::WindowKind::Visualizer);
            }
            crate::snapping::WindowKind::Main => {
                crate::visualizer::reset_hud_main_shown();
                let hwnd = crate::tray::get_main_hwnd();
                if let Some(ctx) = get_egui_ctx() {
                    crate::tray::hide_main_window_only(hwnd, &ctx);
                }
                crate::snapping::handle_window_closed(crate::snapping::WindowKind::Main);
            }
        }
        if let Some(ctx) = get_egui_ctx() {
            ctx.request_repaint();
        }
        crate::spectrum::update_spectrum_needed_state();
    }

    /// 通用 Pop-out 切換控制器：
    /// - 若已開啟（綠色）：關閉該獨立視窗。
    /// - 若未開啟（灰色）：開啟該獨立視窗；若觸發點在該功能的內嵌頁面上，自動退回上一頁（go_back()）。
    /// spawner: 按下觸發按鈕的母視窗（遵循母視窗優先吸附規則）
    pub fn handle_popout_toggle(&mut self, target: crate::snapping::WindowKind, spawner: crate::snapping::WindowKind) {
        if self.is_window_open(target) {
            self.close_popout_window(target);
        } else {
            self.open_popout_window(target, spawner);
            if let Some(embedded_page) = target.embedded_page() {
                if self.current_page == embedded_page {
                    self.go_back();
                }
            }
        }
    }

    /// 切換獨立等化器視窗狀態（開/關）
    pub fn toggle_equalizer_window(&mut self, spawner: crate::snapping::WindowKind) {
        self.handle_popout_toggle(crate::snapping::WindowKind::Equalizer, spawner);
    }

    /// 統一主視窗頂部分頁切換行為（方案 B：若當前已在該分頁則返回，否則跳轉進入該分頁；不關閉獨立視窗）
    pub fn navigate_toggle_page(&mut self, page: Page) {
        if self.current_page == page {
            self.go_back();
        } else {
            self.navigate_to(page);
        }
    }

    /// 前往新頁面，並將當前頁面推入歷史棧中（觸發 Push 滑動過場動畫）
    pub fn navigate_to(&mut self, page: Page) {
        if self.current_page == page {
            return;
        }
        if page == Page::About {
            self.reset_about_scroll = true;
        }
        let from = self.current_page;
        self.page_history.push(from);
        self.current_page = page;
        MAIN_CURRENT_PAGE.store(page as u8, std::sync::atomic::Ordering::SeqCst);
        self.page_transition = Some(PageTransition {
            from,
            to: page,
            direction: TransitionDirection::Push,
            start_time: std::time::Instant::now(),
            duration: std::time::Duration::from_millis(220),
        });
        if let Some(ctx) = get_egui_ctx() {
            ctx.request_repaint();
        }
    }

    /// 返回上一頁；若成功返回則返回 true，若已在最底層頁面則返回 false（觸發 Pop 滑動過場動畫）
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.page_history.pop() {
            let from = self.current_page;
            self.current_page = prev;
            MAIN_CURRENT_PAGE.store(prev as u8, std::sync::atomic::Ordering::SeqCst);
            self.page_transition = Some(PageTransition {
                from,
                to: prev,
                direction: TransitionDirection::Pop,
                start_time: std::time::Instant::now(),
                duration: std::time::Duration::from_millis(220),
            });
            if let Some(ctx) = get_egui_ctx() {
                ctx.request_repaint();
            }
            true
        } else if self.current_page != Page::Main {
            let from = self.current_page;
            self.current_page = Page::Main;
            MAIN_CURRENT_PAGE.store(Page::Main as u8, std::sync::atomic::Ordering::SeqCst);
            self.page_transition = Some(PageTransition {
                from,
                to: Page::Main,
                direction: TransitionDirection::Pop,
                start_time: std::time::Instant::now(),
                duration: std::time::Duration::from_millis(220),
            });
            if let Some(ctx) = get_egui_ctx() {
                ctx.request_repaint();
            }
            true
        } else {
            false
        }
    }

    /// 重設至特定頁面並清空導覽歷史
    pub fn reset_to(&mut self, root: Page) {
        if self.current_page != root {
            let from = self.current_page;
            self.page_transition = Some(PageTransition {
                from,
                to: root,
                direction: TransitionDirection::Pop,
                start_time: std::time::Instant::now(),
                duration: std::time::Duration::from_millis(220),
            });
            if let Some(ctx) = get_egui_ctx() {
                ctx.request_repaint();
            }
        }
        self.page_history.clear();
        self.current_page = root;
        MAIN_CURRENT_PAGE.store(root as u8, std::sync::atomic::Ordering::SeqCst);
    }

    /// 判斷當前是否可以返回上一頁
    pub fn can_go_back(&self) -> bool {
        !self.page_history.is_empty() || self.current_page != Page::Main
    }

}
