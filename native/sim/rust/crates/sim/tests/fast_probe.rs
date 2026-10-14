//! 雙步進量測（手動執行，不進一般測試）：
//! `cargo test --release -p siming-sim --test fast_probe -- --ignored --nocapture`
//! 量中型世界快進 100 年、1000 年要幾秒，以及事件在月內各日的分佈、傳聞簽每遊戲日入簽幾則。

use std::time::Instant;

use siming_sim::calendar::{DAYS_PER_MONTH, DAYS_PER_YEAR};
use siming_sim::{FastStop, Sim, WorldParams, WorldSize};

fn medium(seed: u64) -> Sim {
    let mut p = WorldParams::new(seed);
    p.size = WorldSize::Medium;
    Sim::new_seeded(&p, Some(7)).expect("中型世界應能生成")
}

/// 傳聞簽入簽的最低重要度（game/modules/hud/code/rumor_slip.gd 的 NOTE_IMPORTANCE）；世界大事不論重要度都入簽。
const NOTE_IMPORTANCE: u8 = 2;

#[test]
#[ignore]
fn probe_fast_forward_seconds() {
    for years in [100u32, 1000] {
        let mut s = medium(20261011);
        let start = s.world().tick;
        let limit = start + years * DAYS_PER_YEAR;
        let began = Instant::now();
        let mut events = 0usize;
        s.fast_forward(FastStop::Open, limit, &mut || true, &mut |_, r| events += r.len());
        let secs = began.elapsed().as_secs_f64();
        println!("快進 {years} 年：{secs:.2} 秒，期間寫入 {events} 件事件，在世有名之人 {} 人", s.world().alive_ids.len());
    }
}

#[test]
#[ignore]
fn probe_event_day_distribution_and_rumor_rate() {
    let mut s = medium(20261011);
    let start = s.world().tick;
    let years = 30u32;
    let first = s.world().ledger.events.len();
    // 逐日推進，與玩家平常看到的一樣。
    for d in 1..=years * DAYS_PER_YEAR {
        s.advance_to(start + d);
    }
    let w = s.world();
    let mut by_day = vec![0u32; DAYS_PER_MONTH as usize];
    let mut by_day_noted = vec![0u32; DAYS_PER_MONTH as usize];
    let mut per_tick: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    let (mut all, mut noted) = (0u32, 0u32);
    for e in &w.ledger.events[first..] {
        if e.kind.is_chronicle_only() {
            continue;
        }
        let day = ((e.tick.saturating_sub(1)) % DAYS_PER_MONTH) as usize;
        all += 1;
        by_day[day] += 1;
        if e.importance >= NOTE_IMPORTANCE || e.kind == siming_sim::EventKind::WorldChanged {
            noted += 1;
            by_day_noted[day] += 1;
            *per_tick.entry(e.tick).or_insert(0) += 1;
        }
    }
    let days = (years * DAYS_PER_YEAR) as f64;
    println!("{years} 年 = {days} 遊戲日，中型世界");
    println!("全部事件 {all} 件，平均每日 {:.3} 件", all as f64 / days);
    println!("重要度 >= {NOTE_IMPORTANCE} 或世界大事（會進傳聞簽的候選）{noted} 件，平均每日 {:.3} 件", noted as f64 / days);
    println!("月內各日（第 1 到 30 日）事件數，全部：{by_day:?}");
    println!("月內各日事件數，入簽候選：{by_day_noted:?}");
    let busiest = per_tick.iter().max_by_key(|(_, n)| **n).map(|(t, n)| (*t, *n));
    let days_with = per_tick.len();
    println!("有入簽候選的日子 {days_with} 日（占 {:.1}%），單日最多 {busiest:?}", days_with as f64 * 100.0 / days);
    let mut hist: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    for n in per_tick.values() {
        *hist.entry(*n).or_insert(0) += 1;
    }
    println!("單日入簽候選件數分佈（件數 -> 幾日）：{hist:?}");
}
