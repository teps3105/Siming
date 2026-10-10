//! GDD 2.2–2.4、7.3 世界生成的驗證（T5；星球版：等距圓柱投影、x 東西環繞、面積加權）。

use siming_sim::params::*;
use siming_sim::worldgen::{generate, Generated};
use siming_sim::{Era, Geography, Terrain, WorldParams, WorldSize, NO_CONTINENT, NO_PROVINCE};
use std::collections::BTreeSet;

fn gen(seed: u64) -> Generated {
    generate(&WorldParams::new(seed)).expect("worldgen")
}

fn dims(size: WorldSize) -> (usize, usize, u32) {
    match size {
        WorldSize::Small => (WG_SIZE_S_W, WG_SIZE_S_H, WG_PROVINCES_S),
        WorldSize::Medium => (WG_SIZE_M_W, WG_SIZE_M_H, WG_PROVINCES_M),
        WorldSize::Large => (WG_SIZE_L_W, WG_SIZE_L_H, WG_PROVINCES_L),
    }
}

/// 八鄰（x 東西環繞；GDD 2.2 尺寸）。
fn nb8(w: usize, h: usize, c: usize) -> Vec<usize> {
    let (x, y) = ((c % w) as i32, (c / w) as i32);
    let mut v = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let ny = y + dy;
            if ny >= 0 && ny < h as i32 {
                let nx = (x + dx).rem_euclid(w as i32);
                v.push(ny as usize * w + nx as usize);
            }
        }
    }
    v
}

/// 面積加權的陸地比例（陸地 = 非海格）。
fn land_ratio(geo: &Geography) -> f32 {
    let mut land = 0.0f32;
    let mut total = 0.0f32;
    for (c, &t) in geo.terrain.iter().enumerate() {
        let a = geo.area_weight(c as u32);
        total += a;
        if t != Terrain::Sea {
            land += a;
        }
    }
    land / total
}

/// `member` 的連通分量面積（面積加權；`eight` 為八鄰，否則四鄰）。
fn comp_areas_n(geo: &Geography, member: &[bool], eight: bool) -> Vec<f32> {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    for s in 0..w * h {
        if !member[s] || seen[s] {
            continue;
        }
        let mut stack = vec![s];
        seen[s] = true;
        let mut a = 0.0f32;
        while let Some(c) = stack.pop() {
            a += geo.area_weight(c as u32);
            for nb in nb8(w, h, c) {
                let diag = (nb % w != c % w) && (nb / w != c / w);
                if (!eight && diag) || !member[nb] || seen[nb] {
                    continue;
                }
                seen[nb] = true;
                stack.push(nb);
            }
        }
        out.push(a);
    }
    out
}

/// `member` 的八鄰連通分量面積（面積加權）。
fn comp_areas(geo: &Geography, member: &[bool]) -> Vec<f32> {
    comp_areas_n(geo, member, true)
}

/// 以 4 鄰（或 8 鄰）計算集合內的連通分量大小。
fn comp_sizes(w: usize, h: usize, member: &[bool], eight: bool) -> Vec<usize> {
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    for s in 0..w * h {
        if !member[s] || seen[s] {
            continue;
        }
        let mut stack = vec![s];
        seen[s] = true;
        let mut n = 0;
        while let Some(c) = stack.pop() {
            n += 1;
            for nb in nb8(w, h, c) {
                let diag = (nb % w != c % w) && (nb / w != c / w);
                if (!eight && diag) || !member[nb] || seen[nb] {
                    continue;
                }
                seen[nb] = true;
                stack.push(nb);
            }
        }
        out.push(n);
    }
    out
}

/// GDD 2.2 品質檢查：10 個種子全部通過（面積加權陸地比例、≥ 2 塊 ≥ 5% 的大洲、最大陸塊 ≤ 70%、州郡數與面積、江、洞天）。
#[test]
fn gdd_2_2_quality_checks_pass_for_10_seeds() {
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let (w, h, target) = dims(g.params.size);
        assert_eq!((geo.w as usize, geo.h as usize), (w, h));
        assert!(geo.wrap_x);
        assert!(
            (land_ratio(geo) - WG_LAND_RATIO).abs() <= 0.02,
            "seed {seed} 陸地比例 {}",
            land_ratio(geo)
        );
        let land: Vec<bool> = geo.terrain.iter().map(|&t| t != Terrain::Sea).collect();
        let areas = comp_areas(geo, &land);
        let land_area: f32 = areas.iter().sum();
        let biggest = areas.iter().copied().fold(0.0, f32::max);
        assert!(biggest <= 0.70 * land_area, "seed {seed} 最大陸塊 {}", biggest / land_area);
        let big = areas.iter().filter(|&&a| a >= 0.05 * land_area).count();
        assert!(big >= 2, "seed {seed} 大洲只有 {big} 塊");
        let cnt = g.provinces.len() as f32;
        assert!(
            (cnt - target as f32).abs() <= 0.15 * target as f32,
            "seed {seed} 州郡數 {cnt}"
        );
        for p in &g.provinces {
            let a: f32 = p.cells.iter().map(|&c| geo.area_weight(c)).sum();
            assert!(a >= 30.0, "seed {seed} 州郡 {} 面積 {a}", p.name);
        }
        assert!(geo.nodes.len() >= 8, "seed {seed} 洞天");
        assert!((0..=WG_RETRY).any(|a| siming_sim::worldgen::retry_seed(seed, a) == g.params.seed));
    }
}

/// GDD 6.5：同種子兩次生成，地理與州郡完全相同。
#[test]
fn gdd_6_5_same_seed_generates_identical_world() {
    let a = gen(42);
    let b = gen(42);
    assert_eq!(a.geo, b.geo);
    assert_eq!(a.provinces, b.provinces);
    assert_eq!(a.params, b.params);
    // 不同（實際使用的）種子，高度場不同；品質檢查重試可能使 42 與 43 落在同一種子，故往後找。
    let mut other = 43u64;
    let c = loop {
        let c = gen(other);
        if c.params.seed != a.params.seed {
            break c;
        }
        other += 1;
    };
    assert_ne!(a.geo.height, c.geo.height);
}

/// GDD 2.2 步驟 2：面積加權的陸地比例誤差 ≤ 0.02（含不同開局參數，4.6 範圍 0.25–0.55）。
#[test]
fn gdd_2_2_land_ratio_within_tolerance() {
    for &ratio in &[0.25f32, 0.40, 0.55] {
        let mut p = WorldParams::new(7);
        p.land_ratio = ratio;
        let g = generate(&p).expect("worldgen");
        assert!((land_ratio(&g.geo) - ratio).abs() <= 0.02, "ratio {ratio}");
        assert!(g.geo.sea_level > 0.0 && g.geo.sea_level < 1.0);
    }
}

/// GDD 2.2 尺寸：W = 2H；x 東西環繞——州郡陸鄰、河流、靈脈路徑都可跨越 x = 0 的接縫；緯度與面積權重正確。
#[test]
fn gdd_2_2_grid_wraps_east_west() {
    let mut seam_adjacency = 0;
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let (w, h) = (geo.w as usize, geo.h as usize);
        assert_eq!(w, 2 * h);
        assert!((geo.row_lat(0) - (std::f32::consts::FRAC_PI_2 - std::f32::consts::PI * 0.5 / h as f32)).abs() < 1e-5);
        assert!((geo.area_weight(0) - geo.row_lat(0).cos()).abs() < 1e-4);
        assert!((geo.area_weight((h / 2) as u32 * w as u32) - 1.0).abs() < 1e-3);
        // 接縫兩側同為陸地且分屬不同州郡者，必互為陸鄰。
        for y in 0..h {
            let a = y * w;
            let b = y * w + w - 1;
            let (pa, pb) = (geo.province_of[a], geo.province_of[b]);
            if pa != NO_PROVINCE && pb != NO_PROVINCE && pa != pb {
                assert!(g.provinces[pa as usize].neighbors.contains(&pb), "seed {seed} 接縫陸鄰");
                seam_adjacency += 1;
            }
        }
        // 幹流與靈脈路徑的相鄰步可跨接縫（nb8 含環繞）。
        for r in &geo.rivers {
            for pair in r.cells.windows(2) {
                assert!(nb8(w, h, pair[0] as usize).contains(&(pair[1] as usize)));
            }
        }
    }
    assert!(seam_adjacency > 0, "10 個種子中沒有任何跨接縫的州郡相鄰");
}

/// GDD 2.2 步驟 9 大洲、GDD 7.3 大洲命名：依面積降冪；≥ 5% 陸地者稱洲、較小者稱島；三教輪流；每個陸地格的歸屬一致；
/// 州郡不跨大洲（小島併入除外）。
#[test]
fn gdd_2_2_continents_named_by_three_teachings() {
    let ru = ["冀洲", "兗洲", "青洲", "徐洲", "揚洲", "荊洲", "豫洲", "梁洲", "雍洲", "神洲", "中洲"];
    let shi = ["東勝神洲", "西牛賀洲", "南贍部洲", "北俱蘆洲"];
    let dao = ["祖洲", "瀛洲", "玄洲", "炎洲", "長洲", "元洲", "流洲", "生洲", "鳳麟洲", "聚窟洲"];
    let isles = ["蓬萊島", "方丈島", "岱輿島", "員嶠島"];
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        assert!(geo.continents.len() >= 2);
        let land_area: f32 = (0..geo.len())
            .filter(|&c| geo.terrain[c] != Terrain::Sea)
            .map(|c| geo.area_weight(c as u32))
            .sum();
        let mut teaching: Vec<usize> = Vec::new();
        for (i, c) in geo.continents.iter().enumerate() {
            if i > 0 {
                assert!(c.area <= geo.continents[i - 1].area);
            }
            assert!(c.area >= 120.0);
            assert_eq!(geo.continent_of[c.centroid as usize], i as u16);
            if c.area >= 0.05 * land_area {
                assert!(c.name.ends_with('洲'), "{}", c.name);
            } else {
                assert!(c.name.ends_with('島'), "{}", c.name);
            }
            let t = if ru.contains(&c.name.as_str()) || ru.iter().any(|n| c.name == format!("{}島", n.chars().next().unwrap())) {
                0
            } else if shi.contains(&c.name.as_str()) || shi.iter().any(|n| c.name == format!("{}島", n.chars().next().unwrap())) {
                1
            } else if dao.contains(&c.name.as_str()) || isles.contains(&c.name.as_str()) || dao.iter().any(|n| c.name == format!("{}島", n.chars().next().unwrap())) {
                2
            } else {
                panic!("seed {seed} 大洲名 {} 不在三教字庫", c.name)
            };
            teaching.push(t);
        }
        // 前三塊輪流使用三教（字庫未用盡時）。
        if teaching.len() >= 3 {
            let set: BTreeSet<usize> = teaching[..3].iter().copied().collect();
            assert_eq!(set.len(), 3, "seed {seed} 前三塊 {teaching:?}");
        }
        let names: BTreeSet<&str> = geo.continents.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names.len(), geo.continents.len());
        let mut counted = vec![0u32; geo.continents.len()];
        for c in 0..geo.len() {
            let k = geo.continent_of[c];
            if geo.terrain[c] == Terrain::Sea {
                assert_eq!(k, NO_CONTINENT);
            } else if k != NO_CONTINENT {
                counted[k as usize] += 1;
            }
        }
        for (i, c) in geo.continents.iter().enumerate() {
            assert_eq!(counted[i], c.cells);
        }
        for p in &g.provinces {
            let (w, h) = (geo.w as usize, geo.h as usize);
            // 州郡的主大洲 = 面積最大者；其他大洲的格只能是併入的小島（四鄰連通、面積 < WG_ISLAND_MIN；
            // 州郡成長不越過兩側皆水的對角縫，故島嶼以四鄰計）。
            let mut by_cont: std::collections::BTreeMap<u16, f32> = Default::default();
            for &c in &p.cells {
                *by_cont.entry(geo.continent_of[c as usize]).or_default() += geo.area_weight(c);
            }
            // 主大洲 = 已命名大洲中面積最大者（併入的未命名小島不算；全是小島的州郡則為 NO_CONTINENT）。
            let main = by_cont
                .iter()
                .filter(|(&k, _)| k != NO_CONTINENT)
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(&k, _)| k)
                .unwrap_or(NO_CONTINENT);
            let mut other = vec![false; w * h];
            for &c in &p.cells {
                if geo.continent_of[c as usize] != main {
                    other[c as usize] = true;
                }
            }
            for a in comp_areas_n(geo, &other, false) {
                assert!(a < WG_ISLAND_MIN as f32, "seed {seed} 州郡 {} 跨大洲：主 {main} 分布 {by_cont:?} 其他分量面積 {a:.0}", p.name);
            }
        }
    }
}

/// GDD 2.4：洞天福地 ≥ 8，彼此距離 ≥ 18，強度分級 15/35/50，靈脈網連通且路徑連續。
#[test]
fn gdd_2_4_nodes_ley_and_aura() {
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let w = geo.w as usize;
        assert!(geo.nodes.len() >= 8);
        for (i, a) in geo.nodes.iter().enumerate() {
            assert!(matches!(
                geo.terrain[a.cell as usize],
                Terrain::Mountain | Terrain::Hills
            ));
            assert!([1.0, 0.75, 0.5].contains(&a.strength));
            for b in &geo.nodes[i + 1..] {
                let (ax, ay) = (a.cell as usize % w, a.cell as usize / w);
                let (bx, by) = (b.cell as usize % w, b.cell as usize / w);
                let d2 = (ax as i32 - bx as i32).pow(2) + (ay as i32 - by as i32).pow(2);
                assert!(d2 >= (WG_NODE_MIN_DIST * WG_NODE_MIN_DIST) as i32);
            }
        }
        let great = geo.nodes.iter().filter(|n| n.strength == 1.0).count();
        assert_eq!(great, (geo.nodes.len() as f32 * 0.15).round() as usize);
        // 靈脈：節點連通（MST）且多於 n−1 條邊（有環）。
        assert!(geo.ley.len() >= geo.nodes.len());
        let mut comp: Vec<usize> = (0..geo.nodes.len()).collect();
        for e in &geo.ley {
            let (ca, cb) = (comp[e.a as usize], comp[e.b as usize]);
            for c in comp.iter_mut() {
                if *c == cb {
                    *c = ca;
                }
            }
            assert_eq!(*e.path.first().unwrap(), geo.nodes[e.a as usize].cell);
            assert_eq!(*e.path.last().unwrap(), geo.nodes[e.b as usize].cell);
            for pair in e.path.windows(2) {
                assert!(nb8(w, geo.h as usize, pair[0] as usize).contains(&(pair[1] as usize)));
            }
            let sa = geo.nodes[e.a as usize].strength;
            let sb = geo.nodes[e.b as usize].strength;
            assert_eq!(e.strength, 0.5 * (sa + sb));
        }
        assert!(comp.iter().all(|&c| c == comp[0]), "靈脈網不連通");
        // 格點靈氣在 0..2；節點格的靈氣高於其地形基礎。
        assert!(geo
            .aura_cell
            .iter()
            .all(|&a| (0.0..=AURA_CELL_MAX).contains(&a)));
        for n in &geo.nodes {
            assert!(
                geo.aura_cell[n.cell as usize]
                    > AURA_TERRAIN[geo.terrain[n.cell as usize] as usize]
            );
        }
        for p in &g.provinces {
            assert!((0.0..=1.0).contains(&p.aura));
        }
    }
}

/// GDD 2.2 步驟 6：至少 3 條入海的江（等級 3）；幹流連續、終點鄰海。
#[test]
fn gdd_2_2_at_least_3_level3_rivers_reach_the_sea() {
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let (w, h) = (geo.w as usize, geo.h as usize);
        let mut jiang = 0;
        for r in &geo.rivers {
            assert!(!r.cells.is_empty());
            let last = *r.cells.last().unwrap() as usize;
            assert!(
                nb8(w, h, last)
                    .iter()
                    .any(|&nb| geo.terrain[nb] == Terrain::Sea),
                "seed {seed} {} 未入海",
                r.name
            );
            for pair in r.cells.windows(2) {
                assert!(nb8(w, h, pair[0] as usize).contains(&(pair[1] as usize)));
            }
            let lv = r
                .cells
                .iter()
                .map(|&c| geo.river[c as usize])
                .max()
                .unwrap();
            assert_eq!(lv, r.level);
            let suffix = match r.level {
                3 => "江",
                2 => "河",
                _ => "水",
            };
            assert!(r.name.ends_with(suffix), "{}", r.name);
            if r.level == 3 {
                jiang += 1;
            }
        }
        assert!(jiang >= 3, "seed {seed} 只有 {jiang} 條江");
        // 河流只在陸地格；湖格無河。
        for (i, &lv) in geo.river.iter().enumerate() {
            if lv > 0 {
                assert!(!geo.terrain[i].is_water());
            }
        }
    }
}

/// GDD 2.2 步驟 3：湖的面積 ≥ WG_LAKE_MIN。
#[test]
fn gdd_2_2_lakes_have_min_area() {
    for seed in 1..=5u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let lake: Vec<bool> = geo.terrain.iter().map(|&t| t == Terrain::Lake).collect();
        for s in comp_sizes(geo.w as usize, geo.h as usize, &lake, true) {
            assert!(s >= WG_LAKE_MIN as usize);
        }
    }
}

/// GDD 2.2 步驟 7：每個州郡四鄰連通（面積 < WG_ISLAND_MIN 的併入小島例外；面積為加權）。
#[test]
fn gdd_2_2_provinces_connected_except_islands() {
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let (w, h) = (geo.w as usize, geo.h as usize);
        for p in &g.provinces {
            let mut member = vec![false; w * h];
            for &c in &p.cells {
                member[c as usize] = true;
            }
            let sizes = comp_sizes(w, h, &member, false);
            assert_eq!(sizes.iter().sum::<usize>(), p.cells.len());
            // 四鄰分量的加權面積：最大者之外都須 < WG_ISLAND_MIN。
            let mut seen = vec![false; w * h];
            let mut areas: Vec<f32> = Vec::new();
            for s in 0..w * h {
                if !member[s] || seen[s] {
                    continue;
                }
                let mut stack = vec![s];
                seen[s] = true;
                let mut a = 0.0;
                while let Some(c) = stack.pop() {
                    a += geo.area_weight(c as u32);
                    for nb in nb8(w, h, c) {
                        let diag = (nb % w != c % w) && (nb / w != c / w);
                        if diag || !member[nb] || seen[nb] {
                            continue;
                        }
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
                areas.push(a);
            }
            let main = areas.iter().copied().fold(0.0, f32::max);
            for &a in &areas {
                assert!(
                    a == main || a < WG_ISLAND_MIN as f32,
                    "seed {seed} 州郡 {} 不連通",
                    p.name
                );
            }
        }
    }
}

/// GDD 2.3 欄位：ID 連續、cells 升冪、治所、鄰接對稱、地形統計、宜居度、人口、承載。
#[test]
fn gdd_2_3_province_fields_are_consistent() {
    for seed in 1..=5u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let (w, h) = (geo.w as usize, geo.h as usize);
        let mut covered = 0usize;
        for (i, p) in g.provinces.iter().enumerate() {
            assert_eq!(p.id as usize, i);
            assert!(p.cells.windows(2).all(|x| x[0] < x[1]));
            assert!(p.cells.contains(&p.seat));
            assert!(p.cells.iter().all(|&c| geo.province_of[c as usize] == p.id));
            assert!(p.cells.iter().all(|&c| !geo.terrain[c as usize].is_water()));
            assert_eq!(p.terrain_mix.iter().sum::<u32>() as usize, p.cells.len());
            assert!(p.neighbors.windows(2).all(|x| x[0] < x[1]));
            assert!(p.sea_neighbors.windows(2).all(|x| x[0] < x[1]));
            for &nb in &p.neighbors {
                assert!(g.provinces[nb as usize].neighbors.contains(&p.id));
                assert!(!p.sea_neighbors.contains(&nb));
            }
            for &nb in &p.sea_neighbors {
                assert!(g.provinces[nb as usize].sea_neighbors.contains(&p.id));
            }
            assert!(p.habitability > 0.0);
            let era = POP_INIT_ERA_MIDDLE;
            assert!((p.population - p.habitability * POP_INIT_PER_HAB * era).abs() < 1e-6);
            assert!((p.capacity - p.habitability * POP_CAP_PER_HAB).abs() < 1e-6);
            assert_eq!(p.faith, [0.0; 3]);
            assert_eq!(
                (p.karma_qi, p.merit, p.righteous, p.devastation),
                (0.0, 0.0, 0.0, 0.0)
            );
            // 治所是宜居度最高的格：至少不在水邊以外的低地形上（只檢查地形不為水）。
            assert!(!geo.terrain[p.seat as usize].is_water());
            covered += p.cells.len();
        }
        let land = geo.terrain.iter().filter(|t| !t.is_water()).count();
        assert_eq!(covered, land, "每個陸地格都屬於某州");
        for c in 0..w * h {
            if geo.terrain[c].is_water() {
                assert_eq!(geo.province_of[c], NO_PROVINCE);
            }
        }
        let _ = h;
    }
}

/// GDD 2.3：上古開局的 faith 初值落在規定範圍，其餘時代為 0；人口依時代係數。
#[test]
fn gdd_2_3_ancient_faith_and_era_population() {
    let mut p = WorldParams::new(5);
    p.era = Era::Ancient;
    let a = generate(&p).unwrap();
    for pr in &a.provinces {
        assert!((0.1..0.4).contains(&pr.faith[0]));
        assert!((0.0..0.15).contains(&pr.faith[1]));
        assert!((0.0..0.15).contains(&pr.faith[2]));
        assert!(
            (pr.population - pr.habitability * POP_INIT_PER_HAB * POP_INIT_ERA_ANCIENT).abs()
                < 1e-6
        );
    }
    p.era = Era::Decline;
    let d = generate(&p).unwrap();
    assert!(d.provinces.iter().all(|pr| pr.faith == [0.0; 3]));
    // 末法的格點靈氣倍率低於上古（同一地形／靈脈）。
    let sum = |g: &Generated| g.geo.aura_cell.iter().map(|&x| x as f64).sum::<f64>();
    assert!(sum(&d) < sum(&a));
}

/// GDD 7.3：州郡、江河、山脈、洞天全世界不重名；後綴符合規則。
#[test]
fn gdd_8_3_names_are_unique_with_correct_suffixes() {
    for seed in 1..=10u64 {
        let g = gen(seed);
        let mut all: Vec<&str> = Vec::new();
        for p in &g.provinces {
            assert!(
                p.name.ends_with('州') || p.name.ends_with('郡'),
                "{}",
                p.name
            );
            if p.city_level >= 3 {
                assert!(p.name.ends_with('州'));
            }
            all.push(&p.name);
        }
        for r in &g.geo.rivers {
            all.push(&r.name);
        }
        for r in &g.geo.ranges {
            assert!(["山", "嶺", "嶽"].iter().any(|s| r.name.ends_with(s)));
            all.push(&r.name);
        }
        for n in &g.geo.nodes {
            let ok = if n.strength == 1.0 {
                n.name.ends_with("大洞天")
            } else if n.strength == 0.75 {
                n.name.ends_with("洞天") && !n.name.ends_with("大洞天")
            } else {
                n.name.ends_with("福地")
            };
            assert!(ok, "{}", n.name);
            all.push(&n.name);
        }
        let set: BTreeSet<&str> = all.iter().copied().collect();
        assert_eq!(set.len(), all.len(), "seed {seed} 有重名");
        // 最高的山脈稱嶽，且只有它。
        if let Some(first) = g.geo.ranges.first() {
            assert!(first.name.ends_with('嶽'));
            assert!(g.geo.ranges[1..].iter().all(|r| !r.name.ends_with('嶽')));
            let top = first
                .cells
                .iter()
                .map(|&c| g.geo.height[c as usize])
                .fold(0.0f32, f32::max);
            assert_eq!(g.geo.height[first.peak as usize], top);
        }
        assert!(!g.params.name.is_empty());
    }
    // 自訂世界名保留；空白則自動命名。
    let mut p = WorldParams::new(3);
    p.name = "測試界".into();
    assert_eq!(generate(&p).unwrap().params.name, "測試界");
}

/// GDD 2.2 尺寸：小 256×128、中 384×192、大 512×256 都能生成，州郡數在目標 ±15% 內。
#[test]
fn gdd_2_2_all_sizes_generate() {
    for size in [WorldSize::Small, WorldSize::Medium, WorldSize::Large] {
        let (w, h, target) = dims(size);
        assert_eq!(w, 2 * h);
        let mut p = WorldParams::new(11);
        p.size = size;
        let g = generate(&p).unwrap();
        assert_eq!((g.geo.w as usize, g.geo.h as usize), (w, h));
        assert_eq!(g.geo.continent_of.len(), w * h);
        assert_eq!(g.geo.height.len(), w * h);
        assert_eq!(g.geo.aura_cell.len(), w * h);
        let c = g.provinces.len() as f32;
        assert!((c - target as f32).abs() <= 0.15 * target as f32);
    }
}

/// GDD 6.8：中尺寸世界生成 ≤ 1.5 秒（release；取三次最短）。
#[test]
fn gdd_6_8_medium_generation_within_budget() {
    let mut best = std::time::Duration::MAX;
    for seed in 1..=3u64 {
        let t = std::time::Instant::now();
        let _ = gen(seed);
        best = best.min(t.elapsed());
    }
    assert!(best.as_secs_f32() <= 1.5, "生成耗時 {best:?}");
}

/// 一個世界的地形組成統計（GDD 2.2 地形組成目標；面積加權）。
struct Composition {
    /// 各地形占陸地（非海格）面積的比例，索引為地形代碼。
    share: [f32; Terrain::COUNT],
    ranges: usize,
    /// 最大一條已命名山脈占山脈面積的比例。
    largest_range: f32,
    /// 大河與江（等級 ≥ 2）的格數。
    major_river_cells: usize,
    continents: usize,
    /// 面積 ≥ 陸地 5% 的大洲數，與最大陸塊占陸地的比例。
    big_continents: usize,
    largest_landmass: f32,
    /// 各大洲（≥ 5% 陸地）的海岸曲折度：海岸線長 ÷ 等面積圓周長。
    coast_ratio: Vec<f32>,
    /// 小島（八鄰連通、面積 < 陸地 1%）的數量。
    small_islands: usize,
    /// 森林、沙漠的生態區邊界方向比：南北走向的邊界長 ÷ 全部邊界長（條紋 → 0，片狀 → 約 0.5）。
    forest_ns: f32,
    desert_ns: f32,
}

/// 生態區 `t` 的邊界方向比（只算陸地格之間的邊界；邊長取真實長度：南北走向 1，東西走向 cos(lat)）。
fn biome_ns_ratio(geo: &Geography, t: Terrain) -> f32 {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let (mut ns, mut ew) = (0.0f32, 0.0f32);
    for y in 0..h {
        for x in 0..w {
            let c = y * w + x;
            let a = geo.terrain[c];
            if a == Terrain::Sea {
                continue;
            }
            let r = y * w + (x + 1) % w;
            let b = geo.terrain[r];
            if b != Terrain::Sea && ((a == t) != (b == t)) {
                ns += 1.0;
            }
            if y + 1 < h {
                let d = geo.terrain[c + w];
                if d != Terrain::Sea && ((a == t) != (d == t)) {
                    ew += geo.area_weight(c as u32);
                }
            }
        }
    }
    ns / (ns + ew).max(1e-6)
}

/// 大洲 `k` 的海岸曲折度：陸海邊界長（真實長度）÷ 與其等面積的圓的周長 `2·sqrt(π·A)`。
fn coast_ratio(geo: &Geography, k: u16) -> f32 {
    let (w, h) = (geo.w as usize, geo.h as usize);
    let mut len = 0.0f32;
    let mut area = 0.0f32;
    for y in 0..h {
        for x in 0..w {
            let c = y * w + x;
            if geo.continent_of[c] != k {
                continue;
            }
            area += geo.area_weight(c as u32);
            for dx in [w - 1, 1] {
                if geo.terrain[y * w + (x + dx) % w] == Terrain::Sea {
                    len += 1.0;
                }
            }
            if y > 0 && geo.terrain[c - w] == Terrain::Sea {
                len += geo.area_weight(c as u32);
            }
            if y + 1 < h && geo.terrain[c + w] == Terrain::Sea {
                len += geo.area_weight(c as u32);
            }
        }
    }
    len / (2.0 * (std::f32::consts::PI * area).sqrt())
}

fn composition(g: &Generated) -> Composition {
    let geo = &g.geo;
    let mut share = [0.0f32; Terrain::COUNT];
    let mut land_area = 0.0f32;
    for (c, &t) in geo.terrain.iter().enumerate() {
        if t != Terrain::Sea {
            let a = geo.area_weight(c as u32);
            share[t as usize] += a;
            land_area += a;
        }
    }
    for v in share.iter_mut() {
        *v /= land_area;
    }
    let mountain_area = share[Terrain::Mountain as usize] * land_area;
    let largest = geo
        .ranges
        .iter()
        .map(|r| r.cells.iter().map(|&c| geo.area_weight(c)).sum::<f32>())
        .fold(0.0f32, f32::max);
    let land: Vec<bool> = geo.terrain.iter().map(|&t| t != Terrain::Sea).collect();
    let masses = comp_areas(geo, &land);
    let big: Vec<u16> = geo
        .continents
        .iter()
        .enumerate()
        .filter(|(_, c)| c.area >= 0.05 * land_area)
        .map(|(i, _)| i as u16)
        .collect();
    Composition {
        share,
        ranges: geo.ranges.len(),
        largest_range: largest / mountain_area.max(1e-6),
        major_river_cells: geo.river.iter().filter(|&&l| l >= 2).count(),
        continents: geo.continents.len(),
        big_continents: big.len(),
        largest_landmass: masses.iter().copied().fold(0.0, f32::max) / land_area,
        coast_ratio: big.iter().map(|&k| coast_ratio(geo, k)).collect(),
        small_islands: masses.iter().filter(|&&a| a < 0.01 * land_area).count(),
        forest_ns: biome_ns_ratio(geo, Terrain::Forest),
        desert_ns: biome_ns_ratio(geo, Terrain::Desert),
    }
}

/// GDD 2.2 地形組成目標（決策 #31）：中尺寸、預設參數、20 個種子的平均（面積加權）落在目標區間，每個種子的地勢項 ≥ 下限的一半；
/// 大洲 ≥ 2 塊各 ≥ 5%、最大陸塊 ≤ 70%；山脈呈脈狀（≥ 3 條已命名山脈、最大一條 ≤ 山脈面積的 50%）；
/// 大河與江的格數 ≥ 1.5 × W；海岸曲折度 ≥ `COAST_RATIO_MIN`；小島 ≥ `SMALL_ISLANDS_MIN`；生態區邊界方向比 ≥ `BIOME_NS_MIN`。
#[test]
fn gdd_2_2_terrain_composition_targets() {
    // （地形, 下限, 上限）
    let targets: [(Terrain, f32, f32); 7] = [
        (Terrain::Plains, 0.25, 0.40),
        (Terrain::Forest, 0.15, 0.30),
        (Terrain::Hills, 0.10, 0.20),
        (Terrain::Mountain, 0.08, 0.15),
        (Terrain::Snow, 0.05, 0.15),
        (Terrain::Desert, 0.03, 0.12),
        (Terrain::Swamp, 0.01, 0.06),
    ];
    const SEEDS: u64 = 20;
    let comps: Vec<Composition> = (1..=SEEDS).map(|seed| composition(&gen(seed))).collect();
    let mut avg = [0.0f32; Terrain::COUNT];
    for (i, c) in comps.iter().enumerate() {
        println!(
            "seed {:2}: 平{:4.1} 林{:4.1} 丘{:4.1} 山{:4.1} 雪{:4.1} 沙{:4.1} 沼{:4.1} 湖{:4.1} | 山脈 {} 最大 {:.2} | 大河格 {} | 大洲 {}/{} 最大陸塊 {:.2} | 海岸 {:?} | 小島 {} | 林NS {:.2} 沙NS {:.2}",
            i + 1,
            c.share[2] * 100.0, c.share[5] * 100.0, c.share[3] * 100.0, c.share[4] * 100.0,
            c.share[6] * 100.0, c.share[7] * 100.0, c.share[8] * 100.0, c.share[1] * 100.0,
            c.ranges, c.largest_range, c.major_river_cells, c.big_continents, c.continents, c.largest_landmass,
            c.coast_ratio.iter().map(|r| (r * 100.0).round() / 100.0).collect::<Vec<_>>(),
            c.small_islands, c.forest_ns, c.desert_ns
        );
        for i in 0..Terrain::COUNT {
            avg[i] += c.share[i] / SEEDS as f32;
        }
    }
    for (i, c) in comps.iter().enumerate() {
        let seed = i + 1;
        // 每個種子的下限只管地勢項（平原、森林、丘陵、山脈）；雪原、沙漠、沼澤取決於陸地落在哪個氣候帶，只看平均。
        for (t, lo, _) in &targets[..4] {
            assert!(
                c.share[*t as usize] >= lo / 2.0,
                "seed {seed} {} 只有 {:.3}",
                t.name_zh(),
                c.share[*t as usize]
            );
        }
        assert!(c.ranges >= 3, "seed {seed} 山脈只有 {} 條", c.ranges);
        assert!(c.largest_range <= 0.5, "seed {seed} 最大山脈占 {:.2}", c.largest_range);
        assert!(
            c.major_river_cells >= 3 * WG_SIZE_M_W / 2,
            "seed {seed} 大河與江只有 {} 格",
            c.major_river_cells
        );
        assert!(c.big_continents >= 2, "seed {seed} ≥ 5% 的大洲只有 {}", c.big_continents);
        assert!(c.largest_landmass <= 0.70, "seed {seed} 最大陸塊占 {:.2}", c.largest_landmass);
        for (k, r) in c.coast_ratio.iter().enumerate() {
            assert!(*r >= COAST_RATIO_MIN, "seed {seed} 大洲 {k} 海岸曲折度 {r:.2}");
        }
        assert!(c.small_islands >= SMALL_ISLANDS_MIN, "seed {seed} 小島只有 {}", c.small_islands);
        assert!(c.forest_ns >= BIOME_NS_MIN, "seed {seed} 森林邊界方向比 {:.2}", c.forest_ns);
        assert!(c.desert_ns >= BIOME_NS_MIN, "seed {seed} 沙漠邊界方向比 {:.2}", c.desert_ns);
    }
    println!(
        "平均: 平{:4.1} 林{:4.1} 丘{:4.1} 山{:4.1} 雪{:4.1} 沙{:4.1} 沼{:4.1} 湖{:4.1}",
        avg[2] * 100.0, avg[5] * 100.0, avg[3] * 100.0, avg[4] * 100.0,
        avg[6] * 100.0, avg[7] * 100.0, avg[8] * 100.0, avg[1] * 100.0
    );
    for (t, lo, hi) in &targets {
        let v = avg[*t as usize];
        assert!(
            (*lo..=*hi).contains(&v),
            "{} 平均 {:.3} 不在 {lo}–{hi}",
            t.name_zh(),
            v
        );
    }
}

/// GDD 2.2 品質檢查：200 個種子（含大數種子）輪流以小／中／大尺寸生成，全部成功且高度場雜湊兩兩不同
/// （重試種子以 splitmix64 混合，不同輸入種子不會共用重試鏈）。
#[test]
fn gdd_2_2_many_seeds_generate_distinct_worlds() {
    fn fnv(h: &[f32]) -> u64 {
        let mut x = 0xcbf2_9ce4_8422_2325u64;
        for v in h {
            x ^= v.to_bits() as u64;
            x = x.wrapping_mul(0x0000_0100_0000_01b3);
        }
        x
    }
    let sizes = [WorldSize::Small, WorldSize::Medium, WorldSize::Large];
    let seeds: Vec<u64> = (1..=197u64).chain([1001, 7777, u64::MAX - 3]).collect();
    let mut hashes: Vec<u64> = Vec::new();
    for (i, &seed) in seeds.iter().enumerate() {
        let mut p = WorldParams::new(seed);
        p.size = sizes[i % 3];
        let g = generate(&p).unwrap_or_else(|e| panic!("seed {seed} 尺寸 {:?} 生成失敗：{e:?}", sizes[i % 3]));
        hashes.push(fnv(&g.geo.height));
    }
    let set: BTreeSet<u64> = hashes.iter().copied().collect();
    assert_eq!(set.len(), hashes.len(), "有種子產生相同的世界");
}

/// 細格八鄰（x 環繞）。
fn nb8_fine(w: usize, h: usize, c: usize) -> Vec<usize> {
    nb8(w, h, c)
}

/// GDD 2.2「對照成熟生成器的指標」：海岸線碎形維度（細格盒計數）。
fn coast_fractal_dimension(fine: &siming_sim::FineGeography) -> f32 {
    let (w, h) = (fine.w as usize, fine.h as usize);
    let is_sea = |i: usize| fine.terrain[i] == Terrain::Sea;
    let mut coast = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if is_sea(i) {
                continue;
            }
            if is_sea(y * w + (x + 1) % w) || is_sea(y * w + (x + w - 1) % w)
                || (y > 0 && is_sea(i - w)) || (y + 1 < h && is_sea(i + w))
            {
                coast[i] = true;
            }
        }
    }
    let mut xs: Vec<f32> = Vec::new();
    let mut ys: Vec<f32> = Vec::new();
    for &b in &[2usize, 4, 8, 16] {
        let (bw, bh) = (w.div_ceil(b), h.div_ceil(b));
        let mut boxes = vec![false; bw * bh];
        for y in 0..h {
            for x in 0..w {
                if coast[y * w + x] {
                    boxes[(y / b) * bw + x / b] = true;
                }
            }
        }
        let n = boxes.iter().filter(|&&v| v).count().max(1);
        xs.push((1.0 / b as f32).ln());
        ys.push((n as f32).ln());
    }
    let n = xs.len() as f32;
    let (mx, my) = (xs.iter().sum::<f32>() / n, ys.iter().sum::<f32>() / n);
    let num: f32 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let den: f32 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    num / den
}

/// 山脈伸長比：山脈格座標（x 相對首格展開環繞）的協方差主成分比 sqrt(λ₁/λ₂)。
fn range_elongation(geo: &Geography, cells: &[u32]) -> f32 {
    let w = geo.w as i32;
    let (x0, _) = geo.xy(cells[0]);
    let pts: Vec<(f32, f32)> = cells
        .iter()
        .map(|&c| {
            let (x, y) = geo.xy(c);
            let mut dx = x as i32 - x0 as i32;
            if dx > w / 2 {
                dx -= w;
            } else if dx < -w / 2 {
                dx += w;
            }
            let lat = geo.row_lat(y);
            (dx as f32 * lat.cos(), y as f32)
        })
        .collect();
    let n = pts.len() as f32;
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f32>() / n, pts.iter().map(|p| p.1).sum::<f32>() / n);
    let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in &pts {
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
        sxy += (x - mx) * (y - my);
    }
    let tr = sxx + syy;
    let det = sxx * syy - sxy * sxy;
    let disc = (tr * tr / 4.0 - det).max(0.0).sqrt();
    let (l1, l2) = (tr / 2.0 + disc, (tr / 2.0 - disc).max(1e-3));
    (l1 / l2).sqrt()
}

/// 細格流域樹的 Strahler 階（陸地格依高度最陡下降建樹），回傳每個細格的階（水格為 0）。
fn strahler(fine: &siming_sim::FineGeography) -> Vec<u8> {
    let (w, h) = (fine.w as usize, fine.h as usize);
    let n = w * h;
    // 全部陸地格的流域樹（河流格只是其中流量最大的一部分）。
    let river = |i: usize| !fine.terrain[i].is_water();
    let mut recv = vec![u32::MAX; n];
    let mut cells: Vec<usize> = (0..n).filter(|&i| river(i)).collect();
    for &i in &cells {
        let mut best = (fine.height[i], u32::MAX);
        for nb in nb8_fine(w, h, i) {
            if fine.height[nb] < best.0 && (river(nb) || fine.terrain[nb].is_water()) {
                best = (fine.height[nb], nb as u32);
            }
        }
        recv[i] = best.1;
    }
    // 由高到低處理：上游先算完。
    cells.sort_by(|&a, &b| fine.height[b].cmp(&fine.height[a]).then(a.cmp(&b)));
    let mut order = vec![0u8; n];
    let mut max_child = vec![0u8; n];
    let mut count_max = vec![0u8; n];
    for &c in &cells {
        let o = if max_child[c] == 0 {
            1
        } else if count_max[c] >= 2 {
            max_child[c] + 1
        } else {
            max_child[c]
        };
        order[c] = o;
        let r = recv[c];
        if r != u32::MAX && river(r as usize) {
            let r = r as usize;
            if o > max_child[r] {
                max_child[r] = o;
                count_max[r] = 1;
            } else if o == max_child[r] {
                count_max[r] = count_max[r].saturating_add(1);
            }
        }
    }
    order
}

/// GDD 2.2「對照成熟生成器的指標」（決策 #35）：海岸線碎形維度 1.05–1.45；島嶼面積層次；山脈伸長比與連續度；
/// 最大大洲河網 Strahler ≥ 4（其餘 ≥ 3）、江 ≥ 20 格；森林斑塊 ≥ 8、沙漠 ≥ 4% 時最大斑塊 ≤ 75%。10 個種子，每個種子皆須滿足。
#[test]
fn gdd_2_2_atlas_metrics_vs_mature_generators() {
    for seed in 1..=10u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let fine = &g.fine;
        assert_eq!(fine.scale as usize, WG_GEO_SCALE);
        assert_eq!((fine.w as usize, fine.h as usize), (geo.w as usize * WG_GEO_SCALE, geo.h as usize * WG_GEO_SCALE));
        let (w, h) = (geo.w as usize, geo.h as usize);
        // 海岸線碎形維度。
        let fd = coast_fractal_dimension(fine);
        // 島嶼面積層次。
        let land: Vec<bool> = geo.terrain.iter().map(|&t| t != Terrain::Sea).collect();
        let mut masses = comp_areas(geo, &land);
        masses.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let land_area: f32 = masses.iter().sum();
        let top = masses[0];
        let next3: f32 = masses.iter().skip(1).take(3).sum();
        let small = masses.iter().filter(|&&a| a < 0.01 * land_area).count();
        // 山系（八鄰連通的山脈格，≥ 12 格）的伸長比（面積加權平均、最大三個）；已命名山脈的連續度。
        let mountain: Vec<bool> = geo.terrain.iter().map(|&t| t == Terrain::Mountain).collect();
        let mut seen = vec![false; w * h];
        let mut systems: Vec<(f32, f32)> = Vec::new(); // (面積, 伸長比)
        for s0 in 0..w * h {
            if !mountain[s0] || seen[s0] {
                continue;
            }
            let mut stack = vec![s0];
            seen[s0] = true;
            let mut cells: Vec<u32> = Vec::new();
            while let Some(c) = stack.pop() {
                cells.push(c as u32);
                for nb in nb8(w, h, c) {
                    if mountain[nb] && !seen[nb] {
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
            }
            if cells.len() >= 12 {
                let a: f32 = cells.iter().map(|&c| geo.area_weight(c)).sum();
                systems.push((a, range_elongation(geo, &cells)));
            }
        }
        systems.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let elong_avg = systems.iter().map(|s| s.0 * s.1).sum::<f32>() / systems.iter().map(|s| s.0).sum::<f32>().max(1e-6);
        let top3: Vec<(f32, f32, usize)> = systems.iter().take(3).map(|s| (s.0, s.1, 1)).collect();
        let max_parts = geo
            .ranges
            .iter()
            .map(|r| {
                let mut member = vec![false; w * h];
                for &c in &r.cells {
                    member[c as usize] = true;
                }
                comp_sizes(w, h, &member, true).len()
            })
            .max()
            .unwrap_or(1);
        // 河網：每塊 ≥ 5% 陸地的大洲的最大 Strahler 階；江的長度。
        let order = strahler(fine);
        let big: Vec<u16> = geo
            .continents
            .iter()
            .enumerate()
            .filter(|(_, c)| c.area >= 0.05 * land_area)
            .map(|(i, _)| i as u16)
            .collect();
        let mut max_order = vec![0u8; geo.continents.len()];
        for (i, &o) in order.iter().enumerate() {
            if o > 0 {
                let (fx, fy) = (i % fine.w as usize, i / fine.w as usize);
                let c = (fy / WG_GEO_SCALE) * w + fx / WG_GEO_SCALE;
                let k = geo.continent_of[c];
                if k != NO_CONTINENT {
                    max_order[k as usize] = max_order[k as usize].max(o);
                }
            }
        }
        let min_jiang_len = geo.rivers.iter().filter(|r| r.level == 3).map(|r| r.cells.len()).min().unwrap_or(0);
        // 大陸棚比例（細格海面積中深度 < 0.055 者）。
        let (mut shelf_a, mut sea_a) = (0.0f32, 0.0f32);
        for i in 0..fine.len() {
            if fine.terrain[i] == Terrain::Sea {
                let y = i / fine.w as usize;
                let aw = (std::f32::consts::FRAC_PI_2 - std::f32::consts::PI * (y as f32 + 0.5) / fine.h as f32).cos();
                sea_a += aw;
                if fine.sea_level - fine.height_at(i) < 0.055 {
                    shelf_a += aw;
                }
            }
        }
        let shelf_frac = shelf_a / sea_a.max(1e-6);
        // 斑塊度：遊戲格陸地格中，八鄰（只算陸格）多數地形（≥ 5 格）與自身不同者的比例；氣候地形斑塊的平均大小。
        let mut salt = 0usize;
        let mut land_cells = 0usize;
        for c in 0..w * h {
            if geo.terrain[c].is_water() {
                continue;
            }
            land_cells += 1;
            let mut counts = [0u32; Terrain::COUNT];
            for nb in nb8(w, h, c) {
                if !geo.terrain[nb].is_water() {
                    counts[geo.terrain[nb] as usize] += 1;
                }
            }
            let (mut bt, mut bn) = (geo.terrain[c], 0u32);
            for t in Terrain::ALL {
                if counts[t as usize] > bn {
                    bn = counts[t as usize];
                    bt = t;
                }
            }
            if bn >= 5 && bt != geo.terrain[c] {
                salt += 1;
            }
        }
        let salt_frac = salt as f32 / land_cells.max(1) as f32;
        let climatic: Vec<bool> = geo
            .terrain
            .iter()
            .map(|&t| matches!(t, Terrain::Plains | Terrain::Forest | Terrain::Desert | Terrain::Swamp | Terrain::Snow | Terrain::Hills))
            .collect();
        let mut seen2 = vec![false; w * h];
        let mut patches = 0usize;
        let mut patch_cells = 0usize;
        for s0 in 0..w * h {
            if !climatic[s0] || seen2[s0] {
                continue;
            }
            patches += 1;
            let mut stack = vec![s0];
            seen2[s0] = true;
            while let Some(c) = stack.pop() {
                patch_cells += 1;
                for nb in nb8(w, h, c) {
                    if !seen2[nb] && geo.terrain[nb] == geo.terrain[s0] {
                        seen2[nb] = true;
                        stack.push(nb);
                    }
                }
            }
        }
        let mean_patch = patch_cells as f32 / patches.max(1) as f32;
        // 生態區斑塊。
        let forest: Vec<bool> = geo.terrain.iter().map(|&t| t == Terrain::Forest).collect();
        let forest_patches = comp_sizes(w, h, &forest, true).len();
        let desert: Vec<bool> = geo.terrain.iter().map(|&t| t == Terrain::Desert).collect();
        let desert_areas = comp_areas(geo, &desert);
        let desert_total: f32 = desert_areas.iter().sum();
        let desert_max = desert_areas.iter().copied().fold(0.0f32, f32::max);
        println!(
            "seed {seed:2}: 海岸維度 {fd:.2} | 陸塊 {} 次三/首 {:.2} 小島 {small} | 山系伸長 {elong_avg:.2} 前三 {:?} 命名山脈分量≤{max_parts} | Strahler {:?} 江最短 {min_jiang_len} | 林斑塊 {forest_patches} 沙最大 {:.2} | 棚 {shelf_frac:.3} 鹽椒 {salt_frac:.3} 斑塊均 {mean_patch:.1}",
            masses.len(),
            next3 / top,
            top3.iter().map(|t| (t.1 * 10.0).round() / 10.0).collect::<Vec<_>>(),
            big.iter().map(|&k| max_order[k as usize]).collect::<Vec<_>>(),
            desert_max / desert_total.max(1e-6)
        );
        assert!((1.05..=1.45).contains(&fd), "seed {seed} 海岸線碎形維度 {fd:.2}");
        assert!(masses.len() >= 12 && small >= 6, "seed {seed} 陸塊 {} 小島 {small}", masses.len());
        assert!(next3 / top >= 0.4, "seed {seed} 次三名陸塊之和 / 首名 {:.2}", next3 / top);
        assert!(small >= geo.continents.len().min(6), "seed {seed} 小島 {small} 少於大洲數");
        assert!(elong_avg >= 1.5, "seed {seed} 山系伸長比平均 {elong_avg:.2}");
        if let Some((a, e, _)) = top3.first() {
            assert!(*e >= 1.3, "seed {seed} 最大山系（面積 {a:.0}）伸長比 {e:.2}");
        }
        assert!(max_parts <= 3, "seed {seed} 已命名山脈最多分量 {max_parts}");
        for &k in &big {
            let need = if k == 0 { 4 } else { 3 };
            assert!(max_order[k as usize] >= need, "seed {seed} 大洲 {k} 最大 Strahler 階 {}", max_order[k as usize]);
        }
        assert!(min_jiang_len >= 20, "seed {seed} 江最短 {min_jiang_len} 格");
        assert!(forest_patches >= 8, "seed {seed} 森林斑塊 {forest_patches}");
        assert!((0.04..=0.25).contains(&shelf_frac), "seed {seed} 大陸棚占海 {shelf_frac:.3}");
        assert!(salt_frac <= 0.06, "seed {seed} 鹽椒格比例 {salt_frac:.3}");
        assert!(mean_patch >= 15.0, "seed {seed} 氣候地形斑塊平均 {mean_patch:.1} 格");
        if desert_total >= 0.04 * land_area {
            assert!(desert_max <= 0.75 * desert_total, "seed {seed} 最大沙漠斑塊占 {:.2}", desert_max / desert_total.max(1e-6));
        }
    }
}

/// GDD 2.2 尺寸／品質：極列無方位圖樣。噪聲、板塊、風場都以三維球面點取樣，極列的相鄰格在球面上極近，
/// 高度沿極列的相鄰差應遠小於赤道列（lon/lat 取樣或經度偏移的 warp 會在極點形成漩渦狀圖樣，相鄰差不會縮小）。
#[test]
fn gdd_2_2_polar_rows_have_no_azimuthal_pattern() {
    for seed in 1..=5u64 {
        let g = gen(seed);
        let geo = &g.geo;
        let (w, h) = (geo.w as usize, geo.h as usize);
        let row_diff = |y: usize| -> f32 {
            let mut s = 0.0f32;
            for x in 0..w {
                s += (geo.height[y * w + x] - geo.height[y * w + (x + 1) % w]).abs();
            }
            s / w as f32
        };
        let polar = row_diff(0).max(row_diff(h - 1));
        let mid = 0.5 * (row_diff(h / 2) + row_diff(h / 2 - 1));
        assert!(polar < 0.35 * mid, "seed {seed} 極列相鄰差 {polar:.4} 對赤道列 {mid:.4}");
    }
}

/// GDD 2.2 地形組成目標：海岸曲折度下限（海岸線長 ÷ 等面積圓周長）。
const COAST_RATIO_MIN: f32 = 1.4;
/// GDD 2.2 地形組成目標：每個世界的小島（面積 < 陸地 1%）最少數量。
const SMALL_ISLANDS_MIN: usize = 6;
/// GDD 2.2 地形組成目標：森林、沙漠邊界中南北走向的比例下限。
const BIOME_NS_MIN: f32 = 0.30;
