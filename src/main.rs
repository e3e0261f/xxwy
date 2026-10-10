// src/main.rs
mod camera;
pub mod config;
mod map;
mod player;
mod ui;

use bevy::prelude::*;
use camera::GameCameraPlugin;
use map::MapPlugin;
use player::PlayerPlugin;
use ui::GameUiPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "修仙世界".into(),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                // 【核心修复这一行】：强制全局启用像素画最近邻采样，彻底根治 1 像素缝隙与外溢！
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((GameCameraPlugin, MapPlugin, PlayerPlugin, GameUiPlugin))
        .run();
}
