use eframe::egui;

use crate::controller::{
    AudioSource, DeviceStatus, GuiCmd,
};
use crate::i18n::ALL_LANGUAGES;

use super::super::*;

impl FlyoutApp {
    pub(crate) fn render_settings_page(&mut self, ui: &mut egui::Ui) -> f32 {
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

}
