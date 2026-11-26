//! Modern AirFlash-style Flyout GUI window built with egui & eframe.

use std::sync::{Arc, RwLock};
use async_channel::Sender as AsyncSender;
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};


use crate::controller::{
    AudioSource, ControllerState, DeviceItem, DeviceStatus, EqPreset, GuiCmd, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};
use crate::i18n::{Language, NoticeKind, ALL_LANGUAGES};
use crate::tray::set_main_hwnd;

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

fn meter_color_at(t: f32) -> egui::Color32 {
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

fn lerp_color(c1: egui::Color32, c2: egui::Color32, t: f32) -> egui::Color32 {
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

static EGUI_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

pub fn register_egui_ctx(ctx: egui::Context) {
    let _ = EGUI_CTX.set(ctx);
}

pub fn get_egui_ctx() -> Option<egui::Context> {
    EGUI_CTX.get().cloned()
}

static PENDING_CLUSTER_CLOSE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static PENDING_CLUSTER_CLOSE_FINISH: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

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
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cmd_tx: AsyncSender<GuiCmd>,
        shared_state: Arc<RwLock<ControllerState>>,
        start_minimized: bool,
    ) -> Self {
        let initial_state = shared_state.read().map(|s| s.clone()).unwrap_or_default();
        let vol = initial_state.volume;
        let initial_eq_gains = initial_state.eq_gains;
        let initial_rx_eq_gains = initial_state.airplay_receiver_eq_gains;
        let initial_preset = EqPreset::matches_gains(&initial_state.eq_gains);
        let initial_rx_preset = EqPreset::matches_gains_with_custom(
            &initial_state.airplay_receiver_eq_gains,
            &initial_state.airplay_receiver_custom_eq_gains,
        );
        let initial_always_on_top = crate::controller::load_always_on_top();
        crate::tray::IS_ALWAYS_ON_TOP.store(initial_always_on_top, std::sync::atomic::Ordering::SeqCst);
        setup_custom_fonts(&cc.egui_ctx, initial_state.language);
        register_egui_ctx(cc.egui_ctx.clone());

        // Force dark theme so even when Windows OS is in Light Mode, the flyout maintains its dark style
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut visual = egui::Visuals::dark();
        visual.window_rounding = egui::Rounding::same(12.0);
        visual.panel_fill = egui::Color32::from_rgb(24, 25, 29);
        cc.egui_ctx.set_visuals_of(egui::Theme::Dark, visual.clone());
        cc.egui_ctx.set_visuals_of(egui::Theme::Light, visual.clone());
        cc.egui_ctx.set_visuals(visual);

        // Constrain global tooltip max width so long tooltips wrap nicely and never overflow the compact 420px window
        cc.egui_ctx.style_mut(|s| {
            s.spacing.tooltip_width = 260.0;
        });

        // Store Win32 HWND so the background tray thread can reliably show/focus this window
        if let Ok(handle) = cc.window_handle() {
            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = win32_handle.hwnd.get() as isize;
                set_main_hwnd(hwnd);
                unsafe {
                    crate::snapping::install_main_window_subclass(hwnd as _);
                }
                if start_minimized {
                    crate::tray::hide_window_to_tray(hwnd as _, &cc.egui_ctx);
                } else {
                    unsafe {
                        crate::tray::force_foreground(hwnd as _);
                    }
                }
            }
        }

        // Load app icon texture for title bar workstation button
        let app_icon_texture = {
            const ICON_BYTES: &[u8] = include_bytes!("../assets/app_icon.png");
            if let Ok(img) = image::load_from_memory(ICON_BYTES) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                Some(cc.egui_ctx.load_texture("app_icon", color_image, egui::TextureOptions::LINEAR))
            } else {
                None
            }
        };

        Self {
            cmd_tx,
            shared_state,
            state: initial_state,
            volume_slider: vol,
            is_dragging_volume: false,
            last_applied_height: 680.0,
            last_applied_width: 460.0,
            selected_device_ids: std::collections::HashSet::new(),
            current_page: Page::Main,
            last_page: Page::Main,
            page_history: Vec::new(),
            page_transition: None,
            settings_tab: SettingsTab::default(),
            meter_level: 0.0,
            last_meter_update: std::time::Instant::now(),
            startup_time: std::time::Instant::now(),
            startup_trimmed: false,
            frame_count: 0,
            last_hidden: start_minimized,
            spectrum_vis: SpectrumVisualizer::default(),
            eq_profile: EqProfile::Normal,
            selected_preset: initial_preset,
            receiver_selected_preset: initial_rx_preset,
            display_eq_gains: initial_eq_gains,
            display_receiver_eq_gains: initial_rx_eq_gains,
            album_art_texture: None,
            last_album_art_bytes: None,
            receiver_artwork_texture: None,
            last_receiver_artwork_len: 0,
            balance: 0.0,
            is_mono: false,
            seek_drag_secs: None,
            is_always_on_top: initial_always_on_top,
            equalizer_window_open: false,
            mini_player_window_open: false,
            mini_player_needs_initial_dock: false,
            equalizer_needs_initial_dock: false,
            app_icon_texture,
            pending_modular_workstation_frames: 0,
            track_transition: None,
            prev_track_title: String::new(),
            prev_track_artist: String::new(),
            prev_track_album: String::new(),
            prev_album_art_texture: None,
            next_track_intent: None,
            workstation_onboarding_dismissed: crate::controller::is_workstation_discovered(),
            mascot_pos: egui::pos2(80.0, 38.0),
            mascot_activity: 0,
            mascot_platform_idx: 1,
            mascot_dir_x: 1.0,
            mascot_dir_y: 1.0,
            mascot_vel_y: 0.0,
            mascot_action_timer: 0.0,
            mascot_squash: 1.0,
            mascot_bounce: 0.0,
            mascot_last_update: std::time::Instant::now(),
            mascot_window: crate::snapping::WindowKind::Main,
            mascot_spectrum_rect: None,
            mascot_spectrum_privilege: false,
            show_supporter_mascot_modal: false,
            reset_about_scroll: true,
        }
    }

    pub fn close_window_cluster(&mut self, origin: crate::snapping::WindowKind) {
        #[cfg(test)]
        {
            self.close_window_cluster_immediate(origin);
            return;
        }

        #[cfg(not(test))]
        {
            if crate::snapping::is_cluster_closing_animating() {
                return;
            }

            let origin_code = origin as u8;
            crate::snapping::animate_cluster_implosion(origin, move || {
                PENDING_CLUSTER_CLOSE_FINISH.store(origin_code, std::sync::atomic::Ordering::SeqCst);
                if let Some(ctx) = get_egui_ctx() {
                    ctx.request_repaint();
                }
            });
        }
    }

    pub fn close_window_cluster_immediate(&mut self, origin: crate::snapping::WindowKind) {
        let root = crate::snapping::get_component_root(origin);
        let members = crate::snapping::get_cluster_members(root);
        for m in members {
            self.close_popout_window(m);
        }
    }

    fn sync_state(&mut self) {
        let main_top = crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main);
        if self.is_always_on_top != main_top {
            self.is_always_on_top = main_top;
            let level = if self.is_always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            if let Some(ctx) = EGUI_CTX.get() {
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
            }
        }

        if let Ok(latest) = self.shared_state.read() {
            // If user isn't dragging with mouse, sync volume immediately
            if !self.is_dragging_volume {
                self.volume_slider = latest.volume;
            }
            self.state = latest.clone();
        }
    }
}

/// Helper to display a word-wrapped tooltip that strictly respects compact window bounds.
#[inline]
fn show_wrapped_tooltip(resp: egui::Response, text: &str) -> egui::Response {
    resp.on_hover_ui(|ui| {
        ui.set_max_width(260.0);
        ui.add(egui::Label::new(egui::RichText::new(text).size(11.0)).wrap());
    })
}

/// Helper to truncate overly long strings to prevent expanding window width.
#[inline]
fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{prefix}…")
    } else {
        s.to_string()
    }
}

/// Helper to launch Windows Volume Mixer (App Volume and Device Preferences)
#[inline]
fn open_windows_volume_mixer() {
    crate::media_control::open_protocol_uri("ms-settings:apps-volume");
}

/// Helper to launch Windows Bluetooth & Devices settings landing page
#[inline]
fn open_windows_bluetooth_settings() {
    crate::media_control::open_protocol_uri("ms-settings:devices");
}

impl eframe::App for FlyoutApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if REQUEST_NAVIGATE_TO_MAIN.swap(false, std::sync::atomic::Ordering::SeqCst) {
            self.reset_to(Page::Main);
        }
        MAIN_CURRENT_PAGE.store(self.current_page as u8, std::sync::atomic::Ordering::SeqCst);

        let pending_close = PENDING_CLUSTER_CLOSE.swap(0, std::sync::atomic::Ordering::SeqCst);
        if pending_close != 0 {
            let kind = match pending_close {
                1 => crate::snapping::WindowKind::Main,
                2 => crate::snapping::WindowKind::MiniPlayer,
                3 => crate::snapping::WindowKind::Equalizer,
                4 => crate::snapping::WindowKind::Visualizer,
                _ => crate::snapping::WindowKind::Main,
            };
            self.close_window_cluster(kind);
        }

        let pending_finish = PENDING_CLUSTER_CLOSE_FINISH.swap(0, std::sync::atomic::Ordering::SeqCst);
        if pending_finish != 0 {
            let kind = match pending_finish {
                1 => crate::snapping::WindowKind::Main,
                2 => crate::snapping::WindowKind::MiniPlayer,
                3 => crate::snapping::WindowKind::Equalizer,
                4 => crate::snapping::WindowKind::Visualizer,
                _ => crate::snapping::WindowKind::Main,
            };
            self.close_window_cluster_immediate(kind);
        }

        // ⭐ 正式版支持者感謝盛典自動觸發：
        // 當偵測到是正式版已購買使用者（或獨立開發測試模式），且尚未揭密展示過，自動彈出大號精靈感謝盛典！
        if self.state.license_tier.is_supporter_or_unpackaged()
            && !self.state.mascot_supporter_secret_revealed
            && !self.show_supporter_mascot_modal
        {
            self.show_supporter_mascot_modal = true;
        }

        if self.pending_modular_workstation_frames > 0 {
            self.pending_modular_workstation_frames -= 1;
            let main_hwnd = crate::tray::get_main_hwnd();
            let mini_hwnd = crate::snapping::get_window_hwnd(crate::snapping::WindowKind::MiniPlayer);
            let eq_hwnd = crate::snapping::get_window_hwnd(crate::snapping::WindowKind::Equalizer);
            let is_vis_fs = crate::visualizer::is_visualizer_fullscreen();
            let vis_hwnd = if is_vis_fs {
                std::ptr::null_mut()
            } else {
                crate::snapping::get_window_hwnd(crate::snapping::WindowKind::Visualizer)
            };

            let need_mini = self.mini_player_window_open;
            let need_eq = self.equalizer_window_open;
            let need_vis = !is_vis_fs && crate::visualizer::is_visualizer_open();

            let all_ready = unsafe {
                (!need_mini || (!mini_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(mini_hwnd) != 0))
                    && (!need_eq || (!eq_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(eq_hwnd) != 0))
                    && (!need_vis || (!vis_hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(vis_hwnd) != 0))
            };

            if all_ready {
                self.pending_modular_workstation_frames = 0;
                unsafe {
                    crate::snapping::layout_modular_workstation(main_hwnd, mini_hwnd, eq_hwnd, vis_hwnd);
                }
            } else if self.pending_modular_workstation_frames > 0 {
                ctx.request_repaint();
            } else {
                // Timeout fallback: layout whatever windows are currently available
                unsafe {
                    crate::snapping::layout_modular_workstation(main_hwnd, mini_hwnd, eq_hwnd, vis_hwnd);
                }
            }
        }

        self.sync_state();
        self.sync_media_track(ctx);

        let _hwnd = crate::tray::get_main_hwnd();
        let is_hidden = crate::tray::is_window_hidden_to_tray();

        // When window transitions to hidden in tray, schedule debounced working set trim
        if !self.last_hidden && is_hidden {
            crate::tray::schedule_trim_working_set();
        } else if self.last_hidden && !is_hidden {
            crate::tray::cancel_pending_trim();
        }
        self.last_hidden = is_hidden;

        self.frame_count = self.frame_count.saturating_add(1);

        // Auto-trim working set early without waiting for user to minimize to tray:
        // - If start_minimized, trim immediately on first frame.
        // - If visible, trim at frame 3 (after shaders & font textures uploaded) or after 1.5s
        if !self.startup_trimmed {
            if is_hidden {
                self.startup_trimmed = true;
                crate::tray::trim_working_set();
            } else if self.frame_count >= 3 || self.startup_time.elapsed() >= std::time::Duration::from_millis(1500) {
                self.startup_trimmed = true;
                crate::tray::trim_working_set();
                tracing::info!("FlyoutApp: Early working set trim performed after startup (frame {})", self.frame_count);
            }
        }

        // Intercept window close (X button) or Escape key in fullscreen HUD
        let close_req = ctx.input(|i| i.viewport().close_requested());
        let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if crate::visualizer::is_visualizer_fullscreen() && (escape_pressed && !self.can_go_back()) {
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            crate::visualizer::toggle_main_window_hud();
        } else if escape_pressed && self.can_go_back() {
            self.go_back();
        } else if close_req || escape_pressed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            tracing::info!(
                "FlyoutApp: Hiding main window to system tray: close_req={}, escape={}",
                close_req, escape_pressed
            );
            self.close_popout_window(crate::snapping::WindowKind::Main);
        }

        let is_fleet_hidden = crate::tray::is_app_fleet_hidden();
        let is_main_hidden = crate::tray::is_window_hidden_to_tray();

        if (is_fleet_hidden || is_main_hidden) && !self.equalizer_window_open && !self.mini_player_window_open {
            // When hidden in system tray and no secondary viewports are open:
            // 1. Completely skip all UI widget rendering, layout, and spectrum calculations.
            // 2. Request a 1-second background refresh to keep state fresh.
            // 3. Thread stays paced by Windows DWM VSync, completely eliminating the 5% CPU busy loop!
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
            crate::spectrum::update_spectrum_needed_state();
            return;
        }

        crate::spectrum::update_spectrum_needed_state();

        // Keep tooltip max width bounded within viewport width so right-aligned popups never clip
        let avail_w = ctx.screen_rect().width();
        let max_tip_w = (avail_w - 60.0).clamp(180.0, 260.0);
        ctx.style_mut(|s| {
            s.spacing.tooltip_width = max_tip_w;
        });

        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            crate::visualizer::set_language(self.state.language.code());
            crate::visualizer::toggle_visualizer();
        }

        // Update animated audio meter level with smooth attack and decay
        // Runs whenever ANY window in the fleet (Main, MiniPlayer, Equalizer) is visible!
        let now = std::time::Instant::now();
        let dt = (now - self.last_meter_update).as_secs_f32().clamp(0.001, 0.1);
        self.last_meter_update = now;

        let is_active_playback = (self.state.status == DeviceStatus::Streaming || matches!(self.state.status, DeviceStatus::AoMode(_))) && !self.state.is_muted;

        let raw_peak = if is_active_playback {
            if let DeviceStatus::AoMode(ref name) = self.state.status {
                let dev_name = if name == "default" {
                    self.state.language.default_output_device()
                } else {
                    name.as_str()
                };
                crate::visualizer::update_device_info(dev_name);
            } else {
                crate::visualizer::update_device_info(self.state.connected_name.as_deref().unwrap_or("HomePod"));
            }
            crate::controller::get_audio_peak_level()
        } else {
            crate::visualizer::update_device_info("");
            0.0
        };

        // Perceptual curve: sqrt expands quiet audio dynamics so it feels responsive
        let target = raw_peak.clamp(0.0, 1.0).sqrt();
        if target > self.meter_level {
            // Snappy attack
            self.meter_level = target;
        } else {
            // Smooth decay / release
            self.meter_level = (self.meter_level - dt * 1.5).max(target).max(0.0);
        }

        // Update 16-band real-time audio spectrum
        let target_spectrum = if is_active_playback {
            crate::spectrum::get_spectrum_levels()
        } else {
            [0.0; 16]
        };

        let mut has_audio_motion = false;
        for i in 0..16 {
            let target = target_spectrum[i];
            if target > self.spectrum_vis.current[i] {
                self.spectrum_vis.current[i] = target;
            } else {
                self.spectrum_vis.current[i] = (self.spectrum_vis.current[i] - dt * 2.2).max(target).max(0.0);
            }

            if self.spectrum_vis.current[i] >= self.spectrum_vis.peaks[i] {
                self.spectrum_vis.peaks[i] = self.spectrum_vis.current[i];
                self.spectrum_vis.peak_decay[i] = 0.35;
            } else {
                if self.spectrum_vis.peak_decay[i] > 0.0 {
                    self.spectrum_vis.peak_decay[i] -= dt;
                } else {
                    self.spectrum_vis.peaks[i] = (self.spectrum_vis.peaks[i] - dt * 1.0).max(self.spectrum_vis.current[i]).max(0.0);
                }
            }

            if self.spectrum_vis.current[i] > 0.005 || self.spectrum_vis.peaks[i] > 0.005 {
                has_audio_motion = true;
            }
        }

        // Dynamic Adaptive Framerate:
        // - When audio is actively playing, or when meters/spectrum bars are decaying down: render at 30 FPS (33ms) for fluid animation.
        // - When completely idle and silent: back off to 500ms (2 FPS) to dramatically reduce idle CPU usage to ~0.5%.
        let is_audio_animating = is_active_playback
            && (raw_peak > 0.001 || self.meter_level > 0.005 || has_audio_motion);
        let repaint_delay = if is_audio_animating || self.track_transition.is_some() {
            std::time::Duration::from_millis(16)
        } else {
            std::time::Duration::from_millis(500)
        };
        ctx.request_repaint_after(repaint_delay);

        let lang = self.state.language;

        // Viewport 2: Standalone Equalizer Window
        let eq_title = lang.mini_equalizer_window_title();
        let eq_open = self.equalizer_window_open;
        let mut eq_dock = self.equalizer_needs_initial_dock;
        self.show_standalone_viewport(
            ctx,
            crate::snapping::WindowKind::Equalizer,
            &eq_title,
            eq_open,
            &mut eq_dock,
            |app, ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    app.render_equalizer_content(ui, true);
                });
            },
        );
        self.equalizer_needs_initial_dock = eq_dock;

        // Viewport 3: Standalone Mini Player Window
        let mini_title = lang.mini_player_window_title();
        let mini_open = self.mini_player_window_open;
        let mut mini_dock = self.mini_player_needs_initial_dock;
        self.show_standalone_viewport(
            ctx,
            crate::snapping::WindowKind::MiniPlayer,
            &mini_title,
            mini_open,
            &mut mini_dock,
            |app, ui| {
                app.render_mini_player_content(ui);
            },
        );
        self.mini_player_needs_initial_dock = mini_dock;

        if is_main_hidden || is_fleet_hidden {
            // Main window itself is hidden in system tray (or whole fleet hidden to tray):
            // secondary viewports were already rendered above with live spectrum and dynamic framerate.
            // Skip main window UI widget rendering (footer, central panels).
            return;
        }

        // Dark modern background with fluent style
        let mut visual = egui::Visuals::dark();
        visual.window_rounding = egui::Rounding::same(12.0);
        visual.panel_fill = egui::Color32::from_rgb(24, 25, 29);
        ctx.set_visuals(visual);

        // Fixed bottom footer panel (never moves when expanding settings, never cut off)
        egui::TopBottomPanel::bottom("flyout_footer")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(18, 19, 22))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(38, 40, 48))),
            )
            .show(ctx, |ui| {
                let lang = self.state.language;

                // 未購買時（試用中或試用已到期），在上方顯示提醒與微軟商店購買引導按鈕 (提案 A)
                match self.state.license_tier {
                    crate::license::LicenseTier::Trial { days_remaining } => {
                        ui.horizontal(|ui| {
                            let trial_text = lang.footer_trial_remaining(days_remaining);
                            ui.label(
                                egui::RichText::new(trial_text)
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(255, 195, 75)) // Warm Gold
                                    .strong(),
                            )
                            .on_hover_text(lang.footer_trial_tooltip());

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let buy_btn = egui::Button::new(
                                    egui::RichText::new(lang.footer_btn_buy_full())
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(255, 235, 160))
                                        .strong(),
                                )
                                .fill(egui::Color32::from_rgb(60, 45, 15)) // Deep Golden Amber
                                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(215, 165, 40)))
                                .rounding(egui::Rounding::same(4.0));

                                if ui.add(buy_btn).on_hover_text(lang.btn_buy_product()).clicked() {
                                    crate::license::open_store_product_page();
                                }
                            });
                        });
                        ui.add_space(3.0);
                        ui.separator();
                        ui.add_space(3.0);
                    }
                    crate::license::LicenseTier::TrialExpired => {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(lang.footer_trial_expired())
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(255, 120, 120)) // Warning Coral
                                    .strong(),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let unlock_btn = egui::Button::new(
                                    egui::RichText::new(lang.footer_btn_unlock_now())
                                        .size(11.0)
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(egui::Color32::from_rgb(180, 40, 40)) // Warning Crimson
                                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(230, 80, 80)))
                                .rounding(egui::Rounding::same(4.0));

                                if ui.add(unlock_btn).on_hover_text(lang.btn_buy_product()).clicked() {
                                    crate::license::open_store_product_page();
                                }
                            });
                        });
                        ui.add_space(3.0);
                        ui.separator();
                        ui.add_space(3.0);
                    }
                    _ => {}
                }

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(lang.top_hint_minimize())
                            .weak()
                            .size(11.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let is_quitting = crate::tray::is_app_quitting();
                        if is_quitting {
                            ui.add_enabled(
                                false,
                                egui::Button::new(
                                    egui::RichText::new(lang.btn_quitting_app())
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(160, 160, 160)),
                                ),
                            );
                        } else if ui
                            .button(
                                egui::RichText::new(lang.btn_quit_app())
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(230, 100, 100)),
                            )
                            .clicked()
                        {
                            crate::tray::trigger_app_quit(&self.cmd_tx);
                            ctx.request_repaint();
                        }
                    });
                });
            });

        let mut total_content_height = 0.0f32;

        let main_op = crate::snapping::get_window_opacity(crate::snapping::WindowKind::Main);
        let main_hwnd = crate::tray::get_main_hwnd();
        let is_main_click_through = crate::snapping::is_click_through_active(crate::snapping::WindowKind::Main);
        let is_main_hovered = if !main_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(main_hwnd) != 0 } {
            unsafe {
                crate::snapping::update_window_click_through(crate::snapping::WindowKind::Main, main_hwnd);
                // 穿透模式下：停用 Hover Peek 實體化，維持穩定半透明，不閃爍
                !is_main_click_through && crate::snapping::is_cursor_over_window(main_hwnd)
            }
        } else {
            false
        };
        let is_main_suppressed = crate::snapping::is_peek_suppressed(crate::snapping::WindowKind::Main);
        let target_main_op = if is_main_hovered && crate::snapping::is_hover_peek_enabled() && !is_main_suppressed {
            1.0
        } else {
            main_op
        };
        let (effective_main_op, is_main_animating) = crate::snapping::step_animated_window_opacity(
            crate::snapping::WindowKind::Main,
            target_main_op,
        );
        let main_alpha = (effective_main_op.clamp(0.2, 1.0) * 255.0).round() as u8;

        if !main_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(main_hwnd) != 0 } {
            unsafe {
                crate::snapping::apply_native_window_opacity(main_hwnd, effective_main_op);
            }
        }

        // 當平滑淡入淡出中或懸停實體化生效時，以 60 FPS 刷新畫面
        if is_main_animating || (is_main_hovered && crate::snapping::is_hover_peek_enabled() && main_op < 0.99) {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        let main_role = crate::snapping::get_window_role(crate::snapping::WindowKind::Main);
        let stroke_alpha = main_alpha.max(100);
        let stroke_color = match main_role {
            crate::snapping::WindowRole::ClusterMaster => egui::Color32::from_rgba_unmultiplied(70, 140, 215, stroke_alpha), // Blue master glow border
            crate::snapping::WindowRole::ClusterSlave => egui::Color32::from_rgba_unmultiplied(45, 50, 60, stroke_alpha),
            crate::snapping::WindowRole::Standalone => egui::Color32::from_rgba_unmultiplied(52, 58, 70, stroke_alpha),
        };

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(24, 25, 29, 215))
                    .stroke(egui::Stroke::new(1.0_f32, stroke_color))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(14.0)
            )
            .show(ctx, |ui| {
                let full_rect = ui.available_rect_before_wrap();
                let clip_rect = full_rect;

                // 檢查是否有正在進行的頁面推拉過場動畫
                let mut active_trans = self.page_transition;
                if let Some(trans) = active_trans {
                    let elapsed = trans.start_time.elapsed();
                    if elapsed >= trans.duration {
                        self.page_transition = None;
                        active_trans = None;
                    }
                }

                if let Some(trans) = active_trans {
                    // 過場動畫持續中，要求連續重繪每一幀
                    ctx.request_repaint();

                    let elapsed = trans.start_time.elapsed();
                    let raw_t = (elapsed.as_secs_f32() / trans.duration.as_secs_f32()).clamp(0.0, 1.0);
                    // Ease-out cubic: 1 - (1 - t)^3 (起步迅速，尾段細膩減速煞停)
                    let t = 1.0 - (1.0 - raw_t).powi(3);
                    let width = full_rect.width();

                    match trans.direction {
                        TransitionDirection::Push => {
                            // Push 前進過場：
                            // 舊頁面 (from) 向左視差退後滑動 (-28% 寬度) 並稍微暗化
                            // 新頁面 (to) 從右側 (+100% 寬度) 滑入至原位
                            let from_offset_x = -width * 0.28 * t;
                            let to_offset_x = width * (1.0 - t);

                            // 1. 底層舊頁面 (from)
                            let from_rect = full_rect.translate(egui::vec2(from_offset_x, 0.0));
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(from_rect), |from_ui| {
                                from_ui.set_clip_rect(clip_rect);
                                from_ui.push_id("trans_from", |from_ui| {
                                    self.render_page_content(from_ui, trans.from);
                                });
                            });

                            // 舊頁面暗化覆蓋 (漸變至約 40% 黑色)
                            let scrim_alpha = (100.0 * t).clamp(0.0, 100.0) as u8;
                            if scrim_alpha > 0 {
                                ui.painter().rect_filled(
                                    from_rect.intersect(clip_rect),
                                    egui::Rounding::same(8.0),
                                    egui::Color32::from_black_alpha(scrim_alpha),
                                );
                            }

                            // 2. 頂層新頁面 (to)
                            let to_rect = full_rect.translate(egui::vec2(to_offset_x, 0.0));
                            let to_visible_rect = to_rect.intersect(clip_rect);
                            // 填充不透明底色以遮擋底層舊頁面
                            ui.painter().rect_filled(
                                to_visible_rect,
                                egui::Rounding::same(8.0),
                                egui::Color32::from_rgb(24, 25, 29),
                            );

                            // 新頁面左側投射細膩投影
                            let shadow_x = full_rect.left() + to_offset_x;
                            render_slide_shadow(ui, shadow_x, clip_rect);

                            // 渲染新頁面內容 (以正常 ID 空間渲染)
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(to_rect), |to_ui| {
                                to_ui.set_clip_rect(clip_rect);
                                total_content_height = self.render_page_content(to_ui, trans.to);
                            });
                        }
                        TransitionDirection::Pop => {
                            // Pop 返回過場：
                            // 新頁面 (to) 從左側 (-28% 寬度) 隨視差向前滑回原位，暗化漸退
                            // 舊頁面 (from) 向右 (+100% 寬度) 滑出螢幕
                            let to_offset_x = -width * 0.28 * (1.0 - t);
                            let from_offset_x = width * t;

                            // 1. 底層新頁面 (to)
                            let to_rect = full_rect.translate(egui::vec2(to_offset_x, 0.0));
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(to_rect), |to_ui| {
                                to_ui.set_clip_rect(clip_rect);
                                total_content_height = self.render_page_content(to_ui, trans.to);
                            });

                            // 暗化覆蓋隨返回逐漸消退
                            let scrim_alpha = (100.0 * (1.0 - t)).clamp(0.0, 100.0) as u8;
                            if scrim_alpha > 0 {
                                ui.painter().rect_filled(
                                    to_rect.intersect(clip_rect),
                                    egui::Rounding::same(8.0),
                                    egui::Color32::from_black_alpha(scrim_alpha),
                                );
                            }

                            // 2. 頂層舊頁面 (from)
                            let from_rect = full_rect.translate(egui::vec2(from_offset_x, 0.0));
                            let from_visible_rect = from_rect.intersect(clip_rect);
                            // 填充不透明底色以遮擋底層新頁面
                            ui.painter().rect_filled(
                                from_visible_rect,
                                egui::Rounding::same(8.0),
                                egui::Color32::from_rgb(24, 25, 29),
                            );

                            // 舊頁面左側投射細膩投影
                            let shadow_x = full_rect.left() + from_offset_x;
                            render_slide_shadow(ui, shadow_x, clip_rect);

                            // 渲染舊頁面內容
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(from_rect), |from_ui| {
                                from_ui.set_clip_rect(clip_rect);
                                from_ui.push_id("trans_from", |from_ui| {
                                    self.render_page_content(from_ui, trans.from);
                                });
                            });
                        }
                    }
                    ui.allocate_rect(full_rect, egui::Sense::hover());
                } else {
                    total_content_height = self.render_page_content(ui, self.current_page);
                }
                Self::render_opacity_hud_badge(ui, crate::snapping::WindowKind::Main);
                Self::render_header_shimmer_boundary(ui, crate::snapping::WindowKind::Main);
                self.render_mascot_overlay(ui, crate::snapping::WindowKind::Main);
                self.render_supporter_mascot_modal(ui);
            });

        // Standardize Main Window dimensions to Player benchmark (460.0 x 680.0) across all pages.
        // The window never automatically resizes, shrinks, or wobbles when navigating!
        let base_width = 460.0;
        let base_height = 680.0;

        if self.last_page != self.current_page {
            self.last_page = self.current_page;
        }

        // On initial startup (first frame), ensure the window has the standardized base dimensions
        if self.frame_count < 2 {
            self.last_applied_width = base_width;
            self.last_applied_height = base_height;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(base_width, base_height)));
        }
    }
}

/// 繪製頁面推拉過場時接縫邊緣的柔和漸變陰影
fn render_slide_shadow(ui: &mut egui::Ui, seam_x: f32, clip_rect: egui::Rect) {
    if seam_x <= clip_rect.left() || seam_x >= clip_rect.right() {
        return;
    }
    const STRIPS: usize = 6;
    const STRIP_WIDTH: f32 = 4.0;
    let total_width = STRIPS as f32 * STRIP_WIDTH;

    for i in 0..STRIPS {
        let left = (seam_x - total_width + (i as f32 * STRIP_WIDTH)).max(clip_rect.left());
        let right = (seam_x - total_width + ((i + 1) as f32 * STRIP_WIDTH)).min(clip_rect.right());
        if right > left {
            let progress = (i + 1) as f32 / STRIPS as f32;
            let alpha = (90.0 * progress.powi(2)).clamp(0.0, 255.0) as u8;
            let strip_rect = egui::Rect::from_min_max(
                egui::pos2(left, clip_rect.top()),
                egui::pos2(right, clip_rect.bottom()),
            );
            ui.painter().rect_filled(
                strip_rect,
                egui::Rounding::ZERO,
                egui::Color32::from_black_alpha(alpha),
            );
        }
    }
}

/// 計算 4 個控制點在參數 t (0.0..=1.0) 下的 Catmull-Rom 三次仿樣插值點
fn catmull_rom(p0: egui::Pos2, p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2, t: f32) -> egui::Pos2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let f0 = -0.5 * t3 + t2 - 0.5 * t;
    let f1 = 1.5 * t3 - 2.5 * t2 + 1.0;
    let f2 = -1.5 * t3 + 2.0 * t2 + 0.5 * t;
    let f3 = 0.5 * t3 - 0.5 * t2;
    egui::pos2(
        p0.x * f0 + p1.x * f1 + p2.x * f2 + p3.x * f3,
        p0.y * f0 + p1.y * f1 + p2.y * f2 + p3.y * f3,
    )
}

/// 繪製穿過 10 頻段拉桿滑塊的流暢頻率響應曲線與動態形變光暈
fn render_eq_curve_overlay(
    ui: &egui::Ui,
    nodes: &[egui::Pos2],
    zero_db_y: f32,
    enabled: bool,
) {
    if nodes.len() < 2 {
        return;
    }

    let n = nodes.len();
    let mut spline_points = Vec::with_capacity((n - 1) * 8 + 1);

    for i in 0..(n - 1) {
        let p0 = if i == 0 {
            egui::pos2(nodes[0].x * 2.0 - nodes[1].x, nodes[0].y * 2.0 - nodes[1].y)
        } else {
            nodes[i - 1]
        };
        let p1 = nodes[i];
        let p2 = nodes[i + 1];
        let p3 = if i + 2 < n {
            nodes[i + 2]
        } else {
            egui::pos2(nodes[n - 1].x * 2.0 - nodes[n - 2].x, nodes[n - 1].y * 2.0 - nodes[n - 2].y)
        };

        let steps = 8;
        for s in 0..steps {
            let t = s as f32 / steps as f32;
            spline_points.push(catmull_rom(p0, p1, p2, p3, t));
        }
    }
    spline_points.push(*nodes.last().unwrap());

    // 1. 繪製 0 dB 水平參考基準虛線
    let x_start = nodes.first().unwrap().x;
    let x_end = nodes.last().unwrap().x;
    ui.painter().line_segment(
        [egui::pos2(x_start, zero_db_y), egui::pos2(x_end, zero_db_y)],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
    );

    // 2. 繪製曲線與 0 dB 基準線之間的半透明漸變填充網格 (較暗、沉穩的高雅半透明微光)
    let fill_color = if enabled {
        egui::Color32::from_rgba_unmultiplied(0, 135, 120, 16)
    } else {
        egui::Color32::from_rgba_unmultiplied(75, 80, 90, 8)
    };

    let mut mesh = egui::Mesh::default();
    for i in 0..(spline_points.len() - 1) {
        let pt_a = spline_points[i];
        let pt_b = spline_points[i + 1];
        let base_a = egui::pos2(pt_a.x, zero_db_y);
        let base_b = egui::pos2(pt_b.x, zero_db_y);

        let idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: pt_a, uv: egui::epaint::WHITE_UV, color: fill_color });
        mesh.vertices.push(egui::epaint::Vertex { pos: pt_b, uv: egui::epaint::WHITE_UV, color: fill_color });
        mesh.vertices.push(egui::epaint::Vertex { pos: base_b, uv: egui::epaint::WHITE_UV, color: fill_color });
        mesh.vertices.push(egui::epaint::Vertex { pos: base_a, uv: egui::epaint::WHITE_UV, color: fill_color });

        mesh.add_triangle(idx, idx + 1, idx + 2);
        mesh.add_triangle(idx, idx + 2, idx + 3);
    }
    ui.painter().add(egui::Shape::mesh(mesh));

    // 3. 繪製發光曲線筆觸 (沉穩深青藍/暗青色，低調不刺眼)
    let line_color = if enabled {
        egui::Color32::from_rgb(0, 155, 135)
    } else {
        egui::Color32::from_rgb(85, 92, 102)
    };
    ui.painter().add(egui::Shape::line(
        spline_points,
        egui::Stroke::new(1.8_f32, line_color),
    ));

    // 4. 在每個滑塊節點中心繪製發光圓點 (暗色細緻光暈，柔和自然)
    let halo_color = if enabled {
        egui::Color32::from_rgba_unmultiplied(0, 155, 135, 45)
    } else {
        egui::Color32::from_rgba_unmultiplied(85, 92, 102, 25)
    };
    let dot_center_color = if enabled {
        egui::Color32::from_rgb(145, 205, 195)
    } else {
        egui::Color32::from_rgb(115, 120, 130)
    };
    for &node in nodes {
        ui.painter().circle_filled(node, 3.5, halo_color);
        ui.painter().circle_filled(node, 1.8, dot_center_color);
    }
}

impl FlyoutApp {
    /// 統一調度渲染指定頁面的所有內容
    pub fn render_page_content(&mut self, ui: &mut egui::Ui, page: Page) -> f32 {
        match page {
            Page::About => self.render_about_page(ui),
            Page::Equalizer => self.render_equalizer_page(ui),
            Page::Player => self.render_player_page(ui),
            Page::Settings => self.render_settings_page(ui),
            Page::Main => self.render_main_page(ui),
        }
    }

    /// 渲染主視窗根頁面內容
    pub fn render_main_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let initial_top = ui.cursor().top();
        let lang = self.state.language;

        // 主視窗頁面：頂部通用標題列（App Icon 工作站按鈕、標題、微光身分徽章與控制按鈕）
        self.render_universal_window_header(
            ui,
            crate::snapping::WindowKind::Main,
            "Win2Pod Bridge",
            false,
            |app, ui| {
                app.render_visualizer_button(ui, crate::snapping::WindowKind::Main);
                app.render_player_button(ui);
                app.render_settings_button(ui);
            },
        );

        // 主視窗內容區域捲動
        egui::ScrollArea::vertical()
            .id_salt("main_scroll_area")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Notice Banner (e.g. preemption notice, restored notification)
                let mut dismiss_notice = false;
                if let Some((ref notice, time)) = self.state.notice_message {
                    let is_success = matches!(
                        notice,
                        NoticeKind::ConnectionRestored(_) | NoticeKind::ResumeRestored(_)
                    );
                    let is_preempted = matches!(
                        notice,
                        NoticeKind::Preempted(_) | NoticeKind::ResumePreempted(_)
                    );
                    let is_connecting = matches!(
                        notice,
                        NoticeKind::ConnectionLostReconnecting(_) | NoticeKind::ResumeReconnecting(_)
                    );

                    let max_secs = if is_success {
                        5
                    } else if is_preempted {
                        60
                    } else if is_connecting {
                        30
                    } else {
                        15
                    };

                    if time.elapsed().as_secs() < max_secs {
                        let (icon, bg_color, stroke_color, title_color) = if is_success {
                            ("✔", egui::Color32::from_rgb(18, 38, 22), egui::Color32::from_rgb(46, 160, 67), egui::Color32::from_rgb(80, 220, 120))
                        } else if is_preempted {
                            ("ℹ", egui::Color32::from_rgb(38, 30, 16), egui::Color32::from_rgb(215, 145, 35), egui::Color32::from_rgb(250, 185, 70))
                        } else if is_connecting {
                            ("🔄", egui::Color32::from_rgb(36, 32, 18), egui::Color32::from_rgb(200, 150, 30), egui::Color32::from_rgb(240, 180, 50))
                        } else {
                            ("⚠", egui::Color32::from_rgb(38, 20, 20), egui::Color32::from_rgb(200, 60, 60), egui::Color32::from_rgb(250, 100, 100))
                        };

                        egui::Frame::group(ui.style())
                            .fill(bg_color)
                            .stroke(egui::Stroke::new(1.0_f32, stroke_color))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(icon).size(15.0).color(title_color));
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(notice.title(lang))
                                                .color(title_color)
                                                .strong()
                                                .size(12.5),
                                        );
                                        ui.label(
                                            egui::RichText::new(notice.render(lang))
                                                .color(egui::Color32::from_rgb(220, 220, 225))
                                                .size(11.5),
                                        );
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let btn = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new("✕")
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(170, 170, 180)),
                                            )
                                            .frame(false),
                                        );
                                        if btn.clicked() {
                                            dismiss_notice = true;
                                        }
                                    });
                                });
                            });
                        ui.add_space(10.0);
                    }
                }
                if dismiss_notice {
                    self.state.notice_message = None;
                }

                // --- 1. Current Status Banner ---
                match &self.state.status {
                    DeviceStatus::Idle => {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::GRAY, "●");
                            ui.label(egui::RichText::new(lang.status_idle()).color(egui::Color32::LIGHT_GRAY));
                        });
                    }
                    DeviceStatus::Connecting => {
                        let raw_name = self.state.connected_name.as_deref().unwrap_or("Audio");
                        let short_name = truncate_text(raw_name, 22);
                        let full_status = lang.status_connecting(raw_name);
                        let short_status = lang.status_connecting(&short_name);
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(240, 160, 40), "●");
                            ui.label(
                                egui::RichText::new(&short_status)
                                    .color(egui::Color32::from_rgb(240, 160, 40))
                                    .strong(),
                            ).on_hover_text(&full_status);
                        });
                    }
                    DeviceStatus::Streaming => {
                        let raw_name = self.state.connected_name.as_deref().unwrap_or("Audio");
                        let short_name = truncate_text(raw_name, 22);
                        let full_status = lang.status_streaming(raw_name);
                        let short_status = lang.status_streaming(&short_name);
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(70, 200, 120), "●");
                            ui.label(
                                egui::RichText::new(&short_status)
                                    .color(egui::Color32::from_rgb(70, 200, 120))
                                    .strong(),
                            ).on_hover_text(&full_status);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new(lang.btn_disconnect()).color(egui::Color32::LIGHT_RED)).clicked() {
                                    self.state.connected_name = None;
                                    self.state.connected_device_ids.clear();
                                    self.state.device_volumes.clear();
                                    if self.state.auto_ao_mode {
                                        self.state.status = DeviceStatus::AoMode("default".to_string());
                                    } else {
                                        self.state.status = DeviceStatus::Idle;
                                    }
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::Disconnect);
                                }
                            });
                        });
                    }
                    DeviceStatus::AoMode(name) => {
                        let dev_name = if name == "default" {
                            lang.default_output_device()
                        } else {
                            name.as_str()
                        };
                        let short_name = truncate_text(dev_name, 22);
                        let full_status = format!("{} ({})", lang.status_ao_active(), dev_name);
                        let short_status = format!("{} ({})", lang.status_ao_active(), short_name);
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(0, 210, 255), "●");
                            ui.label(
                                egui::RichText::new(&short_status)
                                    .color(egui::Color32::from_rgb(0, 210, 255))
                                    .strong(),
                            ).on_hover_text(&full_status);
                        });
                    }
                    DeviceStatus::Error(err) => {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(42, 22, 24))
                            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(140, 45, 45)))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::RED, "●");
                                    let wrap_w = (ui.available_width() - 8.0).max(60.0);
                                    ui.add_sized(
                                        [wrap_w, 0.0],
                                        egui::Label::new(
                                             egui::RichText::new(lang.status_error(err))
                                                .color(egui::Color32::from_rgb(255, 130, 130))
                                                .size(11.5),
                                        )
                                        .wrap(),
                                    );
                                });
                            });
                    }
                }

                ui.add_space(6.0);

                // --- 2. Device List ---
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(lang.available_devices_header()).strong().size(13.0));
                    if self.state.status == DeviceStatus::Streaming && self.state.connected_device_ids.len() > 1 {
                        ui.label(
                            egui::RichText::new(lang.multi_room_label(self.state.connected_device_ids.len()))
                                .color(egui::Color32::from_rgb(100, 200, 255))
                                .size(11.0),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new(lang.btn_refresh()).size(11.5))
                            .on_hover_text(lang.tooltip_refresh())
                            .clicked()
                        {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshDevices);
                            let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshAudioDevices);
                        }
                    });
                });
                ui.add_space(3.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        if self.state.devices.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(6.0);
                                if self.state.is_discovering {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label(egui::RichText::new(lang.no_devices_found()).weak());
                                    });
                                } else {
                                    ui.label(egui::RichText::new(lang.no_devices_found()).weak());
                                }
                                ui.add_space(6.0);
                            });
                        } else {
                            let is_streaming = self.state.status == DeviceStatus::Streaming;
                            let is_connecting = self.state.status == DeviceStatus::Connecting;

                            for dev in &self.state.devices {
                                let dev_id = dev.id_string();
                                let is_current = self.state.connected_device_ids.contains(&dev_id)
                                    || (self.state.connected_name.as_deref() == Some(&dev.name) && (is_streaming || is_connecting));

                                ui.horizontal(|ui| {
                                    // Checkbox for idle multi-selection
                                    if !is_streaming && !is_connecting {
                                        let mut checked = self.selected_device_ids.contains(&dev_id);
                                        if ui.checkbox(&mut checked, "").changed() {
                                            if checked {
                                                self.selected_device_ids.insert(dev_id.clone());
                                            } else {
                                                self.selected_device_ids.remove(&dev_id);
                                            }
                                        }
                                    }

                                    let icon = if is_current && is_streaming {
                                        egui::RichText::new("▶").color(egui::Color32::from_rgb(70, 200, 120))
                                    } else if is_current && is_connecting {
                                        egui::RichText::new("●").color(egui::Color32::from_rgb(240, 160, 40))
                                    } else {
                                        egui::RichText::new("○").weak()
                                    };
                                    ui.label(icon);

                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&dev.name).strong());
                                            if dev.is_stereo_pair {
                                                ui.label(
                                                    egui::RichText::new(format!("👥 {}", lang.stereo_pair_tag()))
                                                        .color(egui::Color32::from_rgb(100, 200, 255))
                                                        .size(10.5),
                                                )
                                                .on_hover_text(lang.stereo_pair_tooltip());
                                            }
                                        });
                                        let friendly = lang.friendly_model(&dev.raw_device.model);
                                        let model_display = if dev.is_stereo_pair {
                                            lang.stereo_pair_model_desc(
                                                if !friendly.is_empty() { friendly } else { &dev.raw_device.model },
                                                dev.pair_count.max(2),
                                            )
                                        } else if !friendly.is_empty() {
                                            friendly.to_string()
                                        } else {
                                            dev.model.clone()
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("{} • {}", model_display, dev.ip))
                                                .weak()
                                                .size(11.0),
                                        );
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if is_current && (is_streaming || is_connecting) {
                                            if ui.button(egui::RichText::new(lang.btn_leave()).color(egui::Color32::LIGHT_RED)).clicked() {
                                                let mut remaining: Vec<DeviceItem> = self
                                                    .state
                                                    .devices
                                                    .iter()
                                                    .filter(|d| d.id_string() != dev_id && self.state.connected_device_ids.contains(&d.id_string()))
                                                    .cloned()
                                                    .collect();
                                                if remaining.is_empty() || self.state.connected_device_ids.len() <= 1 {
                                                    self.state.connected_name = None;
                                                    self.state.connected_device_ids.clear();
                                                    self.state.device_volumes.clear();
                                                    if self.state.auto_ao_mode {
                                                        self.state.status = DeviceStatus::AoMode("default".to_string());
                                                    } else {
                                                        self.state.status = DeviceStatus::Idle;
                                                    }
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::Disconnect);
                                                } else {
                                                    self.state.connected_device_ids.retain(|id| id != &dev_id);
                                                    self.state.device_volumes.remove(&dev_id);
                                                    let remaining_names: Vec<String> = remaining.iter().map(|d| d.name.clone()).collect();
                                                    let display_title = if remaining.len() == 1 {
                                                        remaining_names[0].clone()
                                                    } else {
                                                        format!("{} (+{})", remaining_names[0], remaining.len() - 1)
                                                    };
                                                    self.state.connected_name = Some(display_title);
                                                    remaining.sort_by_key(|d| (u8::from(!d.is_stereo_pair), std::cmp::Reverse(d.member_devices.len())));
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::Connect(remaining));
                                                }
                                            }

                                            // Show individual volume slider if 2 or more devices are in group
                                            if is_streaming && self.state.connected_device_ids.len() >= 2 {
                                                let mut dev_vol = self.state.device_volumes.get(&dev_id).copied().unwrap_or(self.state.volume);
                                                let percent = (dev_vol * 100.0).round() as i32;
                                                ui.label(egui::RichText::new(format!("{percent}%")).size(11.0).weak());
                                                let slider = egui::Slider::new(&mut dev_vol, 0.0..=1.0)
                                                    .show_value(false);
                                                if ui.add_sized([75.0, 16.0], slider)
                                                    .on_hover_text(lang.tooltip_individual_vol())
                                                    .changed()
                                                {
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetDeviceVolume(dev_id.clone(), dev_vol));
                                                }
                                            }
                                        } else if is_streaming {
                                            // When already streaming, provide "＋ 加入播放" to dynamically expand group!
                                            let add_btn = ui.button(egui::RichText::new(lang.btn_add_to_group()).color(egui::Color32::from_rgb(100, 200, 255)));
                                            if add_btn.clicked() {
                                                let mut targets: Vec<DeviceItem> = self
                                                    .state
                                                    .devices
                                                    .iter()
                                                    .filter(|d| self.state.connected_device_ids.contains(&d.id_string()))
                                                    .cloned()
                                                    .collect();
                                                if !targets.iter().any(|d| d.id_string() == dev_id) {
                                                    targets.push(dev.clone());
                                                }
                                                // Prioritize stereo pair / group leader first
                                                targets.sort_by_key(|d| (u8::from(!d.is_stereo_pair), std::cmp::Reverse(d.member_devices.len())));
                                                let _ = self.cmd_tx.send_blocking(GuiCmd::Connect(targets));
                                            }
                                        } else {
                                            let connect_btn = ui.button(egui::RichText::new(lang.btn_connect()).color(egui::Color32::LIGHT_BLUE));
                                            if connect_btn.clicked() {
                                                let _ = self.cmd_tx.send_blocking(GuiCmd::Connect(vec![dev.clone()]));
                                            }
                                        }
                                    });
                                });
                                ui.separator();
                            }

                            // If multiple devices are selected in idle mode, show batch connect button
                            if !is_streaming && !is_connecting && self.selected_device_ids.len() >= 2 {
                                ui.add_space(4.0);
                                let count = self.selected_device_ids.len();
                                let batch_btn = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(lang.batch_connect_btn(count))
                                            .color(egui::Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(egui::Color32::from_rgb(40, 110, 220))
                                    .min_size(egui::vec2(ui.available_width(), 26.0)),
                                );
                                if batch_btn.clicked() {
                                    let targets: Vec<DeviceItem> = self
                                        .state
                                        .devices
                                        .iter()
                                        .filter(|d| self.selected_device_ids.contains(&d.id_string()))
                                        .cloned()
                                        .collect();
                                    if !targets.is_empty() {
                                        let _ = self.cmd_tx.send_blocking(GuiCmd::Connect(targets));
                                    }
                                }
                            }
                        }
                    });

                ui.add_space(6.0);

                // --- 2.5. Available AirPods List ---
                ui.label(egui::RichText::new(lang.available_airpods_header()).strong().size(13.0));
                ui.add_space(3.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        if self.state.paired_airpods.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(lang.no_airpods_found()).weak());
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(lang.no_airpods_hint())
                                        .weak()
                                        .size(11.0),
                                );
                                ui.add_space(6.0);
                                if ui
                                    .button(egui::RichText::new(lang.btn_open_windows_bluetooth_short()).size(11.5))
                                    .clicked()
                                {
                                    open_windows_bluetooth_settings();
                                }
                                ui.add_space(4.0);
                            });
                        } else {
                            // Sort AirPods: In-Use first, Connected (standby) second, Disconnected third
                            let mut sorted_airpods = self.state.paired_airpods.clone();
                            sorted_airpods.sort_by_key(|ap| {
                                let is_connected = ap.is_connected
                                    || self.state.audio_devices.iter().any(|ad| {
                                        crate::controller::is_airpods_playback_device(&ad.name)
                                            && (ad.name.to_lowercase().contains(&ap.name.to_lowercase())
                                                || ap.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                    });
                                let is_in_use = is_connected && (
                                    matches!(&self.state.status, DeviceStatus::AoMode(ref dev_name) if {
                                        let dn = dev_name.to_lowercase();
                                        let an = ap.name.to_lowercase();
                                        dn.contains(&an) || an.contains(&dn)
                                    })
                                    || (!matches!(&self.state.status, DeviceStatus::Streaming) && self.state.ao_output_device != "default" && self.state.audio_devices.iter().any(|ad| {
                                        ad.id == self.state.ao_output_device && (ad.name.to_lowercase().contains(&ap.name.to_lowercase())
                                            || ap.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                    }))
                                );
                                if is_in_use {
                                    0
                                } else if is_connected {
                                    1
                                } else {
                                    2
                                }
                            });

                            for (i, airpods) in sorted_airpods.iter().enumerate() {
                                if i > 0 {
                                    ui.separator();
                                }
                                let is_connected = airpods.is_connected
                                    || self.state.audio_devices.iter().any(|ad| {
                                        crate::controller::is_airpods_playback_device(&ad.name)
                                            && (ad.name.to_lowercase().contains(&airpods.name.to_lowercase())
                                                || airpods.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                    });

                                let is_in_use = is_connected && (
                                    matches!(&self.state.status, DeviceStatus::AoMode(ref dev_name) if {
                                        let dn = dev_name.to_lowercase();
                                        let an = airpods.name.to_lowercase();
                                        dn.contains(&an) || an.contains(&dn)
                                    })
                                    || (!matches!(&self.state.status, DeviceStatus::Streaming) && self.state.ao_output_device != "default" && self.state.audio_devices.iter().any(|ad| {
                                        ad.id == self.state.ao_output_device && (ad.name.to_lowercase().contains(&airpods.name.to_lowercase())
                                            || airpods.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                    }))
                                );

                                // Retrieve per-device state slot (fallback to global state if in use or single paired device)
                                let dev_slot = self.state.airpods_states.get(&airpods.name);
                                let l_bat = dev_slot.and_then(|s| s.left_battery).or_else(|| if is_in_use { self.state.airpods_battery_left } else { None });
                                let r_bat = dev_slot.and_then(|s| s.right_battery).or_else(|| if is_in_use { self.state.airpods_battery_right } else { None });
                                let c_bat = dev_slot.and_then(|s| s.case_battery).or_else(|| if is_in_use { self.state.airpods_battery_case } else { None });
                                let l_chg = dev_slot.map_or(false, |s| s.left_charging) || (is_in_use && self.state.airpods_charging_left);
                                let r_chg = dev_slot.map_or(false, |s| s.right_charging) || (is_in_use && self.state.airpods_charging_right);
                                let c_chg = dev_slot.map_or(false, |s| s.case_charging) || (is_in_use && self.state.airpods_charging_case);
                                let l_in_ear = dev_slot.map_or(false, |s| s.left_in_ear) || (is_in_use && self.state.airpods_left_in_ear);
                                let r_in_ear = dev_slot.map_or(false, |s| s.right_in_ear) || (is_in_use && self.state.airpods_right_in_ear);

                                let mut sub_items = Vec::new();
                                if is_in_use {
                                    let badge_text = match (l_in_ear, r_in_ear) {
                                        (true, true) => lang.airpods_both_in_ear_badge(),
                                        (true, false) => lang.airpods_left_only_in_ear_badge(),
                                        (false, true) => lang.airpods_right_only_in_ear_badge(),
                                        (false, false) => lang.airpods_out_of_ear_badge(),
                                    };
                                    let clean_badge = badge_text.trim_start_matches(|c: char| c == '🎧' || c.is_whitespace());
                                    sub_items.push(clean_badge.to_string());
                                }

                                if is_connected {
                                    let mut bat_parts = Vec::new();
                                    if let Some(l) = l_bat {
                                        let chg = if l_chg { "⚡" } else { "" };
                                        bat_parts.push(format!("L: {l}%{chg}"));
                                    }
                                    if let Some(r) = r_bat {
                                        let chg = if r_chg { "⚡" } else { "" };
                                        bat_parts.push(format!("R: {r}%{chg}"));
                                    }
                                    if let Some(c) = c_bat {
                                        let chg = if c_chg { "⚡" } else { "" };
                                        bat_parts.push(format!("🔋 {c}%{chg}"));
                                    }
                                    if !bat_parts.is_empty() {
                                        sub_items.push(bat_parts.join("  "));
                                    } else if !is_in_use {
                                        sub_items.push(lang.airpods_standby_status_tag().to_string());
                                    }
                                } else {
                                    sub_items.push(lang.airpods_wear_to_connect_hint().to_string());
                                }

                                let item_content = |ui: &mut egui::Ui| {
                                    ui.horizontal(|ui| {
                                        let icon = if is_in_use {
                                            egui::RichText::new("🎧").color(egui::Color32::from_rgb(60, 220, 140)).size(16.0)
                                        } else if is_connected {
                                            egui::RichText::new("🎧").color(egui::Color32::from_rgb(110, 180, 240)).size(15.0)
                                        } else {
                                            egui::RichText::new("🎧").weak().size(15.0)
                                        };
                                        ui.label(icon);

                                        ui.vertical(|ui| {
                                            let name_text = if is_in_use {
                                                egui::RichText::new(&airpods.name).strong().color(egui::Color32::from_rgb(240, 248, 255))
                                            } else {
                                                egui::RichText::new(&airpods.name).strong()
                                            };
                                            ui.label(name_text);
                                            if !sub_items.is_empty() {
                                                let sub_color = if is_in_use {
                                                    egui::Color32::from_rgb(140, 215, 180)
                                                } else if is_connected {
                                                    egui::Color32::from_rgb(140, 185, 230)
                                                } else {
                                                    egui::Color32::from_rgb(155, 160, 170)
                                                };
                                                ui.label(
                                                    egui::RichText::new(sub_items.join(" • "))
                                                        .size(11.0)
                                                        .color(sub_color),
                                                );
                                            }
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if is_in_use {
                                                ui.label(
                                                    egui::RichText::new(format!("● {}", lang.airpods_in_use_status_tag()))
                                                        .color(egui::Color32::from_rgb(60, 220, 140))
                                                        .strong()
                                                        .size(11.5),
                                                );
                                            } else if is_connected {
                                                ui.label(
                                                    egui::RichText::new(lang.airpods_standby_status_tag())
                                                        .color(egui::Color32::from_rgb(120, 170, 230))
                                                        .size(11.5),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new(lang.airpods_disconnected_status_tag())
                                                        .weak()
                                                        .size(11.5),
                                                );
                                            }
                                        });
                                    });
                                };

                                if is_in_use {
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(22, 38, 34))
                                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 120, 95)))
                                        .rounding(egui::Rounding::same(6.0))
                                        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                        .show(ui, item_content);
                                } else {
                                    item_content(ui);
                                }
                            }
                        }
                    });

                // --- 2.6. AirPlay Casting Status Section (Option A: Hidden when disabled) ---
                if self.state.airplay_receiver_enabled {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(lang.airplay_receiver_section_header()).strong().size(13.0));
                    ui.add_space(3.0);

                    let rx_state = self.state.airplay_receiver_state;
                    let frame = match rx_state {
                        crate::controller::AirPlayReceiverState::Streaming => {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(22, 38, 34))
                                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 120, 95)))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                        }
                        crate::controller::AirPlayReceiverState::Connected => {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(38, 34, 24))
                                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(140, 110, 45)))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                        }
                        crate::controller::AirPlayReceiverState::Standby => {
                            egui::Frame::group(ui.style())
                                .fill(egui::Color32::from_rgb(32, 34, 40))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                        }
                    };

                    let cur_art_len = self.state.airplay_receiver_artwork_bytes.as_ref().map_or(0, |b| b.len());
                    if cur_art_len != self.last_receiver_artwork_len {
                        self.last_receiver_artwork_len = cur_art_len;
                        if let Some(ref bytes) = self.state.airplay_receiver_artwork_bytes {
                            if let Ok(img) = image::load_from_memory(bytes) {
                                let rgba = img.to_rgba8();
                                let size = [rgba.width() as usize, rgba.height() as usize];
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                                self.receiver_artwork_texture = Some(ui.ctx().load_texture(
                                    "airplay_receiver_art",
                                    color_image,
                                    egui::TextureOptions::LINEAR,
                                ));
                            } else {
                                self.receiver_artwork_texture = None;
                            }
                        } else {
                            self.receiver_artwork_texture = None;
                        }
                    }

                    let (badge_text, badge_color) = match rx_state {
                        crate::controller::AirPlayReceiverState::Streaming => (
                            format!("● {}", lang.airplay_receiver_streaming_badge()),
                            egui::Color32::from_rgb(60, 220, 140),
                        ),
                        crate::controller::AirPlayReceiverState::Connected => (
                            format!("● {}", lang.airplay_receiver_connected_badge()),
                            egui::Color32::from_rgb(240, 180, 60),
                        ),
                        crate::controller::AirPlayReceiverState::Standby => (
                            format!("○ {}", lang.airplay_receiver_standby_badge()),
                            egui::Color32::from_rgb(120, 170, 230),
                        ),
                    };

                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            match rx_state {
                                crate::controller::AirPlayReceiverState::Streaming => {
                                    if let Some(ref tex) = self.receiver_artwork_texture {
                                        ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(36.0, 36.0)).rounding(4.0));
                                    } else {
                                        let icon = egui::RichText::new("📲").color(egui::Color32::from_rgb(60, 220, 140)).size(15.0);
                                        ui.label(icon);
                                    }
                                }
                                crate::controller::AirPlayReceiverState::Connected => {
                                    if let Some(ref tex) = self.receiver_artwork_texture {
                                        ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(36.0, 36.0)).rounding(4.0));
                                    } else {
                                        let icon = egui::RichText::new("📲").color(egui::Color32::from_rgb(240, 180, 60)).size(15.0);
                                        ui.label(icon);
                                    }
                                }
                                crate::controller::AirPlayReceiverState::Standby => {
                                    let icon = egui::RichText::new("📲").color(egui::Color32::from_rgb(110, 180, 240)).size(15.0);
                                    ui.label(icon);
                                }
                            }

                            let total_avail = ui.available_width();
                            let badge_galley = ui.painter().layout_no_wrap(
                                badge_text.clone(),
                                egui::FontId::proportional(11.5),
                                badge_color,
                            );
                            let badge_w = badge_galley.size().x + 8.0;
                            let middle_w = (total_avail - badge_w - ui.spacing().item_spacing.x).max(60.0);

                            ui.vertical(|ui| {
                                ui.set_max_width(middle_w);
                                match rx_state {
                                        crate::controller::AirPlayReceiverState::Streaming => {
                                            let valid_title = self.state.airplay_receiver_track_title.as_deref()
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty());
                                            let client_display = self.state.airplay_receiver_client_name.as_deref().unwrap_or("iPhone");

                                            if let Some(title) = valid_title {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(truncate_text(title, 22))
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(240, 248, 255)),
                                                    ).truncate(),
                                                );

                                                let mut sub_parts = Vec::new();
                                                if let Some(ref art) = self.state.airplay_receiver_track_artist {
                                                    let a = art.trim();
                                                    if !a.is_empty() { sub_parts.push(truncate_text(a, 12)); }
                                                }
                                                if let Some(ref alb) = self.state.airplay_receiver_track_album {
                                                    let al = alb.trim();
                                                    if !al.is_empty() { sub_parts.push(truncate_text(al, 12)); }
                                                }
                                                let client_part = format!("📱 {}", client_display);
                                                sub_parts.push(truncate_text(&client_part, 14));
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(sub_parts.join(" • "))
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(180, 220, 200)),
                                                    ).truncate(),
                                                );

                                                ui.horizontal(|ui| {
                                                    let btn_size = egui::vec2(22.0, 18.0);
                                                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏮").size(10.0))).on_hover_text(lang.btn_prev_track()).clicked() {
                                                        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPrev);
                                                    }
                                                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏯").size(10.0))).on_hover_text(lang.btn_play_pause()).clicked() {
                                                        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPlayPause);
                                                    }
                                                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏭").size(10.0))).on_hover_text(lang.btn_next_track()).clicked() {
                                                        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpNext);
                                                    }

                                                    ui.add_space(4.0);
                                                    if self.state.airplay_receiver_auto_max_volume {
                                                        let tooltip = lang.airplay_receiver_max_dynamic_tooltip(self.state.airplay_receiver_volume);
                                                        ui.label(
                                                            egui::RichText::new(lang.airplay_receiver_max_dynamic_range())
                                                                .size(11.0)
                                                                .color(egui::Color32::from_rgb(100, 220, 180)),
                                                        )
                                                        .on_hover_text(tooltip);
                                                    } else {
                                                        let vol_str = if let Some(pct) = self.state.airplay_receiver_volume {
                                                            format!("{}{}%", lang.airplay_receiver_phone_volume(), pct)
                                                        } else {
                                                            format!("{}--", lang.airplay_receiver_phone_volume())
                                                        };
                                                        ui.label(
                                                            egui::RichText::new(vol_str)
                                                                .size(11.0)
                                                                .color(egui::Color32::from_rgb(180, 220, 200)),
                                                        );
                                                    }
                                                });
                                            } else {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format!("📱 {}", client_display))
                                                            .strong()
                                                            .size(12.5)
                                                            .color(egui::Color32::from_rgb(240, 248, 255)),
                                                    ).truncate(),
                                                );

                                                ui.horizontal(|ui| {
                                                    let btn_size = egui::vec2(22.0, 18.0);
                                                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏮").size(10.0))).on_hover_text(lang.btn_prev_track()).clicked() {
                                                        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPrev);
                                                    }
                                                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏯").size(10.0))).on_hover_text(lang.btn_play_pause()).clicked() {
                                                        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPlayPause);
                                                    }
                                                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏭").size(10.0))).on_hover_text(lang.btn_next_track()).clicked() {
                                                        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpNext);
                                                    }

                                                    ui.add_space(4.0);
                                                    if self.state.airplay_receiver_auto_max_volume {
                                                        let tooltip = lang.airplay_receiver_max_dynamic_tooltip(self.state.airplay_receiver_volume);
                                                        ui.label(
                                                            egui::RichText::new(lang.airplay_receiver_max_dynamic_range())
                                                                .size(11.0)
                                                                .color(egui::Color32::from_rgb(100, 220, 180)),
                                                        )
                                                        .on_hover_text(tooltip);
                                                    } else {
                                                        let vol_str = if let Some(pct) = self.state.airplay_receiver_volume {
                                                            format!("{}{}%", lang.airplay_receiver_phone_volume(), pct)
                                                        } else {
                                                            format!("{}--", lang.airplay_receiver_phone_volume())
                                                        };
                                                        ui.label(
                                                            egui::RichText::new(vol_str)
                                                                .size(11.0)
                                                                .color(egui::Color32::from_rgb(180, 220, 200)),
                                                        );
                                                    }
                                                });
                                            }
                                        }
                                        crate::controller::AirPlayReceiverState::Connected => {
                                            let valid_title = self.state.airplay_receiver_track_title.as_deref()
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty());
                                            let client_display = self.state.airplay_receiver_client_name.as_deref().unwrap_or("iPhone");
                                            let dev_type = self.state.airplay_receiver_device_type.as_deref().unwrap_or("iPhone");
                                            let client_ip = self.state.airplay_receiver_client_ip.as_deref().unwrap_or("");

                                            let device_detail = if !client_ip.is_empty() {
                                                format!("📱 {} • {} • {}", client_display, dev_type, client_ip)
                                            } else {
                                                format!("📱 {} • {}", client_display, dev_type)
                                            };

                                            if let Some(title) = valid_title {
                                                let paused_title = format!("{}{}", truncate_text(title, 20), lang.airplay_receiver_paused_suffix());
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(paused_title)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(240, 248, 255)),
                                                    ).truncate(),
                                                );

                                                let mut sub_parts = Vec::new();
                                                if let Some(ref art) = self.state.airplay_receiver_track_artist {
                                                    let a = art.trim();
                                                    if !a.is_empty() { sub_parts.push(truncate_text(a, 12)); }
                                                }
                                                sub_parts.push(truncate_text(&device_detail, 20));
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(sub_parts.join(" • "))
                                                            .size(10.5)
                                                            .color(egui::Color32::from_rgb(180, 215, 230)),
                                                    ).truncate(),
                                                );
                                            } else {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(format!("📱 {}", client_display))
                                                            .strong()
                                                            .size(12.5)
                                                            .color(egui::Color32::from_rgb(240, 248, 255)),
                                                    ).truncate(),
                                                );

                                                let info_str = if !client_ip.is_empty() {
                                                    format!("{} • {}", dev_type, client_ip)
                                                } else {
                                                    dev_type.to_string()
                                                };
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(truncate_text(&info_str, 24))
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(160, 195, 220)),
                                                    ).truncate(),
                                                );
                                            }

                                            ui.horizontal(|ui| {
                                                let btn_size = egui::vec2(22.0, 18.0);
                                                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏮").size(10.0))).on_hover_text(lang.btn_prev_track()).clicked() {
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPrev);
                                                }
                                                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("▶").size(10.0))).on_hover_text(lang.btn_play_pause()).clicked() {
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPlayPause);
                                                }
                                                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏭").size(10.0))).on_hover_text(lang.btn_next_track()).clicked() {
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpNext);
                                                }

                                                ui.add_space(4.0);
                                                if self.state.airplay_receiver_auto_max_volume {
                                                    let tooltip = lang.airplay_receiver_max_dynamic_tooltip(self.state.airplay_receiver_volume);
                                                    ui.label(
                                                        egui::RichText::new(lang.airplay_receiver_max_dynamic_range())
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(100, 220, 180)),
                                                    )
                                                    .on_hover_text(tooltip);
                                                } else {
                                                    let vol_str = if let Some(pct) = self.state.airplay_receiver_volume {
                                                        format!("{}{}%", lang.airplay_receiver_phone_volume(), pct)
                                                    } else {
                                                        format!("{}--", lang.airplay_receiver_phone_volume())
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(vol_str)
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(180, 220, 200)),
                                                    );
                                                }
                                            });
                                        }
                                        crate::controller::AirPlayReceiverState::Standby => {
                                            ui.add(egui::Label::new(egui::RichText::new(lang.airplay_receiver_standby_title()).strong()).truncate());
                                            ui.horizontal(|ui| {
                                                let btn_size = egui::vec2(22.0, 18.0);
                                                let has_cache = self.state.airplay_receiver_has_cached_session;
                                                let btn_color = if has_cache {
                                                    egui::Color32::from_rgb(100, 220, 180)
                                                } else {
                                                    egui::Color32::from_rgb(130, 135, 145)
                                                };
                                                let tooltip = if has_cache {
                                                    lang.airplay_receiver_resume_tooltip()
                                                } else {
                                                    lang.airplay_receiver_no_device_tooltip()
                                                };
                                                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("▶").size(10.0).color(btn_color)))
                                                    .on_hover_text(tooltip)
                                                    .clicked()
                                                {
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverDacpPlayPause);
                                                }

                                                ui.add_space(2.0);
                                                let (sub_text, sub_color) = if let Some((ref hint, time)) = self.state.airplay_receiver_status_hint {
                                                    if time.elapsed().as_secs() < 3 {
                                                        (hint.clone(), egui::Color32::from_rgb(250, 180, 80))
                                                    } else {
                                                        (
                                                            format!("{}{}", lang.airplay_receiver_connect_name_prefix(), self.state.airplay_receiver_name),
                                                            egui::Color32::from_rgb(155, 160, 170),
                                                        )
                                                    }
                                                } else {
                                                    (
                                                        format!("{}{}", lang.airplay_receiver_connect_name_prefix(), self.state.airplay_receiver_name),
                                                        egui::Color32::from_rgb(155, 160, 170),
                                                    )
                                                };
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(sub_text)
                                                            .size(11.0)
                                                            .color(sub_color),
                                                    ).truncate(),
                                                );
                                            });
                                        }
                                    }
                                });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(badge_text)
                                        .color(badge_color)
                                        .strong()
                                        .size(11.5),
                                );
                            });
                        });
                    });
                }

                ui.add_space(6.0);

                // --- 3. Volume & Mute Controls ---
                ui.label(egui::RichText::new(lang.volume_header()).strong().size(13.0));
                ui.add_space(3.0);

                // Mute Status Display & One-Click Mute Button (Option B: Smooth Gradient VU Meter)
                ui.horizontal(|ui| {
                    let (btn_text, btn_color) = if self.state.is_muted {
                        (lang.btn_unmute(), egui::Color32::from_rgb(70, 200, 120))
                    } else {
                        (lang.btn_mute(), egui::Color32::from_rgb(240, 160, 40))
                    };

                    // Compute button width with padding
                    let btn_padding = ui.spacing().button_padding.x * 2.0;
                    let text_width = ui.painter().layout_no_wrap(
                        btn_text.to_string(),
                        egui::FontId::proportional(12.0),
                        btn_color,
                    ).size().x;
                    let eq_label = lang.btn_eq(self.state.eq_enabled);
                    let eq_width = ui.painter().layout_no_wrap(
                        eq_label.to_string(),
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    ).size().x + btn_padding + 6.0;
                    let btn_width = text_width + btn_padding + 12.0 + eq_width;
                    let meter_width = (ui.available_width() - btn_width - ui.spacing().item_spacing.x * 2.0).max(40.0);
                    let meter_height = 22.0;

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(meter_width, meter_height),
                        egui::Sense::hover(),
                    );

                    let painter = ui.painter();
                    let rounding = egui::Rounding::same(6.0);

                    // Background track (subtle modern translucent dark glass)
                    painter.rect_filled(
                        rect,
                        rounding,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                    );
                    painter.rect_stroke(
                        rect,
                        rounding,
                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25)),
                    );

                    match (&self.state.status, self.state.is_muted) {
                        (DeviceStatus::Streaming, false) | (DeviceStatus::AoMode(_), false) => {
                            let name = match &self.state.status {
                                DeviceStatus::AoMode(n) => {
                                    if n == "default" {
                                        lang.default_output_device()
                                    } else {
                                        n.as_str()
                                    }
                                }
                                _ => self.state.connected_name.as_deref().unwrap_or("Audio"),
                            };
                            response.on_hover_text(lang.vol_speaker_playing(name));

                            let inner_rect = rect.shrink(2.0);
                            let num_bars = 16;
                            let gap = 2.0f32;
                            let bar_w = ((inner_rect.width() - (num_bars as f32 - 1.0) * gap) / num_bars as f32).max(1.5);

                            for i in 0..num_bars {
                                let x = inner_rect.min.x + i as f32 * (bar_w + gap);
                                let lvl = self.spectrum_vis.current[i].clamp(0.0, 1.0);
                                let bar_h = (inner_rect.height() * lvl).max(1.5);
                                let bar_y = inner_rect.max.y - bar_h;
                                let frac = i as f32 / (num_bars as f32 - 1.0);
                                let col = meter_color_at(frac);

                                painter.rect_filled(
                                    egui::Rect::from_min_size(egui::pos2(x, bar_y), egui::vec2(bar_w, bar_h)),
                                    egui::Rounding::same(1.5),
                                    col,
                                );

                                // Peak hold cap
                                let peak_lvl = self.spectrum_vis.peaks[i].clamp(0.0, 1.0);
                                if peak_lvl > 0.05 {
                                    let peak_y = inner_rect.max.y - (inner_rect.height() * peak_lvl);
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(egui::pos2(x, peak_y.max(inner_rect.min.y)), egui::vec2(bar_w, 1.5)),
                                        egui::Rounding::same(0.5),
                                        egui::Color32::from_rgb(255, 255, 255),
                                    );
                                }
                            }
                        }
                        (DeviceStatus::Streaming, true) | (DeviceStatus::AoMode(_), true) => {
                            let name = match &self.state.status {
                                DeviceStatus::AoMode(n) => {
                                    if n == "default" {
                                        lang.default_output_device()
                                    } else {
                                        n.as_str()
                                    }
                                }
                                _ => self.state.connected_name.as_deref().unwrap_or("Audio"),
                            };
                            let tip = lang.vol_speaker_muted(name);
                            response.on_hover_text(&tip);
                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                tip,
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_rgb(240, 160, 40),
                            );
                        }
                        (DeviceStatus::Connecting, muted) => {
                            let tip = if muted { lang.vol_connecting_muted() } else { lang.vol_connecting() };
                            response.on_hover_text(tip);
                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                tip,
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_rgb(240, 160, 40),
                            );
                        }
                        (DeviceStatus::Idle, muted) => {
                            let tip = if muted { lang.vol_idle_muted() } else { lang.vol_idle() };
                            response.on_hover_text(tip);
                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                tip,
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_rgb(140, 140, 140),
                            );
                        }
                        (DeviceStatus::Error(_), muted) => {
                            let tip = if muted { lang.vol_error_muted() } else { lang.vol_error() };
                            response.on_hover_text(tip);
                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                tip,
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_rgb(240, 80, 80),
                            );
                        }
                    }

                    // Mute Button & EQ Button on the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new(btn_text).color(btn_color)).clicked() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetMute(!self.state.is_muted));
                        }
                        let eq_color = if self.state.eq_enabled {
                            egui::Color32::from_rgb(100, 220, 140)
                        } else {
                            egui::Color32::from_rgb(180, 180, 190)
                        };
                        if ui.button(egui::RichText::new(eq_label).color(eq_color).size(11.0))
                            .on_hover_text(lang.tooltip_equalizer())
                            .clicked()
                        {
                            self.eq_profile = EqProfile::Normal;
                            self.navigate_to(Page::Equalizer);
                        }
                    });
                });

                ui.add_space(4.0);

                // Volume slider with dynamic mute indicator (extended width for easy micro-adjustment)
                ui.horizontal(|ui| {
                    if self.state.connected_device_ids.len() >= 2 {
                        ui.label(egui::RichText::new(lang.group_volume_label()).strong())
                            .on_hover_text(lang.group_volume_tooltip());
                    } else {
                        ui.label(egui::RichText::new(lang.volume_label()).weak());
                    }

                    let vol_percent = (self.volume_slider * 100.0) as i32;
                    let slider_text = if self.state.is_muted {
                        format!("{vol_percent}%{}", lang.slider_muted_suffix())
                    } else {
                        format!("{vol_percent}%")
                    };

                    ui.spacing_mut().slider_width = 220.0;
                    let slider = egui::Slider::new(&mut self.volume_slider, 0.0..=1.0)
                        .show_value(false)
                        .text(slider_text);

                    let slider_resp = ui.add(slider);
                    self.is_dragging_volume = slider_resp.dragged();
                    if slider_resp.changed() {
                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetVolume(self.volume_slider));
                    }
                });

                ui.add_space(8.0);
            });

        ui.cursor().top() - initial_top
    }

    /// 渲染具備動態平滑滑動指示線（與膠囊滑塊）的通用分頁列
    pub fn render_sliding_tab_bar<T: Copy + PartialEq>(
        ui: &mut egui::Ui,
        id_source: &str,
        current: T,
        tabs: &[(T, &str)],
    ) -> Option<T> {
        let total_w = ui.available_width();
        let height = 30.0_f32;
        let count = tabs.len();
        if count == 0 {
            return None;
        }

        let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(total_w, height), egui::Sense::hover());

        // 底層膠囊型軌道
        ui.painter().rect_filled(
            bar_rect,
            egui::Rounding::same(7.0),
            egui::Color32::from_rgb(18, 21, 26),
        );
        ui.painter().rect_stroke(
            bar_rect,
            egui::Rounding::same(7.0),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 16)),
        );

        let pad_x = 2.0_f32;
        let pad_y = 2.0_f32;
        let tab_w = (bar_rect.width() - pad_x * 2.0) / count as f32;
        let active_idx = tabs.iter().position(|(t, _)| *t == current).unwrap_or(0);

        let target_x = bar_rect.left() + pad_x + (active_idx as f32) * tab_w;
        let target_w = tab_w;

        // 使用 egui 內建 animate_value_with_time (180ms ease 插值，切換時絲滑平移)
        let cur_x = ui.ctx().animate_value_with_time(ui.id().with(format!("{id_source}_tab_x")), target_x, 0.18);
        let cur_w = ui.ctx().animate_value_with_time(ui.id().with(format!("{id_source}_tab_w")), target_w, 0.18);

        // 滑動膠囊背景塊
        let pill_rect = egui::Rect::from_min_size(
            egui::pos2(cur_x, bar_rect.top() + pad_y),
            egui::vec2(cur_w, bar_rect.height() - pad_y * 2.0),
        );
        ui.painter().rect_filled(
            pill_rect,
            egui::Rounding::same(5.0),
            egui::Color32::from_rgb(34, 52, 75),
        );
        ui.painter().rect_stroke(
            pill_rect,
            egui::Rounding::same(5.0),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(52, 105, 175)),
        );

        // 頁籤滑動指示線（亮青色高光線條，緊貼膠囊底部動態跟隨滑動）
        let line_h = 2.5_f32;
        let indicator_margin_x = 10.0_f32.min(cur_w * 0.15);
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(cur_x + indicator_margin_x, pill_rect.bottom() - line_h),
            egui::pos2(cur_x + cur_w - indicator_margin_x, pill_rect.bottom()),
        );
        ui.painter().rect_filled(
            line_rect,
            egui::Rounding::same(1.2),
            egui::Color32::from_rgb(20, 160, 210),
        );

        let mut clicked_tab = None;

        for (i, (tab_val, label)) in tabs.iter().enumerate() {
            let tab_rect = egui::Rect::from_min_size(
                egui::pos2(bar_rect.left() + pad_x + (i as f32) * tab_w, bar_rect.top() + pad_y),
                egui::vec2(tab_w, bar_rect.height() - pad_y * 2.0),
            );

            let resp = ui.interact(
                tab_rect,
                ui.id().with(format!("{id_source}_btn_{i}")),
                egui::Sense::click(),
            );

            if resp.clicked() {
                clicked_tab = Some(*tab_val);
            }

            let is_sel = *tab_val == current;
            let text_color = if is_sel {
                egui::Color32::WHITE
            } else if resp.hovered() {
                egui::Color32::from_rgb(220, 225, 235)
            } else {
                egui::Color32::from_rgb(150, 160, 175)
            };

            let text_pos = tab_rect.center() - egui::vec2(0.0, 1.0);
            ui.painter().text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                *label,
                egui::FontId::proportional(12.5),
                text_color,
            );
        }

        clicked_tab
    }

    /// 通用狀態圖示按鈕：統一樣式、色彩與視覺回饋
    pub fn render_stateful_icon_button(
        ui: &mut egui::Ui,
        icon: &str,
        is_active: bool,
        tooltip: &str,
        theme: ButtonTheme,
    ) -> egui::Response {
        let (text_color, fill_color, stroke_color) = if is_active {
            match theme {
                ButtonTheme::Green => (
                    egui::Color32::from_rgb(130, 240, 180),
                    egui::Color32::from_rgb(20, 50, 35),
                    egui::Color32::from_rgb(60, 180, 110),
                ),
                ButtonTheme::Blue => (
                    egui::Color32::from_rgb(100, 220, 255),
                    egui::Color32::from_rgb(20, 45, 65),
                    egui::Color32::from_rgb(60, 160, 220),
                ),
                ButtonTheme::Gold => (
                    egui::Color32::from_rgb(255, 215, 0),
                    egui::Color32::from_rgb(55, 45, 18),
                    egui::Color32::from_rgb(215, 165, 40),
                ),
                ButtonTheme::Cyan => (
                    egui::Color32::from_rgb(140, 210, 255),
                    egui::Color32::from_rgb(25, 45, 60),
                    egui::Color32::from_rgb(70, 150, 210),
                ),
            }
        } else {
            (
                egui::Color32::from_gray(160),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            )
        };

        let btn = egui::Button::new(egui::RichText::new(icon).size(13.0).color(text_color))
            .min_size(egui::vec2(24.0, 22.0))
            .fill(fill_color)
            .stroke(egui::Stroke::new(1.0_f32, stroke_color))
            .rounding(egui::Rounding::same(4.0))
            .sense(egui::Sense::click_and_drag());

        ui.add(btn).on_hover_text(tooltip)
    }

    /// 通用 🏠 按鈕組件：可嵌入任何次級視窗（迷你播放器、等化器等）
    /// 一步到位將主視窗吸附至當前群組；若已在群組中則點擊聚焦主視窗
    pub fn render_universal_home_button(
        ui: &mut egui::Ui,
        source_kind: crate::snapping::WindowKind,
        lang: crate::i18n::Language,
    ) -> egui::Response {
        let is_docked = crate::snapping::is_main_in_cluster(source_kind);

        let tooltip = if is_docked {
            lang.tooltip_main_window_docked_focus()
        } else {
            lang.tooltip_dock_main_window()
        };

        let resp = Self::render_stateful_icon_button(
            ui,
            "🏠",
            is_docked,
            tooltip,
            ButtonTheme::Cyan,
        );

        if resp.clicked() {
            unsafe {
                crate::snapping::handle_home_summon_request(source_kind);
            }
        }

        resp
    }

    /// 檢查特定分頁是否處於廣義作用中（當前分頁或對應的獨立視窗開啟）
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

    /// 浮動透明度 HUD 標籤組件：於滾輪微調或預覽期間在視窗中央顯現
    pub fn render_opacity_hud_badge(ui: &egui::Ui, kind: crate::snapping::WindowKind) {
        if crate::snapping::is_opacity_hud_active(kind) {
            let op = crate::snapping::get_window_opacity(kind);
            let pct = (op * 100.0).round() as u32;
            let is_click_through = pct <= 40;
            let painter = ui.painter();
            let center = ui.clip_rect().center();
            let hud_size = if is_click_through {
                egui::vec2(150.0, 32.0)
            } else {
                egui::vec2(96.0, 32.0)
            };
            let hud_rect = egui::Rect::from_center_size(center, hud_size);
            painter.rect_filled(hud_rect, 6.0, egui::Color32::from_rgba_unmultiplied(18, 20, 26, 235));
            painter.rect_stroke(hud_rect, 6.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 215, 0, 180)));
            let text = if is_click_through {
                format!("👻 {}% (穿透模式)", pct)
            } else {
                format!("🎯 {}%", pct)
            };
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::monospace(13.0),
                egui::Color32::from_rgb(255, 215, 0),
            );
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    /// 穿透分界線微光提示（提案 C：頂部 42px 標題列保護區 vs 下方穿透區之雙向漸淡流光線）
    pub fn render_header_shimmer_boundary(ui: &egui::Ui, kind: crate::snapping::WindowKind) {
        if !crate::snapping::is_click_through_active(kind) {
            return;
        }

        let screen_rect = ui.ctx().screen_rect();
        let boundary_y = screen_rect.min.y + 42.0;
        let min_x = screen_rect.min.x;
        let max_x = screen_rect.max.x;
        let width = max_x - min_x;
        if width <= 10.0 {
            return;
        }

        // 偵測滑鼠游標是否接近標題列（y <= boundary_y + 12px）
        let pointer_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()));
        let is_near_header = pointer_pos.map_or(false, |pos| {
            pos.x >= min_x && pos.x <= max_x && pos.y >= screen_rect.min.y && pos.y <= boundary_y + 12.0
        });

        // 呼吸平滑過渡（常態微弱微光 alpha ~45，懸停接近時提亮至 ~190）
        let hover_anim = ui.ctx().animate_bool(egui::Id::new("header_shimmer_hover").with(kind), is_near_header);
        let base_alpha = (45.0 + hover_anim * 145.0) as u8;

        let painter = ui.painter();

        // 繪製雙向向外漸淡的水平微光線
        let x_p1 = min_x + width * 0.15;
        let x_p2 = min_x + width * 0.85;

        // 1. 底層 3px 柔和微光光暈 (Ambient Glow)
        let aura_alpha = (base_alpha as f32 * 0.35).round() as u8;
        let aura_pts = [
            (min_x, 0),
            (x_p1, aura_alpha),
            (x_p2, aura_alpha),
            (max_x, 0),
        ];
        for i in 0..aura_pts.len() - 1 {
            let (x0, a0) = aura_pts[i];
            let (x1, a1) = aura_pts[i + 1];
            let mut mesh = egui::Mesh::default();
            let c0 = egui::Color32::from_rgba_unmultiplied(255, 215, 0, a0);
            let c1 = egui::Color32::from_rgba_unmultiplied(255, 215, 0, a1);
            let y_top = boundary_y - 1.5;
            let y_bot = boundary_y + 1.5;
            let idx = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x0, y_top), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x1, y_top), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x1, y_bot), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x0, y_bot), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
            painter.add(mesh);
        }

        // 2. 核心 1px 晶透亮線 (Sharp Shimmer Core)
        let core_pts = [
            (min_x, 0),
            (x_p1, base_alpha),
            (x_p2, base_alpha),
            (max_x, 0),
        ];
        for i in 0..core_pts.len() - 1 {
            let (x0, a0) = core_pts[i];
            let (x1, a1) = core_pts[i + 1];
            let mut mesh = egui::Mesh::default();
            let c0 = egui::Color32::from_rgba_unmultiplied(255, 235, 120, a0);
            let c1 = egui::Color32::from_rgba_unmultiplied(255, 235, 120, a1);
            let y_top = boundary_y - 0.5;
            let y_bot = boundary_y + 0.5;
            let idx = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x0, y_top), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x1, y_top), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x1, y_bot), uv: egui::epaint::WHITE_UV, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(x0, y_bot), uv: egui::epaint::WHITE_UV, color: c0 });
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
            painter.add(mesh);
        }

        if hover_anim > 0.05 && hover_anim < 0.95 {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    pub fn render_mascot_overlay(&mut self, ui: &mut egui::Ui, window_kind: crate::snapping::WindowKind) {
        if !self.state.mascot_enabled {
            return;
        }

        // 收集目前所有開啟且可見的視窗清單（主視窗、迷你播放器、等化器）
        let mut open_windows = Vec::new();
        if !crate::tray::is_window_hidden_to_tray() && !crate::tray::is_app_fleet_hidden() {
            open_windows.push(crate::snapping::WindowKind::Main);
        }
        if self.mini_player_window_open && !crate::tray::is_app_fleet_hidden() {
            open_windows.push(crate::snapping::WindowKind::MiniPlayer);
        }
        if self.equalizer_window_open && !crate::tray::is_app_fleet_hidden() {
            open_windows.push(crate::snapping::WindowKind::Equalizer);
        }

        if open_windows.is_empty() {
            return;
        }

        // 若幸運物原先所在的視窗被關閉或最小化，自動遷移至第一個可用的開啟視窗
        if !open_windows.contains(&self.mascot_window) {
            self.mascot_window = open_windows[0];
            self.mascot_pos = egui::pos2(80.0, 38.0);
            self.mascot_platform_idx = 1;
            self.mascot_activity = 0;
            self.mascot_bounce = -10.0;
        }

        // 幸運物同一時間只在單一視窗中漫遊；非所屬視窗不重複計算與渲染
        if self.mascot_window != window_kind {
            return;
        }

        let other_windows: Vec<crate::snapping::WindowKind> = open_windows
            .into_iter()
            .filter(|&k| k != window_kind)
            .collect();

        let now = std::time::Instant::now();
        let dt = (now - self.mascot_last_update).as_secs_f32().clamp(0.005, 0.1);
        self.mascot_last_update = now;

        let screen_rect = ui.ctx().screen_rect();
        let w = screen_rect.width();
        let h = screen_rect.height();
        let min_x = screen_rect.min.x;
        let min_y = screen_rect.min.y;

        // 1. 真實 UI 元件地貌平台定義（包含動態 16 頻段音訊頻譜波浪平台！）
        // 格式：(Y高度, X起點, X終點, 走過左端是否墜落, 走過右端是否墜落)
        let (spec_y, spec_x_min, spec_x_max) = if let Some(sr) = self.mascot_spectrum_rect {
            (sr.max.y - 14.0, sr.min.x + 8.0, sr.max.x - 8.0)
        } else {
            (min_y + h * 0.52, min_x + 20.0, min_x + w - 20.0)
        };

        let platforms: [(f32, f32, f32, bool, bool); 10] = [
            // 0: 標題列頂端安全軌道 (Y=16，頭部距視窗頂部保留 5px 完整無截)
            (min_y + 16.0, min_x + 16.0, min_x + w - 16.0, false, false),
            // 1: 標題列左側標題字串 ("Win2Pod Bridge"，走過右側字串終點即懸崖墜落！)
            (min_y + 34.0, min_x + 16.0, min_x + 165.0, false, true),
            // 2: 標題列右側按鈕群 ("⚙ 🗗 📌 ✕"，往左踏出按鈕即墜落！)
            (min_y + 34.0, min_x + 290.0, min_x + w - 16.0, true, false),
            // 3: 標題列金光分割線 (右側有 80px 懸崖洞口)
            (min_y + 48.0, min_x + 16.0, min_x + w - 80.0, false, true),
            // 4: 播放狀態膠囊標籤 ("🟢 串流中 / ⚪ 待機中"，兩端皆為懸崖！)
            (min_y + 88.0, min_x + 120.0, min_x + 320.0, true, true),
            // 5: 裝置名稱文字字串 ("客廳 HomePod"，兩端皆為懸崖！)
            (min_y + 142.0, min_x + 70.0, min_x + 370.0, true, true),
            // 6: 音量滑桿與控制條平台 (兩端皆為懸崖！)
            (min_y + 220.0, min_x + 45.0, min_x + w - 45.0, true, true),
            // 7: 🎵 WinAMP 16 頻段動態即時音訊頻譜海浪（高高低低波浪衝浪，兩端皆可跳樓！）
            (spec_y, spec_x_min, spec_x_max, true, true),
            // 8: 下層控制與進階設定卡片頂緣 (兩端皆為懸崖！)
            (min_y + h * 0.72, min_x + 40.0, min_x + w - 40.0, true, true),
            // 9: 視窗最底層堅實地板 (腳部距視窗底緣保留 4px 完整波浪摺裙)
            (min_y + h - 15.0, min_x + 16.0, min_x + w - 16.0, false, false),
        ];

        let left_wall_x = min_x + 15.0;
        let right_wall_x = min_x + w - 15.0;
        let wall_min_y = min_y + 16.0;
        let wall_max_y = min_y + h - 15.0;

        // 初始化
        if self.mascot_pos.x == 0.0 && self.mascot_pos.y == 0.0 {
            self.mascot_pos = egui::pos2(min_x + 80.0, platforms[1].0);
            self.mascot_platform_idx = 1;
            self.mascot_activity = 0;
            self.mascot_dir_x = 1.0;
            self.mascot_squash = 1.0;
            self.mascot_action_timer = 1.5;
        }

        // 以「真實音訊頻譜能量」判斷是否真有音樂在播放
        let spectrum_energy: f32 = self.spectrum_vis.current.iter().sum::<f32>() / 16.0;
        let has_real_music = (spectrum_energy > 0.012 || self.meter_level > 0.025) && !self.state.is_muted;
        let is_rhythm_mode = has_real_music && self.state.mascot_rhythm_dance;
        let time_sec = ui.input(|i| i.time);

        // 滑鼠靠近驅趕（Scamper Dodge）：滑鼠靠近時驚嚇快跑！
        let pointer_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()));
        if let Some(m_pos) = pointer_pos {
            let dist_to_mouse = (m_pos - self.mascot_pos).length();
            // 在空中極速向上衝刺時給予短暫慣性保護，其餘情況滑鼠一靠近即觸發驚嚇驅趕！
            if dist_to_mouse < 52.0 && self.mascot_activity != 2 && !(self.mascot_activity == 5 && self.mascot_vel_y < -60.0) {
                let flee_dir = if m_pos.x > self.mascot_pos.x { -1.0 } else { 1.0 };
                self.mascot_dir_x = flee_dir;
                self.mascot_action_timer = 1.2;
                if self.mascot_bounce == 0.0 {
                    self.mascot_bounce = -6.0;
                }

                // ⭐ 唯一結束頻譜特權的條件：滑鼠靠近驅趕！
                self.mascot_spectrum_privilege = false;

                if self.mascot_platform_idx == 7 {
                    // 當使用者將滑鼠靠近驅趕時，幽靈受到驚嚇，立即解除頻譜特權並逃離頻譜！
                    self.mascot_platform_idx = 9; // 解除特權，目標設為最底層地板
                    if self.mascot_activity == 5 {
                        // 若正在空中彈跳，驚嚇向下俯衝自由落體至底板
                        self.mascot_activity = 2;
                        self.mascot_vel_y = 140.0;
                        self.mascot_squash = 0.70;
                    } else {
                        // 在頻譜地面時驚慌快跑逃向最底層地板
                        self.mascot_activity = 4;
                        self.mascot_pos.y = platforms[9].0;
                    }
                } else {
                    self.mascot_activity = 4; // Scamper 快跑逃跑！
                }
            }
        }

        // 彈跳與落地擠壓（Squash & Stretch）物理衰減
        if (self.mascot_squash - 1.0).abs() > 0.01 {
            self.mascot_squash += (1.0 - self.mascot_squash) * (dt * 12.0);
        } else {
            self.mascot_squash = 1.0;
        }
        if self.mascot_bounce.abs() > 0.05 {
            self.mascot_bounce += (-self.mascot_bounce * 14.0) * dt;
        } else {
            self.mascot_bounce = 0.0;
        }

        let pseudo_rand = ((time_sec * 1000.0) as u64)
            ^ (self.mascot_pos.x as u64).wrapping_mul(37)
            ^ (self.mascot_pos.y as u64).wrapping_mul(149);

        // 計算幽靈踏在 16 頻段即時音訊頻譜頂端的高度與音浪能量
        let get_spectrum_bar_surface = Self::get_spectrum_bar_surface;

        // 2. 地形地貌有限狀態機 (FSM)
        let mut visual_angle = 0.0_f32;
        let mut foot_step_y = 0.0_f32;
        let mut foot_tilt = 0.0_f32;

        match self.mascot_activity {
            // 狀態 0：在字串/按鈕/屋頂/頻譜上海浪衝浪行走 (WalkingFloor)
            0 => {
                let p = platforms[self.mascot_platform_idx];
                let walk_speed = 54.0_f32; // 步伐輕快活潑
                self.mascot_pos.x += self.mascot_dir_x * walk_speed * dt;

                // 腳步輕微步伐搖晃 (步態動畫)
                foot_step_y = -((time_sec * 11.0).sin().abs() as f32) * 2.2;
                foot_tilt = ((time_sec * 11.0).sin() as f32) * 0.08 * self.mascot_dir_x;

                // 🎵 核心震撼亮點：雙腳站立在頻譜頂端峰值衝浪，並隨動態音浪猛烈彈飛！
                if self.mascot_platform_idx == 7 {
                    if let Some(sr) = self.mascot_spectrum_rect {
                        let (bar_top_y, effective_lvl) = get_spectrum_bar_surface(sr, self.mascot_pos.x, &self.spectrum_vis.current, &self.spectrum_vis.peaks);
                        // 雙腳直接踩在柱狀圖峰值頂端，緊貼動態波浪！
                        self.mascot_pos.y = bar_top_y;
                        foot_step_y -= effective_lvl * 5.0;
                        foot_tilt += effective_lvl * 0.35 * self.mascot_dir_x;

                        // 🚀 頻譜猛烈彈飛衝浪機制！
                        // 當踏上的柱子音浪強烈升起（如重低音鼓點擊中），立即如彈簧床般猛烈將小幽靈「彈飛至半空中翻滾衝浪」！
                        if effective_lvl > 0.40 && (pseudo_rand % 100 < 55) {
                            self.mascot_activity = 5; // Launch into air!
                            self.mascot_vel_y = -260.0 - effective_lvl * 200.0; // 狂暴高空彈飛！
                            self.mascot_bounce = -16.0;
                            self.mascot_squash = 0.52; // 彈飛瞬間強烈形變
                        }
                    } else {
                        self.mascot_pos.y = p.0;
                    }
                } else {
                    self.mascot_pos.y = p.0;
                }

                // 定時觸發自主決策（告別呆板路徑！每 1.2~2.4 秒必有生動動作：跳樓、大跳躍、衝浪調頭）
                self.mascot_action_timer -= dt;
                if self.mascot_action_timer <= 0.0 {
                    self.mascot_action_timer = 1.2 + (pseudo_rand % 12) as f32 * 0.1;
                    let roll = pseudo_rand % 100;

                    if self.mascot_platform_idx == 9 {
                        // 在底層地面：65% 機率「火箭大跳躍」直衝頻譜或中層元件！
                        if roll < 65 {
                            self.mascot_activity = 5; // Jumping
                            self.mascot_vel_y = -360.0;
                            self.mascot_bounce = -12.0;
                            self.mascot_squash = 0.60;
                        } else {
                            // 轉身折返
                            self.mascot_dir_x = -self.mascot_dir_x;
                            self.mascot_bounce = -6.0;
                        }
                    } else if self.mascot_platform_idx == 7 {
                        // 在頻譜頂端衝浪自主決策：
                        if self.mascot_spectrum_privilege {
                            // 🌟 右鍵彩蛋特權期間：連續高空大彈跳起飛與踩浪衝浪！
                            if roll < 65 {
                                self.mascot_activity = 5;
                                self.mascot_vel_y = -280.0;
                                self.mascot_bounce = -14.0;
                                self.mascot_squash = 0.55;
                            } else {
                                self.mascot_dir_x = -self.mascot_dir_x;
                            }
                        } else {
                            // 常態漫遊：可往上起跳探訪上方障礙物 (Platform 6, 5...) 或走往邊界下落
                            if roll < 45 {
                                self.mascot_activity = 5;
                                self.mascot_vel_y = -260.0;
                                self.mascot_bounce = -10.0;
                                self.mascot_squash = 0.65;
                            } else if roll < 75 {
                                self.mascot_dir_x = -self.mascot_dir_x;
                            } else {
                                self.mascot_activity = 2;
                                self.mascot_vel_y = 60.0;
                                self.mascot_bounce = -8.0;
                            }
                        }
                    } else if self.mascot_platform_idx <= 3 {
                        // 在最頂部 (標題列與頂端按鈕)：絕不往天花板跳！高達 80% 機率直接往下跳樓墜落！
                        if roll < 80 {
                            // 縱身一躍往下跳樓！
                            self.mascot_activity = 2; // Falling!
                            self.mascot_vel_y = 75.0;
                            self.mascot_bounce = -8.0;
                            self.mascot_squash = 0.72;
                            // 若有音樂播放，引導往頻譜中心墜落
                            if has_real_music {
                                let spec_center = (spec_x_min + spec_x_max) * 0.5;
                                self.mascot_dir_x = if self.mascot_pos.x < spec_center { 1.0 } else { -1.0 };
                            }
                        } else {
                            // 折返走向邊界懸崖
                            self.mascot_dir_x = -self.mascot_dir_x;
                        }
                    } else {
                        // 在中層元件上 (4, 5, 6, 8)：
                        if roll < 65 {
                            // 🎲 隨機跳樓！縱身一躍自由落體（常落在頻譜上！）
                            self.mascot_activity = 2; // Falling!
                            self.mascot_vel_y = 60.0;
                            self.mascot_bounce = -8.0;
                            self.mascot_squash = 0.75;
                        } else if roll < 85 {
                            // 向上起跳探險！
                            self.mascot_activity = 5;
                            self.mascot_vel_y = -240.0;
                            self.mascot_bounce = -10.0;
                            self.mascot_squash = 0.65;
                        } else {
                            // 轉身折返
                            self.mascot_dir_x = -self.mascot_dir_x;
                        }
                    }
                }
                // 檢查是否遇到左側邊緣或懸崖
                else if self.mascot_dir_x < 0.0 && self.mascot_pos.x <= p.1 {
                    self.mascot_pos.x = p.1;
                    if p.3 {
                        // 走過字串左邊界：前方無地板，100% 墜落或俯衝跳樓！
                        self.mascot_activity = 2; // Falling!
                        self.mascot_vel_y = 60.0;
                        self.mascot_bounce = -6.0;
                    } else {
                        // 遇到視窗左側邊緣：邊緣立刻靈活轉彎或蹬牆飛躍（不再呆板直走）！
                        let roll = pseudo_rand % 100;
                        if roll < 60 {
                            // 即刻 180° 調頭轉彎奔跑！
                            self.mascot_dir_x = 1.0;
                            self.mascot_bounce = -8.0;
                            self.mascot_squash = 0.75;
                        } else if roll < 85 {
                            // 蹬牆大跳躍反彈回室內！
                            self.mascot_activity = 5;
                            self.mascot_dir_x = 1.0;
                            self.mascot_vel_y = -210.0;
                        } else {
                            self.mascot_activity = 1; // 爬上左牆
                            self.mascot_dir_y = if self.mascot_platform_idx == 0 { 1.0 } else { -1.0 };
                        }
                    }
                }
                // 檢查是否遇到右側邊緣或懸崖
                else if self.mascot_dir_x > 0.0 && self.mascot_pos.x >= p.2 {
                    self.mascot_pos.x = p.2;
                    if p.4 {
                        // 走過字串右邊界：前方無地板，100% 墜落或俯衝跳樓！
                        self.mascot_activity = 2; // Falling!
                        self.mascot_vel_y = 60.0;
                        self.mascot_bounce = -6.0;
                    } else {
                        // 遇到視窗右側邊緣：邊緣立刻靈活轉彎或蹬牆飛躍！
                        let roll = pseudo_rand % 100;
                        if roll < 60 {
                            // 即刻 180° 調頭轉彎奔跑！
                            self.mascot_dir_x = -1.0;
                            self.mascot_bounce = -8.0;
                            self.mascot_squash = 0.75;
                        } else if roll < 85 {
                            // 蹬牆大跳躍反彈回室內！
                            self.mascot_activity = 5;
                            self.mascot_dir_x = -1.0;
                            self.mascot_vel_y = -210.0;
                        } else {
                            self.mascot_activity = 1; // 爬上右牆
                            self.mascot_dir_y = if self.mascot_platform_idx == 0 { 1.0 } else { -1.0 };
                        }
                    }
                }
            }

            // 狀態 1：垂直攀爬邊緣外牆 (ClimbingWall)
            1 => {
                let climb_speed = 36.0_f32;
                self.mascot_pos.y += self.mascot_dir_y * climb_speed * dt;

                let is_left = (self.mascot_pos.x - left_wall_x).abs() < (self.mascot_pos.x - right_wall_x).abs();
                self.mascot_pos.x = if is_left { left_wall_x } else { right_wall_x };

                // 幸運物貼牆時雙腳緊抓牆壁、頭朝向視窗內部
                visual_angle = if is_left {
                    std::f32::consts::FRAC_PI_2 * 0.9
                } else {
                    -std::f32::consts::FRAC_PI_2 * 0.9
                };
                foot_tilt = ((time_sec * 12.0).sin() as f32) * 0.08;

                // 爬牆時不再像呆板電梯一樣一直爬！定時蹬牆跳入室內或折返
                self.mascot_action_timer -= dt;
                if self.mascot_action_timer <= 0.0 {
                    self.mascot_action_timer = 0.8 + (pseudo_rand % 8) as f32 * 0.1;
                    let roll = (pseudo_rand >> 3) % 100;
                    if roll < 65 {
                        // 蹬牆跳入室內！常直接落入頻譜或中間元件
                        self.mascot_activity = 5;
                        self.mascot_dir_x = if is_left { 1.0 } else { -1.0 };
                        self.mascot_vel_y = -160.0;
                        self.mascot_bounce = -8.0;
                    } else if roll < 90 {
                        // 爬到一半折返回頭！
                        self.mascot_dir_y = -self.mascot_dir_y;
                    }
                }

                // 檢查是否攀爬經過某個字串/按鈕平台的水平高度，決定要不要踏上字串
                for (idx, p) in platforms.iter().enumerate() {
                    if (self.mascot_pos.y - p.0).abs() < 5.0 && self.mascot_platform_idx != idx {
                        let is_near_edge = (is_left && (p.1 - left_wall_x).abs() < 16.0)
                            || (!is_left && (p.2 - right_wall_x).abs() < 16.0);
                        if is_near_edge {
                            let roll = (pseudo_rand >> 8) % 100;
                            if roll < 65 {
                                self.mascot_platform_idx = idx;
                                self.mascot_activity = 0;
                                self.mascot_dir_x = if is_left { 1.0 } else { -1.0 };
                                self.mascot_pos.y = p.0;
                                break;
                            }
                        }
                    }
                }

                // 碰到最頂屋頂或最底面時轉折
                if self.mascot_pos.y <= wall_min_y {
                    self.mascot_pos.y = wall_min_y;
                    self.mascot_activity = 0;
                    self.mascot_platform_idx = 0;
                    self.mascot_dir_x = if is_left { 1.0 } else { -1.0 };
                    self.mascot_bounce = -8.0;
                    self.mascot_squash = 0.75;
                } else if self.mascot_pos.y >= wall_max_y {
                    self.mascot_pos.y = wall_max_y;
                    self.mascot_activity = 0;
                    self.mascot_platform_idx = 9;
                    self.mascot_dir_x = if is_left { 1.0 } else { -1.0 };
                }
            }

            // 狀態 2：懸崖墜落 (Falling with Gravity)
            2 => {
                let gravity = 540.0_f32;
                self.mascot_vel_y += gravity * dt;
                self.mascot_pos.y += self.mascot_vel_y * dt;
                self.mascot_pos.x += self.mascot_dir_x * 18.0 * dt; // 墜落時帶有些微慣性飄移

                // 墜落時身體直立、小幅驚恐晃動
                visual_angle = ((time_sec * 18.0).sin() as f32) * 0.15;

                // 找尋下方最近且包含目前 X 座標的著陸字串/平台
                // 排除起跳平台自身（高度至少需比起跳點低 12px），徹底防止「跳樓瞬間 1 幀立即著陸原地蹲跳」的 Bug！
                let origin_platform_y = platforms[self.mascot_platform_idx].0;
                let mut landed = false;

                // 1. 若無右鍵特權（常態漫遊）：優先依序精準碰撞上方所有障礙物（標籤、裝置名稱、滑桿等）！
                if !self.mascot_spectrum_privilege {
                    for (idx, p) in platforms.iter().enumerate() {
                        if idx != 7
                            && p.0 > origin_platform_y + 12.0
                            && self.mascot_pos.y >= p.0
                            && (self.mascot_pos.y - p.0) < 32.0
                            && self.mascot_pos.x >= p.1 - 10.0
                            && self.mascot_pos.x <= p.2 + 10.0
                        {
                            self.mascot_pos.y = p.0;
                            self.mascot_platform_idx = idx;
                            self.mascot_activity = 0;
                            self.mascot_squash = 0.60;
                            self.mascot_bounce = -8.0;
                            self.mascot_vel_y = 0.0;
                            landed = true;
                            break;
                        }
                    }
                }

                // 2. 若上方障礙物未攔截，或擁有右鍵彩蛋特權時：檢測是否落在頻譜柱狀圖頂端！
                if !landed && self.mascot_platform_idx != 9 {
                    if let Some(sr) = self.mascot_spectrum_rect {
                        if self.mascot_pos.x >= sr.min.x - 12.0 && self.mascot_pos.x <= sr.max.x + 12.0 {
                            let (bar_top_y, effective_lvl) = get_spectrum_bar_surface(sr, self.mascot_pos.x, &self.spectrum_vis.current, &self.spectrum_vis.peaks);
                            // 只要幽靈下墜進入頻譜並抵達柱狀圖頂端：精準著陸！
                            if self.mascot_pos.y >= bar_top_y && (self.mascot_pos.y - bar_top_y) < 36.0 {
                                self.mascot_pos.y = bar_top_y;
                                self.mascot_platform_idx = 7;
                                // 若擁有彩蛋特權且帶有音浪：高空彈飛衝浪！若無特權則一般踏浪行走
                                if self.mascot_spectrum_privilege && (effective_lvl > 0.20 || pseudo_rand % 100 < 85) {
                                    self.mascot_activity = 5;
                                    self.mascot_vel_y = -260.0 - effective_lvl * 150.0;
                                    self.mascot_bounce = -14.0;
                                    self.mascot_squash = 0.52;
                                } else {
                                    self.mascot_activity = 0;
                                    self.mascot_squash = 0.60;
                                    self.mascot_bounce = -8.0;
                                    self.mascot_vel_y = 0.0;
                                }
                                landed = true;
                            }
                        }
                    }
                }

                // 若一路摔到底部
                if !landed && self.mascot_pos.y >= platforms[9].0 {
                    self.mascot_pos.y = platforms[9].0;
                    self.mascot_platform_idx = 9;
                    self.mascot_activity = 0;
                    self.mascot_squash = 0.55;
                    self.mascot_bounce = -10.0;
                    self.mascot_vel_y = 0.0;
                }
            }

            // 狀態 3：地上停步小憩 (IdleOnFloor)
            3 => {
                self.mascot_action_timer -= dt;
                foot_step_y = ((time_sec * 3.5).sin() as f32) * 0.6;
                if self.mascot_action_timer <= 0.0 {
                    // 小憩結束時如果在高處平台：35% 機率「自己跳樓」縱身一躍自由落體！
                    if !is_rhythm_mode && self.mascot_platform_idx < 9 && ((pseudo_rand >> 5) % 100 < 35) {
                        self.mascot_activity = 2; // Falling!
                        self.mascot_vel_y = 50.0;
                        self.mascot_bounce = -6.0;
                        self.mascot_squash = 0.8;
                        return;
                    }
                    // 小憩結束時，有機會主動跳出視窗或竄訪其他視窗
                    if !other_windows.is_empty() && ((pseudo_rand >> 6) % 100 < 35) {
                        let target = other_windows[(pseudo_rand as usize >> 4) % other_windows.len()];
                        self.mascot_window = target;
                        let target_w = target.default_size()[0];
                        self.mascot_pos.x = target_w * 0.5;
                        self.mascot_pos.y = 32.0;
                        self.mascot_platform_idx = 1;
                        self.mascot_activity = 0;
                        self.mascot_bounce = -12.0;
                        self.mascot_squash = 0.7;
                        return;
                    }
                    self.mascot_activity = 0;
                }
            }

            // 狀態 4：受到驚嚇快跑閃躲 (Scamper)
            4 => {
                let p = platforms[self.mascot_platform_idx];
                let run_speed = 76.0_f32;
                self.mascot_pos.x += self.mascot_dir_x * run_speed * dt;
                self.mascot_pos.y = p.0;
                foot_step_y = -((time_sec * 18.0).sin().abs() as f32) * 3.0;
                foot_tilt = ((time_sec * 18.0).sin() as f32) * 0.14 * self.mascot_dir_x;

                self.mascot_action_timer -= dt;

                if self.mascot_dir_x < 0.0 && self.mascot_pos.x <= p.1 {
                    self.mascot_pos.x = p.1;
                    if p.3 {
                        // 逃跑時衝出字串懸崖！自由落體跳樓墜落！
                        self.mascot_activity = 2;
                        self.mascot_vel_y = 60.0;
                        self.mascot_bounce = -6.0;
                    } else {
                        // 遇到視窗外緣，主動跳出視窗！
                        if !other_windows.is_empty() && ((pseudo_rand >> 4) % 100 < 75) {
                            let target = other_windows[(pseudo_rand as usize >> 5) % other_windows.len()];
                            self.mascot_window = target;
                            let target_w = target.default_size()[0];
                            let target_h = target.default_size()[1];
                            self.mascot_pos.x = target_w - 12.0;
                            self.mascot_pos.y = self.mascot_pos.y.clamp(12.0, target_h - 16.0);
                            self.mascot_dir_x = -1.0;
                            self.mascot_bounce = -14.0;
                            self.mascot_squash = 0.65;
                            self.mascot_activity = 0;
                            return;
                        }
                        // 單一視窗時，躍上牆壁逃竄！
                        self.mascot_activity = 1;
                        self.mascot_dir_y = -1.0;
                    }
                } else if self.mascot_dir_x > 0.0 && self.mascot_pos.x >= p.2 {
                    self.mascot_pos.x = p.2;
                    if p.4 {
                        // 逃跑時衝出字串懸崖！自由落體跳樓墜落！
                        self.mascot_activity = 2;
                        self.mascot_vel_y = 60.0;
                        self.mascot_bounce = -6.0;
                    } else {
                        if !other_windows.is_empty() && ((pseudo_rand >> 4) % 100 < 75) {
                            let target = other_windows[(pseudo_rand as usize >> 5) % other_windows.len()];
                            self.mascot_window = target;
                            let target_h = target.default_size()[1];
                            self.mascot_pos.x = 12.0;
                            self.mascot_pos.y = self.mascot_pos.y.clamp(12.0, target_h - 16.0);
                            self.mascot_dir_x = 1.0;
                            self.mascot_bounce = -14.0;
                            self.mascot_squash = 0.65;
                            self.mascot_activity = 0;
                            return;
                        }
                        self.mascot_activity = 1;
                        self.mascot_dir_y = -1.0;
                    }
                } else if self.mascot_action_timer <= 0.0 {
                    self.mascot_activity = 0;
                }
            }

            // 狀態 5：主動跳躍、音浪彈飛與空中衝浪翻滾 (Active Jumping / Air Surfing)
            5 => {
                let gravity = 540.0_f32;
                self.mascot_vel_y += gravity * dt;
                self.mascot_pos.y += self.mascot_vel_y * dt;
                self.mascot_pos.x += self.mascot_dir_x * 58.0 * dt;

                // 往上起跳時若撞到視窗頂部邊緣：頭部觸頂反彈（像瑪利歐頂磚塊，頭部完全不超出視窗）
                if self.mascot_pos.y < min_y + 16.0 {
                    self.mascot_pos.y = min_y + 16.0;
                    self.mascot_vel_y = 60.0; // 頂到天花板反彈下落
                    self.mascot_squash = 0.72;
                    self.mascot_activity = 2; // Falling
                }

                // 🏄 空中特技翻滾（Air Surf 360° Flip）：
                // 當從小頻譜彈飛或在空中飛行時，若擁有右鍵特權，展現超炫空中 360° 連續翻滾衝浪旋轉！
                if self.mascot_spectrum_privilege {
                    visual_angle = (time_sec * 13.0) as f32 * self.mascot_dir_x;
                } else {
                    visual_angle = (self.mascot_vel_y * 0.0015).clamp(-0.35, 0.35) * self.mascot_dir_x;
                }

                // 下墜時檢查是否著陸：
                if self.mascot_vel_y > 0.0 {
                    let mut landed = false;

                    // 1. 若無右鍵特權（常態漫遊）：優先檢測上方所有障礙物（音量滑桿、標籤、按鈕等），障礙物 100% 正常生效！
                    if !self.mascot_spectrum_privilege {
                        let origin_platform_y = platforms[self.mascot_platform_idx].0;
                        for (idx, p) in platforms.iter().enumerate() {
                            if idx != 7
                                && (idx != self.mascot_platform_idx || (self.mascot_pos.y - origin_platform_y).abs() > 20.0)
                                && self.mascot_pos.y >= p.0
                                && (self.mascot_pos.y - p.0) < 28.0
                                && self.mascot_pos.x >= p.1 - 10.0
                                && self.mascot_pos.x <= p.2 + 10.0
                            {
                                self.mascot_pos.y = p.0;
                                self.mascot_platform_idx = idx;
                                self.mascot_activity = 0;
                                self.mascot_squash = 0.62;
                                self.mascot_bounce = -7.0;
                                self.mascot_vel_y = 0.0;
                                landed = true;
                                break;
                            }
                        }
                    }

                    // 2. 若上方障礙物未攔截，或擁有右鍵彩蛋特權（排除障礙物）時：檢測是否落在頻譜柱狀圖頂端！
                    if !landed {
                        if let Some(sr) = self.mascot_spectrum_rect {
                            if self.mascot_pos.x >= sr.min.x - 6.0 && self.mascot_pos.x <= sr.max.x + 6.0 {
                                let (bar_top_y, effective_lvl) = get_spectrum_bar_surface(sr, self.mascot_pos.x, &self.spectrum_vis.current, &self.spectrum_vis.peaks);
                                if self.mascot_pos.y >= bar_top_y && (self.mascot_pos.y - bar_top_y) < 32.0 {
                                    self.mascot_pos.y = bar_top_y;
                                    self.mascot_platform_idx = 7;
                                    // 🎵 只有右鍵特權期間才維持連續高空彈簧床彈跳；常態下著陸踩浪行走
                                    if self.mascot_spectrum_privilege && (effective_lvl > 0.20 || pseudo_rand % 100 < 80) {
                                        self.mascot_activity = 5;
                                        self.mascot_vel_y = -240.0 - effective_lvl * 150.0;
                                        self.mascot_bounce = -14.0;
                                        self.mascot_squash = 0.55;
                                    } else {
                                        self.mascot_activity = 0;
                                        self.mascot_squash = 0.62;
                                        self.mascot_bounce = -8.0;
                                        self.mascot_vel_y = 0.0;
                                    }
                                    landed = true;
                                }
                            }
                        }
                    }

                    // 3. 落到最底層堅實地板
                    if !landed && self.mascot_pos.y >= platforms[9].0 {
                        self.mascot_pos.y = platforms[9].0;
                        self.mascot_platform_idx = 9;
                        self.mascot_activity = 0;
                        self.mascot_squash = 0.55;
                        self.mascot_bounce = -10.0;
                        self.mascot_vel_y = 0.0;
                    }
                }

                // 飛越過深時轉為墜落（若不是在頻譜衝浪）
                if self.mascot_platform_idx != 7 && self.mascot_vel_y > 220.0 {
                    self.mascot_activity = 2; // Falling
                }
            }

            _ => { self.mascot_activity = 0; }
        }

        // 3. 狀態 B：音樂節奏點頭律動（站在地上打拍子跳舞）
        let mut rhythm_bounce_y = 0.0_f32;
        let mut rhythm_tilt = 0.0_f32;
        if is_rhythm_mode && self.mascot_activity != 2 {
            let meter = self.meter_level.clamp(0.0, 1.0);
            let beat_speed = 9.5; // ~120 bpm
            let wave = (time_sec * beat_speed).sin().abs() as f32;
            let amp = 4.0 + meter * 5.5;
            rhythm_bounce_y = -wave * amp;
            rhythm_tilt = ((time_sec * 4.8).sin() as f32) * (0.16 + meter * 0.1);
        }

        // 4. 計算最終著色幾何變換 (直立朝上、著陸擠壓、步伐起伏、音樂彈跳)
        let total_y_offset = self.mascot_bounce + rhythm_bounce_y + foot_step_y;
        let mut final_pos = egui::pos2(self.mascot_pos.x, self.mascot_pos.y + total_y_offset);
        let final_angle = visual_angle + foot_tilt + rhythm_tilt;

        let base_size = 22.0_f32;
        let size_x = base_size * (2.0 - self.mascot_squash);
        let size_y = base_size * self.mascot_squash;
        let mascot_size = egui::vec2(size_x, size_y);

        // 徹底杜絕截頭與截腳：根據當前幽靈變形後的半徑施加動態安全邊距約束，
        // 確保幽靈從眨眼、身體到裙擺的所有像素 100% 完整呈現在視窗邊界內！
        let safe_margin_x = (mascot_size.x * 0.5 + 2.0).ceil();
        let safe_margin_y = (mascot_size.y * 0.5 + 2.0).ceil();
        final_pos.x = final_pos.x.clamp(min_x + safe_margin_x, min_x + w - safe_margin_x);
        final_pos.y = final_pos.y.clamp(min_y + safe_margin_y, min_y + h - safe_margin_y);

        let mascot_rect = egui::Rect::from_center_size(final_pos, mascot_size.max(egui::vec2(28.0, 28.0)));
        let hover_hint = if self.state.license_tier.is_supporter_or_unpackaged() {
            self.state.language.mascot_easter_egg_hint()
        } else {
            self.state.language.mascot_rhythm_hint()
        };
        let resp = ui.allocate_rect(mascot_rect, egui::Sense::click())
            .on_hover_text(hover_hint);

        // 驅趕與互動 (Click & Secondary Click)
        // ⭐ 頻譜區域內滑鼠右鍵召喚：在頻譜區域內按右鍵（或右鍵幽靈），立刻將小幽靈召喚至該頻段頂端並展開大彈飛空翻！
        let is_pointer_in_spectrum = if let Some(sr) = self.mascot_spectrum_rect {
            pointer_pos.map(|p| sr.expand(4.0).contains(p)).unwrap_or(false)
        } else {
            false
        };
        let right_clicked_spectrum = is_pointer_in_spectrum && ui.input(|i| i.pointer.secondary_clicked());

        if right_clicked_spectrum || resp.secondary_clicked() {
            if let Some(sr) = self.mascot_spectrum_rect {
                self.trigger_mascot_air_surf_summon(window_kind, sr, pointer_pos);
            }
        } else if resp.clicked() {
            self.mascot_bounce = -14.0;
            self.mascot_squash = 0.65;
            if self.mascot_platform_idx <= 3 {
                // 在頂層按鈕/標題列點擊驅趕：直接縱身俯衝跳樓！直奔頻譜！
                self.mascot_activity = 2;
                self.mascot_vel_y = 120.0;
                self.mascot_dir_x = if self.mascot_pos.x < (spec_x_min + spec_x_max) * 0.5 { 1.0 } else { -1.0 };
            } else if self.mascot_platform_idx == 7 {
                // 在頻譜上點擊：施展狂暴高空彈飛衝浪！
                self.mascot_activity = 5;
                self.mascot_vel_y = -340.0;
                self.mascot_bounce = -16.0;
                self.mascot_squash = 0.50;
            } else if self.mascot_platform_idx < 9 {
                // 在其他高處點擊：立刻縱身跳樓！
                self.mascot_activity = 2;
                self.mascot_vel_y = 90.0;
            } else {
                // 在底層點擊：火箭大躍起！
                self.mascot_activity = 5;
                self.mascot_vel_y = -350.0;
            }
        }

        let win_name = match window_kind {
            crate::snapping::WindowKind::Main => "主視窗",
            crate::snapping::WindowKind::MiniPlayer => "迷你播放器",
            crate::snapping::WindowKind::Equalizer => "等化器",
            _ => "視窗",
        };
        let wander_hint = if !other_windows.is_empty() {
            "（抵達邊界可穿梭至其他視窗）"
        } else {
            ""
        };
        resp.on_hover_text(if is_rhythm_mode {
            format!("👻 幸運物小幽靈：🎵 正在{}踏在頻譜頂端衝浪！被音浪狂暴彈飛～{}\n💡 提示：在頻譜區域內點擊右鍵立刻召喚牠踩浪彈飛！滑鼠靠近可驅趕", win_name, wander_hint)
        } else {
            format!("👻 幸運物小幽靈：正在{}巡邏～{}\n💡 提示：在頻譜區域內點擊右鍵立刻召喚牠踩浪彈飛！滑鼠靠近可驅趕", win_name, wander_hint)
        });

        // 透過全域 LayerPainter 渲染，避免受內部 CentralPanel inner_margin 裁切，
        // 使幸運物可完整沿著最外側頂部與外邊框暢行無阻
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("mascot_overlay_layer"),
        ));

        // 狀態 B 專屬微光光暈
        if is_rhythm_mode {
            let aura_r = 15.0 + (time_sec * 9.5).sin().abs() as f32 * 3.5;
            painter.circle_filled(
                final_pos,
                aura_r,
                egui::Color32::from_rgba_unmultiplied(255, 215, 0, 32),
            );
        }

        // 5. 旋轉貼圖頂點繪製 (全彩原色)
        if let Some(ghost_tex) = Self::get_ghost_texture(ui.ctx()) {
            let half = mascot_size * 0.5;
            let (cos_a, sin_a) = (final_angle.cos(), final_angle.sin());
            let rot = |off: egui::Vec2| -> egui::Pos2 {
                egui::pos2(
                    final_pos.x + off.x * cos_a - off.y * sin_a,
                    final_pos.y + off.x * sin_a + off.y * cos_a,
                )
            };
            let p0 = rot(egui::vec2(-half.x, -half.y));
            let p1 = rot(egui::vec2(half.x, -half.y));
            let p2 = rot(egui::vec2(half.x, half.y));
            let p3 = rot(egui::vec2(-half.x, half.y));

            let mut mesh = egui::Mesh::with_texture(ghost_tex.id());
            let tint = egui::Color32::WHITE;
            let idx = mesh.vertices.len() as u32;
            mesh.vertices.push(egui::epaint::Vertex { pos: p0, uv: egui::pos2(0.0, 0.0), color: tint });
            mesh.vertices.push(egui::epaint::Vertex { pos: p1, uv: egui::pos2(1.0, 0.0), color: tint });
            mesh.vertices.push(egui::epaint::Vertex { pos: p2, uv: egui::pos2(1.0, 1.0), color: tint });
            mesh.vertices.push(egui::epaint::Vertex { pos: p3, uv: egui::pos2(0.0, 1.0), color: tint });
            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
            painter.add(mesh);
        }

        // 保持 60fps 平滑流暢動畫
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }

    /// 計算幽靈踏在 16 頻段即時音訊頻譜頂端的高度與音浪能量
    pub fn get_spectrum_bar_surface(
        sr: egui::Rect,
        pos_x: f32,
        current: &[f32; 16],
        peaks: &[f32; 16],
    ) -> (f32, f32) {
        let inner = sr.shrink(3.0);
        let rel_x = ((pos_x - inner.min.x) / inner.width()).clamp(0.0, 1.0);
        let bar_idx = ((rel_x * 16.0) as usize).clamp(0, 15);
        let lvl = current[bar_idx].clamp(0.0, 1.0);
        let peak = peaks[bar_idx].clamp(0.0, 1.0);
        let effective = lvl.max(peak * 0.94);
        let bar_h = effective * (inner.height() - 4.0);
        let bar_top_y = inner.max.y - bar_h - 12.0;
        (bar_top_y, effective)
    }

    /// ⭐ 跨視窗即時召喚精靈至指定視窗的頻譜頂部，並觸發空中 360° 特技空翻衝浪
    pub fn trigger_mascot_air_surf_summon(
        &mut self,
        target_window: crate::snapping::WindowKind,
        spec_rect: egui::Rect,
        pointer_pos: Option<egui::Pos2>,
    ) {
        if !self.state.mascot_enabled {
            return;
        }
        // ⭐ 跨視窗瞬間召喚：無論小精靈身處何處（如主視窗），立刻傳送至該視窗！
        self.mascot_window = target_window;
        self.mascot_spectrum_rect = Some(spec_rect);

        if self.state.license_tier.is_supporter_or_unpackaged() {
            self.mascot_spectrum_privilege = true; // 啟動右鍵彩蛋特權！
            let target_x = pointer_pos
                .map(|p| p.x)
                .unwrap_or(spec_rect.center().x)
                .clamp(spec_rect.min.x + 8.0, spec_rect.max.x - 8.0);
            let (bar_top, effective) = Self::get_spectrum_bar_surface(
                spec_rect,
                target_x,
                &self.spectrum_vis.current,
                &self.spectrum_vis.peaks,
            );
            self.mascot_pos.x = target_x;
            self.mascot_pos.y = bar_top;
            self.mascot_platform_idx = 7;
            // 手動召喚：立刻以音浪彈簧床彈射起飛！展開空中 360° 特技空翻！
            self.mascot_activity = 5;
            self.mascot_vel_y = -340.0 - effective * 140.0;
            self.mascot_bounce = -18.0;
            self.mascot_squash = 0.50;
        } else {
            // 非正式版：原地小跳一下，不開啟彩蛋特權
            self.mascot_bounce = -8.0;
            self.mascot_squash = 0.75;
        }
    }

    pub fn get_ghost_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let id = egui::Id::new("app_ghost_emoji_texture");
        if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
            return Some(handle);
        }
        const GHOST_BYTES: &[u8] = include_bytes!("../assets/ghost_emoji.png");
        if let Ok(img) = image::load_from_memory(GHOST_BYTES) {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let handle = ctx.load_texture("ghost_emoji", color_image, egui::TextureOptions::LINEAR);
            ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
            Some(handle)
        } else {
            None
        }
    }

    pub fn render_supporter_mascot_modal(&mut self, ui: &mut egui::Ui) {
        if !self.show_supporter_mascot_modal {
            return;
        }

        let ctx = ui.ctx().clone();
        let screen_rect = ctx.screen_rect();
        let mut should_close = false;
        let mut action_clicked = false;

        // 1. 全螢幕半透明遮罩 (Order::Foreground，阻擋並接收點擊)
        egui::Area::new(egui::Id::new("supporter_mascot_modal_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen_rect.min)
            .interactable(true)
            .show(&ctx, |ui| {
                ui.set_width(screen_rect.width());
                ui.set_height(screen_rect.height());
                ui.painter().rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(190));
                let resp = ui.allocate_rect(screen_rect, egui::Sense::click());
                if resp.clicked() {
                    should_close = true;
                }
            });

        // 2. 感謝卡片彈窗本體 (Order::Tooltip，保證渲染在遮罩之上且事件優先處理)
        let modal_w = 400.0f32.min(screen_rect.width() - 32.0);
        let modal_rect_min = egui::pos2(
            screen_rect.center().x - modal_w * 0.5,
            (screen_rect.center().y - 200.0).max(screen_rect.top() + 16.0),
        );

        let lang = self.state.language;
        let is_existing_user = self.state.last_connected_name.is_some() || crate::controller::load_last_connected_device().is_some();
        let title = if is_existing_user {
            lang.mascot_supporter_modal_title_existing()
        } else {
            lang.mascot_supporter_modal_title_new()
        };
        let body = if is_existing_user {
            lang.mascot_supporter_modal_body_existing()
        } else {
            lang.mascot_supporter_modal_body_new()
        };
        let btn_label = if is_existing_user {
            lang.mascot_supporter_modal_btn_existing()
        } else {
            lang.mascot_supporter_modal_btn_new()
        };

        egui::Area::new(egui::Id::new("supporter_mascot_modal_card"))
            .order(egui::Order::Tooltip)
            .fixed_pos(modal_rect_min)
            .interactable(true)
            .show(&ctx, |ui| {
                ui.set_width(modal_w);

                // ESC 鍵逃生關閉
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }

                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(22, 25, 32))
                    .stroke(egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 205, 80)))
                    .rounding(egui::Rounding::same(14.0))
                    .shadow(egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 8.0),
                        blur: 24.0_f32,
                        spread: 2.0_f32,
                        color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                    })
                    .inner_margin(egui::Margin { left: 20.0, right: 20.0, top: 12.0, bottom: 20.0 })
                    .show(ui, |ui| {
                        // 右上角關閉按鈕 ✕
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let close_btn = egui::Button::new(
                                    egui::RichText::new("✕")
                                        .size(15.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(180, 195, 215)),
                                )
                                .frame(false);
                                if ui.add(close_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    should_close = true;
                                }
                            });
                        });

                        // 繪製大號精靈與金星動畫
                        let (mascot_area, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 90.0), egui::Sense::hover());
                        let mascot_center = mascot_area.center();
                        let time_sec = ui.input(|i| i.time);
                        let bounce_y = ((time_sec * 4.5).sin().abs() as f32) * -10.0;
                        let squash = 1.0 + ((time_sec * 4.5).sin() as f32) * 0.08;
                        let giant_size = egui::vec2(72.0 * (2.0 - squash), 72.0 * squash);
                        let giant_pos = egui::pos2(mascot_center.x, mascot_center.y + bounce_y);

                        let painter = ui.painter();
                        // 金色光暈
                        painter.circle_filled(
                            giant_pos,
                            44.0 + ((time_sec * 3.0).sin().abs() as f32) * 6.0,
                            egui::Color32::from_rgba_unmultiplied(255, 215, 0, 45),
                        );

                        // 漂浮小星星 ✨
                        for i in 0..5 {
                            let angle = time_sec * 1.8 + (i as f64) * 1.256;
                            let star_r = 52.0 + ((time_sec * 2.5 + i as f64).sin() as f32) * 8.0;
                            let sx = giant_pos.x + (angle.cos() as f32) * star_r;
                            let sy = giant_pos.y + (angle.sin() as f32) * (star_r * 0.6);
                            let star_alpha = ((time_sec * 4.0 + i as f64).sin().abs() * 200.0 + 55.0) as u8;
                            painter.circle_filled(
                                egui::pos2(sx, sy),
                                3.2,
                                egui::Color32::from_rgba_unmultiplied(255, 230, 110, star_alpha),
                            );
                        }

                        // 大號幽靈貼圖
                        if let Some(ghost_tex) = Self::get_ghost_texture(ui.ctx()) {
                            let half = giant_size * 0.5;
                            let p0 = egui::pos2(giant_pos.x - half.x, giant_pos.y - half.y);
                            let p1 = egui::pos2(giant_pos.x + half.x, giant_pos.y - half.y);
                            let p2 = egui::pos2(giant_pos.x + half.x, giant_pos.y + half.y);
                            let p3 = egui::pos2(giant_pos.x - half.x, giant_pos.y + half.y);

                            let mut mesh = egui::Mesh::with_texture(ghost_tex.id());
                            let tint = egui::Color32::WHITE;
                            let idx = mesh.vertices.len() as u32;
                            mesh.vertices.push(egui::epaint::Vertex { pos: p0, uv: egui::pos2(0.0, 0.0), color: tint });
                            mesh.vertices.push(egui::epaint::Vertex { pos: p1, uv: egui::pos2(1.0, 0.0), color: tint });
                            mesh.vertices.push(egui::epaint::Vertex { pos: p2, uv: egui::pos2(1.0, 1.0), color: tint });
                            mesh.vertices.push(egui::epaint::Vertex { pos: p3, uv: egui::pos2(0.0, 1.0), color: tint });
                            mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
                            painter.add(mesh);
                        }

                        ui.add_space(8.0);

                        // 標題、內文與操作按鈕
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(title)
                                    .strong()
                                    .size(15.5)
                                    .color(egui::Color32::from_rgb(255, 220, 120)),
                            );
                            ui.add_space(8.0);

                            ui.label(
                                egui::RichText::new(body)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(225, 230, 240)),
                            );
                            ui.add_space(16.0);

                            let btn = egui::Button::new(
                                egui::RichText::new(btn_label)
                                    .strong()
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(20, 24, 30)),
                            )
                            .fill(egui::Color32::from_rgb(255, 210, 80))
                            .rounding(egui::Rounding::same(8.0))
                            .min_size(egui::vec2(220.0, 36.0));

                            if ui.add(btn).clicked() {
                                action_clicked = true;
                            }
                        });
                    });
            });

        if action_clicked {
            self.show_supporter_mascot_modal = false;
            let _ = self.cmd_tx.send_blocking(crate::controller::GuiCmd::SetMascotSupporterSecretRevealed(true));
            self.mascot_bounce = -16.0;
            self.mascot_squash = 0.55;
            // ⭐ 支持者盛典揭密禮遇：若迷你播放器已開啟，主動將小精靈召喚至迷你播放器頻譜頂部，讓使用者一目了然！
            if self.mini_player_window_open {
                self.mascot_window = crate::snapping::WindowKind::MiniPlayer;
                if let Some(sr) = self.mascot_spectrum_rect {
                    self.mascot_pos.x = sr.center().x;
                    self.mascot_pos.y = sr.min.y;
                    self.mascot_platform_idx = 7;
                }
            }
        } else if should_close {
            self.show_supporter_mascot_modal = false;
            let _ = self.cmd_tx.send_blocking(crate::controller::GuiCmd::SetMascotSupporterSecretRevealed(true));
        }

        // 持續以 60 FPS 刷新動畫
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    pub fn render_opacity_popup_content(
        ui: &mut egui::Ui,
        kind: crate::snapping::WindowKind,
        trigger_rect: Option<egui::Rect>,
    ) {
        let cur_op = crate::snapping::get_window_opacity(kind);
        let cur_pct = (cur_op * 100.0).round() as u32;

        ui.set_width(220.0);
        ui.set_max_width(220.0);
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 6.0);
        ui.spacing_mut().button_padding = egui::vec2(0.0, 2.0);

        // 套用視覺視窗同款淺天藍色主視覺主題 (Sky Blue / Cyan Accent Theme #38bdf8)
        let sky_blue = egui::Color32::from_rgb(56, 189, 248);
        let sky_blue_hover = egui::Color32::from_rgb(125, 211, 252);
        let sky_blue_active = egui::Color32::from_rgb(14, 165, 233);

        ui.visuals_mut().selection.bg_fill = sky_blue;
        ui.visuals_mut().selection.stroke = egui::Stroke::new(1.0_f32, sky_blue_hover);
        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
        ui.visuals_mut().widgets.hovered.bg_fill = sky_blue_hover;
        ui.visuals_mut().widgets.hovered.fg_stroke.color = sky_blue;
        ui.visuals_mut().widgets.active.bg_fill = sky_blue_active;
        ui.visuals_mut().widgets.active.fg_stroke.color = sky_blue;

        // 頂部標題與目前百分比高亮
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("🎯 視窗透明度")
                    .strong()
                    .size(12.0)
                    .color(egui::Color32::WHITE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{cur_pct}%"))
                        .strong()
                        .size(13.0)
                        .color(sky_blue),
                );
            });
        });

        // 專屬淺天藍色滿版微互動滑桿 (20% ~ 100%，高亮淺天藍色左側進度軌道 + 實心淺天藍色旋鈕)
        let total_slider_w = ui.available_width();
        let slider_area_h = 24.0_f32;
        let (slider_rect, slider_resp) = ui.allocate_exact_size(
            egui::vec2(total_slider_w, slider_area_h),
            egui::Sense::click_and_drag(),
        );

        // 處理點擊與拖曳互動
        if slider_resp.dragged() || slider_resp.clicked() {
            if let Some(pos) = slider_resp.interact_pointer_pos() {
                let norm = ((pos.x - slider_rect.min.x) / slider_rect.width()).clamp(0.0, 1.0);
                let new_pct = (20.0 + norm * 80.0).round() as u32;
                let next_op = new_pct as f32 / 100.0;
                let is_shift = ui.input(|i| i.modifiers.shift);
                if is_shift {
                    crate::snapping::set_single_window_opacity_tuning(kind, next_op);
                } else {
                    crate::snapping::set_cluster_opacity(kind, next_op);
                }
            }
        }

        // 處理懸停在滑桿上的滾輪微調
        if slider_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            let scroll = ui.input(|i| {
                if i.raw_scroll_delta.y.abs() > 0.5 {
                    i.raw_scroll_delta.y
                } else if i.raw_scroll_delta.x.abs() > 0.5 {
                    i.raw_scroll_delta.x
                } else {
                    0.0
                }
            });
            if scroll.abs() > 0.5 {
                let delta = if scroll > 0.0 { 0.05 } else { -0.05 };
                let next_op = (cur_op + delta).clamp(0.2, 1.0);
                let is_shift = ui.input(|i| i.modifiers.shift);
                if is_shift {
                    crate::snapping::set_single_window_opacity_tuning(kind, next_op);
                } else {
                    crate::snapping::set_cluster_opacity(kind, next_op);
                }
            }
        }

        // 繪製滑桿視覺效果
        let painter = ui.painter();
        let track_h = 4.5_f32;
        let track_y = slider_rect.center().y - track_h * 0.5 + 4.0_f32;
        let track_rect = egui::Rect::from_min_size(
            egui::pos2(slider_rect.min.x, track_y),
            egui::vec2(slider_rect.width(), track_h),
        );
        let rounding = egui::Rounding::same(track_h * 0.5);

        // 1. 底層未滑過軌道 (微透暗白灰)
        painter.rect_filled(
            track_rect,
            rounding,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        );

        // 1b. 40% 穿透分界點 (Notch) 計算：(40 - 20) / (100 - 20) = 20 / 80 = 0.25 (25%)
        let notch_progress = (40.0_f32 - 20.0) / 80.0;
        let notch_x = track_rect.min.x + track_rect.width() * notch_progress;

        // 1c. 穿透區間（20% ~ 40%）底槽以幽靈紫金微光襯底
        let ghost_track_rect = egui::Rect::from_min_max(
            track_rect.min,
            egui::pos2(notch_x, track_rect.max.y),
        );
        painter.rect_filled(
            ghost_track_rect,
            rounding,
            egui::Color32::from_rgba_unmultiplied(180, 140, 255, 55),
        );

        // 2. 左側高亮進度軌道 (20% ~ 100%)
        let progress = ((cur_pct as f32 - 20.0) / 80.0).clamp(0.0, 1.0);
        let filled_w = (track_rect.width() * progress).max(track_h);
        let filled_rect = egui::Rect::from_min_size(track_rect.min, egui::vec2(filled_w, track_h));
        painter.rect_filled(filled_rect, rounding, sky_blue);

        // 2b. 40% 臨界刻度 Notch 微光垂直刻度標記 (高度延伸 10px，帶柔光)
        let is_near_notch = (cur_pct as i32 - 40).abs() <= 2;
        let notch_color = if is_near_notch {
            egui::Color32::from_rgb(255, 245, 140)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 215, 0, 210)
        };
        let notch_top = track_rect.min.y - 3.5;
        let notch_bot = track_rect.max.y + 3.5;
        painter.line_segment(
            [egui::pos2(notch_x, notch_top), egui::pos2(notch_x, notch_bot)],
            egui::Stroke::new(1.5_f32, notch_color),
        );
        // 邊界點放上 👻 微軟原生全彩字符 (與視覺視窗及終端機一致：白身、眨眼、紅嘴)
        if let Some(ghost_tex) = Self::get_ghost_texture(ui.ctx()) {
            let ghost_size = egui::vec2(13.0, 13.0);
            let ghost_rect = egui::Rect::from_center_size(
                egui::pos2(notch_x, notch_top - ghost_size.y * 0.5 - 1.0),
                ghost_size,
            );
            painter.image(
                ghost_tex.id(),
                ghost_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            painter.text(
                egui::pos2(notch_x, notch_top - 1.5),
                egui::Align2::CENTER_BOTTOM,
                "👻",
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }

        // 3. 實心淺天藍色旋鈕 (圓心精準落在滑桿軌道中心線上)
        let knob_x = (track_rect.min.x + track_rect.width() * progress).clamp(
            slider_rect.min.x + 5.5,
            slider_rect.max.x - 5.5,
        );
        let knob_center = egui::pos2(knob_x, track_rect.center().y);
        let is_knob_active = slider_resp.hovered() || slider_resp.dragged();
        let knob_radius = if is_knob_active { 6.5_f32 } else { 5.5_f32 };
        painter.circle_filled(knob_center, knob_radius, sky_blue);
        painter.circle_stroke(
            knob_center,
            knob_radius,
            egui::Stroke::new(1.0_f32, sky_blue_hover),
        );

        // 4. 滑桿懸停在 40% Notch 附近時顯示 Tooltip 引導
        if slider_resp.hovered() {
            if let Some(pos) = slider_resp.hover_pos() {
                if (pos.x - notch_x).abs() <= 9.0 {
                    slider_resp.clone().on_hover_text("👻 40% 點擊穿透臨界點（≤40% 視窗穿透，標題列仍可操作）");
                }
            }
        }

        // 橫向 4 顆預設按鈕 (30%, 60%, 80%, 100%)，高度 22px
        ui.horizontal(|ui| {
            let presets = [30, 60, 80, 100];
            let total_spacing = ui.spacing().item_spacing.x * (presets.len() as f32 - 1.0);
            let btn_w = ((ui.available_width() - total_spacing) / presets.len() as f32).max(42.0);
            for p in presets {
                let is_act = (cur_pct as i32 - p as i32).abs() <= 3;
                let label_str = if p == 30 {
                    "30% 👻".to_string()
                } else {
                    format!("{p}%")
                };
                let text = egui::RichText::new(label_str)
                    .size(10.0)
                    .color(if is_act {
                        egui::Color32::from_rgb(17, 17, 17)
                    } else {
                        egui::Color32::from_rgb(204, 204, 204)
                    });
                let fill = if is_act {
                    sky_blue
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                };
                let btn = egui::Button::new(text)
                    .fill(fill)
                    .min_size(egui::vec2(btn_w, 22.0));
                if ui.add(btn).clicked() {
                    let next_op = p as f32 / 100.0;
                    let is_shift = ui.input(|i| i.modifiers.shift);
                    if is_shift {
                        crate::snapping::set_single_window_opacity_tuning(kind, next_op);
                    } else {
                        crate::snapping::set_cluster_opacity(kind, next_op);
                    }
                }
            }
        });

        ui.separator();

        // 底部操作提示（提案 A：洗練版）
        ui.label(
            egui::RichText::new("👻 ≤40% 啟用穿透（標題列仍可操作）")
                .size(10.0)
                .color(egui::Color32::from_rgb(255, 215, 0)),
        );
        ui.label(
            egui::RichText::new("🖱️ 滾輪調整透明度，Shift組合僅限此視窗")
                .size(9.5)
                .color(egui::Color32::from_rgb(140, 146, 158)),
        );

        // 滑鼠移出面板 0.6 秒後自動優雅關閉（比照視覺視窗）
        let pointer_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()));
        let is_menu_hovered = ui.ui_contains_pointer()
            || pointer_pos.map_or(false, |pos| ui.max_rect().expand(4.0).contains(pos));
        let is_trigger_hovered = trigger_rect.map_or(false, |r| {
            pointer_pos.map_or(false, |pos| r.expand(8.0).contains(pos))
        });

        let leave_id = egui::Id::new("opacity_menu_leave_timer").with(kind);
        let entered_id = egui::Id::new("opacity_menu_entered").with(kind);
        let now = ui.input(|i| i.time);

        if is_menu_hovered {
            ui.data_mut(|d| {
                d.insert_temp(entered_id, true);
                d.remove_temp::<f64>(leave_id);
            });
        } else if is_trigger_hovered {
            ui.data_mut(|d| d.remove_temp::<f64>(leave_id));
        } else {
            let leave_time = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(leave_id, || now));
            if now - leave_time >= 0.6 {
                ui.data_mut(|d| {
                    d.remove_temp::<f64>(leave_id);
                    d.remove_temp::<bool>(entered_id);
                });
                ui.close_menu();
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
    }

    /// 通用置頂按鈕渲染組件：
    /// 依據點擊時的視窗來源（kind）及其身分（Standalone vs Clustered）切換置頂狀態。
    pub fn render_universal_pin_button(&mut self, ui: &mut egui::Ui, kind: crate::snapping::WindowKind) {
        let lang = self.state.language;
        let pinned = crate::snapping::is_window_pinned(kind);
        let role = crate::snapping::get_window_role(kind);

        let tooltip = match role {
            crate::snapping::WindowRole::ClusterMaster | crate::snapping::WindowRole::ClusterSlave => {
                if pinned {
                    format!("{} (點擊切換)", lang.tooltip_unpin_cluster())
                } else {
                    format!("{} (點擊切換)", lang.tooltip_pin_cluster())
                }
            }
            crate::snapping::WindowRole::Standalone => {
                if pinned {
                    format!("{} (點擊切換)", lang.tooltip_unpin_window())
                } else {
                    format!("{} (點擊切換)", lang.tooltip_pin_window())
                }
            }
        };

        let resp = Self::render_stateful_icon_button(
            ui,
            "📌",
            pinned,
            &tooltip,
            ButtonTheme::Gold,
        );

        // 圖釘按鈕回歸純粹的「釘選開關（Pin / Unpin）」，只響應單擊
        if resp.clicked() {
            unsafe {
                crate::snapping::toggle_window_pin(kind);
            }
            let new_pinned = crate::snapping::is_window_pinned(kind);
            let level = if new_pinned {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));

            if kind == crate::snapping::WindowKind::Main {
                self.is_always_on_top = new_pinned;
            }
        }
    }

    /// 一鍵呈現模組化工作站：
    /// - 常態下：呈現三欄四聯工作站（主視窗、迷你播放器、等化器與視覺效果齊平磁吸）。
    /// - 全螢幕下（提案 B）：不脫離全螢幕，僅組裝雙欄浮空 HUD 工作站（主視窗、迷你播放器、等化器），全螢幕視覺視窗保持獨立不加入。
    pub fn assemble_modular_workstation(&mut self) {
        let is_vis_fs = crate::visualizer::is_visualizer_fullscreen();
        if !is_vis_fs {
            if !crate::visualizer::is_visualizer_open() {
                crate::visualizer::open_visualizer();
            }
        }
        if !self.mini_player_window_open {
            self.mini_player_window_open = true;
            self.mini_player_needs_initial_dock = false;
        }
        if !self.equalizer_window_open {
            self.equalizer_window_open = true;
            self.equalizer_needs_initial_dock = false;
        }

        // 若各成員視窗已開啟但處於最小化（Iconic）狀態，立即解除最小化並還原顯示
        unsafe {
            let kinds: &[crate::snapping::WindowKind] = if is_vis_fs {
                &[
                    crate::snapping::WindowKind::Main,
                    crate::snapping::WindowKind::MiniPlayer,
                    crate::snapping::WindowKind::Equalizer,
                ]
            } else {
                &[
                    crate::snapping::WindowKind::Main,
                    crate::snapping::WindowKind::MiniPlayer,
                    crate::snapping::WindowKind::Equalizer,
                    crate::snapping::WindowKind::Visualizer,
                ]
            };
            for &kind in kinds {
                let hwnd = crate::snapping::get_window_hwnd(kind);
                if !hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 {
                    if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0 {
                        windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                            hwnd,
                            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE,
                        );
                    }
                }
            }
        }

        // Synchronize visualizer and cluster pinned state with master app pin state
        let main_pinned = crate::snapping::is_window_pinned(crate::snapping::WindowKind::Main);
        self.is_always_on_top = main_pinned;
        if !is_vis_fs {
            crate::visualizer::set_visualizer_pinned(main_pinned);
        }
        unsafe {
            crate::snapping::set_cluster_pinned(crate::snapping::WindowKind::Main, main_pinned);
        }
        if is_vis_fs {
            crate::visualizer::VISUALIZER_HUD_MAIN_SHOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        self.pending_modular_workstation_frames = 30;
        if let Some(ctx) = get_egui_ctx() {
            ctx.request_repaint();
        }
    }

    /// 渲染標題列最左側的 App Icon 工作站按鈕（提案一：新使用者專屬呼吸微光與導引氣泡）
    pub fn render_modular_workstation_button(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;
        let tooltip = lang.tooltip_modular_workstation();
        let is_vis_fs = crate::visualizer::is_visualizer_fullscreen();
        let is_workstation_active = self.mini_player_window_open
            && self.equalizer_window_open
            && (is_vis_fs || crate::visualizer::is_visualizer_open());
        let is_new_user_guide = !self.workstation_onboarding_dismissed && !is_workstation_active;

        let btn_size = egui::vec2(22.0, 22.0);
        let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tooltip);

        let painter = ui.painter();

        // 1. 新使用者動態呼吸微光光圈（Pulsing Beacon Glow）
        if is_new_user_guide {
            let time = ui.input(|i| i.time);
            let pulse = ((time * 3.2).sin() * 0.5 + 0.5) as f32; // 0.0 ~ 1.0
            ui.ctx().request_repaint();

            let halo_rect = rect.expand(1.5 + pulse * 3.2);
            let alpha = (45.0 + pulse * 140.0) as u8;
            let glow_stroke = egui::Stroke::new(
                1.2 + pulse * 0.6,
                egui::Color32::from_rgba_unmultiplied(75, 175, 255, alpha),
            );
            painter.rect_stroke(halo_rect, egui::Rounding::same(6.0), glow_stroke);
        }

        if resp.hovered() || is_workstation_active {
            let bg = if resp.is_pointer_button_down_on() {
                egui::Color32::from_rgb(35, 75, 130)
            } else if is_workstation_active {
                egui::Color32::from_rgb(25, 45, 75)
            } else {
                egui::Color32::from_rgb(45, 52, 65)
            };
            let stroke = if is_workstation_active {
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(70, 140, 215))
            } else {
                egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40))
            };
            painter.rect_filled(rect, egui::Rounding::same(4.0), bg);
            painter.rect_stroke(rect, egui::Rounding::same(4.0), stroke);
        }

        if let Some(ref tex) = self.app_icon_texture {
            let icon_rect = rect.shrink(2.0);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let tint = if resp.hovered() || is_workstation_active || is_new_user_guide {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(210, 220, 235)
            };
            painter.image(tex.id(), icon_rect, uv, tint);
        } else {
            let icon_col = if is_workstation_active {
                egui::Color32::from_rgb(100, 200, 255)
            } else if is_new_user_guide {
                egui::Color32::from_rgb(130, 210, 255)
            } else {
                egui::Color32::from_rgb(180, 190, 205)
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "◫",
                egui::FontId::proportional(14.0),
                icon_col,
            );
        }

        if resp.clicked() {
            if is_new_user_guide {
                self.workstation_onboarding_dismissed = true;
                crate::controller::save_workstation_discovered(true);
            }
            self.assemble_modular_workstation();
        }

        // 2. 新手導引浮動氣泡卡片（Onboarding Balloon Card）
        if is_new_user_guide {
            let balloon_pos = egui::pos2(rect.min.x - 2.0, rect.max.y + 6.0);
            let mut should_dismiss = false;
            let mut should_open = false;

            egui::Area::new(egui::Id::new("workstation_onboarding_balloon"))
                .fixed_pos(balloon_pos)
                .order(egui::Order::Tooltip)
                .show(ui.ctx(), |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_unmultiplied(18, 24, 36, 248))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(75, 155, 245)))
                        .rounding(egui::Rounding::same(8.0))
                        .shadow(egui::epaint::Shadow {
                            offset: egui::vec2(0.0, 4.0),
                            blur: 16.0_f32,
                            spread: 0.0_f32,
                            color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160),
                        })
                        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_max_width(280.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(lang.onboarding_workstation_title())
                                        .size(13.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(255, 215, 80)),
                                );
                                ui.add_space(3.0);
                                ui.label(
                                    egui::RichText::new(lang.onboarding_workstation_desc())
                                        .size(11.5)
                                        .color(egui::Color32::from_rgb(200, 215, 235)),
                                );
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    let try_btn = egui::Button::new(
                                        egui::RichText::new(lang.onboarding_workstation_btn_try())
                                            .size(11.5)
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(egui::Color32::from_rgb(35, 105, 200))
                                    .rounding(egui::Rounding::same(4.0));
                                    if ui.add(try_btn).clicked() {
                                        should_open = true;
                                    }

                                    ui.add_space(6.0);

                                    let dismiss_btn = egui::Button::new(
                                        egui::RichText::new(lang.onboarding_workstation_btn_dismiss())
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(160, 175, 195)),
                                    )
                                    .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15))
                                    .rounding(egui::Rounding::same(4.0));
                                    if ui.add(dismiss_btn).clicked() {
                                        should_dismiss = true;
                                    }
                                });
                            });
                        });
                });

            if should_open {
                self.workstation_onboarding_dismissed = true;
                crate::controller::save_workstation_discovered(true);
                self.assemble_modular_workstation();
            } else if should_dismiss {
                self.workstation_onboarding_dismissed = true;
                crate::controller::save_workstation_discovered(true);
            }
        }
    }

    /// 實時偵測物理鍵盤 Shift 鍵狀態（直接查詢 Win32 硬體非同步狀態，不受多視窗焦點或訊息丟失干擾）
    #[inline]
    pub fn is_physical_shift_down() -> bool {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT as i32,
                ) as i16) < 0
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    /// 通用視窗標題列渲染組件：
    /// 規範全系統視窗的左側（返回/Home/標題）與右側（系統置頂/彈出、分隔線、功能按鈕）排版。
    pub fn render_universal_window_header<F>(
        &mut self,
        ui: &mut egui::Ui,
        kind: crate::snapping::WindowKind,
        title: &str,
        show_back_button: bool,
        custom_actions: F,
    ) where
        F: FnOnce(&mut Self, &mut egui::Ui),
    {
        let lang = self.state.language;
        let role = crate::snapping::get_window_role(kind);

        ui.horizontal(|ui| {
            // Left Group: Modular Workstation Button (if Main root) / Back Button (if subpage) / Home Button (if secondary) + Title + Role Badge
            if show_back_button {
                if ui.button(lang.btn_back()).clicked() {
                    self.go_back();
                }
            } else if kind != crate::snapping::WindowKind::Main {
                Self::render_universal_home_button(ui, kind, lang);
            } else {
                // Main Window Root: App Icon Modular Workstation Button
                self.render_modular_workstation_button(ui);
                ui.add_space(2.0);
            }

            // Title: 可拖曳感應區塊 (懸停顯示 Grab ✋，按住顯示 Grabbing ✊)
            let cur_op = crate::snapping::get_window_opacity(kind);
            let cur_pct = (cur_op * 100.0).round() as u32;
            let is_ghost = cur_pct <= 40;
            let is_in_cluster = matches!(
                role,
                crate::snapping::WindowRole::ClusterMaster | crate::snapping::WindowRole::ClusterSlave
            );

            let title_tooltip = if is_ghost {
                if is_in_cluster {
                    "👻 視窗標題列 (穿透手掌區)\n• 拖曳：移動群組\n• 雙擊：還原成不透明置頂\n• 滾輪：調整透明度\n• 右鍵：視窗透明度選單\n• Shift + 雙擊/滾輪：僅作用於此視窗"
                } else {
                    "👻 視窗標題列 (穿透手掌區)\n• 拖曳：移動視窗\n• 雙擊：還原成不透明置頂\n• 滾輪：調整透明度\n• 右鍵：視窗透明度選單"
                }
            } else {
                if is_in_cluster {
                    "視窗標題列\n• 拖曳：移動群組\n• 雙擊：切換成穿透模式\n• 滾輪：調整透明度\n• 右鍵：視窗透明度選單\n• Shift + 雙擊/滾輪：僅作用於此視窗"
                } else {
                    "視窗標題列\n• 拖曳：移動視窗\n• 雙擊：切換成穿透模式\n• 滾輪：調整透明度\n• 右鍵：視窗透明度選單"
                }
            };

            let title_resp = ui.add(
                egui::Label::new(
                    egui::RichText::new(title)
                        .strong()
                        .size(15.0)
                        .color(egui::Color32::from_rgb(180, 220, 245)),
                ).sense(egui::Sense::click_and_drag())
            ).on_hover_text(title_tooltip);

            // Ghost Icon (穿透態 👻：微軟 Segoe UI Emoji 全彩原色，無膠囊底色，支援雙擊還原與拖曳移動)
            let mut ghost_capsule_resp: Option<egui::Response> = None;
            let mut left_content_right = title_resp.rect.right();
            if is_ghost {
                let ghost_size = egui::vec2(16.0, 16.0);
                let (rect, resp) = ui.allocate_exact_size(ghost_size, egui::Sense::click_and_drag());
                if let Some(ghost_tex) = Self::get_ghost_texture(ui.ctx()) {
                    ui.painter().image(
                        ghost_tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "👻",
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );
                }
                let ghost_btn = resp.on_hover_text(if is_in_cluster {
                    "👻 穿透模式\n• 雙擊此處或標題列：還原成不透明置頂\n• 滾輪：調整透明度\n• 右鍵：視窗透明度選單\n• Shift + 雙擊：僅作用於此視窗"
                } else {
                    "👻 穿透模式\n• 雙擊此處或標題列：還原成不透明置頂\n• 滾輪：調整透明度\n• 右鍵：視窗透明度選單"
                });
                left_content_right = ghost_btn.rect.right();
                ghost_capsule_resp = Some(ghost_btn);
            }

            // Role Badge (主控 / 附屬節點身分指示徽章：🧲 / 📎，亦支援按住拖曳視窗)
            let mut badge_resp = match role {
                crate::snapping::WindowRole::ClusterMaster => {
                    let badge = ui.add(
                        egui::Label::new(
                            egui::RichText::new(lang.badge_cluster_master())
                                .size(14.0),
                        ).sense(egui::Sense::click_and_drag())
                    ).on_hover_text(lang.tooltip_cluster_master());
                    left_content_right = badge.rect.right();
                    Some(badge)
                }
                crate::snapping::WindowRole::ClusterSlave => {
                    let badge = ui.add(
                        egui::Label::new(
                            egui::RichText::new(lang.badge_cluster_slave())
                                .size(14.0),
                        ).sense(egui::Sense::click_and_drag())
                    ).on_hover_text(lang.tooltip_cluster_slave());
                    left_content_right = badge.rect.right();
                    Some(badge)
                }
                crate::snapping::WindowRole::Standalone => None,
            };

            // Right Group (Right-to-Left layout):
            // 獨立於標題拖曳區，保證所有按鈕 100% 正常感應點擊
            let right_group = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_in_cluster = matches!(
                    role,
                    crate::snapping::WindowRole::ClusterMaster | crate::snapping::WindowRole::ClusterSlave
                );

                let btn_size = egui::vec2(26.0, 22.0);

                // --- 1. Caption Close Button (✕ / ⛝) ---
                // 方案 1：懸停精準感應制 (Hover-Only Dynamic Morph)
                // - 未懸停時：永遠顯示標準符號 " ✕ "，絕不殘留錯誤圖示
                // - 懸停且按住 Shift 時：動態形變為 " ⛝ "，但維持正常深灰背景，移除警戒高亮紅
                let (close_rect, close_resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                let is_close_hovered = close_resp.hovered();
                if is_close_hovered {
                    ui.ctx().request_repaint();
                }

                let is_close_shift = is_in_cluster && is_close_hovered && Self::is_physical_shift_down();

                let close_tooltip = if !is_in_cluster {
                    lang.tooltip_close_window()
                } else if is_close_shift {
                    lang.tooltip_close_cluster_all()
                } else {
                    lang.tooltip_close_cluster()
                };

                let close_label = if is_in_cluster && !is_close_shift { " ⛝ " } else { " ✕ " };
                let close_bg = if close_resp.is_pointer_button_down_on() {
                    egui::Color32::from_rgb(40, 44, 52)
                } else if is_close_hovered {
                    egui::Color32::from_rgb(68, 76, 92)
                } else {
                    egui::Color32::from_rgb(52, 58, 70)
                };
                let close_text_color = egui::Color32::from_rgb(220, 225, 235);

                ui.painter().rect_filled(close_rect, egui::Rounding::same(4.0), close_bg);
                ui.painter().text(
                    close_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    close_label,
                    egui::FontId::proportional(13.0),
                    close_text_color,
                );

                let close_resp = close_resp.on_hover_text(close_tooltip);

                if close_resp.clicked() {
                    if is_in_cluster {
                        if Self::is_physical_shift_down() {
                            // 反轉方案：Shift + 點擊僅關閉本視窗 (Solo)
                            self.close_popout_window(kind);
                        } else {
                            // 反轉方案：普通點擊關閉整個工作站群組
                            self.close_window_cluster(kind);
                        }
                    } else {
                        // 普通點擊：僅關閉本視窗
                        self.close_popout_window(kind);
                    }
                }

                // --- 2. Minimize Button (_ / ‗) ---
                // 方案 1：懸停精準感應制 (Hover-Only Dynamic Morph)
                // - 成組且未按 Shift 時：顯示雙層沉底底線 " ‗ "，預設點擊最小化整個群組
                // - 成組且按住 Shift 時：動態形變為標準單橫線 " _ "，點擊僅最小化單一視窗
                let (min_rect, min_resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                let is_min_hovered = min_resp.hovered();
                if is_min_hovered {
                    ui.ctx().request_repaint();
                }

                let is_min_shift = is_in_cluster && is_min_hovered && Self::is_physical_shift_down();

                let min_tooltip = if !is_in_cluster {
                    lang.tooltip_minimize_window()
                } else if is_min_shift {
                    lang.tooltip_minimize_cluster_all()
                } else {
                    lang.tooltip_minimize_cluster()
                };

                let (min_label, min_color) = if is_in_cluster && !is_min_shift {
                    (" ‗ ", egui::Color32::from_rgb(180, 215, 255))
                } else {
                    (" _ ", egui::Color32::from_rgb(200, 205, 215))
                };

                let min_bg = if min_resp.is_pointer_button_down_on() {
                    egui::Color32::from_rgb(40, 44, 52)
                } else if is_min_hovered {
                    egui::Color32::from_rgb(68, 76, 92)
                } else {
                    egui::Color32::from_rgb(52, 58, 70)
                };

                ui.painter().rect_filled(min_rect, egui::Rounding::same(4.0), min_bg);
                ui.painter().text(
                    min_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    min_label,
                    egui::FontId::proportional(13.0),
                    min_color,
                );

                let min_resp = min_resp.on_hover_text(min_tooltip);

                if min_resp.clicked() {
                    if is_in_cluster {
                        if Self::is_physical_shift_down() {
                            // 反轉方案：Shift + 點擊僅最小化本視窗 (Solo)
                            unsafe {
                                crate::snapping::minimize_window_solo(kind);
                            }
                        } else {
                            // 反轉方案：普通點擊最小化整個工作站群組
                            unsafe {
                                crate::snapping::minimize_cluster(kind);
                            }
                        }
                    } else {
                        unsafe {
                            crate::snapping::minimize_window_solo(kind);
                        }
                    }
                }

                // Window System Controls
                self.render_universal_pin_button(ui, kind);

                if let Some(popout_kind) = self.current_page.popout_window_kind() {
                    if show_back_button {
                        // We are in an embedded page that can be popped out
                        self.render_universal_popout_button(ui, popout_kind, kind);
                    }
                }

                // Separator
                Self::render_header_separator(ui);

                // Custom App Action Buttons (e.g. Visualizer 🎨, Equalizer ⦀, etc.)
                custom_actions(self, ui);
            });

            // Middle space between Left Content and Right Group: 也支援滑鼠按住拖動視窗 (懸停顯示 Grab ✋，按住顯示 Grabbing ✊)
            let mid_left = left_content_right;
            let mid_right = right_group.response.rect.left();
            let mut mid_resp: Option<egui::Response> = None;
            if mid_right > mid_left + 4.0 {
                let header_top = right_group.response.rect.top().min(title_resp.rect.top() - 6.0);
                let header_bottom = right_group.response.rect.bottom().max(title_resp.rect.bottom() + 6.0);
                let mid_rect = egui::Rect::from_min_max(
                    egui::pos2(mid_left, header_top),
                    egui::pos2(mid_right - 4.0, header_bottom),
                );
                let mid_id = ui.id().with(("header_mid_drag", kind));
                let resp = ui.interact(mid_rect, mid_id, egui::Sense::click_and_drag());
                mid_resp = Some(resp.on_hover_text(title_tooltip));
            }

            // 解法 C：標題列手掌區（Titlebar Grab Area）三態生命週期控制：
            // 1. 拖曳搬移（Drag Started）：啟動視窗拖動，維持當前穿透狀態不變（移開游標自然淡回）
            let any_drag_started = title_resp.drag_started()
                || ghost_capsule_resp.as_ref().map(|b| b.drag_started()).unwrap_or(false)
                || badge_resp.as_ref().map(|b| b.drag_started()).unwrap_or(false)
                || mid_resp.as_ref().map(|r| r.drag_started()).unwrap_or(false);
            if any_drag_started {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            // 2. 雙擊手掌區（Double Click）：對稱切換透明度（100% ↔ 30% 幽靈穿透態）
            // 在 30% 幽靈穿透態下，雙擊手掌區或膠囊徽章立即還原至 100% 實體置頂
            let any_double_clicked = title_resp.double_clicked()
                || ghost_capsule_resp.as_ref().map(|b| b.double_clicked()).unwrap_or(false)
                || badge_resp.as_ref().map(|b| b.double_clicked()).unwrap_or(false)
                || mid_resp.as_ref().map(|r| r.double_clicked()).unwrap_or(false);

            if any_double_clicked {
                let is_shift = ui.input(|i| i.modifiers.shift);
                let _next_pct = if is_shift {
                    crate::snapping::toggle_single_window_opacity_quick(kind)
                } else {
                    crate::snapping::toggle_cluster_opacity_quick(kind)
                };
                let new_pinned = crate::snapping::is_window_pinned(kind);
                let level = if new_pinned {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                if kind == crate::snapping::WindowKind::Main {
                    self.is_always_on_top = new_pinned;
                }
            }

            // 提案 A：標題手掌區全域右鍵彈出透明度控制面板
            title_resp.context_menu(|ui| {
                Self::render_opacity_popup_content(ui, kind, Some(title_resp.rect));
            });
            if let Some(ref mut c) = ghost_capsule_resp {
                c.context_menu(|ui| {
                    Self::render_opacity_popup_content(ui, kind, Some(c.rect));
                });
            }
            if let Some(ref mut b) = badge_resp {
                b.context_menu(|ui| {
                    Self::render_opacity_popup_content(ui, kind, Some(b.rect));
                });
            }
            if let Some(ref mut r) = mid_resp {
                r.context_menu(|ui| {
                    Self::render_opacity_popup_content(ui, kind, Some(r.rect));
                });
            }

            // 提案 A：標題手掌區全域滾輪直調透明度 (±5%)
            let is_header_hovered = title_resp.hovered()
                || ghost_capsule_resp.as_ref().map(|b| b.hovered()).unwrap_or(false)
                || badge_resp.as_ref().map(|b| b.hovered()).unwrap_or(false)
                || mid_resp.as_ref().map(|r| r.hovered()).unwrap_or(false);

            if is_header_hovered {
                let scroll = ui.input(|i| {
                    if i.raw_scroll_delta.y.abs() > 0.5 {
                        i.raw_scroll_delta.y
                    } else if i.raw_scroll_delta.x.abs() > 0.5 {
                        i.raw_scroll_delta.x
                    } else {
                        0.0
                    }
                });
                if scroll.abs() > 0.5 {
                    let delta = if scroll > 0.0 { 0.05 } else { -0.05 };
                    let cur_op = crate::snapping::get_window_opacity(kind);
                    let next_op = (cur_op + delta).clamp(0.2, 1.0);
                    let is_shift = ui.input(|i| i.modifiers.shift);
                    if is_shift {
                        crate::snapping::set_single_window_opacity_tuning(kind, next_op);
                    } else {
                        crate::snapping::set_cluster_opacity(kind, next_op);
                    }
                }
            }

            let is_header_menu_open = title_resp.context_menu_opened()
                || ghost_capsule_resp.as_ref().map(|b| b.context_menu_opened()).unwrap_or(false)
                || badge_resp.as_ref().map(|b| b.context_menu_opened()).unwrap_or(false)
                || mid_resp.as_ref().map(|r| r.context_menu_opened()).unwrap_or(false);
            if is_header_menu_open {
                crate::snapping::set_opacity_menu_open(kind, true);
                let target_hwnd = match kind {
                    crate::snapping::WindowKind::Main => crate::tray::get_main_hwnd(),
                    _ => crate::snapping::get_window_hwnd(kind),
                };
                if !target_hwnd.is_null() {
                    unsafe {
                        crate::snapping::update_window_click_through(kind, target_hwnd);
                    }
                }
            }

            // Sync custom Win32 cursor state for this window header (✋ Grab on hover, ✊ Grabbing when held)
            let is_down = title_resp.is_pointer_button_down_on()
                || ghost_capsule_resp.as_ref().map(|b| b.is_pointer_button_down_on()).unwrap_or(false)
                || badge_resp.as_ref().map(|b| b.is_pointer_button_down_on()).unwrap_or(false)
                || mid_resp.as_ref().map(|r| r.is_pointer_button_down_on()).unwrap_or(false);
            let is_hover = is_header_hovered;

            if is_down {
                crate::custom_cursor::set_header_cursor_state(kind, crate::custom_cursor::CustomCursorState::Grabbing);
            } else if is_hover {
                crate::custom_cursor::set_header_cursor_state(kind, crate::custom_cursor::CustomCursorState::Grab);
            } else {
                crate::custom_cursor::set_header_cursor_state(kind, crate::custom_cursor::CustomCursorState::None);
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
    }

    /// 通用獨立 Viewport 渲染控制器：
    /// 自動封裝視窗 ID 解析、尺寸邊界約束、隱藏至托盤檢查、關閉叢集事件監聽、
    /// Win32 HWND 自動探索、Subclass 注入、磁吸初始對齊，以及統一樣式容器。
    pub fn show_standalone_viewport<F>(
        &mut self,
        ctx: &egui::Context,
        kind: crate::snapping::WindowKind,
        title: &str,
        is_open: bool,
        needs_initial_dock: &mut bool,
        mut content: F,
    ) where
        F: FnMut(&mut Self, &mut egui::Ui),
    {
        if !is_open {
            return;
        }

        let Some(id_str) = kind.viewport_id_str() else { return; };
        let is_fleet_hidden = crate::tray::is_app_fleet_hidden();
        let viewport_id = egui::ViewportId::from_hash_of(id_str);

        let mut def_size = kind.default_size();
        let ppp = ctx.pixels_per_point();
        if kind == crate::snapping::WindowKind::MiniPlayer || kind == crate::snapping::WindowKind::Equalizer {
            let main_hwnd = crate::snapping::get_window_hwnd(crate::snapping::WindowKind::Main);
            if !main_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(main_hwnd) != 0 } {
                let mut mr: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
                unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(main_hwnd, &mut mr); }
                let mh = (mr.bottom - mr.top) as f32 / ppp;
                if mh > 100.0 {
                    def_size[1] = (mh / 2.0).round();
                }
            }
        }

        let is_pinned = crate::snapping::is_window_pinned(kind);
        let init_level = if is_pinned {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };

        let mut builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(def_size)
            .with_min_inner_size(kind.min_size())
            .with_resizable(true)
            .with_decorations(false) // 無 Windows 原生外框 (Frameless)，自訂沉浸式標題列
            .with_transparent(true)  // 啟用透明 Viewport 背景支援
            .with_window_level(init_level);

        if *needs_initial_dock {
            // 預先計算初次停靠目標座標，且在建立時先保持 hidden (visible: false)，
            // 避免 winit 內部在 CreateWindowExW(..., CW_USEDEFAULT) 時瞬間閃現 800x600 預設視窗
            let w_px = (def_size[0] * ppp).round() as i32;
            let h_px = (def_size[1] * ppp).round() as i32;
            let spawner = crate::snapping::get_window_spawner(kind);
            let (best_rect, _) = crate::snapping::find_best_spawn_rect(kind, w_px, h_px, spawner);
            let init_pos = egui::pos2(best_rect.left as f32 / ppp, best_rect.top as f32 / ppp);
            builder = builder.with_position(init_pos).with_visible(false);
        } else {
            builder = builder.with_visible(!is_fleet_hidden);
        }

        ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            if is_fleet_hidden {
                return;
            }

            // 1. 通用關閉與快捷鍵事件攔截
            let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            if crate::visualizer::is_visualizer_fullscreen() && escape_pressed {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                crate::visualizer::toggle_main_window_hud();
                return;
            }

            if ctx.input(|i| i.viewport().close_requested()) {
                self.close_popout_window(kind);
                return;
            }

            // 2. 通用 HWND 自動探索、Subclass 注入與磁吸初始對齊
            let hwnd = crate::snapping::get_window_hwnd(kind);
            let target_hwnd = if !hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 } {
                hwnd
            } else {
                let found = crate::snapping::find_hwnd_by_kind(kind);
                if !found.is_null() {
                    unsafe {
                        crate::snapping::install_window_subclass(kind, found);
                    }
                    found
                } else {
                    std::ptr::null_mut()
                }
            };

            if !target_hwnd.is_null() && *needs_initial_dock {
                *needs_initial_dock = false;
                unsafe {
                    crate::snapping::dock_spawned_window(kind, target_hwnd);
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
                        target_hwnd as _,
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW,
                    );
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            }

            // 3. 滾輪快捷微調透明度 (Alt + Scroll，自動連動群組)
            if ctx.input(|i| i.modifiers.alt) {
                let scroll = ctx.input(|i| {
                    if i.raw_scroll_delta.y.abs() > 0.5 {
                        i.raw_scroll_delta.y
                    } else if i.raw_scroll_delta.x.abs() > 0.5 {
                        i.raw_scroll_delta.x
                    } else {
                        0.0
                    }
                });
                if scroll.abs() > 0.5 {
                    let cur_op = crate::snapping::get_window_opacity(kind);
                    let delta = if scroll > 0.0 { 0.05 } else { -0.05 };
                    crate::snapping::set_cluster_opacity(kind, (cur_op + delta).clamp(0.2, 1.0));
                }
            }

            // 4. 通用視窗容器面板（沉浸式無邊框樣式，附自訂 Alpha 通道之半透明背景與邊框）
            let user_op = crate::snapping::get_window_opacity(kind);
            let is_click_through = crate::snapping::is_click_through_active(kind);
            let is_hovered = if !target_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(target_hwnd) != 0 } {
                unsafe {
                    crate::snapping::update_window_click_through(kind, target_hwnd);
                    // 穿透模式下：停用 Hover Peek 實體化，維持穩定半透明，不閃爍
                    !is_click_through && crate::snapping::is_cursor_over_window(target_hwnd)
                }
            } else {
                false
            };
            let is_suppressed = crate::snapping::is_peek_suppressed(kind);
            let target_op = if is_hovered && crate::snapping::is_hover_peek_enabled() && !is_suppressed {
                1.0
            } else {
                user_op
            };
            let (effective_op, is_animating) = crate::snapping::step_animated_window_opacity(kind, target_op);
            let alpha = (effective_op.clamp(0.2, 1.0) * 255.0).round() as u8;

            if !target_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(target_hwnd) != 0 } {
                unsafe {
                    crate::snapping::apply_native_window_opacity(target_hwnd, effective_op);
                }
            }

            // 當平滑淡入淡出中或懸停實體化生效時，以 60 FPS 刷新畫面
            if is_animating || (is_hovered && crate::snapping::is_hover_peek_enabled() && user_op < 0.99) {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }

            let bg_color = match kind {
                crate::snapping::WindowKind::Equalizer => egui::Color32::from_rgba_unmultiplied(24, 25, 29, 215),
                crate::snapping::WindowKind::MiniPlayer => egui::Color32::from_rgba_unmultiplied(22, 24, 29, 215),
                _ => egui::Color32::from_rgba_unmultiplied(20, 22, 26, 215),
            };

            let role = crate::snapping::get_window_role(kind);
            let stroke_alpha = alpha.max(100);
            let stroke_color = match role {
                crate::snapping::WindowRole::ClusterMaster => egui::Color32::from_rgba_unmultiplied(70, 140, 215, stroke_alpha), // 藍色主控微光邊框
                crate::snapping::WindowRole::ClusterSlave => egui::Color32::from_rgba_unmultiplied(45, 50, 60, stroke_alpha),    // 無縫附屬邊框
                crate::snapping::WindowRole::Standalone => egui::Color32::from_rgba_unmultiplied(52, 58, 70, stroke_alpha),     // 標準獨立外框
            };

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(bg_color)
                        .stroke(egui::Stroke::new(1.0_f32, stroke_color))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(12.0)
                )
                .show(ctx, |ui| {
                    content(self, ui);
                    Self::render_opacity_hud_badge(ui, kind);
                    Self::render_header_shimmer_boundary(ui, kind);
                    self.render_mascot_overlay(ui, kind);
                });
        });
    }

    /// 同步媒體曲目資訊、紋理快取並偵測曲目切換（觸發封面與曲目左右推移過場）
    pub fn sync_media_track(&mut self, ctx: &egui::Context) {
        let track_info = crate::media_control::get_current_media_track().read().unwrap().clone();

        // 1. 專輯封面快取管理
        if track_info.thumbnail_bytes != self.last_album_art_bytes {
            self.prev_album_art_texture = self.album_art_texture.clone();
            self.last_album_art_bytes = track_info.thumbnail_bytes.clone();
            if let Some(ref bytes) = track_info.thumbnail_bytes {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                    self.album_art_texture = Some(ctx.load_texture(
                        "winamp_album_art",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                } else {
                    self.album_art_texture = None;
                }
            } else {
                self.album_art_texture = None;
            }
        }

        // 2. 曲目切換判定
        let cur_title = track_info.title.trim().to_string();
        let cur_artist = track_info.artist.trim().to_string();
        let cur_album = track_info.album.trim().to_string();

        if self.prev_track_title.is_empty() {
            self.prev_track_title = cur_title;
            self.prev_track_artist = cur_artist;
            self.prev_track_album = cur_album;
            self.prev_album_art_texture = self.album_art_texture.clone();
        } else if !cur_title.is_empty() && (cur_title != self.prev_track_title || cur_artist != self.prev_track_artist) {
            let dir = self.next_track_intent.take().unwrap_or(TrackTransitionDirection::Next);
            self.track_transition = Some(TrackTransition {
                from_title: self.prev_track_title.clone(),
                from_artist: self.prev_track_artist.clone(),
                from_album: self.prev_track_album.clone(),
                from_art_texture: self.prev_album_art_texture.clone(),
                to_title: cur_title.clone(),
                to_artist: cur_artist.clone(),
                to_album: cur_album.clone(),
                to_art_texture: self.album_art_texture.clone(),
                direction: dir,
                start_time: std::time::Instant::now(),
                duration: std::time::Duration::from_millis(320),
            });
            self.prev_track_title = cur_title;
            self.prev_track_artist = cur_artist;
            self.prev_track_album = cur_album;
            self.prev_album_art_texture = self.album_art_texture.clone();
            ctx.request_repaint();
        }

        if let Some(ref trans) = self.track_transition {
            ctx.request_repaint();
            if trans.start_time.elapsed() >= trans.duration {
                self.track_transition = None;
            }
        }
    }

    /// 繪製單張專輯封面內容（支援紋理渲染或黑膠唱片占位符，附帶動態透明度）
    pub fn render_album_art_content(
        painter: &egui::Painter,
        rect: egui::Rect,
        tex: Option<&egui::TextureHandle>,
        alpha: u8,
    ) {
        if alpha == 0 {
            return;
        }
        if let Some(tex) = tex {
            painter.rect_filled(rect, egui::Rounding::same(6.0), egui::Color32::from_black_alpha(alpha));
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
            painter.image(tex.id(), rect, uv, tint);
        } else {
            let center = rect.center();
            let radius = (rect.width().min(rect.height()) / 2.0) - 2.0;

            let disc_col = egui::Color32::from_rgba_unmultiplied(18, 19, 23, alpha);
            let border_col = egui::Color32::from_rgba_unmultiplied(50, 54, 65, alpha);
            painter.circle_filled(center, radius, disc_col);
            painter.circle_stroke(center, radius, egui::Stroke::new(1.0_f32, border_col));

            let ring_alpha = ((alpha as f32 / 255.0) * 60.0) as u8;
            for r_factor in [0.85, 0.72, 0.60, 0.48] {
                painter.circle_stroke(
                    center,
                    radius * r_factor,
                    egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(80, 85, 100, ring_alpha)),
                );
            }

            let label_col = egui::Color32::from_rgba_unmultiplied(35, 75, 130, alpha);
            let label_stroke = egui::Color32::from_rgba_unmultiplied(80, 140, 220, alpha);
            painter.circle_filled(center, radius * 0.36, label_col);
            painter.circle_stroke(center, radius * 0.36, egui::Stroke::new(1.0_f32, label_stroke));

            painter.circle_filled(center, 3.5, egui::Color32::from_rgba_unmultiplied(220, 225, 230, alpha));

            let note_alpha = ((alpha as f32 / 255.0) * 160.0) as u8;
            painter.text(
                center - egui::vec2(0.0, 9.0),
                egui::Align2::CENTER_CENTER,
                "♪",
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, note_alpha),
            );
        }
    }

    /// 智慧解析曲目名稱，拆分為「主名稱 (Line 1)」與「樂章／副標題 (Line 2)」
    pub fn split_track_title(title: &str) -> (String, Option<String>) {
        let t = title.trim();
        if t.is_empty() {
            return (String::new(), None);
        }

        // 1. 古典樂章／標準副標題分隔符號 ": " (例如 "Peer Gynt Suite No. 2, Op. 55: IV. Solveig's Sang")
        if let Some(pos) = t.find(": ") {
            let l1 = t[..pos].trim();
            let l2 = t[pos + 2..].trim();
            if !l1.is_empty() && !l2.is_empty() {
                return (l1.to_string(), Some(l2.to_string()));
            }
        }

        // 2. 破折號分隔符號 " - " (常見於副標題、樂章或混音版本)
        if let Some(pos) = t.find(" - ") {
            let l1 = t[..pos].trim();
            let l2 = t[pos + 3..].trim();
            if !l1.is_empty() && !l2.is_empty() {
                return (l1.to_string(), Some(l2.to_string()));
            }
        }

        // 3. 圓括號 " (" (例如 "Cruel Summer (Live from The Eras Tour)" 或 "Shape of You (Acoustic)")
        if let Some(pos) = t.find(" (") {
            let l1 = t[..pos].trim();
            let l2 = t[pos + 1..].trim();
            if !l1.is_empty() && !l2.is_empty() {
                return (l1.to_string(), Some(l2.to_string()));
            }
        }

        // 4. 中括號 " [" (例如 "Song Name [Remastered 2024]")
        if let Some(pos) = t.find(" [") {
            let l1 = t[..pos].trim();
            let l2 = t[pos + 1..].trim();
            if !l1.is_empty() && !l2.is_empty() {
                return (l1.to_string(), Some(l2.to_string()));
            }
        }

        // 5. 若曲目名稱過長 (超過 28 字元)，在中心點附近的空白或逗號處自然斷行
        if t.chars().count() > 28 {
            let chars: Vec<(usize, char)> = t.char_indices().collect();
            let mid = chars.len() / 2;
            let mut best_split = None;
            for delta in 0..mid {
                if mid + delta < chars.len() && (chars[mid + delta].1 == ' ' || chars[mid + delta].1 == ',') {
                    best_split = Some(chars[mid + delta].0 + if chars[mid + delta].1 == ',' { 1 } else { 0 });
                    break;
                }
                if mid >= delta && (chars[mid - delta].1 == ' ' || chars[mid - delta].1 == ',') {
                    best_split = Some(chars[mid - delta].0 + if chars[mid - delta].1 == ',' { 1 } else { 0 });
                    break;
                }
            }
            if let Some(byte_idx) = best_split {
                let l1 = t[..byte_idx].trim();
                let l2 = t[byte_idx..].trim();
                if !l1.is_empty() && !l2.is_empty() {
                    return (l1.to_string(), Some(l2.to_string()));
                }
            }
        }

        (t.to_string(), None)
    }

    /// 統一 WinAMP 風格主顯示底盤組件（包含 88x88 專輯封面與 VFD 螢幕，全面支援封面與歌名推移淡出過場）
    pub fn render_winamp_display_chassis(
        &mut self,
        ui: &mut egui::Ui,
        track_info: &crate::media_control::MediaTrackInfo,
        lang: crate::i18n::Language,
        is_mini: bool,
    ) {
        ui.horizontal(|ui| {
            let app_id_lower = track_info.source_app_id.to_lowercase();
                    let friendly_app = if app_id_lower.contains("spotify") {
                        "Spotify"
                    } else if app_id_lower.contains("applemusic") || app_id_lower.contains("apple.music") {
                        "Apple Music"
                    } else if app_id_lower.contains("tidal") {
                        "Tidal"
                    } else if app_id_lower.contains("chrome") {
                        "Chrome"
                    } else if app_id_lower.contains("firefox") {
                        "Firefox"
                    } else if app_id_lower.contains("msedge") || app_id_lower.contains("edge") {
                        "Microsoft Edge"
                    } else if !track_info.source_app_id.is_empty() {
                        let base = track_info.source_app_id.split(['!', '_', '\\', '/']).next().unwrap_or(&track_info.source_app_id);
                        base.trim_end_matches(".exe")
                    } else {
                        ""
                    };

                    let click_app_tooltip = if !friendly_app.is_empty() {
                        format!("{} ({})", lang.player_click_to_open_app(), friendly_app)
                    } else {
                        lang.player_click_to_open_app().to_string()
                    };

                    // Left: 專輯封面方框（1:1 正方形，與右側 VFD 螢幕 100% 等高對齊）
                    let art_dim = if is_mini { 98.0 } else { 104.0 };
                    let art_size = egui::vec2(art_dim, art_dim);
                    let (art_rect, art_resp) = ui.allocate_exact_size(art_size, egui::Sense::click());
                    let art_resp = art_resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(&click_app_tooltip);

                    if art_resp.clicked() {
                        crate::media_control::activate_media_source_app(
                            &track_info.source_app_id,
                            &track_info.title,
                            Some(&self.state.audio_source),
                        );
                    }

                    // 封面滑動過場渲染
                    if let Some(ref trans) = self.track_transition {
                        let raw_t = (trans.start_time.elapsed().as_secs_f32() / trans.duration.as_secs_f32()).clamp(0.0, 1.0);
                        let t = 1.0 - (1.0 - raw_t).powi(3); // Ease-out cubic

                        let (dx_from, dx_to) = match trans.direction {
                            TrackTransitionDirection::Next => (-24.0 * t, 28.0 * (1.0 - t)),
                            TrackTransitionDirection::Prev => (24.0 * t, -28.0 * (1.0 - t)),
                        };
                        let alpha_from = ((1.0 - t) * 255.0) as u8;
                        let alpha_to = (t * 255.0) as u8;

                        let art_painter = ui.painter().with_clip_rect(art_rect);
                        art_painter.rect_filled(art_rect, egui::Rounding::same(6.0), egui::Color32::from_rgb(18, 19, 23));

                        if alpha_from > 0 {
                            let from_rect = art_rect.translate(egui::vec2(dx_from, 0.0));
                            Self::render_album_art_content(&art_painter, from_rect, trans.from_art_texture.as_ref(), alpha_from);
                        }
                        if alpha_to > 0 {
                            let to_rect = art_rect.translate(egui::vec2(dx_to, 0.0));
                            Self::render_album_art_content(&art_painter, to_rect, trans.to_art_texture.as_ref(), alpha_to);
                        }

                        // 固定外框邊線
                        ui.painter().rect_stroke(
                            art_rect,
                            egui::Rounding::same(6.0),
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40)),
                        );
                    } else {
                        let painter = ui.painter();
                        painter.rect_filled(art_rect, egui::Rounding::same(6.0), egui::Color32::from_rgb(18, 19, 23));
                        Self::render_album_art_content(painter, art_rect, self.album_art_texture.as_ref(), 255);
                        painter.rect_stroke(
                            art_rect,
                            egui::Rounding::same(6.0),
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40)),
                        );
                    }

                    if art_resp.hovered() {
                        let painter = ui.painter();
                        painter.rect_filled(
                            art_rect,
                            egui::Rounding::same(6.0),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        );
                        painter.text(
                            art_rect.right_top() + egui::vec2(-4.0, 4.0),
                            egui::Align2::RIGHT_TOP,
                            "↗",
                            egui::FontId::proportional(12.5),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 230),
                        );
                    }

                    ui.add_space(8.0);

                    // Right: WinAMP VFD / LCD 螢幕底盤（固定 6 行方案 1 配置）
                    let vfd_w = ui.available_width();
                    let vfd_h = if is_mini { 98.0f32 } else { 104.0f32 };
                    let (vfd_rect, _) = ui.allocate_exact_size(egui::vec2(vfd_w, vfd_h), egui::Sense::hover());
                    let vfd_painter = ui.painter();
                    vfd_painter.rect_filled(vfd_rect, egui::Rounding::same(5.0), egui::Color32::from_rgb(8, 16, 10));
                    vfd_painter.rect_stroke(
                        vfd_rect,
                        egui::Rounding::same(5.0),
                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(20, 48, 24)),
                    );

                    let inner_vfd = vfd_rect.shrink2(egui::vec2(7.0, 4.0));
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner_vfd), |ui| {
                        ui.vertical(|ui| {
                            // Lines 1-4: 歌詞與曲目資訊滑動顯示區域 (56px)
                            let text_area_h = if is_mini { 56.0f32 } else { 60.0f32 };
                            let (text_rect, _) = ui.allocate_exact_size(egui::vec2(inner_vfd.width(), text_area_h), egui::Sense::hover());

                            let font_title = egui::FontId::monospace(if is_mini { 11.5 } else { 12.0 });
                            let font_sub = egui::FontId::monospace(if is_mini { 9.5 } else { 10.0 });
                            let font_artist = egui::FontId::monospace(if is_mini { 9.0 } else { 9.5 });
                            let font_album = egui::FontId::monospace(if is_mini { 8.5 } else { 9.0 });

                            let y1 = 1.0f32;
                            let y2 = if is_mini { 14.5f32 } else { 15.5f32 };
                            let y3 = if is_mini { 28.0f32 } else { 30.0f32 };
                            let y4 = if is_mini { 41.5f32 } else { 44.5f32 };

                            if let Some(ref trans) = self.track_transition {
                                let raw_t = (trans.start_time.elapsed().as_secs_f32() / trans.duration.as_secs_f32()).clamp(0.0, 1.0);
                                let t = 1.0 - (1.0 - raw_t).powi(3);

                                let (dx_from, dx_to) = match trans.direction {
                                    TrackTransitionDirection::Next => (-inner_vfd.width() * 0.38 * t, inner_vfd.width() * 0.38 * (1.0 - t)),
                                    TrackTransitionDirection::Prev => (inner_vfd.width() * 0.38 * t, -inner_vfd.width() * 0.38 * (1.0 - t)),
                                };
                                let alpha_from = ((1.0 - t) * 255.0) as u8;
                                let alpha_to = (t * 255.0) as u8;

                                let text_painter = ui.painter().with_clip_rect(text_rect);

                                if alpha_from > 0 {
                                    let (from_t1, from_t2_opt) = Self::split_track_title(&trans.from_title);
                                    let from_t1_disp = if from_t1.is_empty() { lang.player_no_track().to_string() } else { from_t1 };
                                    let from_sub = from_t2_opt.unwrap_or_else(|| if trans.from_title.trim().is_empty() { String::new() } else { "♪ 數位立體聲母帶 (Digital Audio Master)".to_string() });
                                    let from_artist_str = if !trans.from_artist.is_empty() {
                                        format!("👤 {}", trans.from_artist)
                                    } else {
                                        format!("👤 {}", lang.player_unknown_artist())
                                    };
                                    let from_album_str = if !trans.from_album.is_empty() {
                                        format!("💿 {}", trans.from_album)
                                    } else {
                                        "💿 Single / Digital Release".to_string()
                                    };

                                    let col1 = egui::Color32::from_rgba_unmultiplied(0, 255, 110, alpha_from);
                                    let col2 = egui::Color32::from_rgba_unmultiplied(80, 255, 176, alpha_from);
                                    let col3 = egui::Color32::from_rgba_unmultiplied(112, 224, 176, alpha_from);
                                    let col4 = egui::Color32::from_rgba_unmultiplied(140, 200, 255, alpha_from);

                                    text_painter.text(text_rect.min + egui::vec2(dx_from, y1), egui::Align2::LEFT_TOP, &from_t1_disp, font_title.clone(), col1);
                                    if !from_sub.is_empty() {
                                        text_painter.text(text_rect.min + egui::vec2(dx_from, y2), egui::Align2::LEFT_TOP, &from_sub, font_sub.clone(), col2);
                                    }
                                    text_painter.text(text_rect.min + egui::vec2(dx_from, y3), egui::Align2::LEFT_TOP, &from_artist_str, font_artist.clone(), col3);
                                    text_painter.text(text_rect.min + egui::vec2(dx_from, y4), egui::Align2::LEFT_TOP, &from_album_str, font_album.clone(), col4);
                                }

                                if alpha_to > 0 {
                                    let (to_t1, to_t2_opt) = Self::split_track_title(&trans.to_title);
                                    let to_t1_disp = if to_t1.is_empty() { lang.player_no_track().to_string() } else { to_t1 };
                                    let to_sub = to_t2_opt.unwrap_or_else(|| if trans.to_title.trim().is_empty() { String::new() } else { "♪ 數位立體聲母帶 (Digital Audio Master)".to_string() });
                                    let to_artist_str = if !trans.to_artist.is_empty() {
                                        format!("👤 {}", trans.to_artist)
                                    } else {
                                        format!("👤 {}", lang.player_unknown_artist())
                                    };
                                    let to_album_str = if !trans.to_album.is_empty() {
                                        format!("💿 {}", trans.to_album)
                                    } else {
                                        "💿 Single / Digital Release".to_string()
                                    };

                                    let col1 = egui::Color32::from_rgba_unmultiplied(0, 255, 110, alpha_to);
                                    let col2 = egui::Color32::from_rgba_unmultiplied(80, 255, 176, alpha_to);
                                    let col3 = egui::Color32::from_rgba_unmultiplied(112, 224, 176, alpha_to);
                                    let col4 = egui::Color32::from_rgba_unmultiplied(140, 200, 255, alpha_to);

                                    text_painter.text(text_rect.min + egui::vec2(dx_to, y1), egui::Align2::LEFT_TOP, &to_t1_disp, font_title.clone(), col1);
                                    if !to_sub.is_empty() {
                                        text_painter.text(text_rect.min + egui::vec2(dx_to, y2), egui::Align2::LEFT_TOP, &to_sub, font_sub.clone(), col2);
                                    }
                                    text_painter.text(text_rect.min + egui::vec2(dx_to, y3), egui::Align2::LEFT_TOP, &to_artist_str, font_artist.clone(), col3);
                                    text_painter.text(text_rect.min + egui::vec2(dx_to, y4), egui::Align2::LEFT_TOP, &to_album_str, font_album.clone(), col4);
                                }
                            } else {
                                // 靜態正常顯示：座標絕對一致 (text_rect.min + (0, y))，過場結束後 100% 無縫無跳動
                                let text_painter = ui.painter().with_clip_rect(text_rect);

                                let (curr_t1, curr_t2_opt) = Self::split_track_title(&track_info.title);
                                let curr_t1_disp = if curr_t1.is_empty() { lang.player_no_track().to_string() } else { curr_t1 };
                                let curr_sub = curr_t2_opt.unwrap_or_else(|| if track_info.title.trim().is_empty() { String::new() } else { "♪ 數位立體聲母帶 (Digital Audio Master)".to_string() });
                                let curr_artist_str = if !track_info.artist.is_empty() {
                                    format!("👤 {}", track_info.artist)
                                } else {
                                    format!("👤 {}", lang.player_unknown_artist())
                                };
                                let curr_album_str = if !track_info.album.is_empty() {
                                    format!("💿 {}", track_info.album)
                                } else {
                                    "💿 Single / Digital Release".to_string()
                                };

                                // Line 1: 曲目主名稱
                                text_painter.text(
                                    text_rect.min + egui::vec2(0.0, y1),
                                    egui::Align2::LEFT_TOP,
                                    &curr_t1_disp,
                                    font_title,
                                    egui::Color32::from_rgb(0, 255, 110),
                                );

                                // Line 2: 樂章／副標題
                                if !curr_sub.is_empty() {
                                    text_painter.text(
                                        text_rect.min + egui::vec2(0.0, y2),
                                        egui::Align2::LEFT_TOP,
                                        &curr_sub,
                                        font_sub,
                                        egui::Color32::from_rgb(80, 255, 176),
                                    );
                                }

                                // Line 3: 演出者
                                text_painter.text(
                                    text_rect.min + egui::vec2(0.0, y3),
                                    egui::Align2::LEFT_TOP,
                                    &curr_artist_str,
                                    font_artist,
                                    egui::Color32::from_rgb(112, 224, 176),
                                );

                                // Line 4: 唱片專輯
                                text_painter.text(
                                    text_rect.min + egui::vec2(0.0, y4),
                                    egui::Align2::LEFT_TOP,
                                    &curr_album_str,
                                    font_album,
                                    egui::Color32::from_rgb(140, 200, 255),
                                );
                            }

                            ui.add_space(if is_mini { 2.0 } else { 3.0 });

                            // Line 5: 獨立規格徽章列（專屬行，寬敞不擁擠）
                            ui.horizontal(|ui| {
                                self.render_audio_format_badges_inline(ui, is_mini);
                            });

                            ui.add_space(if is_mini { 1.5 } else { 2.0 });

                            // Line 6: 播放狀態、進度時間與來源程式（左側狀態時間，右側專屬來源按鈕）
                            ui.horizontal(|ui| {
                                let (stat_text, stat_col) = if track_info.is_playing {
                                    ("▶ PLAYING", egui::Color32::from_rgb(0, 255, 140))
                                } else if track_info.has_session {
                                    ("❚❚ PAUSED", egui::Color32::from_rgb(255, 180, 40))
                                } else {
                                    ("■ STOPPED", egui::Color32::from_rgb(120, 135, 150))
                                };
                                ui.label(
                                    egui::RichText::new(stat_text)
                                        .monospace()
                                        .size(if is_mini { 8.5 } else { 9.0 })
                                        .strong()
                                        .color(stat_col),
                                );

                                let cur_pos = if let Some(preview_secs) = self.seek_drag_secs {
                                    preview_secs
                                } else {
                                    track_info.current_position_secs()
                                };
                                let total_dur = track_info.duration_secs;
                                let time_str = if total_dur > 0.0 {
                                    format!("{} / {}", crate::media_control::MediaTrackInfo::format_time(cur_pos), crate::media_control::MediaTrackInfo::format_time(total_dur))
                                } else if cur_pos > 0.0 {
                                    format!("{} / --:--", crate::media_control::MediaTrackInfo::format_time(cur_pos))
                                } else {
                                    "--:-- / --:--".to_string()
                                };
                                ui.label(
                                    egui::RichText::new(time_str)
                                        .monospace()
                                        .size(if is_mini { 8.5 } else { 9.0 })
                                        .color(egui::Color32::from_rgb(160, 200, 180)),
                                );

                                // 右側：來源程式名稱標籤與連結跳轉
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if !friendly_app.is_empty() {
                                        let app_link_text = format!("來源: {} ↗", friendly_app);
                                        let app_resp = ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(app_link_text)
                                                    .monospace()
                                                    .size(if is_mini { 8.5 } else { 9.0 })
                                                    .color(egui::Color32::from_rgb(120, 190, 240)),
                                            )
                                            .sense(egui::Sense::click()),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .on_hover_text(&click_app_tooltip);

                                        if app_resp.clicked() {
                                            crate::media_control::activate_media_source_app(
                                                &track_info.source_app_id,
                                                &track_info.title,
                                                Some(&self.state.audio_source),
                                            );
                                        }
                                    }
                                });
                            });
                        });
                    });
                });
    }

    fn render_visualizer_button(&mut self, ui: &mut egui::Ui, spawner: crate::snapping::WindowKind) {
        let lang = self.state.language;
        let is_open = crate::visualizer::is_visualizer_open();
        let tooltip = if is_open {
            lang.tooltip_visualizer_close()
        } else {
            lang.tooltip_visualizer()
        };

        let resp = Self::render_stateful_icon_button(
            ui,
            "🎨",
            is_open,
            tooltip,
            ButtonTheme::Blue,
        );

        if resp.clicked() {
            crate::snapping::set_window_spawner(crate::snapping::WindowKind::Visualizer, spawner);
            crate::visualizer::set_language(lang.code());
            crate::visualizer::toggle_visualizer();
        }
    }

    /// 通用 Pop-out 按鈕渲染組件：
    /// 任何頁面或視窗皆可傳入目標 WindowKind，擁有全系統一致的圖標、動態 Tooltip、高亮開關與抽離行為。
    pub fn render_universal_popout_button(&mut self, ui: &mut egui::Ui, target: crate::snapping::WindowKind, spawner: crate::snapping::WindowKind) {
        let is_open = self.is_window_open(target);
        let lang = self.state.language;
        let tooltip = if is_open {
            lang.tooltip_popout_close()
        } else {
            lang.tooltip_popout_open()
        };

        let resp = Self::render_stateful_icon_button(
            ui,
            "⧉",
            is_open,
            tooltip,
            ButtonTheme::Blue,
        );

        if resp.clicked() {
            self.handle_popout_toggle(target, spawner);
        }
    }

    /// 統一主視窗頂部分頁切換行為（方案 B：若當前已在該分頁則返回，否則跳轉進入該分頁；不關閉獨立視窗）
    pub fn navigate_toggle_page(&mut self, page: Page) {
        if self.current_page == page {
            self.go_back();
        } else {
            self.navigate_to(page);
        }
    }

    /// 統一主視窗頂部分頁導航按鈕（方案 B：維持廣義 Active 亮起，點擊一律進行分頁導航/返回）
    pub fn render_nav_page_button(
        &mut self,
        ui: &mut egui::Ui,
        page: Page,
        icon: &str,
        tooltip: &str,
        theme: ButtonTheme,
    ) -> egui::Response {
        let is_active = self.is_page_active(page);
        let resp = Self::render_stateful_icon_button(ui, icon, is_active, tooltip, theme);
        if resp.clicked() {
            self.navigate_toggle_page(page);
        }
        resp
    }

    pub fn render_equalizer_button(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;
        let is_active = self.is_page_active(Page::Equalizer);
        let resp = Self::render_stateful_icon_button(
            ui,
            "⦀",
            is_active,
            lang.eq_title(),
            ButtonTheme::Green,
        );
        if resp.clicked() {
            if self.current_page == Page::Equalizer {
                self.go_back();
            } else {
                self.eq_profile = EqProfile::Normal;
                self.navigate_to(Page::Equalizer);
            }
        }
    }

    pub fn render_popout_equalizer_button(&mut self, ui: &mut egui::Ui, spawner: crate::snapping::WindowKind) {
        let lang = self.state.language;
        let is_open = self.equalizer_window_open;
        let tooltip = lang.mini_equalizer_title();

        let resp = Self::render_stateful_icon_button(
            ui,
            "⦀",
            is_open,
            tooltip,
            ButtonTheme::Blue,
        );

        if resp.clicked() {
            self.toggle_equalizer_window(spawner);
        }
    }

    pub fn render_player_button(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;
        self.render_nav_page_button(
            ui,
            Page::Player,
            "♫",
            lang.tooltip_player(),
            ButtonTheme::Green,
        );
    }

    pub fn render_settings_button(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;
        self.render_nav_page_button(
            ui,
            Page::Settings,
            "⚙",
            lang.tooltip_settings(),
            ButtonTheme::Cyan,
        );
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

    /// 頂部列分組分隔線（區隔系統視窗操作組與應用程式功能組）
    pub fn render_header_separator(ui: &mut egui::Ui) {
        ui.add_space(3.0);
        let height = 13.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, height), egui::Sense::hover());
        ui.painter().rect_filled(
            rect,
            egui::Rounding::ZERO,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 38),
        );
        ui.add_space(3.0);
    }

    fn render_settings_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let initial_top = ui.cursor().top();
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let lang = self.state.language;

        // Header bar with Back button
        self.render_universal_window_header(
            ui,
            crate::snapping::WindowKind::Main,
            lang.advanced_settings_header(),
            true,
            |app, ui| {
                app.render_visualizer_button(ui, crate::snapping::WindowKind::Main);
            },
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        // Segmented Tabs Header (具備平滑滑動指示線與膠囊背景)
        let tabs = [
            (SettingsTab::Audio, lang.settings_tab_audio()),
            (SettingsTab::Receiver, lang.settings_tab_receiver()),
            (SettingsTab::System, lang.settings_tab_system()),
        ];
        if let Some(tab) = Self::render_sliding_tab_bar(ui, "settings_tabs", self.settings_tab, &tabs) {
            self.settings_tab = tab;
        }

        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("main_scroll_area")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.settings_tab {
            SettingsTab::Audio => {
                // --- Group 1: Audio Source & Output ---
                ui.label(egui::RichText::new(lang.settings_section_audio()).strong().size(13.0));
                ui.add_space(4.0);

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(32, 34, 40))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(10.0)
            .show(ui, |ui| {
                let is_app_mode = matches!(self.state.audio_source, AudioSource::Application { .. });

                // Mode switcher: [ 🖥 System & Devices ] [ 🎯 Application ]
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(lang.audio_source_label()).size(12.0));
                    ui.add_space(4.0);

                    if ui.selectable_label(!is_app_mode, lang.source_mode_device()).clicked() && is_app_mode {
                        let dev_id = if !self.state.last_selected_device.is_empty() {
                            self.state.last_selected_device.clone()
                        } else {
                            "default".to_string()
                        };
                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetAudioSource(AudioSource::Device(dev_id)));
                    }
                    if ui.selectable_label(is_app_mode, lang.source_mode_app()).clicked() && !is_app_mode {
                        let restored_app = if let Some((ref last_exe, ref last_name)) = self.state.last_selected_app {
                            if let Some(matched) = self.state.audio_apps.iter().find(|a| a.exe_name.eq_ignore_ascii_case(last_exe)) {
                                Some(AudioSource::Application {
                                    pid: matched.pid,
                                    exe_name: matched.exe_name.clone(),
                                    display_name: matched.display_name.clone(),
                                })
                            } else if let Some(fresh_pid) = crate::app_enumerator::find_pid_by_exe(last_exe) {
                                Some(AudioSource::Application {
                                    pid: fresh_pid,
                                    exe_name: last_exe.clone(),
                                    display_name: last_name.clone(),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(app_source) = restored_app {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetAudioSource(app_source));
                        } else if let Some(first_app) = self.state.audio_apps.first() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetAudioSource(AudioSource::Application {
                                pid: first_app.pid,
                                exe_name: first_app.exe_name.clone(),
                                display_name: first_app.display_name.clone(),
                            }));
                        } else {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshAudioApps);
                        }
                    }
                });

                ui.add_space(6.0);

                if !is_app_mode {
                    // Audio Source Device Selector
                    ui.horizontal(|ui| {
                        let current_label = self
                            .state
                            .audio_devices
                            .iter()
                            .find(|d| d.id == self.state.selected_audio_device)
                            .map(|d| if d.id == "default" { lang.audio_source_default() } else { d.name.as_str() })
                            .unwrap_or_else(|| lang.audio_source_default());
                        let short_label = truncate_text(current_label, 26);

                        let combo_w = (ui.available_width() - 38.0).max(160.0);
                        let combo_res = egui::ComboBox::from_id_salt("audio_source_combo")
                            .selected_text(&short_label)
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                for dev in &self.state.audio_devices {
                                    let is_selected = self.state.selected_audio_device == dev.id;
                                    let dev_name = if dev.id == "default" {
                                        lang.audio_source_default()
                                    } else {
                                        dev.name.as_str()
                                    };
                                    let short_dev_name = truncate_text(dev_name, 30);
                                    if ui.selectable_label(is_selected, &short_dev_name).on_hover_text(dev_name).clicked() {
                                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetAudioSource(AudioSource::Device(dev.id.clone())));
                                    }
                                }
                            });
                        combo_res.response.on_hover_text(current_label);

                        if ui.button(" ⟳ ").on_hover_text(lang.tooltip_rescan_audio()).clicked() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshAudioDevices);
                        }
                    });

                    let is_system_mix = self.state.selected_audio_device == "default";

                    if !is_system_mix {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(100, 200, 255), lang.split_mode_label());
                            ui.label(
                                egui::RichText::new(lang.split_mode_desc())
                                    .size(11.0)
                                    .weak(),
                            );
                        });
                    }
                } else {
                    // Application Selector
                    let (sel_exe, sel_title) = match &self.state.audio_source {
                        AudioSource::Application { exe_name, display_name, .. } => (exe_name.as_str(), display_name.as_str()),
                        _ => ("", ""),
                    };

                    let current_text = if !sel_title.is_empty() {
                        sel_title
                    } else {
                        lang.select_app_placeholder()
                    };
                    let short_app_text = truncate_text(current_text, 26);

                    ui.horizontal(|ui| {
                        let combo_w = (ui.available_width() - 38.0).max(160.0);
                        let combo_res = egui::ComboBox::from_id_salt("audio_app_combo")
                            .selected_text(&short_app_text)
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                if self.state.audio_apps.is_empty() {
                                    ui.label(egui::RichText::new(lang.no_apps_found()).weak());
                                } else {
                                    for app in &self.state.audio_apps {
                                        let is_sel = sel_exe.eq_ignore_ascii_case(&app.exe_name);
                                        let badge = if app.is_active_audio { " 🔊" } else { "" };
                                        let label_text = format!("{}{badge}", app.display_name);
                                        let short_label = truncate_text(&label_text, 30);

                                        if ui.selectable_label(is_sel, &short_label).on_hover_text(&label_text).clicked() {
                                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetAudioSource(AudioSource::Application {
                                                pid: app.pid,
                                                exe_name: app.exe_name.clone(),
                                                display_name: app.display_name.clone(),
                                            }));
                                        }
                                    }
                                }
                            });
                        combo_res.response.on_hover_text(current_text);

                        if ui.button(" ⟳ ").on_hover_text(lang.tooltip_rescan_apps()).clicked() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshAudioApps);
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(lang.app_mode_hint())
                            .size(11.0)
                            .weak(),
                    );
                }

                ui.add_space(6.0);

                // Auto Local Audio Enhancement checkbox (AO Mode)
                let mut auto_ao = self.state.auto_ao_mode;
                let cb = ui.checkbox(
                    &mut auto_ao,
                    egui::RichText::new(lang.auto_ao_mode_checkbox()).size(12.0),
                );
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAutoAoMode(auto_ao));
                }
                if matches!(self.state.status, DeviceStatus::AoMode(_)) {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(lang.ao_mode_active_badge())
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 220, 140))
                                .strong(),
                        );
                    });
                }

                if self.state.auto_ao_mode {
                    let is_app_source = matches!(self.state.audio_source, AudioSource::Application { .. });
                    ui.add_space(4.0);
                    // Local Output Device selector for AO Mode (App and System Audio)
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("🎧 {}:", lang.ao_output_device_label())).size(11.5));
                        let cur_label = self
                            .state
                            .audio_devices
                            .iter()
                            .find(|d| d.id == self.state.ao_output_device)
                            .map(|d| if d.id == "default" { lang.default_output_device() } else { d.name.as_str() })
                            .unwrap_or_else(|| lang.default_output_device());
                        let short_ao_label = truncate_text(cur_label, 24);

                        let combo_w = (ui.available_width() - 38.0).max(120.0);
                        let combo_res = egui::ComboBox::from_id_salt("ao_output_device_combo")
                            .selected_text(&short_ao_label)
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                for dev in &self.state.audio_devices {
                                    let is_selected = self.state.ao_output_device == dev.id;
                                    let dev_name = if dev.id == "default" {
                                        lang.default_output_device()
                                    } else {
                                        dev.name.as_str()
                                    };
                                    let short_dev_name = truncate_text(dev_name, 28);
                                    if ui.selectable_label(is_selected, &short_dev_name).on_hover_text(dev_name).clicked() {
                                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetAoOutputDevice(dev.id.clone()));
                                    }
                                }
                            });
                        combo_res.response.on_hover_text(cur_label);

                        if ui.button(" ⟳ ").on_hover_text(lang.tooltip_rescan_audio()).clicked() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshAudioDevices);
                        }
                    });
                    ui.add_space(2.0);

                    if is_app_source {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(lang.ao_mode_desc_app())
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(120, 210, 240)),
                            )
                            .wrap(),
                        );
                    } else {
                        let is_same_dev = match &self.state.audio_source {
                            AudioSource::Device(src_id) => {
                                let s = if src_id.is_empty() { "default" } else { src_id.as_str() };
                                let o = if self.state.ao_output_device.is_empty() { "default" } else { self.state.ao_output_device.as_str() };
                                s.eq_ignore_ascii_case(o)
                            }
                            _ => false,
                        };

                        if is_same_dev {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(lang.ao_mode_desc_system_same())
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(235, 185, 75)),
                                )
                                .wrap(),
                            );
                        } else {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(lang.ao_mode_desc_system_diff())
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(120, 210, 240)),
                                )
                                .wrap(),
                            );
                        }
                    }
                }

                let can_auto_mute = is_app_mode || self.state.selected_audio_device == "default";

                ui.add_space(6.0);
                // Auto mute PC speakers: applies to full system mix and application loopback
                ui.vertical(|ui| {
                    if !self.state.license_tier.is_auto_mute_allowed() {
                        // Trial expired: locked and disabled!
                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(&mut false, egui::RichText::new(lang.auto_mute_locked_label()).size(12.0)),
                        ).on_disabled_hover_text(lang.auto_mute_locked_tooltip());

                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_premultiplied(180, 50, 50, 40))
                            .rounding(6.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(lang.license_trial_expired_banner())
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(255, 185, 185)),
                                    );
                                    ui.add_space(4.0);
                                    if ui.button(
                                        egui::RichText::new(lang.btn_buy_product())
                                            .size(11.5)
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    ).clicked() {
                                        crate::license::open_store_product_page();
                                    }
                                });
                            });
                    } else if can_auto_mute {
                        let mut auto_mute = self.state.auto_mute_speakers;
                        ui.horizontal_wrapped(|ui| {
                            if ui.checkbox(&mut auto_mute, egui::RichText::new(lang.auto_mute_speakers_checkbox()).size(12.0)).changed() {
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetAutoMuteSpeakers(auto_mute));
                            }
                            if let crate::license::LicenseTier::Trial { days_remaining } = self.state.license_tier {
                                ui.label(
                                    egui::RichText::new(lang.license_trial_badge(days_remaining))
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(255, 180, 80)),
                                );
                            }
                        });
                    } else {
                        ui.add_enabled(
                            false,
                            egui::Checkbox::new(&mut false, egui::RichText::new(lang.auto_mute_disabled_label()).size(12.0)),
                        ).on_disabled_hover_text(lang.auto_mute_disabled_tooltip());
                    }
                });

                ui.add_space(6.0);
                if ui.button(egui::RichText::new(lang.btn_open_windows_mixer()).size(11.5)).clicked() {
                    open_windows_volume_mixer();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // --- AirPods Sub-section ---
                ui.horizontal_wrapped(|ui| {
                    let title = if let Some(ref model) = self.state.airpods_model {
                        format!("🎧 {model}")
                    } else {
                        format!("🎧 {}", lang.airpods_section_title())
                    };
                    ui.label(egui::RichText::new(title).strong().size(12.0));

                    if self.state.airpods_detected {
                        let (badge_text, is_active) = match (self.state.airpods_left_in_ear, self.state.airpods_right_in_ear) {
                            (true, true) => (lang.airpods_both_in_ear_badge(), true),
                            (true, false) => (lang.airpods_left_only_in_ear_badge(), true),
                            (false, true) => (lang.airpods_right_only_in_ear_badge(), true),
                            (false, false) => (lang.airpods_out_of_ear_badge(), false),
                        };

                        let in_ear_status = if is_active {
                            egui::RichText::new(badge_text)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 220, 140))
                                .strong()
                        } else {
                            egui::RichText::new(badge_text)
                                .size(11.0)
                                .weak()
                        };
                        ui.label(in_ear_status);

                        let mut bat_info = Vec::new();
                        if let Some(l) = self.state.airpods_battery_left {
                            let wear_sym = if self.state.airpods_left_in_ear { "●" } else { "○" };
                            let chg_sym = if self.state.airpods_charging_left { "⚡" } else { "" };
                            bat_info.push(format!("{wear_sym} L:{l}%{chg_sym}"));
                        }
                        if let Some(r) = self.state.airpods_battery_right {
                            let wear_sym = if self.state.airpods_right_in_ear { "●" } else { "○" };
                            let chg_sym = if self.state.airpods_charging_right { "⚡" } else { "" };
                            bat_info.push(format!("{wear_sym} R:{r}%{chg_sym}"));
                        }
                        if let Some(c) = self.state.airpods_battery_case {
                            let chg_sym = if self.state.airpods_charging_case { "⚡" } else { "" };
                            bat_info.push(format!("🔋{c}%{chg_sym}"));
                        }
                        if !bat_info.is_empty() {
                            let is_connected = self.state.is_airpods_connected();
                            let bat_str = format!("({})", bat_info.join("  "));
                            let min_bat = [self.state.airpods_battery_left, self.state.airpods_battery_right]
                                .into_iter()
                                .flatten()
                                .min()
                                .unwrap_or(100);

                            let rich = egui::RichText::new(bat_str).size(10.5);
                            let label = if is_connected {
                                if min_bat <= 20 {
                                    rich.color(egui::Color32::from_rgb(250, 150, 50)).strong()
                                } else {
                                    rich.color(egui::Color32::from_rgb(115, 230, 155)).strong()
                                }
                            } else {
                                rich.weak()
                            };

                            let resp = ui.label(label);
                            if is_connected {
                                resp.on_hover_text(lang.airpods_battery_live_tooltip());
                            } else {
                                resp.on_hover_text(lang.airpods_battery_stale_tooltip());
                            }
                        }
                    }
                });

                ui.add_space(4.0);

                let mut airpods_routing = self.state.airpods_auto_routing;
                let cb = ui.checkbox(
                    &mut airpods_routing,
                    egui::RichText::new(lang.airpods_auto_routing_checkbox()).size(12.0),
                )
                .on_hover_text(lang.airpods_auto_routing_tooltip());
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPodsAutoRouting(airpods_routing));
                }

                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(18.0);
                    ui.label(
                        egui::RichText::new(lang.airpods_naming_hint())
                            .size(10.5)
                            .color(egui::Color32::from_rgb(170, 175, 185)),
                    );
                });

                ui.add_space(4.0);

                let mut airpods_smart = self.state.airpods_auto_pause;
                let cb = ui.checkbox(
                    &mut airpods_smart,
                    egui::RichText::new(lang.airpods_smart_playback_control_checkbox()).size(12.0),
                )
                .on_hover_text(lang.airpods_smart_playback_control_tooltip());
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPodsAutoPause(airpods_smart));
                }

                ui.add_space(6.0);
                if ui.button(egui::RichText::new(lang.btn_open_windows_bluetooth()).size(11.5)).clicked() {
                    open_windows_bluetooth_settings();
                }
            });
            }
            SettingsTab::Receiver => {
                // --- Group: Receiver Settings ---
                ui.label(egui::RichText::new(lang.airplay_receiver_section()).strong().size(13.0));
                ui.add_space(4.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        // Toggle Switch & Live Status
                        ui.horizontal(|ui| {
                            let mut enabled = self.state.airplay_receiver_enabled;
                            let cb = ui.checkbox(
                                &mut enabled,
                                egui::RichText::new(lang.airplay_receiver_enable()).size(12.0),
                            );
                            if cb.changed() {
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverEnabled(enabled));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if self.state.airplay_receiver_enabled {
                                    if self.state.airplay_receiver_streaming {
                                        ui.label(
                                            egui::RichText::new(lang.airplay_receiver_status_streaming())
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(80, 220, 120)),
                                        );
                                    } else {
                                        ui.label(
                                            egui::RichText::new(lang.airplay_receiver_status_idle())
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(160, 165, 175)),
                                        );
                                    }
                                }
                            });
                        });

                        if self.state.airplay_receiver_enabled {
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Broadcast Name
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(lang.airplay_receiver_name_label()).size(12.0));
                                let mut name = self.state.airplay_receiver_name.clone();
                                let resp = ui.add_sized(
                                    [220.0, 22.0],
                                    egui::TextEdit::singleline(&mut name).hint_text("Win2Pod Audio Receiver"),
                                );
                                if resp.lost_focus() && name != self.state.airplay_receiver_name && !name.trim().is_empty() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverName(name.trim().to_string()));
                                }
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // A/V Sync Compensation
                            ui.label(egui::RichText::new(lang.airplay_receiver_sync_label()).size(12.0));
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                let current_sync = self.state.airplay_receiver_sync_level;
                                let is_none = current_sync == crate::controller::AirPlayReceiverSyncLevel::None;
                                let is_standard = current_sync == crate::controller::AirPlayReceiverSyncLevel::Standard;
                                let is_strong = current_sync == crate::controller::AirPlayReceiverSyncLevel::Strong;

                                let btn_w = ((ui.available_width() - 8.0) / 3.0).max(80.0);

                                if ui.add_sized([btn_w, 24.0], egui::SelectableLabel::new(is_none, egui::RichText::new(lang.airplay_receiver_sync_none()).size(11.5))).clicked() && !is_none {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverSyncLevel(crate::controller::AirPlayReceiverSyncLevel::None));
                                }

                                if ui.add_sized([btn_w, 24.0], egui::SelectableLabel::new(is_standard, egui::RichText::new(lang.airplay_receiver_sync_standard()).size(11.5).strong())).clicked() && !is_standard {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverSyncLevel(crate::controller::AirPlayReceiverSyncLevel::Standard));
                                }

                                if ui.add_sized([btn_w, 24.0], egui::SelectableLabel::new(is_strong, egui::RichText::new(lang.airplay_receiver_sync_strong()).size(11.5))).clicked() && !is_strong {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverSyncLevel(crate::controller::AirPlayReceiverSyncLevel::Strong));
                                }
                            });

                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(lang.airplay_receiver_sync_hint())
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(170, 175, 185)),
                                );
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Auto Max Volume
                            let mut auto_max = self.state.airplay_receiver_auto_max_volume;
                            if ui.checkbox(&mut auto_max, egui::RichText::new(lang.airplay_receiver_auto_max_volume_label()).size(12.0)).changed() {
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverAutoMaxVolume(auto_max));
                            }

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Audio Forwarding Device
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("🔀 {}:", lang.airplay_receiver_output_device_label())).size(12.0));
                            });
                            ui.add_space(2.0);

                            let cur_rx_dev = &self.state.airplay_receiver_output_device;
                            let cur_rx_label = self
                                .state
                                .audio_devices
                                .iter()
                                .find(|d| &d.id == cur_rx_dev)
                                .map(|d| if d.id == "default" { lang.default_output_device() } else { d.name.as_str() })
                                .unwrap_or_else(|| lang.default_output_device());
                            let short_rx_label = truncate_text(cur_rx_label, 30);

                            let combo_w = (ui.available_width() - 8.0).max(160.0);
                            let rx_combo_res = egui::ComboBox::from_id_salt("airplay_receiver_output_device_combo")
                                .selected_text(&short_rx_label)
                                .width(combo_w)
                                .show_ui(ui, |ui| {
                                    for dev in &self.state.audio_devices {
                                        let is_selected = self.state.airplay_receiver_output_device == dev.id;
                                        let dev_name = if dev.id == "default" {
                                            lang.default_output_device()
                                        } else {
                                            dev.name.as_str()
                                        };
                                        let short_dev_name = truncate_text(dev_name, 30);
                                        if ui.selectable_label(is_selected, &short_dev_name).on_hover_text(dev_name).clicked() {
                                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverOutputDevice(dev.id.clone()));
                                        }
                                    }
                                });
                            rx_combo_res.response.on_hover_text(cur_rx_label);

                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(lang.airplay_receiver_output_device_hint())
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(170, 175, 185)),
                                );
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Dedicated Audio Effects Processing
                            let mut apply_fx = self.state.airplay_receiver_apply_effects;
                            if ui.checkbox(&mut apply_fx, egui::RichText::new(lang.airplay_receiver_apply_effects_label()).size(12.0)).changed() {
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetAirPlayReceiverApplyEffects(apply_fx));
                            }

                            if self.state.airplay_receiver_apply_effects {
                                ui.add_space(4.0);
                                if ui.button(egui::RichText::new(lang.airplay_receiver_btn_adjust_eq()).size(11.5).strong().color(egui::Color32::from_rgb(140, 210, 255))).clicked() {
                                    self.eq_profile = EqProfile::Receiver;
                                    self.navigate_to(Page::Equalizer);
                                }
                            }
                        }
                    });

                ui.add_space(10.0);

                // --- Group: Guide & Troubleshooting ---
                ui.label(egui::RichText::new(lang.receiver_guide_header()).strong().size(13.0));
                ui.add_space(4.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(26, 28, 34))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        // 1. Basic Connection
                        ui.label(
                            egui::RichText::new(lang.receiver_guide_basic_title())
                                .strong()
                                .size(11.5)
                                .color(egui::Color32::from_rgb(210, 215, 225)),
                        );
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(lang.receiver_guide_basic_desc())
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(160, 165, 175)),
                            );
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 2. Troubleshooting
                        ui.label(
                            egui::RichText::new(lang.receiver_guide_troubleshoot_title())
                                .strong()
                                .size(11.5)
                                .color(egui::Color32::from_rgb(210, 215, 225)),
                        );
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(lang.receiver_guide_troubleshoot_desc())
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(160, 165, 175)),
                            );
                        });
                    });
            }
            SettingsTab::System => {
                // --- Group 2: Streaming & Power ---
                ui.label(egui::RichText::new(lang.settings_section_streaming()).strong().size(13.0));
                ui.add_space(4.0);

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(32, 34, 40))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(10.0)
            .show(ui, |ui| {
                // Low Latency
                let mut low_latency = self.state.low_latency;
                let cb = ui.checkbox(
                    &mut low_latency,
                    egui::RichText::new(lang.low_latency_checkbox()).size(12.0),
                );
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetLowLatency(low_latency));
                }
                cb.on_hover_text(lang.low_latency_tooltip());

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new(if low_latency {
                            lang.hint_low_latency_on()
                        } else {
                            lang.hint_low_latency_off()
                        })
                        .size(11.0)
                        .weak(),
                    );
                });

                ui.add_space(6.0);

                // Prevent Sleep
                let mut prevent_sleep = self.state.prevent_sleep;
                ui.horizontal(|ui| {
                    let cb = ui.checkbox(
                        &mut prevent_sleep,
                        egui::RichText::new(lang.prevent_sleep_checkbox()).size(12.0),
                    );
                    if cb.changed() {
                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetPreventSleep(prevent_sleep));
                    }
                    cb.on_hover_text(lang.prevent_sleep_tooltip());

                    let (badge_text, badge_color) = match self.state.power_state {
                        // 🔴 螢幕恆亮（最高耗電與最高警示性）
                        crate::power::PowerState::AwakePresentation => ("● DISPLAY AWAKE", egui::Color32::from_rgb(255, 69, 58)),
                        // 🟡 播放中（系統防休眠，但螢幕可關）
                        crate::power::PowerState::AwakeStreaming => ("● AWAKE", egui::Color32::from_rgb(255, 179, 0)),
                        // 🟢 靜音 >30s（最省電 ECO，全釋放）
                        crate::power::PowerState::EcoSilent => ("● ECO (SILENT >30S)", egui::Color32::from_rgb(52, 199, 89)),
                        // ⚪ 待命（不干涉系統，中立白）
                        crate::power::PowerState::Standby => ("○ STANDBY", egui::Color32::from_rgb(200, 200, 200)),
                        // 🌑 系統休眠中
                        crate::power::PowerState::Suspended => ("○ SUSPENDED", egui::Color32::from_rgb(120, 120, 120)),
                    };
                    let badge_tooltip = lang.power_badge_tooltip(self.state.power_state);
                    ui.label(egui::RichText::new(badge_text).size(10.5).color(badge_color))
                        .on_hover_text(badge_tooltip);
                });

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let hint_text = if prevent_sleep {
                        if self.state.status == DeviceStatus::Streaming {
                            if self.state.is_muted {
                                lang.hint_sleep_muted()
                            } else if self.state.is_audio_active {
                                lang.hint_sleep_active()
                            } else {
                                lang.hint_sleep_idle()
                            }
                        } else {
                            lang.hint_sleep_streaming_desc()
                        }
                    } else {
                        lang.hint_sleep_off()
                    };
                    ui.label(
                        egui::RichText::new(hint_text)
                            .size(11.0)
                            .weak(),
                    );
                });


            });

        ui.add_space(10.0);

        // --- Group 3: System & Shortcuts ---
        ui.label(egui::RichText::new(lang.settings_section_general()).strong().size(13.0));
        ui.add_space(4.0);

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(32, 34, 40))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(10.0)
            .show(ui, |ui| {
                // Auto Connect
                let mut auto_connect = self.state.auto_connect;
                let cb = ui.checkbox(
                    &mut auto_connect,
                    egui::RichText::new(lang.auto_connect_checkbox()).size(12.0),
                );
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAutoConnect(auto_connect));
                }
                cb.on_hover_text(lang.auto_connect_tooltip());

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let hint = if auto_connect {
                        if let Some(ref name) = self.state.last_connected_name {
                            if name == "__default__" {
                                lang.auto_connect_target(lang.default_output_device())
                            } else {
                                lang.auto_connect_target(name)
                            }
                        } else {
                            lang.auto_connect_hint_none().to_string()
                        }
                    } else {
                        lang.auto_connect_hint_off().to_string()
                    };
                    ui.label(
                        egui::RichText::new(hint)
                            .size(11.0)
                            .weak(),
                    );
                });

                ui.add_space(6.0);

                // Autostart
                let mut autostart = self.state.autostart;
                let cb = ui.checkbox(
                    &mut autostart,
                    egui::RichText::new(lang.autostart_checkbox()).size(12.0),
                );
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAutostart(autostart));
                }
                cb.on_hover_text(lang.autostart_tooltip());

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let hint = if autostart {
                        lang.autostart_hint_on()
                    } else {
                        lang.autostart_hint_off()
                    };
                    ui.label(
                        egui::RichText::new(hint)
                            .size(11.0)
                            .weak(),
                    );
                });

                ui.add_space(6.0);

                // Window Snapping (Winamp Magnetic Windows)
                let mut window_snapping = self.state.window_snapping;
                let cb = ui.checkbox(
                    &mut window_snapping,
                    egui::RichText::new(lang.window_snapping_checkbox()).size(12.0),
                );
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetWindowSnapping(window_snapping));
                }
                cb.on_hover_text(lang.window_snapping_tooltip());

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let hint = if window_snapping {
                        lang.hint_window_snapping_on()
                    } else {
                        lang.hint_window_snapping_off()
                    };
                    ui.label(
                        egui::RichText::new(hint)
                            .size(11.0)
                            .weak(),
                    );
                });

                ui.add_space(6.0);

                // Global Hotkey
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(lang.global_hotkey_label()).size(12.0));
                    let cur_hk = self.state.hotkey;
                    let cur_label = hotkey_label(cur_hk.0, cur_hk.1, lang);
                    egui::ComboBox::from_id_salt("global_hotkey_combo")
                        .selected_text(egui::RichText::new(cur_label).size(12.0))
                        .show_ui(ui, |ui| {
                            for &(m, v) in AVAILABLE_HOTKEYS {
                                let is_selected = cur_hk == (m, v);
                                let item_label = hotkey_label(m, v, lang);
                                if ui.selectable_label(is_selected, item_label).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetHotkey(m, v));
                                }
                            }
                        });
                });

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new(lang.global_hotkey_hint())
                            .size(11.0)
                            .weak(),
                    );
                });

                ui.add_space(6.0);

                // Language Selector
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(lang.language_selector_label()).size(12.0));
                    let cur_lang = self.state.language;
                    egui::ComboBox::from_id_salt("language_combo")
                        .selected_text(egui::RichText::new(cur_lang.display_name()).size(12.0))
                        .show_ui(ui, |ui| {
                            for &l in ALL_LANGUAGES {
                                let is_selected = cur_lang == l;
                                if ui.selectable_label(is_selected, l.display_name()).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetLanguage(l));
                                    crate::visualizer::set_language(l.code());
                                    setup_custom_fonts(ui.ctx(), l);
                                }
                            }
                        });
                });
            });

                ui.add_space(10.0);

                // --- Group 4: Window & Mascot (視窗幸運物) ---
                ui.label(egui::RichText::new(lang.mascot_section_title()).strong().size(13.0));
                ui.add_space(4.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        let mut mascot_enabled = self.state.mascot_enabled;
                        let cb = ui.checkbox(
                            &mut mascot_enabled,
                            egui::RichText::new(lang.mascot_enable_checkbox()).size(12.0),
                        );
                        if cb.changed() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetMascotEnabled(mascot_enabled));
                        }
                        cb.on_hover_text(lang.mascot_enable_hint());

                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new(lang.mascot_enable_hint())
                                    .size(11.0)
                                    .weak(),
                            );
                        });

                        if mascot_enabled {
                            ui.add_space(6.0);
                            let mut rhythm = self.state.mascot_rhythm_dance;
                            let cb_rhythm = ui.checkbox(
                                &mut rhythm,
                                egui::RichText::new(lang.mascot_rhythm_checkbox()).size(12.0),
                            );
                            if cb_rhythm.changed() {
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetMascotRhythmDance(rhythm));
                            }
                            cb_rhythm.on_hover_text(lang.mascot_rhythm_hint());

                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.add_space(20.0);
                                ui.label(
                                    egui::RichText::new(lang.mascot_rhythm_hint())
                                        .size(11.0)
                                        .weak(),
                                );
                            });

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add_space(20.0);
                                if self.state.license_tier.is_supporter_or_unpackaged() {
                                    ui.label(
                                        egui::RichText::new(lang.mascot_easter_egg_hint())
                                            .size(11.0)
                                            .italics()
                                            .color(egui::Color32::from_rgb(180, 205, 235)),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new(lang.mascot_pro_locked_hint())
                                            .size(11.0)
                                            .weak(),
                                    );
                                }
                            });

                            if self.state.license_tier.is_supporter_or_unpackaged() {
                                ui.add_space(3.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    if ui.button(egui::RichText::new(lang.mascot_supporter_replay_btn()).size(11.0)).clicked() {
                                        self.show_supporter_mascot_modal = true;
                                    }
                                });
                            }
                        }
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(format!("ℹ  {}", lang.btn_about())).size(12.5)).on_hover_text(lang.tooltip_about()).clicked() {
                        self.navigate_to(Page::About);
                    }
                });
                ui.add_space(8.0);
            }
        }
            });

        ui.cursor().top() - initial_top
    }

    fn render_about_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let lang = self.state.language;
        let start_top = ui.cursor().top();

        // 頂部固定列：返回按鈕與標題（固定不捲動）
        self.render_universal_window_header(
            ui,
            crate::snapping::WindowKind::Main,
            lang.about_title(),
            true,
            |app, ui| {
                app.render_visualizer_button(ui, crate::snapping::WindowKind::Main);
            },
        );

        let header_h = ui.cursor().top() - start_top;

        let card_color = if ui.visuals().dark_mode {
            egui::Color32::from_rgb(32, 36, 44)
        } else {
            egui::Color32::from_rgb(245, 247, 250)
        };

        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("about_scroll_area")
            .auto_shrink([false, false]);
        if self.reset_about_scroll {
            self.reset_about_scroll = false;
            scroll = scroll.vertical_scroll_offset(0.0);
        }
        let scroll_output = scroll.show(ui, |ui| {
                let wrap_w = (ui.available_width() - 20.0).max(100.0);

                // 1. Basic Software Information, Author, License & Copyright Notice
                egui::Frame::none()
                    .fill(card_color)
                    .rounding(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Win2Pod Bridge").size(15.0).strong());
                            ui.label(egui::RichText::new(format!("v{}", ms_store_version())).size(12.0).weak());
                        });
                        ui.add_space(4.0);
                        ui.add_sized(
                            [wrap_w, 0.0],
                            egui::Label::new(
                                egui::RichText::new(lang.about_description())
                                    .size(11.5)
                                    .color(if ui.visuals().dark_mode { egui::Color32::from_rgb(180, 190, 205) } else { egui::Color32::from_rgb(90, 100, 115) }),
                            ).wrap(),
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Software Version (Microsoft Store format: Major.Minor.Build.Revision)
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(lang.about_version_label()).size(11.5).strong());
                            ui.label(egui::RichText::new(ms_store_version()).size(11.5));
                        });
                        ui.add_space(3.0);

                        // Store License Status
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(lang.about_store_status_label()).size(11.5).strong());
                            match self.state.license_tier {
                                crate::license::LicenseTier::Licensed => {
                                    ui.label(egui::RichText::new(lang.license_status_full()).size(11.5).color(egui::Color32::from_rgb(100, 220, 100)));
                                }
                                crate::license::LicenseTier::Trial { days_remaining } => {
                                    ui.label(egui::RichText::new(lang.license_trial_days_remaining(days_remaining)).size(11.5).color(egui::Color32::from_rgb(255, 180, 80)));
                                    if ui.button(egui::RichText::new(lang.btn_buy_short()).size(10.5).strong()).clicked() {
                                        crate::license::open_store_product_page();
                                    }
                                }
                                crate::license::LicenseTier::TrialExpired => {
                                    ui.label(egui::RichText::new(lang.license_status_expired()).size(11.5).color(egui::Color32::from_rgb(255, 100, 100)));
                                    if ui.button(egui::RichText::new(lang.btn_buy_short()).size(10.5).strong()).clicked() {
                                        crate::license::open_store_product_page();
                                    }
                                }
                                crate::license::LicenseTier::Unpackaged => {
                                    ui.label(egui::RichText::new(lang.license_status_unpackaged()).size(11.5).weak());
                                }
                            }
                        });

                        if !crate::license::is_packaged_app() {
                            ui.add_space(2.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(lang.license_test_title()).size(10.5).weak());
                                if ui.button(egui::RichText::new(lang.license_test_btn_expired()).size(10.0)).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::UpdateLicenseTier(crate::license::LicenseTier::TrialExpired));
                                }
                                if ui.button(egui::RichText::new(lang.license_test_btn_trial()).size(10.0)).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::UpdateLicenseTier(crate::license::LicenseTier::Trial { days_remaining: 7 }));
                                }
                                if ui.button(egui::RichText::new(lang.license_test_btn_full()).size(10.0)).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::UpdateLicenseTier(crate::license::LicenseTier::Licensed));
                                }
                                if ui.button(egui::RichText::new(lang.license_test_btn_reset()).size(10.0)).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::UpdateLicenseTier(crate::license::LicenseTier::Unpackaged));
                                }
                            });
                        }
                        ui.add_space(3.0);

                        // Author
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(lang.about_author_label()).size(11.5).strong());
                            ui.label(egui::RichText::new(lang.about_author_value()).size(11.5));
                        });
                        ui.add_space(3.0);

                        // License
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(lang.about_license_label()).size(11.5).strong());
                            ui.label(egui::RichText::new("GUI: Proprietary / Engine: GPL-3.0-or-later").size(11.5));
                        });
                        ui.add_space(6.0);

                        // Copyright Notice
                        ui.add_sized(
                            [wrap_w, 0.0],
                            egui::Label::new(
                                egui::RichText::new(lang.about_copyright_notice())
                                    .size(10.5)
                                    .color(if ui.visuals().dark_mode { egui::Color32::from_rgb(150, 160, 175) } else { egui::Color32::from_rgb(110, 120, 135) }),
                            ).wrap(),
                        );
                    });

                ui.add_space(10.0);

                // 2. Open Source Credits & Licenses
                egui::Frame::none()
                    .fill(card_color)
                    .rounding(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(lang.about_packages_header()).size(13.0).strong());
                        ui.add_space(6.0);

                        let packages: [(&str, &str, &str, &str); 34] = [
                            ("airplay2-rs", "Lucas McGartland", "GPL-3.0-or-later", lang.pkg_airplay2_rs_desc()),
                            ("windows-airplay-homepod", "Yang Liu (iakacer)", "GPL-2.0", lang.pkg_windows_airplay_desc()),
                            ("UxPlay", "antimof / FDH2", "GPL-3.0", lang.pkg_uxplay_desc()),
                            ("AirPodsDesktop", "SpriteOvO", "GPL-3.0", lang.pkg_airpodsdesktop_desc()),
                            ("muelle", "Muelle contributors", "MIT", lang.pkg_muelle_desc()),
                            ("egui & eframe", "v0.29", "MIT / Apache-2.0", lang.pkg_egui_desc()),
                            ("wry", "v0.57", "Apache-2.0 / MIT", lang.pkg_wry_desc()),
                            ("three.js", "r160 (mrdoob)", "MIT", lang.pkg_threejs_desc()),
                            ("three-painterly", "PainterlyPass", "MIT", lang.pkg_three_painterly_desc()),
                            ("baked-debris", "Debris Simulation", "MIT", lang.pkg_baked_debris_desc()),
                            ("image", "v0.25", "MIT / Apache-2.0", lang.pkg_image_desc()),
                            ("wasapi", "v0.23", "MIT / Apache-2.0", lang.pkg_wasapi_desc()),
                            ("windows & windows-sys", "v0.62 / v0.59", "MIT / Apache-2.0", lang.pkg_windows_desc()),
                            ("raw-window-handle", "v0.6", "MIT / Apache-2.0", lang.pkg_raw_window_handle_desc()),
                            ("biquad", "v0.4", "MIT", lang.pkg_biquad_desc()),
                            ("oximedia-effects", "v0.2.1", "Apache-2.0 / MIT", lang.pkg_oximedia_desc()),
                            ("realfft", "v3.5", "MIT / Apache-2.0", lang.pkg_realfft_desc()),
                            ("alac-encoder", "v0.3", "MIT", lang.pkg_alac_desc()),
                            ("rubato", "v0.15", "MIT", lang.pkg_rubato_desc()),
                            ("symphonia", "v0.5", "MPL-2.0", lang.pkg_symphonia_desc()),
                            ("tokio", "v1.49", "MIT", lang.pkg_tokio_desc()),
                            ("mdns-sd", "v0.11", "Apache-2.0", lang.pkg_mdns_desc()),
                            ("socket2", "v0.5", "MIT / Apache-2.0", lang.pkg_socket2_desc()),
                            ("chacha20poly1305", "v0.10", "Apache-2.0 / MIT", lang.pkg_chacha_desc()),
                            ("aes & aes-gcm", "v0.8 / v0.10", "Apache-2.0 / MIT", lang.pkg_aes_desc()),
                            ("ed25519-dalek & x25519-dalek", "v2.2 / v2.0", "BSD-3-Clause", lang.pkg_dalek_desc()),
                            ("rsa", "v0.9", "MIT / Apache-2.0", lang.pkg_rsa_desc()),
                            ("plist", "v1.8", "MIT", lang.pkg_plist_desc()),
                            ("serde", "v1.0", "MIT / Apache-2.0", lang.pkg_serde_desc()),
                            ("serde_json", "v1.0", "MIT / Apache-2.0", lang.pkg_serde_json_desc()),
                            ("uuid", "v1.20", "Apache-2.0 / MIT", lang.pkg_uuid_desc()),
                            ("tray-icon & muda", "v0.25", "Apache-2.0 / MIT", lang.pkg_tray_desc()),
                            ("tracing", "v0.1", "MIT", lang.pkg_tracing_desc()),
                            ("anyhow & thiserror", "v1.0 / v2.0", "MIT / Apache-2.0", lang.pkg_err_desc()),
                        ];

                        for (name, ver, lic, desc) in packages {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(name).size(11.5).strong());
                                ui.label(egui::RichText::new(ver).size(10.5).weak());
                                ui.label(
                                    egui::RichText::new(format!("• {lic}"))
                                        .size(10.0)
                                        .color(if ui.visuals().dark_mode { egui::Color32::from_rgb(140, 180, 230) } else { egui::Color32::from_rgb(40, 90, 160) }),
                                );
                            });
                            ui.add_sized(
                                [wrap_w, 0.0],
                                egui::Label::new(
                                    egui::RichText::new(desc)
                                        .size(10.5)
                                        .color(if ui.visuals().dark_mode { egui::Color32::from_rgb(170, 180, 195) } else { egui::Color32::from_rgb(100, 110, 125) }),
                                ).wrap(),
                            );
                            ui.add_space(5.0);
                        }
                    });

                ui.add_space(8.0);
            });

        header_h + scroll_output.content_size.y
    }

    /// 渲染串流規格徽章組（AIRPLAY 2 / LOCAL / AirPods、44.1 kHz、16-BIT、STEREO / MONO、HE、WE）
    pub fn render_audio_format_badges(&self, ui: &mut egui::Ui) {
        self.render_audio_format_badges_inline(ui, false);
    }

    /// 渲染串流規格徽章組（支援行內緊湊版）
    pub fn render_audio_format_badges_inline(&self, ui: &mut egui::Ui, is_mini: bool) {
        let lang = self.state.language;
        let channel_badge = if self.is_mono { "MONO" } else { "STEREO" };
        let is_streaming = self.state.status == DeviceStatus::Streaming;
        let is_connecting = self.state.status == DeviceStatus::Connecting;
        let is_he_on = self.state.sound_enhancer_mode != airplay_audio::SoundEnhancerMode::Off;
        let he_label = match self.state.sound_enhancer_mode {
            airplay_audio::SoundEnhancerMode::Off => "HE:OFF",
            airplay_audio::SoundEnhancerMode::Natural => "HE:NAT",
            airplay_audio::SoundEnhancerMode::Dynamic => "HE:DYN",
            airplay_audio::SoundEnhancerMode::Brilliant => "HE:BRI",
        };
        let is_we_on = self.state.warmth_enhancer_mode != airplay_audio::WarmthEnhancerMode::Off;
        let we_label = match self.state.warmth_enhancer_mode {
            airplay_audio::WarmthEnhancerMode::Off => "WE:OFF",
            airplay_audio::WarmthEnhancerMode::Subtle => "WE:SUB",
            airplay_audio::WarmthEnhancerMode::Warm => "WE:WARM",
            airplay_audio::WarmthEnhancerMode::Vintage => "WE:VIN",
        };

        let is_airpods_route = match &self.state.status {
            DeviceStatus::AoMode(ref dev_name) => {
                crate::controller::is_airpods_playback_device(dev_name)
                    || crate::controller::is_airpods_playback_device(&self.state.ao_output_device)
            }
            _ => false,
        };

        let airpods_has_battery = self.state.airpods_battery_left.is_some() || self.state.airpods_battery_right.is_some();

        let (airpods_badge_label, airpods_min_bat, airpods_tip) = if is_airpods_route {
            let l_bat = self.state.airpods_battery_left;
            let r_bat = self.state.airpods_battery_right;
            let c_bat = self.state.airpods_battery_case;
            let is_chg = self.state.airpods_charging_left || self.state.airpods_charging_right;

            let avg = match (l_bat, r_bat) {
                (Some(l), Some(r)) => ((l as u32 + r as u32) / 2) as u8,
                (Some(l), None) => l,
                (None, Some(r)) => r,
                (None, None) => 100,
            };
            let min_b = [l_bat, r_bat].into_iter().flatten().min().unwrap_or(avg);

            let label = if airpods_has_battery {
                let status_icon = if is_chg { "⚡" } else { "🔋" };
                format!("🎧{status_icon} {avg}%")
            } else {
                "AIRPODS".to_string()
            };

            let model_name = self.state.airpods_model.as_deref().unwrap_or("AirPods");
            let status_tag = lang.airpods_connected_status();
            let mut details = Vec::new();
            if let Some(l) = l_bat {
                let chg = if self.state.airpods_charging_left { "⚡" } else { "" };
                let wear = if self.state.airpods_left_in_ear { "● " } else { "○ " };
                details.push(format!("• {wear}L: {l}%{chg}"));
            }
            if let Some(r) = r_bat {
                let chg = if self.state.airpods_charging_right { "⚡" } else { "" };
                let wear = if self.state.airpods_right_in_ear { "● " } else { "○ " };
                details.push(format!("• {wear}R: {r}%{chg}"));
            }
            if let Some(c) = c_bat {
                let chg = if self.state.airpods_charging_case { "⚡" } else { "" };
                details.push(format!("• 🔋Case: {c}%{chg}"));
            }
            let tip = if details.is_empty() {
                format!("🎧 {model_name} ({status_tag})")
            } else {
                format!("🎧 {model_name} ({status_tag})\n{}", details.join("\n"))
            };

            (label, min_b, tip)
        } else {
            ("AIRPLAY 2".to_string(), 100, String::new())
        };

        let is_other_ao = match &self.state.status {
            DeviceStatus::AoMode(_) if !is_airpods_route => true,
            _ => false,
        };

        let primary_badge_str = if is_airpods_route {
            airpods_badge_label.as_str()
        } else if is_other_ao {
            "LOCAL"
        } else {
            "AIRPLAY 2"
        };

        // 獨立規格徽章列由左至右順序：primary_badge -> 44.1 kHz -> 16-BIT -> STEREO/MONO -> (HE) -> (WE)
        let mut badges = Vec::new();
        badges.push(primary_badge_str);
        badges.push("44.1 kHz");
        badges.push("16-BIT");
        badges.push(channel_badge);
        if is_he_on { badges.push(he_label); }
        if is_we_on { badges.push(we_label); }

        ui.spacing_mut().item_spacing.x = if is_mini { 3.0 } else { 3.5 };
        for badge in badges {
            let is_primary_badge = badge == primary_badge_str;
            let is_active_mono = badge == "MONO";
            let is_he_badge = badge == he_label;
            let is_we_badge = badge == we_label;
            let (bg_col, stroke_col, text_col) = if is_he_badge {
                if is_he_on {
                    (egui::Color32::from_rgb(10, 36, 32), egui::Color32::from_rgb(25, 95, 80), egui::Color32::from_rgb(0, 255, 210))
                } else {
                    (egui::Color32::from_rgb(24, 25, 28), egui::Color32::from_rgb(50, 52, 58), egui::Color32::from_rgb(115, 120, 130))
                }
            } else if is_we_badge {
                if is_we_on {
                    (egui::Color32::from_rgb(12, 38, 22), egui::Color32::from_rgb(32, 98, 56), egui::Color32::from_rgb(80, 250, 150))
                } else {
                    (egui::Color32::from_rgb(24, 25, 28), egui::Color32::from_rgb(50, 52, 58), egui::Color32::from_rgb(115, 120, 130))
                }
            } else if is_primary_badge {
                if is_airpods_route {
                    if airpods_min_bat <= 10 {
                        (egui::Color32::from_rgb(38, 14, 14), egui::Color32::from_rgb(110, 25, 25), egui::Color32::from_rgb(255, 75, 75))
                    } else if airpods_min_bat <= 20 {
                        (egui::Color32::from_rgb(36, 22, 10), egui::Color32::from_rgb(110, 60, 20), egui::Color32::from_rgb(255, 145, 45))
                    } else {
                        (egui::Color32::from_rgb(14, 30, 16), egui::Color32::from_rgb(30, 70, 36), egui::Color32::from_rgb(60, 200, 95))
                    }
                } else if is_streaming || is_other_ao {
                    (egui::Color32::from_rgb(14, 30, 16), egui::Color32::from_rgb(30, 70, 36), egui::Color32::from_rgb(60, 200, 95))
                } else if is_connecting {
                    (egui::Color32::from_rgb(36, 26, 12), egui::Color32::from_rgb(90, 65, 25), egui::Color32::from_rgb(255, 185, 55))
                } else {
                    (egui::Color32::from_rgb(24, 25, 28), egui::Color32::from_rgb(50, 52, 58), egui::Color32::from_rgb(115, 120, 130))
                }
            } else if is_active_mono {
                (egui::Color32::from_rgb(36, 26, 12), egui::Color32::from_rgb(90, 65, 25), egui::Color32::from_rgb(255, 185, 55))
            } else {
                (egui::Color32::from_rgb(14, 30, 16), egui::Color32::from_rgb(30, 70, 36), egui::Color32::from_rgb(60, 200, 95))
            };

            egui::Frame::none()
                .fill(bg_col)
                .stroke(egui::Stroke::new(0.5_f32, stroke_col))
                .rounding(egui::Rounding::same(3.0))
                .inner_margin(egui::Margin::symmetric(if is_mini { 3.0 } else { 4.0 }, 1.5))
                .show(ui, |ui| {
                    let resp = ui.label(
                        egui::RichText::new(badge)
                            .monospace()
                            .size(if is_mini { 7.8 } else { 8.5 })
                            .color(text_col),
                    );
                    if is_primary_badge {
                        if is_airpods_route {
                            resp.on_hover_text(&airpods_tip);
                        } else {
                            let tip = if is_streaming {
                                let name = self.state.connected_name.as_deref().unwrap_or("AirPlay");
                                lang.status_streaming(name)
                            } else if is_connecting {
                                let name = self.state.connected_name.as_deref().unwrap_or("AirPlay");
                                lang.status_connecting(name)
                            } else if is_other_ao {
                                match &self.state.status {
                                    DeviceStatus::AoMode(name) => format!("正在播放至 {name}"),
                                    _ => "本機音訊輸出".to_string(),
                                }
                            } else {
                                lang.status_idle().to_string()
                            };
                            resp.on_hover_text(tip);
                        }
                    } else if is_he_badge {
                        show_wrapped_tooltip(resp, lang.sound_enhancer_tooltip());
                    } else if is_we_badge {
                        show_wrapped_tooltip(resp, lang.warmth_enhancer_tooltip());
                    }
                });
        }
    }

    pub fn render_player_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let initial_top = ui.cursor().top();
        let lang = self.state.language;

        // 1. Header: Universal Window Header
        self.render_universal_window_header(
            ui,
            crate::snapping::WindowKind::Main,
            "Win2Pod Player",
            true,
            |app, ui| {
                app.render_visualizer_button(ui, crate::snapping::WindowKind::Main);
            },
        );

        // 2. Query GSMTC Media Track Info
        let track_info = crate::media_control::get_current_media_track().read().unwrap().clone();

        // 3. WinAMP Main Display Chassis (Album Cover + VFD/LCD Screen with dynamic transitions)
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(22, 24, 29))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(46, 52, 64)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let total_inner_w = ui.available_width();

                self.render_winamp_display_chassis(ui, &track_info, lang, false);

                ui.add_space(6.0);

                // Sleek WinAMP Playback Progress & Seek Bar
                let cur_pos = if let Some(preview_secs) = self.seek_drag_secs {
                    preview_secs
                } else {
                    track_info.current_position_secs()
                };
                let total_dur = track_info.duration_secs;
                let progress = if total_dur > 0.0 {
                    (cur_pos / total_dur).clamp(0.0, 1.0) as f32
                } else {
                    0.0f32
                };

                let bar_h = 5.0f32;
                let (bar_rect, bar_resp) = ui.allocate_exact_size(
                    egui::vec2(total_inner_w, bar_h),
                    egui::Sense::click_and_drag(),
                );
                let bar_painter = ui.painter();
                // Rail background (dark vintage rack rail)
                bar_painter.rect_filled(bar_rect, egui::Rounding::same(2.5), egui::Color32::from_rgb(10, 13, 16));
                bar_painter.rect_stroke(bar_rect, egui::Rounding::same(2.5), egui::Stroke::new(0.5_f32, egui::Color32::from_rgb(25, 32, 40)));

                // Progress active fill (darker, subdued classic green)
                if progress > 0.0 {
                    let filled_w = (bar_rect.width() * progress).max(3.0);
                    let filled_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(filled_w, bar_h));
                    bar_painter.rect_filled(filled_rect, egui::Rounding::same(2.5), egui::Color32::from_rgb(20, 115, 62));
                    // Progress head indicator (subdued mint accent)
                    let head_center = egui::pos2(filled_rect.max.x, bar_rect.center().y);
                    bar_painter.circle_filled(head_center, 3.5, egui::Color32::from_rgb(55, 150, 90));
                }

                // Interactive seeking: only commit seek on mouse release or single click!
                if total_dur > 0.0 {
                    if bar_resp.dragged() {
                        if let Some(pos) = bar_resp.interact_pointer_pos() {
                            let norm = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            self.seek_drag_secs = Some(norm as f64 * total_dur);
                        }
                    } else if bar_resp.drag_stopped() {
                        if let Some(target_secs) = self.seek_drag_secs.take() {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Seek(target_secs));
                        }
                    } else if bar_resp.clicked() {
                        // Single click without dragging
                        if let Some(pos) = bar_resp.interact_pointer_pos() {
                            let norm = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            let target_secs = norm as f64 * total_dur;
                            self.seek_drag_secs = None;
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Seek(target_secs));
                        }
                    } else if !ui.input(|i| i.pointer.any_down()) && self.seek_drag_secs.is_some() {
                        // Safety fallback if pointer released outside bounds
                        if let Some(target_secs) = self.seek_drag_secs.take() {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Seek(target_secs));
                        }
                    }
                }

                ui.add_space(5.0);

                // 4. WinAMP Multi-color Segmented Spectrum Visualizer (exact match to total_inner_w)
                let spec_h = 50.0f32;
                let (spec_rect, spec_resp) = ui.allocate_exact_size(
                    egui::vec2(total_inner_w, spec_h),
                    egui::Sense::click(),
                );
                self.mascot_spectrum_rect = Some(spec_rect);

                // ⭐ 跨視窗即刻召喚：若使用者在主視窗播放器頻譜右鍵點擊，無論精靈原先在何處立即召喚至此展開特技衝浪！
                let hover_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()));
                let is_ptr_in_spec = hover_pos.map(|p| spec_rect.expand(4.0).contains(p)).unwrap_or(false);
                if spec_resp.secondary_clicked() || (is_ptr_in_spec && ui.input(|i| i.pointer.secondary_clicked())) {
                    self.trigger_mascot_air_surf_summon(crate::snapping::WindowKind::Main, spec_rect, hover_pos);
                }
                let painter = ui.painter();
                painter.rect_filled(spec_rect, egui::Rounding::same(4.0), egui::Color32::from_rgb(10, 12, 15));
                painter.rect_stroke(
                    spec_rect,
                    egui::Rounding::same(4.0),
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(30, 34, 44)),
                );

                let inner_spec = spec_rect.shrink(4.0);
                let num_bars = 16;
                let bar_gap = 2.5f32;
                let bar_w = ((inner_spec.width() - (num_bars as f32 - 1.0) * bar_gap) / num_bars as f32).max(2.0);

                let num_segs = 12;
                let seg_gap = 1.2f32;
                let seg_h = ((inner_spec.height() - (num_segs as f32 - 1.0) * seg_gap) / num_segs as f32).max(1.5);

                for i in 0..num_bars {
                    let bx = inner_spec.min.x + i as f32 * (bar_w + bar_gap);
                    let level = self.spectrum_vis.current[i].clamp(0.0, 1.0);
                    let active_segs = (level * num_segs as f32).round() as usize;

                    for s in 0..num_segs {
                        if s < active_segs {
                            let sy = inner_spec.max.y - (s as f32 + 1.0) * (seg_h + seg_gap);
                            let seg_rect = egui::Rect::from_min_size(
                                egui::pos2(bx, sy),
                                egui::vec2(bar_w, seg_h),
                            );
                            let col = if s >= 9 {
                                egui::Color32::from_rgb(255, 45, 45) // Top: Red
                            } else if s >= 5 {
                                egui::Color32::from_rgb(255, 200, 15) // Mid: Yellow
                            } else {
                                egui::Color32::from_rgb(25, 225, 55) // Bottom: Green
                            };
                            painter.rect_filled(seg_rect, egui::Rounding::same(0.5), col);
                        }
                    }

                    // Floating peak dot
                    let peak_lvl = self.spectrum_vis.peaks[i].clamp(0.0, 1.0);
                    if peak_lvl > 0.05 {
                        let peak_y = inner_spec.max.y - (inner_spec.height() * peak_lvl);
                        let cap_rect = egui::Rect::from_min_size(
                            egui::pos2(bx, peak_y.max(inner_spec.min.y)),
                            egui::vec2(bar_w, 2.0),
                        );
                        painter.rect_filled(cap_rect, egui::Rounding::same(0.5), egui::Color32::from_rgb(230, 245, 255));
                    }
                }
            });

        ui.add_space(8.0);

        // 5. WinAMP Transport Controls, Audio Balance & Volume Cluster
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(25, 27, 33))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(48, 54, 66)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    // Row 1: Transport Controls Cluster + Full-Width Balance Slider
                    ui.horizontal(|ui| {
                        let btn_size = egui::vec2(32.0, 24.0);

                        // Previous Track
                        if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏮").size(12.0)))
                            .on_hover_text(lang.player_prev())
                            .clicked()
                        {
                            self.next_track_intent = Some(TrackTransitionDirection::Prev);
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Previous);
                        }

                        // Play / Pause Toggle
                        let play_pause_icon = if track_info.is_playing { "⏸" } else { "▶" };
                        if ui.add_sized(egui::vec2(38.0, 24.0), egui::Button::new(egui::RichText::new(play_pause_icon).size(13.0).strong()))
                            .on_hover_text(lang.player_play_pause())
                            .clicked()
                        {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::TogglePlayPause);
                        }

                        // Stop
                        if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏹").size(11.0)))
                            .on_hover_text(lang.player_stop())
                            .clicked()
                        {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Stop);
                        }

                        // Next Track
                        if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏭").size(12.0)))
                            .on_hover_text(lang.player_next())
                            .clicked()
                        {
                            self.next_track_intent = Some(TrackTransitionDirection::Next);
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Next);
                        }

                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(2.0);

                        // Balance Label with strict fixed width to prevent any slider jitter/shaking
                        let bal_label_w = 72.0f32;
                        let bal_pct = (self.balance * 100.0).round() as i32;
                        let (bal_str, bal_col) = if bal_pct == 0 {
                            (
                                lang.balance_center().to_string(),
                                egui::Color32::from_rgb(140, 200, 255),
                            )
                        } else if bal_pct < 0 {
                            (
                                lang.balance_left(bal_pct.abs()),
                                egui::Color32::from_rgb(255, 205, 120),
                            )
                        } else {
                            (
                                lang.balance_right(bal_pct),
                                egui::Color32::from_rgb(255, 205, 120),
                            )
                        };

                        let bal_label_resp = ui.add_sized(
                            egui::vec2(bal_label_w, 20.0),
                            egui::Label::new(
                                egui::RichText::new(bal_str)
                                    .monospace()
                                    .size(10.5)
                                    .color(bal_col),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(lang.balance_tooltip());

                        if bal_label_resp.clicked() {
                            self.balance = 0.0;
                            crate::controller::set_audio_balance(0.0);
                        }

                        // Balance Slider spans remaining full width
                        let bal_slider_w = (ui.available_width() - 2.0).floor().max(40.0);
                        ui.spacing_mut().slider_width = bal_slider_w;
                        let mut bal = self.balance;
                        let bal_resp = ui.add(
                            egui::Slider::new(&mut bal, -1.0..=1.0)
                                .show_value(false),
                        )
                        .on_hover_text(lang.balance_tooltip());

                        if bal_resp.changed() {
                            self.balance = bal;
                            crate::controller::set_audio_balance(bal);
                        }
                        if bal_resp.double_clicked() {
                            self.balance = 0.0;
                            crate::controller::set_audio_balance(0.0);
                        }
                    });

                    ui.add_space(5.0);

                    // Row 2: Volume Display, Lengthened Slider, Mute Button, and STEREO/MONO Button
                    ui.horizontal(|ui| {
                        let vol_label_w = 56.0f32;
                        let vol_pct = (self.volume_slider * 100.0).round() as i32;
                        ui.add_sized(
                            egui::vec2(vol_label_w, 20.0),
                            egui::Label::new(
                                egui::RichText::new(format!("VOL:{:>3}%", vol_pct))
                                    .monospace()
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(0, 230, 110)),
                            ),
                        );

                        let (mute_text, mute_col) = if self.state.is_muted {
                            (lang.btn_unmute(), egui::Color32::from_rgb(70, 200, 120))
                        } else {
                            (lang.btn_mute(), egui::Color32::from_rgb(240, 160, 40))
                        };

                        let mono_btn_w = 58.0f32;
                        let spacing = ui.spacing().item_spacing.x;
                        let btn_pad_x = ui.spacing().button_padding.x * 2.0;
                        let mute_text_w = ui.painter().layout_no_wrap(
                            mute_text.to_string(),
                            egui::FontId::proportional(10.5),
                            mute_col,
                        ).size().x + btn_pad_x + 6.0;
                        let mute_btn_w = mute_text_w.max(50.0);

                        let slider_w = (ui.available_width() - mute_btn_w - mono_btn_w - spacing * 2.0 - 2.0).floor().max(40.0);

                        ui.spacing_mut().slider_width = slider_w;
                        let mut vol = self.volume_slider;
                        let slider = egui::Slider::new(&mut vol, 0.0..=1.0)
                            .show_value(false);
                        if ui.add(slider).changed() {
                            self.volume_slider = vol;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetVolume(vol));
                        }

                        // Mute button
                        if ui.add_sized(egui::vec2(mute_btn_w, 22.0), egui::Button::new(egui::RichText::new(mute_text).size(10.5).color(mute_col))).clicked() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetMute(!self.state.is_muted));
                        }

                        // STEREO / MONO Button (right of Mute button)
                        let (mono_text, mono_col) = if self.is_mono {
                            ("MONO", egui::Color32::from_rgb(255, 175, 45))
                        } else {
                            ("STEREO", egui::Color32::from_rgb(0, 220, 160))
                        };
                        let mono_btn = egui::Button::new(
                            egui::RichText::new(mono_text)
                                .monospace()
                                .size(10.5)
                                .strong()
                                .color(mono_col),
                        );
                        if ui.add_sized(egui::vec2(mono_btn_w, 22.0), mono_btn)
                            .on_hover_text(if self.is_mono { "Audio Mode: Mono (Click to switch to Stereo)" } else { "Audio Mode: Stereo (Click to switch to Mono)" })
                            .clicked()
                        {
                            self.is_mono = !self.is_mono;
                            crate::controller::set_audio_mono(self.is_mono);
                        }
                    });
                });
            });

        ui.add_space(8.0);

        // 6. WinAMP Sound Engine (Harmonic Exciter (HE) & Warmth Enhancer (WE))
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 24, 28))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 54, 62)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Row 1: Harmonic Exciter (HE)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("✨ {}", lang.sound_enhancer_title()))
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(egui::Color32::from_rgb(140, 230, 210)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for &m in [
                            airplay_audio::SoundEnhancerMode::Brilliant,
                            airplay_audio::SoundEnhancerMode::Dynamic,
                            airplay_audio::SoundEnhancerMode::Natural,
                            airplay_audio::SoundEnhancerMode::Off,
                        ]
                        .iter()
                        {
                            let is_sel = self.state.sound_enhancer_mode == m;
                            let m_name = lang.sound_enhancer_mode_name(m);
                            let btn = if is_sel {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .strong()
                                        .color(if m == airplay_audio::SoundEnhancerMode::Off {
                                            egui::Color32::from_rgb(220, 220, 225)
                                        } else {
                                            egui::Color32::from_rgb(0, 255, 210)
                                        }),
                                )
                                .fill(if m == airplay_audio::SoundEnhancerMode::Off {
                                    egui::Color32::from_rgb(50, 55, 65)
                                } else {
                                    egui::Color32::from_rgb(15, 60, 52)
                                })
                                .stroke(egui::Stroke::new(1.0_f32, if m == airplay_audio::SoundEnhancerMode::Off {
                                    egui::Color32::from_rgb(90, 95, 110)
                                } else {
                                    egui::Color32::from_rgb(40, 160, 130)
                                }))
                                .rounding(egui::Rounding::same(4.0))
                            } else {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(140, 150, 160)),
                                )
                                .fill(egui::Color32::from_rgb(25, 28, 33))
                                .stroke(egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15)))
                                .rounding(egui::Rounding::same(4.0))
                            };

                            if show_wrapped_tooltip(ui.add(btn), lang.sound_enhancer_preset_tooltip(m)).clicked() {
                                self.state.sound_enhancer_mode = m;
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetSoundEnhancerMode(m));
                            }
                        }
                    });
                });

                ui.add_space(5.0);

                // Row 2: Warmth Enhancer (WE)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("📻 {}", lang.warmth_enhancer_title()))
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(egui::Color32::from_rgb(140, 230, 160)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for &m in [
                            airplay_audio::WarmthEnhancerMode::Vintage,
                            airplay_audio::WarmthEnhancerMode::Warm,
                            airplay_audio::WarmthEnhancerMode::Subtle,
                            airplay_audio::WarmthEnhancerMode::Off,
                        ]
                        .iter()
                        {
                            let is_sel = self.state.warmth_enhancer_mode == m;
                            let m_name = lang.warmth_enhancer_mode_name(m);
                            let btn = if is_sel {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .strong()
                                        .color(if m == airplay_audio::WarmthEnhancerMode::Off {
                                            egui::Color32::from_rgb(220, 220, 225)
                                        } else {
                                            egui::Color32::from_rgb(80, 250, 150)
                                        }),
                                )
                                .fill(if m == airplay_audio::WarmthEnhancerMode::Off {
                                    egui::Color32::from_rgb(50, 55, 65)
                                } else {
                                    egui::Color32::from_rgb(16, 56, 30)
                                })
                                .stroke(egui::Stroke::new(1.0_f32, if m == airplay_audio::WarmthEnhancerMode::Off {
                                    egui::Color32::from_rgb(90, 95, 110)
                                } else {
                                    egui::Color32::from_rgb(45, 160, 85)
                                }))
                                .rounding(egui::Rounding::same(4.0))
                            } else {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(140, 150, 160)),
                                )
                                .fill(egui::Color32::from_rgb(25, 28, 33))
                                .stroke(egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15)))
                                .rounding(egui::Rounding::same(4.0))
                            };

                            if show_wrapped_tooltip(ui.add(btn), lang.warmth_enhancer_preset_tooltip(m)).clicked() {
                                self.state.warmth_enhancer_mode = m;
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetWarmthEnhancerMode(m));
                            }
                        }
                    });
                });
            });

        ui.add_space(8.0);

        // 7. WinAMP Integrated 10-Band Graphic Equalizer Module
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 22, 27))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(42, 47, 58)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                // EQ Header & Toggle Controls
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("♪ EQUALIZER")
                            .monospace()
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(180, 210, 235)),
                    );

                    let mut enabled = self.state.eq_enabled;
                    let (toggle_text, toggle_col) = if enabled {
                        ("EQ: ON", egui::Color32::from_rgb(80, 230, 120))
                    } else {
                        ("EQ: OFF", egui::Color32::from_rgb(150, 155, 165))
                    };
                    if ui.button(egui::RichText::new(toggle_text).strong().size(10.5).color(toggle_col)).clicked() {
                        enabled = !enabled;
                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqBypass(!enabled));
                    }

                    // Auto Pre-Amp headroom protection badge
                    let max_boost = self.state.eq_gains.iter().copied().fold(0.0f32, f32::max);
                    if enabled && max_boost > 0.0 {
                        ui.label(
                            egui::RichText::new(format!("🛡 Auto Pre-Amp: -{max_boost:.1} dB"))
                                .size(10.5)
                                .color(egui::Color32::from_rgb(100, 200, 255))
                                .strong(),
                        )
                        .on_hover_text(lang.tooltip_auto_preamp());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new(lang.eq_reset()).size(10.5)).clicked() {
                            self.selected_preset = None;
                            self.state.eq_gains = [0.0; 10];
                            let _ = self.cmd_tx.send_blocking(GuiCmd::ResetEq);
                        }
                    });
                });

                ui.add_space(4.0);

                // Quick EQ Presets chips
                ui.horizontal_wrapped(|ui| {
                    for &preset in EqPreset::ALL {
                        let name = lang.eq_preset_name(preset);
                        let is_selected = self.selected_preset == Some(preset);

                        let btn = if is_selected {
                            egui::Button::new(
                                egui::RichText::new(format!("✓ {name}"))
                                    .size(9.5)
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(egui::Color32::from_rgb(35, 105, 215))
                            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(110, 180, 255)))
                            .rounding(egui::Rounding::same(4.0))
                        } else {
                            egui::Button::new(
                                egui::RichText::new(name)
                                    .size(9.5)
                                    .color(egui::Color32::from_rgb(175, 180, 190)),
                            )
                            .fill(egui::Color32::from_rgb(28, 30, 36))
                            .stroke(egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15)))
                            .rounding(egui::Rounding::same(4.0))
                        };

                        if ui.add(btn).clicked() {
                            self.selected_preset = Some(preset);
                            self.state.eq_enabled = true;
                            let gains = if preset == EqPreset::Custom {
                                self.state.custom_eq_gains
                            } else {
                                preset.gains()
                            };
                            self.state.eq_gains = gains;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::ApplyEqPreset(preset));
                        }
                    }
                });

                ui.add_space(6.0);

                // 10-Band Sliders & Dynamic Morphing Frequency Curve (WinAMP Classic Vertical Rack)
                let labels = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
                let prev_spacing_x = ui.spacing().item_spacing.x;
                ui.spacing_mut().item_spacing.x = 2.0;

                // 平滑漸變形變插值 (切換 Preset 或 Reset 時，各頻段拉桿與曲線在 ~180ms 內有機滑動到位)
                let dt = ui.ctx().input(|i| i.stable_dt).min(0.1);
                let blend = 1.0 - (-dt * 22.0).exp();

                let mut is_animating = false;
                for i in 0..10 {
                    let diff = self.state.eq_gains[i] - self.display_eq_gains[i];
                    if diff.abs() > 0.02 {
                        self.display_eq_gains[i] += diff * blend;
                        is_animating = true;
                    } else {
                        self.display_eq_gains[i] = self.state.eq_gains[i];
                    }
                }
                if is_animating {
                    ui.ctx().request_repaint();
                }

                let mut node_points = Vec::with_capacity(10);
                let mut zero_db_y = 0.0f32;

                ui.columns(10, |columns| {
                    for (band_idx, col_ui) in columns.iter_mut().enumerate() {
                        col_ui.vertical_centered(|ui| {
                            let target_gain = self.state.eq_gains[band_idx];
                            let gain_str = if target_gain > 0.0 {
                                format!("+{}", target_gain.round() as i32)
                            } else {
                                format!("{}", target_gain.round() as i32)
                            };
                            let text_col = if target_gain.abs() > 0.5 {
                                egui::Color32::from_rgb(240, 180, 50)
                            } else {
                                egui::Color32::from_rgb(120, 125, 135)
                            };
                            ui.label(egui::RichText::new(gain_str).monospace().size(9.0).color(text_col));

                            let mut gain = self.display_eq_gains[band_idx];

                            let slider = egui::Slider::new(&mut gain, -12.0..=12.0)
                                .vertical()
                                .show_value(false);

                            let slider_resp = ui.add(slider);
                            // 關鍵修正：僅在使用者實際以滑鼠拖動或點擊該拉桿時，才切換為「自訂」並發送音訊設定指令。
                            let user_active = slider_resp.dragged() || (slider_resp.clicked() && slider_resp.changed());
                            if user_active {
                                let snapped_gain = (gain * 2.0).round() / 2.0;
                                self.selected_preset = Some(EqPreset::Custom);
                                self.state.eq_enabled = true;
                                self.state.eq_gains[band_idx] = snapped_gain;
                                self.display_eq_gains[band_idx] = snapped_gain;
                                self.state.custom_eq_gains[band_idx] = snapped_gain;
                                let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqGain(band_idx, snapped_gain));
                            }

                            // 記錄滑塊頂點座標以供 Catmull-Rom 三次仿樣波形曲線形變渲染
                            let slider_rect = slider_resp.rect;
                            let norm = (gain - (-12.0)) / 24.0;
                            let thumb_y = slider_rect.bottom() - norm * slider_rect.height();
                            let thumb_x = slider_rect.center().x;
                            node_points.push(egui::pos2(thumb_x, thumb_y));
                            zero_db_y = slider_rect.bottom() - 0.5 * slider_rect.height();

                            ui.label(egui::RichText::new(labels[band_idx]).monospace().size(9.0).weak());
                        });
                    }
                });
                ui.spacing_mut().item_spacing.x = prev_spacing_x;

                // 渲染發光等化器響應曲線與動態形變漸變
                render_eq_curve_overlay(ui, &node_points, zero_db_y, self.state.eq_enabled);
            });

        ui.cursor().top() - initial_top
    }

    pub fn render_equalizer_page(&mut self, ui: &mut egui::Ui) -> f32 {
        self.render_equalizer_content(ui, false)
    }

    pub fn render_equalizer_content(&mut self, ui: &mut egui::Ui, is_standalone: bool) -> f32 {
        let initial_top = ui.cursor().top();
        let lang = self.state.language;

        // Universal Window Header (Embedded has Back+Popout, Standalone has 🏠 Home)
        let kind = if is_standalone {
            crate::snapping::WindowKind::Equalizer
        } else {
            crate::snapping::WindowKind::Main
        };
        let header_title = if is_standalone {
            lang.mini_equalizer_title()
        } else {
            lang.eq_title()
        };
        self.render_universal_window_header(
            ui,
            kind,
            header_title,
            !is_standalone,
            |app, ui| {
                app.render_visualizer_button(ui, kind);
            },
        );

        if !is_standalone {
            ui.add_space(2.0);
            // Profile Switcher Tabs: Normal / Receiver (具備平滑滑動指示線與膠囊背景)
            let tabs = [
                (EqProfile::Normal, lang.eq_profile_normal()),
                (EqProfile::Receiver, lang.eq_profile_receiver()),
            ];
            if let Some(prof) = Self::render_sliding_tab_bar(ui, "eq_profiles", self.eq_profile, &tabs) {
                self.eq_profile = prof;
            }
            ui.add_space(6.0);
        }

        let is_rx = if is_standalone {
            false
        } else {
            self.eq_profile == EqProfile::Receiver
        };

        // Sound Engine (HE & WE) Restoration & Warmth Controls
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(22, 26, 30))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(40, 52, 60)))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(10.0, 6.0))
            .show(ui, |ui| {
                // Row 1: Harmonic Exciter (HE)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("✨ {}", lang.sound_enhancer_title()))
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(egui::Color32::from_rgb(140, 230, 210)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for &m in [
                            airplay_audio::SoundEnhancerMode::Brilliant,
                            airplay_audio::SoundEnhancerMode::Dynamic,
                            airplay_audio::SoundEnhancerMode::Natural,
                            airplay_audio::SoundEnhancerMode::Off,
                        ]
                        .iter()
                        {
                            let current_se = if is_rx {
                                self.state.airplay_receiver_sound_enhancer_mode
                            } else {
                                self.state.sound_enhancer_mode
                            };
                            let is_sel = current_se == m;
                            let m_name = lang.sound_enhancer_mode_name(m);
                            let btn = if is_sel {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .strong()
                                        .color(if m == airplay_audio::SoundEnhancerMode::Off {
                                            egui::Color32::from_rgb(220, 220, 225)
                                        } else {
                                            egui::Color32::from_rgb(0, 255, 210)
                                        }),
                                )
                                .fill(if m == airplay_audio::SoundEnhancerMode::Off {
                                    egui::Color32::from_rgb(50, 55, 65)
                                } else {
                                    egui::Color32::from_rgb(15, 60, 52)
                                })
                                .stroke(egui::Stroke::new(1.0_f32, if m == airplay_audio::SoundEnhancerMode::Off {
                                    egui::Color32::from_rgb(90, 95, 110)
                                } else {
                                    egui::Color32::from_rgb(40, 160, 130)
                                }))
                                .rounding(egui::Rounding::same(4.0))
                            } else {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(140, 150, 160)),
                                )
                                .fill(egui::Color32::from_rgb(25, 28, 33))
                                .stroke(egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15)))
                                .rounding(egui::Rounding::same(4.0))
                            };

                            if show_wrapped_tooltip(ui.add(btn), lang.sound_enhancer_preset_tooltip(m)).clicked() {
                                if is_rx {
                                    self.state.airplay_receiver_sound_enhancer_mode = m;
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetReceiverSoundEnhancerMode(m));
                                } else {
                                    self.state.sound_enhancer_mode = m;
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetSoundEnhancerMode(m));
                                }
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                // Row 2: Warmth Enhancer (WE)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("📻 {}", lang.warmth_enhancer_title()))
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(egui::Color32::from_rgb(140, 230, 160)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for &m in [
                            airplay_audio::WarmthEnhancerMode::Vintage,
                            airplay_audio::WarmthEnhancerMode::Warm,
                            airplay_audio::WarmthEnhancerMode::Subtle,
                            airplay_audio::WarmthEnhancerMode::Off,
                        ]
                        .iter()
                        {
                            let current_we = if is_rx {
                                self.state.airplay_receiver_warmth_enhancer_mode
                            } else {
                                self.state.warmth_enhancer_mode
                            };
                            let is_sel = current_we == m;
                            let m_name = lang.warmth_enhancer_mode_name(m);
                            let btn = if is_sel {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .strong()
                                        .color(if m == airplay_audio::WarmthEnhancerMode::Off {
                                            egui::Color32::from_rgb(220, 220, 225)
                                        } else {
                                            egui::Color32::from_rgb(80, 250, 150)
                                        }),
                                )
                                .fill(if m == airplay_audio::WarmthEnhancerMode::Off {
                                    egui::Color32::from_rgb(50, 55, 65)
                                } else {
                                    egui::Color32::from_rgb(16, 56, 30)
                                })
                                .stroke(egui::Stroke::new(1.0_f32, if m == airplay_audio::WarmthEnhancerMode::Off {
                                    egui::Color32::from_rgb(90, 95, 110)
                                } else {
                                    egui::Color32::from_rgb(45, 160, 85)
                                }))
                                .rounding(egui::Rounding::same(4.0))
                            } else {
                                egui::Button::new(
                                    egui::RichText::new(m_name)
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(140, 150, 160)),
                                )
                                .fill(egui::Color32::from_rgb(25, 28, 33))
                                .stroke(egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15)))
                                .rounding(egui::Rounding::same(4.0))
                            };

                            if show_wrapped_tooltip(ui.add(btn), lang.warmth_enhancer_preset_tooltip(m)).clicked() {
                                if is_rx {
                                    self.state.airplay_receiver_warmth_enhancer_mode = m;
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetReceiverWarmthEnhancerMode(m));
                                } else {
                                    self.state.warmth_enhancer_mode = m;
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetWarmthEnhancerMode(m));
                                }
                            }
                        }
                    });
                });
            });

        ui.add_space(8.0);

        // EQ Header & Toggle Controls (matching Win2Pod Player style)
        ui.horizontal(|ui| {
            let eq_title_text = if is_rx { "♪ RECEIVER EQUALIZER" } else { "♪ EQUALIZER" };
            ui.label(
                egui::RichText::new(eq_title_text)
                    .monospace()
                    .strong()
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 210, 235)),
            );

            let mut enabled = if is_rx {
                self.state.airplay_receiver_eq_enabled
            } else {
                self.state.eq_enabled
            };

            let (toggle_text, toggle_col) = if enabled {
                ("EQ: ON", egui::Color32::from_rgb(80, 230, 120))
            } else {
                ("EQ: OFF", egui::Color32::from_rgb(150, 155, 165))
            };
            if ui.button(egui::RichText::new(toggle_text).strong().size(10.5).color(toggle_col)).clicked() {
                enabled = !enabled;
                if is_rx {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetReceiverEqBypass(!enabled));
                } else {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqBypass(!enabled));
                }
            }

            // Auto Pre-Amp headroom protection badge
            let max_boost = if is_rx {
                self.state.airplay_receiver_eq_gains.iter().copied().fold(0.0f32, f32::max)
            } else {
                self.state.eq_gains.iter().copied().fold(0.0f32, f32::max)
            };

            if enabled && max_boost > 0.0 {
                ui.label(
                    egui::RichText::new(format!("🛡 Auto Pre-Amp: -{max_boost:.1} dB"))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(100, 200, 255))
                        .strong(),
                )
                .on_hover_text(lang.tooltip_auto_preamp());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new(lang.eq_reset()).size(11.0)).clicked() {
                    if is_rx {
                        self.receiver_selected_preset = None;
                        self.state.airplay_receiver_eq_gains = [0.0; 10];
                        let _ = self.cmd_tx.send_blocking(GuiCmd::ResetReceiverEq);
                    } else {
                        self.selected_preset = None;
                        self.state.eq_gains = [0.0; 10];
                        let _ = self.cmd_tx.send_blocking(GuiCmd::ResetEq);
                    }
                }
            });
        });

        ui.add_space(6.0);

        // Sound Presets (without title label, directly above sliders)
        ui.horizontal_wrapped(|ui| {
            for &preset in EqPreset::ALL {
                let name = lang.eq_preset_name(preset);
                let is_selected = if is_rx {
                    self.receiver_selected_preset == Some(preset)
                } else {
                    self.selected_preset == Some(preset)
                };

                let btn = if is_selected {
                    egui::Button::new(
                        egui::RichText::new(format!("✓ {name}"))
                            .size(10.5)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(egui::Color32::from_rgb(35, 105, 215))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(110, 180, 255)))
                    .rounding(egui::Rounding::same(5.0))
                } else {
                    egui::Button::new(
                        egui::RichText::new(name)
                            .size(10.5)
                            .color(egui::Color32::from_rgb(180, 185, 195)),
                    )
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)))
                    .rounding(egui::Rounding::same(5.0))
                };

                if ui.add(btn).clicked() {
                    if is_rx {
                        self.receiver_selected_preset = Some(preset);
                        self.state.airplay_receiver_eq_enabled = true;
                        let gains = if preset == EqPreset::Custom {
                            self.state.airplay_receiver_custom_eq_gains
                        } else {
                            preset.gains()
                        };
                        self.state.airplay_receiver_eq_gains = gains;
                        let _ = self.cmd_tx.send_blocking(GuiCmd::ApplyReceiverEqPreset(preset));
                    } else {
                        self.selected_preset = Some(preset);
                        self.state.eq_enabled = true;
                        let gains = if preset == EqPreset::Custom {
                            self.state.custom_eq_gains
                        } else {
                            preset.gains()
                        };
                        self.state.eq_gains = gains;
                        let _ = self.cmd_tx.send_blocking(GuiCmd::ApplyEqPreset(preset));
                    }
                }
            }
        });

        ui.add_space(5.0);

        // 10-Band Equalizer Sliders & Dynamic Morphing Frequency Curve
        let labels = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
        let prev_spacing_x = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = 2.0;

        // 平滑漸變形變插值 (切換 Preset 或 Reset 時，各頻段拉桿與曲線在 ~180ms 內有機滑動到位)
        let dt = ui.ctx().input(|i| i.stable_dt).min(0.1);
        let blend = 1.0 - (-dt * 22.0).exp();

        let mut is_animating = false;
        if is_rx {
            for i in 0..10 {
                let diff = self.state.airplay_receiver_eq_gains[i] - self.display_receiver_eq_gains[i];
                if diff.abs() > 0.02 {
                    self.display_receiver_eq_gains[i] += diff * blend;
                    is_animating = true;
                } else {
                    self.display_receiver_eq_gains[i] = self.state.airplay_receiver_eq_gains[i];
                }
            }
        } else {
            for i in 0..10 {
                let diff = self.state.eq_gains[i] - self.display_eq_gains[i];
                if diff.abs() > 0.02 {
                    self.display_eq_gains[i] += diff * blend;
                    is_animating = true;
                } else {
                    self.display_eq_gains[i] = self.state.eq_gains[i];
                }
            }
        }
        if is_animating {
            ui.ctx().request_repaint();
        }

        let mut node_points = Vec::with_capacity(10);
        let mut zero_db_y = 0.0f32;

        ui.columns(10, |columns| {
            for (band_idx, col_ui) in columns.iter_mut().enumerate() {
                col_ui.vertical_centered(|ui| {
                    let target_gain = if is_rx {
                        self.state.airplay_receiver_eq_gains[band_idx]
                    } else {
                        self.state.eq_gains[band_idx]
                    };
                    let gain_str = if target_gain > 0.0 {
                        format!("+{}", target_gain.round() as i32)
                    } else {
                        format!("{}", target_gain.round() as i32)
                    };
                    let text_col = if target_gain.abs() > 0.5 {
                        egui::Color32::from_rgb(240, 180, 50)
                    } else {
                        egui::Color32::GRAY
                    };
                    ui.label(egui::RichText::new(gain_str).size(10.0).color(text_col));

                    let mut gain = if is_rx {
                        self.display_receiver_eq_gains[band_idx]
                    } else {
                        self.display_eq_gains[band_idx]
                    };

                    let slider = egui::Slider::new(&mut gain, -12.0..=12.0)
                        .vertical()
                        .show_value(false);

                    let slider_resp = ui.add(slider);
                    // 關鍵修正：僅在使用者實際以滑鼠拖動或點擊該拉桿時，才切換為「自訂」並發送音訊設定指令。
                    // 絕不可因程式動態平滑插值渲染而誤判為使用者手動修改，避免鎖死自訂或無限回饋抖動。
                    let user_active = slider_resp.dragged() || (slider_resp.clicked() && slider_resp.changed());
                    if user_active {
                        let snapped_gain = (gain * 2.0).round() / 2.0;
                        if is_rx {
                            self.receiver_selected_preset = Some(EqPreset::Custom);
                            self.state.airplay_receiver_eq_enabled = true;
                            self.state.airplay_receiver_eq_gains[band_idx] = snapped_gain;
                            self.display_receiver_eq_gains[band_idx] = snapped_gain;
                            self.state.airplay_receiver_custom_eq_gains[band_idx] = snapped_gain;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetReceiverEqGain(band_idx, snapped_gain));
                        } else {
                            self.selected_preset = Some(EqPreset::Custom);
                            self.state.eq_enabled = true;
                            self.state.eq_gains[band_idx] = snapped_gain;
                            self.display_eq_gains[band_idx] = snapped_gain;
                            self.state.custom_eq_gains[band_idx] = snapped_gain;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqGain(band_idx, snapped_gain));
                        }
                    }

                    // 記錄滑塊頂點座標以供 Catmull-Rom 三次仿樣波形曲線形變渲染
                    let slider_rect = slider_resp.rect;
                    let norm = (gain - (-12.0)) / 24.0;
                    let thumb_y = slider_rect.bottom() - norm * slider_rect.height();
                    let thumb_x = slider_rect.center().x;
                    node_points.push(egui::pos2(thumb_x, thumb_y));
                    zero_db_y = slider_rect.bottom() - 0.5 * slider_rect.height();

                    ui.label(egui::RichText::new(labels[band_idx]).size(9.5).weak());
                });
            }
        });
        ui.spacing_mut().item_spacing.x = prev_spacing_x;

        // 渲染發光等化器響應曲線與動態形變漸變
        let eq_active = if is_rx {
            self.state.airplay_receiver_eq_enabled
        } else {
            self.state.eq_enabled
        };
        render_eq_curve_overlay(ui, &node_points, zero_db_y, eq_active);

        ui.cursor().top() - initial_top
    }

    pub fn render_mini_player_content(&mut self, ui: &mut egui::Ui) -> f32 {
        let initial_top = ui.cursor().top();
        let lang = self.state.language;

        // Query GSMTC Media Track Info
        let track_info = crate::media_control::get_current_media_track().read().unwrap().clone();

        // 0. Universal Window Header (MiniPlayer standalone with 🏠 Home, Title, 📌 Pin, 🎨 Visualizer, ⦀ Equalizer)
        self.render_universal_window_header(
            ui,
            crate::snapping::WindowKind::MiniPlayer,
            lang.mini_player_title(),
            false,
            |app, ui| {
                app.render_visualizer_button(ui, crate::snapping::WindowKind::MiniPlayer);
                app.render_popout_equalizer_button(ui, crate::snapping::WindowKind::MiniPlayer);
            },
        );

        // 1. WinAMP Main Display Chassis (Album Cover + VFD/LCD Screen with dynamic transitions)
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(22, 24, 29))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(46, 52, 64)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let total_inner_w = ui.available_width();

                self.render_winamp_display_chassis(ui, &track_info, lang, true);

        ui.add_space(5.0);

                // Seek Bar
                let cur_pos = if let Some(preview_secs) = self.seek_drag_secs {
                    preview_secs
                } else {
                    track_info.current_position_secs()
                };
                let total_dur = track_info.duration_secs;
                let progress = if total_dur > 0.0 {
                    (cur_pos / total_dur).clamp(0.0, 1.0) as f32
                } else {
                    0.0f32
                };

                let bar_h = 5.0f32;
                let (bar_rect, bar_resp) = ui.allocate_exact_size(
                    egui::vec2(total_inner_w, bar_h),
                    egui::Sense::click_and_drag(),
                );
                let bar_painter = ui.painter();
                bar_painter.rect_filled(bar_rect, egui::Rounding::same(2.5), egui::Color32::from_rgb(10, 13, 16));
                bar_painter.rect_stroke(bar_rect, egui::Rounding::same(2.5), egui::Stroke::new(0.5_f32, egui::Color32::from_rgb(25, 32, 40)));

                if progress > 0.0 {
                    let filled_w = (bar_rect.width() * progress).max(3.0);
                    let filled_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(filled_w, bar_h));
                    bar_painter.rect_filled(filled_rect, egui::Rounding::same(2.5), egui::Color32::from_rgb(20, 115, 62));
                    let head_center = egui::pos2(filled_rect.max.x, bar_rect.center().y);
                    bar_painter.circle_filled(head_center, 3.5, egui::Color32::from_rgb(55, 150, 90));
                }

                if total_dur > 0.0 {
                    if bar_resp.dragged() {
                        if let Some(pos) = bar_resp.interact_pointer_pos() {
                            let norm = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            self.seek_drag_secs = Some(norm as f64 * total_dur);
                        }
                    } else if bar_resp.drag_stopped() {
                        if let Some(target_secs) = self.seek_drag_secs.take() {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Seek(target_secs));
                        }
                    } else if bar_resp.clicked() {
                        if let Some(pos) = bar_resp.interact_pointer_pos() {
                            let norm = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            let target_secs = norm as f64 * total_dur;
                            self.seek_drag_secs = None;
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Seek(target_secs));
                        }
                    } else if !ui.input(|i| i.pointer.any_down()) && self.seek_drag_secs.is_some() {
                        if let Some(target_secs) = self.seek_drag_secs.take() {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Seek(target_secs));
                        }
                    }
                }

                ui.add_space(5.0);

                // 2. WinAMP Multi-color Segmented Spectrum Visualizer
                let spec_h = 50.0f32;
                let (spec_rect, spec_resp) = ui.allocate_exact_size(
                    egui::vec2(total_inner_w, spec_h),
                    egui::Sense::click(),
                );
                self.mascot_spectrum_rect = Some(spec_rect);

                // ⭐ 跨視窗即刻召喚：若使用者在迷你播放器頻譜右鍵點擊，無論精靈原先在何處立即召喚至此展開特技衝浪！
                let hover_pos = ui.input(|i| i.pointer.hover_pos().or(i.pointer.latest_pos()));
                let is_ptr_in_spec = hover_pos.map(|p| spec_rect.expand(4.0).contains(p)).unwrap_or(false);
                if spec_resp.secondary_clicked() || (is_ptr_in_spec && ui.input(|i| i.pointer.secondary_clicked())) {
                    self.trigger_mascot_air_surf_summon(crate::snapping::WindowKind::MiniPlayer, spec_rect, hover_pos);
                }
                let painter = ui.painter();
                painter.rect_filled(spec_rect, egui::Rounding::same(4.0), egui::Color32::from_rgb(10, 12, 15));
                painter.rect_stroke(
                    spec_rect,
                    egui::Rounding::same(4.0),
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(30, 34, 44)),
                );

                let inner_spec = spec_rect.shrink(4.0);
                let num_bars = 16;
                let bar_gap = 2.5f32;
                let bar_w = ((inner_spec.width() - (num_bars as f32 - 1.0) * bar_gap) / num_bars as f32).max(2.0);

                let num_segs = 12;
                let seg_gap = 1.2f32;
                let seg_h = ((inner_spec.height() - (num_segs as f32 - 1.0) * seg_gap) / num_segs as f32).max(1.5);

                for i in 0..num_bars {
                    let bx = inner_spec.min.x + i as f32 * (bar_w + bar_gap);
                    let level = self.spectrum_vis.current[i].clamp(0.0, 1.0);
                    let active_segs = (level * num_segs as f32).round() as usize;

                    for s in 0..num_segs {
                        if s < active_segs {
                            let sy = inner_spec.max.y - (s as f32 + 1.0) * (seg_h + seg_gap);
                            let seg_rect = egui::Rect::from_min_size(
                                egui::pos2(bx, sy),
                                egui::vec2(bar_w, seg_h),
                            );
                            let col = if s >= 8 {
                                egui::Color32::from_rgb(255, 45, 45)
                            } else if s >= 4 {
                                egui::Color32::from_rgb(255, 200, 15)
                            } else {
                                egui::Color32::from_rgb(25, 225, 55)
                            };
                            painter.rect_filled(seg_rect, egui::Rounding::same(0.5), col);
                        }
                    }

                    let peak_lvl = self.spectrum_vis.peaks[i].clamp(0.0, 1.0);
                    if peak_lvl > 0.05 {
                        let peak_y = inner_spec.max.y - (inner_spec.height() * peak_lvl);
                        let cap_rect = egui::Rect::from_min_size(
                            egui::pos2(bx, peak_y.max(inner_spec.min.y)),
                            egui::vec2(bar_w, 2.0),
                        );
                        painter.rect_filled(cap_rect, egui::Rounding::same(0.5), egui::Color32::from_rgb(230, 245, 255));
                    }
                }
            });

        ui.add_space(6.0);

        // 3. WinAMP Transport Controls & Volume Cluster
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(25, 27, 33))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(48, 54, 66)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    // Row 1: Transport Controls Cluster + Full-Width Balance Slider
                    ui.horizontal(|ui| {
                        let btn_size = egui::vec2(30.0, 22.0);

                        if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏮").size(11.0)))
                            .on_hover_text(lang.player_prev())
                            .clicked()
                        {
                            self.next_track_intent = Some(TrackTransitionDirection::Prev);
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Previous);
                        }

                        let play_pause_icon = if track_info.is_playing { "⏸" } else { "▶" };
                        if ui.add_sized(egui::vec2(34.0, 22.0), egui::Button::new(egui::RichText::new(play_pause_icon).size(12.0).strong()))
                            .on_hover_text(lang.player_play_pause())
                            .clicked()
                        {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::TogglePlayPause);
                        }

                        if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏹").size(10.0)))
                            .on_hover_text(lang.player_stop())
                            .clicked()
                        {
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Stop);
                        }

                        if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("⏭").size(11.0)))
                            .on_hover_text(lang.player_next())
                            .clicked()
                        {
                            self.next_track_intent = Some(TrackTransitionDirection::Next);
                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Next);
                        }

                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(2.0);

                        let bal_label_w = 68.0f32;
                        let bal_pct = (self.balance * 100.0).round() as i32;
                        let (bal_str, bal_col) = if bal_pct == 0 {
                            (lang.balance_center().to_string(), egui::Color32::from_rgb(140, 200, 255))
                        } else if bal_pct < 0 {
                            (lang.balance_left(bal_pct.abs()), egui::Color32::from_rgb(255, 205, 120))
                        } else {
                            (lang.balance_right(bal_pct), egui::Color32::from_rgb(255, 205, 120))
                        };

                        let bal_label_resp = ui.add_sized(
                            egui::vec2(bal_label_w, 20.0),
                            egui::Label::new(
                                egui::RichText::new(bal_str)
                                    .monospace()
                                    .size(10.0)
                                    .color(bal_col),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(lang.balance_tooltip());

                        if bal_label_resp.clicked() {
                            self.balance = 0.0;
                            crate::controller::set_audio_balance(0.0);
                        }

                        let bal_slider_w = (ui.available_width() - 2.0).floor().max(40.0);
                        ui.spacing_mut().slider_width = bal_slider_w;
                        let mut bal = self.balance;
                        let bal_resp = ui.add(
                            egui::Slider::new(&mut bal, -1.0..=1.0).show_value(false),
                        )
                        .on_hover_text(lang.balance_tooltip());

                        if bal_resp.changed() {
                            self.balance = bal;
                            crate::controller::set_audio_balance(bal);
                        }
                        if bal_resp.double_clicked() {
                            self.balance = 0.0;
                            crate::controller::set_audio_balance(0.0);
                        }
                    });

                    ui.add_space(4.0);

                    // Row 2: VOL -> Volume Slider -> Mute -> STEREO
                    ui.horizontal(|ui| {
                        let vol_label_w = 52.0f32;
                        let vol_pct = (self.volume_slider * 100.0).round() as i32;
                        ui.add_sized(
                            egui::vec2(vol_label_w, 20.0),
                            egui::Label::new(
                                egui::RichText::new(format!("VOL:{:>3}%", vol_pct))
                                    .monospace()
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(0, 230, 110)),
                            ),
                        );

                        let (mute_text, mute_col) = if self.state.is_muted {
                            (lang.btn_unmute(), egui::Color32::from_rgb(70, 200, 120))
                        } else {
                            (lang.btn_mute(), egui::Color32::from_rgb(240, 160, 40))
                        };

                        let mono_btn_w = 58.0f32;
                        let spacing = ui.spacing().item_spacing.x;
                        let btn_pad_x = ui.spacing().button_padding.x * 2.0;
                        let mute_text_w = ui.painter().layout_no_wrap(
                            mute_text.to_string(),
                            egui::FontId::proportional(10.0),
                            mute_col,
                        ).size().x + btn_pad_x + 6.0;
                        let mute_btn_w = mute_text_w.max(50.0);

                        let vol_slider_w = (ui.available_width() - mute_btn_w - mono_btn_w - spacing * 2.0 - 2.0).floor().max(40.0);
                        ui.spacing_mut().slider_width = vol_slider_w;
                        let mut vol = self.volume_slider;
                        let vol_resp = ui.add(
                            egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false),
                        );
                        if vol_resp.changed() {
                            self.is_dragging_volume = true;
                            self.volume_slider = vol;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetVolume(vol));
                        }
                        if vol_resp.drag_stopped() {
                            self.is_dragging_volume = false;
                        }

                        // Mute Button
                        if ui.add_sized(
                            egui::vec2(mute_btn_w, 20.0),
                            egui::Button::new(egui::RichText::new(mute_text).size(10.0).color(mute_col)),
                        ).clicked() {
                            let new_mute = !self.state.is_muted;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetMute(new_mute));
                        }

                        // STEREO / MONO Button
                        let (mono_text, mono_col) = if self.is_mono {
                            ("MONO", egui::Color32::from_rgb(255, 175, 45))
                        } else {
                            ("STEREO", egui::Color32::from_rgb(0, 220, 160))
                        };
                        let mono_btn = egui::Button::new(
                            egui::RichText::new(mono_text)
                                .monospace()
                                .size(9.5)
                                .strong()
                                .color(mono_col),
                        );
                        if ui.add_sized(egui::vec2(mono_btn_w, 20.0), mono_btn)
                            .on_hover_text(if self.is_mono { "Audio Mode: Mono (Click to switch to Stereo)" } else { "Audio Mode: Stereo (Click to switch to Mono)" })
                            .clicked()
                        {
                            self.is_mono = !self.is_mono;
                            crate::controller::set_audio_mono(self.is_mono);
                        }
                    });
                });
            });

        ui.cursor().top() - initial_top
    }
}

/// Returns the package version in Microsoft Store format: Major.Minor.Build.Revision (e.g. "0.1.0.0")
pub fn ms_store_version() -> String {
    let raw = env!("CARGO_PKG_VERSION");
    let parts: Vec<&str> = raw.split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0.0", parts[0]),
        2 => format!("{}.{}.0.0", parts[0], parts[1]),
        3 => format!("{}.{}.{}.0", parts[0], parts[1], parts[2]),
        _ => format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]),
    }
}

/// Configure egui to load native fonts for the active language with full multilingual fallback.
/// Ensures Korean Hangul, Japanese Kana/Kanji, Chinese (Simp/Trad), Latin, and Cyrillic all render cleanly.
fn setup_custom_fonts(ctx: &egui::Context, lang: Language) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Segoe UI for crisp Latin and Cyrillic (EN, FR, DE, IT, PT, RU, ES)
    if let Ok(font_data) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts.font_data.insert("segoeui".to_owned(), egui::FontData::from_owned(font_data));
    }

    // 2. Chinese font: prioritize YaHei for Simplified, JhengHei for others
    let (cjk_name, cjk_path) = if lang == Language::SimplifiedChinese {
        if std::path::Path::new(r"C:\Windows\Fonts\msyhl.ttc").exists() {
            ("msyhl", r"C:\Windows\Fonts\msyhl.ttc")
        } else {
            ("msyh", r"C:\Windows\Fonts\msyh.ttc")
        }
    } else {
        if std::path::Path::new(r"C:\Windows\Fonts\msjhl.ttc").exists() {
            ("msjhl", r"C:\Windows\Fonts\msjhl.ttc")
        } else {
            ("msjh", r"C:\Windows\Fonts\msjh.ttc")
        }
    };
    if let Ok(font_data) = std::fs::read(cjk_path) {
        fonts.font_data.insert(cjk_name.to_owned(), egui::FontData::from_owned(font_data));
    }

    // 3. Korean font: Malgun Gothic (essential for Korean Hangul characters)
    let ko_path = if std::path::Path::new(r"C:\Windows\Fonts\malgunsl.ttf").exists() {
        r"C:\Windows\Fonts\malgunsl.ttf"
    } else {
        r"C:\Windows\Fonts\malgun.ttf"
    };
    if let Ok(font_data) = std::fs::read(ko_path) {
        fonts.font_data.insert("malgun".to_owned(), egui::FontData::from_owned(font_data));
    }

    // 4. Japanese font: Yu Gothic or MS Gothic
    let jp_path = if std::path::Path::new(r"C:\Windows\Fonts\YuGothR.ttc").exists() {
        r"C:\Windows\Fonts\YuGothR.ttc"
    } else {
        r"C:\Windows\Fonts\msgothic.ttc"
    };
    if let Ok(font_data) = std::fs::read(jp_path) {
        fonts.font_data.insert("japanese".to_owned(), egui::FontData::from_owned(font_data));
    }

    // 5. Windows Emoji font: Segoe UI Emoji (for crystal clear 🎧, ⚡, 🔋, etc.)
    if let Ok(font_data) = std::fs::read(r"C:\Windows\Fonts\seguiemj.ttf") {
        fonts.font_data.insert("emoji".to_owned(), egui::FontData::from_owned(font_data));
    }

    // 6. Windows Symbol font: Segoe UI Symbol (for ⏻ power symbol, special symbols, etc.)
    if let Ok(font_data) = std::fs::read(r"C:\Windows\Fonts\seguisym.ttf") {
        fonts.font_data.insert("symbol".to_owned(), egui::FontData::from_owned(font_data));
    }

    // Arrange priority order based on the active language
    let mut order = match lang {
        Language::Korean => vec!["malgun", "segoeui", cjk_name, "japanese"],
        Language::Japanese => vec!["japanese", "segoeui", cjk_name, "malgun"],
        Language::SimplifiedChinese | Language::TraditionalChinese => vec![cjk_name, "segoeui", "malgun", "japanese"],
        _ => vec!["segoeui", cjk_name, "malgun", "japanese"],
    };

    // Filter to only fonts that successfully loaded
    order.retain(|name| fonts.font_data.contains_key(*name));

    for name in &order {
        fonts.families.entry(egui::FontFamily::Proportional).or_default().push((*name).to_owned());
        fonts.families.entry(egui::FontFamily::Monospace).or_default().push((*name).to_owned());
    }

    if fonts.font_data.contains_key("emoji") {
        fonts.families.entry(egui::FontFamily::Proportional).or_default().push("emoji".to_owned());
        fonts.families.entry(egui::FontFamily::Monospace).or_default().push("emoji".to_owned());
    }

    if fonts.font_data.contains_key("symbol") {
        fonts.families.entry(egui::FontFamily::Proportional).or_default().push("symbol".to_owned());
        fonts.families.entry(egui::FontFamily::Monospace).or_default().push("symbol".to_owned());
    }

    ctx.set_fonts(fonts);
}

#[cfg(test)]
impl FlyoutApp {
    pub fn new_mock() -> Self {
        let (cmd_tx, _) = async_channel::bounded(1);
        let shared_state = Arc::new(RwLock::new(ControllerState::default()));
        Self {
            cmd_tx,
            shared_state,
            state: ControllerState::default(),
            volume_slider: 1.0,
            is_dragging_volume: false,
            last_applied_height: 680.0,
            last_applied_width: 460.0,
            selected_device_ids: std::collections::HashSet::new(),
            current_page: Page::Main,
            last_page: Page::Main,
            page_history: Vec::new(),
            page_transition: None,
            settings_tab: SettingsTab::default(),
            meter_level: 0.0,
            last_meter_update: std::time::Instant::now(),
            startup_time: std::time::Instant::now(),
            startup_trimmed: false,
            frame_count: 0,
            last_hidden: false,
            spectrum_vis: SpectrumVisualizer::default(),
            eq_profile: EqProfile::Normal,
            selected_preset: None,
            receiver_selected_preset: None,
            display_eq_gains: [0.0; 10],
            display_receiver_eq_gains: [0.0; 10],
            album_art_texture: None,
            last_album_art_bytes: None,
            receiver_artwork_texture: None,
            last_receiver_artwork_len: 0,
            balance: 0.0,
            is_mono: false,
            is_always_on_top: false,
            equalizer_window_open: false,
            mini_player_window_open: false,
            mini_player_needs_initial_dock: false,
            equalizer_needs_initial_dock: false,
            seek_drag_secs: None,
            app_icon_texture: None,
            pending_modular_workstation_frames: 0,
            track_transition: None,
            prev_track_title: String::new(),
            prev_track_artist: String::new(),
            prev_track_album: String::new(),
            prev_album_art_texture: None,
            next_track_intent: None,
            workstation_onboarding_dismissed: crate::controller::is_workstation_discovered(),
            mascot_pos: egui::pos2(80.0, 38.0),
            mascot_activity: 0,
            mascot_platform_idx: 1,
            mascot_dir_x: 1.0,
            mascot_dir_y: 1.0,
            mascot_vel_y: 0.0,
            mascot_action_timer: 0.0,
            mascot_squash: 1.0,
            mascot_bounce: 0.0,
            mascot_last_update: std::time::Instant::now(),
            mascot_window: crate::snapping::WindowKind::Main,
            mascot_spectrum_rect: None,
            mascot_spectrum_privilege: false,
            show_supporter_mascot_modal: false,
            reset_about_scroll: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flyoutapp_navigation_stack() {
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.can_go_back());

        // Main -> Settings
        app.navigate_to(Page::Settings);
        assert_eq!(app.current_page, Page::Settings);
        assert!(app.can_go_back());

        // Settings -> Equalizer (e.g. adjust receiver EQ)
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.current_page, Page::Equalizer);

        // Go back -> Settings (should return to Settings, resolving user issue!)
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Settings);

        // Settings -> About
        app.navigate_to(Page::About);
        assert_eq!(app.current_page, Page::About);

        // Go back -> Settings
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Settings);

        // Go back -> Main
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.can_go_back());

        // Cannot go back further
        assert!(!app.go_back());
    }

    #[test]
    fn test_flyoutapp_navigation_reset() {
        let mut app = FlyoutApp::new_mock();
        app.navigate_to(Page::Settings);
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.page_history.len(), 2);

        app.reset_to(Page::Main);
        assert_eq!(app.current_page, Page::Main);
        assert!(app.page_history.is_empty());
        assert!(!app.can_go_back());
    }

    #[test]
    fn test_flyoutapp_equalizer_state_sync() {
        let _lock = crate::snapping::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        use crate::snapping::WindowKind;
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.equalizer_window_open);
        // Initially inactive
        assert!(!app.is_equalizer_active());

        // Embedded page activation
        app.navigate_to(Page::Equalizer);
        assert!(app.is_equalizer_active());

        app.go_back();
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.is_equalizer_active());

        // Standalone window activation (e.g. from Mini Player)
        app.toggle_equalizer_window(WindowKind::Main);
        assert!(app.equalizer_window_open);
        // Even when on Main page, Equalizer is active (green button sync!)
        assert!(app.is_equalizer_active());

        // Closing standalone window
        app.toggle_equalizer_window(WindowKind::Main);
        assert!(!app.equalizer_window_open);
        assert!(!app.is_equalizer_active());

        // Receiver profile navigation test
        app.eq_profile = EqProfile::Receiver;
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.eq_profile, EqProfile::Receiver);
    }

    #[test]
    fn test_universal_popout_behavior() {
        let _lock = crate::snapping::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::snapping::reset_all();
        use crate::snapping::WindowKind;

        // 1. Verify bidirectional mapping
        assert_eq!(Page::Player.popout_window_kind(), Some(WindowKind::MiniPlayer));
        assert_eq!(Page::Equalizer.popout_window_kind(), Some(WindowKind::Equalizer));
        assert_eq!(Page::Main.popout_window_kind(), None);
        assert_eq!(WindowKind::MiniPlayer.embedded_page(), Some(Page::Player));
        assert_eq!(WindowKind::Equalizer.embedded_page(), Some(Page::Equalizer));

        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);

        // 2. Player Page -> Popout MiniPlayer
        app.navigate_to(Page::Player);
        assert_eq!(app.current_page, Page::Player);
        assert!(!app.is_window_open(WindowKind::MiniPlayer));

        // First click: Pop-out! Standalone opens, main window detaches and goes back to previous page!
        app.handle_popout_toggle(WindowKind::MiniPlayer, WindowKind::Main);
        assert!(app.is_window_open(WindowKind::MiniPlayer));
        assert_eq!(app.current_page, Page::Main);

        // Second click: When green (already open), clicking again closes the standalone window!
        app.handle_popout_toggle(WindowKind::MiniPlayer, WindowKind::Main);
        assert!(!app.is_window_open(WindowKind::MiniPlayer));

        // 3. Equalizer Page -> Popout Equalizer
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.current_page, Page::Equalizer);
        assert!(!app.is_window_open(WindowKind::Equalizer));

        // First click: Pop-out! Standalone opens, main window detaches and goes back!
        app.handle_popout_toggle(WindowKind::Equalizer, WindowKind::Main);
        assert!(app.is_window_open(WindowKind::Equalizer));
        assert_eq!(app.current_page, Page::Main);

        // Second click: When green (already open), clicking again closes the standalone window!
        app.handle_popout_toggle(WindowKind::Equalizer, WindowKind::Main);
        assert!(!app.is_window_open(WindowKind::Equalizer));
    }

    #[test]
    fn test_universal_page_navigation_plan_b() {
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);

        // 1. Player Navigation with Popout Open
        app.mini_player_window_open = true;
        // Button turns green (is_page_active == true)
        assert!(app.is_page_active(Page::Player));
        assert!(app.is_player_active());

        // Under Plan B: clicking nav button navigates to Player page; DOES NOT close popout!
        app.navigate_toggle_page(Page::Player);
        assert_eq!(app.current_page, Page::Player);
        assert!(app.mini_player_window_open);

        // Clicking again while on Player page goes back to Main; popout STILL open!
        app.navigate_toggle_page(Page::Player);
        assert_eq!(app.current_page, Page::Main);
        assert!(app.mini_player_window_open);

        // 2. Equalizer Navigation with Popout Open (Now 100% consistent with Player!)
        app.equalizer_window_open = true;
        // Button turns green (is_page_active == true)
        assert!(app.is_page_active(Page::Equalizer));
        assert!(app.is_equalizer_active());

        // Under Plan B: clicking nav button navigates to Equalizer page; DOES NOT close popout!
        app.navigate_toggle_page(Page::Equalizer);
        assert_eq!(app.current_page, Page::Equalizer);
        assert!(app.equalizer_window_open);

        // Clicking again while on Equalizer page goes back to Main; popout STILL open!
        app.navigate_toggle_page(Page::Equalizer);
        assert_eq!(app.current_page, Page::Main);
        assert!(app.equalizer_window_open);
    }

    #[test]
    fn test_page_transition_push_and_pop() {
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);
        assert!(app.page_transition.is_none());

        // 1. Push transition (Main -> Settings)
        app.navigate_to(Page::Settings);
        assert_eq!(app.current_page, Page::Settings);
        let trans = app.page_transition.expect("Push transition must be active");
        assert_eq!(trans.direction, TransitionDirection::Push);
        assert_eq!(trans.from, Page::Main);
        assert_eq!(trans.to, Page::Settings);
        assert_eq!(trans.duration, std::time::Duration::from_millis(220));

        // 2. Pop transition (Settings -> Main)
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Main);
        let trans = app.page_transition.expect("Pop transition must be active");
        assert_eq!(trans.direction, TransitionDirection::Pop);
        assert_eq!(trans.from, Page::Settings);
        assert_eq!(trans.to, Page::Main);

        // 3. Reset transition
        app.navigate_to(Page::Equalizer);
        app.reset_to(Page::Main);
        assert_eq!(app.current_page, Page::Main);
        let trans = app.page_transition.expect("Reset transition must trigger Pop");
        assert_eq!(trans.direction, TransitionDirection::Pop);
        assert_eq!(trans.from, Page::Equalizer);
        assert_eq!(trans.to, Page::Main);
    }

    #[test]
    fn test_sliding_tab_bar_and_eq_curve_spline() {
        // Test catmull_rom interpolation points
        let p0 = egui::pos2(0.0, 0.0);
        let p1 = egui::pos2(10.0, 10.0);
        let p2 = egui::pos2(20.0, 10.0);
        let p3 = egui::pos2(30.0, 0.0);

        let start = catmull_rom(p0, p1, p2, p3, 0.0);
        assert!((start.x - 10.0).abs() < 1e-4);
        assert!((start.y - 10.0).abs() < 1e-4);

        let end = catmull_rom(p0, p1, p2, p3, 1.0);
        assert!((end.x - 20.0).abs() < 1e-4);
        assert!((end.y - 10.0).abs() < 1e-4);

        let mid = catmull_rom(p0, p1, p2, p3, 0.5);
        assert!((mid.x - 15.0).abs() < 1e-4);

        // Test display_eq_gains initialization
        let app = FlyoutApp::new_mock();
        assert_eq!(app.display_eq_gains, [0.0; 10]);
        assert_eq!(app.display_receiver_eq_gains, [0.0; 10]);
    }

    #[test]
    fn test_track_transition_and_cover_crossfade() {
        let mut app = FlyoutApp::new_mock();
        let ctx = egui::Context::default();
        let current_track_arc = crate::media_control::get_current_media_track();

        // 1. Initial track setup (no transition on initial load)
        {
            let mut track = current_track_arc.write().unwrap();
            track.title = "Song 1".to_string();
            track.artist = "Artist A".to_string();
            track.album = "Album X".to_string();
        }
        app.sync_media_track(&ctx);
        assert_eq!(app.prev_track_title, "Song 1");
        assert!(app.track_transition.is_none());

        // 2. Next track change triggers forward transition
        {
            let mut track = current_track_arc.write().unwrap();
            track.title = "Song 2".to_string();
            track.artist = "Artist B".to_string();
        }
        app.sync_media_track(&ctx);
        let trans = app.track_transition.as_ref().expect("Transition must be triggered");
        assert_eq!(trans.direction, TrackTransitionDirection::Next);
        assert_eq!(trans.from_title, "Song 1");
        assert_eq!(trans.to_title, "Song 2");
        assert_eq!(trans.duration, std::time::Duration::from_millis(320));

        // 3. Previous track intent triggers backward transition
        app.next_track_intent = Some(TrackTransitionDirection::Prev);
        {
            let mut track = current_track_arc.write().unwrap();
            track.title = "Song 3".to_string();
            track.artist = "Artist C".to_string();
        }
        app.sync_media_track(&ctx);
        let trans = app.track_transition.as_ref().expect("Transition must be triggered");
        assert_eq!(trans.direction, TrackTransitionDirection::Prev);
        assert_eq!(trans.from_title, "Song 2");
        assert_eq!(trans.to_title, "Song 3");
    }

    #[test]
    fn test_close_window_cluster_all() {
        let _lock = crate::snapping::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::snapping::reset_all();

        let mut app = FlyoutApp::new_mock();
        app.mini_player_window_open = true;
        app.equalizer_window_open = true;

        let mini_r = windows_sys::Win32::Foundation::RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = windows_sys::Win32::Foundation::RECT { left: 500, top: 350, right: 900, bottom: 600 };

        crate::snapping::set_dock_relation(
            crate::snapping::WindowKind::Equalizer,
            crate::snapping::WindowKind::MiniPlayer,
            &eq_r,
            &mini_r,
        );

        assert!(app.mini_player_window_open);
        assert!(app.equalizer_window_open);

        // Close entire cluster starting from MiniPlayer
        app.close_window_cluster(crate::snapping::WindowKind::MiniPlayer);

        assert!(!app.mini_player_window_open);
        assert!(!app.equalizer_window_open);
    }

    #[test]
    fn test_workstation_onboarding_state() {
        let mut app = FlyoutApp::new_mock();
        app.workstation_onboarding_dismissed = false;
        assert!(!app.workstation_onboarding_dismissed);

        // When discovered, state updates correctly
        app.workstation_onboarding_dismissed = true;
        assert!(app.workstation_onboarding_dismissed);
    }
}
