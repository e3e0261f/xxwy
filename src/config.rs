// src/config.rs
// 【注意】：不要在文件顶部写 use bevy::prelude::Vec2; 把它放到需要的模块里，否则会报 Warning。

// =========================================================================
// 【基础工具】：瓦片网格坐标映射器
// =========================================================================
/// 记录瓦片在 Tiled 图片中的行列位置 (从 1 开始数，符合人类直觉)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePos {
    pub row: u32, // 第几行 (从上往下数)
    pub col: u32, // 第几列 (从左往右数)
}

impl TilePos {
    /// 构造函数：创建一个新的行列坐标
    pub const fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// [数学原理]：将 (行, 列) 转换为 Bevy 渲染引擎能看懂的一维索引 (Index)
    /// 公式：(行数 - 1) * 总列数 + (列数 - 1)
    pub const fn to_index(&self, total_columns: u32) -> usize {
        ((self.row - 1) * total_columns + (self.col - 1)) as usize
    }
}

// =========================================================================
// 1. 人物控制与动画参数模块 (对应 base_sprite.png)
// =========================================================================
pub mod player_config {
    use super::TilePos;
    use bevy::prelude::Vec2; // 在模块内部引入，消除全局警告

    // --- 图集真实尺寸与切分规则 ---
    pub const SHEET_COLUMNS: u32 = 12; // 人物图片横向被切成 12 格
    pub const SHEET_ROWS: u32 = 5; // 人物图片纵向被切成 5 格
    pub const FRAME_WIDTH: f32 = 48.0; // 单个人物切片的物理宽度 (像素)
    pub const FRAME_HEIGHT: f32 = 72.0; // 单个人物切片的物理高度 (像素)

    // --- 物理碰撞系统参数 ---
    // [防卡死设计]：角色的物理碰撞盒只有 12x6 大小，只包含脚底板。这样角色的肩膀就不会卡在树枝上。
    pub const PLAYER_COLLIDER_SIZE: Vec2 = Vec2::new(12.0, 6.0);
    // 脚底碰撞箱相对人物图片中心点 (0,0) 的下移量。-28 代表往下移动到鞋底。
    pub const PLAYER_FEET_OFFSET_Y: f32 = -28.0;

    // --- 核心单帧动作 (行-列) ---
    pub const MEDITATE: TilePos = TilePos::new(1, 6); // 1-6 盘膝打坐切片
    pub const DEATH: TilePos = TilePos::new(1, 7); // 1-7 死亡倒地切片

    // --- 各方向静止站立姿态 (下、左、上、右) ---
    pub const IDLE_SOUTH: TilePos = TilePos::new(1, 1); // 停步面朝下
    pub const IDLE_WEST: TilePos = TilePos::new(1, 2); // 停步面朝左
    pub const IDLE_NORTH: TilePos = TilePos::new(1, 3); // 停步面朝上
    pub const IDLE_EAST: TilePos = TilePos::new(1, 4); // 停步面朝右

    // --- 斜角停步姿态 (复用上述切片) ---
    pub const IDLE_SOUTH_WEST: TilePos = TilePos::new(1, 2);
    pub const IDLE_SOUTH_EAST: TilePos = TilePos::new(1, 4);
    pub const IDLE_NORTH_WEST: TilePos = TilePos::new(1, 2);
    pub const IDLE_NORTH_EAST: TilePos = TilePos::new(1, 4);

    // --- 步态分类枚举 ---
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum GaitStyle {
        Normal,         // 正常行走
        NervousStumble, // 顺拐 / 紧张混乱
        MartialArt,     // 运功 / 演武
    }

    // --- 动画序列配置：数组里的元素是播放顺序 ---
    // 1. 正常行走步态
    pub const NORMAL_WALK_SOUTH: &[TilePos] = &[
        TilePos::new(2, 1),
        TilePos::new(2, 2),
        TilePos::new(2, 3),
        TilePos::new(2, 4),
    ];
    pub const NORMAL_WALK_WEST: &[TilePos] = &[
        TilePos::new(3, 1),
        TilePos::new(3, 2),
        TilePos::new(3, 3),
        TilePos::new(3, 4),
    ];
    pub const NORMAL_WALK_NORTH: &[TilePos] = &[
        TilePos::new(4, 1),
        TilePos::new(4, 2),
        TilePos::new(4, 3),
        TilePos::new(4, 4),
    ];
    pub const NORMAL_WALK_EAST: &[TilePos] = &[
        TilePos::new(5, 1),
        TilePos::new(5, 2),
        TilePos::new(5, 3),
        TilePos::new(5, 4),
    ];

    // 2. 顺拐 / 慌乱步态
    pub const STUMBLE_WALK_SOUTH: &[TilePos] =
        &[TilePos::new(2, 5), TilePos::new(2, 6), TilePos::new(2, 7)];
    pub const STUMBLE_WALK_WEST: &[TilePos] =
        &[TilePos::new(3, 5), TilePos::new(3, 6), TilePos::new(3, 7)];
    pub const STUMBLE_WALK_NORTH: &[TilePos] =
        &[TilePos::new(4, 5), TilePos::new(4, 6), TilePos::new(4, 7)];
    pub const STUMBLE_WALK_EAST: &[TilePos] =
        &[TilePos::new(5, 5), TilePos::new(5, 6), TilePos::new(5, 7)];

    // 3. 练功步态
    pub const KUNGFU_WALK_SOUTH: &[TilePos] = &[TilePos::new(2, 8), TilePos::new(2, 9)];
    pub const KUNGFU_WALK_WEST: &[TilePos] = &[TilePos::new(3, 8), TilePos::new(3, 9)];
    pub const KUNGFU_WALK_NORTH: &[TilePos] = &[TilePos::new(4, 8), TilePos::new(4, 9)];
    pub const KUNGFU_WALK_EAST: &[TilePos] = &[TilePos::new(5, 8), TilePos::new(5, 9)];

    // --- 游戏数值参数 ---
    pub const MOVE_SPEED: f32 = 150.0; // 常规移速
    pub const RUN_SPEED_MULTIPLIER: f32 = 10.0; // 按住 Shift 奔跑的倍率
    pub const MEDITATE_IDLE_DELAY: f32 = 1.5; // 松开键盘几秒后自动打坐
    pub const QI_RECOVERY_RATE: f32 = 0.001; // 每秒聚气速度
    pub const MAX_QI: f32 = 1000.0; // 气海上限
}

// =========================================================================
// 2. 地图地貌与大自然生成参数 (针对 RPGpack_sheet.png 和 Aurora Tileset.png)
// =========================================================================
pub mod map_config {
    use super::TilePos;
    use bevy::prelude::Vec2; // 模块内引入

    // --- RPGpack 图集真实规格 ---
    pub const SHEET_COLUMNS: u32 = 40; // 精确列数：图集有 40 列
    pub const SHEET_ROWS: u32 = 26; // 精确行数：图集有 26 行
    pub const TILE_SIZE: f32 = 32.0; // 游戏世界里一个方块的物理尺寸
    pub const CHUNK_TILES: i32 = 16; // 九宫格优化：一个区块包含 16x16 个瓦片

    // --- 物理碰撞与深度锚点 ---
    // [立体遮挡]：树木的深度判定点在树根接触泥土的地方，而不是树中心
    pub const TREE_ROOT_Y_OFFSET: f32 = -14.0;
    pub const BUSH_ROOT_Y_OFFSET: f32 = -14.0;
    // 树干物理碰撞盒大小：只有 14x8，让玩家能顺滑绕开
    pub const TREE_ROOT_COLLIDER_SIZE: Vec2 = Vec2::new(14.0, 8.0);
    pub const TREE_ROOT_COLLIDER_OFFSET: Vec2 = Vec2::new(16.0, -14.0);

    // --- 自然草地调色板 ---
    // 从 40x26 的图集里挑出纯净平坦的绿草瓦片 (避免切到泥土边)
    pub const CLEAN_GRASS_PALETTE: &[(TilePos, f32)] = &[
        (TilePos::new(5, 7), 1.0),
        (TilePos::new(5, 8), 2.0), // 权重设为 2.0，出现频率最高
        (TilePos::new(5, 9), 1.5),
        (TilePos::new(6, 7), 1.0),
        (TilePos::new(6, 8), 1.0),
    ];

    // --- 融合块 (4个小瓦片拼成一个微景观) ---
    #[derive(Clone, Copy)]
    pub struct BlendPatch4 {
        pub name: &'static str,
        pub top_l: TilePos,
        pub top_r: TilePos, // 左上、右上图块
        pub bot_l: TilePos,
        pub bot_r: TilePos, // 左下、右下图块
    }

    // 草中土坑 (4 个图块拼成一个圆坑)
    pub const INSET_GRASS_WITH_DIRT: BlendPatch4 = BlendPatch4 {
        name: "草中土",
        top_l: TilePos::new(2, 8),
        top_r: TilePos::new(2, 9),
        bot_l: TilePos::new(3, 8),
        bot_r: TilePos::new(3, 9),
    };
    pub const DIRT_PUDDLE_CHANCE: f32 = 0.025; // 泥坑在野外生成的几率

    // --- 大树生成蓝图 (8 瓦片拼合体) ---
    #[derive(Clone, Copy)]
    pub struct TreePrefab8 {
        pub name: &'static str,
        pub top_l: TilePos,
        pub top_r: TilePos, // 树尖 (行 21)
        pub mid_l: TilePos,
        pub mid_r: TilePos, // 树冠上 (行 22)
        pub bot_l: TilePos,
        pub bot_r: TilePos, // 树冠下 (行 23)
        pub trunk_l: TilePos,
        pub trunk_r: TilePos, // 树干 (行 24)
    }

    pub const TREE_LIGHT_GREEN_BIG: TreePrefab8 = TreePrefab8 {
        name: "嫩绿阔叶大树",
        top_l: TilePos::new(21, 1),
        top_r: TilePos::new(21, 2),
        mid_l: TilePos::new(22, 1),
        mid_r: TilePos::new(22, 2),
        bot_l: TilePos::new(23, 1),
        bot_r: TilePos::new(23, 2),
        trunk_l: TilePos::new(24, 1),
        trunk_r: TilePos::new(24, 2),
    };

    pub const TREE_LIGHT_GREEN_SLENDER: TreePrefab8 = TreePrefab8 {
        name: "嫩绿修长细树",
        top_l: TilePos::new(21, 3),
        top_r: TilePos::new(21, 4),
        mid_l: TilePos::new(22, 3),
        mid_r: TilePos::new(22, 4),
        bot_l: TilePos::new(23, 3),
        bot_r: TilePos::new(23, 4),
        trunk_l: TilePos::new(24, 3),
        trunk_r: TilePos::new(24, 4),
    };

    pub const TREE_AUTUMN_BIG: TreePrefab8 = TreePrefab8 {
        name: "金秋阔叶大树",
        top_l: TilePos::new(21, 5),
        top_r: TilePos::new(21, 6),
        mid_l: TilePos::new(22, 5),
        mid_r: TilePos::new(22, 6),
        bot_l: TilePos::new(23, 5),
        bot_r: TilePos::new(23, 6),
        trunk_l: TilePos::new(24, 5),
        trunk_r: TilePos::new(24, 6),
    };

    pub const TREE_AUTUMN_SLENDER: TreePrefab8 = TreePrefab8 {
        name: "金秋修长细树",
        top_l: TilePos::new(21, 7),
        top_r: TilePos::new(21, 8),
        mid_l: TilePos::new(22, 7),
        mid_r: TilePos::new(22, 8),
        bot_l: TilePos::new(23, 7),
        bot_r: TilePos::new(23, 8),
        trunk_l: TilePos::new(24, 7),
        trunk_r: TilePos::new(24, 8),
    };

    pub const TREE_DARK_PINE_BIG: TreePrefab8 = TreePrefab8 {
        name: "苍翠阔叶大松",
        top_l: TilePos::new(21, 9),
        top_r: TilePos::new(21, 10),
        mid_l: TilePos::new(22, 9),
        mid_r: TilePos::new(22, 10),
        bot_l: TilePos::new(23, 9),
        bot_r: TilePos::new(23, 10),
        trunk_l: TilePos::new(24, 9),
        trunk_r: TilePos::new(24, 10),
    };

    pub const TREE_DARK_PINE_SLENDER: TreePrefab8 = TreePrefab8 {
        name: "苍翠修长细松",
        top_l: TilePos::new(21, 11),
        top_r: TilePos::new(21, 12),
        mid_l: TilePos::new(22, 11),
        mid_r: TilePos::new(22, 12),
        bot_l: TilePos::new(23, 11),
        bot_r: TilePos::new(23, 12),
        trunk_l: TilePos::new(24, 11),
        trunk_r: TilePos::new(24, 12),
    };

    // 野外随机生成时从中抽取的树木池
    pub const ACTIVE_TREES: &[TreePrefab8] = &[
        TREE_LIGHT_GREEN_BIG,
        TREE_LIGHT_GREEN_SLENDER,
        TREE_AUTUMN_BIG,
        TREE_AUTUMN_SLENDER,
        TREE_DARK_PINE_BIG,
        TREE_DARK_PINE_SLENDER,
    ];

    // --- 草丛配置 (大草丛 4 参数，小灌木 2 参数) ---
    #[derive(Clone, Copy)]
    pub struct BigBush4 {
        pub name: &'static str,
        pub top_l: TilePos,
        pub top_r: TilePos,
        pub bot_l: TilePos,
        pub bot_r: TilePos,
    }

    pub const BUSH_LIGHT_GREEN_BIG: BigBush4 = BigBush4 {
        name: "嫩绿大草丛",
        top_l: TilePos::new(19, 1),
        top_r: TilePos::new(19, 2),
        bot_l: TilePos::new(20, 1),
        bot_r: TilePos::new(20, 2),
    };
    pub const BUSH_AUTUMN_BIG: BigBush4 = BigBush4 {
        name: "金秋大草丛",
        top_l: TilePos::new(19, 5),
        top_r: TilePos::new(19, 6),
        bot_l: TilePos::new(20, 5),
        bot_r: TilePos::new(20, 6),
    };
    pub const BUSH_DARK_GREEN_BIG: BigBush4 = BigBush4 {
        name: "苍翠大草丛",
        top_l: TilePos::new(19, 9),
        top_r: TilePos::new(19, 10),
        bot_l: TilePos::new(20, 9),
        bot_r: TilePos::new(20, 10),
    };

    #[derive(Clone, Copy)]
    pub struct SmallBush2 {
        pub name: &'static str,
        pub left: TilePos,
        pub right: TilePos,
    }

    pub const BUSH_LIGHT_GREEN_SMALL: SmallBush2 = SmallBush2 {
        name: "嫩绿小灌木",
        left: TilePos::new(20, 3),
        right: TilePos::new(20, 4),
    };
    pub const BUSH_AUTUMN_SMALL: SmallBush2 = SmallBush2 {
        name: "金秋小灌木",
        left: TilePos::new(20, 7),
        right: TilePos::new(20, 8),
    };
    pub const BUSH_DARK_GREEN_SMALL: SmallBush2 = SmallBush2 {
        name: "苍翠小灌木",
        left: TilePos::new(20, 11),
        right: TilePos::new(20, 12),
    };

    #[derive(Clone, Copy)]
    pub enum BushStyle {
        Big(BigBush4),
        Small(SmallBush2),
    }

    pub const ACTIVE_BUSHES: &[BushStyle] = &[
        BushStyle::Big(BUSH_LIGHT_GREEN_BIG),
        BushStyle::Small(BUSH_LIGHT_GREEN_SMALL),
        BushStyle::Big(BUSH_AUTUMN_BIG),
        BushStyle::Small(BUSH_AUTUMN_SMALL),
        BushStyle::Big(BUSH_DARK_GREEN_BIG),
        BushStyle::Small(BUSH_DARK_GREEN_SMALL),
    ];

    pub const TREE_CHANCE: f32 = 0.04;
    pub const BUSH_CHANCE: f32 = 0.035;

    // =====================================================================
    // 【自定义物理碰撞中心】：给 Tiled 手绘的建筑和墙壁挂载精确碰撞
    // =====================================================================
    pub fn get_custom_collider(
        _is_aurora: bool,
        _index: usize,
        _total_columns: u32,
    ) -> (Vec2, Vec2) {
        // 返回格式: (碰撞体宽高 Vec2, 碰撞体相对中心偏移 Vec2)
        // 默认满格 32x32 碰撞墙。以后若需半格阻挡，在这里用 _index 进行匹配返回即可。
        (Vec2::new(32.0, 32.0), Vec2::ZERO)
    }
}
