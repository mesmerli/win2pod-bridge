use eframe::egui;

use crate::controller::DeviceStatus;

use super::*;

pub(crate) fn render_slide_shadow(ui: &mut egui::Ui, seam_x: f32, clip_rect: egui::Rect) {
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
pub(crate) fn catmull_rom(p0: egui::Pos2, p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2, t: f32) -> egui::Pos2 {
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
pub(crate) fn render_eq_curve_overlay(
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
                            self.mascot_pos.y = self.mascot_pos.y.clamp(12.0, (target_h - 16.0).max(12.0));
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
                            self.mascot_pos.y = self.mascot_pos.y.clamp(12.0, (target_h - 16.0).max(12.0));
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
        let min_bound_x = min_x + safe_margin_x;
        let max_bound_x = (min_x + w - safe_margin_x).max(min_bound_x);
        final_pos.x = final_pos.x.clamp(min_bound_x, max_bound_x);

        let min_bound_y = min_y + safe_margin_y;
        let max_bound_y = (min_y + h - safe_margin_y).max(min_bound_y);
        final_pos.y = final_pos.y.clamp(min_bound_y, max_bound_y);

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
            let min_spec_x = spec_rect.min.x + 8.0;
            let max_spec_x = (spec_rect.max.x - 8.0).max(min_spec_x);
            let target_x = pointer_pos
                .map(|p| p.x)
                .unwrap_or(spec_rect.center().x)
                .clamp(min_spec_x, max_spec_x);
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
        const GHOST_BYTES: &[u8] = include_bytes!("../../assets/ghost_emoji.png");
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
        let min_knob_x = slider_rect.min.x + 5.5;
        let max_knob_x = (slider_rect.max.x - 5.5).max(min_knob_x);
        let knob_x = (track_rect.min.x + track_rect.width() * progress).clamp(
            min_knob_x,
            max_knob_x,
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

    pub(crate) fn render_visualizer_button(&mut self, ui: &mut egui::Ui, spawner: crate::snapping::WindowKind) {
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

}
