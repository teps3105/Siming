//! GDD 3.11、2.8 天地負荷與大劫。
//!
//! 天地是一個複雜系統：人口、在世有名之人、未了的宿緣線越多，負荷越高（[`load`]，只看模擬狀態，與機器耗時無關）。
//! 負荷逼近並超過承受上限（[`limit`]，依世界尺寸）時降下大劫：各州人口按業氣與殘破加權減少、
//! 一部分宿緣線被切斷（先切雙方都已不在世的、再切強度低的）、在世有名之人按機率身隕（不分境界），
//! 之後一段時間（`CATA_SLOW_YEARS`）新宿緣線不易長出。大劫開啟一個新紀元，劫終再開一個承平紀元。

use serde::{Deserialize, Serialize};

use crate::calendar::Era as CalendarEra;
use crate::era::{EraKind, Lexicon};
use crate::evolution::{year_of, WorldChange};
use crate::field::{self, FieldId};
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::WorldSize;

/// 大劫的狀態（進雜湊，附在世界最後）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Calamity {
    /// 歷次大劫降下的年份與當時負荷對上限的比（千分）。
    pub history: Vec<(u32, u32)>,
    /// 目前大劫結束的年份（`None` 表示不在劫中）。
    pub ends: Option<u32>,
    /// 此年之前新宿緣線不易長出。
    pub slow_until: u32,
}

fn size_index(size: WorldSize) -> usize {
    match size {
        WorldSize::Small => 0,
        WorldSize::Medium => 1,
        WorldSize::Large => 2,
    }
}

/// 承受上限（依世界尺寸，7.1 `CATA_LIMIT`）。
pub fn limit(w: &World) -> f64 {
    CATA_LIMIT[size_index(w.params.size)]
}

/// 天地負荷：人口（百萬）、在世有名之人（千）、未了宿緣線（萬）的加權和。
pub fn load(w: &World) -> f64 {
    let pop: f64 = w.provinces.iter().map(|p| p.population.max(0.0)).sum();
    let living = w.alive_ids.len() as f64;
    let open = w.threads.iter().filter(|t| crate::thread::is_open(t)).count() as f64;
    pop / 1.0e6 * CATA_W_POP + living / 1.0e3 * CATA_W_NOTABLE + open / 1.0e4 * CATA_W_THREAD
}

/// 新宿緣線在大劫之後的一段時間不易長出：確定性地（不耗亂數）以建線序號與時刻擲骰。
pub fn thread_allowed(w: &World) -> bool {
    let year = year_of(w.tick);
    if year >= w.calamity.slow_until {
        return true;
    }
    let left = f64::from(w.calamity.slow_until - year) / f64::from(CATA_SLOW_YEARS + CATA_DURATION_YEARS);
    let rate = 1.0 - CATA_SLOW_THREAD_CUT * left.min(1.0);
    let h = mix(w.params.seed
        ^ (w.threads.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (u64::from(w.tick / crate::calendar::DAYS_PER_MONTH) << 20));
    ((h >> 40) as f64 / (1u64 << 24) as f64) < rate
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn open_era_now(w: &mut World, year: u32, kind: EraKind, change: WorldChange) {
    let used: Vec<String> = w.eras.iter().map(|e| e.name.clone()).collect();
    let name = Lexicon::builtin().name(w.params.seed, w.eras.len() as u32, kind, &used);
    w.eras.push(CalendarEra { name, start_year: year + 1, kind });
    ledger::record(w, NewEvent::new(EventKind::WorldChanged, EVO_EVENT_IMPORTANCE, EventArgs::World { change, new_era: true }));
}

/// 每年一次：劫終收尾；負荷超過上限且距上次大劫夠久就降下大劫。
pub fn yearly(w: &mut World) {
    let year = year_of(w.tick);
    if w.calamity.ends.is_some_and(|e| year >= e) {
        w.calamity.ends = None;
        open_era_now(w, year, EraKind::Peace, WorldChange::CataclysmEnded);
    }
    if w.calamity.ends.is_some() {
        return;
    }
    if w.calamity.history.last().is_some_and(|&(y, _)| year < y + CATA_MIN_GAP_YEARS) {
        return;
    }
    let l = load(w);
    let cap = limit(w);
    if l < cap {
        return;
    }
    strike(w, year, l / cap);
}

fn strike(w: &mut World, year: u32, ratio: f64) {
    let sev = ratio.clamp(1.0, 1.5);
    w.calamity.history.push((year, (ratio * 1000.0) as u32));
    w.calamity.ends = Some(year + CATA_DURATION_YEARS);
    w.calamity.slow_until = year + CATA_DURATION_YEARS + CATA_SLOW_YEARS;
    open_era_now(w, year, EraKind::Cataclysm, WorldChange::CataclysmBegan);
    // 人口：按業氣與殘破加權。
    for p in &mut w.provinces {
        let loss = ((CATA_POP_LOSS + CATA_POP_LOSS_KARMA * f64::from(p.karma_qi) + CATA_POP_LOSS_DEVASTATION * f64::from(p.devastation))
            * sev)
            .clamp(0.0, 0.8);
        p.population *= 1.0 - loss;
        p.devastation = (p.devastation + 0.4).min(1.0);
        field::add_to(p, FieldId::KarmaQi, 0.05);
    }
    // 宿緣線：先切雙方都不在世的，再切強度低的。
    let alive = |w: &World, sid: SoulId| matches!(w.souls.get(sid as usize).map(|s| &s.state), Some(SoulState::Incarnate(_)));
    let mut open: Vec<(u8, u32, ThreadId)> = w
        .threads
        .iter()
        .filter(|t| crate::thread::is_open(t))
        .map(|t| (u8::from(alive(w, t.debtor)) + u8::from(alive(w, t.creditor)), (t.strength * 100.0) as u32, t.id))
        .collect();
    open.sort_unstable();
    let cut = (open.len() as f64 * CATA_THREAD_CUT) as usize;
    for &(_, _, tid) in open.iter().take(cut) {
        crate::thread::fade(w, tid);
    }
    // 在世有名之人：不分境界，按機率身隕。
    let ids: Vec<PersonId> = w.alive_ids.to_vec();
    for pid in ids {
        let h = mix(w.params.seed ^ (u64::from(year) << 32) ^ u64::from(pid) ^ 0xCA7A);
        if ((h >> 40) as f64 / (1u64 << 24) as f64) < CATA_DEATH * sev && w.persons.get(pid as usize).is_some_and(|p| p.alive()) {
            crate::people::die(w, pid, DeathCause::Kalpa, None, &[]);
        }
    }
}
