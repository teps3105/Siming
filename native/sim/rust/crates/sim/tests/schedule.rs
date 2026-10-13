//! GDD 6.3 判定月：個人與宗門的年度判定攤到各月後，事件在月份上的分布。

use siming_sim::calendar;
use siming_sim::schedule::{judge_month, Domain};
use siming_sim::{EventKind, Sim, WorldParams};

/// 測試用的短開天：人口期 600 年、人物期 200 年（預設長度太慢，且測試要看的是其中一段規則）。
fn quick(seed: u64) -> WorldParams {
    let mut p = WorldParams::new(seed);
    p.pop_years = 600;
    p.prerun_years = 200;
    p
}

/// 固定種子跑 90 年（預跑之後），重要度 ≥ 2 的事件依發生月份統計。
fn month_histogram(seed: u64) -> [u32; 12] {
    let mut sim = Sim::new(&quick(seed)).unwrap();
    let start = sim.world().tick;
    for _ in 0..1080 {
        sim.step();
    }
    let mut count = [0u32; 12];
    for e in &sim.world().ledger.events {
        if e.tick > start && e.importance >= 2 {
            count[(calendar::date(e.tick).month - 1) as usize] += 1;
        }
    }
    count
}

#[test]
fn gdd_6_3_events_spread_across_twelve_months() {
    // 人物與事件的演變不要求同種子同結果，事件量也隨演變浮動：三個種子合起來看分布。
    let mut count = [0u32; 12];
    for seed in [7u64, 42, 20261011] {
        let one = month_histogram(seed);
        eprintln!("種子 {seed} 重要度≥2 事件按月 {one:?}");
        for (c, o) in count.iter_mut().zip(one) {
            *c += o;
        }
    }
    let total: u32 = count.iter().sum();
    eprintln!("三個種子合計 {count:?} 合計 {total}");
    assert!(count.iter().all(|&c| c > 0), "12 個月都應有事件 {count:?}");
    assert!(count[11] as f64 / total as f64 <= 0.25, "12 月占比過高 {count:?}");
}

/// 壽終（Died）、饑荒（Famine）、王朝事件三類各自按月分布：這三類原本全擠在年底月。
#[test]
fn gdd_6_3_died_famine_and_dynasty_events_spread_across_months() {
    use EventKind::*;
    let dynasty_kinds = [
        DynastyFounded,
        Succession,
        Usurpation,
        CivilWar,
        Secession,
        Rebellion,
        Annexed,
        WarDeclared,
        BanTao,
        Fengshan,
        Relief,
        DynastyCollapsed,
    ];
    let (mut died, mut famine, mut dynasty) = ([0u32; 12], [0u32; 12], [0u32; 12]);
    for seed in [7u64, 42, 20261011] {
        let mut sim = Sim::new(&quick(seed)).unwrap();
        let start = sim.world().tick;
        for _ in 0..1080 {
            sim.step();
        }
        for e in sim.world().ledger.events.iter().filter(|e| e.tick > start) {
            let m = (calendar::date(e.tick).month - 1) as usize;
            if e.kind == Died {
                died[m] += 1;
            } else if e.kind == Famine {
                famine[m] += 1;
            } else if dynasty_kinds.contains(&e.kind) {
                dynasty[m] += 1;
            }
        }
    }
    eprintln!("壽終等 Died 按月 {died:?}\nFamine 按月 {famine:?}\n王朝事件按月 {dynasty:?}");
    for (name, h) in [("Died", died), ("Famine", famine), ("王朝事件", dynasty)] {
        let total: u32 = h.iter().sum();
        // 饑荒在 30 年內罕見（可能為 0），樣本夠多才檢查占比；整運的饑荒分布見 `siming-cli sim` 的逐月事件統計。
        // 壽終多發生在出生月（歲數判定），不憑空造人後出生的月份比以前集中（補魂集中在少數月），門檻放到 0.35。
        if total >= 24 {
            assert!(h[11] as f64 / total as f64 <= 0.35, "{name} 12 月占比過高 {h:?}");
        }
    }
}

/// 入道事件發生在人物自己的判定月（補足宗門的新弟子不經入道判定，以出生時間排除）。
#[test]
fn gdd_6_3_come_of_age_happens_in_own_judge_month() {
    let mut sim = Sim::new(&quick(42)).unwrap();
    let seed = sim.world().params.seed;
    let start = sim.world().tick;
    for _ in 0..600 {
        sim.step();
    }
    let w = sim.world();
    let mut checked = 0;
    let mut seen = std::collections::BTreeSet::new();
    for e in w.ledger.events.iter().filter(|e| e.kind == EventKind::ComeOfAge && e.tick > start) {
        let pid = e.persons[0];
        let p = &w.persons[pid as usize];
        // 補足的新弟子出生即 12 歲整；自然入道者的出生月與入道月不會剛好相差 12 年整。
        if p.born == e.tick as i32 - 144 {
            continue;
        }
        assert_eq!((calendar::date(e.tick).month - 1) as u8, judge_month(seed, Domain::Person, pid), "人 {pid}");
        assert!(seen.insert(pid), "人 {pid} 重複入道");
        checked += 1;
    }
    assert!(checked > 50, "應有足夠的入道事件可檢查，實得 {checked}");
}
