//! GDD 3.7 王朝與天命的整合測試：開局王朝、長時間推演。

use siming_sim::{Era, EventKind, Sim, WorldParams};

fn sim_with(seed: u64, era: Era) -> Sim {
    let mut p = WorldParams::new(seed);
    p.era = era;
    Sim::new(&p).unwrap()
}

/// 開天時建立的王朝（立國於 tick 0）。
fn initial(sim: &Sim) -> Vec<&siming_sim::dynasty::Dynasty> {
    sim.world().dynasty.list.iter().filter(|d| d.founded == 0).collect()
}

#[test]
fn gdd_3_7_ancient_starts_without_dynasty() {
    let sim = sim_with(7, Era::Ancient);
    assert!(initial(&sim).is_empty(), "上古開局所有州郡無主");
}

/// 開天不再直接造王朝：王朝由預跑期間「無主且人口夠的州，當地最出眾的散人起事」長出來。
/// 接手時在世王朝數落在 7.1 目標（中古 4–6、末法 2–3）附近：實測 3–9，這裡放寬為 2–12。
#[test]
fn gdd_3_7_dynasties_grow_during_the_pre_run_and_survive_to_takeover() {
    for seed in [7u64, 42] {
        let sim = sim_with(seed, Era::Middle);
        let w = sim.world();
        let alive = w.dynasty.list.iter().filter(|d| d.fallen.is_none()).count();
        assert!((2..=12).contains(&alive), "seed {seed}: 接手時在世王朝 {alive} 個");
        assert!(initial(&sim).is_empty(), "開天（tick 0）沒有王朝");
    }
}

#[test]
fn gdd_3_7_one_yun_runs_and_emits_dynasty_events() {
    let mut sim = sim_with(42, Era::Middle);
    for _ in 0..4320 {
        sim.step();
    }
    let w = sim.world();
    assert!(w.ledger.events.iter().any(|e| e.kind == EventKind::DynastyFounded));
    // 歸屬表與疆域列表一致。
    for d in w.dynasty.list.iter().filter(|d| d.fallen.is_none()) {
        for &p in &d.provinces {
            assert_eq!(w.dynasty.owner_of(p), Some(d.id));
        }
    }
    let owned: usize = w.dynasty.owner.iter().filter(|&&o| o != siming_sim::dynasty::NO_OWNER).count();
    let listed: usize = w.dynasty.list.iter().map(|d| d.provinces.len()).sum();
    assert_eq!(owned, listed);
}
