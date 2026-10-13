//! 地形與州郡生成的守門：世界種子只保證地形、州郡與地脈不變，其餘（人物與事件）不再要求同種子同結果。
//! 期望值取自改動前的 main（提交 2abdbd4，以 git archive 解到暫存目錄跑出，與本分支人口期 0 年的結果相同），改動地形、州郡生成或 `poisson_seeds` 會讓它變。

use siming_sim::{Sim, WorldParams};

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= u64::from(*b);
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// 開天前（人口期 0 年、人物期 0 年）的地形格與州郡劃分摘要。
fn terrain_and_provinces(seed: u64) -> (u64, usize, u64) {
    let mut p = WorldParams::new(seed);
    p.pop_years = 0;
    p.prerun_years = 0;
    let sim = Sim::new(&p).unwrap();
    let w = sim.world();
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for t in &w.geo.terrain {
        fnv(&mut h, format!("{t:?}").as_bytes());
    }
    for pr in &w.provinces {
        fnv(&mut h, pr.name.as_bytes());
        for c in &pr.cells {
            fnv(&mut h, &c.to_le_bytes());
        }
    }
    (seed, w.provinces.len(), h)
}

#[test]
fn poisson_seeds_terrain_and_provinces_match_main() {
    let got: Vec<(u64, usize, u64)> = [20261011_u64, 1, 777].iter().map(|&s| terrain_and_provinces(s)).collect();
    let expected: Vec<(u64, usize, u64)> =
        vec![(20261011, 107, 15733051178332242670), (1, 107, 12284866427051590649), (777, 109, 16497272462670242415)];
    assert_eq!(got, expected);
}
