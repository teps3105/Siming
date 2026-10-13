//! GDD 3.5 心魔、斬妖除魔；3.10 魔修（入魔、襲殺）。

use rand::Rng;
use smallvec::smallvec;

use super::{cultivate, kill};
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::people::{combat_power, combat_power_as_target};
use crate::sim::World;
use crate::types::ProvinceId;

/// 7.2 `FellToDemon` 重要度 3。
const FELL_IMPORTANCE: u8 = 3;

/// GDD 3.5 心魔：`(S − G − 80) / 1000`（≤ 0 為 0）；修羅印記 × 2。
pub fn heart_demon_chance(sin: f32, merit: f32, asura: bool) -> f32 {
    let c = ((sin - merit - HEART_DEMON_OFFSET) / HEART_DEMON_DIV).max(0.0);
    if asura {
        c * HEART_DEMON_ASURA_MULT
    } else {
        c
    }
}

/// 脫離宗門：自成員移除、`power` 清空、解除師徒（`leader` 欄位保持原值，由繼任處理偵測）。
pub(super) fn leave_sect(w: &mut World, pid: PersonId) {
    let (power, master, disciples) = {
        let p = &w.persons[pid as usize];
        (p.power, p.master, p.disciples.clone())
    };
    if let Some(pw) = power.and_then(|id| w.powers.get_mut(id as usize)) {
        pw.members.retain(|&m| m != pid);
    }
    if let Some(m) = master.and_then(|id| w.persons.get_mut(id as usize)) {
        m.disciples.retain(|&d| d != pid);
    }
    for d in disciples {
        if let Some(d) = w.persons.get_mut(d as usize) {
            if d.master == Some(pid) {
                d.master = None;
            }
        }
    }
    let p = &mut w.persons[pid as usize];
    p.power = None;
    p.master = None;
    p.disciples.clear();
}

/// GDD 3.5 入魔：身分轉為魔修、離開宗門、魂加「曾入魔」印記、寫入 `FellToDemon`。
///
/// M2 沒有人對宗門的關係值，「原宗門對其關係 −100」不實作（見 3.5 註記）。
pub fn fall_to_demon(w: &mut World, pid: PersonId) {
    fall_to_demon_why(w, pid, Why::SinDemon);
}

/// 同 [`fall_to_demon`]，並記錄入魔的成因。
pub fn fall_to_demon_why(w: &mut World, pid: PersonId, why: Why) {
    let Some(p) = w.persons.get(pid as usize) else { return };
    if !p.alive() || p.role == Role::Demonic {
        return;
    }
    let soul = p.soul;
    leave_sect(w, pid);
    w.persons[pid as usize].role = Role::Demonic;
    if let Some(s) = w.souls.get_mut(soul as usize) {
        s.marks.set(SoulMarks::FELL_TO_DEMON);
    }
    let p = &w.persons[pid as usize];
    let mut e = NewEvent::new(EventKind::FellToDemon, FELL_IMPORTANCE, EventArgs::Role { role: Role::Demonic });
    e.why = why;
    e.province = Some(p.province);
    e.cell = Some(p.cell);
    e.souls = smallvec![soul];
    e.persons = smallvec![pid];
    if let Some(c) = p.last_event {
        e.causes = smallvec![c];
    }
    ledger::record(w, e);
}

fn s_asura(w: &World, pid: PersonId) -> bool {
    w.souls.get(w.persons[pid as usize].soul as usize).is_some_and(|s| s.marks.has(SoulMarks::ASURA))
}

/// 每年心魔判定（修士，非魔修）。機率為 0 時不消耗亂數。回傳是否入魔。
pub(super) fn heart_demon_roll(w: &mut World, pid: PersonId) -> bool {
    let p = &w.persons[pid as usize];
    let Some(s) = w.souls.get(p.soul as usize) else { return false };
    let c = heart_demon_chance(s.sin, s.merit, s.marks.has(SoulMarks::ASURA));
    if c <= 0.0 || w.rng.people.gen::<f32>() >= c {
        return false;
    }
    let why = if s_asura(w, pid) { Why::PastLifeMark } else { Why::SinDemon };
    fall_to_demon_why(w, pid, why);
    true
}

/// 本州與陸鄰中仍符合 `keep` 的非幼年在世者，依 ID 升冪（不含 `me`）。走世界的州郡索引，不逐一掃在世名單。
fn near(w: &mut World, province: ProvinceId, me: PersonId, keep: impl Fn(&Person) -> bool) -> Vec<PersonId> {
    let mut v = w.near_alive(province);
    v.retain(|&id| {
        let q = &w.persons[id as usize];
        id != me && q.role != Role::Child && keep(q)
    });
    v
}

/// 雙方決鬥：勝率 `r/(1+r)`，r = 攻方戰力 / 守方戰力；敗者死亡，業依 3.1。
/// 回傳 `(勝者, 敗者)`。
fn duel(w: &mut World, attacker: PersonId, target: PersonId, why: Why) -> Option<(PersonId, PersonId)> {
    let pa = combat_power(&w.persons[attacker as usize]);
    let pt = combat_power_as_target(&w.persons[target as usize]);
    let r = if pt > 0.0 { pa / pt } else { f32::MAX };
    let win = if r.is_finite() { r / (1.0 + r) } else { 1.0 };
    let (winner, loser) = if w.rng.people.gen::<f32>() < win { (attacker, target) } else { (target, attacker) };
    let loser_power = combat_power(&w.persons[loser as usize]);
    if !kill(w, winner, loser, DeathCause::Killed, None, why) {
        return None;
    }
    if w.persons[winner as usize].role == Role::Demonic {
        cultivate::add_exp(w, winner, loser_power * DEMON_KILL_EXP);
    }
    Some((winner, loser))
}

fn pick(w: &mut World, cands: &[PersonId]) -> PersonId {
    cands[w.rng.people.gen_range(0..cands.len())]
}

/// GDD 3.5 斬妖除魔：金丹以上、淨業力 ≥ 0 的修士，每年 10% 尋找本州或陸鄰的魔修（M2 無妖），有則挑戰一名。
/// 恆對符合資格者消耗 1 次機率抽樣；有候選者另抽選目標與勝負。
pub(super) fn hunt(w: &mut World, pid: PersonId) {
    let p = &w.persons[pid as usize];
    if !p.alive() {
        return;
    }
    let PathState::Cultivator { realm, .. } = p.path else { return };
    if realm < HUNT_MIN_REALM {
        return;
    }
    if w.souls.get(p.soul as usize).is_none_or(|s| s.net_karma() < HUNT_MIN_NET_KARMA) {
        return;
    }
    if w.rng.people.gen::<f32>() >= HUNT_CHANCE {
        return;
    }
    let prov = w.persons[pid as usize].province;
    let cands = near(w, prov, pid, |q| q.role == Role::Demonic);
    if cands.is_empty() {
        return;
    }
    let target = pick(w, &cands);
    duel(w, pid, target, Why::SlayDemon);
}

/// GDD 3.10 魔修襲殺：每年以 `戾氣 × 0.02` 機率襲殺本州或陸鄰一名有名之人（幼年者除外）。
pub(super) fn assassinate(w: &mut World, pid: PersonId) {
    let p = &w.persons[pid as usize];
    if !p.alive() {
        return;
    }
    let chance = p.attrs.ferocity * DEMON_ASSASSINATE_COEF;
    if chance <= 0.0 || w.rng.people.gen::<f32>() >= chance {
        return;
    }
    let prov = w.persons[pid as usize].province;
    let cands = near(w, prov, pid, |q| q.role != Role::Child);
    if cands.is_empty() {
        return;
    }
    let target = pick(w, &cands);
    duel(w, pid, target, Why::DemonAssassin);
}
