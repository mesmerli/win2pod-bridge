use eframe::egui;

use crate::controller::GuiCmd;

use super::super::*;

impl FlyoutApp {
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
