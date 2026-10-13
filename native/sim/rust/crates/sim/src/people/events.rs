//! 個人規則的事件隊列排程與執行（GDD 6.3 事件驅動版）。
//!
//! | 規則 | 觸發方式 |
//! |---|---|
//! | 壽終 | 算得出時刻：出生日 + 壽元；壽元之後延長，觸發時發現未到就重排 |
//! | 入道 | 算得出時刻：出生日 + 12 年 + 0–359 日的隨機延遲 |
//! | 遊歷返回 | 算得出時刻：出發時排定 |
//! | 遊歷出發、婚配 | 每人一個指數等待時間（年率 [`TRAVEL_CHANCE`]、[`MARRY_CHANCE`]）；觸發時才檢查資格與找對象，不合則重抽 |
//! | 修煉、突破、心魔、斬妖／襲殺、預兆 | 年度結算：每人固定的日子（種子、編號雜湊），每 360 日一次 |

use rand::Rng;

use super::{marriage, path, travel};
use crate::calendar::{Day, DAYS_PER_YEAR};
use crate::model::*;
use crate::params::*;
use crate::queue::{Action, PersonAct};
use crate::sim::World;

/// 指數等待：年率 `per_year` 的卜瓦松過程，下次發生距今幾日（至少 1 日）。
fn exp_days(w: &mut World, per_year: f32) -> Day {
    let u = 1.0 - w.rng.people.gen::<f64>();
    let d = -u.ln() * DAYS_PER_YEAR as f64 / per_year as f64;
    d.ceil().clamp(1.0, 36_000_000.0) as Day
}

fn put(w: &mut World, day: Day, pid: PersonId, act: PersonAct) {
    w.queue.schedule(day, Action::Person(pid, act));
}

/// 人物出生時排定：壽終與入道。
pub fn arm_birth(w: &mut World, pid: PersonId) {
    let p = &w.persons[pid as usize];
    let born = p.born.max(0) as Day;
    let die = born.saturating_add(p.lifespan as Day * DAYS_PER_YEAR).max(w.tick + 1);
    let delay = w.rng.people.gen_range(0..DAYS_PER_YEAR);
    let come = (born + COMING_OF_AGE_YEARS * DAYS_PER_YEAR + delay).max(w.tick + 1);
    put(w, die, pid, PersonAct::Age);
    put(w, come, pid, PersonAct::ComeOfAge);
}

/// 入道後排定：年度結算（修士與魔修）、遊歷出發（修士）、婚配（非僧人）。
pub fn arm_adult(w: &mut World, pid: PersonId) {
    let p = &w.persons[pid as usize];
    let (role, cultivating) = (p.role, matches!(p.path, PathState::Cultivator { .. }));
    if cultivating {
        arm_year(w, pid);
    }
    if role == Role::Cultivator {
        arm_travel(w, pid);
    }
    if role != Role::Monk {
        arm_marry(w, pid);
    }
}

/// 下一個年度結算日：嚴格晚於今天、落在此人固定的那一天。
fn arm_year(w: &mut World, pid: PersonId) {
    let phase = crate::schedule::judge_day(w.params.seed, crate::schedule::Domain::Person, pid);
    let now = w.tick;
    let mut d = now - now % DAYS_PER_YEAR + phase;
    if d <= now {
        d += DAYS_PER_YEAR;
    }
    put(w, d, pid, PersonAct::Year);
}

/// 遊歷出發的下一次抽中。
pub fn arm_travel(w: &mut World, pid: PersonId) {
    let d = w.tick + exp_days(w, TRAVEL_CHANCE);
    put(w, d, pid, PersonAct::Travel);
}

/// 婚配的下一次抽中，從年滿 18 歲起算。同一人只有一個有效的婚配事件（`next_marry`）。
pub fn arm_marry(w: &mut World, pid: PersonId) {
    let p = &w.persons[pid as usize];
    let from = (p.born.max(0) as Day + MARRY_AGE * DAYS_PER_YEAR).max(w.tick);
    let d = from + exp_days(w, MARRY_CHANCE);
    w.persons[pid as usize].next_marry = Some(d);
    put(w, d, pid, PersonAct::Marry);
}

/// 執行一件個人事件（`Sim` 的隊列迴圈呼叫；`w.tick` 已設為事件日）。
pub fn fire(w: &mut World, pid: PersonId, act: PersonAct) {
    let day = w.tick;
    let i = pid as usize;
    if !w.persons[i].alive() {
        return;
    }
    match act {
        PersonAct::Age => {
            let p = &w.persons[i];
            if p.age(day) >= p.lifespan as u32 {
                super::die(w, pid, DeathCause::Age, None, &[]);
            } else {
                let due = (p.born.max(0) as Day + p.lifespan as Day * DAYS_PER_YEAR).max(day + 1);
                put(w, due, pid, PersonAct::Age);
            }
        }
        PersonAct::ComeOfAge => path::come_of_age_one(w, pid),
        PersonAct::Return => {
            if w.persons[i].travel_until.is_some_and(|t| t <= day) {
                travel::return_home(w, i);
            }
        }
        PersonAct::Travel => {
            if w.persons[i].role != Role::Cultivator {
                return;
            }
            if !(travel::travel_eligible(w, i) && travel::depart(w, i)) && w.persons[i].travel_until.is_none() {
                arm_travel(w, pid);
            }
        }
        PersonAct::Marry => {
            if w.persons[i].next_marry != Some(day) {
                return;
            }
            w.persons[i].next_marry = None;
            if !marriage::can_marry(w, pid) {
                return;
            }
            if !marriage::marry_now(w, pid) {
                arm_marry(w, pid);
            }
        }
        PersonAct::Year => {
            if !matches!(w.persons[i].path, PathState::Cultivator { .. }) {
                return;
            }
            crate::dao::person_year(w, pid);
            if w.persons[i].alive() {
                put(w, day + DAYS_PER_YEAR, pid, PersonAct::Year);
            }
        }
    }
}
