// src/map/mod.rs
pub mod nature;
pub mod qk; // 注册青城子模块

use bevy::prelude::*;
use nature::{setup_tileset_resource, update_chunks_system, ChunkManager};
use qk::load_qk_data;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkManager>()
            .add_systems(Startup, (load_qk_data, setup_tileset_resource))
            .add_systems(Update, update_chunks_system);
    }
}
