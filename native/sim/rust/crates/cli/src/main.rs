//! GDD 6.7 siming-cli：無頭工具（gen、sim）。

mod render;

use siming_sim::{calendar, BookEntryKind, Era, PathState, Role, Sim, SoulState, WorldParams, WorldSize};
use clap::{Parser, Subcommand, ValueEnum};
use render::Layer;
use serde_json::json;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "siming-cli", about = "Siming 無頭工具（GDD 6.7）")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum SizeArg {
    S,
    M,
    L,
}

#[derive(Clone, Copy, ValueEnum)]
enum EraArg {
    Ancient,
    Middle,
    Decline,
}

#[derive(Subcommand)]
enum Cmd {
    /// 生成世界並匯出圖檔（GDD 2.2–2.4）
    Gen {
        #[arg(long)]
        seed: u64,
        #[arg(long, value_enum, default_value = "m")]
        size: SizeArg,
        #[arg(long, default_value_t = siming_sim::params::WG_LAND_RATIO)]
        land: f32,
        #[arg(long, default_value_t = siming_sim::params::WG_RELIEF)]
        relief: f32,
        #[arg(long, default_value_t = siming_sim::params::WG_HUMIDITY)]
        humidity: f32,
        #[arg(long, value_enum, default_value = "middle")]
        era: EraArg,
        #[arg(long, value_enum, default_value = "atlas")]
        layer: Layer,
        /// 每格像素數
        #[arg(long, default_value_t = 4)]
        scale: usize,
        #[arg(long)]
        out: PathBuf,
    },
    /// 無頭跑 N 運（每運 4320 個月），每世（360 個月）印一行統計
    Sim {
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        yun: u32,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// 無頭跑 N 運，每世印一行天地的慢變量：寒暖、海平面偏移、靈潮（GDD 2.8）
    Evolve {
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 3)]
        yun: u32,
        #[arg(long, value_enum, default_value = "middle")]
        era: EraArg,
        /// 改為統計：從 --seed 起連續這麼多個種子，只算慢變量（不生成世界），印各變量極值的分佈
        #[arg(long)]
        seeds: Option<u32>,
    },
}

const MONTHS_PER_SHI: u32 = 360;
const MONTHS_PER_YUN: u32 = 4320;

fn main() {
    let cli = Cli::parse();
    let r = match cli.cmd {
        Cmd::Gen { seed, size, land, relief, humidity, era, layer, scale, out } => {
            gen(seed, size, land, relief, humidity, era, layer, scale, out)
        }
        Cmd::Sim { seed, yun, report } => sim(seed, yun, report),
        Cmd::Evolve { seed, yun, era, seeds: None } => evolve(seed, yun, era),
        Cmd::Evolve { seed, yun, era, seeds: Some(n) } => evolve_survey(seed, yun, era, n),
    };
    if let Err(e) = r {
        eprintln!("錯誤：{e}");
        std::process::exit(1);
    }
}

#[allow(clippy::too_many_arguments)]
fn gen(
    seed: u64,
    size: SizeArg,
    land: f32,
    relief: f32,
    humidity: f32,
    era: EraArg,
    layer: Layer,
    scale: usize,
    out: PathBuf,
) -> Result<(), String> {
    let mut p = WorldParams::new(seed);
    p.size = match size {
        SizeArg::S => WorldSize::Small,
        SizeArg::M => WorldSize::Medium,
        SizeArg::L => WorldSize::Large,
    };
    p.land_ratio = land;
    p.relief = relief;
    p.humidity = humidity;
    p.era = match era {
        EraArg::Ancient => Era::Ancient,
        EraArg::Middle => Era::Middle,
        EraArg::Decline => Era::Decline,
    };
    let t0 = Instant::now();
    let debug = std::env::var_os("SIMING_WG_DEBUG").is_some();
    let mut diag = |seed: u64, reason: &str| {
        if debug {
            eprintln!("seed {seed}: {reason}");
        }
    };
    let sim = Sim::new_with_diag(&p, &mut diag).map_err(|e| e.to_string())?;
    let ms = t0.elapsed().as_millis();
    let w = sim.world();
    let geo = &w.geo;
    // 陸地比例：面積加權，陸地 = 非海格（GDD 2.2）。
    let (mut land_area, mut total_area) = (0.0f32, 0.0f32);
    for (c, t) in geo.terrain.iter().enumerate() {
        let a = geo.area_weight(c as u32);
        total_area += a;
        if *t != siming_sim::Terrain::Sea {
            land_area += a;
        }
    }
    println!("最終種子：{}", w.params.seed);
    println!("世界名：{}", w.params.name);
    println!("生成時間：{ms} ms");
    println!("州郡數：{}", w.provinces.len());
    println!("陸地比例：{:.3}", land_area / total_area.max(1e-6));
    println!("江河：{}　山脈：{}　洞天：{}", geo.rivers.len(), geo.ranges.len(), geo.nodes.len());

    let cv = render::render(geo, &w.fine, &w.provinces, layer, scale);
    cv.write_png(&out)?;
    let mut side = out.clone().into_os_string();
    side.push(".txt");
    std::fs::write(&side, render::sidecar(geo, &w.provinces)).map_err(|e| e.to_string())?;
    println!("已寫入 {}（{}x{}）", out.display(), cv.w, cv.h);
    Ok(())
}

fn row(sim: &Sim) -> serde_json::Value {
    let w = sim.world();
    let pop: f64 = w.provinces.iter().map(|p| p.population).sum();
    let mut lv = [0u32; 5];
    for p in &w.provinces {
        lv[(p.city_level as usize).min(4)] += 1;
    }
    json!({
        "tick": w.tick,
        "date": calendar::era_date(w.tick, w.evolution.history_years, &w.eras),
        "population": pop,
        "provinces": w.provinces.len(),
        "city_levels": lv,
        "hash": format!("{:016x}", sim.hash()),
    })
}

fn print_row(r: &serde_json::Value) {
    let lv = r["city_levels"]
        .as_array()
        .map(|a| a.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("/"))
        .unwrap_or_default();
    println!(
        "{}  人口 {:.0}  州郡 {}（村/鎮/縣/州/都 {}）  hash {}",
        r["date"].as_str().unwrap_or(""),
        r["population"].as_f64().unwrap_or(0.0),
        r["provinces"],
        lv,
        r["hash"].as_str().unwrap_or(""),
    );
}

/// 一個取樣點的境界分布（路線圖 M2 指標：在世的修士、散修、魔修依境界的人數）。
fn realm_sample(w: &siming_sim::World) -> [u32; 9] {
    let mut d = [0u32; 9];
    for p in w.persons.iter().filter(|p| p.alive()) {
        if let (PathState::Cultivator { realm, .. }, Role::Cultivator | Role::Demonic) = (&p.path, p.role) {
            d[(*realm as usize).min(8)] += 1;
        }
    }
    d
}

fn notables_alive(w: &siming_sim::World) -> u32 {
    w.persons.iter().filter(|p| p.alive()).count() as u32
}

/// 某 tick 區間 (lo, hi] 內的宿緣建立／了結數。`Genesis` 建立的（tick 0）不計。
fn thread_counts(w: &siming_sim::World, lo: u32, hi: u32) -> ([u32; 5], u32, [u32; 5], u32) {
    let (mut resolved, mut faded) = (0, 0);
    let mut created = [0u32; 5];
    let mut by_kind = [0u32; 5];
    for t in &w.threads {
        if t.created.0 > lo && t.created.0 <= hi {
            created[t.kind as usize] += 1;
        }
        if let Some((rt, _)) = t.resolved {
            if rt > lo && rt <= hi {
                resolved += 1;
                by_kind[t.kind as usize] += 1;
            }
        } else if t.strength <= 0.0 {
            faded += 1;
        }
    }
    (created, resolved, by_kind, faded)
}

/// 區間內了結的宿緣依「了結當下的 lives」分組：0、1、≥ 2（事件參數；重要度 ≥ 4 永久保存）。
fn resolved_by_lives(w: &siming_sim::World, lo: u32, hi: u32) -> [[u32; 3]; 5] {
    let mut out = [[0u32; 3]; 5];
    for t in &w.threads {
        let Some((rt, eid)) = t.resolved else { continue };
        if rt <= lo || rt > hi {
            continue;
        }
        if let Some(siming_sim::EventArgs::Thread { lives, .. }) = siming_sim::ledger::get(w, eid).map(|e| &e.args) {
            out[t.kind as usize][(*lives as usize).min(2)] += 1;
        }
    }
    out
}

/// 本執行緒累計的 CPU 時間（奈秒）；僅 Linux（`/proc/thread-self/schedstat`），其他平台回傳 `None`。
fn cpu_ns() -> Option<u64> {
    std::fs::read_to_string("/proc/thread-self/schedstat").ok()?.split_whitespace().next()?.parse().ok()
}

fn era_of(era: EraArg) -> Era {
    match era {
        EraArg::Ancient => Era::Ancient,
        EraArg::Middle => Era::Middle,
        EraArg::Decline => Era::Decline,
    }
}

/// GDD 2.8：多個種子的統計（只算慢變量與紀元年表；世界生成重試換種子的情況不在內）。
fn evolve_survey(seed: u64, yun: u32, era: EraArg, seeds: u32) -> Result<(), String> {
    use siming_sim::era::{EraKind, Lexicon};
    use siming_sim::evolution;
    let lexicon = Lexicon::builtin();
    let span = yun * MONTHS_PER_YUN / 12;
    // 各種子的：寒暖最高、寒暖最低、靈潮最高、靈潮最低。
    let mut extremes: [Vec<f32>; 4] = Default::default();
    let (mut changes, mut lengths, mut opening_year, mut before) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut kinds = std::collections::BTreeMap::<&str, u32>::new();
    let mut opening_kinds = std::collections::BTreeMap::<&str, u32>::new();
    for i in 0..seeds {
        let s = seed + u64::from(i);
        let (mut e, mut eras) = evolution::genesis(s, era_of(era));
        let opening = e.history_years + 1;
        let current = eras.last().expect("年表不空");
        *opening_kinds.entry(current.kind.key()).or_default() += 1;
        opening_year.push(opening - current.start_year + 1);
        before.push(eras.len() as u32);
        let known = eras.len();
        let (mut warm, mut cold, mut high, mut low) = (e.warmth, e.warmth, e.tide, e.tide);
        let mut year = e.history_years + 30;
        while year <= e.history_years + span {
            let c = e.settle(year);
            evolution::open_era(&e, &c, year, s, &lexicon, &mut eras);
            warm = warm.max(e.warmth);
            cold = cold.min(e.warmth);
            high = high.max(e.tide);
            low = low.min(e.tide);
            year += 30;
        }
        for (list, v) in extremes.iter_mut().zip([warm, cold, high, low]) {
            list.push(v);
        }
        changes.push((eras.len() - known) as u32);
        for pair in eras[known - 1..].windows(2) {
            lengths.push(pair[1].start_year - pair[0].start_year);
        }
        for x in &eras[known..] {
            *kinds.entry(x.kind.key()).or_default() += 1;
        }
        let _ = EraKind::ALL;
    }
    println!("{seeds} 個種子（自 {seed} 起）　開局時代 {:?}　{yun} 運", era_of(era));
    println!("{:<12} {:>7} {:>7} {:>7}", "", "最小", "中位", "最大");
    for (label, list) in ["寒暖最高", "寒暖最低", "靈潮最高", "靈潮最低"].iter().zip(extremes.iter_mut()) {
        list.sort_by(|a, b| a.total_cmp(b));
        println!("{:<12} {:>7.3} {:>7.3} {:>7.3}", label, list[0], list[list.len() / 2], list[list.len() - 1]);
    }
    for (label, list) in [
        ("開局後改元次數", &mut changes),
        ("紀元長度（年）", &mut lengths),
        ("開局是紀元第幾年", &mut opening_year),
        ("開局前紀元數", &mut before),
    ] {
        list.sort_unstable();
        if list.is_empty() {
            continue;
        }
        println!("{:<12} {:>7} {:>7} {:>7}", label, list[0], list[list.len() / 2], list[list.len() - 1]);
    }
    let within = changes.iter().filter(|c| (2..=6).contains(*c)).count() * 100 / changes.len();
    println!("開局後改元 2–6 次的種子：{within}%");
    let share = |m: &std::collections::BTreeMap<&str, u32>| {
        let total: u32 = m.values().sum();
        m.iter().map(|(k, v)| format!("{k} {}%", v * 100 / total.max(1))).collect::<Vec<_>>().join("　")
    };
    println!("開局所在的紀元：{}", share(&opening_kinds));
    println!("開局後開啟的紀元：{}", share(&kinds));
    Ok(())
}

/// GDD 2.8：跑完整的推演，印開局前的紀元年表、每世的慢變量與開局後的改元。
fn evolve(seed: u64, yun: u32, era: EraArg) -> Result<(), String> {
    let mut params = WorldParams::new(seed);
    params.era = era_of(era);
    let mut s = Sim::new(&params).map_err(|e| e.to_string())?;
    let e = s.world().evolution.clone();
    println!(
        "最終種子 {}　開局時代 {:?}　開局前的歷史 {} 年　寒暖週期 {} 年與 {} 年　靈潮週期 {} 年",
        s.world().params.seed,
        s.world().params.era,
        e.history_years,
        e.warmth_periods[0],
        e.warmth_periods[1],
        e.tide_period
    );
    println!("開局前的紀元年表：");
    for x in &s.world().eras {
        println!("  第 {:>5} 年起　{}紀元（{}）", x.start_year, x.name, x.kind.key());
    }
    println!("開局日期：{}", calendar::full(s.world().tick, e.history_years, &s.world().eras));
    println!("{:>6} {:>8} {:>10} {:>7}  紀元", "年", "寒暖", "海平面", "靈潮");
    let line = |s: &Sim| {
        let w = s.world();
        let e = &w.evolution;
        let year = calendar::total_year(w.tick, e.history_years);
        let (era, n) = calendar::era_year(w.tick, e.history_years, &w.eras);
        println!("{:>6} {:>8.3} {:>10.4} {:>7.3}  {}紀元 {}年", year, e.warmth, e.sea_shift, e.tide, era.name, n);
    };
    line(&s);
    let known = s.world().eras.len();
    while s.world().tick < yun * MONTHS_PER_YUN {
        s.step();
        if s.world().tick % MONTHS_PER_SHI == 1 {
            // 結算在每世最後一個月；下一個月印，新紀元已是元年。
            line(&s);
        }
    }
    let after = &s.world().eras[known..];
    println!("開局後改元 {} 次：", after.len());
    for x in after {
        println!("  第 {:>5} 年起　{}紀元（{}）", x.start_year, x.name, x.kind.key());
    }
    let events = s.world().ledger.events.iter().filter(|e| e.kind == siming_sim::EventKind::WorldChanged).count();
    println!("因果帳裡的世界大事 {events} 筆");
    Ok(())
}

fn sim(seed: u64, yun: u32, report: Option<PathBuf>) -> Result<(), String> {
    let t0 = Instant::now();
    let mut s = Sim::new(&WorldParams::new(seed)).map_err(|e| e.to_string())?;
    let setup_ms = t0.elapsed().as_millis();
    println!(
        "最終種子 {}　開局時代 {:?}　開天＋預跑 {setup_ms} ms",
        s.world().params.seed,
        s.world().params.era
    );
    let mut rows = vec![row(&s)];
    print_row(&rows[0]);
    let shi_per_yun = MONTHS_PER_YUN / MONTHS_PER_SHI;
    let mut yuns = Vec::new();
    let mut prev_book: std::collections::BTreeMap<BookEntryKind, u32> = Default::default();
    let mut worst_all = (0u128, 0u32);
    for y in 0..yun {
        let (lo, hi) = (s.world().tick, s.world().tick + MONTHS_PER_YUN);
        let ty = Instant::now();
        let (mut worst_yearly, mut worst_tick) = (0u128, 0u32);
        let (mut sum_yearly, mut n_yearly, mut sum_month, mut n_month) = (0u128, 0u32, 0u128, 0u32);
        let mut yearly_us: Vec<u128> = Vec::new();
        let mut notable_samples = Vec::new();
        let mut realm_samples: Vec<[u32; 9]> = Vec::new();
        for shi in 0..shi_per_yun {
            for _ in 0..MONTHS_PER_SHI {
                let (ts, c0) = (Instant::now(), cpu_ns());
                s.step();
                // 優先用本執行緒的 CPU 時間（機器忙碌時較不受排程干擾）；讀不到時用牆鐘。
                let us = match (c0, cpu_ns()) {
                    (Some(a), Some(b)) => (b.saturating_sub(a) / 1000) as u128,
                    _ => ts.elapsed().as_micros(),
                };
                if s.world().tick % 12 == 0 {
                    sum_yearly += us;
                    n_yearly += 1;
                    yearly_us.push(us);
                    if us > worst_yearly {
                        worst_yearly = us;
                        worst_tick = s.world().tick;
                    }
                } else {
                    sum_month += us;
                    n_month += 1;
                }
            }
            notable_samples.push(notables_alive(s.world()));
            // 每運 4 個取樣點：第 3、6、9、12 世末。
            if (shi + 1) % (shi_per_yun / 4) == 0 {
                realm_samples.push(realm_sample(s.world()));
            }
            let r = row(&s);
            print_row(&r);
            rows.push(r);
        }
        let w = s.world();
        let mut mean = [0f64; 9];
        for smp in &realm_samples {
            for i in 0..9 {
                mean[i] += smp[i] as f64 / realm_samples.len() as f64;
            }
        }
        let ratios: Vec<f64> = (1..9).map(|i| if mean[i - 1] > 0.0 { mean[i] / mean[i - 1] } else { f64::NAN }).collect();
        let (created, resolved, by_kind, faded) = thread_counts(w, lo, hi);
        let by_lives = resolved_by_lives(w, lo, hi);
        let mut lib = [0u32; 3];
        for so in &w.souls {
            if let SoulState::Liberated { way, tick } = so.state {
                if tick > lo && tick <= hi {
                    lib[way as usize] += 1;
                }
            }
        }
        let mut book = serde_json::Map::new();
        for (k, rec) in &w.book.entries {
            let d = rec.count - prev_book.get(k).copied().unwrap_or(0);
            if d > 0 {
                book.insert(format!("{k:?}"), json!(d));
            }
            prev_book.insert(*k, rec.count);
        }
        let alive_by_role = |r: Role| w.persons.iter().filter(|p| p.alive() && p.role == r).count();
        let sects = w.powers.iter().filter(|p| p.fallen.is_none()).count();
        let ym = ty.elapsed().as_millis();
        yearly_us.sort_unstable();
        let p95 = yearly_us.get(yearly_us.len().saturating_sub(1) * 95 / 100).copied().unwrap_or(0);
        println!(
            "── 運 {} ──\n  在世有名之人 末 {} 均 {:.0}（修士 {} 僧 {} 儒 {} 魔 {}）　宗門 {}",
            y + 1,
            notable_samples.last().copied().unwrap_or(0),
            notable_samples.iter().map(|&v| v as f64).sum::<f64>() / notable_samples.len() as f64,
            alive_by_role(Role::Cultivator),
            alive_by_role(Role::Monk),
            alive_by_role(Role::Scholar),
            alive_by_role(Role::Demonic),
            sects,
        );
        println!(
            "  境界分布(4點均) {}　逐境比 {}",
            mean.iter().map(|v| format!("{v:.1}")).collect::<Vec<_>>().join("/"),
            ratios.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join("/"),
        );
        println!(
            "  飛升 {} 涅槃 {} 成聖 {}　宿緣 建立 {}（殺業/恩義/情緣/師徒/怨 {:?}）了結 {}（同序 {:?}；殺業/恩義/情緣/師徒/怨 × lives 0/1/≥2 {:?}）消散累計 {}",
            lib[0], lib[1], lib[2], created.iter().sum::<u32>(), created, resolved, by_kind, by_lives, faded
        );
        println!("  天書 {}", serde_json::Value::Object(book.clone()));
        println!(
            "  規模 人物 {} 魂 {} 宿緣 {}（未了 {}）事件 {}",
            w.persons.len(),
            w.souls.len(),
            w.threads.len(),
            w.threads.iter().filter(|t| siming_sim::thread::is_open(t)).count(),
            w.ledger.events.len()
        );
        println!(
            "  耗時 牆鐘 {ym} ms／步進 {} ms　年底月 最差 {:.2} ms（tick {worst_tick}）P95 {:.2} ms 均 {:.2} ms　一般月 均 {:.2} ms",
            (sum_yearly + sum_month) / 1000,
            worst_yearly as f64 / 1000.0,
            p95 as f64 / 1000.0,
            sum_yearly as f64 / n_yearly.max(1) as f64 / 1000.0,
            sum_month as f64 / n_month.max(1) as f64 / 1000.0,
        );
        if worst_yearly > worst_all.0 {
            worst_all = (worst_yearly, worst_tick);
        }
        yuns.push(json!({
            "yun": y + 1,
            "notables_alive_per_shi": notable_samples,
            "realm_samples": realm_samples,
            "realm_mean": mean,
            "realm_ratios": ratios,
            "ascensions": lib[0], "nirvana": lib[1], "sagehood": lib[2],
            "threads_created": created.iter().sum::<u32>(), "threads_created_by_kind": created, "threads_resolved": resolved, "threads_resolved_by_lives": by_lives,
            "threads_resolved_by_kind": by_kind, "threads_faded_total": faded,
            "book": book,
            "elapsed_ms": ym as u64,
            "worst_yearly_month_ms": worst_yearly as f64 / 1000.0,
            "worst_yearly_month_tick": worst_tick,
            "p95_yearly_month_ms": p95 as f64 / 1000.0,
            "mean_yearly_month_ms": sum_yearly as f64 / n_yearly.max(1) as f64 / 1000.0,
            "mean_plain_month_ms": sum_month as f64 / n_month.max(1) as f64 / 1000.0,
        }));
    }
    let ms = t0.elapsed().as_millis();
    let hash = format!("{:016x}", s.hash());
    println!("最終 hash {hash}　耗時 {ms} ms　最差年底月 {:.2} ms", worst_all.0 as f64 / 1000.0);
    if let Some(path) = report {
        let j = json!({
            "seed": s.world().params.seed,
            "yun": yun,
            "rows": rows,
            "yuns": yuns,
            "final_hash": hash,
            "elapsed_ms": ms as u64,
            "setup_ms": setup_ms as u64,
            "worst_yearly_month_ms": worst_all.0 as f64 / 1000.0,
        });
        let txt = serde_json::to_string_pretty(&j).map_err(|e| e.to_string())?;
        std::fs::write(&path, txt).map_err(|e| e.to_string())?;
    }
    Ok(())
}
