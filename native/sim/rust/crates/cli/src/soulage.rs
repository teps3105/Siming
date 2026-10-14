//! 魂齡量測（GDD 3.1 魂力）：長跑世界，量魂誕生到魂散的年數分佈與一個魂一生跨越的紀元數。
//!
//! 做法：開天與預跑後逐月推進 `years` 年（人物期的完整規則，魂力照常消耗與魂散），期間每 `sample` 年取樣一次魂數；
//! 期末彙整每個魂的魂齡（魂散者到魂散，超脫者到超脫，仍在輪迴者算到期末，標為截尾）。
//! `--baseline` 為對照組：每年把所有魂的魂力補滿，等於沒有魂散，量魂不散時總魂數的走向。
//!
//! 紀元數：世界現行紀元表的長度與要採用的長度（最短 8,000、中位約 15,000 年）不同，所以另用該分佈模擬——
//! 紀元長度 = 8,000 + 指數分佈（平均 10,098，使中位 15,000）；每個魂隨機落在一個紀元內（依長度加權），
//! 以 `DRAWS` 組互相獨立的紀元切分各量一次，取平均分佈。另以世界現行紀元表直接數一份供對照。

use std::path::PathBuf;
use std::time::Instant;

use serde_json::json;
use siming_sim::{calendar, Sim, SoulState, WorldParams, WorldSize};

/// 每個魂的紀元切分重抽次數。
const DRAWS: usize = 40;
/// 紀元最短長度與指數部分的平均（年），使中位 = 8,000 + 平均 × ln 2 ≈ 15,000。
const EPOCH_MIN: f64 = 8_000.0;
const EPOCH_EXP_MEAN: f64 = 7_000.0 / std::f64::consts::LN_2;

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn exp(&mut self) -> f64 {
        -EPOCH_EXP_MEAN * self.unit().ln()
    }
}

/// 一個魂活 `age` 年跨越的紀元數：隨機落在某紀元內（依長度加權），數期間越過的紀元邊界。
fn epochs_crossed(rng: &mut SplitMix, age: f64) -> u32 {
    // 長度加權的紀元長度：以 EPOCH_MIN/(EPOCH_MIN+平均) 的權重取指數分佈，其餘取 Gamma(2)。
    let first_exp = if rng.unit() < EPOCH_MIN / (EPOCH_MIN + EPOCH_EXP_MEAN) { rng.exp() } else { rng.exp() + rng.exp() };
    let len = EPOCH_MIN + first_exp;
    let mut left = rng.unit() * len; // 到本紀元結束還剩幾年
    let mut n = 1;
    while age > left {
        n += 1;
        left += EPOCH_MIN + rng.exp();
    }
    n
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[k]
}

fn summary(label: &str, ages: &[f64]) -> serde_json::Value {
    let mut v = ages.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
    let line = format!(
        "{label}：{} 個　平均 {:.0}　中位 {:.0}　P90 {:.0}　P99 {:.0}　最長 {:.0}（年）",
        v.len(),
        mean,
        percentile(&v, 0.5),
        percentile(&v, 0.9),
        percentile(&v, 0.99),
        v.last().copied().unwrap_or(0.0)
    );
    println!("{line}");
    json!({ "組": label, "個數": v.len(), "平均": mean, "中位": percentile(&v, 0.5), "P90": percentile(&v, 0.9), "P99": percentile(&v, 0.99), "最長": v.last().copied().unwrap_or(0.0) })
}

/// 紀元數分佈：回傳 [1 個, 2 個, 3 個, 4 個以上] 的占比。
fn epoch_shares(rng: &mut SplitMix, ages: &[f64]) -> [f64; 4] {
    let mut c = [0u64; 4];
    for &a in ages {
        for _ in 0..DRAWS {
            let n = epochs_crossed(rng, a);
            c[(n.min(4) - 1) as usize] += 1;
        }
    }
    let total: u64 = c.iter().sum::<u64>().max(1);
    c.map(|x| x as f64 / total as f64)
}

pub fn run(seed: u64, years: u32, size: WorldSize, rng_seed: Option<u64>, baseline: bool, sample: u32, cohort_years: u32, json_out: Option<PathBuf>) -> Result<(), String> {
    let t0 = Instant::now();
    let mut params = WorldParams::new(seed);
    params.size = size;
    let rng_seed = rng_seed.unwrap_or(seed);
    let mut s = Sim::new_seeded(&params, Some(rng_seed)).map_err(|e| e.to_string())?;
    let start = s.world().tick;
    let year0 = start / calendar::DAYS_PER_YEAR;
    println!("世界種子 {seed}　亂數種子 {rng_seed}　尺寸 {size:?}　接手年 {year0}　{}", if baseline { "對照組（每年補滿魂力，不魂散）" } else { "魂力照常消耗" });
    println!("開天加預跑 {:.1} 秒", t0.elapsed().as_secs_f64());
    let mut rows = Vec::new();
    for y in 1..=years {
        for _ in 0..12 {
            s.step();
            s.drain_events();
        }
        if baseline {
            for so in s.world_mut().souls.iter_mut() {
                so.power = siming_sim::params::SOUL_POWER_MAX;
            }
        }
        if y % sample == 0 || y == years {
            let w = s.world();
            let (mut inc, mut other, mut dis, mut lib) = (0u32, 0u32, 0u32, 0u32);
            for so in w.souls.iter().skip(1) {
                match so.state {
                    SoulState::Incarnate(_) => inc += 1,
                    SoulState::Dissolved { .. } => dis += 1,
                    SoulState::Liberated { .. } => lib += 1,
                    _ => other += 1,
                }
            }
            println!(
                "{:>6} 年　在世 {inc:>5}　中陰等待他道 {other:>5}　輪迴中合計 {:>5}　累計魂散 {dis:>6}　累計超脫 {lib:>4}　累計新建 {:>6}　{:.0} 秒",
                year0 + y,
                inc + other,
                w.souls.len() - 1,
                t0.elapsed().as_secs_f64()
            );
            rows.push(json!({ "年": year0 + y, "在世": inc, "中陰等待他道": other, "輪迴中": inc + other, "累計魂散": dis, "累計超脫": lib, "累計新建": w.souls.len() - 1 }));
        }
    }
    let w = s.world();
    let end = w.tick;
    let age_of = |so: &siming_sim::Soul| -> (f64, bool) {
        match so.state {
            SoulState::Dissolved { tick } | SoulState::Liberated { tick, .. } => ((tick - so.born) as f64 / calendar::DAYS_PER_YEAR as f64, false),
            _ => ((end - so.born) as f64 / calendar::DAYS_PER_YEAR as f64, true),
        }
    };
    let mut dissolved = Vec::new();
    let mut cohort = Vec::new();
    let mut cohort_censored = 0u32;
    let mut cohort_liberated = 0u32;
    let mut all_censored = 0u32;
    for so in w.souls.iter().skip(1) {
        let (a, censored) = age_of(so);
        if let SoulState::Dissolved { .. } = so.state {
            dissolved.push(a);
        }
        if censored {
            all_censored += 1;
        }
        if so.born >= start && so.born <= start + cohort_years * calendar::DAYS_PER_YEAR {
            cohort.push(a);
            cohort_censored += censored as u32;
            cohort_liberated += matches!(so.state, SoulState::Liberated { .. }) as u32;
        }
    }
    println!("\n== 魂齡（年）==");
    let j_all = summary("全部魂散者", &dissolved);
    let j_cohort = summary(&format!("接手後 {cohort_years} 年內誕生的魂（群體；含超脫與未散者，後兩者算到結束或期末）"), &cohort);
    println!("　群體中 超脫 {cohort_liberated}　期末仍在輪迴（截尾）{cohort_censored}；全體期末仍在輪迴 {all_censored}");
    // 群體中只取魂散者，另列一行。
    let cohort_dissolved: Vec<f64> = w
        .souls
        .iter()
        .skip(1)
        .filter(|so| so.born >= start && so.born <= start + cohort_years * calendar::DAYS_PER_YEAR && matches!(so.state, SoulState::Dissolved { .. }))
        .map(|so| age_of(so).0)
        .collect();
    let j_cd = summary("群體中的魂散者", &cohort_dissolved);
    let buckets = [0.0, 2_000.0, 5_000.0, 10_000.0, 15_000.0, 20_000.0, 30_000.0, 50_000.0, f64::MAX];
    println!("\n魂散者魂齡直方（年）：");
    let mut hist = Vec::new();
    for k in 0..buckets.len() - 1 {
        let n = dissolved.iter().filter(|&&a| a >= buckets[k] && a < buckets[k + 1]).count();
        let hi = if buckets[k + 1] == f64::MAX { "以上".to_string() } else { format!("–{}", buckets[k + 1]) };
        println!("  {:>6}{hi:<8} {n:>6}  {:>5.1}%", buckets[k], 100.0 * n as f64 / dissolved.len().max(1) as f64);
        hist.push(json!({ "下限": buckets[k], "個數": n }));
    }
    println!("\n== 一個魂一生跨越的紀元數（紀元：最短 8,000、中位 15,000 年的模擬切分，每魂 {DRAWS} 組）==");
    let mut srng = SplitMix(seed ^ 0x5EED);
    let sh_d = epoch_shares(&mut srng, &dissolved);
    println!("魂散者：1 個 {:.1}%　2 個 {:.1}%　3 個 {:.1}%　4 個以上 {:.1}%", sh_d[0] * 100.0, sh_d[1] * 100.0, sh_d[2] * 100.0, sh_d[3] * 100.0);
    let sh_c = epoch_shares(&mut srng, &cohort_dissolved);
    println!("群體魂散者：1 個 {:.1}%　2 個 {:.1}%　3 個 {:.1}%　4 個以上 {:.1}%", sh_c[0] * 100.0, sh_c[1] * 100.0, sh_c[2] * 100.0, sh_c[3] * 100.0);
    let le2 = sh_d[0] + sh_d[1];
    println!("跨 1–2 個紀元者占魂散者 {:.1}%", le2 * 100.0);
    // 世界現行紀元表（對照）。
    let starts: Vec<u32> = w.eras.iter().map(|e| e.start_year).collect();
    let year_of = |t: u32| t as f64 / calendar::DAYS_PER_YEAR as f64;
    let mut real = [0u64; 4];
    for so in w.souls.iter().skip(1) {
        if !matches!(so.state, SoulState::Dissolved { .. }) {
            continue;
        }
        let (a, _) = age_of(so);
        let b = year_of(so.born);
        let n = 1 + starts.iter().filter(|&&y| (y as f64) > b && (y as f64) <= b + a).count();
        real[n.min(4) - 1] += 1;
    }
    let tot = real.iter().sum::<u64>().max(1) as f64;
    println!("（對照）世界現行紀元表（{} 個紀元，長度較短）：1 個 {:.1}%　2 個 {:.1}%　3 個 {:.1}%　4 個以上 {:.1}%", starts.len(), real[0] as f64 / tot * 100.0, real[1] as f64 / tot * 100.0, real[2] as f64 / tot * 100.0, real[3] as f64 / tot * 100.0);
    // 魂散者轉世次數。
    let mut lives: Vec<f64> = w.souls.iter().skip(1).filter(|so| matches!(so.state, SoulState::Dissolved { .. })).map(|so| so.rebirths() as f64).collect();
    lives.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("\n魂散者轉世次數：平均 {:.1}　中位 {:.0}　P90 {:.0}　最多 {:.0}", lives.iter().sum::<f64>() / lives.len().max(1) as f64, percentile(&lives, 0.5), percentile(&lives, 0.9), lives.last().copied().unwrap_or(0.0));
    // 診斷：期末仍在輪迴的魂的減緩係數、功德、宿慧、修為階分佈，以及近百年六道判定的去向。
    let mut eases: Vec<f64> = Vec::new();
    let mut merits: Vec<f64> = Vec::new();
    let mut wis: Vec<f64> = Vec::new();
    let mut ranks: Vec<f64> = Vec::new();
    for so in w.souls.iter().skip(1) {
        if !matches!(so.state, SoulState::Dissolved { .. } | SoulState::Liberated { .. }) {
            eases.push(siming_sim::soul::power::ease(so) as f64);
            merits.push(so.merit as f64);
            wis.push(so.wisdom as f64);
            ranks.push(siming_sim::soul::power::attainment_rank(so) as f64);
        }
    }
    for (name, v) in [("減緩係數", &mut eases), ("功德 G", &mut merits), ("宿慧 W", &mut wis), ("修為階", &mut ranks)] {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "在輪迴的魂 {name}：平均 {:.2}　中位 {:.2}　P90 {:.2}　P99 {:.2}　最大 {:.2}",
            v.iter().sum::<f64>() / v.len().max(1) as f64,
            percentile(v, 0.5),
            percentile(v, 0.9),
            percentile(v, 0.99),
            v.last().copied().unwrap_or(0.0)
        );
    }
    let mut paths = std::collections::BTreeMap::new();
    for e in w.ledger.events.iter().filter(|e| e.kind == siming_sim::EventKind::Judged && e.importance > 0) {
        if let siming_sim::EventArgs::Judged { path } = e.args {
            *paths.entry(format!("{path:?}")).or_insert(0u32) += 1;
        }
    }
    println!("近百年（因果帳留存）六道判定去向：{paths:?}");
    println!("期末世界雜湊 {:016x}　總耗時 {:.0} 秒", s.hash(), t0.elapsed().as_secs_f64());
    if let Some(path) = json_out {
        let doc = json!({
            "世界種子": seed, "亂數種子": rng_seed, "接手年": year0, "觀察年數": years, "對照組": baseline,
            "魂散者魂齡": j_all, "群體魂齡": j_cohort, "群體魂散者": j_cd, "直方": hist,
            "紀元數占比_魂散者": sh_d, "紀元數占比_群體魂散者": sh_c, "現行紀元表占比": real.map(|x| x as f64 / tot),
            "各時點": rows,
        });
        let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("{}：{e}", path.display()))?;
    }
    Ok(())
}
