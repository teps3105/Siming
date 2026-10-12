//! GDD 3.4 入道（12 歲）與 4.6 開天直接定身分。

use rand::Rng;
use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::schedule::Round;
use crate::sim::World;
use crate::types::ProvinceId;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Door {
    Tao,
    Buddha,
    Ru,
    Demon,
}

/// 宗門規模上限（GDD 3.5）：`min(40, 5 + 聲望 / 20)`。
pub fn sect_capacity(prestige: f32) -> usize {
    ((SECT_SIZE_BASE as f32 + prestige / SECT_SIZE_PRESTIGE_DIV).floor() as u32).min(SECT_SIZE_MAX) as usize
}

/// 濁氣（GDD 2.4）：`clamp(業氣 − 0.5·功德 − 0.5·正氣, 0, 1)`。
fn turbid(w: &World, province: ProvinceId) -> f32 {
    w.provinces
        .get(province as usize)
        .map_or(0.0, |p| (p.karma_qi - TURBID_MIX_MERIT * p.merit - TURBID_MIX_RIGHT * p.righteous).clamp(0.0, 1.0))
}

/// 本州或陸鄰有山門的宗門中，聲望最高者（同值取 ID 小者）。
fn nearest_sect(w: &World, home: ProvinceId) -> Option<PowerId> {
    let nb: &[ProvinceId] = w.provinces.get(home as usize).map_or(&[], |p| p.neighbors.as_slice());
    let mut best: Option<(f32, PowerId)> = None;
    for p in &w.powers {
        if !matches!(p.kind, PowerKind::Sect(_)) || p.fallen.is_some() {
            continue;
        }
        if p.seat_province != home && !nb.contains(&p.seat_province) {
            continue;
        }
        if best.is_none_or(|(bp, _)| p.prestige > bp) {
            best = Some((p.prestige, p.id));
        }
    }
    best.map(|(_, id)| id)
}

/// GDD 3.4 入道判定：把有名之人從 `Child`（或開天時的新人）定為某個身分，回傳新身分。
///
/// 1. 可入之門：道（根骨 ≥ 3）、釋（釋道緣 ≥ 0.2）、儒（悟性 ≥ 3）、魔（本州濁氣 > 0.5 或帶修羅印記）；
///    依魂的道緣加權抽樣（全為 0 時等機率）。
/// 2. 道：本州或陸鄰有宗門 → 宗門弟子（滿員 → 散修）；否則散修。釋：M2 沒有寺院 → 遊方僧。
///    儒：M2 沒有書院 → 散儒。魔：魔修（M2 無魔門，不屬任何勢力）。
/// 3. 都不符合：M2 沒有王朝，一律散人（M3 起王朝州郡且才略 ≥ 6 → 武將候補）。
///
/// 入道成為修士時壽元改為 `max(現有, 煉氣壽元)`；僧人壽元 + 10。此函式不寫事件，
/// 不建立師徒（拜師見 3.5，由 dao 的招收步驟處理）。恆消耗 People 子流 1 次抽樣。
pub fn enter_path(w: &mut World, pid: PersonId) -> Role {
    let (attrs, home, soul_id) = {
        let p = &w.persons[pid as usize];
        (p.attrs, p.home, p.soul)
    };
    let (aff, asura) = {
        let s = &w.souls[soul_id as usize];
        (s.affinity, s.marks.has(SoulMarks::ASURA))
    };
    let mut doors: Vec<(Door, f32)> = Vec::new();
    if attrs.root >= ENTRY_REQ_TAO_ROOT {
        doors.push((Door::Tao, aff[Teaching::Tao as usize]));
    }
    if aff[Teaching::Buddha as usize] >= ENTRY_REQ_BUDDHA_AFFINITY {
        doors.push((Door::Buddha, aff[Teaching::Buddha as usize]));
    }
    if attrs.insight >= ENTRY_REQ_RU_INSIGHT {
        doors.push((Door::Ru, aff[Teaching::Ru as usize]));
    }
    if turbid(w, home) > ENTRY_REQ_DEMON_TURBID || asura {
        doors.push((Door::Demon, aff[Teaching::Demon as usize]));
    }
    let r = w.rng.people.gen::<f32>();
    let door = if doors.is_empty() {
        None
    } else {
        let total: f32 = doors.iter().map(|d| d.1.max(0.0)).sum();
        if total <= 0.0 {
            Some(doors[((r * doors.len() as f32) as usize).min(doors.len() - 1)].0)
        } else {
            let mut x = r * total;
            let mut chosen = doors[doors.len() - 1].0;
            for &(d, wt) in &doors {
                let wt = wt.max(0.0);
                if x < wt {
                    chosen = d;
                    break;
                }
                x -= wt;
            }
            Some(chosen)
        }
    };

    let cult = PathState::Cultivator { realm: 0, exp: 0.0, last_attempt: None };
    let base_life = REALM_LIFESPAN[0] as u16;
    let (role, path, power, lifespan_to) = match door {
        Some(Door::Tao) => {
            let sect = nearest_sect(w, home).filter(|&id| {
                let s = &w.powers[id as usize];
                s.members.len() < sect_capacity(s.prestige)
            });
            (Role::Cultivator, cult, sect, Some(base_life))
        }
        Some(Door::Buddha) => (Role::Monk, PathState::Monk { fruit: 0, practice: 0.0 }, None, None),
        Some(Door::Ru) => (Role::Scholar, PathState::Scholar { virtue: 0.0, deed: 0.0, word: 0.0 }, None, None),
        Some(Door::Demon) => (Role::Demonic, cult, None, Some(base_life)),
        // GDD 3.4：州郡屬於王朝且才略 ≥ 6 → 武將候補；否則散人。
        None if w.dynasty.owner_of(home).is_some() && attrs.talent >= GENERAL_CANDIDATE_TALENT => {
            (Role::GeneralCandidate, PathState::Mortal, None, None)
        }
        None => (Role::Commoner, PathState::Mortal, None, None),
    };
    if let Some(id) = power {
        w.powers[id as usize].members.push(pid);
    }
    let p = &mut w.persons[pid as usize];
    p.role = role;
    p.path = path;
    p.power = power;
    if let Some(l) = lifespan_to {
        p.lifespan = p.lifespan.max(l);
    }
    if role == Role::Monk {
        p.lifespan += MONK_LIFESPAN_BONUS as u16;
    }
    role
}

/// GDD 6.3 步驟 12：年齡 ≥ 12 且身分為 `Child` 者入道，寫入 `ComeOfAge` 事件（因：出生事件）。
///
/// 逐人判定：年齡由出生月起算，滿 12 歲後在自己的判定月入道（可能晚於 12 歲生日至多 11 個月）；
/// 年中才滿 12 歲者等到下一個自己的判定月。`Sim::step` 每月呼叫。
pub fn monthly_coming_of_age(w: &mut World) {
    coming_of_age_round(w, Round::at(w.tick));
}

/// 同 [`monthly_coming_of_age`]，但一次判定全部人（整年份；測試與工具用）。
pub fn yearly_coming_of_age(w: &mut World) {
    coming_of_age_round(w, Round::All);
}

fn coming_of_age_round(w: &mut World, round: Round) {
    let tick = w.tick;
    for i in w.round_alive(round).into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !p.alive() || p.role != Role::Child || p.age(tick) < COMING_OF_AGE_YEARS {
            continue;
        }
        let pid = p.id;
        let role = enter_path(w, pid);
        let p = &w.persons[i];
        let mut e = NewEvent::new(EventKind::ComeOfAge, 1, EventArgs::Role { role });
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
}
