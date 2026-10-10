//! GDD 6.2 資料模型：M1 所需的公開型別。
//!
//! 這些型別是 sim 與 app／cli 之間的契約；欄位意義見各章節。

use serde::{Deserialize, Serialize};

/// 模擬的一步 = 一個月（GDD 2.6）。
pub type Tick = u32;

/// 州郡 ID；海、湖格以 [`NO_PROVINCE`] 表示。
pub type ProvinceId = u16;
pub const NO_PROVINCE: ProvinceId = u16::MAX;

/// 洞天福地 ID（GDD 2.4）。
pub type NodeId = u16;

/// GDD 2.2 地形代碼。數值即代碼，不可更動。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Terrain {
    Sea = 0,
    Lake = 1,
    Plains = 2,
    Hills = 3,
    Mountain = 4,
    Forest = 5,
    Snow = 6,
    Desert = 7,
    Swamp = 8,
}

impl Terrain {
    pub const COUNT: usize = 9;
    pub const ALL: [Terrain; 9] = [
        Terrain::Sea,
        Terrain::Lake,
        Terrain::Plains,
        Terrain::Hills,
        Terrain::Mountain,
        Terrain::Forest,
        Terrain::Snow,
        Terrain::Desert,
        Terrain::Swamp,
    ];

    pub fn is_water(self) -> bool {
        matches!(self, Terrain::Sea | Terrain::Lake)
    }

    pub fn name_zh(self) -> &'static str {
        match self {
            Terrain::Sea => "海",
            Terrain::Lake => "湖",
            Terrain::Plains => "平原",
            Terrain::Hills => "丘陵",
            Terrain::Mountain => "山脈",
            Terrain::Forest => "森林",
            Terrain::Snow => "雪原",
            Terrain::Desert => "沙漠",
            Terrain::Swamp => "沼澤",
        }
    }
}

/// GDD 4.6 大陸尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    Small,
    Medium,
    Large,
}

/// GDD 4.6 開局時代。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era {
    Ancient,
    Middle,
    Decline,
}

/// GDD 4.6 開局設定。`seed` 為最終使用的種子（品質檢查重試後，GDD 2.2）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldParams {
    pub name: String,
    pub seed: u64,
    pub size: WorldSize,
    pub land_ratio: f32,
    pub relief: f32,
    pub humidity: f32,
    pub era: Era,
}

impl WorldParams {
    /// 以 GDD 7.1 的預設值建立。
    pub fn new(seed: u64) -> Self {
        Self {
            name: String::new(),
            seed,
            size: WorldSize::Medium,
            land_ratio: crate::params::WG_LAND_RATIO,
            relief: crate::params::WG_RELIEF,
            humidity: crate::params::WG_HUMIDITY,
            era: Era::Middle,
        }
    }
}

/// GDD 2.4 洞天福地。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeyNode {
    pub cell: u32,
    /// 強度 s：1.0 大洞天、0.75 洞天、0.5 福地。
    pub strength: f32,
    pub name: String,
}

/// GDD 2.4 靈脈：連接兩個節點，沿 `path` 的格點描出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeyEdge {
    pub a: NodeId,
    pub b: NodeId,
    pub strength: f32,
    pub path: Vec<u32>,
}

/// 已命名的江河（GDD 2.2 第 6 步）。`cells` 為幹流，由源頭到出海口。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedRiver {
    pub name: String,
    pub level: u8,
    pub cells: Vec<u32>,
}

/// 已命名的山脈（GDD 7.3）。`peak` 為最高格；`cells` 為該山脈的山脈格。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedRange {
    pub name: String,
    pub peak: u32,
    pub cells: Vec<u32>,
}

/// GDD 2.2 世界生成的格點輸出。所有格點陣列為列優先：`index = y * w + x`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geography {
    pub w: u16,
    pub h: u16,
    /// GDD 2.2 尺寸：等距圓柱投影，x 東西環繞（格 x = W − 1 與 x = 0 相鄰）。
    pub wrap_x: bool,
    pub sea_level: f32,
    /// 0..1，填窪後的高度。
    pub height: Vec<f32>,
    pub terrain: Vec<Terrain>,
    /// 0 無；1 小河；2 大河；3 江。
    pub river: Vec<u8>,
    pub province_of: Vec<ProvinceId>,
    /// GDD 2.4 格點靈氣，0..2。
    pub aura_cell: Vec<f32>,
    pub nodes: Vec<LeyNode>,
    pub ley: Vec<LeyEdge>,
    pub rivers: Vec<NamedRiver>,
    pub ranges: Vec<NamedRange>,
    /// GDD 2.2 步驟 9 大洲：八鄰連通的陸塊（面積 ≥ `WG_CONTINENT_MIN`），依面積降冪。
    pub continents: Vec<Continent>,
    /// 每格所屬大洲的索引；海格與未命名的小島為 [`NO_CONTINENT`]。
    pub continent_of: Vec<u16>,
}

/// 未命名陸塊或水格的大洲索引。
pub const NO_CONTINENT: u16 = u16::MAX;

/// GDD 2.2「地理解析度」（決策 #35）：世界生成在比遊戲格細 `scale` 倍的格點上進行，遊戲格由其降採樣；
/// 細格資料保留給渲染（6.4 的契約：高度起伏、生態漸層、河寬、海岸線）。所有陣列列優先，`index = y * w + x`，
/// 細格 `(X, Y)` 對應遊戲格 `(X / scale, Y / scale)`。`scale = 0` 表示沒有細格資料（開發用的手工世界）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FineGeography {
    pub scale: u8,
    pub w: u16,
    pub h: u16,
    pub sea_level: f32,
    /// 高度 0..1 量化為 0..65535（填窪後）。
    pub height: Vec<u16>,
    pub terrain: Vec<Terrain>,
    /// 0 無；1 小河；2 大河；3 江。
    pub river: Vec<u8>,
    /// 河流流量：集水降水量的對數刻度 0..255（`255·ln(1 + acc) / ln(1 + max)`），供河寬。
    pub flow: Vec<u8>,
    /// 溫度 t 量化：`clamp(t, 0, 1)·255`。
    pub temperature: Vec<u8>,
    /// 濕度 q 量化：`clamp(q, 0, 1)·255`。
    pub moisture: Vec<u8>,
}

impl FineGeography {
    pub fn is_empty(&self) -> bool {
        self.scale == 0
    }
    pub fn len(&self) -> usize {
        self.w as usize * self.h as usize
    }
    /// 細格高度 0..1。
    pub fn height_at(&self, i: usize) -> f32 {
        self.height[i] as f32 / 65535.0
    }
}

/// GDD 2.2 步驟 9 大洲（陸塊）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Continent {
    pub name: String,
    /// 格數。
    pub cells: u32,
    /// 面積（Σ cos(lat)，赤道格）。
    pub area: f32,
    /// 面積加權中心最近的陸地格。
    pub centroid: u32,
}

impl Geography {
    pub fn len(&self) -> usize {
        self.w as usize * self.h as usize
    }

    /// GDD 2.2 尺寸：列 y 的緯度（弧度；y = 0 為北）：`π/2 − π(y + 0.5)/H`。
    pub fn row_lat(&self, y: u16) -> f32 {
        crate::worldgen::row_lat(self.h as usize, y as usize)
    }

    /// GDD 2.2 尺寸：欄 x 的經度（弧度）：`2π(x + 0.5)/W`。
    pub fn col_lon(&self, x: u16) -> f32 {
        crate::worldgen::col_lon(self.w as usize, x as usize)
    }

    /// GDD 2.2 尺寸：格的面積權重 cos(lat)。
    pub fn area_weight(&self, cell: u32) -> f32 {
        crate::worldgen::cos(self.row_lat((cell / self.w as u32) as u16))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn idx(&self, x: u16, y: u16) -> u32 {
        y as u32 * self.w as u32 + x as u32
    }

    pub fn xy(&self, cell: u32) -> (u16, u16) {
        ((cell % self.w as u32) as u16, (cell / self.w as u32) as u16)
    }
}

/// GDD 2.3 州郡。M1 使用的欄位之外，其餘氣場欄位先保留為 0。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Province {
    pub id: ProvinceId,
    pub name: String,
    /// 所屬格點，依 cell 升冪。
    pub cells: Vec<u32>,
    /// 治所格（宜居度最高的格）。
    pub seat: u32,
    pub neighbors: Vec<ProvinceId>,
    pub sea_neighbors: Vec<ProvinceId>,
    /// 各地形格數，以 [`Terrain`] 代碼為索引。
    pub terrain_mix: [u32; Terrain::COUNT],
    pub habitability: f64,
    pub capacity: f64,
    pub population: f64,
    /// 0..1（GDD 2.4）。
    pub aura: f32,
    pub karma_qi: f32,
    pub merit: f32,
    pub righteous: f32,
    pub devastation: f32,
    /// 道、釋、儒（GDD 2.3）。
    pub faith: [f32; 3],
    /// 0 村 .. 4 都（GDD 2.3）。
    pub city_level: u8,
}

/// 世界生成失敗（品質檢查連續失敗 `WG_RETRY` 次，GDD 2.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldGenError {
    pub last_seed: u64,
    pub reason: String,
}

impl std::fmt::Display for WorldGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "世界生成失敗（最後種子 {}）：{}", self.last_seed, self.reason)
    }
}

impl std::error::Error for WorldGenError {}
