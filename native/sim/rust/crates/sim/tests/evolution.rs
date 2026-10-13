//! GDD 2.8 天地演變：每世結算、世界大事與改元接在推演流程上。

use siming_sim::era::EraKind;
use siming_sim::{calendar, Era, EventArgs, EventKind, LeyNode, Sim, WorldParams};

/// 測試用的短開天：人口期 0 年、人物期 0 年（預設長度太慢，且測試要看的是其中一段規則）。
fn quick(seed: u64) -> WorldParams {
    let mut p = WorldParams::new(seed);
    p.pop_years = 0;
    p.prerun_years = 0;
    p
}

#[test]
fn gdd_2_8_slow_variables_settle_once_per_shi() {
    let mut sim = Sim::new(&quick(20261011)).unwrap();
    let at_creation = sim.world().evolution.clone();
    while sim.world().tick < 359 * 30 {
        sim.step();
    }
    assert_eq!(sim.world().evolution, at_creation, "一世之內不變");
    sim.step();
    let first = sim.world().evolution.clone();
    assert_ne!(first.tide, at_creation.tide, "第 360 個月結算一次");
    assert_eq!(first.warmth_periods, at_creation.warmth_periods, "週期不隨結算改變");
    while sim.world().tick < 719 * 30 {
        sim.step();
    }
    assert_eq!(sim.world().evolution, first);
    sim.step();
    assert_ne!(sim.world().evolution, first, "第 720 個月再結算一次");
}

#[test]
fn gdd_2_8_the_world_starts_at_genesis_and_eras_come_with_the_pre_run() {
    // 開天沒有開局前的歷史：年表只有開天一筆，預跑期間世界大事自然改元。
    let sim = Sim::new(&quick(20261011)).unwrap();
    let w = sim.world();
    assert_eq!(w.evolution.history_years, 0);
    assert_eq!(w.eras.len(), 1);
    assert_eq!(w.eras[0].kind, EraKind::Genesis);
    assert_eq!(w.eras[0].start_year, 1);
    let mut p = WorldParams::new(20261011);
    p.pop_years = 3000;
    p.prerun_years = 0;
    let sim = Sim::new(&p).unwrap();
    let w = sim.world();
    assert!(w.eras.len() >= 3, "三千年的人口期裡天地自然改元，實際 {}", w.eras.len());
    assert!(w.eras.iter().all(|e| e.start_year <= calendar::total_year(w.tick, 0) + 1), "年表裡沒有未來的紀元");
}

#[test]
fn gdd_2_8_world_events_enter_the_ledger_and_open_eras() {
    let mut sim = Sim::new(&quick(20261011)).unwrap();
    let known = sim.world().eras.len();
    // 兩運：720 年。
    while sim.world().tick < 2 * 4320 * 30 {
        sim.step();
    }
    let w = sim.world();
    let events: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::WorldChanged).collect();
    assert!(!events.is_empty(), "兩運內至少有一件世界大事");
    assert!(events.iter().all(|e| e.tick % 10_800 == 0), "世界大事只在每世結算時發生");
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

fn sim_in(era: Era) -> Sim {
    let mut params = quick(20261011);
    params.era = era;
    Sim::new(&params).unwrap()
}

fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len() as f32
}

#[test]
fn gdd_2_8_opening_aura_follows_the_opening_tide() {
    let [ancient, middle, decline] = [Era::Ancient, Era::Middle, Era::Decline].map(sim_in);
    let cell = |s: &Sim| mean(&s.world().geo.aura_cell);
    assert!(cell(&ancient) > cell(&middle) * 1.15, "上古 {} 中古 {}", cell(&ancient), cell(&middle));
    assert!(cell(&decline) < cell(&middle) * 0.7, "末法 {} 中古 {}", cell(&decline), cell(&middle));
    let province = |s: &Sim| mean(&s.world().provinces.iter().map(|p| p.aura).collect::<Vec<_>>());
    assert!(province(&ancient) > province(&middle) && province(&middle) > province(&decline));
}

#[test]
fn gdd_2_8_aura_is_rescaled_by_the_tide_every_shi() {
    let mut sim = sim_in(Era::Middle);
    let before = sim.world().geo.aura_cell.clone();
    let terrain_before = sim.world().geo.terrain.clone();
    let tide_before = sim.world().evolution.tide;
    let provinces_before: Vec<f32> = sim.world().provinces.iter().map(|p| p.aura).collect();
    while sim.world().tick < 360 * 30 {
        sim.step();
    }
    let w = sim.world();
    let ratio = w.evolution.tide / tide_before;
    assert!(ratio < 0.99, "中古開局靈潮正在退，倍率 {ratio}");
    let same_nodes = !w.ledger.events.iter().any(|e| e.kind == EventKind::NodeShifted);
    assert!(same_nodes, "這一世沒有節點升降");
    for (i, (&was, &now)) in before.iter().zip(&w.geo.aura_cell).enumerate() {
        // 地形這一世變了的格，靈氣的地形基礎也跟著變，不在此列。
        if was < 1.9 && w.geo.terrain[i] == terrain_before[i] {
            assert!((now - was * ratio).abs() < 1e-4, "格 {i}：{was} → {now}，倍率 {ratio}");
        }
    }
    let lowered = w.provinces.iter().zip(&provinces_before).filter(|(p, &was)| p.aura < was).count();
    assert!(lowered * 10 >= w.provinces.len() * 8, "州郡靈氣跟著降的只有 {lowered}/{}", w.provinces.len());
}

#[test]
fn gdd_2_8_node_shifts_enter_the_ledger_and_change_the_ley_lines() {
    let mut sim = sim_in(Era::Decline);
    let at_opening: Vec<u8> = sim.world().geo.nodes.iter().map(LeyNode::grade).collect();
    // 末法開局在谷底，靈潮回到 0.7 以上之前還有幾世。
    while sim.world().tick < 6 * 360 * 30 {
        sim.step();
    }
    let w = sim.world();
    let shifts: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::NodeShifted).collect();
    assert!(!shifts.is_empty(), "末法六世內至少有一個洞天福地降級");
    let mut grades = at_opening;
    for e in &shifts {
        let EventArgs::Node { node, from, to } = e.args else { panic!("參數不是節點：{:?}", e.args) };
        assert_eq!(e.tick % 10_800, 0, "只在每世結算時發生");
        assert_eq!(grades[node as usize], from, "事件的原等級與節點當時的等級相同");
        assert_eq!(e.cell, Some(w.geo.nodes[node as usize].cell));
        assert!(e.province.is_some(), "事件掛在節點所在的州");
        grades[node as usize] = to;
    }
    let now: Vec<u8> = w.geo.nodes.iter().map(LeyNode::grade).collect();
    assert_eq!(grades, now, "照事件推演的等級與世界狀態相同");
    for edge in &w.geo.ley {
        let want = 0.5 * (w.geo.nodes[edge.a as usize].strength + w.geo.nodes[edge.b as usize].strength);
        assert_eq!(edge.strength, want, "靈脈強度是兩端目前強度的平均");
    }
    assert!(w.geo.nodes.iter().all(|n| n.strength <= n.origin));
}
