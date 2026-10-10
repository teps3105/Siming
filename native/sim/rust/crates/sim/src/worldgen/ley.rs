//! GDD 2.4 靈脈網與格點靈氣：洞天福地（節點）、靈脈（MST + 環邊 + A* 路徑）、格點靈氣、州郡靈氣。

use super::grid::{cheb, diag_len, key, nb8, Sphere, NONE};
use crate::params::*;
use crate::types::{Era, LeyEdge, LeyNode, Terrain};
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

/// 靈脈靈氣 `exp(−d/3)` 的截斷半徑為此倍數 × 衰減距離（超過時貢獻 < 0.3·e⁻⁵ ≈ 0.002）。
/// 7.1 未列出，屬實作細節（只為效能）。
const LEY_AURA_RADIUS_FACTOR: f32 = 5.0;

/// GDD 2.4 節點：山脈與丘陵格中「靈性」`h × 0.6 + ridged × 0.4` 的局部極大值，依靈性由高到低挑選，
/// 彼此最小距離 `WG_NODE_MIN_DIST`，數量 = 目標州郡數 × `WG_NODE_RATIO`。
/// 若局部極大值不足，以山脈／丘陵格補足（實作細節）。名稱由命名步驟補上。
pub fn nodes(
    sp: &Sphere,
    terrain: &[Terrain],
    height: &[f32],
    ridged: &[f32],
    target_provinces: u32,
) -> Vec<LeyNode> {
    let (w, h) = (sp.w, sp.h);
    let n = w * h;
    let spirit: Vec<f32> = (0..n)
        .map(|i| height[i] * WG_SPIRIT_HEIGHT_WEIGHT + ridged[i] * WG_SPIRIT_RIDGE_WEIGHT)
        .collect();
    let eligible = |c: usize| matches!(terrain[c], Terrain::Mountain | Terrain::Hills);
    let is_max = |c: usize| {
        nb8(w, h, c).all(|(nb, _)| spirit[c] > spirit[nb] || (spirit[c] == spirit[nb] && c < nb))
    };
    let want = (target_provinces as f32 * WG_NODE_RATIO).round() as usize;
    let sort_desc = |v: &mut Vec<usize>| {
        v.sort_by(|&a, &b| spirit[b].partial_cmp(&spirit[a]).unwrap().then(a.cmp(&b)));
    };
    let mut maxima: Vec<usize> = (0..n).filter(|&c| eligible(c) && is_max(c)).collect();
    sort_desc(&mut maxima);
    let mut chosen: Vec<usize> = Vec::new();
    let min2 = (WG_NODE_MIN_DIST * WG_NODE_MIN_DIST) as f32;
    let try_pick = |list: &[usize], chosen: &mut Vec<usize>| {
        for &c in list {
            if chosen.len() >= want {
                break;
            }
            if chosen.iter().all(|&o| sp.dist2(o, c) >= min2) {
                chosen.push(c);
            }
        }
    };
    try_pick(&maxima, &mut chosen);
    if chosen.len() < want {
        let mut rest: Vec<usize> = (0..n).filter(|&c| eligible(c)).collect();
        sort_desc(&mut rest);
        try_pick(&rest, &mut chosen);
    }
    let cnt = chosen.len();
    let top = (cnt as f32 * WG_NODE_GRADE_TOP_RATIO).round() as usize;
    let mid = (cnt as f32 * (WG_NODE_GRADE_TOP_RATIO + WG_NODE_GRADE_MID_RATIO)).round() as usize;
    chosen
        .iter()
        .enumerate()
        .map(|(i, &c)| LeyNode {
            cell: c as u32,
            strength: if i < top {
                WG_NODE_STRENGTH_GREAT
            } else if i < mid {
                WG_NODE_STRENGTH_CAVE
            } else {
                WG_NODE_STRENGTH_BLESSED
            },
            name: String::new(),
        })
        .collect()
}

fn ley_cost(t: Terrain) -> f32 {
    match t {
        Terrain::Mountain => WG_LEY_COST_MOUNTAIN,
        Terrain::Hills => WG_LEY_COST_HILL,
        Terrain::Sea | Terrain::Lake => WG_LEY_COST_WATER,
        _ => WG_LEY_COST_LAND,
    }
}

/// A* 路徑，偏好山脊（GDD 2.4 靈脈）。回傳 a 到 b 的格序列（含兩端）；啟發式為環繞下的 Chebyshev 步數 × 最低代價。
fn astar(w: usize, h: usize, terrain: &[Terrain], a: usize, b: usize) -> Vec<u32> {
    let n = w * h;
    let min_cost = WG_LEY_COST_MOUNTAIN
        .min(WG_LEY_COST_HILL)
        .min(WG_LEY_COST_LAND)
        .min(WG_LEY_COST_WATER);
    let hfun = |c: usize| cheb(w, c, b) as f32 * min_cost;
    let mut g = vec![f32::MAX; n];
    let mut from = vec![NONE; n];
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
    g[a] = 0.0;
    heap.push(Reverse((key(hfun(a)), a as u32)));
    while let Some(Reverse((_, c))) = heap.pop() {
        let c = c as usize;
        if c == b {
            break;
        }
        for (nb, diag) in nb8(w, h, c) {
            let step = ley_cost(terrain[nb]) * if diag { diag_len() } else { 1.0 };
            let ng = g[c] + step;
            if ng < g[nb] {
                g[nb] = ng;
                from[nb] = c as u32;
                heap.push(Reverse((key(ng + hfun(nb)), nb as u32)));
            }
        }
    }
    let mut path = vec![b as u32];
    let mut cur = b;
    while cur != a {
        cur = from[cur] as usize;
        path.push(cur as u32);
    }
    path.reverse();
    path
}

/// GDD 2.4 靈脈（邊）：節點最小生成樹，再為每個節點加一條通往「非 MST 鄰居中最近者」的邊。
pub fn edges(sp: &Sphere, terrain: &[Terrain], nodes: &[LeyNode]) -> Vec<LeyEdge> {
    let (w, h) = (sp.w, sp.h);
    let k = nodes.len();
    if k < 2 {
        return Vec::new();
    }
    let d2 = |i: usize, j: usize| sp.dist2(nodes[i].cell as usize, nodes[j].cell as usize);
    // Prim。
    let mut in_tree = vec![false; k];
    let mut best = vec![(f32::MAX, 0usize); k];
    let mut mst: BTreeSet<(u16, u16)> = BTreeSet::new();
    in_tree[0] = true;
    for j in 1..k {
        best[j] = (d2(0, j), 0);
    }
    for _ in 1..k {
        let mut pick = usize::MAX;
        for j in 0..k {
            if !in_tree[j] && (pick == usize::MAX || best[j].0 < best[pick].0) {
                pick = j;
            }
        }
        in_tree[pick] = true;
        let p = best[pick].1;
        mst.insert((p.min(pick) as u16, p.max(pick) as u16));
        for j in 0..k {
            if !in_tree[j] {
                let d = d2(pick, j);
                if d < best[j].0 {
                    best[j] = (d, pick);
                }
            }
        }
    }
    let mut all = mst.clone();
    for i in 0..k {
        let mut near: Option<(f32, usize)> = None;
        for j in 0..k {
            if j == i || mst.contains(&(i.min(j) as u16, i.max(j) as u16)) {
                continue;
            }
            let d = d2(i, j);
            if near.is_none_or(|(bd, _)| d < bd) {
                near = Some((d, j));
            }
        }
        if let Some((_, j)) = near {
            all.insert((i.min(j) as u16, i.max(j) as u16));
        }
    }
    all.into_iter()
        .map(|(a, b)| LeyEdge {
            a,
            b,
            strength: 0.5 * (nodes[a as usize].strength + nodes[b as usize].strength),
            path: astar(
                w,
                h,
                terrain,
                nodes[a as usize].cell as usize,
                nodes[b as usize].cell as usize,
            ),
        })
        .collect()
}

/// GDD 2.4 格點靈氣：`時代倍率 × (地形基礎 + Σ 節點 s·0.8·exp(−d/8) + 最近靈脈格 s·0.3·exp(−d/3))`，截斷 0..2。
pub fn aura_cell(
    sp: &Sphere,
    terrain: &[Terrain],
    nodes: &[LeyNode],
    ley: &[LeyEdge],
    era: Era,
) -> Vec<f32> {
    let n = sp.w * sp.h;
    let mult = match era {
        Era::Ancient => AURA_ERA_ANCIENT,
        Era::Middle => AURA_ERA_MIDDLE,
        Era::Decline => AURA_ERA_DECLINE,
    };
    // 最近靈脈格：以截斷半徑內的 splat 求每格最近的靈脈格及其強度。
    let mut best_d2 = vec![f32::MAX; n];
    let mut best_s = vec![0.0f32; n];
    let rad = LEY_AURA_RADIUS_FACTOR * AURA_LEY_DECAY;
    let rad2 = rad * rad;
    for e in ley {
        for &pc in &e.path {
            for i in sp.box_cells(pc as usize, rad) {
                let d2 = sp.dist2(pc as usize, i);
                if d2 > rad2 {
                    continue;
                }
                if d2 < best_d2[i] || (d2 == best_d2[i] && e.strength > best_s[i]) {
                    best_d2[i] = d2;
                    best_s[i] = e.strength;
                }
            }
        }
    }
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let mut v = AURA_TERRAIN[terrain[i] as usize];
        for nd in nodes {
            let d = sp.dist(i, nd.cell as usize);
            v += nd.strength * AURA_NODE_COEF * (-d / AURA_NODE_DECAY).exp();
        }
        if best_d2[i] != f32::MAX {
            v += best_s[i] * AURA_LEY_COEF * (-best_d2[i].sqrt() / AURA_LEY_DECAY).exp();
        }
        out[i] = (v * mult).clamp(0.0, AURA_CELL_MAX);
    }
    out
}
