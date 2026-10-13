//! 發生率量測（事件驅動改寫的驗收基準，第 1 波）。
//!
//! 對種子 7、42、20261011 各跑完整開天（人口期 + 人物期），再往後推 1,000 年，
//! 統計這 1,000 年內每種 `EventKind` 的每年平均次數，以及運末的在世人數、各境界人數、宗門數、王朝數、魂數。
//! 事件取自因果帳：被修剪的事件留下墓碑，墓碑保留 `tick` 與 `kind`，所以按種類計數不受修剪影響。
//!
//! 結果寫成 JSON 供比對。舊引擎的基準存在 `tests/rate_baseline.json`。
//!
//! ```text
//! SIMING_RATE_OUT=<檔案> [SIMING_RATE_SEEDS=7,42] cargo test --release --test rate_probe -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;

use siming_sim::{PathState, Sim, WorldParams};

/// 量測窗的年數。
pub const WINDOW_YEARS: u32 = 1000;

/// 一個種子的量測結果。
#[derive(Debug, Clone)]
pub struct Measure {
    pub seed: u64,
    /// 量測窗起點的 tick。
    pub start_tick: u32,
    /// 窗內各事件種類的總次數。
    pub counts: BTreeMap<String, u64>,
    pub alive: u32,
    /// 在世修士各境界人數（下標為境界，從 0 起算）。
    pub realms: Vec<u32>,
    pub powers_alive: u32,
    pub dynasties_alive: u32,
    pub souls: u32,
}

/// 事件種類的名稱（`Debug` 輸出）。
fn kind_name(k: siming_sim::EventKind) -> String {
    format!("{k:?}")
}

/// 開天後再推 `years` 年並量測。
pub fn measure(seed: u64, years: u32) -> Measure {
    let mut sim = Sim::new(&WorldParams::new(seed)).expect("世界生成");
    let start_tick = sim.world().tick;
    for _ in 0..years * 12 {
        sim.step();
        sim.drain_events();
    }
    let w = sim.world();
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for e in w.ledger.events.iter().rev() {
        if e.tick <= start_tick {
            break;
        }
        *counts.entry(kind_name(e.kind)).or_default() += 1;
    }
    let mut realms = vec![0u32; 12];
    let mut alive = 0;
    for &i in &w.alive_ids {
        let p = &w.persons[i as usize];
        if !p.alive() {
            continue;
        }
        alive += 1;
        if let PathState::Cultivator { realm, .. } = p.path {
            let r = realm as usize;
            if r >= realms.len() {
                realms.resize(r + 1, 0);
            }
            realms[r] += 1;
        }
    }
    Measure {
        seed,
        start_tick,
        counts,
        alive,
        realms,
        powers_alive: w.powers.iter().filter(|p| p.fallen.is_none()).count() as u32,
        dynasties_alive: w.dynasty.list.iter().filter(|d| d.fallen.is_none()).count() as u32,
        souls: w.souls.len() as u32,
    }
}

pub fn to_json(m: &[Measure], years: u32) -> serde_json::Value {
    let per_seed: Vec<_> = m
        .iter()
        .map(|x| {
            let rates: BTreeMap<&str, f64> = x.counts.iter().map(|(k, &v)| (k.as_str(), v as f64 / years as f64)).collect();
            serde_json::json!({
                "seed": x.seed,
                "start_tick": x.start_tick,
                "counts": x.counts,
                "rates_per_year": rates,
                "alive": x.alive,
                "realms": x.realms,
                "powers_alive": x.powers_alive,
                "dynasties_alive": x.dynasties_alive,
                "souls": x.souls,
            })
        })
        .collect();
    serde_json::json!({ "engine": "monthly-sweep", "window_years": years, "seeds": per_seed })
}

#[test]
#[ignore = "完整開天加千年，每個種子數分鐘；由 SIMING_RATE_OUT 指定輸出檔"]
fn measure_rates() {
    let seeds: Vec<u64> = std::env::var("SIMING_RATE_SEEDS")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![7, 42, 20261011]);
    let mut all = Vec::new();
    for s in seeds {
        let t = std::time::Instant::now();
        let m = measure(s, WINDOW_YEARS);
        eprintln!(
            "種子 {s}：{:.1} 秒，在世 {}，宗門 {}，王朝 {}，魂 {}",
            t.elapsed().as_secs_f64(),
            m.alive,
            m.powers_alive,
            m.dynasties_alive,
            m.souls
        );
        all.push(m);
    }
    let json = to_json(&all, WINDOW_YEARS);
    let text = serde_json::to_string_pretty(&json).unwrap();
    match std::env::var("SIMING_RATE_OUT") {
        Ok(path) => std::fs::write(path, text).unwrap(),
        Err(_) => println!("{text}"),
    }
}

/// 人物期第 100、1,000、5,000、10,000 年附近每推進一年的平均耗時（毫秒）。窗為 10 年，
/// 起點在上述年數的前 5 年；人物期從第 0 年起逐步推進（`prerun` 為真，同開天）。
#[test]
#[ignore = "人物期推到萬年，需數分鐘至十數分鐘；由 SIMING_PERF_OUT 指定輸出檔"]
fn measure_year_cost() {
    let seed: u64 = std::env::var("SIMING_PERF_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(42);
    let mut p = WorldParams::new(seed);
    p.prerun_years = 0;
    let mut sim = Sim::new(&p).expect("世界生成");
    sim.world_mut().prerun = true;
    let marks = [100u32, 1000, 5000, 10000];
    let mut out = Vec::new();
    let mut year = 0u32;
    for &m in &marks {
        while year < m - 5 {
            for _ in 0..12 {
                sim.step();
            }
            year += 1;
        }
        let t = std::time::Instant::now();
        for _ in 0..10 {
            for _ in 0..12 {
                sim.step();
            }
            year += 1;
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / 10.0;
        let w = sim.world();
        eprintln!("人物期第 {m} 年附近：每年 {ms:.2} 毫秒（在世 {}，帳 {} 筆）", w.alive_ids.len(), w.ledger.events.len());
        out.push(serde_json::json!({
            "year": m, "ms_per_year": ms, "alive": w.alive_ids.len(), "ledger_events": w.ledger.events.len(), "souls": w.souls.len(),
        }));
    }
    let json = serde_json::json!({ "engine": "monthly-sweep", "seed": seed, "marks": out });
    let text = serde_json::to_string_pretty(&json).unwrap();
    match std::env::var("SIMING_PERF_OUT") {
        Ok(path) => std::fs::write(path, text).unwrap(),
        Err(_) => println!("{text}"),
    }
}
