//! GDD 2.2 世界生成（T5 實作）：以等距圓柱投影描述整顆星球（x 東西環繞，y = 0 北極側）。
//!
//! 入口：[`generate`]。步驟 1–9 依序執行（噪聲地形 → 海平面 → 填窪與湖 → 氣候 → 地形 → 河流 → 州郡 → 靈脈 →
//! 命名與大洲），最後做品質檢查；失敗時以 `retry_seed`（splitmix64 混合）重試，最多 `WG_RETRY` 次（GDD 2.2）。
//!
//! 確定性（GDD 6.5）：只使用 `rng::stream(seed, Stream::Worldgen)`；不迭代 `HashMap`／`HashSet`；
//! 浮點只做基本運算與 `exp`、`sqrt`（三角函數以多項式自行實作，見 `grid`）。

#![allow(clippy::needless_range_loop)] // 格點平行陣列以索引走訪，較直觀

mod climate;
mod erosion;
mod grid;
mod height;
mod hydrology;
mod ley;
mod naming;
pub use naming::seat_stems;
mod waters;
pub use waters::{name_waters, NamedWater, WaterKind};
mod noise;
mod provinces;
pub(crate) mod quality;
mod terrain;

use crate::params::*;
use crate::rng::{stream, Stream};
use crate::types::{
    Continent, Era, FineGeography, Geography, NamedRange, NamedRiver, Province, Terrain,
    WorldGenError, WorldParams, WorldSize, NO_CONTINENT,
};
use grid::{components, uniform, Sphere};
use rand_chacha::ChaCha8Rng;

pub use grid::{col_lon, cos, row_lat, sin};

/// 世界生成的完整輸出。`params.seed` 為最終使用的種子。
pub struct Generated {
    pub params: WorldParams,
    pub geo: Geography,
    /// 地理解析度的細格資料（渲染用；決策 #35）。
    pub fine: FineGeography,
    pub provinces: Vec<Province>,
}

/// 命名之前的中間結果（品質檢查用）。
pub(crate) struct Draft {
    pub w: usize,
    pub h: usize,
    pub sp: Sphere,
    pub sea: f32,
    pub height: Vec<f32>,
    /// 山脈分段鍵（板塊對與沿邊界的段號；供命名時把長山脈分段）。
    pub range_key: Vec<u32>,
    pub terrain: Vec<Terrain>,
    pub rivers: hydrology::Rivers,
    pub provinces: provinces::Provinces,
    pub nodes: Vec<crate::types::LeyNode>,
    pub ley: Vec<crate::types::LeyEdge>,
    pub aura: Vec<f32>,
    /// 細格資料（決策 #35）。
    pub fine: FineGeography,
}

/// GDD 2.2 尺寸：(W, H, 目標州郡數, 板塊數, 侵蝕迭代數)。
fn dims(size: WorldSize) -> (usize, usize, u32, usize, u32) {
    match size {
        WorldSize::Small => (WG_SIZE_S_W, WG_SIZE_S_H, WG_PROVINCES_S, WG_PLATES_S, WG_EROSION_ITERS_S),
        WorldSize::Medium => (WG_SIZE_M_W, WG_SIZE_M_H, WG_PROVINCES_M, WG_PLATES_M, WG_EROSION_ITERS_M),
        WorldSize::Large => (WG_SIZE_L_W, WG_SIZE_L_H, WG_PROVINCES_L, WG_PLATES_L, WG_EROSION_ITERS_L),
    }
}

/// 細格（地理解析度）的中間結果，供降採樣與 `FineGeography`。
struct FineDraft {
    w: usize,
    h: usize,
    sea: f32,
    height: Vec<f32>,
    terrain: Vec<Terrain>,
    rivers: hydrology::Rivers,
    t: Vec<f32>,
    q: Vec<f32>,
    ridged: Vec<f32>,
    range_key: Vec<u32>,
}

/// 2×2（`scale²`）降採樣（決策 #35）：高度取平均；地形取多數（海／陸的平手依 (x + y) 奇偶決定，不偏向任一方）；
/// 河流等級取陸地細格的最大值；過小的粗格湖（面積 < `WG_LAKE_MIN`）改為鄰近的非水地形。
struct Coarse {
    height: Vec<f32>,
    terrain: Vec<Terrain>,
    river: Vec<u8>,
    ridged: Vec<f32>,
    range_key: Vec<u32>,
}

fn downsample(fd: &FineDraft, w: usize, h: usize, scale: usize) -> Coarse {
    let n = w * h;
    let mut height = vec![0.0f32; n];
    let mut terrain = vec![Terrain::Sea; n];
    let mut river = vec![0u8; n];
    let mut ridged = vec![0.0f32; n];
    let mut range_key = vec![0u32; n];
    let block = (scale * scale) as f32;
    for y in 0..h {
        for x in 0..w {
            let c = y * w + x;
            let mut counts = [0u32; Terrain::COUNT];
            let mut hsum = 0.0f32;
            let mut rsum = 0.0f32;
            let mut lv = 0u8;
            for dy in 0..scale {
                for dx in 0..scale {
                    let f = (y * scale + dy) * fd.w + x * scale + dx;
                    counts[fd.terrain[f] as usize] += 1;
                    hsum += fd.height[f];
                    rsum += fd.ridged[f];
                    if !fd.terrain[f].is_water() {
                        lv = lv.max(fd.rivers.level[f]);
                    }
                }
            }
            range_key[c] = fd.range_key[(y * scale) * fd.w + x * scale];
            let sea = counts[Terrain::Sea as usize];
            let half = (scale * scale / 2) as u32;
            let is_sea = sea > half || (sea == half && (x + y) % 2 == 0);
            let mut best = Terrain::Plains;
            let mut best_n = 0u32;
            for t in Terrain::ALL {
                if t != Terrain::Sea && counts[t as usize] > best_n {
                    best_n = counts[t as usize];
                    best = t;
                }
            }
            terrain[c] = if is_sea { Terrain::Sea } else { best };
            let mean = hsum / block;
            height[c] = if is_sea { mean.min(fd.sea - 1e-4) } else { mean.max(fd.sea) };
            river[c] = if terrain[c].is_water() { 0 } else { lv };
            ridged[c] = rsum / block;
        }
    }
    // 過小的粗格湖：改為格內最多的非水地形（沒有則平原），並確保高度不低於海平面。
    let lake: Vec<bool> = terrain.iter().map(|&t| t == Terrain::Lake).collect();
    let (label, sizes) = components(w, h, &lake, true);
    for c in 0..n {
        if lake[c] && sizes[label[c] as usize] < WG_LAKE_MIN {
            let (x, y) = (c % w, c / w);
            let mut counts = [0u32; Terrain::COUNT];
            for dy in 0..scale {
                for dx in 0..scale {
                    let f = (y * scale + dy) * fd.w + x * scale + dx;
                    counts[fd.terrain[f] as usize] += 1;
                }
            }
            let mut best = Terrain::Plains;
            let mut best_n = 0u32;
            for t in Terrain::ALL {
                if !t.is_water() && counts[t as usize] > best_n {
                    best_n = counts[t as usize];
                    best = t;
                }
            }
            terrain[c] = best;
        }
    }
    Coarse { height, terrain, river, ridged, range_key }
}

/// 細格幹流映射到粗格：連續重複合併（細格相鄰 → 粗格相同或八鄰），去掉尾端的水格；等級取粗格河流等級的最大值。
fn coarsen_stems(stems: &[hydrology::Stem], fw: usize, w: usize, scale: usize, terrain: &[Terrain], level: &[u8]) -> Vec<hydrology::Stem> {
    let mut out: Vec<hydrology::Stem> = Vec::new();
    for s in stems {
        let mut cells: Vec<u32> = Vec::new();
        for &f in &s.cells {
            let (fx, fy) = (f as usize % fw, f as usize / fw);
            let c = ((fy / scale) * w + fx / scale) as u32;
            if cells.last() != Some(&c) {
                cells.push(c);
            }
        }
        while cells.last().is_some_and(|&c| terrain[c as usize] == Terrain::Sea) {
            cells.pop();
        }
        let lv = cells.iter().map(|&c| level[c as usize]).max().unwrap_or(0);
        if lv == 0 || cells.len() < 2 || (lv == 3 && cells.len() < WG_JIANG_MIN_LEN) {
            continue;
        }
        // 粗格的出海口須鄰海（降採樣可能把河口併進陸格或湖格）。
        let last = *cells.last().unwrap() as usize;
        if !grid::nb8(w, terrain.len() / w, last).any(|(nb, _)| terrain[nb] == Terrain::Sea) {
            continue;
        }
        if lv >= 2 || cells.len() >= hydrology::RIVER_NAME_MIN_LEN {
            out.push(hydrology::Stem { level: lv, cells });
        }
    }
    out.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then(b.cells.len().cmp(&a.cells.len()))
            .then(a.cells.last().cmp(&b.cells.last()))
    });
    out
}

/// 量化細格資料為 `FineGeography`。
fn fine_geography(fd: &FineDraft, scale: usize) -> FineGeography {
    let n = fd.w * fd.h;
    let max_acc = fd.rivers.acc.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    let lmax = (1.0 + max_acc).ln();
    let q8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    FineGeography {
        scale: scale as u8,
        w: fd.w as u16,
        h: fd.h as u16,
        sea_level: fd.sea,
        height: fd.height.iter().map(|&v| (v.clamp(0.0, 1.0) * 65535.0).round() as u16).collect(),
        terrain: fd.terrain.clone(),
        river: (0..n).map(|c| if fd.terrain[c].is_water() { 0 } else { fd.rivers.level[c] }).collect(),
        flow: (0..n)
            .map(|c| {
                if fd.terrain[c].is_water() {
                    0
                } else {
                    (255.0 * (1.0 + fd.rivers.acc[c]).ln() / lmax).round() as u8
                }
            })
            .collect(),
        temperature: fd.t.iter().map(|&v| q8(v)).collect(),
        moisture: fd.q.iter().map(|&v| q8(v)).collect(),
    }
}

/// GDD 2.2 步驟 1–8（不含命名）。步驟 1–6 在細格（地理解析度，決策 #35）上進行，降採樣後做步驟 7–8。
fn build_draft(params: &WorldParams, seed: u64) -> (Draft, ChaCha8Rng, u32) {
    let (w, h, target, plates, iters) = dims(params.size);
    let scale = WG_GEO_SCALE;
    let (fw, fh) = (w * scale, h * scale);
    // 距離單位仍為遊戲格：細格球面的半徑以遊戲格寬計。
    let sp = Sphere::new(w, h);
    let fsp = Sphere::with_radius(fw, fh, sp.r_cells);
    let mut rng = stream(seed, Stream::Worldgen);
    let nz = noise::Noises::new(&mut rng, sp.r_cells);

    // 1–2. 板塊、大陸核、噪聲、隆起圖、海平面（細格）。
    let mut hf = height::generate(&fsp, params.relief, params.land_ratio, plates, &nz, &mut rng);
    // 1.7 河流侵蝕（Braun & Willett 2013）後重取海平面（陸地比例不變）。
    let farea: Vec<f32> = (0..fw * fh).map(|c| fsp.area(c)).collect();
    erosion::erode(&mut hf.h, &hf.uplift, &farea, fw, fh, hf.sea, iters);
    hf.sea = height::sea_level(&hf.h, &farea, params.land_ratio);
    // 1.8 海床：以最終海平面的陸地遮罩塑形海格（大陸棚、坡、深海平原、洋脊、海溝、島弧淺灘、封閉淺海）。
    height::seabed(&mut hf.h, hf.sea, &hf.seabed, &farea, fw, fh, scale);
    // 3. 填窪與湖。
    let lake_min = WG_LAKE_MIN * (scale * scale) as u32;
    let f = hydrology::fill_and_lakes(&hf.h, fw, fh, hf.sea, lake_min);
    // 4. 氣候。
    let cl = climate::compute(&fsp, &f.filled, hf.sea, params.humidity, scale, &nz);
    // 5. 地形。
    let mut fterrain = terrain::classify(&f.filled, hf.sea, &f.lake, &cl.t, &cl.q);
    terrain::despeckle(&mut fterrain, fw, fh, WG_BIOME_MIN_PATCH * (scale * scale) as u32, 2);
    // 6. 河流。
    let fsea: Vec<bool> = fterrain.iter().map(|&t| t == Terrain::Sea).collect();
    let frivers = hydrology::rivers(&f, &cl.q, &fsea, scale);
    let fd = FineDraft {
        w: fw,
        h: fh,
        sea: hf.sea,
        height: f.filled,
        terrain: fterrain,
        rivers: frivers,
        t: cl.t,
        q: cl.q,
        ridged: hf.ridged,
        range_key: hf.range_key,
    };
    // 降採樣到遊戲格。
    let Coarse { height, terrain, river: river_level, ridged, range_key } = downsample(&fd, w, h, scale);
    let stems = coarsen_stems(&fd.rivers.stems, fw, w, scale, &terrain, &river_level);
    let fine = fine_geography(&fd, scale);
    // 7. 州郡。
    let provs = provinces::build(&sp, &terrain, &river_level, target as usize, &mut rng);
    // 8. 靈脈。
    let nodes = ley::nodes(&sp, &terrain, &height, &ridged, target);
    let edges = ley::edges(&sp, &terrain, &nodes);
    let aura = ley::aura_cell(&sp, &terrain, &nodes, &edges, params.era);

    let draft = Draft {
        w,
        h,
        sp,
        sea: hf.sea,
        height,
        range_key,
        terrain,
        rivers: hydrology::Rivers { level: river_level, acc: Vec::new(), stems },
        provinces: provs,
        nodes,
        ley: edges,
        aura,
        fine,
    };
    (draft, rng, target)
}

/// GDD 2.3 城池等級（依人口；都城由模擬決定，生成時沒有）。
fn city_level(pop: f64) -> u8 {
    if pop >= CITY_LEVEL_STATE_POP {
        3
    } else if pop >= CITY_LEVEL_COUNTY_POP {
        2
    } else if pop >= CITY_LEVEL_TOWN_POP {
        1
    } else {
        0
    }
}

/// 山脈：分段鍵相同（所屬板塊、最近的鄰接板塊、沿邊界每 `WG_RANGE_SEGMENT` 格的段號）的山脈格的八鄰連通分量，
/// 格數 ≥ `WG_RANGE_MIN_CELLS`；很長的山系因此依板塊對、邊界兩側與段落分為數脈。最高者稱「嶽」。
/// 依最高格高度降冪排列。
fn find_ranges(d: &Draft, namer: &mut naming::Namer, rng: &mut ChaCha8Rng) -> Vec<NamedRange> {
    let n = d.w * d.h;
    let mountain: Vec<bool> = d.terrain.iter().map(|&t| t == Terrain::Mountain).collect();
    let mut label = vec![grid::NONE; n];
    let mut groups: Vec<Vec<u32>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for s in 0..n {
        if !mountain[s] || label[s] != grid::NONE {
            continue;
        }
        let id = groups.len() as u32;
        label[s] = id;
        stack.push(s);
        let mut g = Vec::new();
        while let Some(c) = stack.pop() {
            g.push(c as u32);
            for (nb, _) in grid::nb8(d.w, d.h, c) {
                if mountain[nb] && label[nb] == grid::NONE && d.range_key[nb] == d.range_key[s] {
                    label[nb] = id;
                    stack.push(nb);
                }
            }
        }
        groups.push(g);
    }
    let mut ranges: Vec<(u32, Vec<u32>)> = groups
        .into_iter()
        .filter(|g| g.len() >= WG_RANGE_MIN_CELLS)
        .map(|g| {
            let mut peak = g[0];
            for &c in &g {
                if d.height[c as usize] > d.height[peak as usize] {
                    peak = c;
                }
            }
            (peak, g)
        })
        .collect();
    ranges.sort_by(|a, b| {
        d.height[b.0 as usize]
            .partial_cmp(&d.height[a.0 as usize])
            .unwrap()
            .then(a.0.cmp(&b.0))
    });
    ranges
        .into_iter()
        .enumerate()
        .map(|(i, (peak, cells))| NamedRange {
            name: namer.range(rng, i == 0),
            peak,
            cells,
        })
        .collect()
}

/// GDD 2.2 步驟 9 大洲：八鄰連通的陸塊（非海格），面積 ≥ `WG_CONTINENT_MIN` 者依面積降冪命名（GDD 7.3 大洲命名）。
/// 回傳 (大洲表, 每格所屬大洲)。
fn find_continents(
    d: &Draft,
    namer: &mut naming::Namer,
    rng: &mut ChaCha8Rng,
) -> (Vec<Continent>, Vec<u16>) {
    let n = d.w * d.h;
    let land: Vec<bool> = d.terrain.iter().map(|&t| t != Terrain::Sea).collect();
    let (label, sizes) = components(d.w, d.h, &land, true);
    let mut area = vec![0.0f32; sizes.len()];
    let mut sum = vec![[0.0f32; 3]; sizes.len()];
    for c in 0..n {
        if land[c] {
            let l = label[c] as usize;
            let a = d.sp.area(c);
            let p = d.sp.unit(c);
            area[l] += a;
            sum[l][0] += a * p[0];
            sum[l][1] += a * p[1];
            sum[l][2] += a * p[2];
        }
    }
    let land_area: f32 = area.iter().sum();
    let mut order: Vec<usize> = (0..sizes.len())
        .filter(|&l| area[l] >= WG_CONTINENT_MIN)
        .collect();
    order.sort_by(|&a, &b| area[b].partial_cmp(&area[a]).unwrap().then(a.cmp(&b)));
    let mut rank = vec![NO_CONTINENT; sizes.len()];
    for (i, &l) in order.iter().enumerate() {
        rank[l] = i as u16;
    }
    // 中心：面積加權向量方向最接近的本陸塊陸地格。
    let mut centroid = vec![0u32; order.len()];
    let mut best = vec![f32::MIN; order.len()];
    for c in 0..n {
        if land[c] {
            let r = rank[label[c] as usize];
            if r != NO_CONTINENT {
                let l = label[c] as usize;
                let dotv = grid::dot(d.sp.unit(c), sum[l]);
                if dotv > best[r as usize] {
                    best[r as usize] = dotv;
                    centroid[r as usize] = c as u32;
                }
            }
        }
    }
    let mut st = namer.continents_start(rng);
    let continents: Vec<Continent> = order
        .iter()
        .enumerate()
        .map(|(i, &l)| Continent {
            name: namer.continent(rng, &mut st, i, area[l] >= WG_CONTINENT_ZHOU * land_area),
            cells: sizes[l],
            area: area[l],
            centroid: centroid[i],
        })
        .collect();
    let of: Vec<u16> = (0..n)
        .map(|c| if land[c] { rank[label[c] as usize] } else { NO_CONTINENT })
        .collect();
    (continents, of)
}

/// 命名、初始人口、州郡靈氣，組成最終輸出。
fn finalize(d: Draft, mut rng: ChaCha8Rng, params: &WorldParams, seed: u64) -> Generated {
    let (w, h) = (d.w, d.h);
    let era_factor = match params.era {
        Era::Ancient => POP_INIT_ERA_ANCIENT,
        Era::Middle => POP_INIT_ERA_MIDDLE,
        Era::Decline => POP_INIT_ERA_DECLINE,
    };

    // 州郡本體（GDD 2.3）。
    let mut provs: Vec<Province> = Vec::with_capacity(d.provinces.list.len());
    for (id, rp) in d.provinces.list.iter().enumerate() {
        let (mut sum, mut max) = (0.0f32, 0.0f32);
        for &c in &rp.cells {
            let a = d.aura[c as usize];
            sum += a;
            max = max.max(a);
        }
        let mean = sum / rp.cells.len() as f32;
        let aura = (AURA_PROVINCE_MEAN_WEIGHT * mean + AURA_PROVINCE_MAX_WEIGHT * max)
            .clamp(AURA_PROVINCE_CLAMP_MIN, AURA_PROVINCE_CLAMP_MAX);
        let population = rp.habitability * POP_INIT_PER_HAB * era_factor;
        let faith = if params.era == Era::Ancient {
            [
                uniform(
                    &mut rng,
                    FAITH_INIT_ANCIENT_TAO_MIN,
                    FAITH_INIT_ANCIENT_TAO_MAX,
                ),
                uniform(
                    &mut rng,
                    FAITH_INIT_ANCIENT_BUDDHA_MIN,
                    FAITH_INIT_ANCIENT_BUDDHA_MAX,
                ),
                uniform(
                    &mut rng,
                    FAITH_INIT_ANCIENT_RU_MIN,
                    FAITH_INIT_ANCIENT_RU_MAX,
                ),
            ]
        } else {
            [0.0; 3]
        };
        provs.push(Province {
            id: id as u16,
            name: String::new(),
            cells: rp.cells.clone(),
            seat: rp.seat,
            neighbors: rp.neighbors.clone(),
            sea_neighbors: rp.sea_neighbors.clone(),
            terrain_mix: rp.terrain_mix,
            habitability: rp.habitability,
            capacity: rp.habitability * POP_CAP_PER_HAB,
            population,
            aura,
            karma_qi: 0.0,
            merit: 0.0,
            righteous: 0.0,
            devastation: 0.0,
            faith,
            city_level: city_level(population),
        });
    }

    // 9. 命名（GDD 7.3）與大洲。
    let mut namer = naming::Namer::new();
    let (continents, continent_of) = find_continents(&d, &mut namer, &mut rng);
    let infos: Vec<naming::ProvinceInfo> = provs
        .iter()
        .zip(&d.provinces.list)
        .map(|(p, rp)| naming::ProvinceInfo {
            cells: p.cells.len(),
            seat: p.seat,
            center: match continent_of[p.seat as usize] {
                NO_CONTINENT => p.seat,
                k => continents[k as usize].centroid,
            },
            city_level: p.city_level,
            terrain_mix: p.terrain_mix,
            coast_cells: rp.coast_cells,
        })
        .collect();
    let names = namer.provinces(&mut rng, &infos, w, h);
    for (p, n) in provs.iter_mut().zip(names) {
        p.name = n;
    }
    let rivers: Vec<NamedRiver> = d
        .rivers
        .stems
        .iter()
        .map(|s| NamedRiver {
            name: namer.river(&mut rng, s.level),
            level: s.level,
            cells: s.cells.clone(),
        })
        .collect();
    let ranges = find_ranges(&d, &mut namer, &mut rng);
    let mut nodes = d.nodes.clone();
    for nd in nodes.iter_mut() {
        nd.name = namer.node(&mut rng, nd.strength);
    }
    let mut out_params = params.clone();
    out_params.seed = seed;
    if out_params.name.is_empty() {
        out_params.name = namer.world(&mut rng);
    }

    let geo = Geography {
        w: w as u16,
        h: h as u16,
        wrap_x: true,
        sea_level: d.sea,
        height: d.height,
        terrain: d.terrain,
        river: d.rivers.level,
        province_of: d.provinces.of,
        aura_cell: d.aura,
        nodes,
        ley: d.ley,
        rivers,
        ranges,
        continents,
        continent_of,
    };
    Generated {
        params: out_params,
        geo,
        fine: d.fine,
        provinces: provs,
    }
}

/// GDD 2.2 品質檢查的重試種子：第 0 次為原種子，之後為 `splitmix64(seed ⊕ attempt·φ)`——不同輸入種子的重試鏈
/// 不會重疊（`seed + 1` 會使相鄰種子落在同一個世界）。
pub fn retry_seed(seed: u64, attempt: u32) -> u64 {
    if attempt == 0 {
        return seed;
    }
    let mut z = seed ^ (attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// GDD 2.2：生成世界。品質檢查失敗時以 `retry_seed` 重試，最多 `WG_RETRY` 次；回傳的 `params.seed` 為實際使用的種子。
pub fn generate(params: &WorldParams) -> Result<Generated, WorldGenError> {
    generate_with(params, &mut |_, _| {})
}

/// 同 [`generate`]，但每次品質檢查失敗時呼叫 `diag(重試種子, 原因)`（供 siming-cli 的除錯輸出；模擬核心不讀環境變數，6.5）。
pub fn generate_with(
    params: &WorldParams,
    diag: &mut dyn FnMut(u64, &str),
) -> Result<Generated, WorldGenError> {
    let mut last_reason = String::new();
    let mut last_seed = params.seed;
    for attempt in 0..=WG_RETRY {
        let seed = retry_seed(params.seed, attempt);
        last_seed = seed;
        let (draft, rng, target) = build_draft(params, seed);
        match quality::check(&draft, &draft.sp, params.land_ratio, target) {
            Ok(()) => return Ok(finalize(draft, rng, params, seed)),
            Err(reason) => {
                diag(seed, &reason);
                last_reason = reason
            }
        }
    }
    Err(WorldGenError {
        last_seed,
        reason: last_reason,
    })
}
