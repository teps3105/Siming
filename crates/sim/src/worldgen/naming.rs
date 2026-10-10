//! GDD 2.2 步驟 9 與 GDD 8.3 地名：州郡、江河、山脈、洞天福地、大洲、世界。
//!
//! 同一世界不重名：重抽最多 20 次，仍重複則加「新」「北」「南」等前綴。

use super::grid::{below, unit};
use crate::types::Terrain;
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// 重名時的最多重抽次數（8.3）。
const REDRAW_MAX: usize = crate::params::WG_NAME_REDRAW_MAX;
/// 重名重抽用盡後的前綴（8.3）。
const PREFIXES: [&str; 8] = ["新", "北", "南", "東", "西", "上", "下", "大"];
/// 州郡名首字改用方位字的機率。8.3 只寫「依主要地形或方位」，屬實作細節。
const DIRECTION_PROB: f32 = 0.25;
/// 沿岸格佔州郡面積達此比例時，首字取水系字庫。8.3 未列出，屬實作細節。
const WATER_COAST_FRAC: f32 = 0.4;
/// 面積前 25% 的州郡稱「州」（8.3）。
const BIG_PROVINCE_TOP: f32 = crate::params::WG_NAME_BIG_PROVINCE_TOP;
/// 方位字的「中」判定：治所與所屬陸塊中心的偏移（經度差占 W、緯度差占 H）均小於此值。
const CENTER_BAND: f32 = 0.04;
/// 三教字庫鍵，輪流順序固定：儒、釋、道（8.3 大洲命名）。
const TEACHINGS: [&str; 3] = ["ru", "shi", "dao"];

#[derive(Deserialize)]
struct Places {
    first_by_terrain: BTreeMap<String, Vec<String>>,
    #[allow(dead_code)]
    direction: Vec<String>,
    second: Vec<String>,
    province_suffix: BTreeMap<String, String>,
    river_names: Vec<String>,
    river_suffix: BTreeMap<String, String>,
    range_names: Vec<String>,
    range_suffix: Vec<String>,
    node_names: Vec<String>,
    node_suffix: BTreeMap<String, String>,
    world_names: Vec<String>,
    world_suffix: Vec<String>,
    continent_names: BTreeMap<String, Vec<String>>,
}

const DATA: &str = include_str!("../../data/places.zh-Hant.json");

/// 取名器：持有字庫與已用名稱集合（`BTreeSet`，GDD 6.5 規則 3）。
pub struct Namer {
    p: Places,
    used: BTreeSet<String>,
}

fn pick<'a>(rng: &mut ChaCha8Rng, v: &'a [String]) -> &'a str {
    &v[below(rng, v.len())]
}

/// 一個州郡命名所需的資訊。
pub struct ProvinceInfo {
    pub cells: usize,
    pub seat: u32,
    /// 所屬陸塊的中心格（方位字相對於此；無則用治所本身）。
    pub center: u32,
    pub city_level: u8,
    pub terrain_mix: [u32; Terrain::COUNT],
    pub coast_cells: u32,
}

/// GDD 8.3 大洲命名的狀態：各教字庫剩餘的名字（不放回）。
pub struct ContinentNamer {
    pools: Vec<Vec<String>>,
    isles: Vec<String>,
    start: usize,
}

impl Namer {
    pub fn new() -> Self {
        Self {
            p: serde_json::from_str(DATA).expect("places.zh-Hant.json"),
            used: BTreeSet::new(),
        }
    }

    /// 8.3：重名重抽最多 20 次；仍重複則加前綴。
    fn unique(
        &mut self,
        rng: &mut ChaCha8Rng,
        mut gen: impl FnMut(&mut ChaCha8Rng, &Places) -> String,
    ) -> String {
        let mut name = gen(rng, &self.p);
        for _ in 0..REDRAW_MAX {
            if !self.used.contains(&name) {
                break;
            }
            name = gen(rng, &self.p);
        }
        if self.used.contains(&name) {
            let base = name.clone();
            'outer: for p in PREFIXES {
                let cand = format!("{p}{base}");
                if !self.used.contains(&cand) {
                    name = cand;
                    break 'outer;
                }
            }
            let mut n = 2;
            while self.used.contains(&name) {
                name = format!("{base}{n}");
                n += 1;
            }
        }
        self.used.insert(name.clone());
        name
    }

    /// GDD 8.3 州郡：兩字 + 州（城池等級 ≥ 3 或面積前 25%）或郡；首字依主要地形或方位，次字通用。
    pub fn provinces(
        &mut self,
        rng: &mut ChaCha8Rng,
        infos: &[ProvinceInfo],
        w: usize,
        h: usize,
    ) -> Vec<String> {
        let mut areas: Vec<usize> = infos.iter().map(|i| i.cells).collect();
        areas.sort_unstable_by(|a, b| b.cmp(a));
        let top_n = ((infos.len() as f32 * BIG_PROVINCE_TOP).floor() as usize).max(1);
        let area_cut = areas.get(top_n - 1).copied().unwrap_or(usize::MAX);
        let mut out = Vec::with_capacity(infos.len());
        for info in infos {
            let big = info.city_level >= 3 || info.cells >= area_cut;
            let suffix_key = if big { "zhou" } else { "jun" };
            let group = dominant_group(info);
            let (sx, sy) = rel_pos(info.seat, info.center, w, h);
            let name = self.unique(rng, |rng, p| {
                let first: String = if unit(rng) < DIRECTION_PROB {
                    direction_char(sx, sy).to_string()
                } else {
                    pick(rng, &p.first_by_terrain[group]).to_string()
                };
                let second = pick(rng, &p.second);
                format!("{first}{second}{}", p.province_suffix[suffix_key])
            });
            out.push(name);
        }
        out
    }

    /// GDD 8.3 江河：一至兩字 + 江（3）、河（2）、水（1）。
    pub fn river(&mut self, rng: &mut ChaCha8Rng, level: u8) -> String {
        let key = level.to_string();
        self.unique(rng, |rng, p| {
            format!("{}{}", pick(rng, &p.river_names), p.river_suffix[&key])
        })
    }

    /// GDD 8.3 山脈：兩字 + 山、嶺、嶽（最高者）。
    pub fn range(&mut self, rng: &mut ChaCha8Rng, highest: bool) -> String {
        self.unique(rng, |rng, p| {
            let suffix = if highest {
                "嶽"
            } else {
                // 非最高者只用「山」「嶺」。
                let plain: Vec<&String> = p
                    .range_suffix
                    .iter()
                    .filter(|s| s.as_str() != "嶽")
                    .collect();
                plain[below(rng, plain.len())].as_str()
            };
            format!("{}{}", pick(rng, &p.range_names), suffix)
        })
    }

    /// GDD 8.3 洞天福地：兩字 + 大洞天（s=1.0）、洞天（0.75）、福地（0.5）。
    pub fn node(&mut self, rng: &mut ChaCha8Rng, strength: f32) -> String {
        let key = if strength >= 1.0 {
            "1.0"
        } else if strength >= 0.75 {
            "0.75"
        } else {
            "0.5"
        };
        self.unique(rng, |rng, p| {
            format!("{}{}", pick(rng, &p.node_names), p.node_suffix[key])
        })
    }

    /// GDD 8.3 大洲命名：起始教由隨機流決定，之後儒、釋、道輪流。
    pub fn continents_start(&self, rng: &mut ChaCha8Rng) -> ContinentNamer {
        ContinentNamer {
            pools: TEACHINGS
                .iter()
                .map(|k| self.p.continent_names[*k].clone())
                .collect(),
            isles: self.p.continent_names["dao_isles"].clone(),
            start: below(rng, TEACHINGS.len()),
        }
    }

    /// GDD 8.3 大洲命名：第 `i` 塊（依面積降冪）用第 `(起始 + i) mod 3` 教的字庫不放回抽取；
    /// `zhou` 者稱洲（字庫名已含洲），較小者：道教優先取三神山／五神山之名 + 島，否則字庫名首字 + 島；字庫用盡改用下一教。
    pub fn continent(
        &mut self,
        rng: &mut ChaCha8Rng,
        st: &mut ContinentNamer,
        i: usize,
        zhou: bool,
    ) -> String {
        let mut t = (st.start + i) % TEACHINGS.len();
        for _ in 0..TEACHINGS.len() {
            let is_dao = TEACHINGS[t] == "dao";
            if !zhou && is_dao && !st.isles.is_empty() {
                let k = below(rng, st.isles.len());
                let base = st.isles.remove(k);
                return self.unique(rng, |_, _| format!("{base}島"));
            }
            if !st.pools[t].is_empty() {
                let k = below(rng, st.pools[t].len());
                let base = st.pools[t].remove(k);
                let name = if zhou {
                    base
                } else {
                    format!("{}島", base.chars().next().unwrap_or('洲'))
                };
                return self.unique(rng, |_, _| name.clone());
            }
            t = (t + 1) % TEACHINGS.len();
        }
        // 三教字庫皆用盡：以世界名字庫補。
        self.unique(rng, |rng, p| {
            format!("{}{}", pick(rng, &p.world_names), if zhou { "洲" } else { "島" })
        })
    }

    /// GDD 8.3 世界：兩字 + 界、洲。
    pub fn world(&mut self, rng: &mut ChaCha8Rng) -> String {
        self.unique(rng, |rng, p| {
            format!(
                "{}{}",
                pick(rng, &p.world_names),
                pick(rng, &p.world_suffix)
            )
        })
    }
}

impl Default for Namer {
    fn default() -> Self {
        Self::new()
    }
}

/// 州郡的主要地形所屬字庫分組（`first_by_terrain` 的鍵）。
fn dominant_group(info: &ProvinceInfo) -> &'static str {
    if info.coast_cells as f32 >= WATER_COAST_FRAC * info.cells as f32 {
        return "water";
    }
    let m = &info.terrain_mix;
    let groups: [(&'static str, u32); 6] = [
        (
            "mountain",
            m[Terrain::Mountain as usize] + m[Terrain::Hills as usize],
        ),
        ("plains", m[Terrain::Plains as usize]),
        ("forest", m[Terrain::Forest as usize]),
        ("desert", m[Terrain::Desert as usize]),
        ("snow", m[Terrain::Snow as usize]),
        ("swamp", m[Terrain::Swamp as usize]),
    ];
    let mut best = groups[0];
    for g in &groups[1..] {
        if g.1 > best.1 {
            best = *g;
        }
    }
    best.0
}

/// 治所相對於陸塊中心的偏移：經度差（環繞取短邊，占 W）、緯度差（占 H）。
fn rel_pos(seat: u32, center: u32, w: usize, h: usize) -> (f32, f32) {
    let (sx, sy) = ((seat as usize % w) as i32, (seat as usize / w) as i32);
    let (cx, cy) = ((center as usize % w) as i32, (center as usize / w) as i32);
    let mut dx = sx - cx;
    if dx > w as i32 / 2 {
        dx -= w as i32;
    } else if dx < -(w as i32) / 2 {
        dx += w as i32;
    }
    (dx as f32 / w as f32, (sy - cy) as f32 / h as f32)
}

/// 依治所相對於所屬陸塊中心的方位取方位字（北在上）。
fn direction_char(dx: f32, dy: f32) -> &'static str {
    if dx.abs() < CENTER_BAND && dy.abs() < CENTER_BAND {
        "中"
    } else if dx.abs() >= dy.abs() {
        if dx > 0.0 {
            "東"
        } else {
            "西"
        }
    } else if dy > 0.0 {
        "南"
    } else {
        "北"
    }
}
