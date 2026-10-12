//! GDD 6.7 siming-cli gen：地圖算圖（純函式，色票取自 GDD 5.1）。

use siming_sim::{FineGeography, Geography, Province, ProvinceId, Terrain, NO_PROVINCE};
use std::path::Path;

pub type Rgb = [u8; 3];

// GDD 5.1 色票。
pub const PAPER: Rgb = [0xED, 0xE4, 0xCF];
pub const PAPER_DARK: Rgb = [0xD9, 0xCB, 0xA8];
pub const INK: Rgb = [0x2B, 0x26, 0x20];
pub const INK_LIGHT: Rgb = [0x6B, 0x61, 0x52];
pub const WATER: Rgb = [0x9F, 0xB2, 0xAE];
pub const WATER_DEEP: Rgb = [0x6E, 0x87, 0x84];
pub const FOREST: Rgb = [0x8C, 0x9A, 0x74];
pub const DESERT: Rgb = [0xD8, 0xC0, 0x8E];
pub const SNOW: Rgb = [0xF4, 0xF1, 0xEA];
pub const SWAMP: Rgb = [0x7F, 0x87, 0x68];
pub const SEAL_RED: Rgb = [0xB2, 0x3A, 0x2E];
pub const GOLD: Rgb = [0xC9, 0xA2, 0x46];
pub const CYAN: Rgb = [0x4F, 0x8F, 0x9A];
pub const KARMA: Rgb = [0x8C, 0x2A, 0x1C];

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Layer {
    /// 對照圖（6.4 細格契約的參考實作）：山體陰影、生態漸層、依流量的河寬、海岸線。
    Atlas,
    /// 除錯：細格高度灰階（海深到陸高），檢查海床與極區。
    Height,
    All,
    Terrain,
    Province,
    Aura,
    Karma,
}

/// 線性混色，t 夾在 0..1。
pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let f = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    [f(0), f(1), f(2)]
}

/// 靈氣色階：0 紙 → 1.0 青 → 2.0 金（aura_cell 範圍 0..2，GDD 2.4）。
pub fn aura_ramp(v: f32) -> Rgb {
    let v = v.clamp(0.0, 2.0);
    if v <= 1.0 {
        lerp(PAPER, CYAN, v)
    } else {
        lerp(CYAN, GOLD, v - 1.0)
    }
}

/// 業氣色階：0 紙 → 1 業色。
pub fn karma_ramp(v: f32) -> Rgb {
    lerp(PAPER, KARMA, v)
}

/// 地形基色（GDD 5.1）。
pub fn terrain_color(t: Terrain) -> Rgb {
    match t {
        Terrain::Sea => WATER_DEEP,
        Terrain::Lake => WATER,
        Terrain::Plains => PAPER,
        Terrain::Hills => lerp(PAPER, INK_LIGHT, 0.45),
        Terrain::Mountain => lerp(INK_LIGHT, INK, 0.35),
        Terrain::Forest => FOREST,
        Terrain::Snow => SNOW,
        Terrain::Desert => DESERT,
        Terrain::Swamp => SWAMP,
    }
}

const PASTELS: [Rgb; 8] = [
    [0xE8, 0xC4, 0xB8],
    [0xC9, 0xDB, 0xB8],
    [0xB8, 0xD0, 0xE0],
    [0xE6, 0xDA, 0xA8],
    [0xD6, 0xBF, 0xDC],
    [0xB8, 0xDE, 0xD4],
    [0xF0, 0xC8, 0xD2],
    [0xCF, 0xCF, 0xB0],
];

/// 貪婪著色：相鄰州郡取不同的柔色（色盡時退而循環取色）。
pub fn province_colors(provinces: &[Province]) -> Vec<Rgb> {
    let mut idx: Vec<usize> = vec![usize::MAX; provinces.len()];
    for (i, p) in provinces.iter().enumerate() {
        let mut used = [false; PASTELS.len()];
        for n in p.neighbors.iter().chain(p.sea_neighbors.iter()) {
            if let Some(&c) = idx.get(*n as usize) {
                if c != usize::MAX {
                    used[c] = true;
                }
            }
        }
        idx[i] = (0..PASTELS.len()).find(|&c| !used[c]).unwrap_or(i % PASTELS.len());
    }
    idx.into_iter().map(|c| PASTELS[c]).collect()
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, fill: Rgb) -> Self {
        let mut px = Vec::with_capacity(w * h * 3);
        for _ in 0..w * h {
            px.extend_from_slice(&fill);
        }
        Self { w, h, px }
    }

    #[cfg(test)]
    pub fn get(&self, x: usize, y: usize) -> Rgb {
        let i = (y * self.w + x) * 3;
        [self.px[i], self.px[i + 1], self.px[i + 2]]
    }

    pub fn set(&mut self, x: i64, y: i64, c: Rgb) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let i = (y as usize * self.w + x as usize) * 3;
            self.px[i..i + 3].copy_from_slice(&c);
        }
    }

    pub fn rect(&mut self, x0: i64, y0: i64, w: i64, h: i64, c: Rgb) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.set(x, y, c);
            }
        }
    }

    pub fn disc(&mut self, cx: i64, cy: i64, r: i64, c: Rgb) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r {
                    self.set(cx + x, cy + y, c);
                }
            }
        }
    }

    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(&self.px).map_err(|e| e.to_string())
    }
}

fn cell_center(geo: &Geography, cell: u32, s: i64) -> (i64, i64) {
    let (x, y) = geo.xy(cell);
    (x as i64 * s + s / 2, y as i64 * s + s / 2)
}

fn fill_cells(cv: &mut Canvas, geo: &Geography, s: usize, f: impl Fn(usize) -> Rgb) {
    for y in 0..cv.h {
        for x in 0..cv.w {
            let c = f((y / s) * geo.w as usize + x / s);
            cv.set(x as i64, y as i64, c);
        }
    }
}

fn terrain_pixels(cv: &mut Canvas, geo: &Geography, s: usize) {
    fill_cells(cv, geo, s, |i| {
        let t = geo.terrain[i];
        let base = terrain_color(t);
        match t {
            // 丘陵／山脈依高度由亮到暗。
            Terrain::Hills | Terrain::Mountain => {
                let span = (1.0 - geo.sea_level).max(1e-3);
                let k = ((geo.height[i] - geo.sea_level) / span).clamp(0.0, 1.0);
                lerp(base, INK, k * 0.5)
            }
            _ => base,
        }
    });
}

fn rivers(cv: &mut Canvas, geo: &Geography, s: usize) {
    let s = s as i64;
    let is_river = |i: usize| geo.river[i] > 0 && !geo.terrain[i].is_water();
    for i in 0..geo.len() {
        if !is_river(i) {
            continue;
        }
        let lv = geo.river[i] as i64;
        let (cx, cy) = cell_center(geo, i as u32, s);
        let w = (s * lv / 4).max(1);
        cv.rect(cx - w / 2, cy - w / 2, w, w, WATER_DEEP);
        // 與下／右鄰河格相連，避免斷線。
        let (x, y) = geo.xy(i as u32);
        for (dx, dy) in [(1i64, 0i64), (0, 1)] {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx < geo.w as i64 && ny < geo.h as i64 {
                let n = (ny as usize) * geo.w as usize + nx as usize;
                if is_river(n) {
                    let (mx, my) = (cx + dx * s / 2, cy + dy * s / 2);
                    cv.rect(mx - w / 2, my - w / 2, w, w, WATER_DEEP);
                }
            }
        }
    }
}

fn province_borders(cv: &mut Canvas, geo: &Geography, s: usize, c: Rgb) {
    let (w, h) = (geo.w as usize, geo.h as usize);
    for y in 0..h {
        for x in 0..w {
            let p: ProvinceId = geo.province_of[y * w + x];
            if p == NO_PROVINCE {
                continue;
            }
            let differs = |q: ProvinceId| q != p && q != NO_PROVINCE;
            if x + 1 < w && differs(geo.province_of[y * w + x + 1]) {
                cv.rect(((x + 1) * s) as i64 - 1, (y * s) as i64, 1, s as i64, c);
            }
            if y + 1 < h && differs(geo.province_of[(y + 1) * w + x]) {
                cv.rect((x * s) as i64, ((y + 1) * s) as i64 - 1, s as i64, 1, c);
            }
        }
    }
}

fn node_dots(cv: &mut Canvas, geo: &Geography, s: usize) {
    let s = s as i64;
    for n in &geo.nodes {
        let (cx, cy) = cell_center(geo, n.cell, s);
        let r = ((s as f32 * (0.5 + n.strength * 1.5)).round() as i64).max(2);
        cv.disc(cx, cy, r + 1, INK);
        cv.disc(cx, cy, r, GOLD);
    }
}

fn seat_squares(cv: &mut Canvas, geo: &Geography, provinces: &[Province], s: usize) {
    let s = s as i64;
    let sz = (s * 3 / 2).max(3);
    for p in provinces {
        let (cx, cy) = cell_center(geo, p.seat, s);
        cv.rect(cx - sz / 2 - 1, cy - sz / 2 - 1, sz + 2, sz + 2, INK);
        cv.rect(cx - sz / 2, cy - sz / 2, sz, sz, SEAL_RED);
    }
}

// 對照圖色票（連續場混色；GDD 5.1 色系的自然色版本）。
const ATLAS_SEA_DEEP: Rgb = [0x3F, 0x5E, 0x72];
const ATLAS_SEA_SHALLOW: Rgb = [0x9C, 0xC3, 0xC8];
const ATLAS_SEA_MID: Rgb = [0x5E, 0x84, 0x96];
const ATLAS_SAND: Rgb = [0xD9, 0xC3, 0x8F];
const ATLAS_STEPPE: Rgb = [0xC6, 0xC2, 0x8E];
const ATLAS_GRASS: Rgb = [0x9F, 0xB3, 0x6E];
const ATLAS_FOREST: Rgb = [0x5E, 0x7F, 0x4E];
const ATLAS_SWAMP: Rgb = [0x6F, 0x7E, 0x5C];
const ATLAS_TUNDRA: Rgb = [0xB8, 0xB4, 0xA6];
const ATLAS_SNOW: Rgb = [0xF2, 0xF1, 0xEC];
const ATLAS_ROCK: Rgb = [0x8E, 0x82, 0x74];
const ATLAS_PEAK: Rgb = [0xE8, 0xE6, 0xE0];
const ATLAS_RIVER: Rgb = [0x4A, 0x6E, 0x86];
const ATLAS_COAST: Rgb = [0x2F, 0x3E, 0x44];

#[inline]
fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 細格生態底色：依濕度（乾→濕）、溫度（雪線）、陸地高度（岩、雪峰）連續混色，沒有硬邊。
fn atlas_land_color(lh: f32, t: f32, q: f32, terrain: Terrain) -> Rgb {
    // 乾→濕：沙 → 草原 → 草地 → 森林。
    let veg = if q < 0.3 {
        lerp(ATLAS_SAND, ATLAS_STEPPE, q / 0.3)
    } else if q < 0.55 {
        lerp(ATLAS_STEPPE, ATLAS_GRASS, (q - 0.3) / 0.25)
    } else {
        lerp(ATLAS_GRASS, ATLAS_FOREST, smoothstep(0.55, 0.85, q))
    };
    let veg = if terrain == Terrain::Swamp { lerp(veg, ATLAS_SWAMP, 0.6) } else { veg };
    // 冷：苔原 → 雪原。
    let cold = lerp(ATLAS_SNOW, ATLAS_TUNDRA, smoothstep(0.12, 0.3, t));
    let mut c = lerp(cold, veg, smoothstep(0.2, 0.38, t));
    // 高：岩（山麓起）→ 雪峰。
    let rock = smoothstep(0.3, 0.6, lh);
    c = lerp(c, ATLAS_ROCK, rock * 0.85);
    c = lerp(c, ATLAS_PEAK, smoothstep(0.7, 0.95, lh));
    c
}

/// 對照圖（6.4 細格契約）：每個細格 `ps` 像素。
pub fn atlas(geo: &Geography, fine: &FineGeography, scale: usize) -> Canvas {
    let (fw, fh) = (fine.w as usize, fine.h as usize);
    let ps = (scale / fine.scale.max(1) as usize).max(1);
    let mut cv = Canvas::new(fw * ps, fh * ps, PAPER);
    let sea = fine.sea_level;
    let span = (1.0 - sea).max(1e-3);
    let hgt = |x: usize, y: usize| fine.height_at(y * fw + x);
    let lat_cos = |y: usize| {
        let lat = std::f32::consts::FRAC_PI_2 - std::f32::consts::PI * (y as f32 + 0.5) / fh as f32;
        lat.cos().max(0.05)
    };
    // 1. 底色 + 山體陰影（光自西北；法向由中央差分求，東西向以 cos(lat) 修正）。
    for y in 0..fh {
        let yn = y.saturating_sub(1);
        let ys = (y + 1).min(fh - 1);
        let cl = lat_cos(y);
        for x in 0..fw {
            let i = y * fw + x;
            let xw = (x + fw - 1) % fw;
            let xe = (x + 1) % fw;
            let h = fine.height_at(i);
            let t = fine.terrain[i];
            let base = if t == Terrain::Sea {
                // 海深色階：大陸棚（淺）→ 大陸坡 → 深海平原（深）；洋脊與海溝由海床高度自然呈現。
                let depth = (sea - h).max(0.0) / sea.max(1e-3);
                let shelf = lerp(ATLAS_SEA_SHALLOW, ATLAS_SEA_MID, smoothstep(0.0, 0.14, depth));
                lerp(shelf, ATLAS_SEA_DEEP, smoothstep(0.12, 0.8, depth))
            } else if t == Terrain::Lake {
                ATLAS_SEA_SHALLOW
            } else {
                let lh = ((h - sea) / span).clamp(0.0, 1.0);
                let tt = fine.temperature[i] as f32 / 255.0;
                let q = fine.moisture[i] as f32 / 255.0;
                atlas_land_color(lh, tt, q, t)
            };
            let c = if t.is_water() {
                base
            } else {
                // 高度以 0..1 對應約 40 格的垂直尺度；坡度放大使陰影可見。
                let k = 28.0;
                let dzdx = (hgt(xe, y) - hgt(xw, y)) * k / (2.0 * cl);
                let dzdy = (hgt(x, ys) - hgt(x, yn)) * k / 2.0;
                // 法向 (−dzdx, −dzdy, 1)，光 (−0.6, −0.6, 0.53)（西北上方）。
                let nl = (-dzdx * -0.6 + -dzdy * -0.6 + 0.53) / ((dzdx * dzdx + dzdy * dzdy + 1.0).sqrt());
                let shade = (0.55 + 0.6 * nl.clamp(0.0, 1.0)).clamp(0.4, 1.15);
                let f = |v: u8| (v as f32 * shade).round().clamp(0.0, 255.0) as u8;
                [f(base[0]), f(base[1]), f(base[2])]
            };
            for dy in 0..ps {
                for dx in 0..ps {
                    cv.set((x * ps + dx) as i64, (y * ps + dy) as i64, c);
                }
            }
        }
    }
    // 2. 河流：依流量的線寬（圓盤相連）。
    for y in 0..fh {
        for x in 0..fw {
            let i = y * fw + x;
            if fine.river[i] == 0 || fine.terrain[i].is_water() {
                continue;
            }
            let f = fine.flow[i] as f32 / 255.0;
            // 小河只畫流量較大者（細格河網密，全部畫會成網格）；線寬隨流量。
            if fine.river[i] == 1 && f < 0.45 {
                continue;
            }
            let r = (ps as f32 * (1.8 * (f - 0.45).max(0.0) * (f - 0.45).max(0.0) * 4.0)).round() as i64;
            let (cx, cy) = ((x * ps + ps / 2) as i64, (y * ps + ps / 2) as i64);
            if r <= 0 {
                cv.set(cx, cy, ATLAS_RIVER);
                cv.set(cx - 1, cy, ATLAS_RIVER);
            } else {
                cv.disc(cx, cy, r, ATLAS_RIVER);
            }
        }
    }
    // 3. 海岸線：陸格與海格之間的細線。
    for y in 0..fh {
        for x in 0..fw {
            let i = y * fw + x;
            if fine.terrain[i] == Terrain::Sea {
                continue;
            }
            let coast = [(x + 1) % fw, (x + fw - 1) % fw].iter().any(|&nx| fine.terrain[y * fw + nx] == Terrain::Sea)
                || (y > 0 && fine.terrain[(y - 1) * fw + x] == Terrain::Sea)
                || (y + 1 < fh && fine.terrain[(y + 1) * fw + x] == Terrain::Sea);
            if coast {
                // 一像素寬的海岸線：畫在陸格朝海的邊上。
                let (cx, cy) = ((x * ps) as i64, (y * ps) as i64);
                let p = ps as i64;
                if fine.terrain[y * fw + (x + 1) % fw] == Terrain::Sea {
                    cv.rect(cx + p - 1, cy, 1, p, ATLAS_COAST);
                }
                if fine.terrain[y * fw + (x + fw - 1) % fw] == Terrain::Sea {
                    cv.rect(cx, cy, 1, p, ATLAS_COAST);
                }
                if y > 0 && fine.terrain[(y - 1) * fw + x] == Terrain::Sea {
                    cv.rect(cx, cy, p, 1, ATLAS_COAST);
                }
                if y + 1 < fh && fine.terrain[(y + 1) * fw + x] == Terrain::Sea {
                    cv.rect(cx, cy + p - 1, p, 1, ATLAS_COAST);
                }
            }
        }
    }
    let _ = geo;
    cv
}

/// 依圖層算圖。`scale` 為每格像素數（至少 1）。
pub fn render(geo: &Geography, fine: &FineGeography, provinces: &[Province], layer: Layer, scale: usize) -> Canvas {
    let s = scale.max(1);
    if layer == Layer::Atlas && !fine.is_empty() {
        return atlas(geo, fine, s);
    }
    if layer == Layer::Height && !fine.is_empty() {
        let (fw, fh) = (fine.w as usize, fine.h as usize);
        let ps = (s / fine.scale.max(1) as usize).max(1);
        let mut cv = Canvas::new(fw * ps, fh * ps, PAPER);
        for y in 0..fh {
            for x in 0..fw {
                let v = (fine.height_at(y * fw + x) * 255.0).round() as u8;
                cv.rect((x * ps) as i64, (y * ps) as i64, ps as i64, ps as i64, [v, v, v]);
            }
        }
        return cv;
    }
    let mut cv = Canvas::new(geo.w as usize * s, geo.h as usize * s, PAPER);
    match layer {
        Layer::Atlas | Layer::Height => {
            terrain_pixels(&mut cv, geo, s);
            rivers(&mut cv, geo, s);
        }
        Layer::Terrain => {
            terrain_pixels(&mut cv, geo, s);
            rivers(&mut cv, geo, s);
        }
        Layer::All => {
            terrain_pixels(&mut cv, geo, s);
            rivers(&mut cv, geo, s);
            province_borders(&mut cv, geo, s, INK);
            node_dots(&mut cv, geo, s);
            seat_squares(&mut cv, geo, provinces, s);
        }
        Layer::Province => {
            let cols = province_colors(provinces);
            fill_cells(&mut cv, geo, s, |i| {
                let p = geo.province_of[i];
                if p == NO_PROVINCE {
                    terrain_color(geo.terrain[i])
                } else {
                    cols[p as usize]
                }
            });
            province_borders(&mut cv, geo, s, INK);
        }
        Layer::Aura => {
            fill_cells(&mut cv, geo, s, |i| if geo.terrain[i].is_water() { PAPER_DARK } else { aura_ramp(geo.aura_cell[i]) });
            for e in &geo.ley {
                for &c in &e.path {
                    let (cx, cy) = cell_center(geo, c, s as i64);
                    cv.rect(cx - 1, cy - 1, 2, 2, CYAN);
                }
            }
            node_dots(&mut cv, geo, s);
        }
        Layer::Karma => {
            let max = provinces.iter().map(|p| p.karma_qi).fold(1.0f32, f32::max);
            fill_cells(&mut cv, geo, s, |i| {
                let p = geo.province_of[i];
                if p == NO_PROVINCE {
                    PAPER_DARK
                } else {
                    karma_ramp(provinces[p as usize].karma_qi / max)
                }
            });
            province_borders(&mut cv, geo, s, INK_LIGHT);
        }
    }
    cv
}

/// 側車文字：州郡、江河、山脈、洞天（地名不渲染於圖上）。
pub fn sidecar(geo: &Geography, provinces: &[Province]) -> String {
    let mut o = String::new();
    o.push_str("# 州郡 id\t名\t治所x,y\t人口\t靈氣\n");
    for p in provinces {
        let (x, y) = geo.xy(p.seat);
        o.push_str(&format!("{}\t{}\t{},{}\t{:.0}\t{:.2}\n", p.id, p.name, x, y, p.population, p.aura));
    }
    o.push_str("# 江河 名\t級\t格數\n");
    for r in &geo.rivers {
        o.push_str(&format!("{}\t{}\t{}\n", r.name, r.level, r.cells.len()));
    }
    o.push_str("# 山脈 名\t主峰x,y\t格數\n");
    for r in &geo.ranges {
        let (x, y) = geo.xy(r.peak);
        o.push_str(&format!("{}\t{},{}\t{}\n", r.name, x, y, r.cells.len()));
    }
    o.push_str("# 洞天 名\tx,y\t強度\n");
    for n in &geo.nodes {
        let (x, y) = geo.xy(n.cell);
        o.push_str(&format!("{}\t{},{}\t{:.2}\n", n.name, x, y, n.strength));
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use siming_sim::{LeyEdge, LeyNode, NamedRiver};

    /// 8x6 小世界：左半州 0、右半州 1，最上列為海，一條河、一個節點。
    fn tiny() -> (Geography, Vec<Province>) {
        let (w, h) = (8u16, 6u16);
        let n = (w * h) as usize;
        let mut terrain = vec![Terrain::Plains; n];
        let mut province_of = vec![0u16; n];
        let mut height = vec![0.5f32; n];
        for x in 0..w as usize {
            terrain[x] = Terrain::Sea;
            province_of[x] = NO_PROVINCE;
            height[x] = 0.1;
        }
        for y in 1..h as usize {
            for x in 0..w as usize {
                let i = y * w as usize + x;
                if x >= 4 {
                    province_of[i] = 1;
                }
                if x == 7 {
                    terrain[i] = Terrain::Mountain;
                    height[i] = 0.9;
                }
            }
        }
        let mut river = vec![0u8; n];
        for y in 1..h as usize {
            river[y * w as usize + 2] = 2;
        }
        let geo = Geography {
            w,
            h,
            wrap_x: false,
            sea_level: 0.3,
            height,
            terrain,
            river,
            province_of,
            aura_cell: (0..n).map(|i| (i % 5) as f32 * 0.5).collect(),
            nodes: vec![LeyNode { cell: 3 * 8 + 5, strength: 1.0, origin: 1.0, name: "甲洞天".into() }],
            ley: vec![LeyEdge { a: 0, b: 0, strength: 1.0, path: vec![3 * 8 + 5, 3 * 8 + 6] }],
            rivers: vec![NamedRiver { name: "乙水".into(), level: 2, cells: vec![10, 18] }],
            ranges: vec![],
            continents: vec![],
            continent_of: vec![siming_sim::NO_CONTINENT; n],
        };
        let mk = |id: u16, seat: u32, nb: u16| Province {
            id,
            name: format!("州{id}"),
            cells: vec![seat],
            seat,
            neighbors: vec![nb],
            sea_neighbors: vec![],
            terrain_mix: [0; Terrain::COUNT],
            habitability: 1.0,
            capacity: 1.0,
            population: 1000.0,
            aura: 0.5,
            karma_qi: 0.0,
            merit: 0.0,
            righteous: 0.0,
            devastation: 0.0,
            faith: [0.0; 3],
            city_level: 0,
            lost: false,
        };
        (geo, vec![mk(0, 2 * 8 + 1, 1), mk(1, 2 * 8 + 5, 0)])
    }

    #[test]
    fn ramps_hit_anchor_colours() {
        assert_eq!(aura_ramp(0.0), PAPER);
        assert_eq!(aura_ramp(1.0), CYAN);
        assert_eq!(aura_ramp(2.0), GOLD);
        assert_eq!(aura_ramp(9.0), GOLD);
        assert_eq!(karma_ramp(0.0), PAPER);
        assert_eq!(karma_ramp(1.0), KARMA);
        assert_eq!(karma_ramp(-1.0), PAPER);
    }

    #[test]
    fn adjacent_provinces_get_different_colours() {
        let (_, p) = tiny();
        let c = province_colors(&p);
        assert_ne!(c[0], c[1]);
    }

    #[test]
    fn layers_render_expected_size_and_content() {
        let (geo, p) = tiny();
        for l in [Layer::All, Layer::Terrain, Layer::Province, Layer::Aura, Layer::Karma] {
            let cv = render(&geo, &FineGeography::default(), &p, l, 4);
            assert_eq!((cv.w, cv.h), (32, 24));
        }
        let t = render(&geo, &FineGeography::default(), &p, Layer::Terrain, 4);
        assert_eq!(t.get(0, 0), WATER_DEEP);
        assert_eq!(t.get(2 * 4 + 2, 3 * 4 + 2), WATER_DEEP); // 河
        assert_eq!(t.get(4, 12), PAPER); // 平原
        let a = render(&geo, &FineGeography::default(), &p, Layer::All, 4);
        assert_eq!(a.get(4 * 4 - 1, 22), INK); // 州界
        let k = render(&geo, &FineGeography::default(), &p, Layer::Karma, 4);
        assert_eq!(k.get(4, 12), PAPER); // 業氣為零
    }

    #[test]
    fn writes_valid_png() {
        let (geo, p) = tiny();
        let dir = std::env::temp_dir().join(format!("siming-cli-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("map.png");
        render(&geo, &FineGeography::default(), &p, Layer::All, 3).write_png(&path).unwrap();
        let dec = png::Decoder::new(std::fs::File::open(&path).unwrap());
        let mut r = dec.read_info().unwrap();
        let mut buf = vec![0; r.output_buffer_size()];
        let info = r.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height), (24, 18));
        let txt = sidecar(&geo, &p);
        assert!(txt.contains("州0") && txt.contains("乙水") && txt.contains("甲洞天"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
