//! Modern AirFlash-style Flyout GUI window built with egui & eframe.

use std::sync::{Arc, RwLock};
use async_channel::Sender as AsyncSender;
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::controller::{
    ControllerState, DeviceStatus, EqPreset, GuiCmd,
};
use crate::i18n::Language;
use crate::tray::set_main_hwnd;

pub mod types;
pub mod components;
pub mod views;
#[cfg(test)]
mod tests;

pub use types::*;
pub(crate) use components::*;
pub use views::*;

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
            const ICON_BYTES: &[u8] = include_bytes!("../../assets/app_icon.png");
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

        let main_hwnd = crate::snapping::get_window_hwnd(crate::snapping::WindowKind::Main);
        let is_main_iconic = !main_hwnd.is_null() && unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(main_hwnd) != 0 };
        let is_main_cluster_min = matches!(
            crate::snapping::get_cluster_state(crate::snapping::WindowKind::Main),
            crate::snapping::ClusterState::Minimized | crate::snapping::ClusterState::Minimizing(_)
        );

        if is_main_hidden || is_fleet_hidden || is_main_iconic || is_main_cluster_min {
            // Main window itself is hidden in system tray, or minimized to taskbar:
            // secondary viewports were already rendered above with live spectrum and dynamic framerate.
            // Skip main window UI widget rendering (footer, central panels) to prevent squashing egui layout into 160x28!
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

        // On initial startup (first frame) or if the window dimensions ever collapse abnormally (< 360x320),
        // enforce the standardized base dimensions to permanently prevent squashed window states.
        let cur_screen = ctx.screen_rect();
        let cur_w = cur_screen.width();
        let cur_h = cur_screen.height();
        if self.frame_count < 2 || cur_w < 360.0 || cur_h < 320.0 {
            self.last_applied_width = base_width;
            self.last_applied_height = base_height;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(base_width, base_height)));
        }
    }
}

/// 繪製頁面推拉過場時接縫邊緣的柔和漸變陰影

impl FlyoutApp {
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
        if self.equalizer_window_open && self.mini_player_window_open {
            let main_hwnd = crate::snapping::get_window_hwnd(crate::snapping::WindowKind::Main);
            if !main_hwnd.is_null() && unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(main_hwnd) != 0
                    && windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(main_hwnd) == 0
            } {
                let mut mr: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
                unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(main_hwnd, &mut mr); }
                let mh = (mr.bottom - mr.top) as f32 / ppp;
                if mh >= 600.0 {
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

        let root = crate::snapping::get_component_root(kind);
        let is_cluster_min = matches!(
            crate::snapping::get_cluster_state(root),
            crate::snapping::ClusterState::Minimized | crate::snapping::ClusterState::Minimizing(_)
        );
        let is_self_min = matches!(
            crate::snapping::get_cluster_state(kind),
            crate::snapping::ClusterState::Minimized | crate::snapping::ClusterState::Minimizing(_)
        );
        let is_minimized = is_cluster_min || is_self_min;
        let should_be_visible = !is_fleet_hidden && !is_minimized;

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
            builder = builder.with_visible(should_be_visible);
        }

        ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
            if is_fleet_hidden || is_minimized {
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
}

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
pub(crate) fn setup_custom_fonts(ctx: &egui::Context, lang: Language) {
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


