//! GDD 5.3 地形圖烘焙：把模擬的細格高度細化成連續高度場，供著色器畫出海岸線、暈渲與設色。
//!
//! 一切只依高度：海岸線是「相對海平面的高度」為 0 的等值線，不是陸／水格的邊界。
//! 細格高度先以三次 B 樣條放大（磨掉格點階梯），再疊上振幅與當地坡度成正比的碎形細節——
//! 坡緩處海岸曲折、坡陡處海岸平直，與實際地形相同。只做顯示用的換算，不進世界雜湊（GDD 6.1）。

use siming_sim::{FineGeography, Geography, Terrain};

/// 高度貼圖相對來源格（細格）的放大倍率。
pub const RELIEF_SS: usize = 4;
/// 海格與陸格的高度至少離海平面這麼遠，放大後海岸才落在兩者之間。
const SHORE_EPS: f32 = 0.002;
/// 碎形細節各倍頻的波長（遊戲格）。
const DETAIL_WAVELENGTHS: [f32; 3] = [1.6, 0.8, 0.4];
/// 細節振幅 = 坡度 × 波長 × 此係數；海岸的橫向曲折約為波長的這個比例。
const DETAIL_SLOPE_GAIN: f32 = 0.42;
/// 溝谷刻蝕的工作格相對來源格的倍率（中尺寸為每遊戲格 4 格）。
const GULLY_SS: usize = 2;
/// 匯流面積（工作格數）達此值時，溝谷刻到最深。
const GULLY_ACC_FULL: f32 = 90.0;
/// 溝谷深度 = (坡度 × `GULLY_SLOPE_GAIN` + 最高點高度 × `GULLY_BASE`) × 山區權重 × 匯流程度。
const GULLY_SLOPE_GAIN: f32 = 2.6;
const GULLY_BASE: f32 = 0.03;
/// 溝谷谷底離海平面至少這麼高（著色器的微細節與取樣內插不會把它壓到海平面以下）。
const GULLY_FLOOR: f32 = 0.012;
/// 山區權重：相對高度占最高點的比例由此區間漸入。
const MOUNTAIN_ALT_START: f32 = 0.14;
const MOUNTAIN_ALT_FULL: f32 = 0.45;
/// 山區短波起伏的減弱比例。
const MOUNTAIN_CALM: f32 = 0.75;
/// 平地上的最小細節振幅（遠小於 `SHORE_EPS`，不會憑空生出島或湖）。
const DETAIL_FLOOR: f32 = 0.0002;

/// 烘焙結果。
pub struct Relief {
    /// 高度貼圖尺寸。
    pub w: usize,
    pub h: usize,
    /// 相對海平面的高度（陸正、海負），列優先。
    pub elevation: Vec<f32>,
    /// 湖面遮罩 0..255（128 為湖岸），與高度貼圖同尺寸。
    pub lake: Vec<u8>,
    /// 來源格尺寸的 RGB8：依溫度、濕度與地形類別的植被底色（sRGB）。
    pub climate: Vec<u8>,
    pub climate_w: usize,
    pub climate_h: usize,
    pub sea_level: f32,
    /// 遊戲格尺寸（世界座標的單位）。
    pub grid_w: usize,
    pub grid_h: usize,
    pub wrap_x: bool,
}

impl Relief {
    /// 世界座標 (x, z)（遊戲格）處相對海平面的高度，雙線性。
    pub fn elevation_at(&self, x: f32, z: f32) -> f32 {
        let fx = x / self.grid_w as f32 * self.w as f32 - 0.5;
        let fy = (z / self.grid_h as f32 * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let fx = if self.wrap_x {
            fx.rem_euclid(self.w as f32)
        } else {
            fx.clamp(0.0, (self.w - 1) as f32)
        };
        let (x0, y0) = ((fx.floor() as usize).min(self.w - 1), fy.floor() as usize);
        let x1 = if self.wrap_x {
            (x0 + 1) % self.w
        } else {
            (x0 + 1).min(self.w - 1)
        };
        let y1 = (y0 + 1).min(self.h - 1);
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let g = |x: usize, y: usize| self.elevation[y * self.w + x];
        let a = g(x0, y0) * (1.0 - tx) + g(x1, y0) * tx;
        let b = g(x0, y1) * (1.0 - tx) + g(x1, y1) * tx;
        a * (1.0 - ty) + b * ty
    }

    /// GDD 5.3 地形高度：`max(相對高度, 0) × 地圖寬 × 0.04`（高度縮放 = 1）。
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.elevation_at(x, z).max(0.0) * self.grid_w as f32 * 0.04
    }

    /// 陸地最高點（高度縮放 = 1）。
    pub fn max_height(&self) -> f32 {
        self.elevation.iter().cloned().fold(0.0, f32::max).max(1e-4) * self.grid_w as f32 * 0.04
    }
}

/// 來源格資料：有細格用細格，沒有（手工世界）則退回遊戲格。
struct Source {
    w: usize,
    h: usize,
    wrap: bool,
    sea_level: f32,
    height: Vec<f32>,
    terrain: Vec<Terrain>,
    temperature: Vec<f32>,
    moisture: Vec<f32>,
}

impl Source {
    fn new(geo: &Geography, fine: &FineGeography) -> Self {
        if fine.is_empty() {
            let (w, h) = (geo.w as usize, geo.h as usize);
            let lat_cos: Vec<f32> = (0..h)
                .map(|y| (std::f32::consts::PI * (y as f32 + 0.5) / h as f32).sin())
                .collect();
            Self {
                w,
                h,
                wrap: geo.wrap_x,
                sea_level: geo.sea_level,
                height: geo.height.clone(),
                terrain: geo.terrain.clone(),
                temperature: (0..w * h).map(|i| lat_cos[i / w]).collect(),
                moisture: vec![0.5; w * h],
            }
        } else {
            Self {
                w: fine.w as usize,
                h: fine.h as usize,
                wrap: geo.wrap_x,
                sea_level: fine.sea_level,
                height: fine.height.iter().map(|&v| v as f32 / 65535.0).collect(),
                terrain: fine.terrain.clone(),
                temperature: fine.temperature.iter().map(|&v| v as f32 / 255.0).collect(),
                moisture: fine.moisture.iter().map(|&v| v as f32 / 255.0).collect(),
            }
        }
    }

    fn at(&self, grid: &[f32], x: i32, y: i32) -> f32 {
        grid_at(grid, self.w, self.h, self.wrap, x, y)
    }

    fn blur(&self, grid: &[f32]) -> Vec<f32> {
        grid_blur(grid, self.w, self.h, self.wrap)
    }

    /// 三次 B 樣條取樣（格中心在整數座標）：C2 連續，放大後沒有格點階梯。
    fn cubic(&self, grid: &[f32], x: f32, y: f32) -> f32 {
        grid_cubic(grid, self.w, self.h, self.wrap, x, y)
    }
}

fn grid_at(grid: &[f32], w: usize, h: usize, wrap: bool, x: i32, y: i32) -> f32 {
    let cy = y.clamp(0, h as i32 - 1) as usize;
    let cx = if wrap {
        x.rem_euclid(w as i32)
    } else {
        x.clamp(0, w as i32 - 1)
    } as usize;
    grid[cy * w + cx]
}

/// 3×3 二項式模糊。
fn grid_blur(grid: &[f32], w: usize, h: usize, wrap: bool) -> Vec<f32> {
    let k = [1.0f32, 2.0, 1.0];
    let mut out = vec![0f32; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut sum = 0.0;
            for (j, ky) in k.iter().enumerate() {
                for (i, kx) in k.iter().enumerate() {
                    sum += grid_at(grid, w, h, wrap, x + i as i32 - 1, y + j as i32 - 1) * ky * kx;
                }
            }
            out[y as usize * w + x as usize] = sum / 16.0;
        }
    }
    out
}

/// 三次 B 樣條取樣（格中心在整數座標）。
fn grid_cubic(grid: &[f32], w: usize, h: usize, wrap: bool, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (wx, wy) = (bspline3(x - x0), bspline3(y - y0));
    let mut sum = 0.0;
    for (j, wyj) in wy.iter().enumerate() {
        for (i, wxi) in wx.iter().enumerate() {
            sum += grid_at(
                grid,
                w,
                h,
                wrap,
                x0 as i32 + i as i32 - 1,
                y0 as i32 + j as i32 - 1,
            ) * wxi
                * wyj;
        }
    }
    sum
}

/// 溝谷的匯流程度（0 稜線 .. 1 谷底）。
///
/// 在工作格上讓每個陸格的水流向最陡的鄰格（D8），由高到低累加匯流面積；面積取對數後模糊一次。
/// 均勻的山坡上水平行而下，刻出的溝谷彼此平行、沿坡向延伸，谷與谷之間的山脊因此有方向。
/// 回傳（匯流程度、寬、高）。
fn carve_gullies(
    src: &Source,
    signed: &[f32],
    slope: &[f32],
    radius: f32,
) -> (Vec<f32>, usize, usize) {
    let (gw, gh) = (src.w * GULLY_SS, src.h * GULLY_SS);
    let n = gw * gh;
    // 工作格的高度：放大後的高度加第一個倍頻的起伏（與成品一致），再加極小的擾動打破平地的並列。
    let mut height = vec![0f32; n];
    let wavelength = DETAIL_WAVELENGTHS[0];
    for y in 0..gh {
        let lat = (0.5 - (y as f32 + 0.5) / gh as f32) * std::f32::consts::PI;
        let (sin_lat, cos_lat) = lat.sin_cos();
        let gy = (y as f32 + 0.5) / GULLY_SS as f32 - 0.5;
        for x in 0..gw {
            let lon = (x as f32 + 0.5) / gw as f32 * std::f32::consts::TAU;
            let gx = (x as f32 + 0.5) / GULLY_SS as f32 - 0.5;
            let p = [
                radius * cos_lat * lon.cos(),
                radius * sin_lat,
                -radius * cos_lat * lon.sin(),
            ];
            let local_slope = src.cubic(slope, gx, gy).max(0.0);
            let f = 1.0 / wavelength;
            let noise = gradient_noise([p[0] * f, p[1] * f, p[2] * f]);
            let jitter = (hash3(x as i32, y as i32, 77) & 0xffff) as f32 / 65535.0 * 2e-6;
            height[y * gw + x] = src.cubic(signed, gx, gy)
                + noise * (local_slope * wavelength * DETAIL_SLOPE_GAIN + DETAIL_FLOOR)
                + jitter;
        }
    }
    let mut order: Vec<u32> = (0..n as u32)
        .filter(|&i| height[i as usize] > 0.0)
        .collect();
    order.sort_unstable_by(|&a, &b| height[b as usize].total_cmp(&height[a as usize]));
    let mut area = vec![0f32; n];
    for &i in &order {
        area[i as usize] = 1.0;
    }
    for &i in &order {
        let i = i as usize;
        let (x, y) = ((i % gw) as i32, (i / gw) as i32);
        let mut best: Option<(usize, f32)> = None;
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let ny = y + dy;
                if ny < 0 || ny >= gh as i32 {
                    continue;
                }
                let nx = if src.wrap {
                    (x + dx).rem_euclid(gw as i32)
                } else {
                    x + dx
                };
                if nx < 0 || nx >= gw as i32 {
                    continue;
                }
                let j = ny as usize * gw + nx as usize;
                let drop = (height[i] - height[j])
                    / if dx != 0 && dy != 0 {
                        std::f32::consts::SQRT_2
                    } else {
                        1.0
                    };
                if drop > 0.0 && best.is_none_or(|b| drop > b.1) {
                    best = Some((j, drop));
                }
            }
        }
        if let Some((j, _)) = best {
            if height[j] > 0.0 {
                area[j] += area[i];
            }
        }
    }
    let full = GULLY_ACC_FULL.ln();
    let flow: Vec<f32> = area
        .iter()
        .map(|&a| {
            if a > 1.0 {
                (a.ln() / full).min(1.0)
            } else {
                0.0
            }
        })
        .collect();
    let flow = grid_blur(&flow, gw, gh, src.wrap);
    (flow, gw, gh)
}

/// 均勻三次 B 樣條在小數位置 t 的四個權重（對應格 −1、0、1、2）。
fn bspline3(t: f32) -> [f32; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [
        (1.0 - 3.0 * t + 3.0 * t2 - t3) / 6.0,
        (4.0 - 6.0 * t2 + 3.0 * t3) / 6.0,
        (1.0 + 3.0 * t + 3.0 * t2 - 3.0 * t3) / 6.0,
        t3 / 6.0,
    ]
}

// ---------------------------------------------------------------- 三維梯度雜訊

fn hash3(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

fn grad(hash: u32, x: f32, y: f32, z: f32) -> f32 {
    // 十二個稜方向（Perlin 改良版）。
    match hash % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

/// 三維梯度雜訊，值域約 −1..1。在球面座標求值，東西接縫與極點都連續。
fn gradient_noise(p: [f32; 3]) -> f32 {
    let (x0, y0, z0) = (p[0].floor(), p[1].floor(), p[2].floor());
    let (fx, fy, fz) = (p[0] - x0, p[1] - y0, p[2] - z0);
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v, w) = (fade(fx), fade(fy), fade(fz));
    let (ix, iy, iz) = (x0 as i32, y0 as i32, z0 as i32);
    let g = |dx: i32, dy: i32, dz: i32| {
        grad(
            hash3(ix + dx, iy + dy, iz + dz),
            fx - dx as f32,
            fy - dy as f32,
            fz - dz as f32,
        )
    };
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = lerp(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = lerp(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = lerp(g(0, 1, 1), g(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

// ---------------------------------------------------------------- 植被底色

/// GDD 5.1「地形圖設色」的植被底色（sRGB）。
const DESERT: [f32; 3] = [0.851, 0.773, 0.604];
const STEPPE: [f32; 3] = [0.816, 0.792, 0.612];
const GRASS: [f32; 3] = [0.729, 0.776, 0.565];
const FOREST: [f32; 3] = [0.478, 0.627, 0.424];
const RAINFOREST: [f32; 3] = [0.380, 0.553, 0.376];
const TAIGA: [f32; 3] = [0.553, 0.651, 0.541];
const TUNDRA: [f32; 3] = [0.765, 0.761, 0.678];
const SNOW: [f32; 3] = [0.945, 0.941, 0.918];
const SWAMP: [f32; 3] = [0.561, 0.635, 0.529];

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 依溫度（0 寒 .. 1 熱）與濕度（0 乾 .. 1 濕）連續取色。
fn climate_color(temperature: f32, moisture: f32) -> [f32; 3] {
    let dry = mix3(
        TUNDRA,
        mix3(STEPPE, DESERT, smoothstep(0.55, 0.85, temperature)),
        smoothstep(0.15, 0.45, temperature),
    );
    let mid = mix3(TUNDRA, GRASS, smoothstep(0.12, 0.40, temperature));
    let wet = mix3(
        TAIGA,
        mix3(FOREST, RAINFOREST, smoothstep(0.60, 0.85, temperature)),
        smoothstep(0.25, 0.50, temperature),
    );
    let land = if moisture < 0.45 {
        mix3(dry, mid, smoothstep(0.15, 0.45, moisture))
    } else {
        mix3(mid, wet, smoothstep(0.45, 0.75, moisture))
    };
    mix3(SNOW, land, smoothstep(0.04, 0.14, temperature))
}

/// 模擬的地形類別對應的底色；`None` 表示該類別不指定顏色（交給氣候與高度）。
fn class_color(t: Terrain) -> Option<[f32; 3]> {
    match t {
        Terrain::Forest => Some(FOREST),
        Terrain::Desert => Some(DESERT),
        Terrain::Swamp => Some(SWAMP),
        Terrain::Snow => Some(SNOW),
        _ => None,
    }
}

// ---------------------------------------------------------------- 烘焙

/// 由世界的地理資料烘焙地形圖所需的高度場、湖面遮罩與植被底色。
pub fn build_relief(geo: &Geography, fine: &FineGeography) -> Relief {
    let src = Source::new(geo, fine);
    let (sw, sh) = (src.w, src.h);
    let (w, h) = (sw * RELIEF_SS, sh * RELIEF_SS);
    let (grid_w, grid_h) = (geo.w as usize, geo.h as usize);
    // 來源格一格等於幾個遊戲格。
    let cell = grid_w as f32 / sw as f32;

    // 相對海平面的高度；海格與陸格各自離海平面至少 SHORE_EPS，使等值線落在兩者之間。
    let signed: Vec<f32> = (0..sw * sh)
        .map(|i| {
            let e = src.height[i] - src.sea_level;
            if src.terrain[i] == Terrain::Sea {
                e.min(-SHORE_EPS)
            } else {
                e.max(SHORE_EPS)
            }
        })
        .collect();
    // 坡度（每遊戲格的高度變化），決定細節振幅。
    let slope: Vec<f32> = (0..sw * sh)
        .map(|i| {
            let (x, y) = ((i % sw) as i32, (i / sw) as i32);
            let dx = (src.at(&signed, x + 1, y) - src.at(&signed, x - 1, y)) / (2.0 * cell);
            let dy = (src.at(&signed, x, y + 1) - src.at(&signed, x, y - 1)) / (2.0 * cell);
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    let slope = src.blur(&slope);
    let lake_mask: Vec<f32> = src
        .terrain
        .iter()
        .map(|&t| if t == Terrain::Lake { 1.0 } else { 0.0 })
        .collect();
    let lake_mask = src.blur(&lake_mask);

    let peak = signed.iter().cloned().fold(0.0, f32::max).max(1e-4);
    let radius = grid_w as f32 / std::f32::consts::TAU;
    let (gullies, gw, gh) = carve_gullies(&src, &signed, &slope, radius);
    let mut elevation = vec![0f32; w * h];
    let mut lake = vec![0u8; w * h];
    for ty in 0..h {
        let v = (ty as f32 + 0.5) / h as f32;
        let lat = (0.5 - v) * std::f32::consts::PI;
        let (sin_lat, cos_lat) = lat.sin_cos();
        let gy = (ty as f32 + 0.5) / RELIEF_SS as f32 - 0.5;
        for tx in 0..w {
            let u = (tx as f32 + 0.5) / w as f32;
            let lon = u * std::f32::consts::TAU;
            let gx = (tx as f32 + 0.5) / RELIEF_SS as f32 - 0.5;
            let base = src.cubic(&signed, gx, gy);
            let local_slope = src.cubic(&slope, gx, gy).max(0.0);
            // 球面座標（單位：赤道上的一個遊戲格）。
            let p = [
                radius * cos_lat * lon.cos(),
                radius * sin_lat,
                -radius * cos_lat * lon.sin(),
            ];
            let mountain = smoothstep(MOUNTAIN_ALT_START, MOUNTAIN_ALT_FULL, base / peak);
            let mut detail = 0.0;
            for (k, wavelength) in DETAIL_WAVELENGTHS.iter().enumerate() {
                let f = 1.0 / wavelength;
                let o = k as f32 * 17.3;
                let n = gradient_noise([p[0] * f + o, p[1] * f - o, p[2] * f + 2.0 * o]);
                // 山區的短波起伏減弱（紋理過密），由大尺度的稜線承擔山形；海岸附近不受影響。
                let calm = if k == 0 {
                    1.0
                } else {
                    1.0 - MOUNTAIN_CALM * mountain
                };
                detail += n * calm * (local_slope * wavelength * DETAIL_SLOPE_GAIN + DETAIL_FLOOR);
            }
            // 山區：刻入順坡而下的溝谷；谷與谷之間就是沿坡向延伸的山脊（稜線高度不變）。
            let mut carve = 0.0;
            if mountain > 0.0 {
                let scale = (RELIEF_SS / GULLY_SS) as f32;
                let (wx, wy) = (
                    (tx as f32 + 0.5) / scale - 0.5,
                    (ty as f32 + 0.5) / scale - 0.5,
                );
                let flow = grid_cubic(&gullies, gw, gh, src.wrap, wx, wy).clamp(0.0, 1.0);
                carve = flow * mountain * (local_slope * GULLY_SLOPE_GAIN + peak * GULLY_BASE);
            }
            let o = ty * w + tx;
            // 溝谷不得刻穿海平面：否則靠海的山腳會冒出水坑。
            let land = base + detail;
            let carve = carve.min((land - GULLY_FLOOR).max(0.0));
            elevation[o] = land - carve;
            lake[o] = (src.cubic(&lake_mask, gx, gy).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    // 植被底色：以氣候連續取色，再向模擬的地形類別靠 45%（類別遮罩先模糊，邊界柔和）。
    let mut climate = vec![0u8; sw * sh * 3];
    let class_weight: Vec<f32> = src
        .terrain
        .iter()
        .map(|&t| if class_color(t).is_some() { 1.0 } else { 0.0 })
        .collect();
    let class_weight = src.blur(&class_weight);
    for i in 0..sw * sh {
        let mut c = climate_color(src.temperature[i], src.moisture[i]);
        if let Some(cc) = class_color(src.terrain[i]) {
            c = mix3(c, cc, 0.45 * class_weight[i]);
        }
        for k in 0..3 {
            climate[i * 3 + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    Relief {
        w,
        h,
        elevation,
        lake,
        climate,
        climate_w: sw,
        climate_h: sh,
        sea_level: src.sea_level,
        grid_w,
        grid_h,
        wrap_x: geo.wrap_x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use siming_sim::{Sim, WorldParams, WorldSize};

    fn small_world() -> Sim {
        let mut p = WorldParams::new(20261011);
        p.size = WorldSize::Small;
        Sim::new(&p).unwrap()
    }

    #[test]
    fn gdd_5_3_relief_sizes_follow_the_source_grid() {
        let sim = small_world();
        let w = sim.world();
        let r = build_relief(&w.geo, &w.fine);
        assert_eq!(
            (r.w, r.h),
            (w.fine.w as usize * RELIEF_SS, w.fine.h as usize * RELIEF_SS)
        );
        assert_eq!(r.elevation.len(), r.w * r.h);
        assert_eq!(r.lake.len(), r.w * r.h);
        assert_eq!(r.climate.len(), r.climate_w * r.climate_h * 3);
        assert!(r.elevation.iter().all(|e| e.is_finite()));
    }

    /// 細化不得改變陸海格局：每個來源格中心的高度符號，與模擬的海／非海一致的比例要在 97% 以上
    /// （不一致的是被平滑掉的單格小島與窄海峽，以及海岸線上的格）。
    #[test]
    fn gdd_5_3_relief_keeps_the_land_sea_layout() {
        let sim = small_world();
        let w = sim.world();
        let r = build_relief(&w.geo, &w.fine);
        let (sw, sh) = (w.fine.w as usize, w.fine.h as usize);
        let mut agree = 0usize;
        for y in 0..sh {
            for x in 0..sw {
                let e = r.elevation
                    [(y * RELIEF_SS + RELIEF_SS / 2) * r.w + x * RELIEF_SS + RELIEF_SS / 2];
                let sea = w.fine.terrain[y * sw + x] == Terrain::Sea;
                if (e < 0.0) == sea {
                    agree += 1;
                }
            }
        }
        let ratio = agree as f32 / (sw * sh) as f32;
        assert!(ratio > 0.97, "陸海一致比例 {ratio}");
    }

    /// 東西接縫連續：最左與最右一行的高度差，不大於相鄰兩行的典型差距。
    #[test]
    fn gdd_5_3_relief_is_continuous_across_the_seam() {
        let sim = small_world();
        let w = sim.world();
        let r = build_relief(&w.geo, &w.fine);
        let mean = |a: usize, b: usize| {
            (0..r.h)
                .map(|y| (r.elevation[y * r.w + a] - r.elevation[y * r.w + b]).abs())
                .sum::<f32>()
                / r.h as f32
        };
        let seam = mean(r.w - 1, 0);
        let inner = mean(r.w / 2, r.w / 2 + 1);
        assert!(seam < inner * 3.0 + 1e-4, "接縫差 {seam}，內部差 {inner}");
    }

    /// 溝谷：匯流程度落在 0..1，海上為 0，山區確實有刻下去的谷。
    #[test]
    fn gdd_5_3_gullies_follow_the_drainage() {
        let sim = small_world();
        let w = sim.world();
        let src = Source::new(&w.geo, &w.fine);
        let signed: Vec<f32> = (0..src.w * src.h)
            .map(|i| src.height[i] - src.sea_level)
            .collect();
        let slope = vec![0.01f32; src.w * src.h];
        let radius = w.geo.w as f32 / std::f32::consts::TAU;
        let (flow, gw, gh) = carve_gullies(&src, &signed, &slope, radius);
        assert_eq!(flow.len(), gw * gh);
        assert!(flow.iter().all(|f| (0.0..=1.0).contains(f)));
        let deep = flow.iter().filter(|&&f| f > 0.5).count();
        assert!(deep > 0, "應有匯流明顯的谷");
        assert!(deep < flow.len() / 4, "谷不應佔滿地面：{deep}");
    }

    /// 溝谷不刻穿海平面：最高的一成陸地（山區）沒有任何紋素低於海平面。
    #[test]
    fn gdd_5_3_gullies_never_cut_below_sea_level() {
        let sim = small_world();
        let w = sim.world();
        let r = build_relief(&w.geo, &w.fine);
        let peak = r.elevation.iter().cloned().fold(0.0, f32::max);
        let (sw, sh) = (w.fine.w as usize, w.fine.h as usize);
        for y in 0..sh {
            for x in 0..sw {
                let cell = w.fine.height[y * sw + x] as f32 / 65535.0 - w.fine.sea_level;
                if cell < peak * 0.5 {
                    continue;
                }
                for ty in 0..RELIEF_SS {
                    for tx in 0..RELIEF_SS {
                        let e = r.elevation[(y * RELIEF_SS + ty) * r.w + x * RELIEF_SS + tx];
                        assert!(e > 0.0, "山區紋素低於海平面：({x}, {y}) = {e}");
                    }
                }
            }
        }
    }

    #[test]
    fn gdd_5_3_height_lookup_matches_the_elevation_grid() {
        let sim = small_world();
        let w = sim.world();
        let r = build_relief(&w.geo, &w.fine);
        assert!(r.max_height() > 0.0);
        // 高度貼圖第 (tx, ty) 個紋素中心的世界座標。
        let (tx, ty) = (r.w / 3, r.h / 2);
        let x = (tx as f32 + 0.5) / r.w as f32 * r.grid_w as f32;
        let z = (ty as f32 + 0.5) / r.h as f32 * r.grid_h as f32;
        assert!((r.elevation_at(x, z) - r.elevation[ty * r.w + tx]).abs() < 1e-5);
    }
}
