// src/map/nature.rs
// =========================================================================
// 【文件说明】：大自然与青城地图渲染的总心脏。
// 职责：
// 1. 显存资源管理：将 RPGpack 和 Aurora 两张图集切片并加载进 GPU。
// 2. 流式大世界 (Chunk) 调度：每帧计算玩家视野，按需加载/卸载区块。
// 3. 双轨制地貌呈现：
//    - 境内（青城）：渲染手绘四层建筑与围墙，并挂载高精度多边形/矩形物理碰撞盒。
//    - 境外（大自然）：程序化随机生成草原、泥坑、8参大树。
// =========================================================================

use crate::config::map_config::*;
use crate::map::qk::{AtlasTarget, QkMap};
use crate::player::Player;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};

/// 瓦片渲染尺寸 (严格 32.0 像素)
pub const TILE_RENDER_SIZE: f32 = TILE_SIZE;

/// 视觉深度分层定义
pub mod layers {
    pub const GROUND: f32 = 0.0; // 地皮层：永远沉底
    pub const INSET_OVERLAY: f32 = 0.05; // 贴地小径/草中土
    pub const Y_SORT_BASE: f32 = 50.0; // 2.5D 立体遮挡基准线
}

/// 统一的物理碰撞组件 (完美支持矩形 AABB 与高精度多边形 Polygon)
#[derive(Component, Clone)]
pub struct Collider {
    pub size: Vec2,
    pub offset: Vec2,
    pub vertices: Option<Vec<Vec2>>, // 若有则是多边形顶点集
    pub min: Vec2,
    pub max: Vec2,
}

/// 区块身份证组件
#[derive(Component)]
pub struct ChunkBelonging {
    #[allow(dead_code)]
    pub coord: IVec2,
}

/// 显卡图集句柄资源
#[derive(Resource)]
pub struct WorldTilesets {
    pub rpgpack_image: Handle<Image>,
    pub rpgpack_layout: Handle<TextureAtlasLayout>,
    pub aurora_image: Handle<Image>,
    pub aurora_layout: Handle<TextureAtlasLayout>,
}

/// 九宫格区块沙盘
#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashMap<IVec2, Vec<Entity>>,
}

// 统一图集资源初始化
pub fn setup_tileset_resource(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let rpgpack_image = asset_server.load("Spritesheet/RPGpack_sheet.png");
    let rpgpack_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(TILE_SIZE as u32, TILE_SIZE as u32),
        SHEET_COLUMNS,
        SHEET_ROWS,
        None,
        None,
    ));

    let aurora_image = asset_server.load("Spritesheet/Aurora Tileset.png");
    let aurora_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(32, 32),
        35,
        22,
        None,
        None,
    ));

    commands.insert_resource(WorldTilesets {
        rpgpack_image,
        rpgpack_layout,
        aurora_image,
        aurora_layout,
    });
}

// 伪随机数生成器
fn pseudo_noise(x: i32, y: i32, seed: u32) -> f32 {
    let mut n = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed;
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n as f32) / (u32::MAX as f32)
}

// =========================================================================
// 【核心系统】：无限世界流式渲染管线
// =========================================================================
pub fn update_chunks_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut chunk_mgr: ResMut<ChunkManager>,
    tilesets: Option<Res<WorldTilesets>>,
    qk_map: Option<Res<QkMap>>,
    mut objects_spawned: Local<bool>,
) {
    let Some(tilesets) = tilesets else {
        return;
    };
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let Ok(window) = window_query.single() else {
        return;
    };

    // -------------------------------------------------------------------------
    // A. 开局首帧：将 Tiled 手绘多边形/矩形碰撞盒注入世界
    // -------------------------------------------------------------------------
    if !*objects_spawned {
        if let Some(qk) = &qk_map {
            for col in &qk.colliders {
                let sort_z = layers::Y_SORT_BASE - (col.min.y * 0.001);
                commands.spawn((
                    Transform::from_xyz(col.center_x, col.center_y, sort_z),
                    Collider {
                        size: col.size,
                        offset: Vec2::ZERO,
                        vertices: col.vertices.clone(), // 正确传递 Option<Vec<Vec2>>
                        min: col.min,
                        max: col.max,
                    },
                ));
            }
            *objects_spawned = true;
            info!(
                "🏛️ 成功在世界中激活 {} 处 Tiled 手绘碰撞体！",
                qk.colliders.len()
            );
        }
    }

    // -------------------------------------------------------------------------
    // B. 九宫格视距计算与垃圾回收
    // -------------------------------------------------------------------------
    let chunk_world_size = CHUNK_TILES as f32 * TILE_SIZE;
    let radius_x = ((window.width() * 0.5) / chunk_world_size).ceil() as i32 + 1;
    let radius_y = ((window.height() * 0.5) / chunk_world_size).ceil() as i32 + 1;

    let current_cx = (player_tf.translation.x / chunk_world_size).floor() as i32;
    let current_cy = (player_tf.translation.y / chunk_world_size).floor() as i32;

    let mut required_chunks = HashSet::new();
    for dx in -radius_x..=radius_x {
        for dy in -radius_y..=radius_y {
            required_chunks.insert(IVec2::new(current_cx + dx, current_cy + dy));
        }
    }

    let mut chunks_to_remove = Vec::new();
    for &coord in chunk_mgr.loaded_chunks.keys() {
        if !required_chunks.contains(&coord) {
            chunks_to_remove.push(coord);
        }
    }
    for coord in chunks_to_remove {
        if let Some(entities) = chunk_mgr.loaded_chunks.remove(&coord) {
            for entity in entities {
                commands.entity(entity).despawn();
            }
        }
    }

    let global_seed = 9527;

    // -------------------------------------------------------------------------
    // C. 逐个区块流式瓦片生成
    // -------------------------------------------------------------------------
    for chunk_coord in required_chunks {
        if chunk_mgr.loaded_chunks.contains_key(&chunk_coord) {
            continue;
        }

        let mut spawned = Vec::new();

        for local_x in 0..CHUNK_TILES {
            for local_y in 0..CHUNK_TILES {
                let gx = chunk_coord.x * CHUNK_TILES + local_x;
                let gy = chunk_coord.y * CHUNK_TILES + local_y;
                let wx = gx as f32 * TILE_SIZE;
                let wy = gy as f32 * TILE_SIZE;

                let is_inside_qk = QkMap::is_inside(gx, gy);

                // --- 路线 A：青城特区手绘四层渲染 ---
                if is_inside_qk {
                    if let Some(qk) = &qk_map {
                        if let Some(idx) = qk.coord_to_index(gx, gy) {
                            let tile_bottom_y = wy - (TILE_SIZE * 0.5);
                            let y_sort_z = layers::Y_SORT_BASE - (tile_bottom_y * 0.001);

                            let render_layers: [(&str, &Vec<u32>, f32, bool); 4] = [
                                ("4-ccdi", &qk.ccdi_data, layers::GROUND, false),
                                ("3-wwql", &qk.wwql_data, y_sort_z, true),
                                ("2-dclu", &qk.dclu_data, layers::INSET_OVERLAY, false),
                                ("1-jmvu", &qk.jmvu_data, y_sort_z + 0.0001, false),
                            ];

                            for (_layer_name, data_array, z_pos, need_collision) in render_layers {
                                if idx < data_array.len() {
                                    let gid = data_array[idx];
                                    if let Some(target) = qk.resolve_gid(gid) {
                                        let (image, layout, index, is_aurora) = match target {
                                            AtlasTarget::Aurora(i) => (
                                                tilesets.aurora_image.clone(),
                                                tilesets.aurora_layout.clone(),
                                                i,
                                                true,
                                            ),
                                            AtlasTarget::RpgPack(i) => (
                                                tilesets.rpgpack_image.clone(),
                                                tilesets.rpgpack_layout.clone(),
                                                i,
                                                false,
                                            ),
                                        };

                                        let mut e = commands.spawn((
                                            Sprite {
                                                image,
                                                texture_atlas: Some(TextureAtlas { layout, index }),
                                                custom_size: Some(Vec2::new(
                                                    TILE_RENDER_SIZE,
                                                    TILE_RENDER_SIZE,
                                                )),
                                                ..default()
                                            },
                                            Transform::from_xyz(wx, wy, z_pos),
                                            ChunkBelonging { coord: chunk_coord },
                                        ));

                                        if need_collision {
                                            let total_cols =
                                                if is_aurora { 35 } else { SHEET_COLUMNS };
                                            let (c_size, c_offset) =
                                                get_custom_collider(is_aurora, index, total_cols);
                                            e.insert(Collider {
                                                size: c_size,
                                                offset: c_offset,
                                                vertices: None,
                                                min: Vec2::ZERO,
                                                max: Vec2::ZERO,
                                            });
                                        }

                                        spawned.push(e.id());
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }

                // --- 路线 B：城外大自然随机森林生成 ---
                let ground_noise = pseudo_noise(gx, gy, global_seed);
                let idx = ((ground_noise * CLEAN_GRASS_PALETTE.len() as f32) as usize)
                    .min(CLEAN_GRASS_PALETTE.len() - 1);
                let base_tile = CLEAN_GRASS_PALETTE[idx].0;

                let ground_id = commands
                    .spawn((
                        Sprite {
                            image: tilesets.rpgpack_image.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: tilesets.rpgpack_layout.clone(),
                                index: base_tile.to_index(SHEET_COLUMNS),
                            }),
                            custom_size: Some(Vec2::new(TILE_RENDER_SIZE, TILE_RENDER_SIZE)),
                            ..default()
                        },
                        Transform::from_xyz(wx, wy, layers::GROUND),
                        ChunkBelonging { coord: chunk_coord },
                    ))
                    .id();
                spawned.push(ground_id);

                // 草中泥坑
                let puddle_noise = pseudo_noise(gx, gy, global_seed + 15);
                if local_x % 4 == 0 && local_y % 4 == 0 && puddle_noise < DIRT_PUDDLE_CHANCE {
                    let patch = INSET_GRASS_WITH_DIRT;
                    let parts = [
                        (0, 1, patch.top_l),
                        (1, 1, patch.top_r),
                        (0, 0, patch.bot_l),
                        (1, 0, patch.bot_r),
                    ];
                    for (ox, oy, tpos) in parts {
                        let e = commands
                            .spawn((
                                Sprite {
                                    image: tilesets.rpgpack_image.clone(),
                                    texture_atlas: Some(TextureAtlas {
                                        layout: tilesets.rpgpack_layout.clone(),
                                        index: tpos.to_index(SHEET_COLUMNS),
                                    }),
                                    custom_size: Some(Vec2::new(
                                        TILE_RENDER_SIZE,
                                        TILE_RENDER_SIZE,
                                    )),
                                    ..default()
                                },
                                Transform::from_xyz(
                                    wx + ox as f32 * TILE_SIZE,
                                    wy + oy as f32 * TILE_SIZE,
                                    layers::INSET_OVERLAY,
                                ),
                                ChunkBelonging { coord: chunk_coord },
                            ))
                            .id();
                        spawned.push(e);
                    }
                }

                // 大树生成
                let tree_noise = pseudo_noise(gx, gy, global_seed + 20);
                if local_x % 3 == 0
                    && local_y % 3 == 0
                    && (gx as f32).hypot(gy as f32) > 3.0
                    && tree_noise < TREE_CHANCE
                {
                    let tree_idx = ((pseudo_noise(gx, gy, global_seed + 25)
                        * ACTIVE_TREES.len() as f32) as usize)
                        .min(ACTIVE_TREES.len() - 1);
                    let tree = ACTIVE_TREES[tree_idx];

                    let tree_root_y = wy + TREE_ROOT_Y_OFFSET;
                    let tree_sort_z = layers::Y_SORT_BASE - (tree_root_y * 0.001);

                    let parts = [
                        (0, 3, tree.top_l, true),
                        (1, 3, tree.top_r, true),
                        (0, 2, tree.mid_l, true),
                        (1, 2, tree.mid_r, true),
                        (0, 1, tree.bot_l, true),
                        (1, 1, tree.bot_r, true),
                        (0, 0, tree.trunk_l, false),
                        (1, 0, tree.trunk_r, false),
                    ];

                    for (ox, oy, tpos, is_canopy) in parts {
                        let z = if is_canopy {
                            tree_sort_z + 0.0001
                        } else {
                            tree_sort_z
                        };
                        let e = commands
                            .spawn((
                                Sprite {
                                    image: tilesets.rpgpack_image.clone(),
                                    texture_atlas: Some(TextureAtlas {
                                        layout: tilesets.rpgpack_layout.clone(),
                                        index: tpos.to_index(SHEET_COLUMNS),
                                    }),
                                    custom_size: Some(Vec2::new(
                                        TILE_RENDER_SIZE,
                                        TILE_RENDER_SIZE,
                                    )),
                                    ..default()
                                },
                                Transform::from_xyz(
                                    wx + ox as f32 * TILE_SIZE,
                                    wy + oy as f32 * TILE_SIZE,
                                    z,
                                ),
                                ChunkBelonging { coord: chunk_coord },
                            ))
                            .id();
                        spawned.push(e);
                    }

                    let root_collider_id = commands
                        .spawn((
                            Transform::from_xyz(
                                wx + TREE_ROOT_COLLIDER_OFFSET.x,
                                wy + TREE_ROOT_COLLIDER_OFFSET.y,
                                tree_sort_z,
                            ),
                            Collider {
                                size: TREE_ROOT_COLLIDER_SIZE,
                                offset: TREE_ROOT_COLLIDER_OFFSET,
                                vertices: None,
                                min: Vec2::ZERO,
                                max: Vec2::ZERO,
                            },
                            ChunkBelonging { coord: chunk_coord },
                        ))
                        .id();
                    spawned.push(root_collider_id);
                }
            }
        }
        chunk_mgr.loaded_chunks.insert(chunk_coord, spawned);
    }
}
