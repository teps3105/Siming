//! 成道漏斗統計（`siming_sim::funnel`）：各表總數對得起來、同參數兩次輸出相同、統計不改世界。

use serde_json::json;
use siming_sim::funnel::Observer;
use siming_sim::{Sim, WorldParams, WorldSize};

const SEED: u64 = 7;
const WARMUP: u32 = 10;
const YEARS: u32 = 40;

fn small_sim() -> Sim {
    let mut params = WorldParams::new(SEED);
    params.size = WorldSize::Small;
    Sim::new_seeded(&params, Some(SEED)).expect("世界生成")
}

/// 跑預熱加觀察期；`observe` 為真時每月掃描。
fn run(sim: &mut Sim, obs: &mut Observer, observe: bool) {
    for _ in 0..(WARMUP + YEARS) * 12 {
        sim.step();
        sim.drain_events();
        if observe {
            obs.observe(sim.world());
        }
    }
}

#[test]
fn funnel_totals_reconcile() {
    let mut sim = small_sim();
    let mut obs = Observer::new(sim.world(), WARMUP);
    run(&mut sim, &mut obs, true);
    let w = sim.world();
    let report = obs.report(w, json!({}));

    // 母體獨立數一次：出生不早於起點且已歿。
    let direct = w.persons.iter().filter(|p| p.died.is_some() && p.born as i64 >= obs.start_tick as i64).count() as u64;
    let alive = w.persons.iter().filter(|p| p.died.is_none() && p.born as i64 >= obs.start_tick as i64).count() as u64;
    assert!(direct > 100, "小世界 {YEARS} 年應有足夠多的已歿世，實得 {direct}");
    assert_eq!(report.ended, direct);
    assert_eq!(report.alive, alive);

    assert_eq!(report.check(), Vec::<String>::new(), "自我核對");
    // 各路線人數和 = 結束的世數；各死因和 = 死亡數（死因表每列再核對，見 check）。
    assert_eq!(report.table("入道路線").col_sum("人數"), direct);
    let causes = report.table("死因（依最高境界分層）");
    let all = causes.row("全部").expect("全部列");
    let sum: u64 = all.iter().skip(2).filter_map(|c| c.count()).sum();
    assert_eq!(sum, direct);
    // 境界漏斗：到過本境者 = 死在本境者 + 到過下一境者。
    let realm = report.table("境界漏斗");
    for r in 0..8 {
        let (here, died, next) = (realm.rows[r][1].count().unwrap(), realm.rows[r][3].count().unwrap(), realm.rows[r][4].count().unwrap());
        assert_eq!(here, died + next, "境界 {r}");
    }
}

#[test]
fn funnel_same_input_same_output() {
    let render = || {
        let mut sim = small_sim();
        let mut obs = Observer::new(sim.world(), WARMUP);
        run(&mut sim, &mut obs, true);
        let r = obs.report(sim.world(), json!({}));
        (r.to_text(), r.to_json().to_string())
    };
    assert_eq!(render(), render());
}

#[test]
fn funnel_does_not_change_world() {
    let mut a = small_sim();
    let mut b = small_sim();
    let mut oa = Observer::new(a.world(), WARMUP);
    let mut ob = Observer::new(b.world(), WARMUP);
    assert_eq!(a.hash(), b.hash());
    run(&mut a, &mut oa, true);
    run(&mut b, &mut ob, false);
    assert_eq!(a.hash(), b.hash(), "觀察者不得改變世界雜湊");
    let _ = oa.report(a.world(), json!({}));
    assert_eq!(a.hash(), b.hash(), "產生報表不得改變世界雜湊");
}
