use eframe::egui;

use crate::controller::GuiCmd;

use super::super::*;

impl FlyoutApp {
    pub(crate) fn render_about_page(&mut self, ui: &mut egui::Ui) -> f32 {
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

}
