// src/camera/mod.rs
use crate::player::Player;
use bevy::prelude::*;

pub struct GameCameraPlugin;

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, camera_follow_player);
    }
}

#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn camera_follow_player(
    player_query: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let Ok(mut camera_tf) = camera_query.single_mut() else {
        return;
    };

    let target_x = player_tf.translation.x;
    let target_y = player_tf.translation.y;

    // 平滑插值
    let lerp_factor = (8.0 * time.delta_secs()).min(1.0);
    camera_tf.translation.x += (target_x - camera_tf.translation.x) * lerp_factor;
    camera_tf.translation.y += (target_y - camera_tf.translation.y) * lerp_factor;

    // 【核心修复】：最终位置像素对齐取整，消除浮点数亚像素撕裂！
    camera_tf.translation.x = camera_tf.translation.x.round();
    camera_tf.translation.y = camera_tf.translation.y.round();
}
