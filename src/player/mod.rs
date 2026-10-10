// src/player/mod.rs
// =========================================================================
// 【模块说明】：玩家控制中心与物理行为引擎。
// 职责：
// 1. 响应键盘输入（WASD/方向键移动、Shift 疾跑加速、G 键切换步态）。
// 2. 维护 8 方向朝向记忆与动画帧切片平滑切换（下、左、上、右及四斜角）。
// 3. 实现轴分离的 AABB 与高精度多边形（Polygon）滑动碰撞检测（撞墙不卡死、顺墙滑行）。
// 4. 实现无字隐喻核心：停步超过 1.5 秒自动切入 1-6 盘膝打坐状态并稳定聚气。
// =========================================================================

use crate::config::player_config::*;
use crate::map::nature::Collider;
use bevy::prelude::*;

/// 【玩家核心插件】：将玩家相关的系统（Startup、Update）注册进 Bevy 引擎总成中
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (player_movement_and_collision, player_meditation));
    }
}

// =========================================================================
// 【数据结构与组件定义 (Components)】
// =========================================================================

/// 玩家角色面向的八方空间朝向枚举
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FacingDirection {
    South,     // 正南 (下)
    West,      // 正西 (左)
    North,     // 正北 (上)
    East,      // 正东 (右)
    SouthWest, // 西南 (斜左下)
    SouthEast, // 东南 (斜右下)
    NorthWest, // 西北 (斜左上)
    NorthEast, // 东北 (斜右上)
}

/// 玩家的核心状态组件：挂载在玩家实体上，记录其运行时的一切修仙属性与行为标志
#[derive(Component)]
pub struct Player {
    pub speed: f32,              // 当前移动速度
    pub idle_timer: f32,         // 站立静止计时器 (用于判定何时触发打坐)
    pub is_meditating: bool,     // 是否正在盘膝打坐吐纳
    pub is_dead: bool,           // 是否力竭倒地
    pub is_running: bool,        // 是否按住了 Shift 疾跑
    pub qi: f32,                 // 当前气海灵气值
    pub max_qi: f32,             // 气海灵气上限
    pub facing: FacingDirection, // 最后一次停步时的面朝方向
    pub gait: GaitStyle,         // 当前步态风格 (正常、顺拐、练功)
}

/// 动画步频计时器组件：控制步行时每隔多少秒切换一次双腿交替的切片帧
#[derive(Component)]
pub struct StepTimer(pub Timer);

// =========================================================================
// 【初始化系统】：在世界原点生成玩家实体
// =========================================================================
fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // 1. 加载人物贴图总集
    let texture = asset_server.load("Spritesheet/base_sprite.png");

    // 2. 按照 48x72 像素规格对 base_sprite 进行网格切分 (12列 x 5行)
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(FRAME_WIDTH as u32, FRAME_HEIGHT as u32),
        SHEET_COLUMNS,
        SHEET_ROWS,
        None,
        None,
    );
    let layout_handle = texture_atlas_layouts.add(layout);

    // 3. 计算初始 Y-Sort 深度：以鞋底接触地面的物理线换算 Z 轴
    let initial_feet_y = 0.0 + PLAYER_FEET_OFFSET_Y;
    let initial_z = 50.0 - (initial_feet_y * 0.001);

    // 4. 正式向世界投放玩家实体
    commands.spawn((
        Sprite {
            image: texture,
            texture_atlas: Some(TextureAtlas {
                layout: layout_handle,
                index: IDLE_SOUTH.to_index(SHEET_COLUMNS), // 默认正面站立切片 (1-1)
            }),
            custom_size: Some(Vec2::new(FRAME_WIDTH, FRAME_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, initial_z), // 初始出生在世界中心
        Player {
            speed: MOVE_SPEED,
            idle_timer: 0.0,
            is_meditating: false,
            is_dead: false,
            is_running: false,
            qi: 0.0,
            max_qi: MAX_QI,
            facing: FacingDirection::South,
            gait: GaitStyle::Normal,
        },
        StepTimer(Timer::from_seconds(0.16, TimerMode::Repeating)), // 每 0.16 秒切换一帧步态
    ));
}

// =========================================================================
// 【几何碰撞核心算法库】：点到线段距离、多边形点包含判定
// =========================================================================

/// [数学原理]：计算空间点 p 到线段 ab 的最短距离的平方。
/// 用于精准判断玩家脚底圆心是否触碰到了 Tiled 手绘多边形的任意一条斜边。
fn dist_sq_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let len_sq = ab.length_squared();
    if len_sq == 0.0 {
        return ap.length_squared();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length_squared()
}

/// [射线投射法 (Ray Casting)]：判断点 p 是否位于任意闭合多边形内部。
/// 用于防止玩家以极高速度直接穿透/穿模进多边形墙体内部。
fn point_inside_polygon(p: Vec2, vertices: &[Vec2]) -> bool {
    let mut inside = false;
    let n = vertices.len();
    let mut j = n - 1;
    for i in 0..n {
        let vi = vertices[i];
        let vj = vertices[j];
        if ((vi.y > p.y) != (vj.y > p.y))
            && (p.x < (vj.x - vi.x) * (p.y - vi.y) / (vj.y - vi.y) + vi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 【统合碰撞检测器】：判断候选脚底位置是否与任意 AABB 矩形或多边形发生碰撞
fn check_collision(
    candidate_feet: Vec2,
    player_radius: f32,
    obs_tf: &Transform,
    collider: &Collider,
) -> bool {
    // 情况 A：如果该障碍物带有高精度多边形顶点数据 (Polygon)
    if let Some(vertices) = &collider.vertices {
        let min = collider.min;
        let max = collider.max;

        // 粗筛：如果脚底距离多边形包围盒太远，直接开绿灯放行，零性能损耗
        if candidate_feet.x < min.x - player_radius
            || candidate_feet.x > max.x + player_radius
            || candidate_feet.y < min.y - player_radius
            || candidate_feet.y > max.y + player_radius
        {
            return false;
        }
        // 若陷在多边形内部 -> 判定为碰撞
        if point_inside_polygon(candidate_feet, vertices) {
            return true;
        }
        // 细筛：若脚底到多边形任意一条边线段的距离小于玩家半径 -> 判定为碰撞
        let r_sq = player_radius * player_radius;
        let n = vertices.len();
        for i in 0..n {
            let a = vertices[i];
            let b = vertices[(i + 1) % n];
            if dist_sq_to_segment(candidate_feet, a, b) < r_sq {
                return true;
            }
        }
        false
    }
    // 情况 B：标准矩形盒碰撞 (AABB)
    else {
        let obs_pos = obs_tf.translation.truncate() + collider.offset;
        let player_half = Vec2::splat(player_radius);
        let obs_half = collider.size * 0.5;

        (candidate_feet.x - obs_pos.x).abs() < (player_half.x + obs_half.x)
            && (candidate_feet.y - obs_pos.y).abs() < (player_half.y + obs_half.y)
    }
}

// =========================================================================
// 【核心物理移动与动画更新系统】：每帧处理移动、朝向、步态、Y-Sort与碰撞滑动
// =========================================================================
fn player_movement_and_collision(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Player, &mut Sprite, &mut StepTimer)>,
    collider_query: Query<(&Transform, &Collider), Without<Player>>,
) {
    let Ok((mut transform, mut player, mut sprite, mut timer)) = query.single_mut() else {
        return;
    };

    // 1. 如果角色力竭死亡，直接锁定死状切片并终止后续移动计算
    if player.is_dead {
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = DEATH.to_index(SHEET_COLUMNS);
        }
        return;
    }

    // 2. 按下 G 键可循环切换三种步态 (正常 -> 顺拐 -> 练功)
    if keyboard.just_pressed(KeyCode::KeyG) {
        player.gait = match player.gait {
            GaitStyle::Normal => GaitStyle::NervousStumble,
            GaitStyle::NervousStumble => GaitStyle::MartialArt,
            GaitStyle::MartialArt => GaitStyle::Normal,
        };
    }

    // 3. 奔跑判定：按住 Shift 键，移速乘以 2.0 倍
    player.is_running =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let current_speed = if player.is_running {
        player.speed * RUN_SPEED_MULTIPLIER
    } else {
        player.speed
    };

    // 4. 读取键盘方向键 (WASD 或 方向键)
    let mut dir = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }

    // 禁用左右镜像翻转，因为原图自带独立左右侧脸切片
    sprite.flip_x = false;

    // --- 分支 A：玩家正在移动 ---
    if dir != Vec2::ZERO {
        // 根据向量计算 8 方向朝向
        player.facing = if dir.x < 0.0 && dir.y < 0.0 {
            FacingDirection::SouthWest
        } else if dir.x > 0.0 && dir.y < 0.0 {
            FacingDirection::SouthEast
        } else if dir.x < 0.0 && dir.y > 0.0 {
            FacingDirection::NorthWest
        } else if dir.x > 0.0 && dir.y > 0.0 {
            FacingDirection::NorthEast
        } else if dir.y < 0.0 {
            FacingDirection::South
        } else if dir.x < 0.0 {
            FacingDirection::West
        } else if dir.y > 0.0 {
            FacingDirection::North
        } else {
            FacingDirection::East
        };

        dir = dir.normalize();
        let move_delta = dir * current_speed * time.delta_secs();

        let current_pos = transform.translation.truncate();
        let player_radius = 6.0; // 脚底触碰半径

        // 【工业级轴分离滑行碰撞】：
        // 先尝试在 X 轴上移动，如果撞墙则取消 X 移动；再尝试在 Y 轴上移动，实现完美的沿墙滑行感！
        let candidate_x = current_pos.x + move_delta.x;
        let feet_y = current_pos.y + PLAYER_FEET_OFFSET_Y;
        let mut can_move_x = true;
        for (obs_tf, collider) in collider_query.iter() {
            if check_collision(
                Vec2::new(candidate_x, feet_y),
                player_radius,
                obs_tf,
                collider,
            ) {
                can_move_x = false;
                break;
            }
        }
        if can_move_x {
            transform.translation.x = candidate_x;
        }

        let new_pos_x = transform.translation.x;
        let candidate_y = current_pos.y + move_delta.y;
        let candidate_feet_y = candidate_y + PLAYER_FEET_OFFSET_Y;
        let mut can_move_y = true;
        for (obs_tf, collider) in collider_query.iter() {
            if check_collision(
                Vec2::new(new_pos_x, candidate_feet_y),
                player_radius,
                obs_tf,
                collider,
            ) {
                can_move_y = false;
                break;
            }
        }
        if can_move_y {
            transform.translation.y = candidate_y;
        }

        // [2.5D 立体遮挡 Y-Sort]：实时根据鞋底实际 Y 坐标动态更新 Z 轴深度
        let final_feet_y = transform.translation.y + PLAYER_FEET_OFFSET_Y;
        transform.translation.z = 50.0 - (final_feet_y * 0.001);

        // 重置静止计时，打断打坐状态
        player.idle_timer = 0.0;
        player.is_meditating = false;

        // 根据朝向与当前步态挑选正确的帧序列数组
        let frames = match (player.facing, player.gait) {
            (
                FacingDirection::South | FacingDirection::SouthEast | FacingDirection::SouthWest,
                GaitStyle::Normal,
            ) => NORMAL_WALK_SOUTH,
            (FacingDirection::West, GaitStyle::Normal) => NORMAL_WALK_WEST,
            (
                FacingDirection::North | FacingDirection::NorthEast | FacingDirection::NorthWest,
                GaitStyle::Normal,
            ) => NORMAL_WALK_NORTH,
            (FacingDirection::East, GaitStyle::Normal) => NORMAL_WALK_EAST,

            (
                FacingDirection::South | FacingDirection::SouthEast | FacingDirection::SouthWest,
                GaitStyle::NervousStumble,
            ) => STUMBLE_WALK_SOUTH,
            (FacingDirection::West, GaitStyle::NervousStumble) => STUMBLE_WALK_WEST,
            (
                FacingDirection::North | FacingDirection::NorthEast | FacingDirection::NorthWest,
                GaitStyle::NervousStumble,
            ) => STUMBLE_WALK_NORTH,
            (FacingDirection::East, GaitStyle::NervousStumble) => STUMBLE_WALK_EAST,

            (
                FacingDirection::South | FacingDirection::SouthEast | FacingDirection::SouthWest,
                GaitStyle::MartialArt,
            ) => KUNGFU_WALK_SOUTH,
            (FacingDirection::West, GaitStyle::MartialArt) => KUNGFU_WALK_WEST,
            (
                FacingDirection::North | FacingDirection::NorthEast | FacingDirection::NorthWest,
                GaitStyle::MartialArt,
            ) => KUNGFU_WALK_NORTH,
            (FacingDirection::East, GaitStyle::MartialArt) => KUNGFU_WALK_EAST,
        };

        // 奔跑时步频翻倍（脚步倒腾得更快）
        let step_delta = if player.is_running {
            time.delta().mul_f32(RUN_SPEED_MULTIPLIER)
        } else {
            time.delta()
        };

        timer.0.tick(step_delta);
        if timer.0.just_finished() && !frames.is_empty() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                let current_frame = atlas.index;
                let indices: Vec<usize> =
                    frames.iter().map(|f| f.to_index(SHEET_COLUMNS)).collect();
                let next = match indices.iter().position(|&x| x == current_frame) {
                    Some(idx) => indices[(idx + 1) % indices.len()],
                    None => indices[0],
                };
                atlas.index = next;
            }
        }
    }
    // --- 分支 B：玩家静止不动 ---
    else {
        player.idle_timer += time.delta_secs();

        // 静态 Z 轴微调
        let feet_y = if player.is_meditating {
            transform.translation.y - 20.0
        } else {
            transform.translation.y + PLAYER_FEET_OFFSET_Y
        };
        transform.translation.z = 50.0 - (feet_y * 0.001);

        // 如果静止超过设定时间，自动切入 1-6 盘膝打坐姿态！
        if player.idle_timer >= MEDITATE_IDLE_DELAY {
            player.is_meditating = true;
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = MEDITATE.to_index(SHEET_COLUMNS);
            }
        } else if !player.is_meditating {
            // 根据最后面朝的方向精确恢复站立帧
            let idle_pos = match player.facing {
                FacingDirection::South => IDLE_SOUTH,
                FacingDirection::West => IDLE_WEST,
                FacingDirection::North => IDLE_NORTH,
                FacingDirection::East => IDLE_EAST,
                FacingDirection::SouthWest => IDLE_SOUTH_WEST,
                FacingDirection::SouthEast => IDLE_SOUTH_EAST,
                FacingDirection::NorthWest => IDLE_NORTH_WEST,
                FacingDirection::NorthEast => IDLE_NORTH_EAST,
            };
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = idle_pos.to_index(SHEET_COLUMNS);
            }
        }
    }
}

// =========================================================================
// 【吐纳聚气系统】：打坐时自动恢复灵气并伴随周身灵光脉动
// =========================================================================
fn player_meditation(time: Res<Time>, mut query: Query<(&mut Player, &mut Sprite)>) {
    let Ok((mut player, mut sprite)) = query.single_mut() else {
        return;
    };

    if player.is_meditating && !player.is_dead {
        // 灵气稳步增长
        player.qi = (player.qi + QI_RECOVERY_RATE * time.delta_secs()).min(player.max_qi);
        // 周身泛起温润的蓝光呼吸脉动
        let pulse = (time.elapsed_secs() * 3.5).sin() * 0.15 + 0.85;
        sprite.color = Color::srgb(pulse * 0.9, pulse * 0.9, pulse * 1.3);
    } else {
        sprite.color = Color::WHITE;
    }
}
