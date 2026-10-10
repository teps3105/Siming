//! GDD 5.1 色票。著色器中不寫死顏色（GDD 9.4），一律由此轉成 uniform 傳入。
use bevy::prelude::*;

pub const PAPER: u32 = 0xEDE4CF;
pub const PAPER_DARK: u32 = 0xD9CBA8;
pub const INK: u32 = 0x2B2620;
pub const INK_LIGHT: u32 = 0x6B6152;
// 山河底圖：淺絳為底、青綠為飾（GDD 5.1「山河設色」）。
/// 江河與湖面：石青淡染。
pub const WATER: u32 = 0x8DB3B6;
/// 深海：花青。
pub const WATER_DEEP: u32 = 0x3E6379;
/// 平原：絹上淺赭。
pub const PLAINS: u32 = 0xE8DDBC;
/// 丘陵：赭石淡染。
pub const HILLS: u32 = 0xC4A073;
/// 山地底色：灰赭（高度漸變色階見 `mountain_ramp`）。
pub const MOUNTAIN: u32 = 0xB5AE9C;
/// 森林：石綠。
pub const FOREST: u32 = 0x5E8A5A;
/// 沙漠：黃赭。
pub const DESERT: u32 = 0xDDBE7E;
pub const SNOW: u32 = 0xF3F1EA;
/// 沼澤：藤黃加花青，偏黃的淡灰綠（與森林以明度、與湖以色相區分）。
pub const SWAMP: u32 = 0x9AA682;
/// 海深色階（淺 → 深，五階）：青白 → 淺青 → 青 → 花青 → 深花青。
pub const SEA_RAMP: [u32; 5] = [0xD3E0D6, 0xA8C7C3, 0x7DA4AD, 0x5A8296, 0x3E6379];
/// 山地高度色階（低 → 高）：赭石 → 石綠 → 青綠 → 石青 → 雪。
pub const MOUNTAIN_RAMP: [u32; 5] = [0xBC9A68, 0x9DAA80, 0x7FA39A, 0x8FA6BA, 0xEEF0EC];
pub const SEAL_RED: u32 = 0xB23A2E;
pub const GOLD: u32 = 0xC9A246;
pub const CYAN: u32 = 0x4F8F9A;

/// sRGB 十六進位 → 線性 RGBA（著色器在線性空間運算，由 sRGB 畫面緩衝編碼輸出）。
pub fn lin(hex: u32) -> Vec4 {
    let c = Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
    let l = c.to_linear();
    Vec4::new(l.red, l.green, l.blue, 1.0)
}

pub fn lin_color(hex: u32) -> LinearRgba {
    let v = lin(hex);
    LinearRgba::new(v.x, v.y, v.z, 1.0)
}

/// 地形代碼（`Terrain as u8`）對應的底色。丘陵為淡墨調、山脈為紙陰影調（GDD 5.1「ink-light：丘陵」）。
pub fn terrain_colors() -> [Vec4; 9] {
    [
        lin(SEA_RAMP[2]),  // Sea
        lin(SEA_RAMP[1]),  // Lake
        lin(PLAINS),       // Plains
        lin(HILLS),        // Hills
        lin(MOUNTAIN),     // Mountain
        lin(FOREST),       // Forest
        lin(SNOW),         // Snow
        lin(DESERT),       // Desert
        lin(SWAMP),        // Swamp
    ]
}

pub fn sea_ramp() -> [Vec4; 5] {
    SEA_RAMP.map(lin)
}

pub fn mountain_ramp() -> [Vec4; 5] {
    MOUNTAIN_RAMP.map(lin)
}

/// GDD 5.1 勢力著色：24 色淡彩調色盤（HSV 彩度 0.25–0.35、明度 0.75–0.85）。
/// 色相以 7 為步距跳排，使相鄰索引的色相差距大。
pub fn pastel24() -> [Vec4; 24] {
    let mut out = [Vec4::ONE; 24];
    for (i, o) in out.iter_mut().enumerate() {
        let hue = ((i * 7) % 24) as f32 * 15.0;
        let s = 0.25 + 0.10 * ((i % 3) as f32 / 2.0);
        let v = 0.75 + 0.10 * (((i / 3) % 3) as f32 / 2.0);
        let l = Color::hsv(hue, s, v).to_linear();
        *o = Vec4::new(l.red, l.green, l.blue, 1.0);
    }
    out
}
