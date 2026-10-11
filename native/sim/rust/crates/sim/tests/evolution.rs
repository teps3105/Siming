//! GDD 2.8 天地演變：每世結算、世界大事與改元接在推演流程上。

use siming_sim::era::EraKind;
use siming_sim::{calendar, EventArgs, EventKind, Sim, WorldParams};

#[test]
fn gdd_2_8_slow_variables_settle_once_per_shi() {
    let mut sim = Sim::new(&WorldParams::new(20261011)).unwrap();
    let at_creation = sim.world().evolution.clone();
    while sim.world().tick < 359 {
        sim.step();
    }
    assert_eq!(sim.world().evolution, at_creation, "一世之內不變");
    sim.step();
    let first = sim.world().evolution.clone();
    assert_ne!(first.tide, at_creation.tide, "第 360 個月結算一次");
    assert_eq!(first.warmth_periods, at_creation.warmth_periods, "週期不隨結算改變");
    while sim.world().tick < 719 {
        sim.step();
    }
    assert_eq!(sim.world().evolution, first);
    sim.step();
    assert_ne!(sim.world().evolution, first, "第 720 個月再結算一次");
}

#[test]
fn gdd_2_8_the_world_opens_with_a_history() {
    let sim = Sim::new(&WorldParams::new(20261011)).unwrap();
    let w = sim.world();
    let history = w.evolution.history_years;
    assert!(history >= 1800, "中古開局前至少 1,800 年，實際 {history}");
    assert_eq!(w.eras[0].kind, EraKind::Genesis);
    assert_eq!(w.eras[0].start_year, 1);
    assert!(w.eras.len() >= 3, "開局前已有數個紀元");
    assert!(w.eras.iter().all(|e| e.start_year <= history + 1), "年表裡沒有未來的紀元");
    assert!(calendar::total_year(w.tick, history) > 1800, "開局不是開天第一年");
}

#[test]
fn gdd_2_8_world_events_enter_the_ledger_and_open_eras() {
    let mut sim = Sim::new(&WorldParams::new(20261011)).unwrap();
    let known = sim.world().eras.len();
    // 兩運：720 年。
    while sim.world().tick < 2 * 4320 {
        sim.step();
    }
    let w = sim.world();
    let events: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::WorldChanged).collect();
    assert!(!events.is_empty(), "兩運內至少有一件世界大事");
    assert!(events.iter().all(|e| e.tick % 360 == 0), "世界大事只在每世結算時發生");
    let opened = events.iter().filter(|e| matches!(e.args, EventArgs::World { new_era: true, .. })).count();
    assert_eq!(opened, w.eras.len() - known, "開啟紀元的大事與年表新增的筆數相同");
    assert!(w.eras.len() > known, "兩運內至少改元一次");
    for pair in w.eras.windows(2) {
        assert!(pair[1].start_year >= pair[0].start_year + 60, "紀元 {} 短於 60 年", pair[0].name);
    }
    // 日期落在年表最後一個已開始的紀元裡。
    let history = w.evolution.history_years;
    let (era, year) = calendar::era_year(w.tick, history, &w.eras);
    assert_eq!(era.name, w.eras.last().unwrap().name);
    assert!(year >= 1);
}
