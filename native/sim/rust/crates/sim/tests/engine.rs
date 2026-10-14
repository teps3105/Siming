//! 事件驅動引擎骨幹：日曆、隊列、`advance_to`、`advance_until`、亂數子流（第 1 波 B 段）。
//!
//! 規則尚在月步進裡；這裡驗的是推進機制本身：不論怎麼切時間，結果與一個月一個月走相同，
//! 隊列事件在正確的日子執行，取消不會誤發。

use siming_sim::calendar::{DAYS_PER_MONTH, DAYS_PER_YEAR};
use siming_sim::queue::Action;
use siming_sim::sim::next_month_end;
use siming_sim::sim::Until;
use siming_sim::{EventKind, FastOutcome, FastStop, Sim, WorldParams};

fn quick(seed: u64) -> WorldParams {
    let mut p = WorldParams::new(seed);
    p.pop_years = 300;
    p.prerun_years = 60;
    p
}

fn world(rng: u64) -> Sim {
    Sim::new_seeded(&quick(7), Some(rng)).unwrap()
}

#[test]
fn step_is_advance_to_next_month_end() {
    let mut a = world(11);
    let mut b = world(11);
    assert_eq!(a.hash(), b.hash());
    for _ in 0..24 {
        a.step();
        let t = next_month_end(b.world().tick);
        b.advance_to(t);
    }
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.world().tick, a.world().tick);
}

#[test]
fn advancing_in_odd_slices_matches_month_steps() {
    let mut a = world(12);
    let mut b = world(12);
    let start = b.world().tick;
    for _ in 0..18 {
        a.step();
    }
    // 以不整齊的日數切片推進到同一天。
    let target = start + 18 * DAYS_PER_MONTH;
    let mut d = start;
    for k in [7u32, 1, 45, 100, 3] {
        d += k;
        b.advance_to(d);
    }
    b.advance_to(target);
    assert_eq!(b.world().tick, a.world().tick);
    assert_eq!(b.world().tick, target);
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn queued_events_fire_on_their_day_in_order() {
    let mut s = world(13);
    let d0 = s.world().tick;
    let q = &mut s.world_mut().queue;
    let a = q.schedule(d0 + 5, Action::Marker(1));
    let b = q.schedule(d0 + 40, Action::Marker(2));
    let c = q.schedule(d0 + 40, Action::Marker(3));
    q.cancel(b);
    s.advance_to(d0 + 4);
    assert_eq!(s.world().queue.fired, 0);
    assert!(s.world().queue.is_live(a));
    s.advance_to(d0 + 5);
    assert_eq!(s.world().queue.fired, 1);
    assert_eq!(s.world().tick, d0 + 5);
    s.advance_to(d0 + 39);
    assert_eq!(s.world().queue.fired, 1);
    s.advance_to(d0 + 40);
    assert_eq!(s.world().queue.fired, 2, "取消的 2 號不發，3 號發");
    assert!(!s.world().queue.is_live(c));
}

#[test]
fn an_event_in_the_same_day_as_month_end_runs_after_the_month_step() {
    let mut s = world(14);
    let end = next_month_end(s.world().tick);
    s.world_mut().queue.schedule(end, Action::Marker(1));
    s.advance_to(end);
    assert_eq!(s.world().queue.fired, 1);
    assert_eq!(s.world().tick, end);
}

#[test]
fn advance_until_years_advances_exactly_that_long() {
    let mut s = world(15);
    let d0 = s.world().tick;
    assert_eq!(s.advance_until(Until::Years(3), u32::MAX), None);
    assert_eq!(s.world().tick, d0 + 3 * DAYS_PER_YEAR);
}

#[test]
fn advance_until_importance_stops_at_the_event() {
    let mut s = world(16);
    let d0 = s.world().tick;
    let id = s.advance_until(Until::Importance(3), 50 * DAYS_PER_YEAR).expect("五十年內應有重要度 3 的事件");
    let w = s.world();
    let e = &w.ledger.events[id as usize];
    assert!(e.importance >= 3);
    assert!(w.tick > d0 && w.tick == e.tick, "停在事件發生的那一日");
    assert!(e.tick <= d0 + 50 * DAYS_PER_YEAR);
}

#[test]
fn advance_until_soul_stops_at_an_event_of_that_soul() {
    let mut s = world(17);
    let soul = {
        let w = s.world();
        w.persons.iter().find(|p| p.alive()).map(|p| p.soul).expect("開天後應有在世者")
    };
    let id = s.advance_until(Until::Soul(soul), 80 * DAYS_PER_YEAR).expect("八十年內該魂應有事件");
    assert!(s.world().ledger.events[id as usize].souls.contains(&soul));
}

#[test]
fn events_carry_their_day() {
    let s = world(18);
    let w = s.world();
    let late = w.ledger.events.iter().rev().find(|e| e.tick > 0).expect("有事件");
    assert_eq!(late.tick % DAYS_PER_MONTH, 0, "舊規則的事件落在該月最後一日");
}

#[test]
fn day_steps_match_month_steps() {
    let mut a = world(21);
    let mut b = world(21);
    for _ in 0..24 {
        a.step();
    }
    let t = b.world().tick;
    for d in 1..=24 * DAYS_PER_MONTH {
        b.advance_to(t + d);
    }
    assert_eq!(a.world().tick, b.world().tick);
    assert_eq!(a.hash(), b.hash(), "逐日推進與逐月推進得到同一個世界");
}

#[test]
fn fast_forward_open_reaches_limit_and_matches_stepping() {
    let mut a = world(22);
    let mut b = world(22);
    let limit = a.world().tick + 5 * DAYS_PER_YEAR + 11;
    let mut seen = 0usize;
    let out = a.fast_forward(FastStop::Open, limit, &mut || true, &mut |_, r| seen += r.len());
    assert_eq!(out, FastOutcome::Reached);
    assert_eq!(a.world().tick, limit, "停在指定的那一天");
    b.advance_to(limit);
    assert_eq!(a.hash(), b.hash());
    assert!(seen > 0, "觀察者收到期間寫入的事件");
}

#[test]
fn fast_forward_stops_at_next_world_event() {
    let mut s = world(23);
    let d0 = s.world().tick;
    let out = s.fast_forward(FastStop::Kind(EventKind::WorldChanged), d0 + 2000 * DAYS_PER_YEAR, &mut || true, &mut |_, _| {});
    let FastOutcome::Hit(id) = out else { panic!("兩千年內應有世界大事：{out:?}") };
    let w = s.world();
    let e = &w.ledger.events[id as usize];
    assert_eq!(e.kind, EventKind::WorldChanged);
    assert_eq!(w.tick, e.tick, "停在世界大事發生的那一日");
}

#[test]
fn fast_forward_stops_at_event_of_soul_on_its_day() {
    let mut s = world(24);
    let soul = s.world().persons.iter().find(|p| p.alive()).map(|p| p.soul).expect("開天後應有在世者");
    let d0 = s.world().tick;
    let out = s.fast_forward(FastStop::Soul(soul), d0 + 80 * DAYS_PER_YEAR, &mut || true, &mut |_, _| {});
    let FastOutcome::Hit(id) = out else { panic!("八十年內該魂應有事件：{out:?}") };
    let e = &s.world().ledger.events[id as usize];
    assert!(e.souls.contains(&soul));
    assert_eq!(s.world().tick, e.tick);
}

#[test]
fn fast_forward_pauses_on_budget_and_resumes_to_the_same_world() {
    let mut a = world(25);
    let mut b = world(25);
    let limit = a.world().tick + 3 * DAYS_PER_YEAR;
    let mut pauses = 0;
    loop {
        let mut units = 0;
        let out = a.fast_forward(
            FastStop::Open,
            limit,
            &mut || {
                units += 1;
                units <= 7
            },
            &mut |_, _| {},
        );
        match out {
            FastOutcome::Paused => pauses += 1,
            FastOutcome::Reached => break,
            FastOutcome::Hit(_) => unreachable!(),
        }
    }
    assert!(pauses > 0);
    b.advance_to(limit);
    assert_eq!(a.hash(), b.hash(), "分段快進與一次推進得到同一個世界");
}

#[test]
fn save_in_the_middle_of_a_month_loads_back() {
    let mut s = world(26);
    let t = s.world().tick;
    s.advance_to(t + 2 * DAYS_PER_MONTH + 9);
    let file = s.save();
    let loaded = Sim::load_with(&file, |p| Sim::new_seeded(p, file.rng_origin)).expect("月中存檔應能重播");
    assert_eq!(loaded.world().tick, s.world().tick);
    assert_eq!(loaded.hash(), s.hash());
}
