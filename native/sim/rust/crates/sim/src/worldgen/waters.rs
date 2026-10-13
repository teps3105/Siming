//! GDD 7.3 水域命名：海區（洋、海）與湖泊。
//!
//! 相連的海依「離岸最遠的幾個深處」分成數個海區：以各深處為種子同時向外擴張，先到的佔有該格。
//! 夠深的海區稱洋，其餘稱海；沒有深處的小海（封閉海、海灣）整片算一個海。湖泊每個連通的湖一個名字。
//! 不消耗世界生成的隨機流：名字由種子與水域的順序算出。
//!
//! 海陸隨天地演變改變之後（GDD 2.8），水域重新劃分，名字以生成時的水域為準沿用（[`name_waters_since`]）：
//! 結果只看種子、生成時的地形與目前的地形，與中間經過無關，讀檔重播與一路玩下來得到同樣的名字。

use crate::types::{Geography, Terrain};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// 水域的種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterKind {
    Ocean,
    Sea,
    Lake,
}

/// 一個有名字的水域。
#[derive(Debug, Clone, PartialEq)]
pub struct NamedWater {
    pub name: String,
    pub kind: WaterKind,
    /// 名字標在哪一格：海區是離岸最遠的格，湖是最靠近重心的格。
    pub cell: u32,
    /// 格數。
    pub area: u32,
}

/// 離岸至少這麼多格才算「深處」，可以當海區的種子。
const PEAK_MIN: u32 = 4;
/// 兩個海區種子的最小間距：地圖寬除以這個數（中尺寸 384 格 → 42 格）。
const PEAK_SPACING_DIV: usize = 9;
/// 海區最深處離岸達到「地圖寬除以這個數」格就稱洋（中尺寸 → 27 格）。
const OCEAN_DEPTH_DIV: usize = 14;
/// 小於這麼多格的海區與湖不命名。
const SEA_MIN_AREA: u32 = 12;
const LAKE_MIN_AREA: u32 = 3;
/// 湖的稱呼依大小：達到這個格數稱澤，達到下一個稱湖，其餘稱池。
const LAKE_MARSH_AREA: u32 = 40;
const LAKE_LAKE_AREA: u32 = 8;

#[derive(Deserialize)]
struct WaterWords {
    sea_names: Vec<String>,
    lake_names: Vec<String>,
}

const DATA: &str = include_str!("../../data/places.zh-Hant.json");

/// 為世界的海區與湖泊命名。回傳依標記格升冪排列。
pub fn name_waters(seed: u64, geo: &Geography) -> Vec<NamedWater> {
    name_fresh(seed, &survey(geo.w as usize, geo.h as usize, &geo.terrain))
}

/// 海陸變過之後的水域名：`origin` 是生成時遊戲格的地形。
///
/// 生成時的水域由大到小認領目前的水域——標記格現在落在哪個水域，那個水域就沿用它的名字
/// （海區與湖各認各的，一個水域只被認領一次）。沒被認領的是新出現的水域，依標記格另取新名，
/// 不與生成時用過的名相同。生成時的水域若標記格已成陸地，或所在的水域被更大的認領，名字就此消失。
pub fn name_waters_since(seed: u64, geo: &Geography, origin: &[Terrain]) -> Vec<NamedWater> {
    let (w, h) = (geo.w as usize, geo.h as usize);
    name_since(seed, &survey(w, h, origin), &survey(w, h, &geo.terrain))
}

/// 一張地形上劃分出來的水域。
struct Survey {
    /// 種類、標記格、格數；依標記格升冪。
    waters: Vec<(WaterKind, u32, u32)>,
    /// 每格屬於 `waters` 的第幾個；不屬於任何有名水域的格為 `u32::MAX`。
    owner: Vec<u32>,
}

fn is_lake(kind: WaterKind) -> bool {
    kind == WaterKind::Lake
}

fn stem_pool(words: &WaterWords, kind: WaterKind, area: u32) -> (&Vec<String>, &'static str, u8) {
    match kind {
        WaterKind::Ocean => (&words.sea_names, "洋", 1u8),
        WaterKind::Sea => (&words.sea_names, "海", 2u8),
        WaterKind::Lake if area >= LAKE_MARSH_AREA => (&words.lake_names, "澤", 3u8),
        WaterKind::Lake if area >= LAKE_LAKE_AREA => (&words.lake_names, "湖", 3u8),
        WaterKind::Lake => (&words.lake_names, "池", 3u8),
    }
}

/// 依水域的順序取名（世界生成時的取法）。
fn name_fresh(seed: u64, survey: &Survey) -> Vec<NamedWater> {
    let words: WaterWords = serde_json::from_str(DATA).expect("places.zh-Hant.json");
    let mut used = BTreeSet::new();
    let mut counters: BTreeMap<u8, u64> = BTreeMap::new();
    survey
        .waters
        .iter()
        .map(|&(kind, cell, area)| {
            let (pool, suffix, tag) = stem_pool(&words, kind, area);
            let order = counters.entry(tag).or_insert(0);
            *order += 1;
            // 同一個名（不含稱呼）只給一個水域：不會同時有「星洋」與「星海」。
            let mut name = String::new();
            for attempt in 0..(pool.len() as u64 * 8) {
                let hash = mix(seed ^ ((tag as u64) << 56) ^ (*order << 16) ^ attempt);
                let stem = &pool[(hash as usize) % pool.len()];
                if used.insert(stem.clone()) {
                    name = format!("{stem}{suffix}");
                    break;
                }
            }
            if name.is_empty() {
                // 字庫用盡：加上序號保證不重名。
                name = format!("{}{}{}", pool[0], order, suffix);
            }
            NamedWater { name, kind, cell, area }
        })
        .collect()
}

fn name_since(seed: u64, origin: &Survey, now: &Survey) -> Vec<NamedWater> {
    let words: WaterWords = serde_json::from_str(DATA).expect("places.zh-Hant.json");
    let named = name_fresh(seed, origin);
    // 生成時用過的名（不含稱呼）都不再給新水域，消失的名字不會換個地方冒出來。
    let mut used: BTreeSet<String> = named
        .iter()
        .map(|water| {
            let mut stem = water.name.clone();
            stem.pop();
            stem
        })
        .collect();
    let mut names: Vec<Option<String>> = vec![None; now.waters.len()];
    let mut elders: Vec<&NamedWater> = named.iter().collect();
    elders.sort_by(|a, b| b.area.cmp(&a.area).then(a.cell.cmp(&b.cell)));
    for elder in elders {
        let at = now.owner[elder.cell as usize];
        if at == u32::MAX {
            continue;
        }
        let slot = &mut names[at as usize];
        if slot.is_none() && is_lake(now.waters[at as usize].0) == is_lake(elder.kind) {
            *slot = Some(elder.name.clone());
        }
    }
    now.waters
        .iter()
        .zip(names)
        .map(|(&(kind, cell, area), name)| {
            let name = name.unwrap_or_else(|| {
                let (pool, suffix, tag) = stem_pool(&words, kind, area);
                for attempt in 0..(pool.len() as u64 * 8) {
                    // 新水域依標記格取名，與生成時依順序取名的雜湊錯開（最高位元）。
                    let hash = mix(seed ^ (1 << 63) ^ ((tag as u64) << 56) ^ ((cell as u64) << 16) ^ attempt);
                    let stem = &pool[(hash as usize) % pool.len()];
                    if used.insert(stem.clone()) {
                        return format!("{stem}{suffix}");
                    }
                }
                format!("{}{}{}", pool[0], cell, suffix)
            });
            NamedWater { name, kind, cell, area }
        })
        .collect()
}

/// 把一張地形劃分成海區與湖。
fn survey(w: usize, h: usize, terrain: &[Terrain]) -> Survey {
    let n = w * h;
    let neighbours = |c: usize| {
        let (x, y) = (c % w, c / w);
        let mut out = vec![y * w + (x + 1) % w, y * w + (x + w - 1) % w];
        if y > 0 {
            out.push((y - 1) * w + x);
        }
        if y + 1 < h {
            out.push((y + 1) * w + x);
        }
        out
    };
    let is_sea = |c: usize| terrain[c] == Terrain::Sea;

    // 每個海格離岸幾格（地圖的南北緣不算岸）。
    let mut depth = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    for c in 0..n {
        if is_sea(c) && neighbours(c).iter().any(|&m| !is_sea(m)) {
            depth[c] = 1;
            queue.push_back(c);
        }
    }
    while let Some(c) = queue.pop_front() {
        for m in neighbours(c) {
            if is_sea(m) && depth[m] == u32::MAX {
                depth[m] = depth[c] + 1;
                queue.push_back(m);
            }
        }
    }

    // 種子：由深到淺挑，與已挑的種子至少相隔 spacing 格（東西相接）。
    let spacing = (w / PEAK_SPACING_DIV).max(8);
    let mut candidates: Vec<usize> = (0..n).filter(|&c| is_sea(c) && depth[c] != u32::MAX && depth[c] >= PEAK_MIN).collect();
    candidates.sort_by(|&a, &b| depth[b].cmp(&depth[a]).then(a.cmp(&b)));
    let apart = |a: usize, b: usize| {
        let dx = (a % w).abs_diff(b % w);
        let dx = dx.min(w - dx);
        let dy = (a / w).abs_diff(b / w);
        dx.max(dy)
    };
    let mut seeds: Vec<usize> = Vec::new();
    for c in candidates {
        if seeds.iter().all(|&s| apart(c, s) >= spacing) {
            seeds.push(c);
        }
    }

    // 各種子同時向外擴張，先到的佔有該格。
    let mut region = vec![usize::MAX; n];
    let mut queue = VecDeque::new();
    for (i, &s) in seeds.iter().enumerate() {
        region[s] = i;
        queue.push_back(s);
    }
    while let Some(c) = queue.pop_front() {
        for m in neighbours(c) {
            if is_sea(m) && region[m] == usize::MAX {
                region[m] = region[c];
                queue.push_back(m);
            }
        }
    }
    // 沒有種子的小海：整片連通的海算一個海區，種子取離岸最遠的格。
    for c in 0..n {
        if !is_sea(c) || region[c] != usize::MAX {
            continue;
        }
        let id = seeds.len();
        let mut best = c;
        region[c] = id;
        queue.push_back(c);
        while let Some(k) = queue.pop_front() {
            if depth[k] != u32::MAX && (depth[best] == u32::MAX || depth[k] > depth[best]) {
                best = k;
            }
            for m in neighbours(k) {
                if is_sea(m) && region[m] == usize::MAX {
                    region[m] = id;
                    queue.push_back(m);
                }
            }
        }
        seeds.push(best);
    }
    let mut areas = vec![0u32; seeds.len()];
    for c in 0..n {
        if region[c] != usize::MAX {
            areas[region[c]] += 1;
        }
    }

    let ocean_depth = (w / OCEAN_DEPTH_DIV).max(6) as u32;
    // 每個水域連同它佔的格：海區的格由 region 查，湖的格在找到時記下。
    let mut waters: Vec<(WaterKind, u32, u32)> = Vec::new();
    let mut sea_slot = vec![u32::MAX; seeds.len()];
    for (i, &s) in seeds.iter().enumerate() {
        if areas[i] < SEA_MIN_AREA {
            continue;
        }
        let deep = depth[s] != u32::MAX && depth[s] >= ocean_depth;
        sea_slot[i] = waters.len() as u32;
        waters.push((if deep { WaterKind::Ocean } else { WaterKind::Sea }, s as u32, areas[i]));
    }
    let mut owner = vec![u32::MAX; n];
    for c in 0..n {
        if region[c] != usize::MAX {
            owner[c] = sea_slot[region[c]];
        }
    }

    // 湖：每個連通的湖一個；名字標在最靠近重心的格。
    let mut seen = vec![false; n];
    for c in 0..n {
        if terrain[c] != Terrain::Lake || seen[c] {
            continue;
        }
        let mut cells = vec![c];
        seen[c] = true;
        let mut i = 0;
        while i < cells.len() {
            for m in neighbours(cells[i]) {
                if terrain[m] == Terrain::Lake && !seen[m] {
                    seen[m] = true;
                    cells.push(m);
                }
            }
            i += 1;
        }
        if (cells.len() as u32) < LAKE_MIN_AREA {
            continue;
        }
        // 重心：x 以第一格為基準展開，避開東西接縫。
        let x0 = (c % w) as f32;
        let unwrap = |k: usize| {
            let dx = (k % w) as f32 - x0;
            let wf = w as f32;
            (x0 + dx - wf * (dx / wf).round(), (k / w) as f32)
        };
        let count = cells.len() as f32;
        let (mx, my) = cells.iter().fold((0.0f32, 0.0f32), |a, &k| {
            let p = unwrap(k);
            (a.0 + p.0 / count, a.1 + p.1 / count)
        });
        let mut best = cells[0];
        let mut best_d = f32::MAX;
        for &k in &cells {
            let p = unwrap(k);
            let d = (p.0 - mx).powi(2) + (p.1 - my).powi(2);
            if d < best_d || (d == best_d && k < best) {
                best = k;
                best_d = d;
            }
        }
        for &k in &cells {
            owner[k] = waters.len() as u32;
        }
        waters.push((WaterKind::Lake, best as u32, cells.len() as u32));
    }

    // 依標記格升冪排列，owner 跟著換成排序後的位置。
    let mut order: Vec<usize> = (0..waters.len()).collect();
    order.sort_by_key(|&i| waters[i].1);
    let mut place = vec![0u32; waters.len()];
    for (to, &from) in order.iter().enumerate() {
        place[from] = to as u32;
    }
    for slot in owner.iter_mut().filter(|slot| **slot != u32::MAX) {
        *slot = place[*slot as usize];
    }
    Survey { waters: order.iter().map(|&i| waters[i]).collect(), owner }
}

/// SplitMix64 的混合步驟。
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 以字元畫出的地形：`.` 海、`#` 平原、`o` 湖。
    fn grid(rows: &[&str]) -> (usize, usize, Vec<Terrain>) {
        let terrain = rows
            .iter()
            .flat_map(|row| row.chars())
            .map(|c| match c {
                '.' => Terrain::Sea,
                'o' => Terrain::Lake,
                _ => Terrain::Plains,
            })
            .collect();
        (rows[0].chars().count(), rows.len(), terrain)
    }

    /// 兩片海（各 4×5 格）隔著陸地，陸地上一個湖；南邊兩列是陸地。
    const ORIGIN: [&str; 9] = [
        "####################",
        "#....#####....######",
        "#....#####....###oo#",
        "#....#####....###oo#",
        "#....#####....######",
        "#....#####....######",
        "####################",
        "####################",
        "####################",
    ];

    fn names(waters: &[NamedWater]) -> Vec<&str> {
        waters.iter().map(|water| water.name.as_str()).collect()
    }

    #[test]
    fn unchanged_land_keeps_every_name() {
        let (w, h, terrain) = grid(&ORIGIN);
        let origin = survey(w, h, &terrain);
        assert_eq!(origin.waters.len(), 3, "兩片海與一個湖");
        assert_eq!(name_since(7, &origin, &origin), name_fresh(7, &origin));
    }

    #[test]
    fn a_water_that_is_still_there_keeps_its_name_and_a_new_one_gets_a_new_name() {
        let (w, h, terrain) = grid(&ORIGIN);
        let origin = survey(w, h, &terrain);
        let before = name_fresh(7, &origin);
        // 左邊的海往東淹過去幾格，湖乾了，南邊另外淹出一片新的海。
        let (_, _, changed) = grid(&[
            "####################",
            "#......###....######",
            "#......###....######",
            "#......###....######",
            "#....#####....######",
            "#....#####....######",
            "####################",
            "########......######",
            "########......######",
        ]);
        let after = name_since(7, &origin, &survey(w, h, &changed));
        assert_eq!(after.len(), 3, "兩片舊海與一片新海");
        let old = names(&before);
        let seas: Vec<&str> = before.iter().filter(|water| water.kind != WaterKind::Lake).map(|water| water.name.as_str()).collect();
        for sea in &seas {
            assert!(names(&after).contains(sea), "{sea} 還在，名字沿用");
        }
        assert_eq!(names(&after).iter().filter(|name| !old.contains(name)).count(), 1, "新海另取一個名");
        let lake = before.iter().find(|water| water.kind == WaterKind::Lake).expect("湖");
        assert!(!names(&after).contains(&lake.name.as_str()), "乾掉的湖名字消失");
        let stems: BTreeSet<String> = after.iter().map(|water| water.name.chars().take(water.name.chars().count() - 1).collect()).collect();
        assert_eq!(stems.len(), after.len(), "名（不含稱呼）不重複");
    }

    #[test]
    fn two_seas_that_join_keep_the_name_of_the_larger() {
        let (w, h, terrain) = grid(&[
            "####################",
            "#.....####....######",
            "#.....####....######",
            "#.....####....######",
            "#.....####....######",
            "####################",
        ]);
        let origin = survey(w, h, &terrain);
        let before = name_fresh(11, &origin);
        assert_eq!(before.len(), 2);
        let larger = before.iter().max_by_key(|water| water.area).expect("海");
        let (_, _, joined) = grid(&[
            "####################",
            "#.............######",
            "#.............######",
            "#.............######",
            "#.............######",
            "####################",
        ]);
        let after = name_since(11, &origin, &survey(w, h, &joined));
        assert_eq!(names(&after), vec![larger.name.as_str()]);
    }
}
