//! GDD 3.5 遊歷：出發（6.3 步驟 11）與返回（步驟 3）。

use rand::Rng;
use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::schedule::Round;
use crate::sim::World;
use crate::types::ProvinceId;

/// 沿陸鄰 `TRAVEL_RANGE_STEPS` 步內（不含自身）的州郡，依 ID 升冪。
pub fn travel_candidates(w: &World, from: ProvinceId) -> Vec<ProvinceId> {
    let n = w.provinces.len();
    let mut dist = vec![u32::MAX; n];
    let Some(d0) = dist.get_mut(from as usize) else { return Vec::new() };
    *d0 = 0;
    let mut frontier = vec![from];
    for step in 1..=TRAVEL_RANGE_STEPS {
        let mut next = Vec::new();
        for &p in &frontier {
            for &q in &w.provinces[p as usize].neighbors {
                if dist[q as usize] == u32::MAX {
                    dist[q as usize] = step;
                    next.push(q);
                }
            }
        }
        frontier = next;
    }
    (0..n as ProvinceId).filter(|&i| i != from && dist[i as usize] != u32::MAX).collect()
}

fn random_cell(w: &mut World, province: ProvinceId) -> u32 {
    let Some(p) = w.provinces.get(province as usize) else { return 0 };
    if p.cells.is_empty() {
        return p.seat;
    }
    let k = w.rng.people.gen_range(0..p.cells.len());
    w.provinces[province as usize].cells[k]
}

fn travel_event(w: &mut World, pid: PersonId, from: ProvinceId, to: ProvinceId, cell: u32) {
    let p = &w.persons[pid as usize];
    let mut e = NewEvent::new(EventKind::Traveled, 1, EventArgs::Travel { from, to });
    e.province = Some(from);
    e.cell = Some(cell);
    e.souls = smallvec![p.soul];
    e.persons = smallvec![pid];
    if let Some(c) = p.last_event {
        e.causes = smallvec![c];
    }
    ledger::record(w, e);
}

/// GDD 3.5 遊歷（出發）：築基以上、非掌門的修士與散修，每年 10% 出外遊歷。
/// 目的地為 3 步內（陸鄰）隨機一州，權重 = 該州靈氣（全為 0 時等權）；一次移動到目的地，
/// 停留 12–60 個月（`travel_until`）後由 [`monthly_due`](super::monthly_due) 一次返回 home。
/// 每個符合資格者恆消耗 1 次機率抽樣；出發者另抽目的地、格、停留月數。
///
/// 逐人判定，輪到判定月的人才處理；`Sim::step` 每月呼叫。
pub fn monthly_travel(w: &mut World) {
    travel_round(w, Round::at(w.tick));
}

/// 同 [`monthly_travel`]，但一次判定全部人（整年份；測試與工具用）。
pub fn yearly_travel(w: &mut World) {
    travel_round(w, Round::All);
}

fn travel_round(w: &mut World, round: Round) {
    let tick = w.tick;
    for i in w.round_alive(round).into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !p.alive() || p.role != Role::Cultivator || p.travel_until.is_some() {
            continue;
        }
        let PathState::Cultivator { realm, .. } = p.path else { continue };
        if realm < TRAVEL_MIN_REALM {
            continue;
        }
        if p.power.and_then(|id| w.powers.get(id as usize)).is_some_and(|pw| pw.leader == Some(p.id)) {
            continue;
        }
        if w.rng.people.gen::<f32>() >= TRAVEL_CHANCE {
            continue;
        }
        let (pid, from, from_cell) = (p.id, p.province, p.cell);
        let cands = travel_candidates(w, from);
        if cands.is_empty() {
            continue;
        }
        let weights: Vec<f64> = cands.iter().map(|&c| w.provinces[c as usize].aura.max(0.0) as f64).collect();
        let total: f64 = weights.iter().sum();
        let r = w.rng.people.gen::<f64>();
        let to = if total <= 0.0 {
            cands[((r * cands.len() as f64) as usize).min(cands.len() - 1)]
        } else {
            let mut x = r * total;
            let mut chosen = *cands.last().unwrap();
            for (&c, &wt) in cands.iter().zip(&weights) {
                if x < wt {
                    chosen = c;
                    break;
                }
                x -= wt;
            }
            chosen
        };
        let cell = random_cell(w, to);
        let stay = w.rng.people.gen_range(TRAVEL_STAY_YEARS_MIN * 12..=TRAVEL_STAY_YEARS_MAX * 12) * crate::calendar::DAYS_PER_MONTH;
        let p = &mut w.persons[i];
        p.province = to;
        p.cell = cell;
        p.travel_until = Some(tick + stay);
        travel_event(w, pid, from, to, from_cell);
    }
}

/// 返回 home：回到山門格（所屬勢力在 home 州時），否則 home 州隨機一格。
pub(super) fn return_home(w: &mut World, i: usize) {
    let (pid, from, home, power) = {
        let p = &w.persons[i];
        (p.id, p.province, p.home, p.power)
    };
    let seat = power.and_then(|id| w.powers.get(id as usize)).filter(|pw| pw.seat_province == home).map(|pw| pw.seat_cell);
    let cell = match seat {
        Some(c) => c,
        None => random_cell(w, home),
    };
    let p = &mut w.persons[i];
    p.province = home;
    p.cell = cell;
    p.travel_until = None;
    travel_event(w, pid, from, home, cell);
}
