//! Modern AirFlash-style Flyout GUI window built with egui & eframe.

use std::sync::{Arc, RwLock};
use async_channel::Sender as AsyncSender;
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};


use crate::controller::{
    AudioSource, ControllerState, DeviceItem, DeviceStatus, EqPreset, GuiCmd, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};
use crate::i18n::{Language, ALL_LANGUAGES};
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
pub enum Page {
    #[default]
    Main,
    Settings,
    About,
    Equalizer,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Audio,
    System,
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
    pub settings_tab: SettingsTab,
    pub meter_level: f32,
    pub last_meter_update: std::time::Instant,
    pub startup_time: std::time::Instant,
    pub startup_trimmed: bool,
    pub frame_count: u32,
    pub last_hidden: bool,
    pub spectrum_vis: SpectrumVisualizer,
    pub selected_preset: Option<EqPreset>,
    pub album_art_texture: Option<egui::TextureHandle>,
    pub last_album_art_bytes: Option<Vec<u8>>,
    pub balance: f32,
    pub is_mono: bool,
    pub seek_drag_secs: Option<f64>,
    pub is_always_on_top: bool,
}

static EGUI_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

pub fn register_egui_ctx(ctx: egui::Context) {
    let _ = EGUI_CTX.set(ctx);
}

pub fn get_egui_ctx() -> Option<egui::Context> {
    EGUI_CTX.get().cloned()
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
        let initial_preset = EqPreset::matches_gains(&initial_state.eq_gains);
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
            settings_tab: SettingsTab::default(),
            meter_level: 0.0,
            last_meter_update: std::time::Instant::now(),
            startup_time: std::time::Instant::now(),
            startup_trimmed: false,
            frame_count: 0,
            last_hidden: start_minimized,
            spectrum_vis: SpectrumVisualizer::default(),
            selected_preset: initial_preset,
            album_art_texture: None,
            last_album_art_bytes: None,
            balance: 0.0,
            is_mono: false,
            seek_drag_secs: None,
            is_always_on_top: initial_always_on_top,
        }
    }

    fn sync_state(&mut self) {
        let global_top = crate::tray::IS_ALWAYS_ON_TOP.load(std::sync::atomic::Ordering::SeqCst);
        if self.is_always_on_top != global_top {
            self.is_always_on_top = global_top;
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_state();

        let hwnd = crate::tray::get_main_hwnd();
        let is_hidden = crate::tray::is_window_hidden_to_tray();

        // When window transitions to hidden in tray, trim working set
        if !self.last_hidden && is_hidden {
            crate::tray::trim_working_set();
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

        // Intercept window close (X button) or Escape key: hide to system tray instead of exiting
        let close_req = ctx.input(|i| i.viewport().close_requested());
        let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if escape_pressed && self.current_page == Page::About {
            self.current_page = Page::Settings;
        } else if escape_pressed && (self.current_page == Page::Settings || self.current_page == Page::Equalizer || self.current_page == Page::Player) {
            self.current_page = Page::Main;
        } else if close_req || escape_pressed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            tracing::info!(
                "FlyoutApp: Hiding window to system tray: close_req={}, escape={}",
                close_req, escape_pressed
            );
            if !hwnd.is_null() {
                crate::tray::hide_window_to_tray(hwnd, ctx);
            }
            return;
        }

        if is_hidden {
            // When hidden in system tray:
            // 1. Completely skip all UI widget rendering, layout, and spectrum calculations.
            // 2. Request a 1-second background refresh to keep state fresh.
            // 3. Thread stays paced by Windows DWM VSync, completely eliminating the 5% CPU busy loop!
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
            return;
        }

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
        let repaint_delay = if is_audio_animating {
            std::time::Duration::from_millis(33)
        } else {
            std::time::Duration::from_millis(500)
        };
        ctx.request_repaint_after(repaint_delay);

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
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(lang.top_hint_minimize())
                            .weak()
                            .size(11.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new(lang.btn_quit_app())
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(230, 100, 100)),
                            )
                            .clicked()
                        {
                            let _ = self.cmd_tx.try_send(GuiCmd::Quit);
                            std::thread::spawn(|| {
                                std::thread::sleep(std::time::Duration::from_millis(2500));
                                std::process::exit(0);
                            });
                        }
                    });
                });
            });

        let mut total_content_height = 0.0f32;

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 25, 29)).inner_margin(16.0))
            .show(ctx, |ui| {
                let initial_top = ui.cursor().top();
                let lang = self.state.language;

                // 軟體資訊頁面：第一行（返回按鈕與標題）固定在頂部，僅內容區域捲動
                if self.current_page == Page::About {
                    total_content_height = self.render_about_page(ui);
                    return;
                }

                if self.current_page == Page::Equalizer {
                    total_content_height = self.render_equalizer_page(ui);
                    return;
                }

                if self.current_page == Page::Player {
                    total_content_height = self.render_player_page(ui);
                    return;
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.current_page == Page::Settings {
                            self.render_settings_page(ui);
                            total_content_height = ui.cursor().top() - initial_top;
                            return;
                        }

                        // Header bar
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("Win2Pod Bridge").strong().size(18.0));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                self.render_pin_button(ui);
                                self.render_visualizer_button(ui);
                                if ui
                                    .button(egui::RichText::new("♫").size(13.0))
                                    .on_hover_text(lang.tooltip_player())
                                    .clicked()
                                {
                                    self.current_page = Page::Player;
                                }
                                if ui
                                    .button(egui::RichText::new("⚙").size(13.0))
                                    .on_hover_text(lang.tooltip_settings())
                                    .clicked()
                                {
                                    self.current_page = Page::Settings;
                                }
                            });
                        });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Notice Banner (e.g. preemption notice)
                if let Some((ref notice, time)) = self.state.notice_message {
                    if time.elapsed().as_secs() < 60 {
                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::from_rgb(38, 30, 16))
                            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(215, 145, 35)))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("ℹ").size(15.0));
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(lang.preempted_banner_title())
                                                .color(egui::Color32::from_rgb(250, 185, 70))
                                                .strong()
                                                .size(12.5),
                                        );
                                        ui.label(
                                            egui::RichText::new(notice.render(lang))
                                                .color(egui::Color32::from_rgb(220, 220, 225))
                                                .size(11.5),
                                        );
                                    });
                                });
                            });
                        ui.add_space(10.0);
                    }
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

                ui.add_space(10.0);

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
                ui.add_space(4.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(8.0)
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
                                    } else if dev.is_stereo_pair {
                                        egui::RichText::new("🔊").color(egui::Color32::from_rgb(100, 200, 255))
                                    } else {
                                        egui::RichText::new("○").weak()
                                    };
                                    ui.label(icon);

                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&dev.name).strong());
                                            if dev.is_stereo_pair {
                                                ui.label(
                                                    egui::RichText::new(lang.stereo_pair_tag())
                                                        .color(egui::Color32::from_rgb(100, 200, 255))
                                                        .size(10.0),
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

                ui.add_space(10.0);

                // --- 2.5. Available AirPods List ---
                ui.label(egui::RichText::new(lang.available_airpods_header()).strong().size(13.0));
                ui.add_space(4.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(32, 34, 40))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(8.0)
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

                ui.add_space(10.0);

                // --- 3. Volume & Mute Controls ---
                ui.label(egui::RichText::new(lang.volume_header()).strong().size(13.0));
                ui.add_space(4.0);

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
                            .on_hover_text(lang.tooltip_player())
                            .clicked()
                        {
                            self.current_page = Page::Player;
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
                total_content_height = ui.cursor().top() - initial_top;
            });
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

impl FlyoutApp {
    fn render_pin_button(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;
        let (tooltip, text_color, fill_color, stroke_color) = if self.is_always_on_top {
            (
                lang.tooltip_unpin_window(),
                egui::Color32::from_rgb(255, 215, 0),     // Vibrant Gold
                egui::Color32::from_rgb(55, 45, 18),     // Subtle golden glow fill
                egui::Color32::from_rgb(215, 165, 40),    // Gold border
            )
        } else {
            (
                lang.tooltip_pin_window(),
                egui::Color32::from_gray(160),            // Subdued clean gray
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            )
        };

        let btn = egui::Button::new(egui::RichText::new("📌").size(13.0).color(text_color))
            .fill(fill_color)
            .stroke(egui::Stroke::new(1.0_f32, stroke_color))
            .rounding(egui::Rounding::same(4.0));

        if ui.add(btn).on_hover_text(tooltip).clicked() {
            self.is_always_on_top = !self.is_always_on_top;
            crate::tray::IS_ALWAYS_ON_TOP.store(self.is_always_on_top, std::sync::atomic::Ordering::SeqCst);
            crate::controller::save_always_on_top(self.is_always_on_top);
            let level = if self.is_always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));

            // Direct Win32 SetWindowPos enforcement for maximum reliability
            let hwnd = crate::tray::get_main_hwnd();
            if !hwnd.is_null() {
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
                    };
                    let target = if self.is_always_on_top { HWND_TOPMOST } else { HWND_NOTOPMOST };
                    SetWindowPos(hwnd, target, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                }
            }

            // Propagate pin state to magnetically docked visualizer
            let bond = crate::snapping::get_dock_bond();
            if bond.is_bonded {
                crate::visualizer::set_visualizer_pinned(self.is_always_on_top);
            }
        }
    }

    fn render_visualizer_button(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;
        let is_open = crate::visualizer::is_visualizer_open();
        let (tooltip, text_color, fill_color, stroke_color) = if is_open {
            (
                lang.tooltip_visualizer_close(),
                egui::Color32::from_rgb(100, 220, 255),
                egui::Color32::from_rgb(20, 45, 65),
                egui::Color32::from_rgb(60, 160, 220),
            )
        } else {
            (
                lang.tooltip_visualizer(),
                egui::Color32::from_gray(160),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 28),
            )
        };

        let btn = egui::Button::new(egui::RichText::new("🎨").size(13.0).color(text_color))
            .fill(fill_color)
            .stroke(egui::Stroke::new(1.0_f32, stroke_color))
            .rounding(egui::Rounding::same(4.0));

        if ui.add(btn).on_hover_text(tooltip).clicked() {
            crate::visualizer::set_language(lang.code());
            crate::visualizer::toggle_visualizer();
        }
    }

    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let lang = self.state.language;

        // Header bar with Back button
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new(lang.btn_back()).size(12.5)).clicked() {
                self.current_page = Page::Main;
            }
            ui.add_space(4.0);
            ui.heading(egui::RichText::new(lang.advanced_settings_header()).strong().size(17.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.render_pin_button(ui);
                self.render_visualizer_button(ui);
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        // Segmented Tabs Header
        ui.horizontal(|ui| {
            let is_audio = self.settings_tab == SettingsTab::Audio;
            let is_system = self.settings_tab == SettingsTab::System;
            let tab_w = ((ui.available_width() - 8.0) / 2.0).max(120.0);

            if ui.add_sized([tab_w, 26.0], egui::SelectableLabel::new(is_audio, egui::RichText::new(lang.settings_tab_audio()).size(12.5).strong())).clicked() {
                self.settings_tab = SettingsTab::Audio;
            }
            if ui.add_sized([tab_w, 26.0], egui::SelectableLabel::new(is_system, egui::RichText::new(lang.settings_tab_system()).size(12.5).strong())).clicked() {
                self.settings_tab = SettingsTab::System;
            }
        });

        ui.add_space(8.0);

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
                let cb = ui.checkbox(
                    &mut prevent_sleep,
                    egui::RichText::new(lang.prevent_sleep_checkbox()).size(12.0),
                );
                if cb.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetPreventSleep(prevent_sleep));
                }
                cb.on_hover_text(lang.prevent_sleep_tooltip());

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
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(format!("ℹ  {}", lang.btn_about())).size(12.5)).on_hover_text(lang.tooltip_about()).clicked() {
                        self.current_page = Page::About;
                    }
                });
                ui.add_space(8.0);
            }
        }
    }

    fn render_about_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let lang = self.state.language;
        let start_top = ui.cursor().top();

        // 頂部固定列：返回按鈕與標題（固定不捲動）
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new(lang.btn_back()).size(12.5)).clicked() {
                self.current_page = Page::Settings;
            }
            ui.add_space(4.0);
            ui.heading(egui::RichText::new(lang.about_title()).size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.render_pin_button(ui);
                self.render_visualizer_button(ui);
            });
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let header_h = ui.cursor().top() - start_top;

        let card_color = if ui.visuals().dark_mode {
            egui::Color32::from_rgb(32, 36, 44)
        } else {
            egui::Color32::from_rgb(245, 247, 250)
        };

        let scroll_output = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
                            ui.label(egui::RichText::new("GNU General Public License v3.0 or later (GPL-3.0-or-later)").size(11.5));
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

                        let packages: [(&str, &str, &str, &str); 32] = [
                            ("airplay2-rs", "Lucas McGartland", "GPL-3.0-or-later", lang.pkg_airplay2_rs_desc()),
                            ("windows-airplay-homepod", "Yang Liu (iakacer)", "GPL-2.0", lang.pkg_windows_airplay_desc()),
                            ("AirPodsDesktop", "SpriteOvO", "GPL-3.0", lang.pkg_airpodsdesktop_desc()),
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

    pub fn render_player_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let initial_top = ui.cursor().top();
        let lang = self.state.language;

        // 1. Header: Back button & Title
        ui.horizontal(|ui| {
            if ui.button(lang.btn_back()).clicked() {
                self.current_page = Page::Main;
            }
            ui.heading(
                egui::RichText::new("Win2Pod Player")
                    .strong()
                    .size(15.0)
                    .color(egui::Color32::from_rgb(180, 220, 245)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.render_pin_button(ui);
                self.render_visualizer_button(ui);
            });
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        // 2. Query GSMTC Media Track Info
        let track_info = crate::media_control::get_current_media_track().read().unwrap().clone();

        // Texture caching: update if thumbnail bytes changed
        if track_info.thumbnail_bytes != self.last_album_art_bytes {
            self.last_album_art_bytes = track_info.thumbnail_bytes.clone();
            if let Some(ref bytes) = track_info.thumbnail_bytes {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                    self.album_art_texture = Some(ui.ctx().load_texture(
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

        // 3. WinAMP Main Display Chassis (Album Cover + VFD/LCD Screen)
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(22, 24, 29))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(46, 52, 64)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let total_inner_w = ui.available_width();

                ui.horizontal(|ui| {
                    // Friendly app name detection for tooltip
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

                    // Left: Album Art Box (88x88 px) - Clickable to open/activate media player
                    let art_size = egui::vec2(88.0, 88.0);
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

                    let painter = ui.painter();

                    if let Some(ref tex) = self.album_art_texture {
                        // Render loaded album art
                        painter.rect_filled(art_rect, egui::Rounding::same(6.0), egui::Color32::BLACK);
                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        painter.image(tex.id(), art_rect, uv, egui::Color32::WHITE);
                        painter.rect_stroke(
                            art_rect,
                            egui::Rounding::same(6.0),
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40)),
                        );
                    } else {
                        // Retro vinyl record placeholder
                        let center = art_rect.center();
                        let radius = (art_size.x.min(art_size.y) / 2.0) - 2.0;

                        // Outer vinyl disc
                        painter.circle_filled(center, radius, egui::Color32::from_rgb(18, 19, 23));
                        painter.circle_stroke(center, radius, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 54, 65)));

                        // Vinyl groove concentric rings
                        for r_factor in [0.85, 0.72, 0.60, 0.48] {
                            painter.circle_stroke(
                                center,
                                radius * r_factor,
                                egui::Stroke::new(0.5_f32, egui::Color32::from_rgba_unmultiplied(80, 85, 100, 60)),
                            );
                        }

                        // Center label
                        painter.circle_filled(center, radius * 0.36, egui::Color32::from_rgb(35, 75, 130));
                        painter.circle_stroke(center, radius * 0.36, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(80, 140, 220)));

                        // Spindle hole
                        painter.circle_filled(center, 4.0, egui::Color32::from_rgb(220, 225, 230));

                        // Musical note indicator
                        painter.text(
                            center - egui::vec2(0.0, 10.0),
                            egui::Align2::CENTER_CENTER,
                            "♪",
                            egui::FontId::proportional(12.0),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 160),
                        );
                    }

                    // Hover cue on album art: subtle glow and external launch icon
                    if art_resp.hovered() {
                        painter.rect_filled(
                            art_rect,
                            egui::Rounding::same(6.0),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        );
                        painter.text(
                            art_rect.right_top() + egui::vec2(-4.0, 4.0),
                            egui::Align2::RIGHT_TOP,
                            "↗",
                            egui::FontId::proportional(13.0),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 230),
                        );
                    }

                    ui.add_space(8.0);

                    // Right: WinAMP VFD / LCD Screen (exact width matching spectrum below)
                    let vfd_w = ui.available_width();
                    let vfd_h = 88.0f32;
                    let (vfd_rect, _) = ui.allocate_exact_size(egui::vec2(vfd_w, vfd_h), egui::Sense::hover());
                    let vfd_painter = ui.painter();
                    vfd_painter.rect_filled(vfd_rect, egui::Rounding::same(5.0), egui::Color32::from_rgb(8, 16, 10));
                    vfd_painter.rect_stroke(
                        vfd_rect,
                        egui::Rounding::same(5.0),
                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(20, 48, 24)),
                    );

                    let inner_vfd = vfd_rect.shrink2(egui::vec2(8.0, 6.0));
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner_vfd), |ui| {
                        ui.vertical(|ui| {
                            // Track Title (Glowing green marquee style, clickable to bring up media player)
                            let title_text = if track_info.title.trim().is_empty() {
                                lang.player_no_track().to_string()
                            } else {
                                track_info.title.clone()
                            };
                            let title_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(title_text)
                                        .monospace()
                                        .size(12.5)
                                        .strong()
                                        .color(egui::Color32::from_rgb(0, 255, 110)),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(&click_app_tooltip);

                            if title_resp.clicked() {
                                crate::media_control::activate_media_source_app(
                                    &track_info.source_app_id,
                                    &track_info.title,
                                    Some(&self.state.audio_source),
                                );
                            }

                            ui.add_space(2.0);

                            // Artist & Album
                            let artist_text = if track_info.artist.trim().is_empty() {
                                lang.player_unknown_artist().to_string()
                            } else if !track_info.album.trim().is_empty() {
                                format!("{} • {}", track_info.artist, track_info.album)
                            } else {
                                track_info.artist.clone()
                            };
                            ui.label(
                                egui::RichText::new(artist_text)
                                    .monospace()
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(110, 220, 150)),
                            );

                            ui.add_space(3.0);

                            // Stream Specs Badges (Dynamic STEREO / MONO display & AirPlay Connection Indicator & Harmonic Exciter (HE) Badge)
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
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

                                for badge in &[primary_badge_str, "44.1 kHz", "16-BIT", channel_badge, he_label, we_label] {
                                    let is_primary_badge = *badge == primary_badge_str;
                                    let is_active_mono = *badge == "MONO";
                                    let is_he_badge = *badge == he_label;
                                    let is_we_badge = *badge == we_label;
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
                                        .inner_margin(egui::Margin::symmetric(4.0, 1.5))
                                        .show(ui, |ui| {
                                            let resp = ui.label(
                                                egui::RichText::new(*badge)
                                                    .monospace()
                                                    .size(8.5)
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
                            });

                            ui.add_space(2.0);

                            // Playback status pill & Track Position / Duration timer
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
                                        .size(9.5)
                                        .strong()
                                        .color(stat_col),
                                );

                                // Track Position & Duration (responsive to drag preview)
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

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        egui::RichText::new(time_str)
                                            .monospace()
                                            .size(9.5)
                                            .color(egui::Color32::from_rgb(0, 230, 110)),
                                    );
                                });
                            });
                        });
                    });
                });

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
                let (spec_rect, _spec_resp) = ui.allocate_exact_size(
                    egui::vec2(total_inner_w, spec_h),
                    egui::Sense::hover(),
                );
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

                // 10-Band Sliders (WinAMP Classic Vertical Rack)
                let labels = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
                let col_gap = 2.0f32;
                let avail_w = ui.available_width();
                let slider_col_w = ((avail_w - 9.0 * col_gap) / 10.0).floor().max(20.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = col_gap;
                    for band_idx in 0..10 {
                        ui.allocate_ui_with_layout(
                            egui::vec2(slider_col_w, 120.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                let mut gain = self.state.eq_gains[band_idx];
                                let gain_str = if gain > 0.0 {
                                    format!("+{}", gain.round() as i32)
                                } else {
                                    format!("{}", gain.round() as i32)
                                };
                                let text_col = if gain.abs() > 0.5 {
                                    egui::Color32::from_rgb(240, 185, 55)
                                } else {
                                    egui::Color32::from_rgb(120, 125, 135)
                                };
                                ui.label(egui::RichText::new(gain_str).monospace().size(9.0).color(text_col));

                                let slider = egui::Slider::new(&mut gain, -12.0..=12.0)
                                    .vertical()
                                    .show_value(false)
                                    .step_by(0.5);

                                if ui.add(slider).changed() {
                                    self.selected_preset = Some(EqPreset::Custom);
                                    self.state.eq_enabled = true;
                                    self.state.eq_gains[band_idx] = gain;
                                    self.state.custom_eq_gains[band_idx] = gain;
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqGain(band_idx, gain));
                                }

                                ui.label(egui::RichText::new(labels[band_idx]).monospace().size(9.0).weak());
                            },
                        );
                    }
                });
            });

        ui.cursor().top() - initial_top
    }

    pub fn render_equalizer_page(&mut self, ui: &mut egui::Ui) -> f32 {
        let initial_top = ui.cursor().top();
        let lang = self.state.language;

        // Header: Back button & Title
        ui.horizontal(|ui| {
            if ui.button(lang.btn_back()).clicked() {
                self.current_page = Page::Main;
            }
            ui.heading(egui::RichText::new(lang.eq_title()).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.render_pin_button(ui);
                self.render_visualizer_button(ui);
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Enable Checkbox & Reset Button
        ui.horizontal(|ui| {
            let mut enabled = self.state.eq_enabled;
            if ui.checkbox(&mut enabled, egui::RichText::new(lang.eq_enable()).size(13.0).strong()).changed() {
                let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqBypass(!enabled));
            }

            // Auto Pre-Amp headroom protection badge
            let max_boost = self.state.eq_gains.iter().copied().fold(0.0f32, f32::max);
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
                    self.selected_preset = None;
                    self.state.eq_gains = [0.0; 10];
                    let _ = self.cmd_tx.send_blocking(GuiCmd::ResetEq);
                }
            });
        });

        ui.add_space(6.0);

        // Sound Presets
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(lang.eq_presets_label()).size(11.0).weak());
            for &preset in EqPreset::ALL {
                let name = lang.eq_preset_name(preset);
                let is_selected = self.selected_preset == Some(preset);

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

        ui.add_space(8.0);

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

        // Live Real-Time Spectrum Visualizer
        ui.label(egui::RichText::new(lang.eq_spectrum_title()).size(11.5).weak());
        let spec_height = 65.0f32;
        let (spec_rect, _spec_resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), spec_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        painter.rect_filled(
            spec_rect,
            egui::Rounding::same(8.0),
            egui::Color32::from_rgb(18, 19, 22),
        );
        painter.rect_stroke(
            spec_rect,
            egui::Rounding::same(8.0),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
        );

        let inner_spec = spec_rect.shrink(6.0);
        let num_bars = 16;
        let gap = 3.0f32;
        let bar_w = ((inner_spec.width() - (num_bars as f32 - 1.0) * gap) / num_bars as f32).max(2.0);

        for i in 0..num_bars {
            let x = inner_spec.min.x + i as f32 * (bar_w + gap);
            let level = self.spectrum_vis.current[i].clamp(0.0, 1.0);
            let bar_h = (inner_spec.height() * level).max(1.5);
            let bar_y = inner_spec.max.y - bar_h;

            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(x, bar_y),
                egui::vec2(bar_w, bar_h),
            );
            let frac = i as f32 / (num_bars as f32 - 1.0);
            let col = meter_color_at(frac);
            painter.rect_filled(bar_rect, egui::Rounding::same(2.0), col);

            // Peak cap
            let peak_lvl = self.spectrum_vis.peaks[i].clamp(0.0, 1.0);
            if peak_lvl > 0.05 {
                let peak_y = inner_spec.max.y - (inner_spec.height() * peak_lvl);
                let cap_rect = egui::Rect::from_min_size(
                    egui::pos2(x, peak_y.max(inner_spec.min.y)),
                    egui::vec2(bar_w, 2.0),
                );
                painter.rect_filled(cap_rect, egui::Rounding::same(1.0), egui::Color32::from_rgb(255, 255, 255));
            }
        }

        ui.add_space(12.0);

        // 10-Band Equalizer Sliders
        let labels = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
        let col_gap = 2.0f32;
        let avail_w = ui.available_width();
        let slider_col_w = ((avail_w - 9.0 * col_gap) / 10.0).floor().max(20.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = col_gap;
            for band_idx in 0..10 {
                ui.allocate_ui_with_layout(
                    egui::vec2(slider_col_w, 150.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let mut gain = self.state.eq_gains[band_idx];
                        let gain_str = if gain > 0.0 {
                            format!("+{}", gain.round() as i32)
                        } else {
                            format!("{}", gain.round() as i32)
                        };
                        let text_col = if gain.abs() > 0.5 {
                            egui::Color32::from_rgb(240, 180, 50)
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.label(egui::RichText::new(gain_str).size(10.0).color(text_col));

                        let slider = egui::Slider::new(&mut gain, -12.0..=12.0)
                            .vertical()
                            .show_value(false)
                            .step_by(0.5);

                        if ui.add(slider).changed() {
                            self.selected_preset = Some(EqPreset::Custom);
                            self.state.eq_enabled = true;
                            self.state.eq_gains[band_idx] = gain;
                            self.state.custom_eq_gains[band_idx] = gain;
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetEqGain(band_idx, gain));
                        }

                        ui.label(egui::RichText::new(labels[band_idx]).size(9.5).weak());
                    },
                );
            }
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

    ctx.set_fonts(fonts);
}
