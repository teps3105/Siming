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

#[test]
fn gdd_3_7_middle_starts_with_4_to_6_dynasties_covering_70_percent() {
    for seed in [7u64, 42, 20261011] {
        let sim = sim_with(seed, Era::Middle);
        let init = initial(&sim);
        assert!((4..=6).contains(&init.len()), "seed {seed}: {} 個", init.len());
        let w = sim.world();
        let land = w.provinces.iter().filter(|p| !p.lost).count() as f32;
        let owned = init.iter().map(|d| d.provinces.len()).sum::<usize>() as f32;
        let cover = owned / land;
        assert!((0.6..=0.8).contains(&cover), "seed {seed}: 覆蓋 {cover}");
        for d in &init {
            assert!(d.ruler.is_some() && d.founded == 0);
        }
    }
}

#[test]
fn gdd_3_7_decline_starts_with_2_to_3_dynasties() {
    for seed in [7u64, 42] {
        let sim = sim_with(seed, Era::Decline);
        let n = initial(&sim).len();
        assert!((2..=3).contains(&n), "seed {seed}: {n} 個");
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
