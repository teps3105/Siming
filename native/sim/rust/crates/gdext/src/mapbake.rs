//! GDD 5.3、8.3 地圖烘焙：把世界資料換算成地形網格、貼圖與河流折線。
//! 只做顯示用的換算，不改變世界狀態、不進世界雜湊（GDD 6.1 依賴原則）；不依賴 godot，可單獨測試。

use siming_sim::{Geography, Province, Terrain, NO_PROVINCE};

/// 貼圖相對遊戲格的超取樣倍率（海岸有號場、水深）。
pub const SS: usize = 4;
/// 地形網格相對遊戲格的超取樣倍率（GDD 5.3：2×2）。
pub const MESH_SS: usize = 2;
/// GDD 5.2：平面視角下南北緣之外可見的寬度（地圖寬的比例）。
pub const FRONTIER_FRAC: f32 = 0.05;
/// `paper` 與 `stroke` 貼圖的邊長。
pub const PAPER_SIZE: usize = 512;
pub const STROKE_SIZE: usize = 256;

const PAPER_SEED: u32 = 0x5A17_0001;
const STROKE_SEED: u32 = 0x5A17_0002;

// ---------------------------------------------------------------- 雜訊工具

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2D)
        ^ (y as u32).wrapping_mul(0x1656_67B1)
        ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// 可平鋪的值雜訊。`(u, v)` 為 [0,1) 的紋理座標，`fx, fy` 為整數頻率（格數）。
fn value_noise(u: f32, v: f32, fx: i32, fy: i32, seed: u32) -> f32 {
    let (x, y) = (u * fx as f32, v * fy as f32);
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (x - x0, y - y0);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let g = |dx: i32, dy: i32| hash01((ix + dx).rem_euclid(fx), (iy + dy).rem_euclid(fy), seed);
    let a = g(0, 0) + (g(1, 0) - g(0, 0)) * sx;
    let b = g(0, 1) + (g(1, 1) - g(0, 1)) * sx;
    a + (b - a) * sy
}

fn fbm(u: f32, v: f32, fx: i32, fy: i32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm) = (0.0, 1.0, 0.0);
    for o in 0..octaves {
        let k = 1 << o;
        sum += amp * value_noise(u, v, fx * k, fy * k, seed.wrapping_add(o * 101));
        norm += amp;
        amp *= 0.5;
    }
    sum / norm
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------- 程序貼圖

/// `paper`：512×512 平鋪 RGBA8。低頻明暗斑 + 高頻纖維。存的是乘數（均值約 0.96），色相由 uniform 的 paper 色決定。
pub fn gen_paper() -> Vec<u8> {
    const N: usize = PAPER_SIZE;
    let mut lum = vec![0f32; N * N];
    for y in 0..N {
        for x in 0..N {
            let (u, v) = (x as f32 / N as f32, y as f32 / N as f32);
            let blotch = fbm(u, v, 3, 3, 4, PAPER_SEED) - 0.5;
            let fine = value_noise(u, v, 128, 128, PAPER_SEED ^ 7) - 0.5;
            lum[y * N + x] = 0.965 + blotch * 0.12 + fine * 0.035;
        }
    }
    // 纖維：沿隨機方向的短線段（環繞平鋪）。
    for i in 0..2600u32 {
        let r = |k: u32| hash01(i as i32, k as i32, PAPER_SEED ^ 0x77);
        let (px, py) = (r(1) * N as f32, r(2) * N as f32);
        let ang = r(3) * std::f32::consts::TAU;
        let len = 5.0 + r(4) * 18.0;
        let delta = (r(5) - 0.5) * 0.09;
        let steps = len as usize;
        for s in 0..steps {
            let t = s as f32;
            let (fx, fy) = (px + ang.cos() * t, py + ang.sin() * t);
            let (ix, iy) = (
                fx.rem_euclid(N as f32) as usize,
                fy.rem_euclid(N as f32) as usize,
            );
            lum[iy * N + ix] += delta;
        }
    }
    let mut data = Vec::with_capacity(N * N * 4);
    for l in lum {
        let l = l.clamp(0.0, 1.0);
        let q = |c: f32| (l * c * 255.0).round().clamp(0.0, 255.0) as u8;
        data.extend_from_slice(&[q(1.0), q(0.985), q(0.955), 255]);
    }
    data
}

/// `stroke`：256×256 平鋪 R8。橫向拉伸 4 倍的 fBm + 閾值化，模擬皴法乾筆。
pub fn gen_stroke() -> Vec<u8> {
    const N: usize = STROKE_SIZE;
    let mut data = vec![0u8; N * N];
    for y in 0..N {
        for x in 0..N {
            let (u, v) = (x as f32 / N as f32, y as f32 / N as f32);
            let streak = fbm(u, v, 3, 14, 4, STROKE_SEED); // 橫向頻率低 → 橫向拉長約 4–5 倍
            let grain = value_noise(u, v, 96, 96, STROKE_SEED ^ 5);
            let dry =
                smoothstep(0.38, 0.62, streak) * (0.55 + 0.45 * smoothstep(0.25, 0.75, grain));
            data[y * N + x] = (dry * 255.0) as u8;
        }
    }
    data
}

// ---------------------------------------------------------------- 世界貼圖

fn bilinear(grid: &[f32], w: usize, h: usize, x: f32, y: f32, wrap: bool) -> f32 {
    // 東西環繞：x 以 w 為週期，x = w − 1 與 x = 0 相鄰（GDD 2.2）。
    let x = if wrap {
        x.rem_euclid(w as f32)
    } else {
        x.clamp(0.0, (w - 1) as f32)
    };
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = ((x.floor() as usize).min(w - 1), y.floor() as usize);
    let x1 = if wrap {
        (x0 + 1) % w
    } else {
        (x0 + 1).min(w - 1)
    };
    let y1 = (y0 + 1).min(h - 1);
    let (tx, ty) = (x - x0 as f32, y - y0 as f32);
    let a = grid[y0 * w + x0] * (1.0 - tx) + grid[y0 * w + x1] * tx;
    let b = grid[y1 * w + x0] * (1.0 - tx) + grid[y1 * w + x1] * tx;
    a * (1.0 - ty) + b * ty
}

/// 多源 BFS（8 鄰）：水格到最近陸地格的步數，以及最近陸地格的地形代碼（陸地格即自身）。
fn land_distance(geo: &Geography) -> (Vec<f32>, Vec<u8>) {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let n = w * h;
    let mut dist = vec![f32::MAX; n];
    let mut code = vec![2u8; n];
    let mut q = std::collections::VecDeque::new();
    for i in 0..n {
        if !geo.terrain[i].is_water() {
            dist[i] = 0.0;
            code[i] = geo.terrain[i] as u8;
            q.push_back(i);
        }
    }
    while let Some(i) = q.pop_front() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (mut nx, ny) = (x + dx, y + dy);
                if geo.wrap_x {
                    nx = nx.rem_euclid(w as i32);
                }
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if dist[j] == f32::MAX {
                    dist[j] = dist[i] + 1.0;
                    code[j] = code[i];
                    q.push_back(j);
                }
            }
        }
    }
    for d in &mut dist {
        if *d == f32::MAX {
            *d = 16.0;
        }
    }
    (dist, code)
}

/// 二次 B 樣條取樣（格中心在整數座標）：模糊與內插合一，等值線 C1 連續。
fn bspline_sample(grid: &[f32], w: usize, h: usize, x: f32, y: f32, wrap: bool) -> f32 {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let mut sum = 0.0;
    for j in y0 - 1..=y0 + 2 {
        let wy = bspline2(y - j as f32);
        if wy <= 0.0 {
            continue;
        }
        let cy = j.clamp(0, h as i32 - 1) as usize;
        for i in x0 - 1..=x0 + 2 {
            let wx = bspline2(x - i as f32);
            if wx <= 0.0 {
                continue;
            }
            let cx = if wrap {
                i.rem_euclid(w as i32)
            } else {
                i.clamp(0, w as i32 - 1)
            } as usize;
            sum += grid[cy * w + cx] * wx * wy;
        }
    }
    sum
}

/// 3×3 二項式模糊（1 2 1 ⊗ 1 2 1）／16。
fn blur_binomial(src: &[f32], w: usize, h: usize, wrap: bool) -> Vec<f32> {
    let k = [1.0f32, 2.0, 1.0];
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for (j, ky) in k.iter().enumerate() {
                let ny = (y as i32 + j as i32 - 1).clamp(0, h as i32 - 1) as usize;
                for (i, kx) in k.iter().enumerate() {
                    let mut nx = x as i32 + i as i32 - 1;
                    nx = if wrap {
                        nx.rem_euclid(w as i32)
                    } else {
                        nx.clamp(0, w as i32 - 1)
                    };
                    sum += src[ny * w + nx as usize] * ky * kx;
                }
            }
            out[y * w + x] = sum / 16.0;
        }
    }
    out
}

/// 州郡 ID 向水域外擴 `steps` 格（8 鄰，最近優先）。
fn dilate_provinces(geo: &Geography, steps: u32) -> Vec<u16> {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let mut cur = geo.province_of.clone();
    for _ in 0..steps {
        let prev = cur.clone();
        for y in 0..h {
            for x in 0..w {
                if prev[y * w + x] != NO_PROVINCE {
                    continue;
                }
                'n: for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (mut nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if geo.wrap_x {
                            nx = nx.rem_euclid(w as i32);
                        }
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let p = prev[ny as usize * w + nx as usize];
                        if p != NO_PROVINCE {
                            cur[y * w + x] = p;
                            break 'n;
                        }
                    }
                }
            }
        }
    }
    cur
}

/// 二次 B 樣條核（支撐 3 格、C1 連續）。
fn bspline2(d: f32) -> f32 {
    let a = d.abs();
    if a < 0.5 {
        0.75 - a * a
    } else if a < 1.5 {
        0.5 * (1.5 - a) * (1.5 - a)
    } else {
        0.0
    }
}

/// 每個河格的下游：(格, dx, dy)。複製模擬的 D8 最陡下降（坡度以球面步長計：東西向 cos 緯度）；
/// 無更低鄰格的平地則取等高的水／河鄰格；具名幹流（由源頭到出海口的格序）覆寫為權威答案。
/// 只連到河格或水格，避免沿岸每個河格各畫一根垂直短刺（梳齒）。
fn river_flow(
    geo: &Geography,
    nbrs: &dyn Fn(usize) -> Vec<(usize, i32, i32)>,
    is_river: &dyn Fn(usize) -> bool,
) -> Vec<Option<(usize, i32, i32)>> {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let row_cos = |y: usize| {
        (std::f32::consts::PI * (y as f32 + 0.5) / h as f32)
            .sin()
            .max(1e-3)
    };
    let mut down: Vec<Option<(usize, i32, i32)>> = vec![None; w * h];
    for (i, d) in down.iter_mut().enumerate() {
        if !is_river(i) {
            continue;
        }
        let hi = geo.height[i];
        let mut best: Option<(usize, i32, i32, f32)> = None;
        for (j, dx, dy) in nbrs(i) {
            if geo.height[j] >= hi {
                continue;
            }
            let ew = if dx != 0 {
                0.5 * (row_cos(i / w) + row_cos(j / w))
            } else {
                0.0
            };
            let ns = if dy != 0 { 1.0 } else { 0.0 };
            let slope = (hi - geo.height[j]) / (ew * ew + ns * ns).sqrt().max(1e-3);
            if best.is_none_or(|b| slope > b.3) {
                best = Some((j, dx, dy, slope));
            }
        }
        if best.is_none() {
            // 平地：取等高（容差內）的水格，其次河格
            for (j, dx, dy) in nbrs(i) {
                let ok = geo.terrain[j].is_water() || is_river(j);
                if ok
                    && (geo.height[j] - hi).abs() < 1e-5
                    && best
                        .is_none_or(|b| geo.terrain[j].is_water() && !geo.terrain[b.0].is_water())
                {
                    best = Some((j, dx, dy, 0.0));
                }
            }
        }
        if let Some((j, dx, dy, _)) = best {
            if geo.terrain[j].is_water() || is_river(j) {
                *d = Some((j, dx, dy));
            }
        }
    }
    // 具名幹流：相鄰格依序相連
    for r in &geo.rivers {
        for pair in r.cells.windows(2) {
            let (a, b) = (pair[0] as usize, pair[1] as usize);
            if !is_river(a) {
                continue;
            }
            if let Some(&(j, dx, dy)) = nbrs(a).iter().find(|n| n.0 == b) {
                down[a] = Some((j, dx, dy));
            }
        }
    }
    down
}

/// GDD 5.1：州郡依序貪婪著色，鄰郡不同色。回傳每州的調色盤索引。
pub fn color_provinces(provinces: &[Province]) -> Vec<usize> {
    let mut idx = vec![usize::MAX; provinces.len()];
    for p in provinces {
        let id = p.id as usize;
        let mut used = [false; 24];
        for &n in &p.neighbors {
            if let Some(&c) = idx.get(n as usize) {
                if c != usize::MAX {
                    used[c] = true;
                }
            }
        }
        let start = (id * 5) % 24;
        idx[id] = (0..24)
            .map(|k| (start + k) % 24)
            .find(|&c| !used[c])
            .unwrap_or(start);
    }
    idx
}

/// 世界貼圖的原始位元組（GDD 5.3「貼圖」）。
pub struct WorldTextures {
    /// `SS` 倍尺寸的 RGBA8：R、G 保留、B = 正規化水深、A = 陸／水有號場（0.5 為海岸線）。
    pub terrain: Vec<u8>,
    /// `SS` 倍尺寸的 R8：格點靈氣 / 2。
    pub aura: Vec<u8>,
    /// 遊戲格尺寸的 RGBA8：平原、丘陵、山脈、森林的類別權重。
    pub cls_a: Vec<u8>,
    /// 遊戲格尺寸的 RGBA8：雪原、沙漠、沼澤的類別權重。
    pub cls_b: Vec<u8>,
    /// 遊戲格尺寸的 RG8：州郡 ID 低、高位元組（向水域外擴 2 格；無州郡為 65535）。
    pub province_id: Vec<u8>,
}

/// 建立世界貼圖。
pub fn build_world_textures(geo: &Geography) -> WorldTextures {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let (tw, th) = (w * SS, h * SS);

    // 海岸：陸／水有號場與 3×3 二項式模糊各取一半，再以二次 B 樣條取樣（等值線圓順、單格湖不消失）。
    let signed_raw: Vec<f32> = geo
        .terrain
        .iter()
        .map(|t| if t.is_water() { -1.0 } else { 1.0 })
        .collect();
    let signed: Vec<f32> = blur_binomial(&signed_raw, w, h, geo.wrap_x)
        .iter()
        .zip(&signed_raw)
        .map(|(b, r)| 0.5 * (b + r))
        .collect();
    let (dist, nland) = land_distance(geo);
    // 海深（GDD 5.3「水深」）：正規化深度 (海平面 − 高度) / 海平面；湖以離岸距離。
    let sea_level = geo.sea_level.max(1e-3);
    let depth_cell: Vec<f32> = (0..w * h)
        .map(|i| match geo.terrain[i] {
            Terrain::Sea => ((sea_level - geo.height[i]) / sea_level).clamp(0.0, 1.0),
            Terrain::Lake => 0.35 * (dist[i] / 4.0).clamp(0.0, 1.0),
            _ => 0.0,
        })
        .collect();
    let depth_cell = blur_binomial(&depth_cell, w, h, geo.wrap_x);
    let aura_cell: Vec<f32> = geo
        .aura_cell
        .iter()
        .map(|a| (a / 2.0).clamp(0.0, 1.0))
        .collect();

    let mut terrain = vec![0u8; tw * th * 4];
    let mut aura = vec![0u8; tw * th];
    for ty in 0..th {
        for tx in 0..tw {
            let o = ty * tw + tx;
            // 紋素中心在格點座標（格中心為整數）的位置
            let (gx, gy) = (
                (tx as f32 + 0.5) / SS as f32 - 0.5,
                (ty as f32 + 0.5) / SS as f32 - 0.5,
            );
            let f = bspline_sample(&signed, w, h, gx, gy, geo.wrap_x);
            let d = bspline_sample(&depth_cell, w, h, gx, gy, geo.wrap_x);
            terrain[o * 4 + 2] = (d.clamp(0.0, 1.0) * 255.0).round() as u8;
            terrain[o * 4 + 3] = ((0.5 + 0.5 * f) * 255.0).round() as u8;
            aura[o] = (bilinear(&aura_cell, w, h, gx, gy, geo.wrap_x) * 255.0).round() as u8;
        }
    }

    // 地形類別權重：遮罩 → 3×3 模糊 → RGBA8。
    let mut cls_a = vec![0u8; w * h * 4];
    let mut cls_b = vec![0u8; w * h * 4];
    for k in 2u8..=8 {
        let mask: Vec<f32> = nland
            .iter()
            .map(|&c| if c == k { 1.0 } else { 0.0 })
            .collect();
        let blurred = blur_binomial(&mask, w, h, geo.wrap_x);
        let (buf, ch) = if k <= 5 {
            (&mut cls_a, (k - 2) as usize)
        } else {
            (&mut cls_b, (k - 6) as usize)
        };
        for i in 0..w * h {
            buf[i * 4 + ch] = (blurred[i] * 255.0).round() as u8;
        }
    }

    let province_id: Vec<u8> = dilate_provinces(geo, 2)
        .iter()
        .flat_map(|p| p.to_le_bytes())
        .collect();
    WorldTextures {
        terrain,
        aura,
        cls_a,
        cls_b,
        province_id,
    }
}

/// GDD 5.1 勢力著色：24 色淡彩調色盤（HSV 彩度 0.25–0.35、明度 0.75–0.85），sRGB 0..1。
pub fn pastel24() -> [[f32; 3]; 24] {
    let mut out = [[1.0f32; 3]; 24];
    for (i, o) in out.iter_mut().enumerate() {
        let hue = ((i * 7) % 24) as f32 * 15.0;
        let s = 0.25 + 0.10 * ((i % 3) as f32 / 2.0);
        let v = 0.75 + 0.10 * (((i / 3) % 3) as f32 / 2.0);
        let c = v * s;
        let hp = hue / 60.0;
        let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
        let (r, g, b) = match hp as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        *o = [r + v - c, g + v - c, b + v - c];
    }
    out
}

/// 州郡 ID → 淡彩的 256×1 RGBA8 查表（sRGB）；alpha 255 表示該州上色（GDD 5.4 天下模式）。
pub fn province_color_table(provinces: &[Province]) -> Vec<u8> {
    let pal = pastel24();
    let assignment = color_provinces(provinces);
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut t = vec![0u8; 256 * 4];
    for p in provinces {
        let id = p.id as usize;
        if id < 255 && p.id != NO_PROVINCE {
            let c = pal[assignment[id]];
            t[id * 4..id * 4 + 4].copy_from_slice(&[q(c[0]), q(c[1]), q(c[2]), 255]);
        }
    }
    t
}

// ---------------------------------------------------------------- 地形網格

/// GDD 5.3 地形高度：陸地與湖面 `(h − sea) × 地圖寬 × 0.04`；海面為 0（高度縮放 = 1）。
pub fn terrain_height(geo: &Geography, cell: usize) -> f32 {
    let h = geo.height[cell];
    // 湖面維持自身（填窪後）的高度；若壓成 0，高地上的湖會陷成一圈懸崖。
    if geo.terrain[cell] == Terrain::Sea || h < geo.sea_level {
        0.0
    } else {
        (h - geo.sea_level) * geo.w as f32 * 0.04
    }
}

/// 格點高度場的副本（高度縮放 = 1），供網格、河流與選取取高度。
pub struct TerrainCpu {
    pub w: usize,
    pub h: usize,
    pub heights: Vec<f32>,
    /// 陸地最高點。
    pub max_height: f32,
    pub wrap_x: bool,
}

impl TerrainCpu {
    pub fn new(geo: &Geography) -> Self {
        let n = geo.w as usize * geo.h as usize;
        let heights: Vec<f32> = (0..n).map(|c| terrain_height(geo, c)).collect();
        let max_height = heights.iter().cloned().fold(0.0, f32::max).max(0.001);
        Self {
            w: geo.w as usize,
            h: geo.h as usize,
            heights,
            max_height,
            wrap_x: geo.wrap_x,
        }
    }

    /// 世界座標 (x, z) 處的高度，格中心在 (x+0.5, z+0.5)，雙線性。
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        bilinear(&self.heights, self.w, self.h, x - 0.5, z - 0.5, self.wrap_x)
    }
}

/// 網格陣列：位置（y = 縮放 1 的高度）、法線、UV（整張地圖 0..1）、曲率（山脊線用）、索引。
#[derive(Default)]
pub struct MeshArrays {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub curvature: Vec<[f32; 2]>,
    pub indices: Vec<i32>,
}

/// GDD 5.3 地形網格：單一網格，格點高度以 2×2 雙線性超取樣；南北緣外各加一條極地邊疆帶。
/// 東西向多取一行頂點（x = 地圖寬），使相鄰副本在接縫處相接。
pub fn build_terrain_mesh(cpu: &TerrainCpu) -> MeshArrays {
    let (vw, vh) = (cpu.w * MESH_SS + 1, cpu.h * MESH_SS);
    let (mw, mh) = (cpu.w as f32, cpu.h as f32);
    let sx = mw / (vw - 1) as f32;
    let sz = mh / (vh - 1) as f32;
    let mut m = MeshArrays::default();
    for j in 0..vh {
        for i in 0..vw {
            let (x, z) = (i as f32 * sx, j as f32 * sz);
            let y = cpu.height_at(x, z);
            // 曲率（高度場 Hessian 的 Frobenius 範數，步長 1 格）：逐頂點算好再內插。
            let e = 1.0;
            let hxx = cpu.height_at(x + e, z) - 2.0 * y + cpu.height_at(x - e, z);
            let hzz = cpu.height_at(x, z + e) - 2.0 * y + cpu.height_at(x, z - e);
            let hxz = (cpu.height_at(x + e, z + e)
                - cpu.height_at(x + e, z - e)
                - cpu.height_at(x - e, z + e)
                + cpu.height_at(x - e, z - e))
                / 4.0;
            m.curvature
                .push([(hxx * hxx + hzz * hzz + 2.0 * hxz * hxz).sqrt(), 0.0]);
            m.positions.push([x, y, z]);
            m.uvs.push([x / mw, z / mh]);
            // 法線以 ±0.75 格的寬模板求中央差，避免格點階梯狀的山影。
            let e = 0.75;
            let dx = (cpu.height_at(x + e, z) - cpu.height_at(x - e, z)) / (2.0 * e);
            let dz = (cpu.height_at(x, z + e) - cpu.height_at(x, z - e)) / (2.0 * e);
            let len = (dx * dx + 1.0 + dz * dz).sqrt();
            m.normals.push([-dx / len, 1.0 / len, -dz / len]);
        }
    }
    let pad = mw * FRONTIER_FRAC * 1.6;
    let n0 = (vw * vh) as i32;
    for (zz, v) in [(-pad, 0.0f32), (mh + pad, 1.0)] {
        for i in 0..vw {
            let x = i as f32 * sx;
            m.positions.push([x, 0.0, zz]);
            m.uvs.push([x / mw, v]);
            m.normals.push([0.0, 1.0, 0.0]);
            m.curvature.push([0.0, 0.0]);
        }
    }
    // Godot 以順時針為正面（由上方看）。
    for j in 0..vh - 1 {
        for i in 0..vw - 1 {
            let a = (j * vw + i) as i32;
            let (b, c, d) = (a + 1, a + vw as i32, a + vw as i32 + 1);
            m.indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    for i in 0..vw as i32 - 1 {
        let (no, nm) = (n0 + i, i);
        m.indices
            .extend_from_slice(&[no, no + 1, nm, no + 1, nm + 1, nm]);
        let (sm, so) = (((vh - 1) * vw) as i32 + i, n0 + vw as i32 + i);
        m.indices
            .extend_from_slice(&[sm, sm + 1, so, sm + 1, so + 1, so]);
    }
    m
}

// ---------------------------------------------------------------- 河流折線

/// 河流帶狀網格（GDD 5.3「河流」）：每個頂點帶位置（y = 縮放 1 的高度）、沿河切線、
/// `side`（±1，著色器依螢幕像素向兩側展開）、河流等級與收細係數。
#[derive(Default)]
pub struct RiverArrays {
    pub positions: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
    /// x = side（±1）、y = 等級（1 小河、2 大河、3 江）。
    pub side_level: Vec<[f32; 2]>,
    /// x = 寬度係數（源頭 0.6 → 1）、y 保留。
    pub taper: Vec<[f32; 2]>,
    pub indices: Vec<i32>,
}

/// 極區不畫河流的緯度門檻（GDD 5.3：|緯度| > 72°）。
const RIVER_MAX_LAT_DEG: f32 = 72.0;
/// 孤立的非具名河段至少要有這麼多格才畫。
const RIVER_MIN_CELLS: u32 = 4;
/// 每個河格的貝茲曲線取樣段數。
const RIVER_CURVE_STEPS: usize = 6;

/// 依高度推出每個河格的下游格，每格以「經上、下游中點、本格為控制點」的二次貝茲曲線取樣成帶。
pub fn build_river_mesh(geo: &Geography, cpu: &TerrainCpu) -> RiverArrays {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let is_river = |i: usize| geo.river[i] > 0 && !geo.terrain[i].is_water();
    // 顯示等級：具名幹流整條以該河的等級繪製（否則同一條江上下游等級不同，遠景會斷成虛線）。
    let mut display_level: Vec<u8> = geo.river.iter().map(|&r| r.min(3)).collect();
    for r in &geo.rivers {
        for &c in &r.cells {
            let c = c as usize;
            display_level[c] = display_level[c].max(r.level.min(3));
        }
    }
    let lvl = |i: usize| display_level[i] as f32;
    let nbrs = |i: usize| {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let mut v = Vec::with_capacity(8);
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx_raw, ny) = (x + dx, y + dy);
                let nx = if geo.wrap_x {
                    nx_raw.rem_euclid(w as i32)
                } else {
                    nx_raw
                };
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                v.push((ny as usize * w + nx as usize, dx, dy));
            }
        }
        v
    };
    let down = river_flow(geo, &nbrs, &is_river);
    let mut ups: Vec<Vec<(usize, i32, i32)>> = vec![Vec::new(); w * h];
    for (i, d) in down.iter().enumerate() {
        if let Some((j, dx, dy)) = *d {
            if is_river(j) {
                ups[j].push((i, -dx, -dy));
            }
        }
    }
    // 略過不足 4 格的孤立非具名河段（GDD 5.3）：否則近景滿是零碎短劃。
    let mut comp: Vec<usize> = (0..w * h).collect();
    fn root(comp: &mut [usize], mut i: usize) -> usize {
        while comp[i] != i {
            comp[i] = comp[comp[i]];
            i = comp[i];
        }
        i
    }
    for (i, d) in down.iter().enumerate() {
        if let Some((j, _, _)) = *d {
            if is_river(i) && is_river(j) {
                let (a, b) = (root(&mut comp, i), root(&mut comp, j));
                comp[a] = b;
            }
        }
    }
    let mut comp_size = vec![0u32; w * h];
    for i in 0..w * h {
        if is_river(i) {
            let r = root(&mut comp, i);
            comp_size[r] += 1;
        }
    }
    for r in &geo.rivers {
        for &c in &r.cells {
            let r = root(&mut comp, c as usize);
            comp_size[r] = u32::MAX;
        }
    }
    let keep: Vec<bool> = (0..w * h)
        .map(|i| is_river(i) && comp_size[root(&mut comp, i)] >= RIVER_MIN_CELLS)
        .collect();
    let lat_of_row = |y: usize| 90.0 - 180.0 * (y as f32 + 0.5) / h as f32;
    let mut out = RiverArrays::default();
    // 一條曲線：p0 → p2，控制點 p1；k0、k2 為兩端的寬度係數。
    let mut curve =
        |p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), level: f32, k0: f32, k2: f32| {
            let at = |t: f32| {
                let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t);
                (
                    a * p0.0 + b * p1.0 + c * p2.0,
                    a * p0.1 + b * p1.1 + c * p2.1,
                )
            };
            let n = RIVER_CURVE_STEPS;
            let pts: Vec<[f32; 3]> = (0..=n)
                .map(|k| {
                    let (x, z) = at(k as f32 / n as f32);
                    [x, cpu.height_at(x, z), z]
                })
                .collect();
            let base = out.positions.len() as i32;
            for k in 0..=n {
                let (a, b) = (pts[k.saturating_sub(1)], pts[(k + 1).min(n)]);
                let mut t = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let len = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt().max(1e-6);
                t = [t[0] / len, t[1] / len, t[2] / len];
                let kk = k0 + (k2 - k0) * k as f32 / n as f32;
                for side in [-1.0f32, 1.0] {
                    out.positions.push(pts[k]);
                    out.tangents.push(t);
                    out.side_level.push([side, level]);
                    out.taper.push([kk, 0.0]);
                }
            }
            for k in 0..n as i32 {
                let a = base + k * 2;
                out.indices
                    .extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
            }
        };
    for i in 0..w * h {
        if !keep[i] || lat_of_row(i / w).abs() > RIVER_MAX_LAT_DEG {
            continue;
        }
        let c = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
        let off = |dx: i32, dy: i32| (c.0 + dx as f32 * 0.5, c.1 + dy as f32 * 0.5);
        let level = lvl(i);
        match down[i] {
            Some((_, dx, dy)) => {
                let end = off(dx, dy);
                if ups[i].is_empty() {
                    // 源頭：由本格中心出發，收細。
                    curve(c, c, end, level, 0.6, 1.0);
                } else {
                    for &(_, px, py) in &ups[i] {
                        curve(off(px, py), c, end, level, 1.0, 1.0);
                    }
                }
            }
            None => {
                // 終點（窪地）：由各上游中點收束到本格中心。
                for &(_, px, py) in &ups[i] {
                    curve(off(px, py), c, c, level, 1.0, 1.0);
                }
            }
        }
    }
    out
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
    fn gdd_5_3_world_textures_have_expected_sizes() {
        let sim = small_world();
        let geo = &sim.world().geo;
        let (w, h) = (geo.w as usize, geo.h as usize);
        let t = build_world_textures(geo);
        assert_eq!(t.terrain.len(), w * SS * h * SS * 4);
        assert_eq!(t.aura.len(), w * SS * h * SS);
        assert_eq!(t.cls_a.len(), w * h * 4);
        assert_eq!(t.cls_b.len(), w * h * 4);
        assert_eq!(t.province_id.len(), w * h * 2);
        assert_eq!(gen_paper().len(), PAPER_SIZE * PAPER_SIZE * 4);
        assert_eq!(gen_stroke().len(), STROKE_SIZE * STROKE_SIZE);
        assert_eq!(province_color_table(&sim.world().provinces).len(), 256 * 4);
    }

    #[test]
    fn gdd_5_3_terrain_mesh_covers_map_and_frontier() {
        let sim = small_world();
        let geo = &sim.world().geo;
        let cpu = TerrainCpu::new(geo);
        let m = build_terrain_mesh(&cpu);
        let (vw, vh) = (cpu.w * MESH_SS + 1, cpu.h * MESH_SS);
        assert_eq!(m.positions.len(), vw * vh + 2 * vw);
        assert_eq!(m.normals.len(), m.positions.len());
        assert_eq!(m.uvs.len(), m.positions.len());
        assert_eq!(m.curvature.len(), m.positions.len());
        assert_eq!(m.indices.len(), (vw - 1) * (vh - 1) * 6 + (vw - 1) * 12);
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
        // 東西接縫：首末兩行頂點高度相同（x = 0 與 x = 地圖寬）。
        for j in 0..vh {
            let (a, b) = (m.positions[j * vw], m.positions[j * vw + vw - 1]);
            assert!((a[1] - b[1]).abs() < 1e-4, "列 {j} 接縫高度不連續");
        }
    }

    #[test]
    fn gdd_5_3_river_mesh_is_strip_outside_polar_zone() {
        let sim = small_world();
        let geo = &sim.world().geo;
        let cpu = TerrainCpu::new(geo);
        let r = build_river_mesh(geo, &cpu);
        assert!(!r.positions.is_empty(), "世界應有河流");
        assert_eq!(r.positions.len() % ((RIVER_CURVE_STEPS + 1) * 2), 0);
        assert_eq!(r.tangents.len(), r.positions.len());
        assert_eq!(r.side_level.len(), r.positions.len());
        assert_eq!(r.taper.len(), r.positions.len());
        assert!(r.indices.iter().all(|&i| (i as usize) < r.positions.len()));
        let h = geo.h as f32;
        for (p, sl) in r.positions.iter().zip(&r.side_level) {
            let lat = 90.0 - 180.0 * p[2] / h;
            assert!(
                lat.abs() <= RIVER_MAX_LAT_DEG + 180.0 / h,
                "極區不應有河流頂點：緯度 {lat}"
            );
            assert!(sl[0].abs() == 1.0 && (1.0..=3.0).contains(&sl[1]));
        }
    }
}
