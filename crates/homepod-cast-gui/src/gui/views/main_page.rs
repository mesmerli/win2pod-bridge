use eframe::egui;

use crate::controller::{
    DeviceItem, DeviceStatus, GuiCmd,
};
use crate::i18n::NoticeKind;

use super::super::*;

impl FlyoutApp {
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

}
