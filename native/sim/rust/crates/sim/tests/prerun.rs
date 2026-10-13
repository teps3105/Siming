//! GDD 4.6 分期預跑（人口期 → 人物期）的驗收：開天人口為 0，世界自己長出人、修士、宗門與王朝。
//! 長度取較短的設定（人口期 3,000 年 + 人物期 300 年），完整長度的版本標 `#[ignore]`。

use std::collections::BTreeMap;

use siming_sim::genesis;
use siming_sim::{EventKind, PathState, Sim, WorldParams};

const SEED: u64 = 20261011;

fn params(pop: u32, pre: u32) -> WorldParams {
    let mut p = WorldParams::new(SEED);
    p.pop_years = pop;
    p.prerun_years = pre;
    p
}

#[test]
fn genesis_state_has_only_the_first_settlements() {
    let sim = Sim::new(&params(0, 0)).unwrap();
    let w = sim.world();
    assert_eq!(w.tick, 0);
    assert!(w.persons.is_empty(), "開天沒有有名之人");
    assert_eq!(w.souls.len(), 1, "只有天道魂");
    assert!(w.powers.is_empty(), "沒有宗門");
    assert!(w.dynasty.list.is_empty(), "沒有王朝");
    assert!(w.threads.is_empty());
    let settled: std::collections::BTreeSet<u16> = w.settlements.iter().map(|s| s.0).collect();
    for p in &w.provinces {
        if !settled.contains(&p.id) {
            assert_eq!(p.population, 0.0, "州 {} 沒有聚落卻有人口", p.id);
        }
    }
    let total: f64 = w.provinces.iter().map(|p| p.population).sum();
    assert_eq!(total, w.settlements.len() as f64 * siming_sim::params::GENESIS_SETTLE_POP, "人口只來自第一批聚落");
    assert!(!w.settlements.is_empty());
}

#[test]
fn settlement_is_deterministic_and_on_land() {
    let a = Sim::new(&params(0, 0)).unwrap();
    let b = Sim::new(&params(0, 0)).unwrap();
    assert_eq!(a.world().settlements, b.world().settlements);
    let other = Sim::new(&WorldParams::new(SEED + 1)).map(|s| s.world().settlements.clone());
    let _ = other; // 換種子會重生成世界，這裡只要求同種子一致。
    let w = a.world();
    let width = w.geo.w as usize;
    for &(p, cell) in &w.settlements {
        let c = cell as usize;
        assert!(c < w.geo.continent_of.len());
        assert_ne!(w.geo.continent_of[c], siming_sim::types::NO_CONTINENT, "聚落 {cell} 不在陸地上");
        assert_eq!(w.geo.province_of[c], p);
        let _ = width;
    }
}

/// 人口期的人口走勢：人口從無到有、滿載的州占多數，沿海路遷徙讓沒有聚落的大洲也有人，且晚於第 0 年。
#[test]
fn population_grows_from_nothing_and_crosses_the_sea() {
    let mut sim = Sim::new(&params(0, 0)).unwrap();
    let continents: BTreeMap<u16, u16> =
        sim.world().provinces.iter().map(|p| (p.id, sim.world().geo.continent_of[p.seat as usize])).collect();
    let seeded: std::collections::BTreeSet<u16> = sim.world().settlements.iter().map(|s| continents[&s.0]).collect();
    let mut first_year: BTreeMap<u16, u32> = BTreeMap::new();
    let mut million_year = None;
    for century in 1..=100u32 {
        genesis::grow_population(sim.world_mut(), 100, &mut |_, _| {});
        let w = sim.world();
        for p in &w.provinces {
            if p.population > 100.0 {
                first_year.entry(continents[&p.id]).or_insert(century * 100);
            }
        }
        let total: f64 = w.provinces.iter().map(|p| p.population).sum();
        if million_year.is_none() && total >= 1.0e6 {
            million_year = Some(century * 100);
        }
    }
    let w = sim.world();
    let total: f64 = w.provinces.iter().map(|p| p.population).sum();
    let populated = w.provinces.iter().filter(|p| !p.lost && p.population > 100.0).count();
    let land = w.provinces.iter().filter(|p| !p.lost).count();
    eprintln!(
        "萬年後人口 {total:.0} 有人州 {populated}/{land} 首次有人的大洲年份 {first_year:?} 起源大洲 {seeded:?} 百萬人口年 {million_year:?}"
    );
    assert!((1.0e7..8.0e7).contains(&total), "一萬年後總人口 {total:.0}");
    assert!(populated * 10 >= land * 9, "有人的州只有 {populated}/{land}");
    let all: std::collections::BTreeSet<u16> = continents.values().copied().collect();
    for k in &all {
        if !seeded.contains(k) {
            if let Some(&y) = first_year.get(k) {
                assert!(y > 0, "無聚落的大洲 {k} 在第 {y} 年就有人");
            }
        }
    }
    let reached = all.iter().filter(|k| first_year.contains_key(k)).count();
    assert!(reached * 10 >= all.len() * 9, "海路可達的大洲 {reached}/{} 有人", all.len());
}

struct Run {
    sim: Sim,
}

fn short_run() -> Run {
    Run { sim: Sim::new(&params(3000, 300)).unwrap() }
}

fn checks(sim: &Sim) {
    let w = sim.world();
    // 每個在世者都有 Born 事件，tick 等於降生時刻。
    let mut born: BTreeMap<u32, i32> = BTreeMap::new();
    let mut first_comeofage: BTreeMap<u32, u32> = BTreeMap::new();
    let mut breakthroughs: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for e in &w.ledger.events {
        let Some(&pid) = e.persons.first() else { continue };
        match e.kind {
            EventKind::Born | EventKind::Rebirth => {
                born.insert(pid, e.tick as i32);
            }
            EventKind::ComeOfAge => {
                first_comeofage.entry(pid).or_insert(e.tick);
            }
            EventKind::Breakthrough => breakthroughs.entry(pid).or_default().push(e.tick),
            _ => {}
        }
    }
    let alive: Vec<_> = w.persons.iter().filter(|p| p.alive()).collect();
    assert!(!alive.is_empty());
    let mut missing = 0;
    for p in &alive {
        match born.get(&p.id) {
            Some(&t) if t == p.born => {}
            _ => missing += 1,
        }
    }
    // 宗門補足的 12 歲弟子、王朝補的成年散人是「降生後才把 born 往前改」，事件不能倒填（帳依時刻遞增），
    // 所以這些人的 Born 時刻晚於 born；實測約 5%，容許 10%。
    assert!(missing * 10 <= alive.len(), "{missing}/{} 位在世者沒有與降生時刻相符的 Born／Rebirth 事件", alive.len());
    // 入道在第一次突破之前，突破時刻遞增。
    for p in &alive {
        if let Some(ts) = breakthroughs.get(&p.id) {
            assert!(ts.windows(2).all(|x| x[0] <= x[1]), "人 {} 突破時刻未遞增 {ts:?}", p.id);
            if let Some(&c) = first_comeofage.get(&p.id) {
                assert!(c <= ts[0], "人 {} 入道 {c} 晚於首次突破 {}", p.id, ts[0]);
            }
        }
    }
    // 境界人數從低到高遞減。
    let mut realms = [0u32; 9];
    for p in &alive {
        if let PathState::Cultivator { realm, .. } = p.path {
            realms[realm as usize] += 1;
        }
    }
    eprintln!("接手時境界分布 {realms:?}");
    for r in 0..4 {
        assert!(realms[r] >= realms[r + 1], "境界 {r} 人數 {} 少於境界 {} 的 {}", realms[r], r + 1, realms[r + 1]);
    }
    // 現存宗門都有創立事件。
    let founded: std::collections::BTreeSet<u32> =
        w.ledger.events.iter().filter(|e| e.kind == EventKind::PowerFounded).flat_map(|e| e.powers.iter().copied()).collect();
    for pw in w.powers.iter().filter(|p| p.fallen.is_none()) {
        assert!(founded.contains(&pw.id), "宗門 {} 沒有創立事件", pw.name);
    }
}

#[test]
fn short_pre_run_grows_people_sects_and_keeps_their_history() {
    let run = short_run();
    let w = run.sim.world();
    assert_eq!(w.tick, 3300 * 12);
    assert!(w.persons.iter().any(|p| p.alive()));
    // 第一位修士出現的年份晚於人口達到百萬的年份（人物期才有有名之人）。
    let first_cultivator = w.ledger.events.iter().filter(|e| e.kind == EventKind::ComeOfAge).map(|e| e.tick).min();
    assert!(first_cultivator.is_none_or(|t| t / 12 >= 3000), "修士出現在人口期內");
    checks(&run.sim);
}

/// 一個世界中重要度 ≥ 2 的事件，依（種類，距 `start` 的年序）計數；只取 `start` 之後。
fn counts_after(sim: &Sim, start: u32) -> BTreeMap<(EventKind, u32), u32> {
    let mut by: BTreeMap<(EventKind, u32), u32> = BTreeMap::new();
    for e in sim.world().ledger.events.iter().filter(|e| e.tick > start && e.importance >= 2) {
        *by.entry((e.kind, (e.tick - start - 1) / 12)).or_default() += 1;
    }
    by
}

/// 三個種子合計後，每類事件接手後第 1 年的筆數不超過第 2–10 年平均的 2 倍（下限 5 筆，防小數雜訊）。
fn assert_no_backlog(by: &BTreeMap<(EventKind, u32), u32>) {
    let kinds: std::collections::BTreeSet<EventKind> = by.keys().map(|k| k.0).collect();
    for k in kinds {
        let y1 = by.get(&(k, 0)).copied().unwrap_or(0) as f64;
        let rest: f64 = (1..10).map(|y| by.get(&(k, y)).copied().unwrap_or(0) as f64).sum::<f64>() / 9.0;
        assert!(y1 <= (2.0 * rest).max(5.0), "{k:?} 第 1 年 {y1} 件，第 2–10 年平均 {rest:.1}（三個種子合計）");
    }
}

/// 接手後第 1 年各類事件不積壓，含饑荒與賑濟。饑荒每逢一世（30 年）的邊界會因天地演變改了承載而集中出現
/// （人口期與人物期都一樣，與步長無關），而接手時刻本身就落在一世的邊界上，
/// 所以先走過 7 年離開邊界，再取「接手後第 1 年」那一年為窗。
#[test]
fn first_year_after_takeover_has_no_backlog() {
    let mut total: BTreeMap<(EventKind, u32), u32> = BTreeMap::new();
    for seed in [SEED, SEED + 1, SEED + 2] {
        let mut p = params(3000, 300);
        p.seed = seed;
        let mut sim = Sim::new(&p).unwrap();
        for _ in 0..84 {
            sim.step();
        }
        let start = sim.world().tick;
        for _ in 0..120 {
            sim.step();
        }
        for (k, v) in counts_after(&sim, start) {
            *total.entry(k).or_default() += v;
        }
    }
    assert_no_backlog(&total);
}

/// 步長切換前後的同一條規則：同一個人口期末的世界，各州人口與饑荒的規則以年步長（`grow_population`）
/// 與月步長（各州在自己的判定月做一次年度更新）各走 300 年，三個種子合計的饑荒筆數與總人口要相近。
/// 目前人物期沒有年步長的路徑（年步長只有人口期），所以以這條「同世界兩種步長」對照代替人物期中途切換。
#[test]
fn year_step_and_month_step_agree_on_population_and_famine() {
    use siming_sim::schedule::Round;
    let (mut fam_year, mut fam_month, mut pop_year, mut pop_month) = (0u32, 0u32, 0.0f64, 0.0f64);
    for seed in [SEED, SEED + 1, SEED + 2] {
        let mut p = params(1500, 0);
        p.seed = seed;
        let base = Sim::new(&p).unwrap();
        let count =
            |w: &siming_sim::World| w.ledger.events.iter().filter(|e| e.kind == EventKind::Famine && e.tick > 1500 * 12).count() as u32;
        let total = |w: &siming_sim::World| w.provinces.iter().map(|p| p.population).sum::<f64>();
        let mut a = base.world().clone();
        genesis::grow_population(&mut a, 300, &mut |_, _| {});
        let mut b = base.world().clone();
        for _ in 0..300 * 12 {
            b.tick += 1;
            let tick = b.tick;
            let f = siming_sim::population::monthly_with(&mut b.provinces, &[], seed, Round::at(tick));
            siming_sim::ledger::record_famines(&mut b, &f);
            if tick % 12 == 0 {
                if tick % 360 == 0 {
                    siming_sim::evolution::shi_settle(&mut b);
                }
                siming_sim::cataclysm::yearly(&mut b);
            }
        }
        (fam_year, fam_month, pop_year, pop_month) =
            (fam_year + count(&a), fam_month + count(&b), pop_year + total(&a), pop_month + total(&b));
    }
    eprintln!("年步長 饑荒 {fam_year} 人口 {pop_year:.0}；月步長 饑荒 {fam_month} 人口 {pop_month:.0}");
    assert!(fam_year.abs_diff(fam_month) <= 3 + fam_year.max(fam_month) / 2, "饑荒筆數 年 {fam_year} 月 {fam_month}");
    assert!((pop_month / pop_year - 1.0).abs() < 0.05, "總人口 年 {pop_year:.0} 月 {pop_month:.0}");
}

/// 完整長度（人口期 10,000 年 + 人物期 1,000 年）；慢，手動跑。
#[test]
#[ignore = "完整長度約 12 秒；cargo test -- --ignored"]
fn full_length_pre_run() {
    let sim = Sim::new(&WorldParams::new(SEED)).unwrap();
    checks(&sim);
    let w = sim.world();
    let alive_dyn = w.dynasty.list.iter().filter(|d| d.fallen.is_none()).count();
    assert!(alive_dyn >= 1, "長預跑後王朝歸零");
}
