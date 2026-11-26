use eframe::egui;

use crate::controller::{
    EqPreset, GuiCmd,
};

use super::super::*;

impl FlyoutApp {
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

}
