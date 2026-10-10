//! GDD 2.2 步驟 1–2：板塊＋噪聲＋模板混合地形與海平面（決策 #32；分層管線見 7.4 §3.0）。
//!
//! - **L1 板塊**（Tectonics.js／WorldEngine 的結果形態；Civ 的分離保證）：球面上 `WG_PLATES` 個隨機點的 Voronoi
//!   （以 domain warp 扭曲後的座標判定，邊界曲折）；每板塊有隨機運動向量；隨機順序標為大陸板塊（先取互不相鄰者）
//!   直到面積足夠。邊界依相對運動分類：聚合→山脈帶／海岸山脈／島弧與海溝；張裂→裂谷／洋脊；轉形→輕微。
//! - **L2 大陸核**（Azgaar 的 Hill 模板）：每個大陸板塊內一個核（含 2–4 個葉）的高斯丘，乘板塊邊緣門（到板塊邊界
//!   的距離，即 Civ 的 rift／Azgaar 的 Strait），大陸不越過板塊邊界；再乘輪廓調變；極區的核振幅衰減（Azgaar 的邊緣遮罩）。
//! - **L3 造山帶**（mapgen4 的距離場）：到板塊聚合邊界的距離帶 × 脊狀多重碎形；板塊內另以低頻雜訊等值線帶作古老山系。
//! - **L4 細節**（fBm ＋ domain warp）：`base = 0.42·大陸層 + 0.58·細節 fBm + 島群 + 海洋邊界作用`。
//! - **L5 海床**：大陸棚／坡／深海平原、洋脊、海溝、島弧淺灘、封閉淺海（海平面以下有物理意義）。
//! - **L6 海平面**：面積加權分位數；最大陸塊過大時沿板塊邊界切海峽（7.4 §3.1 的補救），連通分量驗收在 `quality.rs`。

use super::grid::{
    below, chord, chord2, cos, dot, normalize, sin, stretch, uniform, Sphere, PI, TAU,
};
use super::noise::Noises;
use crate::params::*;
use rand_chacha::ChaCha8Rng;

/// 海床所需的每格邊界資料（在最終海平面定出後才塑形海床）。
pub struct SeabedInfo {
    /// 到最近板塊邊界的距離（遊戲格）。
    pub bdist: Vec<f32>,
    /// 邊界作用類別：0 無；1 海溝側（隱沒帶海洋側或島弧非仰衝側）；2 洋脊；3 島弧仰衝側。
    pub margin: Vec<u8>,
    /// 島弧斷續門（0..1）。
    pub arc_gate: Vec<f32>,
    /// 輪廓雜訊（0..1，棚寬）。
    pub shape: Vec<f32>,
    /// 細節雜訊（0..1，深海平原起伏）。
    pub detail: Vec<f32>,
}

pub struct HeightField {
    /// 0..1 的高度（填窪前）。
    pub h: Vec<f32>,
    /// 海床資料（`seabed` 用）。
    pub seabed: SeabedInfo,
    /// 脊狀雜訊（供靈脈「靈性」使用，GDD 2.4）。
    pub ridged: Vec<f32>,
    /// 隆起圖（0..1；陸地的地勢項，供侵蝕迭代使用，Cordonnier et al. 2016）。
    pub uplift: Vec<f32>,
    /// 山脈分段鍵：所屬板塊、最近的鄰接板塊與沿邊界每 `WG_RANGE_SEGMENT` 格的段號
    /// （同一邊界兩側分屬不同值；供山脈分段，GDD 2.2 步驟 9）。
    pub range_key: Vec<u32>,
    pub sea: f32,
}

/// 板塊邊界類型（依兩板塊的相對運動與性質）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boundary {
    /// 陸陸聚合：主山脈帶。
    Collision,
    /// 洋陸聚合：大陸側海岸山脈，海洋側海溝。
    Subduction,
    /// 洋洋聚合：島弧（仰衝側）與海溝。
    Arc,
    /// 陸陸張裂：裂谷。
    Rift,
    /// 洋洋張裂：洋脊。
    Ridge,
    /// 洋陸張裂：被動邊緣，無作用。
    Passive,
    /// 轉形：陸陸者為淺裂谷，其餘無作用。
    Transform,
}

pub struct Plate {
    pub center: [f32; 3],
    pub continental: bool,
    pub motion: [f32; 3],
    /// 面積（赤道格）。
    pub area: f32,
}

/// 大陸核的一個高斯丘：單位球上的中心、半徑（赤道格）與振幅（極區的核整體變弱，避免沿板塊邊界殘留環狀陸地）。
#[derive(Clone, Copy, Debug)]
pub struct Lobe {
    pub center: [f32; 3],
    pub radius: f32,
    pub amp: f32,
}

/// 極區衰減（Azgaar 的邊緣遮罩）：`1 − WG_POLAR_FADE_AMT × smoothstep(WG_POLAR_FADE_LAT, WG_POLAR_FADE_END, |lat|)`。
fn polar_fade(p: [f32; 3]) -> f32 {
    let lat = super::grid::asin_deg(p[2]);
    1.0 - WG_POLAR_FADE_AMT * smoothstep(WG_POLAR_FADE_LAT, WG_POLAR_FADE_END, lat.abs())
}

/// GDD 2.2 步驟 2：面積加權分位數，使陸地（`h >= sea`）面積比例等於 `land_ratio`。
pub fn sea_level(h: &[f32], area: &[f32], land_ratio: f32) -> f32 {
    let mut idx: Vec<u32> = (0..h.len() as u32).collect();
    idx.sort_by(|&a, &b| {
        h[b as usize]
            .partial_cmp(&h[a as usize])
            .unwrap()
            .then(a.cmp(&b))
    });
    let total: f32 = area.iter().sum();
    let want = land_ratio * total;
    let mut acc = 0.0f32;
    for &i in &idx {
        acc += area[i as usize];
        if acc >= want {
            return h[i as usize];
        }
    }
    h[*idx.last().unwrap() as usize]
}

/// 平滑階梯 `3t² − 2t³`，`t` 夾在 0..1。
#[inline]
pub fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 高斯帶形 `exp(−(d/w)²)`。
#[inline]
fn band(d: f32, w: f32) -> f32 {
    let t = d / w;
    (-t * t).exp()
}

/// 球面均勻隨機點。
fn random_unit(rng: &mut ChaCha8Rng) -> [f32; 3] {
    let z = uniform(rng, -1.0, 1.0);
    let phi = uniform(rng, 0.0, TAU);
    let r = (1.0 - z * z).max(0.0).sqrt();
    [r * cos(phi), r * sin(phi), z]
}

/// 自 `c` 沿切平面上的隨機方向移動 `dist` 格後的球面點。
fn offset_point(rng: &mut ChaCha8Rng, c: [f32; 3], dist: f32, r_cells: f32) -> [f32; 3] {
    let v = random_unit(rng);
    let k = dot(v, c);
    let t = normalize([v[0] - k * c[0], v[1] - k * c[1], v[2] - k * c[2]]);
    let s = dist / r_cells;
    normalize([c[0] + t[0] * s, c[1] + t[1] * s, c[2] + t[2] * s])
}

/// 兩板塊之間的邊界類型：相對運動 `v_i − v_j` 在連線方向 `n`（i → j）上的分量 > `WG_PLATE_CONV_FRAC` × 相對速率為聚合，
/// < −同值為張裂，其餘為轉形。
fn classify(a: &Plate, b: &Plate) -> Boundary {
    let n = normalize([
        b.center[0] - a.center[0],
        b.center[1] - a.center[1],
        b.center[2] - a.center[2],
    ]);
    let rel = [
        a.motion[0] - b.motion[0],
        a.motion[1] - b.motion[1],
        a.motion[2] - b.motion[2],
    ];
    let speed = dot(rel, rel).sqrt().max(1e-6);
    let conv = dot(rel, n) / speed;
    if conv > WG_PLATE_CONV_FRAC {
        match (a.continental, b.continental) {
            (true, true) => Boundary::Collision,
            (false, false) => Boundary::Arc,
            _ => Boundary::Subduction,
        }
    } else if conv < -WG_PLATE_CONV_FRAC {
        match (a.continental, b.continental) {
            (true, true) => Boundary::Rift,
            (false, false) => Boundary::Ridge,
            _ => Boundary::Passive,
        }
    } else {
        Boundary::Transform
    }
}

/// GDD 2.2 步驟 1.1（L1）：板塊。中心為彼此夾角 ≥ `WG_PLATE_SEP` 的隨機球面點（拒絕取樣，連續失敗 `WG_PLATE_TRIES`
/// 次後間距乘 `WG_PLATE_SEP_SHRINK`）；隨機順序逐一標為大陸板塊，直到大陸板塊面積 ≥ 陸地比例 × `WG_PLATE_CONT_OVER`：第一輪只取與
/// 已選者不相鄰的板塊，不足時第二輪每次取與已選者相鄰數最少者。回傳 (板塊, 每格所屬板塊, 每格扭曲後座標)。
pub fn make_plates(
    sp: &Sphere,
    nz: &Noises,
    count: usize,
    land_ratio: f32,
    rng: &mut ChaCha8Rng,
) -> (Vec<Plate>, Vec<u8>, Vec<[f32; 3]>) {
    let n = sp.w * sp.h;
    let mut sep = 2.0 * sin(0.5 * WG_PLATE_SEP * PI / 180.0);
    let mut centers: Vec<[f32; 3]> = Vec::with_capacity(count);
    let mut tries = 0u32;
    while centers.len() < count {
        let p = random_unit(rng);
        if centers.iter().all(|c| chord(*c, p) >= sep) {
            centers.push(p);
            tries = 0;
        } else {
            tries += 1;
            if tries >= WG_PLATE_TRIES {
                sep *= WG_PLATE_SEP_SHRINK;
                tries = 0;
            }
        }
    }
    let mut plates: Vec<Plate> = centers
        .into_iter()
        .map(|center| {
            // 運動向量：中心處切平面上的隨機方向 × 隨機速率。
            let east = normalize([-center[1], center[0], 0.0]);
            let north = normalize([
                -center[2] * center[0],
                -center[2] * center[1],
                center[0] * center[0] + center[1] * center[1],
            ]);
            let ang = uniform(rng, 0.0, TAU);
            let (ca, sa) = (cos(ang), sin(ang));
            let speed = uniform(rng, WG_PLATE_SPEED_MIN, WG_PLATE_SPEED_MAX);
            let motion = [
                speed * (ca * east[0] + sa * north[0]),
                speed * (ca * east[1] + sa * north[1]),
                speed * (ca * east[2] + sa * north[2]),
            ];
            Plate {
                center,
                continental: false,
                motion,
                area: 0.0,
            }
        })
        .collect();
    // 以扭曲後座標判定所屬板塊（邊界曲折）。
    let mut warped = vec![[0.0f32; 3]; n];
    let mut plate_of = vec![0u8; n];
    for c in 0..n {
        let pw = nz.warp(sp.unit(c));
        warped[c] = pw;
        let mut best = (f32::MAX, 0usize);
        for (k, p) in plates.iter().enumerate() {
            let d = chord2(pw, p.center);
            if d < best.0 {
                best = (d, k);
            }
        }
        plate_of[c] = best.1 as u8;
        plates[best.1].area += sp.area(c);
    }
    // 板塊相鄰表（共享邊界的格對；x 環繞）。
    let mut adj = vec![false; count * count];
    for c in 0..n {
        let (x, y) = (c % sp.w, c / sp.w);
        let right = y * sp.w + (x + 1) % sp.w;
        let a = plate_of[c] as usize;
        let b = plate_of[right] as usize;
        if a != b {
            adj[a * count + b] = true;
            adj[b * count + a] = true;
        }
        if y + 1 < sp.h {
            let b = plate_of[c + sp.w] as usize;
            if a != b {
                adj[a * count + b] = true;
                adj[b * count + a] = true;
            }
        }
    }
    // 大陸板塊：隨機順序累積面積；第一輪只取與已選大陸板塊不相鄰者（大洲彼此分離），不足時第二輪補足。
    let total: f32 = plates.iter().map(|p| p.area).sum();
    let mut order: Vec<usize> = (0..count).collect();
    for i in (1..count).rev() {
        let j = below(rng, i + 1);
        order.swap(i, j);
    }
    let want = land_ratio * WG_PLATE_CONT_OVER * total;
    let mut acc = 0.0f32;
    for &k in &order {
        if acc >= want {
            break;
        }
        let touches = (0..count).any(|m| plates[m].continental && adj[k * count + m]);
        if !touches {
            plates[k].continental = true;
            acc += plates[k].area;
        }
    }
    while acc < want {
        let mut best: Option<(usize, usize)> = None;
        for &k in &order {
            if plates[k].continental {
                continue;
            }
            let deg = (0..count)
                .filter(|&m| plates[m].continental && adj[k * count + m])
                .count();
            if best.is_none_or(|(_, d)| deg < d) {
                best = Some((k, deg));
            }
        }
        match best {
            Some((k, _)) => {
                plates[k].continental = true;
                acc += plates[k].area;
            }
            None => break,
        }
    }
    (plates, plate_of, warped)
}

/// GDD 2.2 步驟 1.2（L2）：大陸核（Azgaar 的 Hill 模板）。每個大陸板塊一個核：中心自板塊中心沿隨機切向偏移
/// `U(0, WG_CORE_OFFSET)·Rp`（`Rp = sqrt(板塊面積/π)`），半徑 `Rp × U(WG_CORE_RADIUS_MIN, MAX)`；
/// 另有 `WG_CORE_LOBES` 個葉（偏移 `WG_LOBE_OFFSET·R`、半徑 `WG_LOBE_RADIUS·R`）。
pub fn continent_cores(rng: &mut ChaCha8Rng, r_cells: f32, plates: &[Plate]) -> Vec<Lobe> {
    let mut lobes = Vec::new();
    for p in plates.iter().filter(|p| p.continental) {
        let rp = (p.area / PI).sqrt();
        let off = rp * uniform(rng, 0.0, WG_CORE_OFFSET);
        let center = offset_point(rng, p.center, off, r_cells);
        let radius = rp * uniform(rng, WG_CORE_RADIUS_MIN, WG_CORE_RADIUS_MAX);
        lobes.push(Lobe { center, radius, amp: polar_fade(center) });
        let k = WG_CORE_LOBES_MIN + below(rng, WG_CORE_LOBES_MAX - WG_CORE_LOBES_MIN + 1);
        for _ in 0..k {
            let off = radius * uniform(rng, WG_LOBE_OFFSET_MIN, WG_LOBE_OFFSET_MAX);
            let c2 = offset_point(rng, center, off, r_cells);
            let rr = radius * uniform(rng, WG_LOBE_RADIUS_MIN, WG_LOBE_RADIUS_MAX);
            lobes.push(Lobe { center: c2, radius: rr, amp: polar_fade(c2) });
        }
    }
    lobes
}

/// 大陸層：各丘 `exp(−(d/R)²)` 的最大值（d 為弦距，赤道格）。
#[inline]
fn continent_field(lobes: &[Lobe], p: [f32; 3], r_cells: f32) -> f32 {
    let mut best = 0.0f32;
    for l in lobes {
        let d2 = chord2(p, l.center) * r_cells * r_cells / (l.radius * l.radius);
        if d2 < 9.0 {
            best = best.max(l.amp * (-d2).exp());
        }
    }
    best
}

/// GDD 2.2 步驟 1–2：板塊＋噪聲＋模板混合地形與海平面。`relief` 為開局參數「山勢」（0–1，預設 0.5）。
pub fn generate(
    sp: &Sphere,
    relief: f32,
    land_ratio: f32,
    plate_count: usize,
    nz: &Noises,
    rng: &mut ChaCha8Rng,
) -> HeightField {
    let n = sp.w * sp.h;
    let area: Vec<f32> = (0..n).map(|c| sp.area(c)).collect();

    // L1 板塊與邊界類型。
    let (plates, plate_of, warped) = make_plates(sp, nz, plate_count, land_ratio, rng);
    let k = plates.len();
    let mut kind = vec![Boundary::Transform; k * k];
    for i in 0..k {
        for j in 0..k {
            if i != j {
                kind[i * k + j] = classify(&plates[i], &plates[j]);
            }
        }
    }
    // L2 大陸核。
    let lobes = continent_cores(rng, sp.r_cells, &plates);

    // L4 雜訊場（各自拉伸到 0..1）。
    let mut detail = vec![0.0f32; n];
    let mut shape = vec![0.0f32; n];
    let mut island = vec![0.0f32; n];
    let mut orogeny = vec![0.0f32; n];
    let mut ridged = vec![0.0f32; n];
    let mut hill_mask = vec![0.0f32; n];
    let mut hills = vec![0.0f32; n];
    for c in 0..n {
        let p = sp.unit(c);
        let pw = warped[c];
        detail[c] = nz.detail(pw);
        shape[c] = nz.shape(p);
        island[c] = nz.island(pw);
        orogeny[c] = nz.orogeny(pw);
        ridged[c] = nz.ridged(pw);
        hill_mask[c] = nz.hill_mask(p);
        hills[c] = nz.hills(pw);
    }
    stretch(&mut detail);
    stretch(&mut shape);
    stretch(&mut island);
    stretch(&mut orogeny);
    stretch(&mut ridged);
    stretch(&mut hill_mask);
    stretch(&mut hills);

    // 每格的邊界資料：到最近邊界的距離 b（與第二近板塊中心的中垂面距離，赤道格）、鄰接板塊 j、分段鍵。
    let mut bdist = vec![0.0f32; n];
    let mut other = vec![0u8; n];
    let mut range_key = vec![0u32; n];
    let seg = WG_RANGE_SEGMENT / sp.r_cells;
    for c in 0..n {
        let i = plate_of[c] as usize;
        let pw = warped[c];
        let di2 = chord2(pw, plates[i].center);
        let mut j = usize::MAX;
        let mut dj2 = f32::MAX;
        for (m, p) in plates.iter().enumerate() {
            if m != i {
                let d = chord2(pw, p.center);
                if d < dj2 {
                    dj2 = d;
                    j = m;
                }
            }
        }
        let (ci, cj) = (plates[i].center, plates[j].center);
        // 沿邊界的座標：邊界大圓的軸 n = ci × cj，dot(p, n) 沿邊界單調。
        let axis = normalize([
            ci[1] * cj[2] - ci[2] * cj[1],
            ci[2] * cj[0] - ci[0] * cj[2],
            ci[0] * cj[1] - ci[1] * cj[0],
        ]);
        let along = ((dot(pw, axis) + 1.0) / seg).floor().clamp(0.0, 255.0) as u32;
        range_key[c] = (((i * k + j) as u32) << 8) | along;
        let cc = chord(ci, cj).max(1e-6);
        bdist[c] = ((dj2 - di2) / (2.0 * cc) * sp.r_cells).max(0.0);
        other[c] = j as u8;
    }

    // L2＋L4 基底 = 大陸層（× 板塊邊緣門、輪廓調變）+ 細節 + 島群 + 海洋邊界作用（海溝、島弧、洋脊、裂谷）。
    let mut base = vec![0.0f32; n];
    for c in 0..n {
        let i = plate_of[c] as usize;
        let j = other[c] as usize;
        let b = bdist[c];
        // 板塊邊緣門：大陸不越過張裂、轉形、被動邊界（海峽）；碰撞縫合線只設半門（底值 `WG_COLLISION_GATE`：核強者相連成一洲，山脈帶在縫合線上；否則為窄海）；
        // 隱沒帶海岸同樣設門，海岸山脈帶則內移 `WG_BAND_COAST_INSET`（山脈在岸上而非海溝裡）。
        let edge = WG_PLATE_EDGE * (0.5 + shape[c]);
        let gate = if !plates[i].continental {
            0.0
        } else if kind[i * k + j] == Boundary::Collision {
            WG_COLLISION_GATE + (1.0 - WG_COLLISION_GATE) * smoothstep(0.0, edge, b)
        } else {
            smoothstep(0.0, edge, b)
        };
        let cont = continent_field(&lobes, warped[c], sp.r_cells)
            * gate
            * (WG_SHAPE_BASE + WG_SHAPE_SPAN * shape[c]);
        let ocean = match kind[i * k + j] {
            Boundary::Subduction if !plates[i].continental => -WG_TRENCH * band(b, WG_BAND_TRENCH),
            Boundary::Arc => {
                if i < j {
                    // 島弧沿邊界斷續（造山雜訊門），避免整條大圓的連續弧。
                    WG_ARC * band(b, WG_BAND_ARC) * ridged[c] * smoothstep(WG_ARC_ORO_LO, WG_ARC_ORO_HI, orogeny[c])
                } else {
                    -WG_TRENCH * band(b, WG_BAND_TRENCH)
                }
            }
            Boundary::Rift => -WG_RIFT * band(b, WG_BAND_RIFT),
            Boundary::Ridge => WG_OCEAN_RIDGE * band(b, WG_BAND_RIFT),
            Boundary::Transform if plates[i].continental && plates[j].continental => {
                -WG_RIFT * WG_TRANSFORM_RIFT * band(b, WG_BAND_RIFT)
            }
            _ => 0.0,
        };
        base[c] = WG_CONT_WEIGHT * cont
            + (1.0 - WG_CONT_WEIGHT) * detail[c]
            + WG_ISLAND_AMP * smoothstep(WG_ISLAND_LO, 1.0, island[c])
            + ocean;
    }
    let sea0 = sea_level(&base, &area, land_ratio);

    // L3 地勢：陸地高度 e（0..1）= 壓平的內陸 + 山脈 + max(山麓, 丘陵)；海格保留基底（海床起伏）。
    let mtn_amp = WG_MOUNTAIN_AMP * WG_RELIEF_SCALE * relief;
    let mut hh = vec![0.0f32; n];
    let mut uplift = vec![0.0f32; n];
    for c in 0..n {
        if base[c] < sea0 {
            hh[c] = base[c];
            continue;
        }
        let i = plate_of[c] as usize;
        let j = other[c] as usize;
        let b = bdist[c];
        let lb = (base[c] - sea0) / (1.0 - sea0);
        let lowland = WG_LOWLAND_RISE * lb;
        let inland = smoothstep(0.0, WG_INLAND_GATE, lb);
        // 板塊聚合帶（距離場）：陸陸碰撞為主山脈帶，洋陸隱沒的大陸側為海岸山脈帶；沿帶以造山雜訊門斷續（山脈分段、隘口）。
        let (amp, w, inset) = match kind[i * k + j] {
            Boundary::Collision => (WG_UPLIFT_COLLISION, WG_BAND_COLLISION, 0.0),
            Boundary::Subduction => (WG_UPLIFT_COAST, WG_BAND_COAST, WG_BAND_COAST_INSET),
            _ => (0.0, 1.0, 0.0),
        };
        let o = orogeny[c];
        let bb = (b - inset).abs();
        let g = band(bb, w);
        let plate_core = amp * g * smoothstep(WG_BELT_ORO_LO, WG_BELT_ORO_HI, o);
        let plate_foot = amp * band(bb, WG_FOOTHILL_BAND * w) * (1.0 - g);
        // 板塊內造山帶：造山雜訊兩條等值線附近的曲折長帶（古老山系）。
        let dist = (o - WG_OROGENY_ISO_A).abs().min((o - WG_OROGENY_ISO_B).abs());
        let belt = (1.0 - dist / WG_OROGENY_HALF).max(0.0);
        let bc = smoothstep(WG_OROGENY_CORE, 1.0, belt);
        let noise_core = WG_INTRAPLATE * bc;
        let noise_foot = WG_INTRAPLATE * smoothstep(0.0, WG_OROGENY_CORE, belt) * (1.0 - bc);
        let core = plate_core.max(noise_core);
        let r = ridged[c];
        let mountain = mtn_amp * core * inland * r * r;
        let foot = WG_FOOTHILL_AMP * plate_foot.max(noise_foot) * inland * hills[c];
        let hm = smoothstep(WG_HILL_MASK_LO, WG_HILL_MASK_HI, hill_mask[c]);
        let hill = WG_HILL_AMP * hm * inland * hills[c];
        let e = (lowland + mountain + foot.max(hill)).clamp(0.0, 1.0);
        hh[c] = sea0 + (1.0 - sea0) * e;
        uplift[c] = e;
    }

    // L6 海平面：面積加權分位數（陸格只升不降，故與 sea₀ 相同）。
    let mut sea = sea_level(&hh, &area, land_ratio);

    // L6 補救（7.4 §3.1；Civ 的 rift／Azgaar 的 Strait）：最大陸塊占陸地 > WG_STRAIT_MAX_SHARE 時，
    // 在該陸塊內「陸地格最多」的板塊邊界上切一道海峽（沿邊界帶下陷），重取海平面；最多 WG_STRAIT_TRIES 次。
    for _ in 0..WG_STRAIT_TRIES {
        let land: Vec<bool> = hh.iter().map(|&v| v >= sea).collect();
        let (label, sizes) = super::grid::components(sp.w, sp.h, &land, true);
        let mut comp_area = vec![0.0f32; sizes.len()];
        for c in 0..n {
            if land[c] {
                comp_area[label[c] as usize] += area[c];
            }
        }
        let land_area: f32 = comp_area.iter().sum();
        let mut biggest = 0usize;
        for (i, &a) in comp_area.iter().enumerate() {
            if a > comp_area[biggest] {
                biggest = i;
            }
        }
        if land_area <= 0.0 || comp_area[biggest] <= WG_STRAIT_MAX_SHARE * land_area {
            break;
        }
        // 候選邊界：無序板塊對 {i, j}，計最大陸塊內 b < 帶寬 的陸地格數。
        let mut count = vec![0u32; k * k];
        for c in 0..n {
            if land[c] && label[c] as usize == biggest && bdist[c] < WG_STRAIT_W {
                let (i, j) = (plate_of[c] as usize, other[c] as usize);
                let (a, b2) = if i < j { (i, j) } else { (j, i) };
                count[a * k + b2] += 1;
            }
        }
        let mut best = (0u32, 0usize);
        for (idx, &v) in count.iter().enumerate() {
            if v > best.0 {
                best = (v, idx);
            }
        }
        if best.0 == 0 {
            break;
        }
        let (pi, pj) = (best.1 / k, best.1 % k);
        for c in 0..n {
            let (i, j) = (plate_of[c] as usize, other[c] as usize);
            if (i == pi && j == pj) || (i == pj && j == pi) {
                hh[c] = (hh[c] - WG_STRAIT_DEPTH * band(bdist[c], WG_STRAIT_W)).max(0.0);
            }
        }
        sea = sea_level(&hh, &area, land_ratio);
    }

    // L5 海床資料：在侵蝕與最終海平面之後由 `seabed` 塑形（陸地遮罩須用最終海平面）。
    let mut margin = vec![0u8; n];
    let mut arc_gate = vec![0.0f32; n];
    for c in 0..n {
        let i = plate_of[c] as usize;
        let j = other[c] as usize;
        let kd = kind[i * k + j];
        let trench_side = (kd == Boundary::Subduction && !plates[i].continental) || (kd == Boundary::Arc && i > j);
        margin[c] = if trench_side {
            1
        } else if kd == Boundary::Ridge {
            2
        } else if kd == Boundary::Arc {
            3
        } else {
            0
        };
        if margin[c] == 3 {
            arc_gate[c] = smoothstep(WG_ARC_ORO_LO, WG_ARC_ORO_HI, orogeny[c]);
        }
    }
    let seabed_info = SeabedInfo { bdist, margin, arc_gate, shape, detail };

    HeightField {
        h: hh,
        seabed: seabed_info,
        ridged,
        uplift,
        range_key,
        sea,
    }
}

/// GDD 2.2 步驟 1 海床（L5）：以最終海平面的陸地遮罩塑形海格高度——大陸棚（被動邊緣寬、海溝側窄；小島的棚更窄）
/// → 大陸坡 → 深海平原（低幅雜訊）；洋脊在張裂邊界隆起、海溝在隱沒帶加深、島弧仰衝側為淺灘；小面積的封閉海為淺海。
/// 海格高度 = 海平面 − 深度；陸格不動。`scale` 為地理解析度（距離換算）。
pub fn seabed(hh: &mut [f32], sea: f32, info: &SeabedInfo, area: &[f32], w: usize, h: usize, scale: usize) {
    let n = w * h;
    let land: Vec<bool> = hh.iter().map(|&v| v >= sea).collect();
    let (dland, src) = land_distance(w, h, &land);
    let (llabel, _) = super::grid::components(w, h, &land, true);
    let mut land_area: Vec<f32> = Vec::new();
    for c in 0..n {
        if land[c] {
            let l = llabel[c] as usize;
            if land_area.len() <= l {
                land_area.resize(l + 1, 0.0);
            }
            land_area[l] += area[c];
        }
    }
    let sea_mask: Vec<bool> = land.iter().map(|&l| !l).collect();
    let (slabel, _) = super::grid::components(w, h, &sea_mask, true);
    let mut sea_area: Vec<f32> = Vec::new();
    for c in 0..n {
        if !land[c] {
            let l = slabel[c] as usize;
            if sea_area.len() <= l {
                sea_area.resize(l + 1, 0.0);
            }
            sea_area[l] += area[c];
        }
    }
    let k = scale as f32;
    let cell_area = k * k; // 細格面積換算成遊戲格
    for c in 0..n {
        if land[c] {
            continue;
        }
        let b = info.bdist[c];
        let d = dland[c] / k;
        // 棚寬：被動邊緣 8 × (0.5 + shape)；海溝側 × 0.3；小陸塊（< 300 格）按面積縮窄。
        let mut shelf_w = WG_SHELF_W * (0.5 + info.shape[c]);
        if info.margin[c] == 1 {
            shelf_w *= WG_SHELF_SUBDUCTION;
        }
        if src[c] != u32::MAX {
            let a = land_area[llabel[src[c] as usize] as usize] / cell_area;
            shelf_w *= (a / WG_SHELF_FULL_AREA).sqrt().clamp(0.25, 1.0);
        }
        let deep = smoothstep(shelf_w, shelf_w + WG_SLOPE_W, d);
        let mut depth = if d <= shelf_w {
            WG_SHELF_DEPTH + (WG_SHELF_EDGE_DEPTH - WG_SHELF_DEPTH) * (d / shelf_w.max(1e-3))
        } else {
            WG_SHELF_EDGE_DEPTH + (WG_ABYSS_DEPTH - WG_SHELF_EDGE_DEPTH) * deep
                + WG_ABYSS_NOISE * (2.0 * info.detail[c] - 1.0) * deep
        };
        match info.margin[c] {
            1 => depth += WG_TRENCH_DEEPEN * band(b, WG_BAND_ARC) * deep,
            2 => depth -= WG_RIDGE_RISE * band(b, WG_BAND_RIFT) * deep,
            3 => {
                let bank = band(b, WG_BAND_ARC) * info.arc_gate[c];
                if bank > 0.3 {
                    depth = depth.min(WG_ARC_BANK_DEPTH);
                }
            }
            _ => {}
        }
        if sea_area[slabel[c] as usize] / cell_area < WG_ENCLOSED_SEA_AREA {
            depth = depth.min(WG_ENCLOSED_SEA_DEPTH);
        }
        hh[c] = (sea - depth.max(WG_SHELF_DEPTH * 0.5)).max(0.0);
    }
}

/// 海格到最近陸格的球面距離（Dijkstra，八鄰、步長 `step_len`；x 環繞；陸格為 0；單位為細格）與最近的陸格。
fn land_distance(w: usize, h: usize, land: &[bool]) -> (Vec<f32>, Vec<u32>) {
    use super::grid::key;
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let n = w * h;
    let mut d = vec![f32::MAX; n];
    let mut src = vec![u32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    for c in 0..n {
        if land[c] {
            d[c] = 0.0;
            src[c] = c as u32;
            heap.push(Reverse((0, c as u32)));
        }
    }
    while let Some(Reverse((k, c))) = heap.pop() {
        let c = c as usize;
        if k > key(d[c]) {
            continue;
        }
        for (nb, _) in super::grid::nb8(w, h, c) {
            let nd = d[c] + super::hydrology::step_len(w, h, c, nb);
            if nd < d[nb] {
                d[nb] = nd;
                src[nb] = src[c];
                heap.push(Reverse((key(nd), nb as u32)));
            }
        }
    }
    for v in d.iter_mut() {
        if *v == f32::MAX {
            *v = 0.0;
        }
    }
    (d, src)
}
