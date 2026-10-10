//! 開發用的合成世界（`SIMING_DEV_WORLD=1`）：在 T5 世界生成完成前，供渲染與介面開發測試。
//! 不是遊戲內容，M1 完成後可移除。

use siming_sim::{Geography, Province, Sim, Terrain, WorldParams, NO_PROVINCE};

pub fn synthetic_sim(seed: u64) -> Sim {
    // GDD 2.2：2:1 等距圓柱、東西環繞。主陸塊跨過 x = 0 接縫，另有一座島在對面，便於檢查無縫。
    let (w, h) = (256u16, 128u16);
    let n = w as usize * h as usize;
    let sea = 0.35f32;
    let mut height = vec![0f32; n];
    let mut terrain = vec![Terrain::Sea; n];
    let mut river = vec![0u8; n];
    let mut aura_cell = vec![0.05f32; n];
    let s = (seed % 97) as f32 * 0.1;
    for y in 0..h {
        for x in 0..w {
            let i = y as usize * w as usize + x as usize;
            let (fx0, fy) = (x as f32 / w as f32, y as f32 / h as f32 - 0.5);
            // 以接縫（x = 0）為中心的環狀經度差，-0.5..0.5
            let fx = if fx0 > 0.5 { fx0 - 1.0 } else { fx0 };
            let isle = {
                let (dx, dy) = (fx0 - 0.5, fy + 0.1);
                (dx * dx * 6.0 + dy * dy * 14.0).sqrt()
            };
            let d = (fx * fx * 2.4 + fy * fy * 2.6).sqrt();
            let wave = (fx * 9.0 + s).sin() * (fy * 7.0 - s).cos() * 0.12 + (fx * 23.0).sin() * (fy * 19.0).sin() * 0.05;
            let v = (0.75 - d * 1.3 + wave).max(0.62 - isle * 1.6 + wave * 0.5).clamp(0.0, 1.0);
            height[i] = v;
            let land = (v - sea) / (1.0 - sea);
            terrain[i] = if v < sea {
                Terrain::Sea
            } else if land > 0.62 {
                Terrain::Mountain
            } else if fy < -0.38 {
                Terrain::Snow
            } else if land > 0.38 {
                Terrain::Hills
            } else if fx > 0.3 {
                Terrain::Desert
            } else if fx < -0.25 {
                Terrain::Forest
            } else {
                Terrain::Plains
            };
            aura_cell[i] = match terrain[i] {
                Terrain::Mountain => 0.9,
                Terrain::Hills | Terrain::Forest => 0.6,
                Terrain::Sea | Terrain::Lake => 0.05,
                _ => 0.3,
            };
            if !terrain[i].is_water() && ((fx * 40.0 + fy * 13.0).sin().abs() < 0.03) {
                river[i] = if land < 0.2 { 3 } else { 1 };
            }
        }
    }
    // 州郡：以 16×16 格為一塊。
    let mut province_of = vec![NO_PROVINCE; n];
    let mut provinces: Vec<Province> = Vec::new();
    let mut block_id = std::collections::BTreeMap::new();
    for i in 0..n {
        if terrain[i].is_water() {
            continue;
        }
        let (x, y) = (i % w as usize, i / w as usize);
        let key = (y / 16, x / 16);
        let next = block_id.len() as u16;
        let id = *block_id.entry(key).or_insert(next);
        province_of[i] = id;
        if id as usize == provinces.len() {
            provinces.push(Province {
                id,
                name: format!("試{}郡", id),
                cells: vec![],
                seat: i as u32,
                neighbors: vec![],
                sea_neighbors: vec![],
                terrain_mix: [0; Terrain::COUNT],
                habitability: 0.0,
                capacity: 0.0,
                population: 0.0,
                aura: 0.0,
                karma_qi: 0.0,
                merit: 0.0,
                righteous: 0.0,
                devastation: 0.0,
                faith: [0.0; 3],
                city_level: 0,
            });
        }
        let p = &mut provinces[id as usize];
        p.cells.push(i as u32);
        p.terrain_mix[terrain[i] as usize] += 1;
        p.habitability += 0.5;
        p.aura += aura_cell[i];
    }
    for p in &mut provinces {
        p.aura /= p.cells.len() as f32;
        p.population = p.habitability * 1500.0;
        p.capacity = p.habitability * 3000.0;
        p.seat = p.cells[p.cells.len() / 2];
    }
    let geo = Geography {
        w,
        h,
        wrap_x: true,
        sea_level: sea,
        height,
        terrain,
        river,
        province_of,
        aura_cell,
        nodes: dev_nodes(w as usize, h as usize),
        ley: dev_ley(w as usize, h as usize),
        rivers: vec![],
        ranges: vec![],
        continents: vec![],
        continent_of: vec![siming_sim::NO_CONTINENT; n],
    };
    let mut params = WorldParams::new(seed);
    params.name = "試煉界".into();
    Sim::from_parts(params, geo, provinces)
}

fn dev_nodes(w: usize, h: usize) -> Vec<siming_sim::LeyNode> {
    let c = |fx: f32, fy: f32| ((fy * h as f32) as usize * w + (fx * w as f32) as usize) as u32;
    vec![
        siming_sim::LeyNode { cell: c(0.95, 0.45), strength: 1.0, name: "甲".into() },
        siming_sim::LeyNode { cell: c(0.06, 0.55), strength: 0.75, name: "乙".into() },
        siming_sim::LeyNode { cell: c(0.02, 0.68), strength: 0.5, name: "丙".into() },
    ]
}

fn dev_ley(w: usize, h: usize) -> Vec<siming_sim::LeyEdge> {
    let path = |a: (f32, f32), b: (f32, f32)| -> Vec<u32> {
        (0..=40)
            .map(|i| {
                let t = i as f32 / 40.0;
                // 東西環繞：沿最短弧走（可跨接縫）
                let mut dx = b.0 - a.0;
                if dx > 0.5 {
                    dx -= 1.0;
                } else if dx < -0.5 {
                    dx += 1.0;
                }
                let x = ((a.0 + dx * t).rem_euclid(1.0) * w as f32).min(w as f32 - 1.0);
                let y = (a.1 + (b.1 - a.1) * t + 0.05 * (t * 6.0).sin()) * h as f32;
                (y as usize * w + x as usize) as u32
            })
            .collect()
    };
    vec![
        siming_sim::LeyEdge { a: 0, b: 1, strength: 0.9, path: path((0.95, 0.45), (0.06, 0.55)) },
        siming_sim::LeyEdge { a: 1, b: 2, strength: 0.5, path: path((0.06, 0.55), (0.02, 0.68)) },
    ]
}
