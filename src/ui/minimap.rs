// src/ui/minimap.rs
use crate::config::map_config::TILE_SIZE;
use crate::map::qk::{QkMap, QK_MAX_X, QK_MAX_Y, QK_MIN_X, QK_MIN_Y};
use crate::player::{FacingDirection, Player};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

// =========================================================================
// 【小地图视觉与规格配置区】
// =========================================================================
/// 罗盘雷达半径 (像素)：70.0 代表直径 140px
pub const MINIMAP_RADIUS: f32 = 70.0;
/// 距离屏幕左下角的物理边距
pub const MINIMAP_MARGIN: f32 = 25.0;
/// 缩放比率：0.038
pub const MINIMAP_SCALE: f32 = 0.038;

// =========================================================================
// 【核心系统】：自由绝对定位区域（解决平头与居中偏离）
// =========================================================================
pub fn render_minimap(
    mut contexts: EguiContexts,
    player_query: Query<(&Transform, &Player)>,
    qk_map: Option<Res<QkMap>>,
    mut baked_texture: Local<Option<egui::TextureHandle>>,
) -> Result {
    let Ok((player_tf, player)) = player_query.single() else {
        return Ok(());
    };

    let ctx = contexts.ctx_mut()?;

    // -------------------------------------------------------------------------
    // 1. 首帧地质烘焙：将 300x200 青城数据瞬间烘焙成 200KB 像素缩略图
    // -------------------------------------------------------------------------
    if baked_texture.is_none() {
        if let Some(qk) = &qk_map {
            let w = qk.width as usize;
            let h = qk.height as usize;
            let mut image =
                egui::ColorImage::new([w, h], vec![egui::Color32::from_rgb(22, 60, 42); w * h]);

            for row in 0..h {
                for col in 0..w {
                    let idx = row * w + col;
                    let mut pixel_color = egui::Color32::from_rgb(22, 60, 42); // 自然深草底

                    // 1. 地表层 (ccdi)
                    if idx < qk.ccdi_data.len() && qk.ccdi_data[idx] > 0 {
                        pixel_color = egui::Color32::from_rgb(28, 75, 52);
                    }
                    // 2. 道路小径层 (dclu)
                    if idx < qk.dclu_data.len() && qk.dclu_data[idx] > 0 {
                        pixel_color = egui::Color32::from_rgb(76, 163, 85);
                    }
                    // 3. 围墙与断崖层 (wwql)
                    if idx < qk.wwql_data.len() && qk.wwql_data[idx] > 0 {
                        pixel_color = egui::Color32::from_rgb(105, 48, 32);
                    }
                    // 4. 建筑与神树层 (jmvu)
                    if idx < qk.jmvu_data.len() && qk.jmvu_data[idx] > 0 {
                        let gid = qk.jmvu_data[idx];
                        if gid < qk.aurora_firstgid {
                            pixel_color = egui::Color32::from_rgb(140, 85, 50);
                        } else {
                            pixel_color = egui::Color32::from_rgb(38, 122, 185);
                            // 蓝顶屋舍
                        }
                    }

                    image[(col, row)] = pixel_color;
                }
            }

            *baked_texture =
                Some(ctx.load_texture("qk_minimap_baked", image, egui::TextureOptions::NEAREST));
        }
    }

    // -------------------------------------------------------------------------
    // 2. 高精度画布物理尺寸与核心绝对坐标换算
    // -------------------------------------------------------------------------
    let player_world_pos = player_tf.translation.truncate();

    // 换算当前玩家的大致世界瓦片整数坐标
    let coord_tile_x = (player_world_pos.x / TILE_SIZE).round() as i32;
    let coord_tile_y = (player_world_pos.y / TILE_SIZE).round() as i32;

    // 解包 Option<Rect>，并在获取失败时安全回退至默认 1280x720 分辨率
    let screen_rect = ctx
        .input(|i| i.raw.screen_rect)
        .unwrap_or(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        ));

    // 计算绝对圆心位置（靠屏幕左下角，留足缓冲边距，彻底消灭外围铜线被切的可能）
    let center_x = MINIMAP_MARGIN + MINIMAP_RADIUS + 6.0;
    let center_y = screen_rect.max.y - MINIMAP_MARGIN - MINIMAP_RADIUS - 32.0;
    let center = egui::pos2(center_x, center_y);

    // -------------------------------------------------------------------------
    // 3. 绝对定位画布层：使用高度包容、绝无裁剪边框的 Area 自由绘制
    // -------------------------------------------------------------------------
    egui::Area::new(egui::Id::new("minimap_absolute_area"))
        .fixed_pos(egui::pos2(0.0, 0.0)) // 锁定全屏基准点
        .interactable(false) // 允许鼠标穿透
        .show(ctx, |ui| {
            let painter = ui.painter();

            // A. 绘制罗盘最底层绿色自然底板
            painter.circle_filled(center, MINIMAP_RADIUS, egui::Color32::from_rgb(20, 52, 36));

            // B. 完美的放大镜效果：利用三角形扇面 (Mesh) 绘制严格的真·圆形贴图
            if let Some(tex) = baked_texture.as_ref() {
                let qk_left_px = QK_MIN_X as f32 * TILE_SIZE;
                let qk_right_px = QK_MAX_X as f32 * TILE_SIZE;
                let qk_top_px = QK_MAX_Y as f32 * TILE_SIZE;
                let qk_bottom_px = QK_MIN_Y as f32 * TILE_SIZE;

                let qk_w = qk_right_px - qk_left_px;
                let qk_h = qk_top_px - qk_bottom_px;

                let segments = 64;
                let mut mesh = egui::Mesh::with_texture(tex.id());

                // 中心顶点
                let center_u = (player_world_pos.x - qk_left_px) / qk_w;
                let center_v = (qk_top_px - player_world_pos.y) / qk_h; // Y 轴翻转
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: center,
                    uv: egui::pos2(center_u, center_v),
                    color: egui::Color32::WHITE,
                });

                // 沿圆周生成边缘顶点
                for i in 0..=segments {
                    let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                    let dir = egui::vec2(angle.cos(), angle.sin());
                    let edge_pos = center + dir * MINIMAP_RADIUS;

                    // 根据小地图物理像素反推对应世界坐标中的 UV 采样位置
                    let world_x = player_world_pos.x + dir.x * (MINIMAP_RADIUS / MINIMAP_SCALE);
                    let world_y = player_world_pos.y - dir.y * (MINIMAP_RADIUS / MINIMAP_SCALE);

                    let u = (world_x - qk_left_px) / qk_w;
                    let v = (qk_top_px - world_y) / qk_h;

                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: edge_pos,
                        uv: egui::pos2(u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)),
                        color: egui::Color32::WHITE,
                    });

                    // 组装三角形索引
                    if i > 0 {
                        mesh.indices.push(0);
                        mesh.indices.push(i as u32);
                        mesh.indices.push((i + 1) as u32);
                    }
                }

                // 自由绘制真圆形贴图（没有窗口容器拦截，平头彻底杜绝！）
                painter.add(egui::Shape::mesh(mesh));
            }

            // C. 绘制青铜罗盘内圈收边线
            painter.circle_stroke(
                center,
                MINIMAP_RADIUS - 0.5,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(18, 24, 20)),
            );

            // D. 绘制厚重青铜外框 (覆盖在刚好切平的中线上)
            painter.circle_stroke(
                center,
                MINIMAP_RADIUS + 1.5,
                egui::Stroke::new(4.5, egui::Color32::from_rgb(160, 128, 70)),
            );

            // E. 四正方位神兽意象着色 (北-玄武玛雅蓝，南-朱雀红褐，西-白虎白蓝，东-青龙青绿)
            let ticks = [
                (
                    egui::vec2(0.0, -MINIMAP_RADIUS),
                    egui::Color32::from_rgb(143, 76, 56), // 北/上方方位：朱雀 (红褐/赤赭)
                ),
                (
                    egui::vec2(0.0, MINIMAP_RADIUS),
                    egui::Color32::from_rgb(115, 172, 196), // 南/下方方位：玄武 (玛雅蓝/碧靛)
                ),
                (
                    egui::vec2(-MINIMAP_RADIUS, 0.0),
                    egui::Color32::from_rgb(200, 220, 240), // 西/左方方位：白虎 (白蓝/霜蓝)
                ),
                (
                    egui::vec2(MINIMAP_RADIUS, 0.0),
                    egui::Color32::from_rgb(66, 155, 114), // 东/右方方位：青龙 (青绿/翡翠)
                ),
            ];
            for (offset, color) in ticks {
                painter.circle_filled(center + offset, 3.5, color);
                painter.circle_stroke(
                    center + offset,
                    3.5,
                    egui::Stroke::new(0.6, egui::Color32::from_rgb(24, 28, 26)),
                );
            }

            // F. 正中央金色指南神针
            let forward_vec = match player.facing {
                FacingDirection::North => egui::vec2(0.0, -1.0),
                FacingDirection::South => egui::vec2(0.0, 1.0),
                FacingDirection::East => egui::vec2(1.0, 0.0),
                FacingDirection::West => egui::vec2(-1.0, 0.0),
                FacingDirection::NorthEast => egui::vec2(0.707, -0.707),
                FacingDirection::NorthWest => egui::vec2(-0.707, -0.707),
                FacingDirection::SouthEast => egui::vec2(0.707, 0.707),
                FacingDirection::SouthWest => egui::vec2(-0.707, 0.707),
            };
            let right_vec = egui::vec2(-forward_vec.y, forward_vec.x);

            let tip = center + forward_vec * 8.5;
            let left_wing = center - forward_vec * 4.0 - right_vec * 4.5;
            let right_wing = center - forward_vec * 4.0 + right_vec * 4.5;

            if player.is_meditating {
                painter.circle_filled(
                    center,
                    7.5,
                    egui::Color32::from_rgba_premultiplied(70, 180, 240, 90),
                );
            }

            painter.add(egui::Shape::convex_polygon(
                vec![tip, left_wing, center, right_wing],
                egui::Color32::from_rgb(255, 215, 60),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 80, 20)),
            ));
            painter.circle_filled(center, 2.0, egui::Color32::from_rgb(180, 40, 20));

            // -------------------------------------------------------------
            // G. 【核心修复区】：对齐 egui 0.36 包含 5 个参数的最新 rect API
            // -------------------------------------------------------------
            let badge_w = 112.0;
            let badge_h = 18.0;
            let badge_pos = egui::pos2(center_x - (badge_w / 2.0), center_y + MINIMAP_RADIUS + 8.0);
            let badge_rect = egui::Rect::from_min_size(badge_pos, egui::vec2(badge_w, badge_h));
            // 补全第5个缺失的参数：egui::StrokeKind::Outside
            painter.rect(
                badge_rect,
                3.0,
                egui::Color32::from_black_alpha(190),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(140, 110, 60)),
                egui::StrokeKind::Outside,
            );
            // 高精度渲染文本，死死扣在矩形几何中心处
            let text_string = format!("X: {}, Y: {}", coord_tile_x, coord_tile_y);
            painter.text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                text_string,
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(220, 195, 130),
            );
        });
    Ok(())
}
