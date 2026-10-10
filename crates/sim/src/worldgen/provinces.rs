//! GDD 2.2 步驟 7（州郡）與 GDD 2.3（宜居度）。
//!
//! 流程：宜居度 → 加權 Poisson-disk 種子 → 多源 Dijkstra → 小島 → 合併過小者 → 相鄰（含海路）→ 治所。
//! 面積一律為面積加權（Σ cos(lat)，赤道格），距離為球面距離（赤道格寬），x 東西環繞。

use super::grid::{components, diag_len, key, nb4, nb8, unit, wrap_x, Sphere, NONE};
use crate::params::*;
use crate::types::Terrain;
use rand_chacha::ChaCha8Rng;
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

/// 尚未編為最終 `Province` 的州郡資料。
pub struct RawProvince {
    pub cells: Vec<u32>,
    pub seat: u32,
    pub neighbors: Vec<u16>,
    pub sea_neighbors: Vec<u16>,
    pub terrain_mix: [u32; Terrain::COUNT],
    pub habitability: f64,
    /// 鄰接海或湖的格數（命名時判斷水系州郡用）。
    pub coast_cells: u32,
}

pub struct Provinces {
    /// 每格的州郡 ID；水格為 `u16::MAX`。
    pub of: Vec<u16>,
    pub list: Vec<RawProvince>,
}

/// GDD 2.3 宜居度（每格）：（地形 + 鄰接河流加成（取最大）+ 鄰接海或湖加成）× 面積權重 cos(lat)。
pub fn cell_habitability(sp: &Sphere, terrain: &[Terrain], river: &[u8]) -> Vec<f64> {
    let (w, h) = (sp.w, sp.h);
    let n = w * h;
    let mut hab = vec![0.0f64; n];
    for c in 0..n {
        if terrain[c].is_water() {
            continue;
        }
        let mut v = HAB_TERRAIN[terrain[c] as usize];
        let mut rv = river[c];
        let mut coast = false;
        for (nb, _) in nb8(w, h, c) {
            rv = rv.max(river[nb]);
            coast |= terrain[nb].is_water();
        }
        v += match rv {
            1 => HAB_RIVER_SMALL,
            2 => HAB_RIVER_LARGE,
            3 => HAB_RIVER_JIANG,
            _ => 0.0,
        };
        if coast {
            v += HAB_COAST;
        }
        hab[c] = v * sp.area(c) as f64;
    }
    hab
}

/// GDD 2.2 步驟 7 成長代價：進入該格的代價。
fn step_cost(t: Terrain, river: u8) -> f32 {
    let mut c = WG_PROV_COST_BASE;
    match t {
        Terrain::Hills => c += WG_PROV_COST_HILL,
        Terrain::Mountain => c += WG_PROV_COST_MOUNTAIN,
        _ => {}
    }
    if river >= 2 {
        c += WG_PROV_COST_CROSS_RIVER;
    }
    c
}

/// GDD 2.2 步驟 7 種子點：以宜居度加權的 Poisson-disk 取樣（最小間距 `0.8 × sqrt(陸地面積 / 目標)`，球面距離）。
/// 若候選用盡而數量未達目標，將間距縮為 0.9 倍後繼續。
fn poisson_seeds(
    sp: &Sphere,
    cand: &[u32],
    hab: &[f64],
    land_area: f32,
    target: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<u32> {
    let (w, h) = (sp.w, sp.h);
    let mut seeds: Vec<u32> = Vec::new();
    if cand.is_empty() || target == 0 {
        return seeds;
    }
    let mut r = WG_PROV_SPACING * (land_area / target as f32).sqrt();
    let mut alive: Vec<bool> = vec![true; cand.len()];
    let mut pos_of = vec![NONE; w * h];
    for (i, &c) in cand.iter().enumerate() {
        pos_of[c as usize] = i as u32;
    }
    while seeds.len() < target && r >= 1.0 {
        let total: f64 = cand
            .iter()
            .enumerate()
            .filter(|(i, _)| alive[*i])
            .map(|(_, &c)| hab[c as usize])
            .sum();
        if total <= 0.0 {
            // 候選用盡：縮小間距，重算存活者。
            r *= 0.9;
            let r2 = r * r;
            for (i, &c) in cand.iter().enumerate() {
                alive[i] = !seeds.iter().any(|&s| sp.dist2(s as usize, c as usize) < r2)
                    && !seeds.contains(&c);
            }
            continue;
        }
        let mut u = unit(rng) as f64 * total;
        let mut pick = usize::MAX;
        for (i, &c) in cand.iter().enumerate() {
            if !alive[i] {
                continue;
            }
            pick = i;
            u -= hab[c as usize];
            if u < 0.0 {
                break;
            }
        }
        let s = cand[pick] as usize;
        seeds.push(s as u32);
        alive[pick] = false;
        let r2 = r * r;
        for c in sp.box_cells(s, r) {
            if sp.dist2(s, c) < r2 {
                let p = pos_of[c];
                if p != NONE {
                    alive[p as usize] = false;
                }
            }
        }
    }
    seeds
}

/// 多源 BFS：每格取得最近的已標記來源的標籤與步數（八鄰、穿過水格）。
fn nearest_label(w: usize, h: usize, lab: &[u32], sources: &[bool]) -> (Vec<u32>, Vec<u32>) {
    let n = w * h;
    let mut near = vec![NONE; n];
    let mut dist = vec![NONE; n];
    let mut q: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for c in 0..n {
        if sources[c] {
            near[c] = lab[c];
            dist[c] = 0;
            q.push_back(c);
        }
    }
    while let Some(c) = q.pop_front() {
        for (nb, _) in nb8(w, h, c) {
            if dist[nb] == NONE {
                dist[nb] = dist[c] + 1;
                near[nb] = near[c];
                q.push_back(nb);
            }
        }
    }
    (near, dist)
}

/// GDD 2.2 步驟 7：從種子成長出州郡，回傳每格標籤（陸地格全數標記；水格為 `NONE`）。
fn grow(w: usize, h: usize, terrain: &[Terrain], river: &[u8], seeds: &[u32]) -> Vec<u32> {
    let n = w * h;
    let land: Vec<bool> = terrain.iter().map(|t| !t.is_water()).collect();
    let mut lab = vec![NONE; n];
    let mut dist = vec![f32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(u32, u32, u32)>> = BinaryHeap::new();
    for (i, &s) in seeds.iter().enumerate() {
        dist[s as usize] = 0.0;
        lab[s as usize] = i as u32;
        heap.push(Reverse((key(0.0), s, i as u32)));
    }
    while let Some(Reverse((k, c, l))) = heap.pop() {
        let c = c as usize;
        if lab[c] != l || k != key(dist[c]) {
            continue;
        }
        let (cx, cy) = (c % w, c / w);
        for (nb, diag) in nb8(w, h, c) {
            if !land[nb] {
                continue;
            }
            if diag {
                // 不穿過兩側皆為水的對角縫。
                let (nx, ny) = (nb % w, nb / w);
                if !land[cy * w + nx] && !land[ny * w + cx] {
                    continue;
                }
            }
            let sc = step_cost(terrain[nb], river[nb]);
            let nd = dist[c] + if diag { sc * diag_len() } else { sc };
            if nd < dist[nb] {
                dist[nb] = nd;
                lab[nb] = l;
                heap.push(Reverse((key(nd), nb as u32, l)));
            }
        }
    }
    lab
}

/// GDD 2.2 步驟 7 小島 / 孤立碎片：把一組格併入最近的已標記州郡。同一塊八鄰連通陸塊（大洲）上有州郡者，
/// 先在該陸塊內以八鄰 BFS 找最近的州郡（州郡不跨大洲）；只有自成一塊的小島才併入球面上最近的沿岸州郡。
fn absorb_nearest(w: usize, h: usize, lab: &mut [u32], pending: &[Vec<u32>], comp8: &[u32]) {
    if pending.is_empty() {
        return;
    }
    let n = w * h;
    let mut is_pending = vec![false; n];
    for g in pending {
        for &c in g {
            is_pending[c as usize] = true;
        }
    }
    let sources: Vec<bool> = (0..n).map(|c| lab[c] != NONE && !is_pending[c]).collect();
    let (near, dist) = nearest_label(w, h, lab, &sources);
    let mut seen = vec![false; n];
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for g in pending {
        // 同陸塊內的 BFS（八鄰；固定順序）。
        let mut found = NONE;
        let mut touched: Vec<usize> = Vec::new();
        for &c in g {
            seen[c as usize] = true;
            touched.push(c as usize);
            queue.push_back(c as usize);
        }
        while let Some(c) = queue.pop_front() {
            if sources[c] {
                found = lab[c];
                break;
            }
            for (nb, _) in nb8(w, h, c) {
                if !seen[nb] && comp8[nb] != NONE && comp8[nb] == comp8[c] {
                    seen[nb] = true;
                    touched.push(nb);
                    queue.push_back(nb);
                }
            }
        }
        queue.clear();
        for c in touched {
            seen[c] = false;
        }
        if found != NONE {
            for &c in g {
                lab[c as usize] = found;
            }
            continue;
        }
        let mut best = (NONE, NONE);
        for &c in g {
            let d = dist[c as usize];
            if d < best.0 {
                best = (d, near[c as usize]);
            }
        }
        if best.1 != NONE {
            for &c in g {
                lab[c as usize] = best.1;
            }
        }
    }
}

/// 以標籤相同的四鄰連通分量；回傳 (comp id 陣列, 各分量格數, 各分量標籤)。
fn label_components(w: usize, h: usize, lab: &[u32]) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let n = w * h;
    let mut comp = vec![NONE; n];
    let mut sizes = Vec::new();
    let mut labels = Vec::new();
    let mut stack = Vec::new();
    for s in 0..n {
        if lab[s] == NONE || comp[s] != NONE {
            continue;
        }
        let id = sizes.len() as u32;
        comp[s] = id;
        stack.push(s);
        let mut cnt = 0;
        while let Some(c) = stack.pop() {
            cnt += 1;
            for nb in nb4(w, h, c) {
                if comp[nb] == NONE && lab[nb] == lab[s] {
                    comp[nb] = id;
                    stack.push(nb);
                }
            }
        }
        sizes.push(cnt);
        labels.push(lab[s]);
    }
    (comp, sizes, labels)
}

/// GDD 2.2 步驟 7 合併過小者：面積 < 平均 35%（且 < 30 赤道格）的州郡，併入代價最低的陸鄰（共享邊界的平均成長代價最小者）。
fn merge_small(
    sp: &Sphere,
    terrain: &[Terrain],
    river: &[u8],
    lab: &mut [u32],
    n_labels: usize,
) {
    let (w, h) = (sp.w, sp.h);
    let n = w * h;
    let land_total: f32 = (0..n).filter(|&c| lab[c] != NONE).map(|c| sp.area(c)).sum();
    let mut live = vec![false; n_labels];
    for &l in lab.iter() {
        if l != NONE {
            live[l as usize] = true;
        }
    }
    let live_count = live.iter().filter(|&&b| b).count();
    if live_count == 0 {
        return;
    }
    let mean = land_total / live_count as f32;
    let thresh = (WG_PROVINCE_MIN_RATIO * mean).max(WG_PROVINCE_MIN_CELLS as f32);
    let mut blocked = vec![false; n_labels]; // 無陸鄰而無法合併者
    loop {
        let mut area = vec![0.0f32; n_labels];
        for (c, &l) in lab.iter().enumerate() {
            if l != NONE {
                area[l as usize] += sp.area(c);
            }
        }
        let mut pick = NONE;
        for l in 0..n_labels {
            if area[l] > 0.0
                && !blocked[l]
                && area[l] < thresh
                && (pick == NONE || area[l] < area[pick as usize])
            {
                pick = l as u32;
            }
        }
        if pick == NONE {
            break;
        }
        // 共享邊界統計：label -> (代價和, 邊數)。
        let mut acc: Vec<(u32, f32, u32)> = Vec::new();
        for c in 0..n {
            if lab[c] != pick {
                continue;
            }
            for nb in nb4(w, h, c) {
                let l = lab[nb];
                if l == NONE || l == pick {
                    continue;
                }
                let cost =
                    0.5 * (step_cost(terrain[c], river[c]) + step_cost(terrain[nb], river[nb]));
                match acc.iter_mut().find(|e| e.0 == l) {
                    Some(e) => {
                        e.1 += cost;
                        e.2 += 1;
                    }
                    None => acc.push((l, cost, 1)),
                }
            }
        }
        if acc.is_empty() {
            blocked[pick as usize] = true;
            continue;
        }
        let mut best = 0;
        for i in 1..acc.len() {
            let (a, b) = (&acc[i], &acc[best]);
            let (ma, mb) = (a.1 / a.2 as f32, b.1 / b.2 as f32);
            if ma < mb || (ma == mb && (a.2 > b.2 || (a.2 == b.2 && a.0 < b.0))) {
                best = i;
            }
        }
        let target = acc[best].0;
        for l in lab.iter_mut() {
            if *l == pick {
                *l = target;
            }
        }
    }
}

/// 點 a、b 之間的連線（x 取環繞後的短邊）是否經過海格（取樣：每步一格）。
fn line_crosses_sea(w: usize, terrain: &[Terrain], a: usize, b: usize) -> bool {
    let (ax, ay) = ((a % w) as i32, (a / w) as i32);
    let (bx, by) = ((b % w) as i32, (b / w) as i32);
    let mut dx = bx - ax;
    if dx > w as i32 / 2 {
        dx -= w as i32;
    } else if dx < -(w as i32) / 2 {
        dx += w as i32;
    }
    let dy = by - ay;
    let steps = dx.abs().max(dy.abs());
    for s in 1..steps {
        let x = (ax as f32 + dx as f32 * s as f32 / steps as f32).round() as i32;
        let y = (ay as f32 + dy as f32 * s as f32 / steps as f32).round() as i32;
        if terrain[y as usize * w + wrap_x(w, x)] == Terrain::Sea {
            return true;
        }
    }
    false
}

/// 以 `member` 的連通分量（四鄰），回傳每格所屬分量的面積（赤道格）。
fn component_area(sp: &Sphere, label: &[u32], count: usize) -> Vec<f32> {
    let mut area = vec![0.0f32; count];
    for (c, &l) in label.iter().enumerate() {
        if l != NONE {
            area[l as usize] += sp.area(c);
        }
    }
    area
}

/// GDD 2.2 步驟 7：完整的州郡生成。
pub fn build(
    sp: &Sphere,
    terrain: &[Terrain],
    river: &[u8],
    target: usize,
    rng: &mut ChaCha8Rng,
) -> Provinces {
    let (w, h) = (sp.w, sp.h);
    let n = w * h;
    let hab = cell_habitability(sp, terrain, river);
    let land: Vec<bool> = terrain.iter().map(|t| !t.is_water()).collect();
    let land_area: f32 = (0..n).filter(|&c| land[c]).map(|c| sp.area(c)).sum();

    // 島嶼（四鄰連通陸塊，面積加權）。
    let (cl, csz) = components(w, h, &land, false);
    let carea = component_area(sp, &cl, csz.len());
    let cand: Vec<u32> = (0..n)
        .filter(|&c| land[c] && carea[cl[c] as usize] >= WG_ISLAND_MIN as f32)
        .map(|c| c as u32)
        .collect();
    let seeds = poisson_seeds(sp, &cand, &hab, land_area, target, rng);
    let mut lab = grow(w, h, terrain, river, &seeds);
    let mut n_labels = seeds.len();

    // 未被種子涵蓋的陸地：≥ WG_ISLAND_MIN 者自成一州，其餘稍後併入最近的沿岸州郡。
    let unclaimed: Vec<bool> = (0..n).map(|c| land[c] && lab[c] == NONE).collect();
    let (ul, usz) = components(w, h, &unclaimed, false);
    let uarea = component_area(sp, &ul, usz.len());
    let mut pending: Vec<Vec<u32>> = Vec::new();
    let mut new_label = vec![NONE; usz.len()];
    for (i, &s) in uarea.iter().enumerate() {
        if s >= WG_ISLAND_MIN as f32 {
            new_label[i] = n_labels as u32;
            n_labels += 1;
        }
    }
    let mut small: Vec<Vec<u32>> = vec![Vec::new(); usz.len()];
    for c in 0..n {
        if unclaimed[c] {
            let id = ul[c] as usize;
            if new_label[id] != NONE {
                lab[c] = new_label[id];
            } else {
                small[id].push(c as u32);
            }
        }
    }
    pending.extend(small.into_iter().filter(|g| !g.is_empty()));

    // 四鄰不連通的碎片：併入共享邊界最長的鄰州；無鄰者視為小島。
    {
        let (comp, sizes, labels) = label_components(w, h, &lab);
        let mut main_of: Vec<(u32, u32)> = vec![(NONE, 0); n_labels]; // label -> (comp, size)
        for (ci, &l) in labels.iter().enumerate() {
            if sizes[ci] > main_of[l as usize].1 {
                main_of[l as usize] = (ci as u32, sizes[ci]);
            }
        }
        let mut frag: Vec<Vec<u32>> = vec![Vec::new(); sizes.len()];
        for c in 0..n {
            if lab[c] != NONE {
                let ci = comp[c] as usize;
                if main_of[labels[ci] as usize].0 != ci as u32 {
                    frag[ci].push(c as u32);
                }
            }
        }
        for g in frag.into_iter().filter(|g| !g.is_empty()) {
            let mut votes: Vec<(u32, u32)> = Vec::new();
            for &c in &g {
                for nb in nb4(w, h, c as usize) {
                    let l = lab[nb];
                    if l == NONE || l == lab[c as usize] {
                        continue;
                    }
                    match votes.iter_mut().find(|v| v.0 == l) {
                        Some(v) => v.1 += 1,
                        None => votes.push((l, 1)),
                    }
                }
            }
            let mut best: Option<(u32, u32)> = None;
            for &(l, v) in &votes {
                if best.is_none_or(|(bl, bv)| v > bv || (v == bv && l < bl)) {
                    best = Some((l, v));
                }
            }
            match best {
                Some((l, _)) => {
                    for &c in &g {
                        lab[c as usize] = l;
                    }
                }
                None => {
                    for &c in &g {
                        lab[c as usize] = NONE;
                    }
                    pending.push(g);
                }
            }
        }
    }
    let non_sea: Vec<bool> = terrain.iter().map(|&t| t != Terrain::Sea).collect();
    let (comp8, _) = components(w, h, &non_sea, true);
    absorb_nearest(w, h, &mut lab, &pending, &comp8);

    merge_small(sp, terrain, river, &mut lab, n_labels);

    // 重新編號：依最小格索引升冪，ID 0..n 連續。
    let mut remap = vec![NONE; n_labels];
    let mut next = 0u32;
    for c in 0..n {
        let l = lab[c];
        if l != NONE && remap[l as usize] == NONE {
            remap[l as usize] = next;
            next += 1;
        }
    }
    let count = next as usize;
    let mut of = vec![u16::MAX; n];
    let mut cells: Vec<Vec<u32>> = vec![Vec::new(); count];
    for c in 0..n {
        if lab[c] != NONE {
            let id = remap[lab[c] as usize];
            of[c] = id as u16;
            cells[id as usize].push(c as u32);
        }
    }

    // 陸鄰：共享邊界格。
    let mut nbr: Vec<BTreeSet<u16>> = vec![BTreeSet::new(); count];
    for c in 0..n {
        if of[c] == u16::MAX {
            continue;
        }
        for nb in nb4(w, h, c) {
            if of[nb] != u16::MAX && of[nb] != of[c] {
                nbr[of[c] as usize].insert(of[nb]);
            }
        }
    }

    // 沿岸格（海或湖，用於命名）與沿海格（僅海，用於海路）。
    let mut coast_cells = vec![0u32; count];
    let mut sea_coast = vec![false; n];
    for c in 0..n {
        if of[c] == u16::MAX {
            continue;
        }
        let mut near_water = false;
        let mut near_sea = false;
        for (nb, _) in nb8(w, h, c) {
            near_water |= terrain[nb].is_water();
            near_sea |= terrain[nb] == Terrain::Sea;
        }
        if near_water {
            coast_cells[of[c] as usize] += 1;
        }
        sea_coast[c] = near_sea;
    }

    // 海鄰：隔海、球面距離 ≤ WG_SEA_LANE 的沿岸州郡（兩者連線須經過海格；陸鄰不算）。
    let mut lanes: Vec<BTreeSet<u16>> = vec![BTreeSet::new(); count];
    let r2 = (WG_SEA_LANE * WG_SEA_LANE) as f32;
    for a in 0..n {
        if !sea_coast[a] {
            continue;
        }
        let pa = of[a];
        for b in sp.box_cells(a, WG_SEA_LANE as f32) {
            if !sea_coast[b] || sp.dist2(a, b) > r2 {
                continue;
            }
            let pb = of[b];
            if pb <= pa || nbr[pa as usize].contains(&pb) || lanes[pa as usize].contains(&pb) {
                continue;
            }
            if line_crosses_sea(w, terrain, a, b) {
                lanes[pa as usize].insert(pb);
                lanes[pb as usize].insert(pa);
            }
        }
    }

    let mut list = Vec::with_capacity(count);
    for id in 0..count {
        let cs = std::mem::take(&mut cells[id]);
        let mut mix = [0u32; Terrain::COUNT];
        let mut hsum = 0.0f64;
        let mut seat = cs[0];
        for &c in &cs {
            mix[terrain[c as usize] as usize] += 1;
            hsum += hab[c as usize];
            if hab[c as usize] > hab[seat as usize] {
                seat = c;
            }
        }
        list.push(RawProvince {
            cells: cs,
            seat,
            neighbors: nbr[id].iter().copied().collect(),
            sea_neighbors: lanes[id].iter().copied().collect(),
            terrain_mix: mix,
            habitability: hsum,
            coast_cells: coast_cells[id],
        });
    }
    Provinces { of, list }
}
