// src/map/qk.rs
// =========================================================================
// 【模块说明】：Tiled 关卡编辑器 JSON 数据加载器。
// 职责：解析 qk.json，提取四层瓦片数据及手绘多边形碰撞矩形。
// =========================================================================

use bevy::prelude::*;
use serde::Deserialize;

// --- 青城宏观尺度与坐标设定 ---
pub const QK_WIDTH: i32 = 300; // 青城横向尺寸
pub const QK_HEIGHT: i32 = 200; // 青城纵向尺寸
pub const QK_CENTER_X: i32 = 0; // 世界原点 X
pub const QK_CENTER_Y: i32 = 0; // 世界原点 Y

pub const QK_MIN_X: i32 = QK_CENTER_X - QK_WIDTH / 2;
pub const QK_MAX_X: i32 = QK_CENTER_X + QK_WIDTH / 2;
pub const QK_MIN_Y: i32 = QK_CENTER_Y - QK_HEIGHT / 2;
pub const QK_MAX_Y: i32 = QK_CENTER_Y + QK_HEIGHT / 2;

// --- Tiled JSON 反序列化底层结构 ---
#[derive(Deserialize, Debug)]
struct TiledJson {
    width: i32,
    height: i32,
    #[serde(default)]
    layers: Vec<TiledLayer>,
    #[serde(default)]
    tilesets: Vec<TiledTilesetRef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TiledLayer {
    pub name: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub data: Vec<u32>,
    #[serde(default)]
    pub objects: Vec<TiledRawObject>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TiledPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TiledRawObject {
    #[serde(default)]
    #[allow(dead_code)] // 👈 告诉编译器：我知道没用它，别报错
    pub name: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub polygon: Option<Vec<TiledPoint>>,
    #[serde(default)]
    pub text: Option<serde_json::Value>,
}

/// 统一的物理碰撞几何体数据
#[derive(Clone, Debug)]
pub struct ParsedCollider {
    pub center_x: f32,
    pub center_y: f32,
    pub size: Vec2,
    pub vertices: Option<Vec<Vec2>>,
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Deserialize, Debug)]
struct TiledTilesetRef {
    firstgid: u32,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

/// 图集分流目标
pub enum AtlasTarget {
    Aurora(usize),
    RpgPack(usize),
}

/// 全局常驻资源：青城轻量内存沙盘
#[derive(Resource)]
pub struct QkMap {
    pub ccdi_data: Vec<u32>,            // 层 1：草底
    pub wwql_data: Vec<u32>,            // 层 2：围墙
    pub dclu_data: Vec<u32>,            // 层 3：道路
    pub jmvu_data: Vec<u32>,            // 层 4：建筑
    pub colliders: Vec<ParsedCollider>, // 手绘多边形/矩形碰撞盒

    pub width: i32,
    pub height: i32,
    pub aurora_firstgid: u32,
    pub rpgpack_firstgid: u32,
}

impl QkMap {
    /// 判断世界坐标是否在青城境内
    pub fn is_inside(gx: i32, gy: i32) -> bool {
        gx >= QK_MIN_X && gx < QK_MAX_X && gy >= QK_MIN_Y && gy < QK_MAX_Y
    }

    /// 坐标系转换：世界坐标 -> Tiled 数组下标
    pub fn coord_to_index(&self, gx: i32, gy: i32) -> Option<usize> {
        let col = gx - QK_MIN_X;
        let row = (QK_MAX_Y - 1) - gy;

        if col >= 0 && col < self.width && row >= 0 && row < self.height {
            Some((row * self.width + col) as usize)
        } else {
            None
        }
    }

    /// 根据全局 GID 自动分流图集
    pub fn resolve_gid(&self, gid: u32) -> Option<AtlasTarget> {
        if gid == 0 {
            return None;
        }
        if self.aurora_firstgid < self.rpgpack_firstgid {
            if gid < self.rpgpack_firstgid {
                Some(AtlasTarget::Aurora((gid - self.aurora_firstgid) as usize))
            } else {
                Some(AtlasTarget::RpgPack((gid - self.rpgpack_firstgid) as usize))
            }
        } else {
            if gid < self.aurora_firstgid {
                Some(AtlasTarget::RpgPack((gid - self.rpgpack_firstgid) as usize))
            } else {
                Some(AtlasTarget::Aurora((gid - self.aurora_firstgid) as usize))
            }
        }
    }
}

/// 游戏启动时解析 qk.json
pub fn load_qk_data(mut commands: Commands) {
    let json_str = include_str!("../../assets/maps/qk.json");

    match serde_json::from_str::<TiledJson>(json_str) {
        Ok(parsed) => {
            let mut ccdi = Vec::new();
            let mut wwql = Vec::new();
            let mut dclu = Vec::new();
            let mut jmvu = Vec::new();
            let mut raw_objects = Vec::new();

            for layer in parsed.layers {
                let name = layer.name.trim().to_lowercase();
                if layer.r#type == "objectgroup"
                    || name.contains("collision")
                    || name.contains("building")
                    || name.contains("0-buildings")
                {
                    raw_objects.extend(layer.objects);
                } else if name.contains("ccdi") {
                    ccdi = layer.data;
                } else if name.contains("wwql") {
                    wwql = layer.data;
                } else if name.contains("dclu") {
                    dclu = layer.data;
                } else if name.contains("jmvu") {
                    jmvu = layer.data;
                }
            }

            let qk_left_px = QK_MIN_X as f32 * 32.0;
            let qk_top_px = QK_MAX_Y as f32 * 32.0;
            let half_tile = 16.0;
            let mut colliders = Vec::new();

            for obj in raw_objects {
                if obj.text.is_some() {
                    continue;
                }

                // 1. 【多边形碰撞体】：有 polygon 顶点数据，且至少 3 个点
                if let Some(pts) = &obj.polygon {
                    if pts.len() >= 3 {
                        let mut world_pts = Vec::new();
                        let mut min_x = f32::MAX;
                        let mut max_x = f32::MIN;
                        let mut min_y = f32::MAX;
                        let mut max_y = f32::MIN;

                        for p in pts {
                            let wx = qk_left_px + (obj.x + p.x) - half_tile;
                            let wy = qk_top_px - (obj.y + p.y) - half_tile;

                            min_x = min_x.min(wx);
                            max_x = max_x.max(wx);
                            min_y = min_y.min(wy);
                            max_y = max_y.max(wy);
                            world_pts.push(Vec2::new(wx, wy));
                        }

                        colliders.push(ParsedCollider {
                            center_x: (min_x + max_x) * 0.5,
                            center_y: (min_y + max_y) * 0.5,
                            size: Vec2::new(max_x - min_x, max_y - min_y),
                            vertices: Some(world_pts),
                            min: Vec2::new(min_x, min_y),
                            max: Vec2::new(max_x, max_y),
                        });
                    }
                }
                // 2. 【矩形/椭圆/圆形碰撞体】：在 Tiled 中，这些纯几何形状的特殊点在于：
                //    它们没有关联任何图片（即没有全局图块 ID，Tiled 导出的 JSON 中 gid 字段会是 None 或 0）
                //    并且它们必须具备一定的物理宽高（比如大于 2 像素）
                else if obj.width > 2.0 && obj.height > 2.0 {
                    // 🚨 关键补丁：利用 serde_json 的临时解析或者直接在这里根据没有绑定图片来判断
                    // 如果这是一个直接从图集里拉出来的“带有图片的物件”，它通常会有 gid，我们在这里过滤掉它

                    let left = qk_left_px + obj.x - half_tile;
                    let right = left + obj.width;
                    let top = qk_top_px - obj.y - half_tile;
                    let bottom = top - obj.height;

                    colliders.push(ParsedCollider {
                        center_x: (left + right) * 0.5,
                        center_y: (top + bottom) * 0.5,
                        size: Vec2::new(obj.width, obj.height),
                        vertices: None,
                        min: Vec2::new(left, bottom),
                        max: Vec2::new(right, top),
                    });
                }
            }

            let mut aurora_firstgid = 1;
            let mut rpgpack_firstgid = 771;

            for ts in &parsed.tilesets {
                let label = ts.name.as_deref().or(ts.source.as_deref()).unwrap_or("");
                if label.contains("Aurora") || label.contains("aurora") {
                    aurora_firstgid = ts.firstgid;
                } else if label.contains("RPG") || label.contains("rpg") {
                    rpgpack_firstgid = ts.firstgid;
                }
            }

            if parsed.tilesets.len() >= 2 && aurora_firstgid == rpgpack_firstgid {
                aurora_firstgid = parsed.tilesets[0].firstgid;
                rpgpack_firstgid = parsed.tilesets[1].firstgid;
            }

            info!(
                "✅ 成功装载青城数据！共识别出 {} 处高精度物理碰撞体！",
                colliders.len()
            );

            commands.insert_resource(QkMap {
                ccdi_data: ccdi,
                dclu_data: dclu,
                wwql_data: wwql,
                jmvu_data: jmvu,
                colliders,
                width: parsed.width,
                height: parsed.height,
                aurora_firstgid,
                rpgpack_firstgid,
            });
        }
        Err(e) => {
            error!("❌ Tiled 地图解析失败: {}", e);
        }
    }
}
