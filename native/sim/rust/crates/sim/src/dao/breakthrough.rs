//! GDD 3.5 突破、天劫、飛升、劫雲將聚預兆。

use rand::Rng;
use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::people;
use crate::sim::World;
use crate::types::Tick;

/// 大乘的境界索引（8）：此境界的突破即飛升劫。
pub const ASCENSION_REALM: u8 = (REALM_EXP.len() - 1) as u8;

/// 7.2 `Breakthrough` 重要度：新境界 1：1；2–3：2；4–5：3；6+：4。
fn breakthrough_importance(new_realm: u8) -> u8 {
    match new_realm {
        0..=1 => 1,
        2..=3 => 2,
        4..=5 => 3,
        _ => 4,
    }
}

/// 7.2 `Tribulation` 重要度：新境界 2–5：3；6+：4（飛升劫為 4）。
fn tribulation_importance(new_realm: u8) -> u8 {
    if new_realm >= 6 {
        4
    } else {
        3
    }
}

/// GDD 3.5 突破成功率：`B × (0.8 + 悟性 × 0.04) × (賜福期間 1.5)`，上限 0.95。
pub fn breakthrough_chance(realm: u8, insight: f32, blessed: bool) -> f32 {
    let mut c = REALM_BT[(realm as usize).min(REALM_BT.len() - 1)] * (BT_INSIGHT_BASE + insight * BT_INSIGHT_COEF);
    if blessed {
        c *= BLESS_BT_MULT;
    }
    c.min(BT_MAX)
}

/// GDD 3.5 突破失敗的走火身死機率：當前境界 ≥ 2 時 `0.1 + 0.03 × 當前境界`，否則 0。
pub fn fail_death_chance(realm: u8) -> f32 {
    if realm >= BT_FAIL_DEATH_MIN_REALM {
        BT_FAIL_DEATH_BASE + BT_FAIL_DEATH_PER_REALM * realm as f32
    } else {
        0.0
    }
}

/// GDD 3.5 渡劫存活率（新境界 n ≥ 2）：`0.92 × exp(−S / (60 + 40 × n))`。
pub fn tribulation_survival(new_realm: u8, sin: f32) -> f32 {
    TRIB_SURVIVE_BASE * (-sin / (TRIB_SURVIVE_DIV_BASE + TRIB_SURVIVE_DIV_PER_REALM * new_realm as f32)).exp()
}

/// GDD 3.5 飛升劫存活率：`0.80 × exp(−S / 300)`。
pub fn ascension_survival(sin: f32) -> f32 {
    TRIB_ASCEND_BASE * (-sin / TRIB_ASCEND_DIV).exp()
}

/// 冷卻已過：從未嘗試，或距上次嘗試 ≥ 36 個月。
pub fn cooldown_ready(last_attempt: Option<Tick>, tick: Tick) -> bool {
    last_attempt.is_none_or(|t| tick.saturating_sub(t) >= BT_COOLDOWN_YEARS * 12)
}

fn realm_state(p: &Person) -> Option<(u8, f32, Option<Tick>)> {
    match p.path {
        PathState::Cultivator { realm, exp, last_attempt } => Some((realm, exp, last_attempt)),
        _ => None,
    }
}

fn set_state(w: &mut World, pid: PersonId, realm: u8, exp: f32, last: Option<Tick>) {
    w.persons[pid as usize].path = PathState::Cultivator { realm, exp, last_attempt: last };
}

/// 達到突破條件：修為已滿且冷卻已過。
pub(super) fn eligible(p: &Person, tick: Tick) -> bool {
    realm_state(p).is_some_and(|(r, exp, last)| exp >= REALM_EXP[r as usize] && cooldown_ready(last, tick))
}

/// 一次突破嘗試（含天劫）。`forced` 時成功率與存活率皆為 1，且不消耗亂數。
/// 回傳人物是否仍在世（飛升亦算離世，回傳 `false`）。
pub(super) fn attempt(w: &mut World, pid: PersonId, forced: bool) -> bool {
    let tick = w.tick;
    let p = &w.persons[pid as usize];
    let Some((realm, exp, _)) = realm_state(p) else { return true };
    let blessed = p.blessed_until.is_some_and(|t| t > tick);
    let chance = breakthrough_chance(realm, p.attrs.insight, blessed);
    let success = forced || w.rng.people.gen::<f32>() < chance;
    if !success {
        set_state(w, pid, realm, exp * BT_FAIL_KEEP, Some(tick));
        let death = fail_death_chance(realm);
        if death > 0.0 && w.rng.people.gen::<f32>() < death {
            people::die(w, pid, DeathCause::BreakthroughFailed, None, &[]);
            return false;
        }
        return true;
    }
    if realm >= ASCENSION_REALM {
        return tribulation(w, pid, ASCENSION_REALM, true, forced);
    }
    let new = realm + 1;
    if new >= 2 {
        tribulation(w, pid, new, false, forced)
    } else {
        advance(w, pid, new);
        true
    }
}

/// 進入新境界：修為歸零、壽元取大、寫入 `Breakthrough`。
fn advance(w: &mut World, pid: PersonId, new: u8) {
    let tick = w.tick;
    set_state(w, pid, new, 0.0, Some(tick));
    let p = &mut w.persons[pid as usize];
    p.lifespan = p.lifespan.max(REALM_LIFESPAN[new as usize] as u16);
    // GDD 7.3 道號：金丹起取得，之後依新境界更新尊稱。
    people::assign_title(w, pid);
    let p = &w.persons[pid as usize];
    let mut e = NewEvent::new(EventKind::Breakthrough, breakthrough_importance(new), EventArgs::Realm { realm: new });
    e.province = Some(p.province);
    e.cell = Some(p.cell);
    e.souls = smallvec![p.soul];
    e.persons = smallvec![pid];
    if let Some(pw) = p.power {
        e.powers = smallvec![pw];
    }
    if let Some(c) = p.last_event {
        e.causes = smallvec![c];
    }
    ledger::record(w, e);
}

/// 天劫。`new` 為新境界（飛升劫時為 8）。回傳人物是否仍在世。
fn tribulation(w: &mut World, pid: PersonId, new: u8, ascension: bool, forced: bool) -> bool {
    let sin = w.souls[w.persons[pid as usize].soul as usize].sin;
    let survive = if ascension { ascension_survival(sin) } else { tribulation_survival(new, sin) };
    let outcome = if forced || w.rng.people.gen::<f32>() < survive {
        TribulationOutcome::Passed
    } else if w.rng.people.gen::<f32>() < TRIB_SHED_BODY_CHANCE {
        TribulationOutcome::ShedBody
    } else {
        TribulationOutcome::Perished
    };
    let p = &w.persons[pid as usize];
    let mut e = NewEvent::new(
        EventKind::Tribulation,
        tribulation_importance(new),
        EventArgs::Tribulation { realm: new, outcome },
    );
    e.province = Some(p.province);
    e.cell = Some(p.cell);
    e.souls = smallvec![p.soul];
    e.persons = smallvec![pid];
    if let Some(pw) = p.power {
        e.powers = smallvec![pw];
    }
    if let Some(c) = p.last_event {
        e.causes = smallvec![c];
    }
    let te = ledger::record(w, e);
    match outcome {
        TribulationOutcome::Passed if ascension => {
            let power = w.persons[pid as usize].power;
            people::die(w, pid, DeathCause::Ascended, Some(Liberation::Ascension), &[te]);
            if let Some(PowerKind::Sect(s)) = power.and_then(|id| w.powers.get_mut(id as usize)).map(|pw| &mut pw.kind)
            {
                s.ascended += 1;
            }
            false
        }
        TribulationOutcome::Passed => {
            advance(w, pid, new);
            true
        }
        TribulationOutcome::ShedBody => {
            people::die(w, pid, DeathCause::ShedBody, None, &[te]);
            false
        }
        TribulationOutcome::Perished => {
            people::die(w, pid, DeathCause::Tribulation, None, &[te]);
            false
        }
    }
}

/// GDD 6.3 步驟 11 末尾：境界 ≥ 元嬰、修為已滿、突破冷卻將在一年內結束者，寫入 `TribulationOmen`。
pub(super) fn yearly_omens(w: &mut World) {
    let tick = w.tick;
    for i in w.alive_snapshot().into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !p.alive() {
            continue;
        }
        let Some((realm, exp, last)) = realm_state(p) else { continue };
        if realm < OMEN_MIN_REALM || exp < REALM_EXP[realm as usize] || !cooldown_ready(last, tick + 12) {
            continue;
        }
        let mut e = NewEvent::new(EventKind::TribulationOmen, OMEN_IMPORTANCE, EventArgs::None);
        e.province = Some(p.province);
        e.cell = Some(p.cell);
        e.souls = smallvec![p.soul];
        e.persons = smallvec![p.id];
        if let Some(pw) = p.power {
            e.powers = smallvec![pw];
        }
        if let Some(c) = p.last_event {
            e.causes = smallvec![c];
        }
        ledger::record(w, e);
    }
}

/// 7.2 `TribulationOmen` 重要度 2。
const OMEN_IMPORTANCE: u8 = 2;
/// 3.5 預兆的最低境界：元嬰（3）。
const OMEN_MIN_REALM: u8 = 3;
