//! GDD 4.6 開天（分期預跑）（U8）。
//!
//! 開天當下天地間沒有人：各州人口為 0，沒有魂（天道除外）、有名之人、宗門、王朝。世界分兩段長出來：
//!
//! 1. **人口期**（`WorldParams::pop_years` 年）：[`build`] 依種子在陸地上灑下第一批聚落
//!    （同州郡生成的種子點規則：以宜居度加權的 Poisson-disk 取樣，亂數走 `rng.genesis`），之後 [`grow_population`]
//!    逐年只跑州郡層級——人口成長、饑荒、遷徙擴散、天地演變（每世結算，紀元自然更替）。不逐月、無有名之人。
//! 2. **人物期**（`WorldParams::prerun_years` 年）：接在人口期之後，逐月跑完整的 `Sim::step`（`World::prerun` 為真，
//!    事件寫入因果帳但不進 `Sim::drain_events`）。有名之人名額隨人口（[`crate::soul::notable_cap`]），
//!    由人間補魂長出來；修士、宗門、王朝都由既有規則在這段期間累積。
//!
//! 玩家接手時 `tick` = (人口期 + 人物期) × 12。時刻從開天的 0 往上數，沒有負的時刻。
//!
//! # 亂數
//!
//! 聚落落點只用 `genesis` 子流（`poisson_seeds`）；其餘由各規則使用自己的子流，兩段的銜接是確定性的。
//!
//! # 介面
//!
//! | 函式 | 用途 |
//! |---|---|
//! | [`run`]、[`run_with_progress`] | `Sim::new` 呼叫：整個開天（含進度回報） |
//! | [`build`]、[`grow_population`] | 測試與分段使用 |
//! | [`camera_start`] | 鏡頭初始位置（聲望最高的宗門山門） |

#[cfg(test)]
mod tests;

use crate::dao;
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::{is_shi, Sim, World};
use crate::soul;
use crate::types::{ProvinceId, WorldSize, NO_CONTINENT};
use crate::worldgen::grid::Sphere;
use crate::worldgen::provinces::{cell_habitability, poisson_seeds};

/// 開天的階段（進度回報用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// 灑下第一批聚落。
    Settle,
    /// 人口期（年）。
    Population,
    /// 人物期（月）。
    People,
}

/// 建立聚落、人口期、人物期。
pub fn run(sim: &mut Sim) {
    run_with_progress(sim, &mut |_, _| {});
}

/// 同 [`run`]，各階段回報進度 `progress(階段, 0..1)`；人口期與人物期約每百年回報一次。
pub fn run_with_progress(sim: &mut Sim, progress: &mut dyn FnMut(Stage, f32)) {
    progress(Stage::Settle, 0.0);
    build(sim.world_mut());
    progress(Stage::Settle, 1.0);
    let pop_years = sim.world().params.pop_years.min(GENESIS_POP_YEARS_MAX);
    grow_population(sim.world_mut(), pop_years, progress);
    let years = sim.world().params.prerun_years.min(GENESIS_PRERUN_YEARS_MAX);
    sim.world_mut().prerun = true;
    for k in 0..years * 12 {
        sim.step();
        if k % 1200 == 0 {
            progress(Stage::People, k as f32 / (years * 12).max(1) as f32);
        }
    }
    sim.world_mut().prerun = false;
    sim.drain_events();
    progress(Stage::People, 1.0);
}

fn size_index(size: WorldSize) -> usize {
    match size {
        WorldSize::Small => 0,
        WorldSize::Medium => 1,
        WorldSize::Large => 2,
    }
}

/// 4.6：天道魂、`Genesis` 事件、全州人口歸零、第一批聚落。
pub fn build(w: &mut World) {
    soul::ensure_heaven(w);
    w.tick = 0;
    ledger::record(w, NewEvent::new(EventKind::Genesis, 1, EventArgs::None));
    for p in &mut w.provinces {
        p.population = 0.0;
        p.city_level = 0;
    }
    settle(w);
}

/// 第一批聚落：陸地上（大洲內、屬於某州郡的格）以宜居度加權做 Poisson-disk 取樣
/// （重用州郡生成的 `poisson_seeds`），每個取樣點是 `GENESIS_SETTLE_POP` 人的小群體，落在該點所在的州郡。
fn settle(w: &mut World) {
    let (gw, gh) = (w.geo.w as usize, w.geo.h as usize);
    let sp = Sphere::new(gw, gh);
    let cand: Vec<u32> = (0..gw * gh)
        .filter(|&c| {
            w.geo.continent_of.get(c).is_some_and(|&k| k != NO_CONTINENT)
                && w.geo.province_of.get(c).is_some_and(|&p| (p as usize) < w.provinces.len())
        })
        .map(|c| c as u32)
        .collect();
    if cand.is_empty() {
        return;
    }
    let hab = cell_habitability(&sp, &w.geo.terrain, &w.geo.river);
    let land_area: f32 = cand.iter().map(|&c| sp.area(c as usize)).sum();
    let target = GENESIS_SETTLE_COUNT[size_index(w.params.size)] as usize;
    let seeds = poisson_seeds(&sp, &cand, &hab, land_area, target, GENESIS_SETTLE_SPACING, &mut w.rng.genesis);
    for c in seeds {
        let p = w.geo.province_of[c as usize];
        w.provinces[p as usize].population += GENESIS_SETTLE_POP;
        w.settlements.push((p, c));
    }
}

/// 人口期：逐年只跑州郡層級（成長、饑荒、遷徙、殘破恢復、城池等級），每世（30 年）做一次天地演變結算。
/// `years` 年後 `tick` = `years × 12`。不逐月、無有名之人。
pub fn grow_population(w: &mut World, years: u32, progress: &mut dyn FnMut(Stage, f32)) {
    for y in 0..years {
        w.tick += 12;
        let famines = crate::population::yearly_with(&mut w.provinces, &[]);
        ledger::record_famines(w, &famines);
        if is_shi(w.tick) {
            crate::evolution::shi_settle(w);
        }
        crate::cataclysm::yearly(w);
        ledger::prune(w);
        if y % 100 == 0 {
            progress(Stage::Population, y as f32 / years.max(1) as f32);
        }
    }
    progress(Stage::Population, 1.0);
}

/// 聲望最高的在世宗門（同值取 ID 小者）。
fn top_sect(w: &World) -> Option<PowerId> {
    let mut best: Option<(f32, PowerId)> = None;
    for p in &w.powers {
        if p.fallen.is_some() || !matches!(p.kind, PowerKind::Sect(_)) {
            continue;
        }
        let pr = dao::effective_prestige(p);
        if best.is_none_or(|(b, _)| pr > b) {
            best = Some((pr, p.id));
        }
    }
    best.map(|b| b.1)
}

/// 鏡頭初始位置（聲望最高的宗門山門）。沒有宗門時回傳 `None`。
pub fn camera_start(w: &World) -> Option<(ProvinceId, u32)> {
    top_sect(w).map(|id| (w.powers[id as usize].seat_province, w.powers[id as usize].seat_cell))
}
