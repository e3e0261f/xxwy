// src/ui/mod.rs
pub mod minimap; // 声明小地图子模块

use crate::player::Player;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use minimap::render_minimap; // 引入小地图渲染系统

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        // 初始化 Egui 插件，并将 UI 系统挂载到 0.42 规范的专用 EguiPrimaryContextPass 阶段
        app.add_plugins(EguiPlugin::default()).add_systems(
            EguiPrimaryContextPass,
            (render_minimal_ui, render_minimap), // 灵气槽与小地图罗盘同步调度！
        );
    }
}

/// 右上角极简无字气海槽：以视觉色彩流动隐喻修仙吸纳
fn render_minimal_ui(mut contexts: EguiContexts, player_query: Query<&Player>) -> Result {
    let Ok(player) = player_query.single() else {
        return Ok(());
    };

    egui::Window::new("qi_orb")
        .anchor(egui::Align2::RIGHT_TOP, [-20.0, 20.0])
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .show(contexts.ctx_mut()?, |ui| {
            let ratio = (player.qi / player.max_qi).clamp(0.0, 1.0);

            // 动态色彩：入定时是聚气青蓝，蓄满时泛出道蕴金芒
            let fill_color = if ratio >= 1.0 {
                egui::Color32::from_rgb(255, 215, 0)
            } else {
                egui::Color32::from_rgb(45, 170, 225)
            };

            ui.add(
                egui::ProgressBar::new(ratio)
                    .fill(fill_color)
                    .desired_width(130.0)
                    .desired_height(12.0)
                    .animate(player.is_meditating),
            );
        });

    Ok(())
}
