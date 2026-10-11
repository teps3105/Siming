//! GDD 7.1 參數總表（T2 實作）。常數名與 7.1「代號」相同。

// ---------------------------------------------------------------------------
// 世界生成（GDD 2.2）
// ---------------------------------------------------------------------------

/// 7.1 `WG_SIZE_S`（小）格點寬度 256（GDD 2.2 尺寸：等距圓柱投影，W = 2H）。
pub const WG_SIZE_S_W: usize = 256;
/// 7.1 `WG_SIZE_S`（小）格點高度 128（GDD 2.2 尺寸）。
pub const WG_SIZE_S_H: usize = 128;
/// 7.1 `WG_SIZE_M`（中）格點寬度 384（GDD 2.2 尺寸，預設）。
pub const WG_SIZE_M_W: usize = 384;
/// 7.1 `WG_SIZE_M`（中）格點高度 192（GDD 2.2 尺寸，預設）。
pub const WG_SIZE_M_H: usize = 192;
/// 7.1 `WG_SIZE_L`（大）格點寬度 512（GDD 2.2 尺寸）。
pub const WG_SIZE_L_W: usize = 512;
/// 7.1 `WG_SIZE_L`（大）格點高度 256（GDD 2.2 尺寸）。
pub const WG_SIZE_L_H: usize = 256;

/// 7.1 `WG_PROVINCES_S`（小）目標州郡數 60（GDD 2.2 尺寸）。
pub const WG_PROVINCES_S: u32 = 60;
/// 7.1 `WG_PROVINCES_M`（中）目標州郡數 110（GDD 2.2 尺寸）。
pub const WG_PROVINCES_M: u32 = 110;
/// 7.1 `WG_PROVINCES_L`（大）目標州郡數 180（GDD 2.2 尺寸）。
pub const WG_PROVINCES_L: u32 = 180;

/// 7.1 `WG_GEO_SCALE` 地理解析度 2：世界生成在每軸 2 倍（4 倍格數）的細格上進行，遊戲格由 2×2 降採樣（決策 #35）。
pub const WG_GEO_SCALE: usize = 2;
/// 7.1 `WG_LAND_RATIO` 陸地比例（面積加權），預設 0.40，可調範圍 0.25–0.55（GDD 2.2 步驟 2 海平面）。
pub const WG_LAND_RATIO: f32 = 0.40;
/// 7.1 `WG_RELIEF` 山勢，預設 0.5，範圍 0–1（GDD 2.2 步驟 1 山脈）。
pub const WG_RELIEF: f32 = 0.5;
/// 7.1 `WG_HUMIDITY` 濕潤度，預設 1.0，範圍 0.6–1.4（GDD 2.2 步驟 4 氣候）。
pub const WG_HUMIDITY: f32 = 1.0;

/// 7.1 `WG_LACUNARITY` 每個八度的頻率倍數 2（GDD 2.2 步驟 1 雜訊）。
pub const WG_LACUNARITY: f32 = 2.0;
/// 7.1 `WG_FBM` fBm 增益（每八度振幅倍數）0.5；全部 fBm 與脊狀雜訊共用（GDD 2.2 步驟 1 雜訊）。
pub const WG_FBM_GAIN: f32 = 0.5;
/// 7.1 `WG_FBM` 細節 fBm 的增益 0.6（高八度較強，海岸更碎；GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_FBM_DETAIL_GAIN: f32 = 0.6;
/// 7.1 `WG_FBM` 細節 fBm 的特徵尺度 48 格（基頻 = 1/48 格；GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_FBM_CELLS: f32 = 48.0;
/// 7.1 `WG_FBM` 細節 fBm 的八度數 7（GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_FBM_OCTAVES: u32 = 7;
/// 7.1 `WG_WARP` domain warp 第一層特徵尺度 140 格（GDD 2.2 步驟 1 雜訊）。
pub const WG_WARP1_CELLS: f32 = 140.0;
/// 7.1 `WG_WARP` domain warp 第一層振幅 40 格（GDD 2.2 步驟 1 雜訊）。
pub const WG_WARP1_AMP: f32 = 40.0;
/// 7.1 `WG_WARP` domain warp 第二層特徵尺度 40 格（GDD 2.2 步驟 1 雜訊）。
pub const WG_WARP2_CELLS: f32 = 40.0;
/// 7.1 `WG_WARP` domain warp 第二層振幅 22 格（GDD 2.2 步驟 1 雜訊）。
pub const WG_WARP2_AMP: f32 = 22.0;
/// 7.1 `WG_PLATES_S`（小）板塊數 10（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATES_S: usize = 10;
/// 7.1 `WG_PLATES_M`（中）板塊數 14（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATES_M: usize = 14;
/// 7.1 `WG_PLATES_L`（大）板塊數 18（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATES_L: usize = 18;
/// 7.1 `WG_PLATE_SEP` 板塊中心之間的最小夾角 36°（拒絕取樣；GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_SEP: f32 = 36.0;
/// 7.1 `WG_PLATE_SEP` 拒絕取樣連續失敗 32 次後，最小間距乘 `WG_PLATE_SEP_SHRINK`（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_TRIES: u32 = 32;
/// 7.1 `WG_PLATE_SEP` 間距縮減倍數 0.9（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_SEP_SHRINK: f32 = 0.9;
/// 7.1 `WG_PLATE_CONT_OVER` 大陸板塊面積 = 陸地比例 × 1.0 的球面（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_CONT_OVER: f32 = 1.0;
/// 7.1 `WG_PLATE_SPEED` 板塊速率下限 0.5（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_SPEED_MIN: f32 = 0.5;
/// 7.1 `WG_PLATE_SPEED` 板塊速率上限 1.0（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_SPEED_MAX: f32 = 1.0;
/// 7.1 `WG_PLATE_CONV_FRAC` 聚合／張裂判定：法向相對速度 ≥ 相對速率的 0.2（GDD 2.2 步驟 1 板塊）。
pub const WG_PLATE_CONV_FRAC: f32 = 0.2;
/// 7.1 `WG_PLATE_EDGE` 板塊邊緣門 12 格：大陸層乘 smoothstep(0, 12 × (0.5 + shape), 到板塊邊界的距離)（門寬隨輪廓雜訊起伏，海峽曲折）（GDD 2.2 步驟 1 大陸核）。
pub const WG_PLATE_EDGE: f32 = 12.0;
/// 7.1 `WG_PLATE_EDGE` 碰撞縫合線的門底值 0.5：門 = 0.5 + 0.5 × smoothstep(0, 12, b)（GDD 2.2 步驟 1 大陸核）。
pub const WG_COLLISION_GATE: f32 = 0.5;
/// 7.1 `WG_STRAIT` 補救門檻：最大陸塊占陸地 > 0.6 時切海峽（GDD 2.2 步驟 2 海平面）。
pub const WG_STRAIT_MAX_SHARE: f32 = 0.6;
/// 7.1 `WG_STRAIT` 最多切 6 道海峽（GDD 2.2 步驟 2 海平面）。
pub const WG_STRAIT_TRIES: u32 = 6;
/// 7.1 `WG_STRAIT` 海峽帶寬 8 格（GDD 2.2 步驟 2 海平面）。
pub const WG_STRAIT_W: f32 = 8.0;
/// 7.1 `WG_STRAIT` 海峽下陷 0.5 × exp(−(b/8)²)（GDD 2.2 步驟 2 海平面）。
pub const WG_STRAIT_DEPTH: f32 = 0.5;
/// 7.1 `WG_SHELF` 大陸棚寬 2 格 × (0.5 + shape)（被動邊緣；GDD 2.2 步驟 1 海床）。
pub const WG_SHELF_W: f32 = 2.0;
/// 7.1 `WG_SHELF` 隱沒邊緣（海溝側）的大陸棚寬倍數 0.3（GDD 2.2 步驟 1 海床）。
pub const WG_SHELF_SUBDUCTION: f32 = 0.3;
/// 7.1 `WG_SHELF` 大陸棚深度：海平面下 0.02 起、到棚緣 0.05（GDD 2.2 步驟 1 海床）。
pub const WG_SHELF_DEPTH: f32 = 0.02;
/// 7.1 `WG_SHELF` 棚緣深度 0.05（GDD 2.2 步驟 1 海床）。
pub const WG_SHELF_EDGE_DEPTH: f32 = 0.05;
/// 7.1 `WG_SHELF` 棚寬依最近陸塊面積縮窄：面積 ≥ 300 格為全寬，否則乘 sqrt(面積/300)（下限 0.25；GDD 2.2 步驟 1 海床）。
pub const WG_SHELF_FULL_AREA: f32 = 300.0;
/// 7.1 `WG_SLOPE_W` 大陸坡寬 4 格（棚緣到深海平原；GDD 2.2 步驟 1 海床）。
pub const WG_SLOPE_W: f32 = 4.0;
/// 7.1 `WG_ABYSS` 深海平原深度 0.35（海平面下；GDD 2.2 步驟 1 海床）。
pub const WG_ABYSS_DEPTH: f32 = 0.35;
/// 7.1 `WG_ABYSS` 深海平原起伏 ±0.03（細節雜訊；GDD 2.2 步驟 1 海床）。
pub const WG_ABYSS_NOISE: f32 = 0.03;
/// 7.1 `WG_OCEAN_RIDGE` 洋脊自深海平原隆起 0.15 × g(b, 6)（GDD 2.2 步驟 1 海床）。
pub const WG_RIDGE_RISE: f32 = 0.15;
/// 7.1 `WG_TRENCH` 海溝自深海平原再深 0.10 × g(b, 4)（GDD 2.2 步驟 1 海床）。
pub const WG_TRENCH_DEEPEN: f32 = 0.10;
/// 7.1 `WG_ARC` 島弧淺灘：仰衝側帶內深度上限 0.06（GDD 2.2 步驟 1 海床）。
pub const WG_ARC_BANK_DEPTH: f32 = 0.06;
/// 7.1 `WG_ENCLOSED_SEA` 面積 < 150 格的封閉海深度上限 0.08（GDD 2.2 步驟 1 海床）。
pub const WG_ENCLOSED_SEA_AREA: f32 = 150.0;
/// 7.1 `WG_ENCLOSED_SEA` 封閉海深度上限 0.08（GDD 2.2 步驟 1 海床）。
pub const WG_ENCLOSED_SEA_DEPTH: f32 = 0.08;
/// 7.1 `WG_BIOME_SMOOTH` 溫度與濕度場的方框平滑半徑 2 格（遊戲格；GDD 2.2 步驟 4 氣候）。
pub const WG_BIOME_SMOOTH: usize = 2;
/// 7.1 `WG_BIOME_MIN_PATCH` 氣候地形（平原、森林、沙漠、沼澤、雪原、丘陵）的最小斑塊 12 格（遊戲格面積；細格乘解析度²；
/// 更小者併入周圍最多的地形；GDD 2.2 步驟 5 地形）。
pub const WG_BIOME_MIN_PATCH: u32 = 12;
/// 7.1 `WG_CORE_OFFSET` 核中心偏離板塊中心的距離上限 0.35 × 板塊等面積圓半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_CORE_OFFSET: f32 = 0.35;
/// 7.1 `WG_CORE_RADIUS` 核半徑下限 0.7 × 板塊等面積圓半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_CORE_RADIUS_MIN: f32 = 0.7;
/// 7.1 `WG_CORE_RADIUS` 核半徑上限 0.95 × 板塊等面積圓半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_CORE_RADIUS_MAX: f32 = 0.95;
/// 7.1 `WG_CORE_LOBES` 每核最少葉數 2（GDD 2.2 步驟 1 大陸核）。
pub const WG_CORE_LOBES_MIN: usize = 2;
/// 7.1 `WG_CORE_LOBES` 每核最多葉數 4（GDD 2.2 步驟 1 大陸核）。
pub const WG_CORE_LOBES_MAX: usize = 4;
/// 7.1 `WG_LOBE_OFFSET` 葉中心偏離核中心的距離下限 0.4 × 核半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_LOBE_OFFSET_MIN: f32 = 0.4;
/// 7.1 `WG_LOBE_OFFSET` 葉中心偏離核中心的距離上限 1.0 × 核半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_LOBE_OFFSET_MAX: f32 = 1.0;
/// 7.1 `WG_LOBE_RADIUS` 葉半徑下限 0.5 × 核半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_LOBE_RADIUS_MIN: f32 = 0.5;
/// 7.1 `WG_LOBE_RADIUS` 葉半徑上限 0.8 × 核半徑（GDD 2.2 步驟 1 大陸核）。
pub const WG_LOBE_RADIUS_MAX: f32 = 0.8;
/// 7.1 `WG_SHAPE` 大陸輪廓調變雜訊特徵尺度 70 格（GDD 2.2 步驟 1 大陸核）。
pub const WG_SHAPE_CELLS: f32 = 70.0;
/// 7.1 `WG_SHAPE` 大陸輪廓調變雜訊八度數 3（GDD 2.2 步驟 1 大陸核）。
pub const WG_SHAPE_OCTAVES: u32 = 3;
/// 7.1 `WG_SHAPE` 大陸層乘 (0.7 + 0.6·shape)：底值 0.7（GDD 2.2 步驟 1 大陸核）。
pub const WG_SHAPE_BASE: f32 = 0.7;
/// 7.1 `WG_SHAPE` 大陸層乘 (0.7 + 0.6·shape)：幅度 0.6（GDD 2.2 步驟 1 大陸核）。
pub const WG_SHAPE_SPAN: f32 = 0.6;
/// 7.1 `WG_POLAR_FADE` 大陸核振幅自核中心緯度 60° 起衰減（GDD 2.2 步驟 1 大陸核）。
pub const WG_POLAR_FADE_LAT: f32 = 60.0;
/// 7.1 `WG_POLAR_FADE` 到緯度 85° 衰減至 (1 − 0.7)（GDD 2.2 步驟 1 大陸核）。
pub const WG_POLAR_FADE_END: f32 = 85.0;
/// 7.1 `WG_POLAR_FADE` 極區衰減量 0.7（GDD 2.2 步驟 1 大陸核）。
pub const WG_POLAR_FADE_AMT: f32 = 0.7;
/// 7.1 `WG_CONT_WEIGHT` 大陸層權重 0.42：base = 0.42·大陸層 + 0.58·細節 + 島群（GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_CONT_WEIGHT: f32 = 0.42;
/// 7.1 `WG_ISLAND` 島群雜訊特徵尺度 40 格（GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_ISLAND_CELLS: f32 = 40.0;
/// 7.1 `WG_ISLAND` 島群雜訊八度數 4（GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_ISLAND_OCTAVES: u32 = 4;
/// 7.1 `WG_ISLAND` 島群門檻：smoothstep(0.68, 1.0, 島群雜訊)（GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_ISLAND_LO: f32 = 0.68;
/// 7.1 `WG_ISLAND` 島群抬升 0.35（GDD 2.2 步驟 1 大陸與海岸）。
pub const WG_ISLAND_AMP: f32 = 0.35;
/// 7.1 `WG_LOWLAND_RISE` 內陸壓平：陸地高度的低地項 = 0.15·lb（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_LOWLAND_RISE: f32 = 0.15;
/// 7.1 `WG_INLAND_GATE` 內陸門：山脈與丘陵乘 smoothstep(0, 0.12, lb)（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_INLAND_GATE: f32 = 0.12;
/// 7.1 `WG_OROGENY` 造山雜訊特徵尺度 130 格（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_OROGENY_CELLS: f32 = 130.0;
/// 7.1 `WG_OROGENY` 造山雜訊（與丘陵帶遮罩）八度數 3（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_OROGENY_OCTAVES: u32 = 3;
/// 7.1 `WG_OROGENY` 造山帶等值線 A = 0.35（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_OROGENY_ISO_A: f32 = 0.35;
/// 7.1 `WG_OROGENY` 造山帶等值線 B = 0.68（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_OROGENY_ISO_B: f32 = 0.68;
/// 7.1 `WG_OROGENY` 造山帶半寬 0.18（雜訊值；帶 = 1 − 到最近等值線的距離 / 0.18；GDD 2.2 步驟 1）。
pub const WG_OROGENY_HALF: f32 = 0.18;
/// 7.1 `WG_OROGENY` 帶心門檻 0.25：帶值 smoothstep(0.25, 1) 為山脈、其下為山麓（GDD 2.2 步驟 1）。
pub const WG_OROGENY_CORE: f32 = 0.25;
/// 7.1 `WG_UPLIFT` 陸陸聚合的造山帶強度 1.0（GDD 2.2 步驟 1 造山帶）。
pub const WG_UPLIFT_COLLISION: f32 = 1.0;
/// 7.1 `WG_UPLIFT` 洋陸聚合大陸側海岸山脈的造山帶強度 0.85（GDD 2.2 步驟 1 造山帶）。
pub const WG_UPLIFT_COAST: f32 = 0.85;
/// 7.1 `WG_BAND` 主山脈帶寬 10 格：帶 = exp(−(b/w)²)（GDD 2.2 步驟 1 造山帶）。
pub const WG_BAND_COLLISION: f32 = 10.0;
/// 7.1 `WG_BAND` 海岸山脈帶寬 7 格（GDD 2.2 步驟 1 造山帶）。
pub const WG_BAND_COAST: f32 = 7.0;
/// 7.1 `WG_BAND` 海岸山脈帶中心自板塊邊界內移 12 格（山脈在岸上；GDD 2.2 步驟 1 造山帶）。
pub const WG_BAND_COAST_INSET: f32 = 12.0;
/// 7.1 `WG_BAND` 島弧帶寬 4 格（GDD 2.2 步驟 1 海洋邊界）。
pub const WG_BAND_ARC: f32 = 4.0;
/// 7.1 `WG_BAND` 海溝帶寬 5 格（GDD 2.2 步驟 1 海洋邊界）。
pub const WG_BAND_TRENCH: f32 = 5.0;
/// 7.1 `WG_BAND` 裂谷／洋脊帶寬 6 格（GDD 2.2 步驟 1 海洋邊界）。
pub const WG_BAND_RIFT: f32 = 6.0;
/// 7.1 `WG_ARC` 島弧抬升 0.20 × 帶 × 脊狀雜訊 × 斷續門（基底；GDD 2.2 步驟 1 海洋邊界）。
pub const WG_ARC: f32 = 0.20;
/// 7.1 `WG_ARC` 島弧斷續門 smoothstep(0.5, 0.8, 造山雜訊) 的下限（只在雜訊高處成島鏈；GDD 2.2 步驟 1 海洋邊界）。
pub const WG_ARC_ORO_LO: f32 = 0.5;
/// 7.1 `WG_ARC` 島弧斷續門上限 0.8（GDD 2.2 步驟 1 海洋邊界）。
pub const WG_ARC_ORO_HI: f32 = 0.8;
/// 7.1 `WG_TRENCH` 海溝下陷 0.08（基底；GDD 2.2 步驟 1 海洋邊界）。
pub const WG_TRENCH: f32 = 0.08;
/// 7.1 `WG_RIFT` 陸陸張裂的裂谷下陷 0.20（基底；GDD 2.2 步驟 1 海洋邊界）。
pub const WG_RIFT: f32 = 0.20;
/// 7.1 `WG_TRANSFORM_RIFT` 陸陸轉形的淺裂谷 = 裂谷的 0.4 倍（GDD 2.2 步驟 1 海洋邊界）。
pub const WG_TRANSFORM_RIFT: f32 = 0.4;
/// 7.1 `WG_OCEAN_RIDGE` 洋脊抬升 0.05（基底；GDD 2.2 步驟 1 海洋邊界）。
pub const WG_OCEAN_RIDGE: f32 = 0.05;
/// 7.1 `WG_BELT_ORO` 板塊聚合帶與島弧沿帶斷續：乘 smoothstep(0.25, 0.6, 造山雜訊)，下限 0.25（GDD 2.2 步驟 1 造山帶）。
pub const WG_BELT_ORO_LO: f32 = 0.25;
/// 7.1 `WG_BELT_ORO` 斷續門上限 0.6（GDD 2.2 步驟 1 造山帶）。
pub const WG_BELT_ORO_HI: f32 = 0.6;
/// 7.1 `WG_INTRAPLATE` 板塊內造山帶（雜訊等值線帶）的強度 0.8（GDD 2.2 步驟 1 造山帶）。
pub const WG_INTRAPLATE: f32 = 0.8;
/// 7.1 `WG_FOOTHILL` 板塊聚合帶的山麓帶寬 = 帶寬 × 3.0（GDD 2.2 步驟 1 造山帶）。
pub const WG_FOOTHILL_BAND: f32 = 3.0;
/// 7.1 `WG_FOOTHILL` 山麓振幅 0.20：山麓項 = 0.20 × smoothstep(0, 0.25, 帶) × (1 − 帶心) × 內陸門 × 起伏（GDD 2.2 步驟 1）。
pub const WG_FOOTHILL_AMP: f32 = 0.20;
/// 7.1 `WG_RIDGE` 脊狀多重碎形特徵尺度 24 格（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_RIDGE_CELLS: f32 = 24.0;
/// 7.1 `WG_RIDGE` 脊狀多重碎形八度數 5（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_RIDGE_OCTAVES: u32 = 5;
/// 7.1 `WG_RIDGE` 脊狀雜訊的銳化倍數 1.3：每八度取 (1 − |1.3·n|)²（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_RIDGE_SHARPEN: f32 = 1.3;
/// 7.1 `WG_RIDGE` 多重碎形的八度間加權倍數 2.0（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_RIDGE_WEIGHT: f32 = 2.0;
/// 7.1 `WG_MOUNTAIN_AMP` 山脈振幅 1.7：山脈項 = 1.7 × relief × 2 × 造山帶 × 內陸門 × ridged²（GDD 2.2 步驟 1）。
pub const WG_MOUNTAIN_AMP: f32 = 1.7;
/// 7.1 `WG_RELIEF_SCALE` 山脈項乘 relief × 2（預設山勢 0.5 時為 1；GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_RELIEF_SCALE: f32 = 2.0;
/// 7.1 `WG_HILL_MASK` 丘陵帶遮罩特徵尺度 110 格（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_HILL_MASK_CELLS: f32 = 110.0;
/// 7.1 `WG_HILL_MASK` 丘陵帶遮罩 smoothstep 下限 0.55（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_HILL_MASK_LO: f32 = 0.55;
/// 7.1 `WG_HILL_MASK` 丘陵帶遮罩 smoothstep 上限 0.80（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_HILL_MASK_HI: f32 = 0.80;
/// 7.1 `WG_HILL` 丘陵起伏特徵尺度 22 格（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_HILL_CELLS: f32 = 22.0;
/// 7.1 `WG_HILL` 丘陵起伏八度數 3（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_HILL_OCTAVES: u32 = 3;
/// 7.1 `WG_HILL` 丘陵振幅 0.08：丘陵項 = 0.08 × 丘陵帶 × 內陸門 × 起伏（GDD 2.2 步驟 1 山脈與丘陵）。
pub const WG_HILL_AMP: f32 = 0.08;
/// 7.1 `WG_EROSION_ITERS_S`（小）侵蝕迭代數 10（GDD 2.2 步驟 1 侵蝕；6.8 效能預算）。
pub const WG_EROSION_ITERS_S: u32 = 10;
/// 7.1 `WG_EROSION_ITERS_M`（中）侵蝕迭代數 6（GDD 2.2 步驟 1 侵蝕）。
pub const WG_EROSION_ITERS_M: u32 = 6;
/// 7.1 `WG_EROSION_ITERS_L`（大）侵蝕迭代數 8（GDD 2.2 步驟 1 侵蝕）。
pub const WG_EROSION_ITERS_L: u32 = 8;
/// 7.1 `WG_EROSION_ROUTE_EVERY` 侵蝕的流向（Priority-Flood）每 2 次迭代重算一次（GDD 2.2 步驟 1 侵蝕）。
pub const WG_EROSION_ROUTE_EVERY: u32 = 2;
/// 7.1 `WG_EROSION_DT` 侵蝕時間步 1.0（GDD 2.2 步驟 1 侵蝕）。
pub const WG_EROSION_DT: f32 = 1.0;
/// 7.1 `WG_EROSION_K` 侵蝕係數 0.02：`f = K·sqrt(A)·dt / d`（m = 0.5、n = 1；GDD 2.2 步驟 1 侵蝕）。
pub const WG_EROSION_K: f32 = 0.02;
/// 7.1 `WG_EROSION_UPLIFT` 隆起率 0.01：每步抬升 `0.01 × 隆起圖 × dt`（GDD 2.2 步驟 1 侵蝕）。
pub const WG_EROSION_UPLIFT: f32 = 0.01;
/// 7.1 `WG_RANGE_SEGMENT` 山脈分段間距 48 格：命名時以球面上間距約 48 格的固定點集分段（GDD 2.2 步驟 9）。
pub const WG_RANGE_SEGMENT: f32 = 48.0;
/// 7.1 `WG_CLIMATE_NOISE` 氣候雜訊（溫度、濕度）特徵尺度 110 格（GDD 2.2 步驟 4 氣候）。
pub const WG_CLIMATE_CELLS: f32 = 110.0;
/// 7.1 `WG_CLIMATE_NOISE` 溫度雜訊八度數 3（濕度為 4；GDD 2.2 步驟 4 氣候）。
pub const WG_CLIMATE_OCTAVES: u32 = 3;
/// 7.1 `WG_DRY_BELT` 乾燥帶緯度擺動雜訊的特徵尺度 160 格（GDD 2.2 步驟 4 氣候）。
pub const WG_DRY_WOBBLE_CELLS: f32 = 160.0;
/// 7.1 `WG_DRY_BELT` 乾燥帶緯度擺動雜訊的八度數 2（GDD 2.2 步驟 4 氣候）。
pub const WG_DRY_WOBBLE_OCTAVES: u32 = 2;
/// 7.1 `WG_MOUNTAIN_H` 山脈高度門檻 0.55（陸地高度，GDD 2.2 步驟 5 地形）。
pub const WG_MOUNTAIN_H: f32 = 0.55;
/// 7.1 `WG_HILL_H` 丘陵高度門檻 0.25（陸地高度，GDD 2.2 步驟 5 地形）。
pub const WG_HILL_H: f32 = 0.25;

/// 7.1 `WG_TEMP` 赤道溫度 1.0（t = 1.0 − 1.1·|lat|/90° − 0.3·陸高 + 0.22·(noise − 0.5)；GDD 2.2 步驟 4 氣候）。
pub const WG_TEMP_EQUATOR: f32 = 1.0;
/// 7.1 `WG_TEMP` 緯度降溫係數 1.1（乘 |lat|/90°；GDD 2.2 步驟 4 氣候）。
pub const WG_TEMP_LAT_COEF: f32 = 1.1;
/// 7.1 `WG_TEMP` 陸地高度降溫係數 0.3（GDD 2.2 步驟 4 氣候）。
pub const WG_TEMP_ALT_COEF: f32 = 0.30;
/// 7.1 `WG_TEMP` 溫度雜訊係數 0.22，乘 (noise − 0.5)（GDD 2.2 步驟 4 氣候）。
pub const WG_TEMP_NOISE_COEF: f32 = 0.22;

/// 7.1 `WG_WIND` 信風帶上限緯度 30°：|lat| < 30° 風自東來（GDD 2.2 步驟 4 氣候）。
pub const WG_WIND_TRADE_LAT: f32 = 30.0;
/// 7.1 `WG_WIND` 西風帶上限緯度 60°：30°–60° 風自西來，> 60° 極地東風（GDD 2.2 步驟 4 氣候）。
pub const WG_WIND_WESTERLY_LAT: f32 = 60.0;
/// 7.1 `WG_MOIST` 氣團濕度的起始值 0.5（第一圈掃描只為建立狀態；GDD 2.2 步驟 4 氣候）。
pub const WG_MOIST_START: f32 = 0.5;
/// 7.1 `WG_MOIST` 氣團經過海格每格回濕 +0.15（GDD 2.2 步驟 4 氣候）。
pub const WG_MOIST_SEA_GAIN: f32 = 0.15;
/// 7.1 `WG_MOIST` 氣團經過陸格每格失濕 −0.012（GDD 2.2 步驟 4 氣候）。
pub const WG_MOIST_LAND_LOSS: f32 = 0.012;
/// 7.1 `WG_OROG` 空氣含水上限隨陸高降低：cap = 1 − 0.9 × 陸高（mapgen4 地形雨；GDD 2.2 步驟 4 氣候）。
pub const WG_OROG_CAP: f32 = 0.9;
/// 7.1 `WG_OROG` 超過含水上限的部分每格降下 0.5（rain = 0.5 × (m − cap)；GDD 2.2 步驟 4 氣候）。
pub const WG_OROG_RAIN: f32 = 0.5;
/// 7.1 `WG_WIND` 風帶邊界的混合半寬 4°（GDD 2.2 步驟 4 氣候）。
pub const WG_WIND_BLEND: f32 = 4.0;
/// 7.1 `WG_MOIST` 氣團濕度的南北平滑半徑 4 列（取 9 列平均，消除逐列掃描的條紋；GDD 2.2 步驟 4 氣候）。
pub const WG_MOIST_SMOOTH_ROWS: usize = 4;
/// 7.1 `WG_HUMID_MIX` 濕度的雜訊權重 0.4（q = 0.4·noise + 0.4·氣團濕度 + 0.2·coast − 乾燥帶；GDD 2.2 步驟 4）。
pub const WG_HUMID_NOISE_WEIGHT: f32 = 0.4;
/// 7.1 `WG_HUMID_MIX` 濕度的氣團權重 0.4（GDD 2.2 步驟 4 氣候）。
pub const WG_HUMID_MOIST_WEIGHT: f32 = 0.4;
/// 7.1 `WG_HUMID_MIX` 濕度的海岸權重 0.2（GDD 2.2 步驟 4 氣候）。
pub const WG_HUMID_COAST_WEIGHT: f32 = 0.2;
/// 7.1 `WG_COAST_DECAY` 濕度的海岸衰減距離 10 格，exp(−d/10)（GDD 2.2 步驟 4 氣候）。
pub const WG_COAST_DECAY: f32 = 10.0;
/// 7.1 `WG_DRY_BELT` 副熱帶乾燥帶的濕度減量 0.12（q −= 0.12·exp(−((|lat| − 25°)/20°)²)·片狀係數；GDD 2.2 步驟 4）。
pub const WG_DRY_BELT_DEPTH: f32 = 0.12;
/// 7.1 `WG_DRY_BELT` 副熱帶乾燥帶中心緯度 25°（GDD 2.2 步驟 4）。
pub const WG_DRY_BELT_LAT: f32 = 25.0;
/// 7.1 `WG_DRY_BELT` 副熱帶乾燥帶半寬 20°（GDD 2.2 步驟 4）。
pub const WG_DRY_BELT_HALF: f32 = 20.0;
/// 7.1 `WG_DRY_BELT` 乾燥帶強度隨濕度雜訊起伏：乘 (0.3 + 1.4·noise)，底值 0.3（沙漠沿帶呈片狀；GDD 2.2 步驟 4）。
pub const WG_DRY_BELT_PATCH_BASE: f32 = 0.3;
/// 7.1 `WG_DRY_BELT` 乾燥帶強度隨濕度雜訊起伏的幅度 1.4（GDD 2.2 步驟 4）。
pub const WG_DRY_BELT_PATCH_SPAN: f32 = 1.4;
/// 7.1 `WG_DRY_BELT` 乾燥帶中心緯度的擺動幅度 ±10°，由低頻雜訊決定（GDD 2.2 步驟 4）。
pub const WG_DRY_BELT_WOBBLE: f32 = 10.0;

/// 7.1 `WG_SNOW_T` 雪原溫度門檻 0.23，t 低於此為雪原（GDD 2.2 步驟 5 地形）。
pub const WG_SNOW_T: f32 = 0.23;
/// 7.1 `WG_DESERT_Q` 沙漠濕度門檻 0.22，q 低於此為沙漠（GDD 2.2 步驟 5 地形）。
pub const WG_DESERT_Q: f32 = 0.22;
/// 7.1 `WG_DESERT_T` 沙漠溫度門檻 0.3，t 高於此為沙漠（GDD 2.2 步驟 5 地形）。
pub const WG_DESERT_T: f32 = 0.3;
/// 7.1 `WG_SWAMP_Q` 沼澤濕度門檻 0.74，q 高於此為沼澤（GDD 2.2 步驟 5 地形）。
pub const WG_SWAMP_Q: f32 = 0.74;
/// 7.1 `WG_SWAMP_H` 沼澤陸地高度上限 0.12（GDD 2.2 步驟 5 地形）。
pub const WG_SWAMP_H: f32 = 0.12;
/// 7.1 `WG_FOREST_Q` 森林濕度門檻 0.55，q 高於此為森林（GDD 2.2 步驟 5 地形）。
pub const WG_FOREST_Q: f32 = 0.55;

/// 7.1 `WG_CONTINENT_MIN` 命名陸塊的最小面積 120（赤道格；GDD 2.2 步驟 9 大洲）。
pub const WG_CONTINENT_MIN: f32 = 120.0;
/// 7.1 `WG_CONTINENT_ZHOU` 稱「洲」的陸塊面積 ≥ 陸地的 5%（GDD 7.3 大洲命名）。
pub const WG_CONTINENT_ZHOU: f32 = 0.05;

/// 7.1 `WG_LAKE_MIN` 湖的最小面積 6 格（GDD 2.2 步驟 3 填窪與湖）。
pub const WG_LAKE_MIN: u32 = 6;
/// 7.1 `WG_LAKE_DEPTH` 湖的最小水深 0.01（填平後高度 − 原高度；GDD 2.2 步驟 3 填窪與湖）。
pub const WG_LAKE_DEPTH: f32 = 0.01;

/// 7.1 `WG_JIANG_MIN_LEN` 命名為「江」的幹流最短長度 20 格（較短的江級幹流不命名；GDD 2.2 步驟 6）。
pub const WG_JIANG_MIN_LEN: usize = 20;
/// 7.1 `WG_RIVER_Q` 小河分位 0.10，即陸地累積量前 10%（GDD 2.2 步驟 6 河流）。
pub const WG_RIVER_Q_SMALL: f32 = 0.10;
/// 7.1 `WG_RIVER_Q` 大河分位 0.05，即陸地累積量前 5%（GDD 2.2 步驟 6 河流）。
pub const WG_RIVER_Q_LARGE: f32 = 0.05;
/// 7.1 `WG_RIVER_Q` 江分位 0.015，即陸地累積量前 1.5%（GDD 2.2 步驟 6 河流）。
pub const WG_RIVER_Q_JIANG: f32 = 0.015;

/// 7.1 `WG_PROV_COST` 州郡成長基本代價 1（GDD 2.2 步驟 7 州郡）。
pub const WG_PROV_COST_BASE: f32 = 1.0;
/// 7.1 `WG_PROV_COST` 丘陵額外代價 +1.5（GDD 2.2 步驟 7 州郡）。
pub const WG_PROV_COST_HILL: f32 = 1.5;
/// 7.1 `WG_PROV_COST` 山脈額外代價 +3（GDD 2.2 步驟 7 州郡）。
pub const WG_PROV_COST_MOUNTAIN: f32 = 3.0;
/// 7.1 `WG_PROV_COST` 跨越大河或江的額外代價 +6（GDD 2.2 步驟 7 州郡）。
pub const WG_PROV_COST_CROSS_RIVER: f32 = 6.0;

/// 7.1 `WG_ISLAND_MIN` 獨立州郡島嶼的最小面積 120 格（GDD 2.2 步驟 7 小島）。
pub const WG_ISLAND_MIN: u32 = 120;

/// 7.1 `WG_PROVINCE_MIN_RATIO` 過小州郡的面積比例 0.35，相對平均面積（GDD 2.2 步驟 7 合併過小者）。
pub const WG_PROVINCE_MIN_RATIO: f32 = 0.35;
/// 7.1 `WG_PROVINCE_MIN_CELLS` 州郡的最小面積 30 格（GDD 2.2 品質檢查）。
pub const WG_PROVINCE_MIN_CELLS: u32 = 30;

/// 7.1 `WG_SEA_LANE` 海鄰判定距離 12 格（GDD 2.2 步驟 7 相鄰）。
pub const WG_SEA_LANE: u32 = 12;

/// 7.1 `WG_NODE_RATIO` 洞天福地數量比例 0.2，乘以州郡數（GDD 2.4 節點）。
pub const WG_NODE_RATIO: f32 = 0.2;
/// 7.1 `WG_NODE_MIN_DIST` 洞天福地彼此最小距離 18 格（GDD 2.4 節點）。
pub const WG_NODE_MIN_DIST: u32 = 18;

/// 7.1 `WG_NODE_GRADE` 大洞天的靈性分位比例 0.15，即前 15%（GDD 2.4 節點等級）。
pub const WG_NODE_GRADE_TOP_RATIO: f32 = 0.15;
/// 7.1 `WG_NODE_GRADE` 洞天的靈性分位比例 0.35，即接下來 35%（GDD 2.4 節點等級）。
pub const WG_NODE_GRADE_MID_RATIO: f32 = 0.35;
/// 7.1 `WG_NODE_GRADE` 大洞天的強度 s = 1.0（GDD 2.4 節點等級）。
pub const WG_NODE_STRENGTH_GREAT: f32 = 1.0;
/// 7.1 `WG_NODE_GRADE` 洞天的強度 s = 0.75（GDD 2.4 節點等級）。
pub const WG_NODE_STRENGTH_CAVE: f32 = 0.75;
/// 7.1 `WG_NODE_GRADE` 福地的強度 s = 0.5（GDD 2.4 節點等級）。
pub const WG_NODE_STRENGTH_BLESSED: f32 = 0.5;

/// 7.1 `WG_LEY_COST` 靈脈路徑在山脈的代價 0.5（GDD 2.4 靈脈 A* 代價）。
pub const WG_LEY_COST_MOUNTAIN: f32 = 0.5;
/// 7.1 `WG_LEY_COST` 靈脈路徑在丘陵的代價 0.8（GDD 2.4 靈脈 A* 代價）。
pub const WG_LEY_COST_HILL: f32 = 0.8;
/// 7.1 `WG_LEY_COST` 靈脈路徑在其他陸地的代價 1.5（GDD 2.4 靈脈 A* 代價）。
pub const WG_LEY_COST_LAND: f32 = 1.5;
/// 7.1 `WG_LEY_COST` 靈脈路徑在水上的代價 4（GDD 2.4 靈脈 A* 代價）。
pub const WG_LEY_COST_WATER: f32 = 4.0;

/// 7.1 `WG_RANGE_MIN_CELLS` 一條已命名山脈的最少格數 12（GDD 2.2 步驟 9）。
pub const WG_RANGE_MIN_CELLS: usize = 12;
/// 7.1 `WG_SPIRIT` 靈性 = 0.6 × 高度 + 0.4 × 脊狀雜訊：高度權重 0.6（GDD 2.4 節點）。
pub const WG_SPIRIT_HEIGHT_WEIGHT: f32 = 0.6;
/// 7.1 `WG_SPIRIT` 靈性的脊狀雜訊權重 0.4（GDD 2.4 節點）。
pub const WG_SPIRIT_RIDGE_WEIGHT: f32 = 0.4;
/// 7.1 `WG_NAME_REDRAW_MAX` 命名撞名時最多重抽 20 次（GDD 7.3）。
pub const WG_NAME_REDRAW_MAX: usize = 20;
/// 7.1 `WG_NAME_BIG_PROVINCE_TOP` 州郡命名：面積前 25% 者稱「州」候選（GDD 7.3）。
pub const WG_NAME_BIG_PROVINCE_TOP: f32 = 0.25;
/// 7.1 `WG_QC_LAND_TOL` 品質檢查：陸地比例誤差 ≤ 0.02（GDD 2.2 品質檢查）。
pub const WG_QC_LAND_TOL: f32 = 0.02;
/// 7.1 `WG_QC_MAIN_LANDMASS` 品質檢查：最大陸塊 ≤ 陸地的 0.70（GDD 2.2 品質檢查）。
pub const WG_QC_MAIN_LANDMASS: f32 = 0.70;
/// 7.1 `WG_QC_CONTINENTS` 品質檢查：至少 2 塊大洲（GDD 2.2 品質檢查）。
pub const WG_QC_CONTINENTS: usize = 2;
/// 7.1 `WG_QC_CONTINENT_SHARE` 品質檢查：大洲各 ≥ 陸地的 5%（GDD 2.2 品質檢查）。
pub const WG_QC_CONTINENT_SHARE: f32 = 0.05;
/// 7.1 `WG_QC_PROVINCE_TOL` 品質檢查：州郡數在目標 ±15% 內（GDD 2.2 品質檢查）。
pub const WG_QC_PROVINCE_TOL: f32 = 0.15;
/// 7.1 `WG_QC_MIN_JIANG` 品質檢查：至少 3 條入海的江（GDD 2.2 品質檢查）。
pub const WG_QC_MIN_JIANG: usize = 3;
/// 7.1 `WG_QC_MIN_NODES` 品質檢查：洞天福地 ≥ 8（GDD 2.2 品質檢查）。
pub const WG_QC_MIN_NODES: usize = 8;
/// 7.1 `WG_RETRY` 品質檢查失敗時的最多重試次數 8（GDD 2.2 品質檢查）。
pub const WG_RETRY: u32 = 8;

// ---------------------------------------------------------------------------
// 州郡與人口（GDD 2.3）
// ---------------------------------------------------------------------------

/// 7.1 `HAB_TERRAIN` 各地形宜居度，依地形代碼順序排列：
/// 海 0、湖 0、平原 1.0、丘陵 0.4、山脈 0.1、森林 0.5、雪原 0.05、沙漠 0.05、沼澤 0.2
/// （GDD 2.3 宜居度）。
pub const HAB_TERRAIN: [f64; 9] = [0.0, 0.0, 1.0, 0.4, 0.1, 0.5, 0.05, 0.05, 0.2];

/// 7.1 `HAB_RIVER` 鄰接小河的宜居加成 0.3（GDD 2.3 宜居度加成）。
pub const HAB_RIVER_SMALL: f64 = 0.3;
/// 7.1 `HAB_RIVER` 鄰接大河的宜居加成 0.6（GDD 2.3 宜居度加成）。
pub const HAB_RIVER_LARGE: f64 = 0.6;
/// 7.1 `HAB_RIVER` 鄰接江的宜居加成 0.8（GDD 2.3 宜居度加成）。
pub const HAB_RIVER_JIANG: f64 = 0.8;
/// 7.1 `HAB_COAST` 鄰接海或湖的宜居加成 0.3（GDD 2.3 宜居度加成）。
pub const HAB_COAST: f64 = 0.3;

/// 7.1 `POP_INIT_PER_HAB` 每單位宜居度的初始人口基數 1500（GDD 2.3 人口）。
pub const POP_INIT_PER_HAB: f64 = 1500.0;
/// 7.1 `POP_INIT_PER_HAB` 上古開局時代係數 0.3（GDD 2.3 人口；對應 Era::Ancient）。
pub const POP_INIT_ERA_ANCIENT: f64 = 0.3;
/// 7.1 `POP_INIT_PER_HAB` 中古開局時代係數 1.0（GDD 2.3 人口；對應 Era::Middle）。
pub const POP_INIT_ERA_MIDDLE: f64 = 1.0;
/// 7.1 `POP_INIT_PER_HAB` 末法開局時代係數 1.2（GDD 2.3 人口；對應 Era::Decline）。
pub const POP_INIT_ERA_DECLINE: f64 = 1.2;

/// 7.1 `POP_CAP_PER_HAB` 每單位宜居度的人口承載基數 3000（GDD 2.3 人口承載）。
pub const POP_CAP_PER_HAB: f64 = 3000.0;
/// 7.1 `CAP_DEVASTATION` 殘破度對承載的折減係數 0.5：(1 − 0.5·devastation)（GDD 2.3 人口承載）。
pub const CAP_DEVASTATION: f64 = 0.5;
/// 7.1 `CAP_KARMA` 業氣對承載的折減係數 0.3：(1 − 0.3·karma_qi)（GDD 2.3 人口承載）。
pub const CAP_KARMA: f64 = 0.3;

/// 7.1 `POP_GROWTH` 每年人口成長率 r = 0.008（GDD 2.3 人口成長）。
pub const POP_GROWTH: f64 = 0.008;
/// 7.1 `POP_GROWTH` 盛世成長倍率 1.5（GDD 2.3 人口成長）。
pub const POP_GROWTH_PROSPEROUS_MULT: f64 = 1.5;

/// 7.1 `FAMINE` 饑荒觸發門檻：P > 1.05K（GDD 2.3 饑荒）。
pub const FAMINE_THRESHOLD: f64 = 1.05;
/// 7.1 `FAMINE` 饑荒額外死亡係數 0.5：(P − K) × 0.5（GDD 2.3 饑荒）。
pub const FAMINE_DEATH_RATE: f64 = 0.5;
/// 7.1 `FAMINE` 饑荒的業氣增量 +0.02（GDD 2.3 饑荒）。
pub const FAMINE_KARMA: f32 = 0.02;

/// 7.1 `POP_MIGRATE` 遷徙觸發門檻：P > 0.9K（GDD 2.3 遷徙）。
pub const POP_MIGRATE_THRESHOLD: f64 = 0.9;
/// 7.1 `POP_MIGRATE` 遷徙比例 1%（GDD 2.3 遷徙）。
pub const POP_MIGRATE_RATE: f64 = 0.01;

/// 7.1 `DEVASTATION_RECOVER` 殘破度每年恢復量 0.1（GDD 2.3 殘破度）。
pub const DEVASTATION_RECOVER: f32 = 0.1;

/// 7.1 `CITY_LEVEL` 鎮的人口門檻 20,000（「2 萬」；GDD 2.3 城池等級 1）。
pub const CITY_LEVEL_TOWN_POP: f64 = 20_000.0;
/// 7.1 `CITY_LEVEL` 縣的人口門檻 80,000（「8 萬」；GDD 2.3 城池等級 2）。
pub const CITY_LEVEL_COUNTY_POP: f64 = 80_000.0;
/// 7.1 `CITY_LEVEL` 州的人口門檻 200,000（「20 萬」；GDD 2.3 城池等級 3）。
pub const CITY_LEVEL_STATE_POP: f64 = 200_000.0;
/// 7.1 `CITY_LEVEL` 都城的城池等級 4（GDD 2.3 城池等級 4）。
pub const CITY_LEVEL_CAPITAL: u8 = 4;

/// 7.1 `TAX_RATE` 每人年稅 0.001（GDD 2.3 稅收與兵源）。
pub const TAX_RATE: f64 = 0.001;

/// 7.1 `STANDING_RATE` 常備軍比例 0.005，即 P × 0.005（GDD 2.3 稅收與兵源）。
pub const STANDING_RATE: f64 = 0.005;
/// 7.1 `LEVY_RATE` 動員兵力上限比例 0.02，即 P × 0.02（GDD 2.3 稅收與兵源）。
pub const LEVY_RATE: f64 = 0.02;

/// 7.1 `FAITH_DRIFT` 教化每年向來源靠攏的比例 5%（GDD 2.3 教化）。
pub const FAITH_DRIFT: f32 = 0.05;

/// 7.1 `FAITH_TARGET` 道的聲望換算除數 300，即 min(1, Σ聲望/300)（GDD 2.3 教化目標）。
pub const FAITH_TAO_REPUTATION_DIVISOR: f32 = 300.0;
/// 7.1 `FAITH_TARGET` 道的教化目標上限 1，即 min(1, …)（GDD 2.3 教化目標）。
pub const FAITH_TAO_CAP: f32 = 1.0;
/// 7.1 `FAITH_TARGET` 釋的鄰州擴散係數 0.3，即 0.3·H（GDD 2.3 教化目標）。
pub const FAITH_BUDDHA_NEIGHBOR_SPILL: f32 = 0.3;
/// 7.1 `FAITH_TARGET` 儒的書院教化目標 0.5（GDD 2.3 教化目標）。
pub const FAITH_RU_ACADEMY: f32 = 0.5;
/// 7.1 `FAITH_TARGET` 儒的尊儒國策加成 +0.3（GDD 2.3 教化目標）。
pub const FAITH_RU_RESPECT_BONUS: f32 = 0.3;

/// 7.1 `FAITH_INIT_ANCIENT` 上古道教化初值下限 0.1，U(0.1, 0.4)（GDD 2.3 教化；4.6 上古開局）。
pub const FAITH_INIT_ANCIENT_TAO_MIN: f32 = 0.1;
/// 7.1 `FAITH_INIT_ANCIENT` 上古道教化初值上限 0.4，U(0.1, 0.4)（GDD 2.3 教化；4.6 上古開局）。
pub const FAITH_INIT_ANCIENT_TAO_MAX: f32 = 0.4;
/// 7.1 `FAITH_INIT_ANCIENT` 上古釋教化初值下限 0，U(0, 0.15)（GDD 2.3 教化；4.6 上古開局）。
pub const FAITH_INIT_ANCIENT_BUDDHA_MIN: f32 = 0.0;
/// 7.1 `FAITH_INIT_ANCIENT` 上古釋教化初值上限 0.15，U(0, 0.15)（GDD 2.3 教化；4.6 上古開局）。
pub const FAITH_INIT_ANCIENT_BUDDHA_MAX: f32 = 0.15;
/// 7.1 `FAITH_INIT_ANCIENT` 上古儒教化初值下限 0，U(0, 0.15)（GDD 2.3 教化；4.6 上古開局）。
pub const FAITH_INIT_ANCIENT_RU_MIN: f32 = 0.0;
/// 7.1 `FAITH_INIT_ANCIENT` 上古儒教化初值上限 0.15，U(0, 0.15)（GDD 2.3 教化；4.6 上古開局）。
pub const FAITH_INIT_ANCIENT_RU_MAX: f32 = 0.15;

// ---------------------------------------------------------------------------
// 靈氣與氣場（GDD 2.4）
// ---------------------------------------------------------------------------

/// 7.1 `AURA_TERRAIN` 各地形基礎靈氣，依地形代碼順序排列：
/// 海 0.05、湖 0.05、平原 0.30、丘陵 0.50、山脈 0.70、森林 0.50、雪原 0.40、沙漠 0.15、沼澤 0.35
/// （GDD 2.4 格點靈氣）。
pub const AURA_TERRAIN: [f32; 9] = [0.05, 0.05, 0.30, 0.50, 0.70, 0.50, 0.40, 0.15, 0.35];

/// 7.1 `AURA_NODE` 節點靈氣公式的係數 0.8，即 0.8 × s × exp(−d/8)（GDD 2.4 格點靈氣）。
pub const AURA_NODE_COEF: f32 = 0.8;
/// 7.1 `AURA_NODE` 節點靈氣的衰減距離 8 格，即 exp(−d/8)（GDD 2.4 格點靈氣）。
pub const AURA_NODE_DECAY: f32 = 8.0;
/// 7.1 `AURA_LEY` 靈脈靈氣公式的係數 0.3，即 0.3 × s × exp(−d/3)（GDD 2.4 格點靈氣）。
pub const AURA_LEY_COEF: f32 = 0.3;
/// 7.1 `AURA_LEY` 靈脈靈氣的衰減距離 3 格，即 exp(−d/3)（GDD 2.4 格點靈氣）。
pub const AURA_LEY_DECAY: f32 = 3.0;


/// 7.1 `EVO_WARMTH_P1` 寒暖第一週期的長度下限 600 年（GDD 2.8 慢變量）。
pub const EVO_WARMTH_P1_MIN: u32 = 600;
/// 7.1 `EVO_WARMTH_P1` 寒暖第一週期的長度上限 900 年（GDD 2.8 慢變量）。
pub const EVO_WARMTH_P1_MAX: u32 = 900;
/// 7.1 `EVO_WARMTH_P2` 寒暖第二週期的長度下限 1,500 年（GDD 2.8 慢變量）。
pub const EVO_WARMTH_P2_MIN: u32 = 1500;
/// 7.1 `EVO_WARMTH_P2` 寒暖第二週期的長度上限 2,250 年（GDD 2.8 慢變量）。
pub const EVO_WARMTH_P2_MAX: u32 = 2250;
/// 7.1 `EVO_WARMTH_WEIGHT` 寒暖第一週期的權重 0.6（GDD 2.8 慢變量）。
pub const EVO_WARMTH_W1: f32 = 0.6;
/// 7.1 `EVO_WARMTH_WEIGHT` 寒暖第二週期的權重 0.4（GDD 2.8 慢變量）。
pub const EVO_WARMTH_W2: f32 = 0.4;
/// 7.1 `EVO_SEA_RANGE` 海平面偏移的上下限 0.03（高度單位；GDD 2.8 慢變量）。
pub const EVO_SEA_RANGE: f32 = 0.03;
/// 7.1 `EVO_SEA_APPROACH` 海平面每世向目標靠近的比例 0.5（GDD 2.8 慢變量）。
pub const EVO_SEA_APPROACH: f32 = 0.5;
/// 7.1 `EVO_TIDE_PERIOD` 靈潮週期的長度下限 900 年（GDD 2.8 慢變量）。
pub const EVO_TIDE_PERIOD_MIN: u32 = 900;
/// 7.1 `EVO_TIDE_PERIOD` 靈潮週期的長度上限 1,400 年（GDD 2.8 慢變量）。
pub const EVO_TIDE_PERIOD_MAX: u32 = 1400;
/// 7.1 `EVO_TIDE_AMPLITUDE` 靈潮在 1 上下的振幅 0.4（GDD 2.8 慢變量）。
pub const EVO_TIDE_AMPLITUDE: f32 = 0.4;
/// 7.1 `EVO_TIDE_RANGE` 靈潮的下限 0.5（GDD 2.8 慢變量）。
pub const EVO_TIDE_MIN: f32 = 0.5;
/// 7.1 `EVO_TIDE_RANGE` 靈潮的上限 1.4（GDD 2.8 慢變量）。
pub const EVO_TIDE_MAX: f32 = 1.4;
/// 7.1 `EVO_TIDE_TURN` 上古開局時靈潮在週期裡的位置 0.365 圈：過了高點、退到 1.3（GDD 2.8、4.6）。
pub const EVO_TIDE_TURN_ANCIENT: f32 = 0.365;
/// 7.1 `EVO_TIDE_TURN` 中古開局時靈潮在週期裡的位置 0.5 圈：退到 1.0（GDD 2.8、4.6）。
pub const EVO_TIDE_TURN_MIDDLE: f32 = 0.5;
/// 7.1 `EVO_TIDE_TURN` 末法開局時靈潮在週期裡的位置 0.75 圈：谷底 0.6（GDD 2.8、4.6）。
pub const EVO_TIDE_TURN_DECLINE: f32 = 0.75;
/// 7.1 `EVO_HISTORY` 上古開局前的歷史 600–1,200 年（GDD 2.8 開局前的歷史）。
pub const EVO_HISTORY_ANCIENT: (u32, u32) = (600, 1200);
/// 7.1 `EVO_HISTORY` 中古開局前的歷史 1,800–3,000 年（GDD 2.8 開局前的歷史）。
pub const EVO_HISTORY_MIDDLE: (u32, u32) = (1800, 3000);
/// 7.1 `EVO_HISTORY` 末法開局前的歷史 3,600–5,400 年（GDD 2.8 開局前的歷史）。
pub const EVO_HISTORY_DECLINE: (u32, u32) = (3600, 5400);
/// 7.1 `EVO_WARMTH_GATE` 大寒、大暖的進入門檻：寒暖的絕對值達 0.6（GDD 2.8 世界大事）。
pub const EVO_WARMTH_ENTER: f32 = 0.6;
/// 7.1 `EVO_WARMTH_GATE` 大寒、大暖的結束門檻：寒暖的絕對值回到 0.3（GDD 2.8 世界大事）。
pub const EVO_WARMTH_LEAVE: f32 = 0.3;
/// 7.1 `EVO_DECLINE_GATE` 末法的進入門檻：靈潮降到 0.7（GDD 2.8 世界大事）。
pub const EVO_DECLINE_ENTER: f32 = 0.7;
/// 7.1 `EVO_DECLINE_GATE` 末法的結束門檻：靈潮回到 0.9（GDD 2.8 世界大事）。
pub const EVO_DECLINE_LEAVE: f32 = 0.9;
/// 7.1 `EVO_REVIVAL_GATE` 靈氣復甦的進入門檻：靈潮漲到 1.3（GDD 2.8 世界大事）。
pub const EVO_REVIVAL_ENTER: f32 = 1.3;
/// 7.1 `EVO_REVIVAL_GATE` 靈氣復甦的結束門檻：靈潮退到 1.2（GDD 2.8 世界大事）。
pub const EVO_REVIVAL_LEAVE: f32 = 1.2;
/// 7.1 `EVO_ERA_MIN_YEARS` 兩次改元至少相隔 60 年（GDD 2.8 世界大事）。
pub const EVO_ERA_MIN_YEARS: u32 = 60;
/// 7.1 `EVO_NODE_FADE` 靈潮低於 0.7 時，洞天福地每世可能降一級（GDD 2.8 每世結算）。
pub const EVO_NODE_FADE_BELOW: f32 = 0.7;
/// 7.1 `EVO_NODE_RISE` 靈潮高於 1.1 時，降過級的洞天福地每世可能回復一級（GDD 2.8 每世結算）。
pub const EVO_NODE_RISE_ABOVE: f32 = 1.1;
/// 7.1 `EVO_NODE_CHANCE` 洞天福地每世降級或回復的機率 0.1（GDD 2.8 每世結算）。
pub const EVO_NODE_CHANCE: f32 = 0.1;
/// 7.1 `EVO_NODE_IMPORTANCE` 洞天福地升降級的事件重要度 3（GDD 2.8、7.2）。
pub const EVO_NODE_IMPORTANCE: u8 = 3;
/// 7.1 `EVO_EVENT_IMPORTANCE` 世界大事的事件重要度 5（GDD 2.8、7.2）。
pub const EVO_EVENT_IMPORTANCE: u8 = 5;

/// 7.1 `AURA_CELL_MAX` 格點靈氣上限 2（GDD 2.4 格點靈氣，截斷 0..2）。
pub const AURA_CELL_MAX: f32 = 2.0;

/// 7.1 `AURA_PROVINCE_MIX` 州郡靈氣的格點平均權重 0.7（GDD 2.4 州郡氣場）。
pub const AURA_PROVINCE_MEAN_WEIGHT: f32 = 0.7;
/// 7.1 `AURA_PROVINCE_MIX` 州郡靈氣的格點最大值權重 0.3（GDD 2.4 州郡氣場）。
pub const AURA_PROVINCE_MAX_WEIGHT: f32 = 0.3;
/// 7.1 `AURA_PROVINCE_MIX` 州郡靈氣截斷下限 0（GDD 2.4 州郡氣場，截斷 0..1）。
pub const AURA_PROVINCE_CLAMP_MIN: f32 = 0.0;
/// 7.1 `AURA_PROVINCE_MIX` 州郡靈氣截斷上限 1（GDD 2.4 州郡氣場，截斷 0..1）。
pub const AURA_PROVINCE_CLAMP_MAX: f32 = 1.0;

/// 7.1 `KARMA_DECAY` 業氣每年衰減 3%（GDD 2.4 州郡氣場）。
pub const KARMA_DECAY: f32 = 0.03;
/// 7.1 `MERIT_DECAY` 功德每年衰減 2%（GDD 2.4 州郡氣場）。
pub const MERIT_DECAY: f32 = 0.02;
/// 7.1 `RIGHT_DECAY` 正氣每年衰減 2%（GDD 2.4 州郡氣場）。
pub const RIGHT_DECAY: f32 = 0.02;

/// 7.1 `KARMA_DIFFUSE` 業氣每季擴散比例 8%（GDD 2.4 州郡氣場）。
pub const KARMA_DIFFUSE: f32 = 0.08;
/// 7.1 `MERIT_DIFFUSE` 功德每季擴散比例 4%（GDD 2.4 州郡氣場）。
pub const MERIT_DIFFUSE: f32 = 0.04;
/// 7.1 `RIGHT_DIFFUSE` 正氣每季擴散比例 4%（GDD 2.4 州郡氣場）。
pub const RIGHT_DIFFUSE: f32 = 0.04;

/// 7.1 `TURBID_MIX` 濁氣公式中功德的係數 0.5：turbid = 業氣 − 0.5·功德 − 0.5·正氣（GDD 2.4 濁氣）。
pub const TURBID_MIX_MERIT: f32 = 0.5;
/// 7.1 `TURBID_MIX` 濁氣公式中正氣的係數 0.5：turbid = 業氣 − 0.5·功德 − 0.5·正氣（GDD 2.4 濁氣）。
pub const TURBID_MIX_RIGHT: f32 = 0.5;

/// 7.1 `TURBID_PLAGUE` 疫病觸發的濁氣門檻 0.3（GDD 2.4 濁氣）。
pub const TURBID_PLAGUE_THRESHOLD: f32 = 0.3;
/// 7.1 `TURBID_PLAGUE` 疫病機率係數 0.05，即每年 turbid × 0.05（GDD 2.4 濁氣）。
pub const TURBID_PLAGUE_PROB_COEF: f32 = 0.05;
/// 7.1 `TURBID_PLAGUE` 疫病人口損失下限 5%（GDD 2.4 濁氣）。
pub const TURBID_PLAGUE_POP_LOSS_MIN: f64 = 0.05;
/// 7.1 `TURBID_PLAGUE` 疫病人口損失上限 15%（GDD 2.4 濁氣）。
pub const TURBID_PLAGUE_POP_LOSS_MAX: f64 = 0.15;
/// 7.1 `TURBID_PLAGUE` 疫病的業氣增量 +0.03（GDD 2.4 濁氣）。
pub const TURBID_PLAGUE_KARMA: f32 = 0.03;

/// 7.1 `TURBID_BEAST` 妖滋生的濁氣門檻 0.5（GDD 2.4 濁氣）。
pub const TURBID_BEAST_THRESHOLD: f32 = 0.5;
/// 7.1 `TURBID_BEAST` 妖滋生機率係數 0.2，即 (turbid − 0.5) × 0.2（GDD 2.4 濁氣）。
pub const TURBID_BEAST_PROB_COEF: f32 = 0.2;

/// 7.1 `TURBID_DEN` 魔窟形成的濁氣門檻 0.8（GDD 2.4 濁氣）。
pub const TURBID_DEN_THRESHOLD: f32 = 0.8;
/// 7.1 `DEN_CLEAR` 魔窟消失的濁氣門檻 0.4，濁氣低於此消失（GDD 2.4 濁氣）。
pub const DEN_CLEAR_THRESHOLD: f32 = 0.4;

/// 7.1 `KARMA_BATTLE` 戰鬥與妖襲的業氣係數 2，即 死傷 / 人口 × 2（GDD 2.4 業氣來源）。
pub const KARMA_BATTLE_COEF: f32 = 2.0;
/// 7.1 `KARMA_MASSACRE` 屠城的業氣增量 0.4（GDD 2.4 業氣來源）。
pub const KARMA_MASSACRE: f32 = 0.4;
/// 7.1 `KARMA_PER_EXECUTION` 每名被處死或滅佛中被殺的有名之人的業氣增量 0.02（GDD 2.4 業氣來源）。
pub const KARMA_PER_EXECUTION: f32 = 0.02;

// ── 7.1 補充（M1 規格確認後加入）──────────────────────────

/// GDD 7.1 `WG_PROV_SPACING`：州郡種子點 Poisson 最小間距係數（GDD 2.2）。
pub const WG_PROV_SPACING: f32 = 0.8;
/// GDD 7.1 `CAP_DROUGHT`：旱澇時的承載倍率（GDD 2.3、4.2）。
pub const CAP_DROUGHT: f64 = 0.5;
/// GDD 7.1 `CAP_REBUILD`：劫後重建時的承載倍率（GDD 2.3、3.11）。
pub const CAP_REBUILD: f64 = 1.5;

// ── M2：魂、宿緣、有名之人、道、開天 ──────────────────────

// ---- 魂與輪迴（GDD 3.1、3.2）----

/// 7.1 `SIN_KILL` 殺有名之人的殺業基數 10（GDD 3.1 殺業；兇手 S += 10 × 無辜係數）。
pub const SIN_KILL_BASE: f32 = 10.0;
/// 7.1 `SIN_KILL` 無辜係數：死者淨業力 ≥ 0 時為 1（GDD 3.1）。
pub const SIN_KILL_INNOCENT_COEF: f32 = 1.0;
/// 7.1 `SIN_KILL` 無辜係數：死者淨業力 < 0 時為 0.3（GDD 3.1）。
pub const SIN_KILL_GUILTY_COEF: f32 = 0.3;
/// 7.1 `EVIL_THRESHOLD` 斬惡判定：死者 S > G + 50 時視為惡（GDD 3.1 斬惡）。
pub const EVIL_THRESHOLD_OFFSET: f32 = 50.0;
/// 7.1 `MERIT_SLAY_EVIL` 斬惡的功德增量 +5（GDD 3.1 斬惡）。
pub const MERIT_SLAY_EVIL: f32 = 5.0;
/// 7.1 `SIN_WAR_PER` 主將每計一批的殺業增量 1（GDD 3.1 戰爭主將）。
pub const SIN_WAR_PER: f32 = 1.0;
/// 7.1 `SIN_WAR_PER` 主將殺業的計數單位 5,000 殺敵，即每 5,000 殺敵計一批（GDD 3.1 戰爭主將）。
pub const SIN_WAR_KILLS_PER: u32 = 5_000;
/// 7.1 `SIN_WAR_PER` 正義之戰的殺業倍率 0.5（GDD 3.1 戰爭主將）。
pub const SIN_WAR_JUST_MULT: f32 = 0.5;
/// 7.1 `SIN_MASSACRE` 屠城殺業基數 100（GDD 3.1 屠城：100 + 死者 / 1,000）。
pub const SIN_MASSACRE_BASE: f32 = 100.0;
/// 7.1 `SIN_MASSACRE` 屠城殺業的死者除數 1,000（GDD 3.1 屠城）。
pub const SIN_MASSACRE_DEATHS_DIV: u32 = 1_000;
/// 7.1 `WRAITH_PER` 每計一單位屠城死者產生的怨靈數 1（GDD 3.1 怨靈）。
pub const WRAITH_PER: u32 = 1;
/// 7.1 `WRAITH_PER` 怨靈計數的死者單位 10,000 人（GDD 3.1 怨靈）。
pub const WRAITH_DEATHS_UNIT: u32 = 10_000;
/// 7.1 `WRAITH_MAX` 單次屠城產生怨靈的上限 5（GDD 3.1 怨靈）。
pub const WRAITH_MAX: u32 = 5;
/// 7.1 `MERIT_DISCIPLE` 收徒的功德增量 2（GDD 3.1 功德）。
pub const MERIT_DISCIPLE: f32 = 2.0;
/// 7.1 `MERIT_MONK_YEAR` 僧人每年功德的基數 1（GDD 3.1 功德，公式 1 + 悟性 × 0.2）。
pub const MERIT_MONK_YEAR_BASE: f32 = 1.0;
/// 7.1 `MERIT_MONK_YEAR` 僧人每年功德的悟性係數 0.2（GDD 3.1 功德）。
pub const MERIT_MONK_YEAR_INSIGHT_COEF: f32 = 0.2;
/// 7.1 `MERIT_GOOD_RULE` 善政每年功德的德行基準 5，即 (德行 − 5)（GDD 3.1 每年）。
pub const MERIT_GOOD_RULE_VIRTUE_BASE: f32 = 5.0;
/// 7.1 `MERIT_GOOD_RULE` 善政每年功德的係數 0.5（GDD 3.1 每年）。
pub const MERIT_GOOD_RULE_COEF: f32 = 0.5;
/// 7.1 `SIN_TYRANNY` 暴政每年殺業的戾氣基準 6，即 (戾氣 − 6)（GDD 3.1 每年）。
pub const SIN_TYRANNY_FEROCITY_BASE: f32 = 6.0;
/// 7.1 `SIN_TYRANNY` 暴政每年殺業的係數 0.5（GDD 3.1 每年）。
pub const SIN_TYRANNY_COEF: f32 = 0.5;
/// 7.1 `MERIT_RELIEF` 賑災功德的救活人數單位 2,000 人，即 救活人數 / 2,000（GDD 3.1 賑災）。
pub const MERIT_RELIEF_PER_LIVES: u32 = 2_000;
/// 7.1 `MERIT_CONVERT` 度化的功德增量 20（GDD 3.1 度化；3.6）。
pub const MERIT_CONVERT: f32 = 20.0;
/// 7.1 `CONVERT_SIN_MULT` 被度化者 S 的倍率 0.7（GDD 3.1 度化；3.6）。
pub const CONVERT_SIN_MULT: f32 = 0.7;
/// 7.1 `THREAD_INTEREST` 每次轉世的利息倍率 1.1（強度 ×1.1，GDD 3.2 利息）。
pub const THREAD_INTEREST_MULT: f32 = 1.1;
/// 7.1 `THREAD_INTEREST` 利息後的強度上限 100（GDD 3.2 利息）。
pub const THREAD_STRENGTH_CAP: f32 = 100.0;
/// 7.1 `THREAD_INTEREST_SIN` 利息附帶的殺業係數 0.1：S += 強度 × 0.1（以舊強度計，GDD 3.2 利息）。
pub const THREAD_INTEREST_SIN_COEF: f32 = 0.1;
/// 7.1 `WISDOM_DECAY` 宿慧每次轉世的保留倍率 0.5（GDD 3.2 宿慧）。
pub const WISDOM_DECAY: f32 = 0.5;
/// 7.1 `WISDOM_GAIN` 前世最高境界的宿慧倍率 8（每境界 ×8，GDD 3.1 前世成就）。
pub const WISDOM_GAIN_REALM: f32 = 8.0;
/// 7.1 `WISDOM_GAIN` 前世最高果位的宿慧倍率 15（每果位 ×15，GDD 3.1 前世成就）。
pub const WISDOM_GAIN_FRUIT: f32 = 15.0;
/// 7.1 `WISDOM_GAIN` 前世三不朽的宿慧除數 10（三不朽 ÷10，GDD 3.1 前世成就）。
pub const WISDOM_GAIN_IMMORTAL_DIV: f32 = 10.0;
/// 7.1 `WISDOM_GAIN` 前世兵解的宿慧增量 +30（GDD 3.1 前世成就；3.5 兵解）。
pub const WISDOM_GAIN_SHED_BODY: f32 = 30.0;
/// 7.1 `AFFINITY_KEEP` 道緣的前世保留比例 0.7（GDD 3.1 道緣）。
pub const AFFINITY_KEEP: f32 = 0.7;
/// 7.1 `AFFINITY_KEEP` 入魔時道緣的魔分量增量 +0.2（GDD 3.1 道緣）。
pub const AFFINITY_DEMON_GAIN: f32 = 0.2;
/// 7.1 `AFFINITY_KEEP` 被度化時道緣的釋分量增量 +0.3（GDD 3.1 道緣）。
pub const AFFINITY_CONVERTED_BUDDHA_GAIN: f32 = 0.3;
/// 7.1 `FAITH4_TURBID` faith4 中濁氣的係數 0.5，即 faith4 = (道, 釋, 儒, 濁氣 × 0.5)（GDD 3.1 faith4）。
pub const FAITH4_TURBID: f32 = 0.5;
/// 7.1 `BARDO_MONTHS` 中陰時長 2 個月（GDD 3.2 中陰；本值已為月數）。
pub const BARDO_MONTHS: u32 = 2;
/// 7.1 `PATH_NOISE` 六道判定雜訊的振幅 15，即 ±15（GDD 3.2 判定）。
pub const PATH_NOISE: f32 = 15.0;
/// 7.1 `PATH_HEAVEN` 天道門檻：業障 x ≥ 200（GDD 3.2 天道）。
pub const PATH_HEAVEN_X: f32 = 200.0;
/// 7.1 `PATH_HEAVEN` 天道門檻：宿慧 W ≥ 50（GDD 3.2 天道）。
pub const PATH_HEAVEN_W: f32 = 50.0;
/// 7.1 `PATH_HEAVEN` 天道停留年數下限 30 年（GDD 3.2 天道；程式換算月數時 × 12）。
pub const PATH_HEAVEN_YEARS_MIN: u32 = 30;
/// 7.1 `PATH_HEAVEN` 天道停留年數上限 100 年（GDD 3.2 天道；程式換算月數時 × 12）。
pub const PATH_HEAVEN_YEARS_MAX: u32 = 100;
/// 7.1 `PATH_HEAVEN` 回來時功德 G 的倍率 0.5（GDD 3.2 天道）。
pub const PATH_HEAVEN_RETURN_G_MULT: f32 = 0.5;
/// 7.1 `PATH_ASURA` 阿修羅道的殺業門檻 S ≥ 150（GDD 3.2 阿修羅道）。
pub const PATH_ASURA_S: f32 = 150.0;
/// 7.1 `PATH_ASURA` 阿修羅道的魔分量門檻 魔 ≥ 0.4（GDD 3.2 阿修羅道）。
pub const PATH_ASURA_DEMON: f32 = 0.4;
/// 7.1 `PATH_HUMAN` 人道的業障門檻 x ≥ −50（GDD 3.2 人道）。
pub const PATH_HUMAN_X: f32 = -50.0;
/// 7.1 `PATH_BEAST` 畜生道的業障門檻 x ≥ −300（GDD 3.2 畜生道）。
pub const PATH_BEAST_X: f32 = -300.0;
/// 7.1 `PATH_BEAST` 宿慧門檻 W ≥ 30：M4 起生為妖；M2–M3 暫行時同樣一律為獸（GDD 3.2 畜生道）。
pub const PATH_BEAST_WISDOM: f32 = 30.0;
/// 7.1 `PATH_BEAST` 為獸停留年數下限 10 年（GDD 3.2 畜生道；程式換算月數時 × 12）。
pub const PATH_BEAST_YEARS_MIN: u32 = 10;
/// 7.1 `PATH_BEAST` 為獸停留年數上限 30 年（GDD 3.2 畜生道；程式換算月數時 × 12）。
pub const PATH_BEAST_YEARS_MAX: u32 = 30;
/// 7.1 `PATH_BEAST` 為獸回來時殺業的倍率 0.9（GDD 3.2 畜生道）。
pub const PATH_BEAST_RETURN_S_MULT: f32 = 0.9;
/// 7.1 `PATH_HUNGRY` 餓鬼道的業障門檻 x ≥ −600（GDD 3.2 餓鬼道）。
pub const PATH_HUNGRY_X: f32 = -600.0;
/// 7.1 `PATH_HUNGRY` 餓鬼道停留年數下限 50 年（GDD 3.2 餓鬼道；程式換算月數時 × 12）。
pub const PATH_HUNGRY_YEARS_MIN: u32 = 50;
/// 7.1 `PATH_HUNGRY` 餓鬼道停留年數上限 150 年（GDD 3.2 餓鬼道；程式換算月數時 × 12）。
pub const PATH_HUNGRY_YEARS_MAX: u32 = 150;
/// 7.1 `PATH_HUNGRY` 餓鬼道每年殺業的衰減倍率 0.98（GDD 3.2 餓鬼道）。
pub const PATH_HUNGRY_S_DECAY_PER_YEAR: f32 = 0.98;
/// 7.1 `PATH_HELL` 地獄道停留年數下限 150 年（GDD 3.2 地獄道；程式換算月數時 × 12）。
pub const PATH_HELL_YEARS_MIN: u32 = 150;
/// 7.1 `PATH_HELL` 地獄道停留年數上限 500 年（GDD 3.2 地獄道；程式換算月數時 × 12）。
pub const PATH_HELL_YEARS_MAX: u32 = 500;
/// 7.1 `PATH_HELL` 地獄道每年殺業的衰減倍率 0.99（GDD 3.2 地獄道）。
pub const PATH_HELL_S_DECAY_PER_YEAR: f32 = 0.99;
/// 7.1 `SALVATION` 超度的功德係數 50，即 50 × 香火（GDD 3.2 超度）。
pub const SALVATION_PER_INCENSE: f32 = 50.0;
/// 7.1 `PULL_MIX` 因果牽引權重：人口 0.5（GDD 3.2 因果牽引）。
pub const PULL_MIX_POP: f32 = 0.5;
/// 7.1 `PULL_MIX` 因果牽引權重：道緣 0.2（GDD 3.2 因果牽引）。
pub const PULL_MIX_AFFINITY: f32 = 0.2;
/// 7.1 `PULL_MIX` 因果牽引權重：宿緣 0.3（GDD 3.2 因果牽引）。
pub const PULL_MIX_THREAD: f32 = 0.3;
/// 7.1 `PULL_MIX` 無宿緣對象時的人口權重 0.7（GDD 3.2 因果牽引）。
pub const PULL_MIX_NO_THREAD_POP: f32 = 0.7;
/// 7.1 `PULL_MIX` 無宿緣對象時的道緣權重 0.3（GDD 3.2 因果牽引）。
pub const PULL_MIX_NO_THREAD_AFFINITY: f32 = 0.3;
/// 7.1 `WAIT_MIN_STRENGTH` 等待配對的宿緣強度下限 50（GDD 3.2 等待）。
pub const WAIT_MIN_STRENGTH: f32 = 50.0;
/// 7.1 `WAIT_MAX` 等待期滿的年數上限 30 年（GDD 3.2 等待；程式換算月數時 × 12）。
pub const WAIT_MAX_YEARS: u32 = 30;
/// 7.1 `BIRTH_DEFER` 達承載上限時延後出生 1 年（GDD 3.2 延後出生；程式換算月數時 × 12）。
pub const BIRTH_DEFER_YEARS: u32 = 1;

// ---- 宿緣（GDD 3.3）----

/// 7.1 `THREAD_INIT` 殺業線初始強度基數 40（GDD 3.3 初始強度）。
pub const THREAD_INIT_KILL: f32 = 40.0;
/// 7.1 `THREAD_INIT` 殺業線初始強度的修士境界係數 5，即 + 修士境界 × 5（GDD 3.3）。
pub const THREAD_INIT_KILL_REALM_MULT: f32 = 5.0;
/// 7.1 `THREAD_INIT` 殺業線初始強度中君主的加成 +20（GDD 3.3）。
pub const THREAD_INIT_KILL_MONARCH_BONUS: f32 = 20.0;
/// 7.1 `THREAD_INIT` 恩義線初始強度 30（GDD 3.3）。
pub const THREAD_INIT_GRACE: f32 = 30.0;
/// 7.1 `THREAD_INIT` 情緣線初始強度 30（GDD 3.3）。
pub const THREAD_INIT_LOVE: f32 = 30.0;
/// 7.1 `THREAD_INIT` 師徒線初始強度 20（GDD 3.3）。
pub const THREAD_INIT_MASTER: f32 = 20.0;
/// 7.1 `THREAD_INIT` 怨線初始強度 60（GDD 3.3）。
pub const THREAD_INIT_FEUD: f32 = 60.0;
/// 7.1 `THREAD_INIT` 司命牽線初始強度 50（GDD 3.3）。
pub const THREAD_INIT_TIANJI: f32 = 50.0;
/// 7.1 `MEET_RANGE` 相遇距離 1 步（沿陸鄰，GDD 3.3 相遇）。
pub const MEET_RANGE_STEPS: u32 = 1;
/// 7.1 `MEET_CHANCE` 相遇機率係數 0.15，即 0.15 × 強度 / 50（GDD 3.3 相遇頻率）。
pub const MEET_CHANCE_COEF: f32 = 0.15;
/// 7.1 `MEET_CHANCE` 相遇機率的強度除數 50（GDD 3.3 相遇頻率）。
pub const MEET_CHANCE_STRENGTH_DIV: f32 = 50.0;
/// 7.1 `MEET_CHANCE` 同州相遇機率的倍率 2（GDD 3.3 相遇頻率）。
pub const MEET_SAME_PROVINCE_MULT: f32 = 2.0;
/// 7.1 `MEET_PER_SOUL_PER_YEAR` 每個魂每年最多處理的相遇次數 1（GDD 3.3 相遇頻率）。
pub const MEET_PER_SOUL_PER_YEAR: u32 = 1;
/// 7.1 `FEUD_CLASH` 殺業衝突機率 50%（GDD 3.3 相遇結果）。
pub const FEUD_CLASH_CHANCE: f32 = 0.5;
/// 7.1 `ATONE_RESCUE` 化解救命的機率係數 0.3，即 0.3 × 德行 / 10（GDD 3.3 相遇結果）。
pub const ATONE_RESCUE_COEF: f32 = 0.3;
/// 7.1 `ATONE_RESCUE` 化解救命的德行除數 10（GDD 3.3 相遇結果）。
pub const ATONE_RESCUE_VIRTUE_DIV: f32 = 10.0;
/// 7.1 `GRACE_REPAY` 報恩機率 50%（GDD 3.3 相遇結果）。
pub const GRACE_REPAY_CHANCE: f32 = 0.5;
/// 7.1 `GRACE_CULT` 報恩時的修為加成 10%（GDD 3.3 相遇結果）。
pub const GRACE_CULT_BONUS: f32 = 0.10;
/// 7.1 `LOVE_UNITE` 年齡差 ≤ 10 時的結合機率 0.6（GDD 3.3 相遇結果）。
pub const LOVE_UNITE_CHANCE_NEAR_AGE: f32 = 0.6;
/// 7.1 `LOVE_UNITE` 年齡差 > 10 時的結合機率 0.1（GDD 3.3 相遇結果）。
pub const LOVE_UNITE_CHANCE_FAR_AGE: f32 = 0.1;
/// 7.1 `LOVE_UNITE` 年齡差判定門檻 10 歲（GDD 3.3 相遇結果）。
pub const LOVE_NEAR_AGE_GAP: u32 = 10;
/// 7.1 `LOVE_FAIL` 情緣結合失敗時強度 −10（GDD 3.3 相遇結果）。
pub const LOVE_FAIL_STRENGTH_PENALTY: f32 = -10.0;
/// 7.1 `MASTER_JOIN` 同教拜師機率 0.7（GDD 3.3 相遇結果）。
pub const MASTER_JOIN_CHANCE: f32 = 0.7;
/// 7.1 `FEUD_ESCALATE` 冤冤相報時舊線的強度增量 +30，另建獨立新線（GDD 3.3）。
pub const FEUD_ESCALATE_OLD_BONUS: f32 = 30.0;
/// 7.1 `FEUD_CHAIN` 天書「冤冤相報」的殺業線累計條數 3（GDD 3.3；4.5 天書）。
pub const FEUD_CHAIN_COUNT: u32 = 3;
/// 7.1 `RESOLVE_REVENGE` 報仇了結時殺業 S 的扣減係數 0.5，即 S −強度 × 0.5（GDD 3.3 了結）。
pub const RESOLVE_REVENGE_S_COEF: f32 = 0.5;
/// 7.1 `RESOLVE_ATONE` 化解了結時殺業 S 的扣減係數 1.0，即 S −強度（GDD 3.3 了結）。
pub const RESOLVE_ATONE_S_COEF: f32 = 1.0;
/// 7.1 `RESOLVE_OTHER` 其他了結時功德 G 的增量係數 0.2，即 G +強度 × 0.2（GDD 3.3 了結）。
pub const RESOLVE_OTHER_G_COEF: f32 = 0.2;
/// 7.1 `OMEN_RANGE` 預兆距離 2 步（GDD 3.3 預兆）。
pub const OMEN_RANGE_STEPS: u32 = 2;
/// 7.1 `OMEN_MIN` 預兆的宿緣強度下限 50（GDD 3.3 預兆）。
pub const OMEN_MIN_STRENGTH: f32 = 50.0;
/// 7.1 `LEDGER_TRIVIAL_KEEP` 重要性 1 的事件保留年數 100 年（GDD 3.3 因果帳）。
pub const LEDGER_TRIVIAL_KEEP_YEARS: u32 = 100;

// ---- 有名之人（GDD 3.4）----

/// 7.1 `ATTR_ROOT` 根骨的隨機下限 0，即 U(0,6) + W/40（GDD 3.4 屬性）。
pub const ATTR_ROOT_MIN: f32 = 0.0;
/// 7.1 `ATTR_ROOT` 根骨的隨機上限 6，即 U(0,6) + W/40（GDD 3.4 屬性）。
pub const ATTR_ROOT_MAX: f32 = 6.0;
/// 7.1 `ATTR_ROOT` 根骨的宿慧除數 40，即 W/40（GDD 3.4 屬性）。
pub const ATTR_ROOT_WISDOM_DIV: f32 = 40.0;
/// 7.1 `INSIGHT` 悟性的隨機下限 0，即 U(0,6) + W/40（與根骨同式，GDD 3.4 屬性）。
pub const ATTR_INSIGHT_MIN: f32 = 0.0;
/// 7.1 `INSIGHT` 悟性的隨機上限 6，即 U(0,6) + W/40（與根骨同式，GDD 3.4 屬性）。
pub const ATTR_INSIGHT_MAX: f32 = 6.0;
/// 7.1 `INSIGHT` 悟性的宿慧除數 40，即 W/40（與根骨同式，GDD 3.4 屬性）。
pub const ATTR_INSIGHT_WISDOM_DIV: f32 = 40.0;
/// 7.1 `ATTR_TALENT` 才略的隨機下限 0，即 U(0,8)（GDD 3.4 屬性）。
pub const ATTR_TALENT_MIN: f32 = 0.0;
/// 7.1 `ATTR_TALENT` 才略的隨機上限 8，即 U(0,8)（GDD 3.4 屬性）。
pub const ATTR_TALENT_MAX: f32 = 8.0;
/// 7.1 `VIRTUE` 德行的隨機下限 2，即 U(2,8) + (G−S)/100（GDD 3.4 屬性）。
pub const ATTR_VIRTUE_MIN: f32 = 2.0;
/// 7.1 `VIRTUE` 德行的隨機上限 8，即 U(2,8) + (G−S)/100（GDD 3.4 屬性）。
pub const ATTR_VIRTUE_MAX: f32 = 8.0;
/// 7.1 `VIRTUE` 德行的淨業力除數 100，即 (G−S)/100（GDD 3.4 屬性）。
pub const ATTR_VIRTUE_NET_DIV: f32 = 100.0;
/// 7.1 `FEROCITY` 戾氣的隨機下限 0，即 U(0,6) + 修羅 4（GDD 3.4 屬性）。
pub const ATTR_FEROCITY_MIN: f32 = 0.0;
/// 7.1 `FEROCITY` 戾氣的隨機上限 6，即 U(0,6) + 修羅 4（GDD 3.4 屬性）。
pub const ATTR_FEROCITY_MAX: f32 = 6.0;
/// 7.1 `FEROCITY` 修羅的戾氣加成 4，即 + 修羅 4（GDD 3.4 屬性）。
pub const ATTR_FEROCITY_ASURA_BONUS: f32 = 4.0;
/// 7.1 `POWER` 修士戰力的底數 3，即 3^境界（GDD 3.4 戰力）。
pub const POWER_CULTIVATOR_BASE: f32 = 3.0;
/// 7.1 `POWER` 修士戰力的層數除數 10，即 (1 + 層/10)（GDD 3.4 戰力）。
pub const POWER_LAYER_DIV: f32 = 10.0;
/// 7.1 `POWER` 僧人戰力的基數 2，即 2 × 3^果位（GDD 3.4 戰力）。
pub const POWER_MONK_BASE: f32 = 2.0;
/// 7.1 `POWER` 僧人戰力的果位底數 3，即 2 × 3^果位（GDD 3.4 戰力）。
pub const POWER_MONK_FRUIT_BASE: f32 = 3.0;
/// 7.1 `POWER` 大儒固定戰力 27（GDD 3.4 戰力）。
pub const POWER_GREAT_CONFUCIAN: f32 = 27.0;
/// 7.1 `POWER` 武將戰力的基數 3，即 3 + 才略（GDD 3.4 戰力）。
pub const POWER_GENERAL_BASE: f32 = 3.0;
/// 7.1 `POWER` 武將戰力的才略係數 1，即 3 + 才略 × 1（GDD 3.4 戰力）。
pub const POWER_GENERAL_TALENT_COEF: f32 = 1.0;
/// 7.1 `POWER` 君主戰力的基數 2，即 2 + 才略 × 0.5（GDD 3.4 戰力）。
pub const POWER_MONARCH_BASE: f32 = 2.0;
/// 7.1 `POWER` 君主戰力的才略係數 0.5（GDD 3.4 戰力）。
pub const POWER_MONARCH_TALENT_COEF: f32 = 0.5;
/// 7.1 `POWER` 君主被襲時戰力倍率 3（GDD 3.4 戰力）。
pub const POWER_MONARCH_ASSAULT_MULT: f32 = 3.0;
/// 7.1 `POWER` 其他人物戰力的基數 1，即 1 + 才略 × 0.2（GDD 3.4 戰力）。
pub const POWER_OTHER_BASE: f32 = 1.0;
/// 7.1 `POWER` 其他人物戰力的才略係數 0.2（GDD 3.4 戰力）。
pub const POWER_OTHER_TALENT_COEF: f32 = 0.2;
/// 7.1 `ENTRY_REQ` 道的入道門檻：根骨 ≥ 3（GDD 3.4 入道）。
pub const ENTRY_REQ_TAO_ROOT: f32 = 3.0;
/// 7.1 `ENTRY_REQ` 釋的入道門檻：道緣 ≥ 0.2（GDD 3.4 入道）。
pub const ENTRY_REQ_BUDDHA_AFFINITY: f32 = 0.2;
/// 7.1 `ENTRY_REQ` 儒的入道門檻：悟性 ≥ 3（GDD 3.4 入道）。
pub const ENTRY_REQ_RU_INSIGHT: f32 = 3.0;
/// 7.1 `ENTRY_REQ` 武將候補的入道門檻：才略 ≥ 6（GDD 3.4 入道）。
pub const ENTRY_REQ_GENERAL_TALENT: f32 = 6.0;
/// 7.1 `ENTRY_REQ` 魔的入道門檻：本州濁氣 > 0.5，或帶修羅印記（GDD 3.4 入道）。
pub const ENTRY_REQ_DEMON_TURBID: f32 = 0.5;
/// 7.1 `COMING_OF_AGE` 入道年齡 12 歲（GDD 3.4 入道）。
pub const COMING_OF_AGE_YEARS: u32 = 12;
/// 7.1 `MARRY` 婚配最低年齡 18 歲（GDD 3.4 婚配）。
pub const MARRY_AGE: u32 = 18;
/// 7.1 `MARRY` 每年婚配機率 10%（GDD 3.4 婚配）。
pub const MARRY_CHANCE: f32 = 0.10;
/// 7.1 `MARRY` 婚配的年齡差上限 10 歲（GDD 3.4 婚配）。
pub const MARRY_AGE_GAP_MAX: u32 = 10;
/// 7.1 `ROYAL_BIRTH` 王族生育機率 15%（GDD 3.4 王族）。
pub const ROYAL_BIRTH_CHANCE: f32 = 0.15;
/// 7.1 `ROYAL_MAX` 王族子嗣上限 5 人（GDD 3.4 王族）。
pub const ROYAL_MAX_CHILDREN: u32 = 5;
/// 7.1 `MORTAL_LIFESPAN` 凡人壽元常態分布的平均 62 歲，即 N(62, 12)（GDD 3.4 壽元）。
pub const MORTAL_LIFESPAN_MEAN: f32 = 62.0;
/// 7.1 `MORTAL_LIFESPAN` 凡人壽元常態分布的標準差 12，即 N(62, 12)（GDD 3.4 壽元）。
pub const MORTAL_LIFESPAN_STD: f32 = 12.0;
/// 7.1 `MORTAL_LIFESPAN` 凡人壽元截斷下限 30 歲，即 30..100（GDD 3.4 壽元）。
pub const MORTAL_LIFESPAN_MIN: u32 = 30;
/// 7.1 `MORTAL_LIFESPAN` 凡人壽元截斷上限 100 歲，即 30..100（GDD 3.4 壽元）。
pub const MORTAL_LIFESPAN_MAX: u32 = 100;
/// 7.1 `MORTAL_LIFESPAN` 僧人壽元加成 10 歲（GDD 3.4 壽元）。
pub const MONK_LIFESPAN_BONUS: u32 = 10;
/// 7.1 `NOTABLE_CAP` 有名之人在世上限，依規模 [S, M, L] = [2,500, 4,000, 5,500]（含幼年者；GDD 3.4 在世上限）。
pub const NOTABLE_CAP: [u32; 3] = [2_500, 4_000, 5_500];
/// 7.1 `NOTABLE_FLOOR_RATIO` 補魂門檻比例 0.5，即 NOTABLE_FLOOR = NOTABLE_CAP × 0.5（GDD 3.4 人間補魂）。
pub const NOTABLE_FLOOR_RATIO: f32 = 0.5;
/// 7.1 `REFILL_RATE` 人間補魂速率 0.1，每年建立 ceil((NOTABLE_FLOOR − 在世數) × 0.1) 個新魂（GDD 3.4 人間補魂）。
pub const REFILL_RATE: f32 = 0.1;

// ---- 道（GDD 3.5）----
// 境界索引 0 起算：0 煉氣、1 築基、2 金丹、3 元嬰、4 化神、5 煉虛、6 合體、7 渡劫、8 大乘（GDD 3.5 境界表）。

/// 7.1 `REALM_EXP` 各境界修滿所需修為 E，依境界索引 0..=8：12, 14, 17, 25, 38, 57, 85, 128, 192（M2 校準 U10；GDD 3.5 境界表）。
pub const REALM_EXP: [f32; 9] = [12.0, 14.0, 17.0, 25.0, 38.0, 57.0, 85.0, 128.0, 192.0];
/// 7.1 `LAYER` 小境界層數上限 10，即 min(10, …)（GDD 3.5 小境界；只用於顯示與戰力）。
pub const LAYER_MAX: u32 = 10;
/// 7.1 `LAYER` 小境界層數比例係數 10，即 floor(修為 / E × 10) + 1（GDD 3.5 小境界）。
pub const LAYER_SCALE: f32 = 10.0;
/// 7.1 `REALM_LIFESPAN` 各境界壽元（年），依境界索引 0..=8：120, 200, 400, 700, 1200, 2000, 3000, 5000, 8000（M2 校準 U10：維持設計值；GDD 3.5 境界表）。
pub const REALM_LIFESPAN: [u32; 9] = [120, 200, 400, 700, 1_200, 2_000, 3_000, 5_000, 8_000];
/// 7.1 `REALM_BT` 各境界突破到下一境的基礎率 B，依境界索引 0..=8：0.08, 0.16, 0.17 × 6（金丹至渡劫）, 0.15（大乘，即飛升劫）（M2 校準 U10；GDD 3.5 境界表）。
pub const REALM_BT: [f32; 9] = [0.08, 0.16, 0.17, 0.17, 0.17, 0.17, 0.17, 0.17, 0.15];
/// 7.1 `CULT_ROOT` 根骨係數的基數 0.6，即 0.6 + 0.24 × 根骨（GDD 3.5 修煉）。
pub const CULT_ROOT_BASE: f32 = 0.6;
/// 7.1 `CULT_ROOT` 根骨係數的根骨係數 0.24（GDD 3.5 修煉）。
pub const CULT_ROOT_COEF: f32 = 0.24;
/// 7.1 `AURA` 靈氣係數的基數 0.5，即 0.5 + 1.5 × 靈氣（GDD 3.5 修煉）。
pub const CULT_AURA_BASE: f32 = 0.5;
/// 7.1 `AURA` 靈氣係數的靈氣係數 1.5（GDD 3.5 修煉）。
pub const CULT_AURA_COEF: f32 = 1.5;
/// 7.1 `WISDOM` 宿慧係數的基數 1，即 1 + W/200（GDD 3.5 修煉）。
pub const CULT_WISDOM_BASE: f32 = 1.0;
/// 7.1 `WISDOM` 宿慧係數的除數 200，即 1 + W/200（GDD 3.5 修煉）。
pub const CULT_WISDOM_DIV: f32 = 200.0;
/// 7.1 `CULT_NOISE` 修煉隨機係數的下限 0.8，即 U(0.8, 1.2)（GDD 3.5 修煉）。
pub const CULT_NOISE_MIN: f32 = 0.8;
/// 7.1 `CULT_NOISE` 修煉隨機係數的上限 1.2，即 U(0.8, 1.2)（GDD 3.5 修煉）。
pub const CULT_NOISE_MAX: f32 = 1.2;
/// 7.1 `CULT_SECT` 宗門弟子的修煉倍率 1.2（GDD 3.5 修煉）。
pub const CULT_SECT: f32 = 1.2;
/// 7.1 `CULT_NODE` 山門在洞天福地上的加成係數 0.3，即 × (1 + 節點強度 × 0.3)（GDD 3.5 修煉）。
pub const CULT_NODE_COEF: f32 = 0.3;
/// 7.1 `CULT_ROGUE` 散修的修煉倍率 1.0（GDD 3.5 修煉）。
pub const CULT_ROGUE: f32 = 1.0;
/// 7.1 `BLESS_CULT` 賜福期間修煉倍率 2（GDD 3.5 修煉；4.2 賜福）。
pub const BLESS_CULT_MULT: f32 = 2.0;
/// 7.1 `BLESS_BT` 賜福期間突破成功率倍率 1.5（GDD 3.5 突破；4.2 賜福）。
pub const BLESS_BT_MULT: f32 = 1.5;
/// 7.1 `BT_COOLDOWN` 突破冷卻 3 年（36 個月；GDD 3.5 突破，冷卻以月數計）。
pub const BT_COOLDOWN_YEARS: u32 = 3;
/// 7.1 `BT_INSIGHT` 突破成功率的悟性基數 0.8，即 0.8 + 悟性 × 0.04（GDD 3.5 突破）。
pub const BT_INSIGHT_BASE: f32 = 0.8;
/// 7.1 `BT_INSIGHT` 突破成功率的悟性係數 0.04（GDD 3.5 突破）。
pub const BT_INSIGHT_COEF: f32 = 0.04;
/// 7.1 `BT_MAX` 突破成功率上限 0.95（GDD 3.5 突破）。
pub const BT_MAX: f32 = 0.95;
/// 7.1 `BT_FAIL_KEEP` 突破失敗後修為保留比例 0.85（GDD 3.5 突破）。
pub const BT_FAIL_KEEP: f32 = 0.85;
/// 7.1 `BT_FAIL_DEATH` 走火身死機率的基數 0.1，即 0.1 + 0.03 × 當前境界（GDD 3.5 突破）。
pub const BT_FAIL_DEATH_BASE: f32 = 0.1;
/// 7.1 `BT_FAIL_DEATH` 走火身死機率的境界係數 0.03（GDD 3.5 突破）。
pub const BT_FAIL_DEATH_PER_REALM: f32 = 0.03;
/// 7.1 `BT_FAIL_DEATH` 適用的最低當前境界 2（金丹，境界索引；GDD 3.5 突破）。
pub const BT_FAIL_DEATH_MIN_REALM: u8 = 2;
/// 7.1 `TRIB_SURVIVE` 渡劫存活率的基數 0.92（GDD 3.5 天劫）。
pub const TRIB_SURVIVE_BASE: f32 = 0.92;
/// 7.1 `TRIB_SURVIVE` 渡劫存活率分母的基數 60，即 (60 + 40 × 新境界)（GDD 3.5 天劫）。
pub const TRIB_SURVIVE_DIV_BASE: f32 = 60.0;
/// 7.1 `TRIB_SURVIVE` 渡劫存活率分母的新境界係數 40（GDD 3.5 天劫）。
pub const TRIB_SURVIVE_DIV_PER_REALM: f32 = 40.0;
/// 7.1 `TRIB_ASCEND` 飛升劫存活率的基數 0.80（GDD 3.5 天劫）。
pub const TRIB_ASCEND_BASE: f32 = 0.80;
/// 7.1 `TRIB_ASCEND` 飛升劫存活率的分母 300（GDD 3.5 天劫）。
pub const TRIB_ASCEND_DIV: f32 = 300.0;
/// 7.1 `TRIB_SHED_BODY` 天劫失敗中轉為兵解的機率 40%（其餘為身死道消；GDD 3.5 天劫）。
pub const TRIB_SHED_BODY_CHANCE: f32 = 0.40;
/// 7.1 `TRIB_BOLTS` 天劫落雷數為新境界 + 1，此為額外的 1 道（GDD 3.5 天劫）。
pub const TRIB_BOLTS_EXTRA: u32 = 1;
/// 7.1 `HEART_DEMON` 心魔強度的淨業力偏移 80，即 (S − G − 80) / 1000（GDD 3.5 心魔）。
pub const HEART_DEMON_OFFSET: f32 = 80.0;
/// 7.1 `HEART_DEMON` 心魔強度的除數 1000（GDD 3.5 心魔）。
pub const HEART_DEMON_DIV: f32 = 1000.0;
/// 7.1 `HEART_DEMON` 修羅道心魔倍率 2（GDD 3.5 心魔）。
pub const HEART_DEMON_ASURA_MULT: f32 = 2.0;
/// 7.1 `TRAVEL` 遊歷每年觸發機率 10%（GDD 3.5 遊歷）。
pub const TRAVEL_CHANCE: f32 = 0.10;
/// 7.1 `TRAVEL` 遊歷目的地距離上限 3 步，沿陸鄰（GDD 3.5 遊歷）。
pub const TRAVEL_RANGE_STEPS: u32 = 3;
/// 7.1 `TRAVEL` 遊歷停留年數下限 1 年（12 個月；GDD 3.5 遊歷，程式換算月數時 × 12）。
pub const TRAVEL_STAY_YEARS_MIN: u32 = 1;
/// 7.1 `TRAVEL` 遊歷停留年數上限 5 年（60 個月；GDD 3.5 遊歷，程式換算月數時 × 12）。
pub const TRAVEL_STAY_YEARS_MAX: u32 = 5;
/// 7.1 `TRAVEL` 遊歷的最低境界 1（築基，境界索引；GDD 3.5 遊歷）。
pub const TRAVEL_MIN_REALM: u8 = 1;
/// 7.1 `HUNT` 斬妖除魔每年觸發機率 10%（GDD 3.5 斬妖除魔）。
pub const HUNT_CHANCE: f32 = 0.10;
/// 7.1 `HUNT` 斬妖除魔的最低境界 2（金丹，境界索引；GDD 3.5 斬妖除魔）。
pub const HUNT_MIN_REALM: u8 = 2;
/// 7.1 `HUNT` 斬妖除魔的淨業力下限 0（淨業力 ≥ 0；GDD 3.5 斬妖除魔）。
pub const HUNT_MIN_NET_KARMA: f32 = 0.0;
/// 7.1 `SECT_FOUND` 散修創立宗門的每年機率 3%（GDD 3.5 宗門創立）。
pub const SECT_FOUND_CHANCE: f32 = 0.03;
/// 7.1 `SECT_FOUND` 散修創立宗門的最低境界 2（金丹，境界索引；GDD 3.5 宗門創立）。
pub const SECT_FOUND_MIN_REALM: u8 = 2;
/// 7.1 `SECT_FOUND` 山門之間的最小間距 2 步（GDD 3.5 宗門創立）。
pub const SECT_FOUND_MIN_SPACING_STEPS: u32 = 2;
/// 7.1 `SECT_SIZE` 宗門規模的基數 5，即 5 + 聲望 / 20（GDD 3.5 宗門規模）。
pub const SECT_SIZE_BASE: u32 = 5;
/// 7.1 `SECT_SIZE` 宗門規模的聲望除數 20，即 5 + 聲望 / 20（GDD 3.5 宗門規模）。
pub const SECT_SIZE_PRESTIGE_DIV: f32 = 20.0;
/// 7.1 `SECT_SIZE` 宗門規模上限 40（GDD 3.5 宗門規模）。
pub const SECT_SIZE_MAX: u32 = 40;
/// 7.1 `SECT_JOIN_ROGUE` 散修每年投靠宗門的機率 2%（GDD 3.5 投靠）。
pub const SECT_JOIN_ROGUE_CHANCE: f32 = 0.02;
/// 7.1 `MAX_DISCIPLES` 長老可收徒弟的上限 5（徒弟 < 5；GDD 3.5 收徒）。
pub const MAX_DISCIPLES: u32 = 5;
/// 7.1 `MAX_DISCIPLES` 長老的最低境界 2（金丹，境界索引；GDD 3.5 長老）。
pub const SECT_ELDER_MIN_REALM: u8 = 2;
/// 7.1 `SECT_RADIUS` 勢力範圍半徑的基數 1，即 1 + floor(log2(1 + 聲望/30))（GDD 3.5 勢力範圍）。
pub const SECT_RADIUS_BASE: u32 = 1;
/// 7.1 `SECT_RADIUS` 勢力範圍的聲望除數 30（GDD 3.5 勢力範圍）。
pub const SECT_RADIUS_PRESTIGE_DIV: f32 = 30.0;
/// 7.1 `SECT_RADIUS` 勢力範圍半徑上限 4（GDD 3.5 勢力範圍）。
pub const SECT_RADIUS_MAX: u32 = 4;
/// 7.1 `PRESTIGE` 聲望公式值中每名飛升者的加成 50，即 Σ 成員戰力^0.5 + 飛升人數 × 50（GDD 3.5 聲望）。
pub const PRESTIGE_ASCENDED: f32 = 50.0;
/// 7.1 `PRESTIGE_MOD_DECAY` 宗門聲望修正每年衰減倍率 0.95（GDD 3.5 聲望）。
pub const PRESTIGE_MOD_DECAY: f32 = 0.95;
/// 7.1 `SECT_WAR` 宗門爭脈之戰每年發生機率 4%（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_CHANCE: f32 = 0.04;
/// 7.1 `SECT_WAR` 敗方前 5 名於戰中戰死的機率 30%（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_LOSER_DEATH_CHANCE: f32 = 0.30;
/// 7.1 `SECT_WAR` 勝方於戰中戰死的機率 10%（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_WINNER_DEATH_CHANCE: f32 = 0.10;
/// 7.1 `SECT_WAR_TOP` 結算時取戰力前 5 名參戰（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_TOP: u32 = 5;
/// 7.1 `SECT_WAR_KILLERS` 殺者自對方前 3 名挑選（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_KILLERS: u32 = 3;
/// 7.1 `SECT_WAR_FALL` 戰力剩餘低於 20% 即覆滅（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_FALL: f32 = 0.2;
/// 7.1 `SECT_SPLIT` 分裂條件：次強戰力 ≥ 最強的 70%（GDD 3.5 分裂）。
pub const SECT_SPLIT_SECOND_RATIO: f32 = 0.70;
/// 7.1 `SECT_SPLIT` 分裂條件：成員數 ≥ 8（GDD 3.5 分裂）。
pub const SECT_SPLIT_MIN_MEMBERS: u32 = 8;
/// 7.1 `SECT_SPLIT` 分裂機率 35%（GDD 3.5 分裂）。
pub const SECT_SPLIT_CHANCE: f32 = 0.35;
/// 7.1 `SECT_SPLIT_RELATION` 分裂後兩宗的關係值 −80（GDD 3.5 分裂）。
pub const SECT_SPLIT_RELATION: f32 = -80.0;
/// 7.1 `SECT_WAR_MUL` 爭脈之戰戰力總和的隨機係數下限 0.6，即 U(0.6, 1.4)（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_MUL_MIN: f32 = 0.6;
/// 7.1 `SECT_WAR_MUL` 爭脈之戰戰力總和的隨機係數上限 1.4，即 U(0.6, 1.4)（GDD 3.5 爭脈之戰）。
pub const SECT_WAR_MUL_MAX: f32 = 1.4;
/// 7.1 `EVIL_SECT_RELATION` 正道與邪道宗門的初始關係值 −50（GDD 3.5 宗門關係）。
pub const EVIL_SECT_RELATION: f32 = -50.0;

// ---- 開天（GDD 4.6）----

/// 7.1 `GENESIS_MERIT` 先天魂功德 G 的下限 0，即 U(0, 120)（GDD 4.6 開天）。
pub const GENESIS_MERIT_MIN: f32 = 0.0;
/// 7.1 `GENESIS_MERIT` 先天魂功德 G 的上限 120，即 U(0, 120)（GDD 4.6 開天）。
pub const GENESIS_MERIT_MAX: f32 = 120.0;
/// 7.1 `GENESIS_SIN` 先天魂殺業 S 的下限 0，即 U(0, 80)（GDD 4.6 開天）。
pub const GENESIS_SIN_MIN: f32 = 0.0;
/// 7.1 `GENESIS_SIN` 先天魂殺業 S 的上限 80，即 U(0, 80)（GDD 4.6 開天）。
pub const GENESIS_SIN_MAX: f32 = 80.0;
/// 7.1 `GENESIS_SIN` 修士的先天殺業再乘 0.5（GDD 4.6 開天）。
pub const GENESIS_SIN_CULTIVATOR_MULT: f32 = 0.5;
/// 7.1 `GENESIS_WISDOM` 先天魂宿慧 W 的下限 0，即 U(0, 40)（GDD 4.6 開天）。
pub const GENESIS_WISDOM_MIN: f32 = 0.0;
/// 7.1 `GENESIS_WISDOM` 先天魂宿慧 W 的上限 40，即 U(0, 40)（GDD 4.6 開天）。
pub const GENESIS_WISDOM_MAX: f32 = 40.0;
/// 7.1 `GENESIS_AFFINITY` 出生州 faith4 的權重 0.5，即 normalize(faith4 × 0.5 + 身分偏向 × 0.5)（GDD 4.6 開天）。
pub const GENESIS_AFFINITY_FAITH_WEIGHT: f32 = 0.5;
/// 7.1 `GENESIS_AFFINITY` 身分偏向的權重 0.5（GDD 4.6 開天）。
pub const GENESIS_AFFINITY_STATUS_WEIGHT: f32 = 0.5;
/// 7.1 `GENESIS_AFFINITY` 身分偏向對應教派的分量 1（修士 道、僧 釋、儒士 儒、魔修 魔；散人均等，GDD 4.6 開天）。
pub const GENESIS_AFFINITY_STATUS_BIAS: f32 = 1.0;
/// 7.1 `GENESIS_THREADS` 開局宿緣數係數 0.1，即 有名之人數 × 0.1（GDD 4.6 開天）。
pub const GENESIS_THREADS_PER_NOTABLE: f32 = 0.1;
/// 7.1 `GENESIS_THREADS` 開局宿緣雙方的距離上限 3 步（GDD 4.6 開天）。
pub const GENESIS_THREAD_RANGE_STEPS: u32 = 3;
/// 7.1 `GENESIS_PYRAMID` 修士每高一境的人數比例 0.35（GDD 4.6 開天）。
pub const GENESIS_PYRAMID_RATIO: f32 = 0.35;
/// 7.1 `GENESIS_AGE_MAX` 開局年齡下限 12 歲，年齡均勻分布於 12 至 壽元 × 0.6（GDD 4.6 開天）。
pub const GENESIS_AGE_MIN: u32 = 12;
/// 7.1 `GENESIS_AGE_MAX` 開局年齡上限為壽元 × 0.6（GDD 4.6 開天）。
pub const GENESIS_AGE_MAX_LIFESPAN_RATIO: f32 = 0.6;
/// 7.1 `GENESIS_PRERUN_YEARS` 開局預跑 10 年（GDD 4.6 開天）。
pub const GENESIS_PRERUN_YEARS: u32 = 10;
/// 7.1 `GENESIS_PRERUN_YEARS` 預跑的 tick 範圍為 1..=120（10 年 × 12 個月；GDD 4.6 開天）。
pub const GENESIS_PRERUN_TICK_END: u32 = 120;
/// 7.1 `GENESIS_HIGH_REALM` 中古、末法開局保證的渡劫以上修士人數 2（GDD 4.6 開天）。
pub const GENESIS_HIGH_REALM_COUNT: u32 = 2;
/// 7.1 `GENESIS_HIGH_REALM` 上述修士的最低境界 7（渡劫，境界索引；GDD 4.6 開天與 3.5 境界表）。
pub const GENESIS_HIGH_REALM_MIN_REALM: u8 = 7;
/// 7.1 `GENESIS_FORCED_BT` 開局保證的強制突破於 tick 123，即預跑結束後第 3 個月（GDD 4.6 開天）。
pub const GENESIS_FORCED_BT_TICK: u32 = 123;
/// 7.1 `GENESIS_SECTS` 開天宗門數（中尺寸），依 [上古, 中古, 末法] 排列（GDD 4.6 開局時代）。
pub const GENESIS_SECTS: [u32; 3] = [12, 10, 5];
/// 7.1 `GENESIS_NOTABLE` 初始有名之人數佔 `NOTABLE_CAP` 的比例，依 [上古, 中古, 末法] 排列（GDD 4.6 開局時代）。
pub const GENESIS_NOTABLE_RATIO: [f32; 3] = [0.3, 0.75, 0.75];
/// 7.1 `GENESIS_ROAMERS` 上古開局另有的遊方僧數（中尺寸；GDD 4.6 開局時代）。
pub const GENESIS_ROAMING_MONKS_ANCIENT: u32 = 20;
/// 7.1 `GENESIS_ROAMERS` 上古開局另有的散儒數（中尺寸；GDD 4.6 開局時代）。
pub const GENESIS_ROAMING_SCHOLARS_ANCIENT: u32 = 20;

// ---- 魔修（GDD 3.10；M2 僅列魔修相關者）----

/// 7.1 `DEMON_CULT_KARMA` 入魔的業氣基數 0.5，即 0.5 + 2 × 業氣（GDD 3.10 魔修）。
pub const DEMON_CULT_KARMA_BASE: f32 = 0.5;
/// 7.1 `DEMON_CULT_KARMA` 入魔業氣的業氣係數 2（GDD 3.10 魔修）。
pub const DEMON_CULT_KARMA_COEF: f32 = 2.0;
/// 7.1 `DEMON_KILL_EXP` 魔修斬殺的修為倍率 2，即 戰力 × 2（GDD 3.10 魔修）。
pub const DEMON_KILL_EXP: f32 = 2.0;
/// 7.1 `MASSACRE_EXP`（魔修屠城，記為 `DEMON_MASSACRE_EXP`）的修為增量 5，即每萬凡人 +5（GDD 3.10 魔修）。
pub const DEMON_MASSACRE_EXP: f32 = 5.0;
/// 7.1 `MASSACRE_EXP` 的人口單位 10,000 人，即每萬凡人（GDD 3.10 魔修）。
pub const DEMON_MASSACRE_EXP_UNIT: u32 = 10_000;
/// 7.1 `DEMON_ASSASSINATE` 魔修暗殺係數 0.02，即 戾氣 × 0.02（GDD 3.10 魔修）。
pub const DEMON_ASSASSINATE_COEF: f32 = 0.02;

// ---- 天書（GDD 4.5，U7）----

/// 7.1 `BOOK_REINCARNATED` 天書「轉世大能」前世最高境界下限 2（金丹，境界索引；GDD 4.5）。
pub const BOOK_REINCARNATED_MIN_REALM: u8 = 2;
/// 7.1 `BOOK_REINCARNATED` 天書「轉世大能」今生達成的年齡上限，須小於 50 歲（GDD 4.5）。
pub const BOOK_REINCARNATED_MAX_AGE: u32 = 50;
/// 7.1 `BOOK_THREE_LIVES` 天書「三世血仇」殺業線了結時的最低 `lives` 2（GDD 4.5）。
pub const BOOK_THREE_LIVES_MIN: u16 = 2;
/// 7.1 `BOOK_REUNITED` 天書「再續前緣」情緣線了結時的最低 `lives` 2（GDD 4.5）。
pub const BOOK_REUNITED_MIN_LIVES: u16 = 2;
/// 7.1 `TEACH_MERIT` 師父每收一徒的功德 +2（GDD 3.1 傳道授業）。
pub const MASTER_TEACH_MERIT: f32 = 2.0;
