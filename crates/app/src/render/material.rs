//! GDD 6.4 地圖材質：自訂 `Material` + `terrain_ink.wgsl`（5.3 步驟 1–8）。
#![allow(dead_code)] // ShaderType 衍生巨集會產生未使用的 check 函式
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};

use super::palette::*;

/// uniform 區塊（GDD 9.4：色值全由此傳入）。欄位順序需與 WGSL 的 `InkParams` 一致。
#[derive(Clone, Copy, ShaderType, Debug)]
#[allow(dead_code)]
pub struct InkParams {
    pub paper: Vec4,
    pub paper_dark: Vec4,
    pub ink: Vec4,
    pub ink_light: Vec4,
    pub water: Vec4,
    pub water_deep: Vec4,
    pub gold: Vec4,
    pub cyan: Vec4,
    pub seal_red: Vec4,
    pub terrain: [Vec4; 9],
    /// 海深色階（淺 → 深）。
    pub sea: [Vec4; 5],
    /// 山地高度色階（低 → 高）。
    pub ramp: [Vec4; 5],
    /// x 高度縮放、y 古地圖化權重、z 地圖模式（0 山河 1 天下 … 3 靈脈）、w 時間（秒）。
    pub view: Vec4,
    /// x 停留州郡（-1 無）、y 選取州郡（-1 無）、z 州郡疊色不透明度、w 因果模式權重（M1 恆為 0）。
    pub sel: Vec4,
    /// x 地圖寬、y 地圖高、z 最高點高度（縮放 1）、w 靈脈場不透明度。
    pub map: Vec4,
    /// x 霧起點、y 霧終點（鏡頭距離，世界單位）、z 邊緣淡出寬度、w 保留。
    pub fog: Vec4,
    /// x 球面過渡權重（0 平面、1 球面）、y 球半徑、z 球面高度位移倍率、w 東西環繞（1 = 是）。
    pub globe: Vec4,
    /// xyz 指向光源的單位向量（地形山影；預設西北光）、w 山影強度。燈光層的掛鉤（6.4）。
    pub light: Vec4,
    /// 球面局部座標 → 世界座標（旋轉 + 球心平移；GDD 5.3 星球視角）。
    pub grot: Mat4,
}

impl InkParams {
    pub fn new(map_w: f32, map_h: f32, max_height: f32, wrap_x: bool) -> Self {
        Self {
            paper: lin(PAPER),
            paper_dark: lin(PAPER_DARK),
            ink: lin(INK),
            ink_light: lin(INK_LIGHT),
            water: lin(WATER),
            water_deep: lin(WATER_DEEP),
            gold: lin(GOLD),
            cyan: lin(CYAN),
            seal_red: lin(SEAL_RED),
            terrain: terrain_colors(),
            sea: sea_ramp(),
            ramp: mountain_ramp(),
            view: Vec4::new(1.0, 0.0, 0.0, 0.0),
            sel: Vec4::new(-1.0, -1.0, 0.45, 0.0),
            map: Vec4::new(map_w, map_h, max_height, 0.7),
            fog: Vec4::new(1e6, 2e6, 6.0, 0.0),
            globe: Vec4::new(0.0, map_w / std::f32::consts::TAU, 2.0, if wrap_x { 1.0 } else { 0.0 }),
            light: Vec3::new(-0.6, 0.7, -0.5).normalize().extend(1.0),
            grot: Mat4::IDENTITY,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TerrainInkMaterial {
    #[uniform(0)]
    pub params: InkParams,
    /// 紙紋（重複 + 線性），另兩張以相同取樣器取樣。
    #[texture(1)]
    #[sampler(2)]
    pub paper: Handle<Image>,
    #[texture(3)]
    pub stroke: Handle<Image>,
    /// 地形類別權重（平原、丘陵、山脈、森林），格點大小、可線性取樣。
    #[texture(4)]
    pub cls_a: Handle<Image>,
    /// 地形類別權重（雪原、沙漠、沼澤）。
    #[texture(10)]
    pub cls_b: Handle<Image>,
    #[texture(5, sample_type = "u_int")]
    pub province_id: Handle<Image>,
    #[texture(6, filterable = false)]
    pub province_color: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub field: Handle<Image>,
    #[texture(9)]
    pub terrain: Handle<Image>,
}

impl Material for TerrainInkMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain_ink.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_ink.wgsl".into()
    }
}

/// 靈脈線與節點標記（GDD 5.4 靈脈模式的額外圖層）。
#[derive(Clone, Copy, ShaderType, Debug)]
#[allow(dead_code)]
pub struct LeyParams {
    pub cyan: Vec4,
    pub gold: Vec4,
    pub ink: Vec4,
    /// x 高度縮放、y 寬度（世界單位）、z 時間、w 模式（0 線帶、1 節點圓盤）。
    pub view: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct LeyMaterial {
    #[uniform(0)]
    pub params: LeyParams,
}

impl LeyMaterial {
    pub fn new(disc: bool) -> Self {
        Self {
            params: LeyParams {
                cyan: lin(CYAN),
                gold: lin(GOLD),
                ink: lin(INK),
                view: Vec4::new(1.0, 0.4, 0.0, if disc { 1.0 } else { 0.0 }),
            },
        }
    }
}

impl Material for LeyMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/ley_line.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/ley_line.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
