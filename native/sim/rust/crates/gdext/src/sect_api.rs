//! GDD 5.8 宗門卡的橋接查詢：一個宗門的卡面資料、門人、門史。
//! 全部唯讀；放在 `SimWorld` 的第二個 `impl` 區塊，不動既有的類別定義。

use godot::prelude::*;
use siming_sim::{calendar, dao, ledger, EventArgs, EventKind, PathState, Power, World};

use crate::SimWorld;

/// 友好、敵對各列前幾名。
const RELATION_TOP: usize = 5;
/// 境界數（煉氣至大乘）。
const REALMS: usize = 9;

fn gs(text: &str) -> GString {
    GString::from(text)
}

fn realm_of(w: &World, id: u32) -> i32 {
    match w.persons.get(id as usize).map(|p| &p.path) {
        Some(PathState::Cultivator { realm, .. }) => *realm as i32,
        _ => -1,
    }
}

fn sect_power(w: &World, id: i32) -> Option<&Power> {
    usize::try_from(id).ok().and_then(|i| w.powers.get(i)).filter(|p| dao::sect_of(p).is_some())
}

/// 某個時刻在曆法上的紀元名、紀元內年份與月份。
fn era_stamp(w: &World, tick: siming_sim::Tick) -> (String, i32, i32) {
    let (era, year) = calendar::era_year(tick, w.evolution.history_years, &w.eras);
    (era.name.clone(), year as i32, calendar::date(tick).month as i32)
}

fn relation_rows(w: &World, pw: &Power, friendly: bool) -> Array<VarDictionary> {
    let mut rows: Vec<(u32, i8)> = pw.relations.iter().map(|(&id, &v)| (id, v)).filter(|&(_, v)| if friendly { v > 0 } else { v < 0 }).collect();
    rows.sort_by_key(|&(id, v)| (if friendly { -(v as i32) } else { v as i32 }, id));
    let mut out = Array::new();
    for (id, value) in rows.into_iter().take(RELATION_TOP) {
        let target = w.powers.get(id as usize);
        let mut d = VarDictionary::new();
        d.set("id", id as i32);
        d.set("name", &gs(target.map_or("", |p| p.name.as_str())));
        d.set("value", value as i32);
        d.set("is_sect", target.is_some_and(|p| dao::sect_of(p).is_some()));
        d.set("alive", target.is_some_and(|p| p.fallen.is_none()));
        out.push(&d);
    }
    out
}

#[godot_api(secondary)]
impl SimWorld {
    /// 勢力編號對應的名字；沒有為空字串。
    #[func]
    fn power_name(&self, id: i32) -> GString {
        let Some(s) = self.sim.as_ref() else { return GString::new() };
        gs(usize::try_from(id).ok().and_then(|i| s.world().powers.get(i)).map_or("", |p| p.name.as_str()))
    }

    /// 宗門卡的卡面資料（已覆滅者也有）；沒有這個宗門回傳空字典。欄位：
    /// - `id`、`name`、`evil`、`alive`；山門 `province`／`province_name`／`pos`／`node`（洞天福地名，無為空字串）、
    ///   勢力範圍 `radius`（步）與 `range`（州郡編號）；`prestige`（有效聲望）；
    /// - `leader`（掌門的人物編號，無為 -1）、`leader_name`、`leader_realm`；
    /// - `member_count`、`members`（在世門人，依境界由高到低、同境界依編號；每筆 `id`、`name`、`realm`〔非修士為 -1〕、`is_leader`）、
    ///   `realm_counts`（九個境界各幾人）、`other_members`（非修士的門人數）；
    /// - `friends`、`foes`（友好、敵對前五名；每筆 `id`、`name`、`value`、`is_sect`、`alive`）；
    /// - `ascended`（飛升人數，宗門自己的計數）、`ascended_people`（飛升當時屬此宗、且事件還留著的人；每筆 `id`、`name`、`event`、`era`、`year`、`month`）；
    /// - 立派 `founded`（tick）、`founded_era`、`founded_year`，現在的紀元 `now_era`；
    /// - 覆滅 `fallen`（tick，在世為 -1）、`fallen_era`、`fallen_year`、`fallen_month`（在世為空字串、0、0）、
    ///   `fall_event`（覆滅事件編號，沒有為 -1）、`fall_cause`（第一個起因事件的種類代號，沒有為空字串）。
    #[func]
    fn sect(&self, id: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let w = s.world();
        let Some(pw) = sect_power(w, id) else { return d };
        let Some(sect) = dao::sect_of(pw) else { return d };
        let gw = w.geo.w as u32;
        d.set("id", pw.id as i32);
        d.set("name", &gs(&pw.name));
        d.set("evil", sect.evil);
        d.set("alive", pw.fallen.is_none());
        d.set("province", pw.seat_province as i32);
        d.set("province_name", &gs(w.provinces.get(pw.seat_province as usize).map_or("", |p| p.name.as_str())));
        d.set("pos", Vector2::new((pw.seat_cell % gw) as f32 + 0.5, (pw.seat_cell / gw) as f32 + 0.5));
        d.set("node", &gs(sect.seat_node.and_then(|n| w.geo.nodes.get(n as usize)).map_or("", |n| n.name.as_str())));
        d.set("radius", sect.radius as i32);
        let range: Vec<i32> = dao::sect_range(w, pw.id).iter().map(|&q| q as i32).collect();
        d.set("range", &PackedInt32Array::from(range.as_slice()));
        d.set("prestige", dao::effective_prestige(pw));
        let leader = pw.leader.and_then(|l| w.persons.get(l as usize));
        d.set("leader", leader.map_or(-1, |l| l.id as i32));
        d.set("leader_name", &gs(leader.map_or("", |l| l.name.as_str())));
        d.set("leader_realm", leader.map_or(-1, |l| realm_of(w, l.id)));

        let mut rows: Vec<(i32, u32)> = pw
            .members
            .iter()
            .filter(|&&m| w.persons.get(m as usize).is_some_and(|p| p.alive() && p.power == Some(pw.id)))
            .map(|&m| (realm_of(w, m), m))
            .collect();
        rows.sort_by_key(|&(realm, m)| (std::cmp::Reverse(realm), m));
        let mut realm_counts = [0i32; REALMS];
        let mut other = 0;
        let mut members = Array::<VarDictionary>::new();
        for &(realm, m) in &rows {
            match usize::try_from(realm).ok().filter(|&r| r < REALMS) {
                Some(r) => realm_counts[r] += 1,
                None => other += 1,
            }
            let mut row = VarDictionary::new();
            row.set("id", m as i32);
            row.set("name", &gs(&w.persons[m as usize].name));
            row.set("realm", realm);
            row.set("is_leader", pw.leader == Some(m));
            members.push(&row);
        }
        d.set("member_count", rows.len() as i32);
        d.set("members", &members);
        d.set("realm_counts", &PackedInt32Array::from(realm_counts.as_slice()));
        d.set("other_members", other);
        d.set("friends", &relation_rows(w, pw, true));
        d.set("foes", &relation_rows(w, pw, false));

        d.set("ascended", sect.ascended as i32);
        let mut ascended = Array::<VarDictionary>::new();
        for e in w.ledger.events.iter().filter(|e| e.kind == EventKind::Ascended && e.importance > 0 && e.powers.contains(&pw.id)) {
            let Some(person) = e.persons.first().and_then(|&p| w.persons.get(p as usize)) else { continue };
            let (era, year, month) = era_stamp(w, e.tick);
            let mut row = VarDictionary::new();
            row.set("id", person.id as i32);
            row.set("name", &gs(&person.name));
            row.set("event", e.id as i64);
            row.set("era", &gs(&era));
            row.set("year", year);
            row.set("month", month);
            ascended.push(&row);
        }
        d.set("ascended_people", &ascended);

        let (era, year, _) = era_stamp(w, pw.founded);
        d.set("founded", pw.founded as i64);
        d.set("founded_era", &gs(&era));
        d.set("founded_year", year);
        d.set("now_era", &gs(&era_stamp(w, w.tick).0));
        match pw.fallen {
            Some(tick) => {
                let (era, year, month) = era_stamp(w, tick);
                d.set("fallen", tick as i64);
                d.set("fallen_era", &gs(&era));
                d.set("fallen_year", year);
                d.set("fallen_month", month);
            }
            None => {
                d.set("fallen", -1i64);
                d.set("fallen_era", &gs(""));
                d.set("fallen_year", 0);
                d.set("fallen_month", 0);
            }
        }
        let fall = w.ledger.events.iter().find(|e| e.kind == EventKind::PowerFallen && e.importance > 0 && e.powers.first() == Some(&pw.id));
        d.set("fall_event", fall.map_or(-1, |e| e.id as i64));
        let cause = fall.and_then(|e| e.causes.first()).and_then(|&c| ledger::get(w, c));
        d.set("fall_cause", &gs(&cause.map_or(String::new(), |c| format!("{:?}", c.kind))));
        d
    }

    /// 宗門的門史，由舊到新：開派（`PowerFounded`／`SectSplit`）、繼位（`Succession`）、爭脈（`SectWar`）、覆滅（`PowerFallen`），
    /// 事件的 `powers` 含這個宗門者；修剪過的墓碑與王朝的繼位不算。每筆欄位同 `event`，另加 `era`（當時的紀元名）。
    /// 沒有這個宗門為空陣列。
    #[func]
    fn sect_history(&self, id: i32) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else { return out };
        let w = s.world();
        let Some(pw) = sect_power(w, id) else { return out };
        let mut events: Vec<&siming_sim::Event> = w
            .ledger
            .events
            .iter()
            .filter(|e| {
                e.importance > 0
                    && e.powers.contains(&pw.id)
                    && match e.kind {
                        EventKind::PowerFounded | EventKind::SectSplit | EventKind::SectWar | EventKind::PowerFallen => true,
                        EventKind::Succession => !matches!(e.args, EventArgs::Dynasty { .. }),
                        _ => false,
                    }
            })
            .collect();
        events.sort_by_key(|e| (e.tick, e.id));
        for e in events {
            let mut d = self.event_record(w, e, None);
            d.set("era", &gs(&era_stamp(w, e.tick).0));
            out.push(&d);
        }
        out
    }
}
