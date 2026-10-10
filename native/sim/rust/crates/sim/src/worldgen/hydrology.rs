//! GDD 2.2 步驟 3（填窪與湖）與步驟 6（河流：流向、累積、分級、幹流）。

use super::grid::{components, cos, key, nb8, row_lat, NONE};
use crate::params::*;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// 步驟 3 的結果。
pub struct Filled {
    /// 填窪後的高度（海格不變）。
    pub filled: Vec<f32>,
    /// 流向目標（格索引）；海格與流出地圖者為 [`NONE`]。
    pub flow: Vec<u32>,
    /// priority-flood 的彈出順序：流向目標一定排在其上游之前。
    pub order: Vec<u32>,
    pub lake: Vec<bool>,
}

/// 相鄰兩格的球面步長（赤道格）：東西向為 `cos(lat)`、南北向為 1、對角為兩者的平方和開根號（極區東西鄰格很近，
/// 避免在極列造成假的匯流）。
pub fn step_len(w: usize, hh: usize, c: usize, nb: usize) -> f32 {
    let (x, y) = (c % w, c / w);
    let (nx, ny) = (nb % w, nb / w);
    let ew = if x != nx {
        let lat = 0.5 * (row_lat(hh, y) + row_lat(hh, ny));
        cos(lat).max(1e-3)
    } else {
        0.0
    };
    let ns = if y != ny { 1.0 } else { 0.0 };
    (ew * ew + ns * ns).sqrt()
}

/// GDD 2.2 步驟 3：以 priority-flood（Barnes et al. 2014）對陸地填窪；填平後水深 ≥ `WG_LAKE_DEPTH` 且面積 ≥
/// `lake_min`（`WG_LAKE_MIN` × 解析度²）的窪地標為湖，其餘小窪地視為填平。同時依 D8 最陡下降求流向（坡度以球面步長計；平地與湖面依 flood
/// 的來源方向，即 ε 變體的效果，保證每格都能流出）。
pub fn fill_and_lakes(h: &[f32], w: usize, hh: usize, sea: f32, lake_min: u32) -> Filled {
    let n = w * hh;
    let is_sea: Vec<bool> = h.iter().map(|&v| v < sea).collect();
    let mut filled = h.to_vec();
    let mut parent = vec![NONE; n];
    let mut visited = vec![false; n];
    let mut order: Vec<u32> = Vec::with_capacity(n);
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();

    for c in 0..n {
        if is_sea[c] {
            visited[c] = true;
            continue;
        }
        let y = c / w;
        // 極區列（y = 0、H − 1）視為可排出；x 東西環繞沒有邊。
        let border = y == 0 || y + 1 == hh;
        if border || nb8(w, hh, c).any(|(nb, _)| is_sea[nb]) {
            visited[c] = true;
            heap.push(Reverse((key(h[c]), c as u32)));
        }
    }
    while let Some(Reverse((_, c))) = heap.pop() {
        let c = c as usize;
        order.push(c as u32);
        for (nb, _) in nb8(w, hh, c) {
            if visited[nb] {
                continue;
            }
            visited[nb] = true;
            filled[nb] = h[nb].max(filled[c]);
            parent[nb] = c as u32;
            heap.push(Reverse((key(filled[nb]), nb as u32)));
        }
    }

    // 湖：填平後高於原高度 ≥ WG_LAKE_DEPTH 的連通窪地，面積 ≥ WG_LAKE_MIN（淺的填平部分不算湖）。
    let depression: Vec<bool> = (0..n)
        .map(|c| !is_sea[c] && filled[c] - h[c] >= WG_LAKE_DEPTH)
        .collect();
    let (label, sizes) = components(w, hh, &depression, true);
    let mut lake = vec![false; n];
    for c in 0..n {
        if depression[c] && sizes[label[c] as usize] >= lake_min {
            lake[c] = true;
        }
    }

    // D8 最陡下降（在填窪後的高度上）；無更低鄰格時沿 flood 的來源方向。
    let mut flow = vec![NONE; n];
    for c in 0..n {
        if is_sea[c] {
            continue;
        }
        let mut best = NONE;
        let mut best_slope = 0.0f32;
        for (nb, _) in nb8(w, hh, c) {
            if filled[nb] < filled[c] {
                let slope = (filled[c] - filled[nb]) / step_len(w, hh, c, nb);
                if slope > best_slope {
                    best_slope = slope;
                    best = nb as u32;
                }
            }
        }
        flow[c] = if best != NONE { best } else { parent[c] };
    }

    Filled {
        filled,
        flow,
        order,
        lake,
    }
}

/// 一條已找出的幹流（尚未命名）。`cells` 由源頭到出海口。
pub struct Stem {
    pub level: u8,
    pub cells: Vec<u32>,
}

pub struct Rivers {
    /// 0 無；1 小河；2 大河；3 江。
    pub level: Vec<u8>,
    /// 累積降水量（流量；供細格渲染的河寬）。
    pub acc: Vec<f32>,
    /// 入海的江河幹流，依（等級、長度）降冪。
    pub stems: Vec<Stem>,
}

/// 小河幹流需達到的最短長度（格）才命名。7.1 未列出，屬實作細節；大河與江一律命名。
pub const RIVER_NAME_MIN_LEN: usize = 8;

/// GDD 2.2 步驟 6：依流向累積降水（每格 = `q`），以陸地格累積量的分位數分級，取入海河的最長幹流。
pub fn rivers(f: &Filled, q: &[f32], sea_cell: &[bool], scale: usize) -> Rivers {
    let n = q.len();
    let mut acc = vec![0.0f32; n];
    for c in 0..n {
        if !sea_cell[c] {
            acc[c] = q[c].max(0.0);
        }
    }
    for &c in f.order.iter().rev() {
        let c = c as usize;
        let t = f.flow[c];
        if t != NONE && !sea_cell[t as usize] {
            let a = acc[c];
            acc[t as usize] += a;
        }
    }

    // 分級：陸地（非海、非湖）格累積量的分位數。
    let mut vals: Vec<f32> = (0..n)
        .filter(|&c| !sea_cell[c] && !f.lake[c])
        .map(|c| acc[c])
        .collect();
    vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let thr = |ratio: f32| -> f32 {
        if vals.is_empty() {
            return f32::MAX;
        }
        let k = ((vals.len() as f32 * ratio).ceil() as usize).clamp(1, vals.len());
        vals[k - 1]
    };
    // 分位以遊戲格定義；細格的河道格數約為 scale 倍，分位除以 scale 使降採樣後的河網密度相同。
    let k = scale as f32;
    let (t1, t2, t3) = (
        thr(WG_RIVER_Q_SMALL / k),
        thr(WG_RIVER_Q_LARGE / k),
        thr(WG_RIVER_Q_JIANG / k),
    );
    let mut level = vec![0u8; n];
    for c in 0..n {
        if sea_cell[c] || f.lake[c] {
            continue;
        }
        let a = acc[c];
        level[c] = if a >= t3 {
            3
        } else if a >= t2 {
            2
        } else if a >= t1 {
            1
        } else {
            0
        };
    }

    // 最長幹流：在「河格或湖格」構成的上游樹中，每格記錄最長上游鏈。
    let passable = |c: usize| level[c] > 0 || f.lake[c];
    let mut len = vec![0u32; n];
    let mut up = vec![NONE; n];
    for c in 0..n {
        if passable(c) {
            len[c] = 1;
        }
    }
    for &c in f.order.iter().rev() {
        let c = c as usize;
        if !passable(c) {
            continue;
        }
        let t = f.flow[c];
        if t == NONE {
            continue;
        }
        let t = t as usize;
        if sea_cell[t] || !passable(t) {
            continue;
        }
        let better = len[c] + 1 > len[t]
            || (len[c] + 1 == len[t] && up[t] != NONE && acc[c] > acc[up[t] as usize]);
        if better {
            len[t] = len[c] + 1;
            up[t] = c as u32;
        }
    }
    let mut stems: Vec<Stem> = Vec::new();
    for m in 0..n {
        if !passable(m) {
            continue;
        }
        let t = f.flow[m];
        if t == NONE || !sea_cell[t as usize] {
            continue; // 只取入海者
        }
        let mut cells = vec![m as u32];
        let mut cur = m;
        while up[cur] != NONE {
            cur = up[cur] as usize;
            cells.push(cur as u32);
        }
        cells.reverse();
        let lv = cells.iter().map(|&c| level[c as usize]).max().unwrap_or(0);
        if lv == 0 {
            continue;
        }
        if lv >= 2 || cells.len() >= RIVER_NAME_MIN_LEN {
            stems.push(Stem { level: lv, cells });
        }
    }
    stems.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then(b.cells.len().cmp(&a.cells.len()))
            .then(a.cells.last().cmp(&b.cells.last()))
    });
    Rivers { level, acc, stems }
}
