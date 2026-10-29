//! Modern AirFlash-style Flyout GUI window built with egui & eframe.

use std::sync::{Arc, RwLock};
use async_channel::Sender as AsyncSender;
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindowVisible, ShowWindow, SW_HIDE};

use crate::controller::{
    ControllerState, DeviceItem, DeviceStatus, GuiCmd, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
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
}

pub struct FlyoutApp {
    pub cmd_tx: AsyncSender<GuiCmd>,
    pub shared_state: Arc<RwLock<ControllerState>>,
    pub state: ControllerState,
    pub volume_slider: f32,
    pub last_applied_height: f32,
    pub selected_device_ids: std::collections::HashSet<String>,
    pub is_dragging_volume: bool,
    pub current_page: Page,
}

static EGUI_CTX: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

pub fn register_egui_ctx(ctx: egui::Context) {
    let _ = EGUI_CTX.set(ctx);
}

pub fn request_ui_repaint() {
    if let Some(ctx) = EGUI_CTX.get() {
        ctx.request_repaint();
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
        setup_custom_fonts(&cc.egui_ctx, initial_state.language);
        register_egui_ctx(cc.egui_ctx.clone());

        if start_minimized {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // Store Win32 HWND so the background tray thread can reliably show/focus this window
        if let Ok(handle) = cc.window_handle() {
            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = win32_handle.hwnd.get() as isize;
                set_main_hwnd(hwnd);
                if start_minimized {
                    unsafe {
                        ShowWindow(hwnd as _, SW_HIDE);
                    }
                    crate::tray::trim_working_set();
                }
            }
        }

        Self {
            cmd_tx,
            shared_state,
            state: initial_state,
            volume_slider: vol,
            is_dragging_volume: false,
            last_applied_height: 0.0,
            selected_device_ids: std::collections::HashSet::new(),
            current_page: Page::Main,
        }
    }

    fn sync_state(&mut self) {
        if let Ok(latest) = self.shared_state.read() {
            // If user isn't dragging with mouse, sync volume immediately
            if !self.is_dragging_volume {
                self.volume_slider = latest.volume;
            }
            self.state = latest.clone();
        }
    }
}

impl eframe::App for FlyoutApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_state();

        let hwnd = crate::tray::get_main_hwnd();
        let is_visible = !hwnd.is_null() && unsafe { IsWindowVisible(hwnd) != 0 };

        // Intercept window close (X button) or Escape key: minimize/hide to system tray instead of exiting
        let close_req = ctx.input(|i| i.viewport().close_requested());
        let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if escape_pressed && self.current_page == Page::About {
            self.current_page = Page::Settings;
        } else if escape_pressed && self.current_page == Page::Settings {
            self.current_page = Page::Main;
        } else if close_req || escape_pressed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if crate::tray::was_just_shown(500) {
                tracing::info!(
                    "FlyoutApp: Ignoring close/escape event (close_req={}, escape={}) because window was just shown within 500ms",
                    close_req, escape_pressed
                );
            } else {
                tracing::info!(
                    "FlyoutApp: Hiding window to system tray: close_req={}, escape={}",
                    close_req, escape_pressed
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                if !hwnd.is_null() {
                    unsafe {
                        ShowWindow(hwnd, SW_HIDE);
                    }
                }
                self.current_page = Page::Main;
                // Release the ~100MB OpenGL GPU driver context once when hiding to tray
                crate::tray::trim_working_set();
            }
        }

        // When visible, repaint smoothly (250ms). When hidden in tray, completely skip rendering
        // to save CPU/battery/RAM and prevent OpenGL context recreation.
        if is_visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        } else {
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
                            crate::controller::restore_original_speaker_mute();
                            let _ = self.cmd_tx.send_blocking(GuiCmd::Quit);
                            std::process::exit(0);
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
                                if ui
                                    .button(egui::RichText::new("⚙").size(13.0))
                                    .on_hover_text(lang.tooltip_settings())
                                    .clicked()
                                {
                                    self.current_page = Page::Settings;
                                }
                                if ui.button(lang.btn_refresh()).on_hover_text(lang.tooltip_refresh()).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshDevices);
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
                                    ui.label(egui::RichText::new("ℹ️").size(15.0));
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
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(240, 160, 40), "●");
                            let name = self.state.connected_name.as_deref().unwrap_or("Audio");
                            ui.label(
                                egui::RichText::new(lang.status_connecting(name))
                                    .color(egui::Color32::from_rgb(240, 160, 40))
                                    .strong(),
                            );
                        });
                    }
                    DeviceStatus::Streaming => {
                        let name = self.state.connected_name.as_deref().unwrap_or("Audio");
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(70, 200, 120), "●");
                            ui.label(
                                egui::RichText::new(lang.status_streaming(name))
                                    .color(egui::Color32::from_rgb(70, 200, 120))
                                    .strong(),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new(lang.btn_disconnect()).color(egui::Color32::LIGHT_RED)).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::Disconnect);
                                }
                            });
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
                                ui.label(egui::RichText::new(lang.no_devices_found()).weak());
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
                                                if remaining.is_empty() {
                                                    let _ = self.cmd_tx.send_blocking(GuiCmd::Disconnect);
                                                } else {
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

                // --- 3. Volume & Mute Controls ---
                ui.label(egui::RichText::new(lang.volume_header()).strong().size(13.0));
                ui.add_space(4.0);

                // Mute Status Display & One-Click Mute Button
                ui.horizontal(|ui| {
                    let label_text = match (&self.state.status, self.state.is_muted) {
                        (DeviceStatus::Streaming, true) => {
                            let name = self.state.connected_name.as_deref().unwrap_or("Audio");
                            egui::RichText::new(lang.vol_speaker_muted(name))
                                .color(egui::Color32::from_rgb(240, 160, 40))
                                .strong()
                        }
                        (DeviceStatus::Streaming, false) => {
                            let name = self.state.connected_name.as_deref().unwrap_or("Audio");
                            egui::RichText::new(lang.vol_speaker_playing(name))
                                .color(egui::Color32::from_rgb(70, 200, 120))
                                .strong()
                        }
                        (DeviceStatus::Connecting, true) => {
                            egui::RichText::new(lang.vol_connecting_muted())
                                .color(egui::Color32::from_rgb(240, 160, 40))
                        }
                        (DeviceStatus::Connecting, false) => {
                            egui::RichText::new(lang.vol_connecting())
                                .color(egui::Color32::from_rgb(240, 160, 40))
                        }
                        (DeviceStatus::Idle, true) => {
                            egui::RichText::new(lang.vol_idle_muted())
                                .color(egui::Color32::from_rgb(200, 150, 60))
                        }
                        (DeviceStatus::Idle, false) => {
                            egui::RichText::new(lang.vol_idle())
                                .color(egui::Color32::LIGHT_GRAY)
                        }
                        (DeviceStatus::Error(_), true) => {
                            egui::RichText::new(lang.vol_error_muted())
                                .color(egui::Color32::from_rgb(240, 80, 80))
                        }
                        (DeviceStatus::Error(_), false) => {
                            egui::RichText::new(lang.vol_error())
                                .color(egui::Color32::from_rgb(240, 80, 80))
                        }
                    };

                    ui.label(label_text);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (btn_text, btn_color) = if self.state.is_muted {
                            (lang.btn_unmute(), egui::Color32::from_rgb(70, 200, 120))
                        } else {
                            (lang.btn_mute(), egui::Color32::from_rgb(240, 160, 40))
                        };
                        if ui.button(egui::RichText::new(btn_text).color(btn_color)).clicked() {
                            let _ = self.cmd_tx.send_blocking(GuiCmd::SetMute(!self.state.is_muted));
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
                    } else if self.state.status != DeviceStatus::Streaming {
                        format!("{vol_percent}%{}", lang.slider_default_suffix())
                    } else {
                        format!("{vol_percent}%")
                    };

                    ui.spacing_mut().slider_width = 220.0;
                    let slider = egui::Slider::new(&mut self.volume_slider, 0.0..=1.0)
                        .show_value(false)
                        .text(slider_text);

                    let slider_resp = ui.add(slider);
                    if slider_resp.drag_started() {
                        self.is_dragging_volume = true;
                    }
                    if slider_resp.drag_stopped() {
                        self.is_dragging_volume = false;
                    }
                    if slider_resp.changed() {
                        let _ = self.cmd_tx.send_blocking(GuiCmd::SetVolume(self.volume_slider));
                    }
                });

                ui.add_space(8.0);
                total_content_height = ui.cursor().top() - initial_top;
            });
        });

        // Dynamic auto-adjustment: window automatically resizes to fit all controls without manual dragging!
        let min_h = match self.current_page {
            Page::Settings => 440.0,
            Page::About => 480.0,
            Page::Main => 340.0,
        };
        let target_height = (total_content_height + 84.0).ceil().clamp(min_h, 750.0);
        if (self.last_applied_height - target_height).abs() > 6.0 {
            self.last_applied_height = target_height;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(420.0, target_height)));
        }
    }
}

impl FlyoutApp {
    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        let lang = self.state.language;

        // Header bar with Back button
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new(lang.btn_back()).size(12.5)).clicked() {
                self.current_page = Page::Main;
            }
            ui.add_space(4.0);
            ui.heading(egui::RichText::new(lang.advanced_settings_header()).strong().size(17.0));
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // --- Group 1: Audio Source & Output ---
        ui.label(egui::RichText::new(lang.settings_section_audio()).strong().size(13.0));
        ui.add_space(4.0);

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(32, 34, 40))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(10.0)
            .show(ui, |ui| {
                // Audio Source Device Selector
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(lang.audio_source_label()).size(12.0));
                    let current_label = self
                        .state
                        .audio_devices
                        .iter()
                        .find(|d| d.id == self.state.selected_audio_device)
                        .map(|d| if d.id == "default" { lang.audio_source_default() } else { d.name.as_str() })
                        .unwrap_or_else(|| lang.audio_source_default());

                    egui::ComboBox::from_id_salt("audio_source_combo")
                        .selected_text(current_label)
                        .show_ui(ui, |ui| {
                            for dev in &self.state.audio_devices {
                                let is_selected = self.state.selected_audio_device == dev.id;
                                let dev_name = if dev.id == "default" {
                                    lang.audio_source_default()
                                } else {
                                    dev.name.as_str()
                                };
                                if ui.selectable_label(is_selected, dev_name).clicked() {
                                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAudioDevice(dev.id.clone()));
                                }
                            }
                        });

                    if ui.button(" ⟳ ").on_hover_text(lang.tooltip_rescan_audio()).clicked() {
                        let _ = self.cmd_tx.send_blocking(GuiCmd::RefreshAudioDevices);
                    }
                });

                let is_system_mix = self.state.selected_audio_device == "default";

                if !is_system_mix {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 255), lang.split_mode_label());
                        ui.label(
                            egui::RichText::new(lang.split_mode_desc())
                                .size(11.0)
                                .weak(),
                        );
                    });
                    ui.add_space(2.0);
                    if ui.button(egui::RichText::new(lang.btn_open_windows_mixer()).size(11.0)).clicked() {
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "ms-settings:apps-volume"])
                            .spawn();
                    }
                }

                ui.add_space(6.0);
                // Auto mute PC speakers: only applies when using full system mix
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
                    } else if is_system_mix {
                        let mut auto_mute = self.state.auto_mute_speakers;
                        ui.horizontal(|ui| {
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
            });

        ui.add_space(10.0);

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
                let resp = ui.checkbox(
                    &mut low_latency,
                    egui::RichText::new(lang.low_latency_checkbox()).size(12.0),
                );
                if resp.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetLowLatency(low_latency));
                }
                resp.on_hover_text(lang.low_latency_tooltip());

                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
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
                let resp = ui.checkbox(
                    &mut prevent_sleep,
                    egui::RichText::new(lang.prevent_sleep_checkbox()).size(12.0),
                );
                if resp.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetPreventSleep(prevent_sleep));
                }
                resp.on_hover_text(lang.prevent_sleep_tooltip());

                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    let hint_text = if prevent_sleep {
                        if self.state.status == DeviceStatus::Streaming {
                            if self.state.is_audio_active {
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
                let resp = ui.checkbox(
                    &mut auto_connect,
                    egui::RichText::new(lang.auto_connect_checkbox()).size(12.0),
                );
                if resp.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAutoConnect(auto_connect));
                }
                resp.on_hover_text(lang.auto_connect_tooltip());

                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    let hint = if auto_connect {
                        if let Some(ref name) = self.state.last_connected_name {
                            lang.auto_connect_target(name)
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
                let resp = ui.checkbox(
                    &mut autostart,
                    egui::RichText::new(lang.autostart_checkbox()).size(12.0),
                );
                if resp.changed() {
                    let _ = self.cmd_tx.send_blocking(GuiCmd::SetAutostart(autostart));
                }
                resp.on_hover_text(lang.autostart_tooltip());

                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
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

                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
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
                                    setup_custom_fonts(ui.ctx(), l);
                                }
                            }
                        });
                });
            });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new(format!("ℹ  {}", lang.btn_about())).size(12.5)).clicked() {
                self.current_page = Page::About;
            }
        });
        ui.add_space(8.0);
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
                            ui.label(egui::RichText::new("GNU General Public License v2.0 (GPL-2.0)").size(11.5));
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

                // 2. Open Source Packages & Dependencies
                egui::Frame::none()
                    .fill(card_color)
                    .rounding(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(lang.about_packages_header()).size(13.0).strong());
                        ui.add_space(6.0);

                        let packages: [(&str, &str, &str, &str); 19] = [
                            ("egui & eframe", "v0.29", "MIT / Apache-2.0", lang.pkg_egui_desc()),
                            ("image", "v0.25", "MIT / Apache-2.0", lang.pkg_image_desc()),
                            ("wasapi", "v0.23", "MIT / Apache-2.0", lang.pkg_wasapi_desc()),
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
                            ("uuid", "v1.20", "Apache-2.0 / MIT", lang.pkg_uuid_desc()),
                            ("tray-icon & muda", "v0.24", "Apache-2.0 / MIT", lang.pkg_tray_desc()),
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

    ctx.set_fonts(fonts);
}
