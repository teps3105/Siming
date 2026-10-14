//! GDD 3.1 魂力：每次轉世與在惡道的年數消耗魂力，耗盡則魂散。
//!
//! 消耗 = 基本消耗 ÷ 減緩係數。減緩係數 = 1 + 修為項 + 功德項 + 宿慧項（見 [`ease`]），
//! 修為高、功德多、宿慧深者消耗慢。純函式，不碰亂數；魂散本身寫因果帳並斷宿緣（[`dissolve`]）。

use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;

/// 修為階 0..9：歷世最高境界（+1）、果位（+1，每階折 2）、三不朽總和（每 40 為一階）取最大。
pub fn attainment_rank(soul: &Soul) -> f32 {
    let best = |f: fn(&Achievements) -> f32| f(&soul.best_prev).max(f(&soul.best_now));
    let realm = best(|a| a.realm.map_or(0.0, |r| r as f32 + 1.0));
    let fruit = best(|a| a.fruit.map_or(0.0, |f| (f as f32 + 1.0) * 2.0));
    let imm = best(|a| a.immortality / SOUL_EASE_IMMORTAL_DIV);
    realm.max(fruit).max(imm).clamp(0.0, 9.0)
}

/// 減緩係數 ≥ 1：`1 + 修為項 + 功德項 + 宿慧項`。修為項隨階數平方成長，功德項隨 ln 成長，宿慧項到 100 封頂。
pub fn ease(soul: &Soul) -> f32 {
    let rank = attainment_rank(soul);
    1.0 + SOUL_EASE_RANK * rank * rank
        + SOUL_EASE_MERIT * (1.0 + soul.merit.max(0.0) / SOUL_EASE_MERIT_SCALE).ln()
        + SOUL_EASE_WISDOM * (soul.wisdom / SOUL_EASE_WISDOM_FULL).clamp(0.0, 1.0)
}

/// 一次轉世消耗的魂力。
pub fn rebirth_cost(soul: &Soul) -> f32 {
    SOUL_REBIRTH_COST / ease(soul)
}

/// 在惡道（畜生、餓鬼、地獄）停留 `years` 年消耗的魂力；天道、人道、阿修羅道不消耗。
pub fn realm_cost(soul: &Soul, path: SixPath, years: f32) -> f32 {
    let rate = match path {
        SixPath::Beast => SOUL_BEAST_COST_PER_YEAR,
        SixPath::Hungry => SOUL_HUNGRY_COST_PER_YEAR,
        SixPath::Hell => SOUL_HELL_COST_PER_YEAR,
        SixPath::Heaven | SixPath::Human | SixPath::Asura => return 0.0,
    };
    rate * years / ease(soul)
}

/// 魂力夠不夠再轉世一次（扣完仍 > 0）。
pub fn can_rebirth(soul: &Soul) -> bool {
    soul.power - rebirth_cost(soul) > 0.0
}

/// 魂散（GDD 3.1）：狀態改為 `Dissolved`，未了的宿緣線斷（強度歸零、記 `severed`，不算了結），
/// 寫入一筆 `SoulDissolved`，以最後一世的最近事件為因。魂已散、已超脫或是天道則不做事。
pub fn dissolve(w: &mut World, sid: SoulId) {
    let tick = w.tick;
    let Some(soul) = w.souls.get(sid as usize) else { return };
    if sid == HEAVEN_SOUL || matches!(soul.state, SoulState::Dissolved { .. } | SoulState::Liberated { .. }) {
        return;
    }
    let last_life = soul.lives.last().copied();
    let cause = last_life.and_then(|p| w.persons.get(p as usize)).and_then(|p| p.last_event);
    let place = soul.last_place;
    let age_years = tick.saturating_sub(soul.born) / crate::calendar::DAYS_PER_YEAR;
    let rebirths = soul.rebirths();
    let open: Vec<ThreadId> =
        soul.threads.iter().copied().filter(|&t| w.threads.get(t as usize).is_some_and(crate::thread::is_open)).collect();
    // 重要度：有強度 ≥ 50 的未了宿緣、或修為達化神（境界 ≥ 4）者 3；其餘 2。
    let strong = open.iter().any(|&t| w.threads[t as usize].strength >= 50.0);
    let high = soul.best_prev.realm.max(soul.best_now.realm).is_some_and(|r| r >= 4);
    let importance = if strong || high { 3 } else { 2 };
    let mut e = NewEvent::new(
        EventKind::SoulDissolved,
        importance,
        EventArgs::Dissolved { age_years, rebirths, severed: open.len() as u16 },
    );
    e.province = Some(place.0);
    e.cell = Some(place.1);
    e.souls = smallvec![sid];
    if let Some(p) = last_life {
        e.persons = smallvec![p];
    }
    if let Some(c) = cause {
        e.causes = smallvec![c];
    }
    let eid = ledger::record(w, e);
    for t in open {
        let th = &mut w.threads[t as usize];
        th.strength = 0.0;
        th.severed = Some((tick, eid));
    }
    let soul = &mut w.souls[sid as usize];
    soul.power = 0.0;
    soul.guide = None;
    soul.state = SoulState::Dissolved { tick };
    w.soul_vacancies += 1;
}
