//! GDD 2.2 步驟 5：地形分類。

use super::grid::{components, nb8};
use crate::params::*;
use crate::types::Terrain;

/// GDD 2.2 步驟 5 去斑：氣候地形（平原、森林、沙漠、沼澤、雪原、丘陵）面積 < `min_patch` 的八鄰連通斑塊，
/// 改為斑塊邊界外最多的（非水、非山脈）地形；做 `passes` 輪。山脈、湖、海不動。
pub fn despeckle(terrain: &mut [Terrain], w: usize, h: usize, min_patch: u32, passes: u32) {
    let climatic =
        |t: Terrain| matches!(t, Terrain::Plains | Terrain::Forest | Terrain::Desert | Terrain::Swamp | Terrain::Snow | Terrain::Hills);
    for _ in 0..passes {
        let n = w * h;
        let member: Vec<bool> = terrain.iter().map(|&t| climatic(t)).collect();
        // 同類相鄰才同一斑塊：以類別標籤分量。
        let mut label = vec![u32::MAX; n];
        let mut groups: Vec<Vec<u32>> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        for s in 0..n {
            if !member[s] || label[s] != u32::MAX {
                continue;
            }
            let id = groups.len() as u32;
            label[s] = id;
            stack.push(s);
            let mut g = Vec::new();
            while let Some(c) = stack.pop() {
                g.push(c as u32);
                for (nb, _) in nb8(w, h, c) {
                    if label[nb] == u32::MAX && terrain[nb] == terrain[s] {
                        label[nb] = id;
                        stack.push(nb);
                    }
                }
            }
            groups.push(g);
        }
        let mut changed = false;
        for g in &groups {
            if g.len() as u32 >= min_patch {
                continue;
            }
            let mut votes = [0u32; Terrain::COUNT];
            let own = terrain[g[0] as usize];
            for &c in g {
                for (nb, _) in nb8(w, h, c as usize) {
                    let t = terrain[nb];
                    if t != own && climatic(t) {
                        votes[t as usize] += 1;
                    }
                }
            }
            let mut best = (0u32, own);
            for t in Terrain::ALL {
                if votes[t as usize] > best.0 {
                    best = (votes[t as usize], t);
                }
            }
            if best.0 > 0 {
                for &c in g {
                    terrain[c as usize] = best.1;
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let _ = components;
}

/// GDD 2.2 步驟 5：依序判定（先符合者優先）。`lake` 為步驟 3 的標記。
pub fn classify(height: &[f32], sea: f32, lake: &[bool], t: &[f32], q: &[f32]) -> Vec<Terrain> {
    (0..height.len())
        .map(|i| {
            if height[i] < sea {
                return Terrain::Sea;
            }
            if lake[i] {
                return Terrain::Lake;
            }
            let lh = (height[i] - sea) / (1.0 - sea);
            if lh > WG_MOUNTAIN_H {
                Terrain::Mountain
            } else if t[i] < WG_SNOW_T {
                Terrain::Snow
            } else if lh > WG_HILL_H {
                Terrain::Hills
            } else if q[i] < WG_DESERT_Q && t[i] > WG_DESERT_T {
                Terrain::Desert
            } else if q[i] > WG_SWAMP_Q && lh < WG_SWAMP_H {
                Terrain::Swamp
            } else if q[i] > WG_FOREST_Q {
                Terrain::Forest
            } else {
                Terrain::Plains
            }
        })
        .collect()
}
