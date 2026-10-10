//! GDD 9.3 貼圖：程序貼圖（啟動時以固定種子產生，與世界種子無關）與世界貼圖。
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use siming_sim::{Geography, Province, NO_PROVINCE};

/// 世界貼圖相對格點的解析度倍率（1024×640 ÷ 256×160 = 4）。
pub const SS: usize = 4;

const PAPER_SEED: u32 = 0x5A17_0001;
const STROKE_SEED: u32 = 0x5A17_0002;

// ---------------------------------------------------------------- 雜訊工具

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2D) ^ (y as u32).wrapping_mul(0x1656_67B1) ^ seed.wrapping_mul(0x9E37_79B9);
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

// ---------------------------------------------------------------- 影像建立

fn sampler(addr: ImageAddressMode) -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: addr,
        address_mode_v: addr,
        address_mode_w: addr,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    })
}

fn make_image(w: usize, h: usize, format: TextureFormat, data: Vec<u8>, mips: u32, addr: ImageAddressMode) -> Image {
    let mut img = Image::new_uninit(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(data); // 含完整 mip 鏈
    img.texture_descriptor.mip_level_count = mips;
    img.sampler = sampler(addr);
    img
}

/// 逐層 2×2 方框縮小，附加 mip 鏈（`ch` 為每像素位元組數）。
fn with_mips(mut base: Vec<u8>, w: usize, h: usize, ch: usize) -> (Vec<u8>, u32) {
    let mut out = base.clone();
    let (mut cw, mut chh) = (w, h);
    let mut levels = 1;
    while cw > 1 && chh > 1 {
        let (nw, nh) = (cw / 2, chh / 2);
        let mut next = vec![0u8; nw * nh * ch];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..ch {
                    let s = |dx: usize, dy: usize| base[((2 * y + dy) * cw + 2 * x + dx) * ch + c] as u32;
                    next[(y * nw + x) * ch + c] = ((s(0, 0) + s(1, 0) + s(0, 1) + s(1, 1) + 2) / 4) as u8;
                }
            }
        }
        out.extend_from_slice(&next);
        base = next;
        cw = nw;
        chh = nh;
        levels += 1;
    }
    (out, levels)
}

// ---------------------------------------------------------------- 程序貼圖

/// `paper`：512×512 平鋪 RGBA8。低頻明暗斑 + 高頻纖維。存的是乘數（均值約 0.96），色相由 uniform 的 paper 色決定。
pub fn gen_paper() -> Image {
    const N: usize = 512;
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
            let (ix, iy) = (fx.rem_euclid(N as f32) as usize, fy.rem_euclid(N as f32) as usize);
            lum[iy * N + ix] += delta;
        }
    }
    let mut data = Vec::with_capacity(N * N * 4);
    for l in lum {
        let l = l.clamp(0.0, 1.0);
        let q = |c: f32| (l * c * 255.0).round().clamp(0.0, 255.0) as u8;
        data.extend_from_slice(&[q(1.0), q(0.985), q(0.955), 255]);
    }
    let (data, mips) = with_mips(data, N, N, 4);
    make_image(N, N, TextureFormat::Rgba8Unorm, data, mips, ImageAddressMode::Repeat)
}

/// `stroke`：256×256 平鋪 R8。橫向拉伸 4 倍的 fBm + 閾值化，模擬皴法乾筆。
pub fn gen_stroke() -> Image {
    const N: usize = 256;
    let mut data = vec![0u8; N * N];
    for y in 0..N {
        for x in 0..N {
            let (u, v) = (x as f32 / N as f32, y as f32 / N as f32);
            let streak = fbm(u, v, 3, 14, 4, STROKE_SEED); // 橫向頻率低 → 橫向拉長約 4–5 倍
            let grain = value_noise(u, v, 96, 96, STROKE_SEED ^ 5);
            let dry = smoothstep(0.38, 0.62, streak) * (0.55 + 0.45 * smoothstep(0.25, 0.75, grain));
            data[y * N + x] = (dry * 255.0) as u8;
        }
    }
    let (data, mips) = with_mips(data, N, N, 1);
    make_image(N, N, TextureFormat::R8Unorm, data, mips, ImageAddressMode::Repeat)
}

// ---------------------------------------------------------------- 世界貼圖

pub struct WorldImages {
    pub terrain: Image,
    pub province_id: Image,
    pub field: Image,
    pub province_color: Image,
    pub cls_a: Image,
    pub cls_b: Image,
}

fn bilinear(grid: &[f32], w: usize, h: usize, x: f32, y: f32, wrap: bool) -> f32 {
    // 東西環繞：x 以 w 為週期，x = w − 1 與 x = 0 相鄰（GDD 2.2）。
    let x = if wrap { x.rem_euclid(w as f32) } else { x.clamp(0.0, (w - 1) as f32) };
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = ((x.floor() as usize).min(w - 1), y.floor() as usize);
    let x1 = if wrap { (x0 + 1) % w } else { (x0 + 1).min(w - 1) };
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

/// 河流有號距離場的夾取範圍（紋素）；著色器以 `(G − 0.5)·2·RIVER_EM` 還原。
const RIVER_EM: f32 = 2.0;


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
            let cx = if wrap { i.rem_euclid(w as i32) } else { i.clamp(0, w as i32 - 1) } as usize;
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
                    nx = if wrap { nx.rem_euclid(w as i32) } else { nx.clamp(0, w as i32 - 1) };
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
    let row_cos = |y: usize| (std::f32::consts::PI * (y as f32 + 0.5) / h as f32).sin().max(1e-3);
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
            let ew = if dx != 0 { 0.5 * (row_cos(i / w) + row_cos(j / w)) } else { 0.0 };
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
                if ok && (geo.height[j] - hi).abs() < 1e-5 && best.is_none_or(|b| geo.terrain[j].is_water() && !geo.terrain[b.0].is_water()) {
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

/// 河流：依高度推下游格，每段以二次貝茲（經上下游中點、本格為控制點，等價 Chaikin）光柵化成
/// 中心為 1、邊緣為 0 的「帳篷」剖面，寫入 G 通道；半徑依等級，源頭收細。
fn paint_rivers(geo: &Geography, tw: usize, th: usize) -> Vec<f32> {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let mut out = vec![-RIVER_EM; tw * th];
    let radius = [0.0f32, 0.5, 0.75, 1.05]; // 半寬，紋素
    let is_river = |i: usize| geo.river[i] > 0 && !geo.terrain[i].is_water();
    let lvl = |i: usize| geo.river[i].min(3) as usize;
    // 8 鄰（含東西環繞）：回傳 (鄰格索引, 以本格為原點的格位移 dx, dy)
    let nbrs = |i: usize| {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let mut v = Vec::with_capacity(8);
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx_raw, ny) = (x + dx, y + dy);
                let nx = if geo.wrap_x { nx_raw.rem_euclid(w as i32) } else { nx_raw };
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                v.push((ny as usize * w + nx as usize, dx, dy));
            }
        }
        v
    };
    // 下游：與模擬的 D8 最陡下降一致（`geo.height` 是填窪後高度），具名幹流（Geography.rivers）以其格序為準。
    let down = river_flow(geo, &nbrs, &is_river);
    let mut ups: Vec<Vec<(usize, i32, i32)>> = vec![Vec::new(); w * h];
    for (i, d) in down.iter().enumerate() {
        if let Some((j, dx, dy)) = *d {
            if is_river(j) {
                ups[j].push((i, -dx, -dy));
            }
        }
    }
    let ss = SS as f32;
    let centre = |i: usize| (((i % w) as f32 + 0.5) * ss, ((i / w) as f32 + 0.5) * ss);
    let tw_i = tw as i32;
    let wrap = geo.wrap_x;
    // 以 (半徑 ra → rb) 的膠囊段光柵化；x 可超出貼圖，環繞時折回。
    let mut seg = |a: (f32, f32), b: (f32, f32), ra: f32, rb: f32| {
        let len2 = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).max(1e-6);
        let rmax = ra.max(rb) + RIVER_EM;
        let (minx, maxx) = ((a.0.min(b.0) - rmax).floor() as i32, (a.0.max(b.0) + rmax).ceil() as i32);
        let (miny, maxy) = ((a.1.min(b.1) - rmax).floor().max(0.0) as i32, (a.1.max(b.1) + rmax).ceil().min(th as f32 - 1.0) as i32);
        for y in miny..=maxy {
            for x in minx..=maxx {
                let xi = if wrap {
                    x.rem_euclid(tw_i)
                } else if x < 0 || x >= tw_i {
                    continue;
                } else {
                    x
                };
                let p = (x as f32 + 0.5, y as f32 + 0.5);
                let t = (((p.0 - a.0) * (b.0 - a.0) + (p.1 - a.1) * (b.1 - a.1)) / len2).clamp(0.0, 1.0);
                let d = ((p.0 - (a.0 + (b.0 - a.0) * t)).powi(2) + (p.1 - (a.1 + (b.1 - a.1) * t)).powi(2)).sqrt();
                let r = (ra + (rb - ra) * t).max(0.05);
                let v = (r - d).max(-RIVER_EM);
                let o = &mut out[y as usize * tw + xi as usize];
                if v > *o {
                    *o = v;
                }
            }
        }
    };
    let curve = |seg: &mut dyn FnMut((f32, f32), (f32, f32), f32, f32), p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), r0: f32, r2: f32| {
        const N: usize = 10;
        let at = |t: f32| {
            let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t);
            (a * p0.0 + b * p1.0 + c * p2.0, a * p0.1 + b * p1.1 + c * p2.1)
        };
        let mut prev = at(0.0);
        for k in 1..=N {
            let t = k as f32 / N as f32;
            let cur = at(t);
            let (ra, rb) = (r0 + (r2 - r0) * (k - 1) as f32 / N as f32, r0 + (r2 - r0) * t);
            seg(prev, cur, ra, rb);
            prev = cur;
        }
    };
    for i in 0..w * h {
        if !is_river(i) {
            continue;
        }
        let c = centre(i);
        let r_c = radius[lvl(i)];
        let off = |dx: i32, dy: i32, f: f32| (c.0 + dx as f32 * ss * f, c.1 + dy as f32 * ss * f);
        match down[i] {
            Some((j, dx, dy)) => {
                let end = off(dx, dy, 0.5);
                let r_end = if is_river(j) { 0.5 * (r_c + radius[lvl(j)]) } else { r_c };
                if ups[i].is_empty() {
                    curve(&mut seg, c, c, end, r_c * 0.6, r_end);
                } else {
                    for &(p, px, py) in &ups[i] {
                        let start = off(px, py, 0.5);
                        curve(&mut seg, start, c, end, 0.5 * (r_c + radius[lvl(p)]), r_end);
                    }
                }
            }
            None => {
                // 終點（窪地／出圖）：由各上游中點收束到本格中心
                for &(p, px, py) in &ups[i] {
                    seg(off(px, py, 0.5), c, 0.5 * (r_c + radius[lvl(p)]), r_c);
                }
                if ups[i].is_empty() {
                    seg(c, c, r_c, r_c);
                }
            }
        }
    }
    out
}

/// 建立世界貼圖。`terrain`：R = 保留、G = 河流剖面、B = 離岸距離、A = 陸/水有號場（0.5 為海岸線）。
pub fn build_world_images(geo: &Geography, provinces: &[Province], colors: &[[u8; 4]; 256]) -> WorldImages {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let (tw, th) = (w * SS, h * SS);

    // 海岸：陸／水有號場以二次 B 樣條取樣（模糊＋內插合一），等值線圓順且不吃掉單格湖。
    let signed_raw: Vec<f32> = geo.terrain.iter().map(|t| if t.is_water() { -1.0 } else { 1.0 }).collect();
    // 先與 3×3 二項式模糊各取一半：磨掉沿斜線的格尺度波紋（單格湖仍為水，只是縮小）。
    let signed: Vec<f32> = blur_binomial(&signed_raw, w, h, geo.wrap_x).iter().zip(&signed_raw).map(|(b, r)| 0.5 * (b + r)).collect();
    let (dist, nland) = land_distance(geo);
    let river = paint_rivers(geo, tw, th);
    // 海深（GDD 5.3「水深」）：正規化深度 (海平面 − 高度) / 海平面；湖以離岸距離（淺緣）。
    // 之後以二次 B 樣條取樣成 4× 貼圖，大陸棚、斜坡、海脊與海溝因此平滑而非階梯。
    let sea_level = geo.sea_level.max(1e-3);
    let depth_cell: Vec<f32> = (0..w * h)
        .map(|i| match geo.terrain[i] {
            siming_sim::Terrain::Sea => ((sea_level - geo.height[i]) / sea_level).clamp(0.0, 1.0),
            siming_sim::Terrain::Lake => 0.35 * (dist[i] / 4.0).clamp(0.0, 1.0),
            _ => 0.0,
        })
        .collect();
    // 先 3×3 二項式模糊：磨掉沿岸格尺度的扇貝狀等深線。
    let depth_cell = blur_binomial(&depth_cell, w, h, geo.wrap_x);
    let aura: Vec<f32> = geo.aura_cell.iter().map(|a| (a / 2.0).clamp(0.0, 1.0)).collect();

    let mut terrain = vec![0u8; tw * th * 4];
    let mut field = vec![0u8; tw * th];
    for ty in 0..th {
        for tx in 0..tw {
            let o = ty * tw + tx;
            // 紋素中心在格點座標（格中心為整數）的位置
            let (gx, gy) = ((tx as f32 + 0.5) / SS as f32 - 0.5, (ty as f32 + 0.5) / SS as f32 - 0.5);
            let f = bspline_sample(&signed, w, h, gx, gy, geo.wrap_x);
            let d = bspline_sample(&depth_cell, w, h, gx, gy, geo.wrap_x);
            // G：到河緣的有號距離（紋素；內正外負），夾在 ±RIVER_EM。
            terrain[o * 4 + 1] = ((0.5 + 0.5 * river[o] / RIVER_EM).clamp(0.0, 1.0) * 255.0).round() as u8;
            terrain[o * 4 + 2] = (d.clamp(0.0, 1.0) * 255.0).round() as u8;
            terrain[o * 4 + 3] = ((0.5 + 0.5 * f) * 255.0).round() as u8;
            field[o] = (bilinear(&aura, w, h, gx, gy, geo.wrap_x) * 255.0).round() as u8;
        }
    }
    let _ = provinces;

    // 地形類別權重（平原、丘陵、山脈、森林｜雪原、沙漠、沼澤）：遮罩 → 3×3 模糊 → RGBA8。
    let mut cls_a = vec![0u8; w * h * 4];
    let mut cls_b = vec![0u8; w * h * 4];
    for k in 2u8..=8 {
        let mask: Vec<f32> = nland.iter().map(|&c| if c == k { 1.0 } else { 0.0 }).collect();
        let blurred = blur_binomial(&mask, w, h, geo.wrap_x);
        let (buf, ch) = if k <= 5 { (&mut cls_a, (k - 2) as usize) } else { (&mut cls_b, (k - 6) as usize) };
        for i in 0..w * h {
            buf[i * 4 + ch] = (blurred[i] * 255.0).round() as u8;
        }
    }

    // 州郡 ID：格點解析度、向水域外擴 2 格（避免海岸處露出未著色細縫）。
    // 平滑邊界由著色器以 B 樣條投票逐像素求得（見 5.3「區域邊界平滑」），不在 CPU 光柵化。
    let pid: Vec<u8> = dilate_provinces(geo, 2).iter().flat_map(|p| p.to_le_bytes()).collect();

    let mk = |w: usize, h: usize, fmt, data| {
        let mut i = make_image(w, h, fmt, data, 1, ImageAddressMode::ClampToEdge);
        i.asset_usage = RenderAssetUsages::RENDER_WORLD;
        i
    };
    // 東西環繞的世界：u 方向重複取樣，使海岸、河流於接縫處連續。
    let mk_wrap = |w: usize, h: usize, fmt, data| {
        let mut i = mk(w, h, fmt, data);
        if geo.wrap_x {
            if let ImageSampler::Descriptor(d) = &mut i.sampler {
                d.address_mode_u = ImageAddressMode::Repeat;
            }
        }
        i
    };
    let mut color_bytes = Vec::with_capacity(256 * 4);
    for c in colors {
        color_bytes.extend_from_slice(c);
    }
    let mut pc = mk(256, 1, TextureFormat::Rgba8Unorm, color_bytes);
    // 查表以 textureLoad 取值，最近鄰
    pc.sampler = ImageSampler::nearest();
    let mut pid_img = mk(w, h, TextureFormat::R16Uint, pid);
    pid_img.sampler = ImageSampler::nearest();
    WorldImages {
        terrain: mk_wrap(tw, th, TextureFormat::Rgba8Unorm, terrain),
        province_id: pid_img,
        field: mk_wrap(tw, th, TextureFormat::R8Unorm, field),
        province_color: pc,
        cls_a: mk_wrap(w, h, TextureFormat::Rgba8Unorm, cls_a),
        cls_b: mk_wrap(w, h, TextureFormat::Rgba8Unorm, cls_b),
    }
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
        idx[id] = (0..24).map(|k| (start + k) % 24).find(|&c| !used[c]).unwrap_or(start);
    }
    idx
}

/// 依地圖模式組出 256 色查表（RGBA8 線性）。alpha = 該州是否上色。
pub fn province_color_table(
    mode: crate::state::MapMode,
    provinces: &[Province],
    assignment: &[usize],
    paper: Vec4,
) -> [[u8; 4]; 256] {
    let pal = super::palette::pastel24();
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut t = [[q(paper.x), q(paper.y), q(paper.z), 0u8]; 256];
    if matches!(mode, crate::state::MapMode::Tianxia) {
        for p in provinces {
            if (p.id as usize) < 255 && p.id != NO_PROVINCE {
                let c = pal[assignment[p.id as usize]];
                t[p.id as usize] = [q(c.x), q(c.y), q(c.z), 255];
            }
        }
    }
    t
}
