use eframe::egui;

use crate::controller::{
    EqPreset, GuiCmd,
};

use super::super::*;

impl FlyoutApp {
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

}
