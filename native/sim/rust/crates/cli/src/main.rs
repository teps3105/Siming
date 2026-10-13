//! GDD 6.7 siming-cli：無頭工具（gen、sim）。

mod render;

use clap::{Parser, Subcommand, ValueEnum};
use render::Layer;
use serde_json::json;
use siming_sim::{calendar, BookEntryKind, Era, PathState, Role, Sim, SoulState, WorldParams, WorldSize};
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
enum FunnelSize {
    Small,
    Medium,
    Large,
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
    /// 王朝量測（只量不改規則）：崩潰後各時點的無主大州與可立國人才，以及崩潰前天命 ΔM 各項的平均貢獻
    DynastyProbe {
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        yun: u32,
    },
    /// 成道漏斗統計（只讀世界與事件帳，不改規則）：觀察期內結束的每一世在哪一關被淘汰
    Funnel {
        /// 世界種子；人物期亂數種子預設同此值（固定，同參數兩次輸出相同）
        #[arg(long)]
        seed: u64,
        /// 觀察期長度（年），自預熱結束起算
        #[arg(long, default_value_t = 720)]
        years: u32,
        #[arg(long, value_enum, default_value = "medium")]
        size: FunnelSize,
        /// 預熱年數：開天與預跑完成後再跑這麼多年，其間出生者不計入母體
        #[arg(long, default_value_t = 60)]
        warmup: u32,
        /// 人物期亂數種子（預設取 --seed）
        #[arg(long)]
        rng: Option<u64>,
        /// 另存 JSON
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// 門人統計（GDD 3.4）：每世印世界人口、門人總數、入道率、五門占比、養分最低的門；--json 另存各州明細
    Adepts {
        /// 世界種子；人物期亂數種子預設同此值（固定，同參數兩次輸出相同）
        #[arg(long)]
        seed: u64,
        /// 開天與預跑完成後再跑的年數
        #[arg(long, default_value_t = 1000)]
        years: u32,
        #[arg(long, value_enum, default_value = "medium")]
        size: FunnelSize,
        /// 人物期亂數種子（預設取 --seed）
        #[arg(long)]
        rng: Option<u64>,
        /// 另存 JSON（每世一列的全局統計加期末各州明細）
        #[arg(long)]
        json: Option<PathBuf>,
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
        Cmd::Funnel { seed, years, size, warmup, rng, json } => funnel(seed, years, size, warmup, rng, json),
        Cmd::Adepts { seed, years, size, rng, json } => adepts(seed, years, size, rng, json),
        Cmd::DynastyProbe { seed, yun } => dynasty_probe(seed, yun),
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
    let lv = r["city_levels"].as_array().map(|a| a.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("/")).unwrap_or_default();
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

/// 一運的王朝統計（GDD 3.7；對照 7.1 平衡目標「王朝數」「每運王朝更替」「平時軍費佔比」）。
struct DynastyStats {
    line: String,
    json: serde_json::Value,
}

fn dynasty_stats(w: &siming_sim::World, lo: u32, hi: u32, samples: &[u32]) -> DynastyStats {
    use siming_sim::EventKind as K;
    let count = |k: K| w.ledger.events.iter().filter(|e| e.kind == k && e.tick > lo && e.tick <= hi).count();
    let collapsed = count(K::DynastyCollapsed);
    let founded = count(K::DynastyFounded);
    let usurp = count(K::Usurpation);
    let secede = count(K::Secession);
    let civil = count(K::CivilWar);
    let rebel = count(K::Rebellion);
    let annexed = count(K::Annexed);
    let wars = count(K::WarDeclared);
    let fengshan = count(K::Fengshan);
    let relief = count(K::Relief);
    let ban = count(K::BanTao);
    let list = &w.dynasty.list;
    let alive: Vec<&siming_sim::dynasty::Dynasty> = list.iter().filter(|d| d.fallen.is_none()).collect();
    let fallen: Vec<f64> = list.iter().filter_map(|d| d.fallen.map(|t| (t - d.founded) as f64 / 360.0)).collect();
    let mean_life = if fallen.is_empty() { f64::NAN } else { fallen.iter().sum::<f64>() / fallen.len() as f64 };
    let mean_n = samples.iter().map(|&v| v as f64).sum::<f64>() / samples.len().max(1) as f64;
    let (smin, smax) = (samples.iter().copied().min().unwrap_or(0), samples.iter().copied().max().unwrap_or(0));
    let (mut inc, mut mil, mut tre, mut mand, mut prov) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0usize);
    for d in &alive {
        let f = siming_sim::dynasty::finance(w, d.id as usize);
        inc += f.income;
        mil += f.military;
        tre += d.treasury;
        mand += d.mandate as f64;
        prov += d.provinces.len();
    }
    let land = w.provinces.iter().filter(|p| !p.lost).count().max(1);
    let n = alive.len().max(1) as f64;
    let share = if inc > 0.0 { mil / inc } else { 0.0 };
    let line = format!(
        "王朝 末 {} 均 {:.1}（區間 {}–{}）覆蓋 {:.0}%　更替(崩潰) {} 立國 {} 篡位 {} 割據 {} 內戰 {} 起義判定 {} 開疆 {} 宣戰 {} 封禪 {} 賑災 {} 禁道 {}　累計已亡 {} 個、平均壽命 {:.0} 年　在世均天命 {:.0}　軍費/收入 {:.0}%　國庫/年收入 {:.1}",
        alive.len(),
        mean_n,
        smin,
        smax,
        prov as f64 / land as f64 * 100.0,
        collapsed,
        founded,
        usurp,
        secede,
        civil,
        rebel,
        annexed,
        wars,
        fengshan,
        relief,
        ban,
        fallen.len(),
        mean_life,
        mand / n,
        share * 100.0,
        if inc > 0.0 { tre / inc } else { 0.0 },
    );
    let json = json!({
        "alive_end": alive.len(), "alive_mean": mean_n, "alive_min": smin, "alive_max": smax,
        "coverage": prov as f64 / land as f64,
        "changes": collapsed, "founded": founded, "usurpations": usurp, "secessions": secede, "civil_wars": civil,
        "rebellions": rebel, "annexed": annexed, "wars_declared": wars, "fengshan": fengshan, "relief": relief, "ban_tao": ban,
        "mean_lifespan_years": if mean_life.is_nan() { serde_json::Value::Null } else { json!(mean_life) },
        "fallen_total": fallen.len(),
        "mean_mandate": mand / n, "military_share": share,
        "treasury_years": if inc > 0.0 { tre / inc } else { 0.0 },
    });
    DynastyStats { line, json }
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
        let (mut e, mut eras) = evolution::genesis(s, era_of(era), 0);
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
    while s.world().tick < yun * MONTHS_PER_YUN * 30 {
        s.step();
        if s.world().tick % (MONTHS_PER_SHI * 30) == 30 {
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

/// 成道漏斗：開天預跑後先預熱，再逐月推進並由觀察者掃新事件，最後輸出文字表（與 JSON）。
fn funnel(seed: u64, years: u32, size: FunnelSize, warmup: u32, rng: Option<u64>, json_out: Option<PathBuf>) -> Result<(), String> {
    use siming_sim::funnel::Observer;
    let t0 = Instant::now();
    let mut params = WorldParams::new(seed);
    params.size = match size {
        FunnelSize::Small => WorldSize::Small,
        FunnelSize::Medium => WorldSize::Medium,
        FunnelSize::Large => WorldSize::Large,
    };
    let rng_seed = rng.unwrap_or(seed);
    let mut s = Sim::new_seeded(&params, Some(rng_seed)).map_err(|e| e.to_string())?;
    let setup_s = t0.elapsed().as_secs_f64();
    let mut obs = Observer::new(s.world(), warmup);
    let t1 = Instant::now();
    for _ in 0..(warmup + years) * 12 {
        s.step();
        s.drain_events();
        obs.observe(s.world());
    }
    let run_s = t1.elapsed().as_secs_f64();
    let meta = json!({
        "世界種子": s.world().params.seed,
        "人物期亂數種子": rng_seed,
        "尺寸": format!("{:?}", s.world().params.size),
        "觀察期（年）": years,
        "預熱（年）": warmup,
        "母體起點（tick）": obs.start_tick,
        "期末（tick）": s.world().tick,
        "期末世界雜湊": format!("{:016x}", s.hash()),
        "開天加預跑耗時（秒）": (setup_s * 10.0).round() / 10.0,
        "預熱加觀察耗時（秒）": (run_s * 10.0).round() / 10.0,
    });
    let report = obs.report(s.world(), meta);
    print!("{}", report.to_text());
    let bad = report.check();
    if !bad.is_empty() {
        eprintln!("總數核對不一致：{bad:?}");
    }
    if let Some(path) = json_out {
        let text = serde_json::to_string_pretty(&report.to_json()).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("{}：{e}", path.display()))?;
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err("總數核對不一致".into())
    }
}

/// 全局門人統計的一列。
fn adept_line(year: u32, sm: &siming_sim::adepts::Summary) -> String {
    use siming_sim::adepts::DOOR_NAMES;
    let sh = sm.shares();
    let low = sm.lowest_nourish().map_or("—".to_string(), |(d, v)| format!("{} {:.2}", DOOR_NAMES[d], v));
    format!(
        "{year:>6}年  人口 {:>10.0}  門人 {:>8.0}  入道率 {:>6.3}%  道 {:>5.1}%  佛 {:>5.1}%  儒 {:>5.1}%  魔 {:>5.1}%  散修 {:>5.1}%  養分最低 {low}",
        sm.population,
        sm.total,
        sm.rate() * 100.0,
        sh[0] * 100.0,
        sh[1] * 100.0,
        sh[2] * 100.0,
        sh[3] * 100.0,
        sh[4] * 100.0
    )
}

fn adepts(seed: u64, years: u32, size: FunnelSize, rng: Option<u64>, json_out: Option<PathBuf>) -> Result<(), String> {
    use siming_sim::adepts::{summary, Summary, DOOR_NAMES};
    let mut params = WorldParams::new(seed);
    params.size = match size {
        FunnelSize::Small => WorldSize::Small,
        FunnelSize::Medium => WorldSize::Medium,
        FunnelSize::Large => WorldSize::Large,
    };
    let rng_seed = rng.unwrap_or(seed);
    let mut s = Sim::new_seeded(&params, Some(rng_seed)).map_err(|e| e.to_string())?;
    let year0 = s.world().tick / 360;
    let row_json = |year: u32, sm: &Summary| {
        json!({
            "年": year, "世界人口": sm.population.round(), "門人總數": sm.total.round(), "入道率": sm.rate(),
            "五門占比": sm.shares(), "五門養分": sm.nourish,
        })
    };
    let mut rows = Vec::new();
    let sm = summary(s.world());
    println!("世界種子 {seed}　人物期亂數種子 {rng_seed}　尺寸 {:?}　接手年 {year0}（門序：{}）", s.world().params.size, DOOR_NAMES.join("、"));
    println!("{}", adept_line(year0, &sm));
    rows.push(row_json(year0, &sm));
    for y in 1..=years {
        for _ in 0..12 {
            s.step();
            s.drain_events();
        }
        if y % 30 == 0 || y == years {
            let sm = summary(s.world());
            println!("{}", adept_line(year0 + y, &sm));
            rows.push(row_json(year0 + y, &sm));
        }
    }
    println!("期末世界雜湊 {:016x}", s.hash());
    if let Some(path) = json_out {
        let w = s.world();
        let provinces: Vec<_> = w
            .provinces
            .iter()
            .zip(&w.adepts.provinces)
            .map(|(p, a)| {
                json!({
                    "州": p.id, "名": p.name, "人口": p.population.round(), "門人": a.n.map(|x| x.round()),
                    "占比": a.n.map(|x| if a.total() > 0.0 { x / a.total() } else { 0.0 }), "養分": a.nourish,
                    "濁氣": siming_sim::field::derive::turbid_of(p),
                })
            })
            .collect();
        let doc = json!({ "門序": DOOR_NAMES, "世界種子": seed, "期末年": year0 + years, "每世": rows, "各州": provinces });
        let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| format!("{}：{e}", path.display()))?;
    }
    Ok(())
}

fn sim(seed: u64, yun: u32, report: Option<PathBuf>) -> Result<(), String> {
    let t0 = Instant::now();
    let mut s = Sim::new(&WorldParams::new(seed)).map_err(|e| e.to_string())?;
    let setup_ms = t0.elapsed().as_millis();
    println!("最終種子 {}　開局時代 {:?}　開天＋預跑 {setup_ms} ms", s.world().params.seed, s.world().params.era);
    let mut rows = vec![row(&s)];
    print_row(&rows[0]);
    let shi_per_yun = MONTHS_PER_YUN / MONTHS_PER_SHI;
    let mut yuns = Vec::new();
    let mut prev_book: std::collections::BTreeMap<BookEntryKind, u32> = Default::default();
    let mut worst_all = (0u128, 0u32);
    for y in 0..yun {
        let (lo, hi) = (s.world().tick, s.world().tick + MONTHS_PER_YUN * 30);
        let ty = Instant::now();
        let (mut worst_yearly, mut worst_tick) = (0u128, 0u32);
        let (mut sum_yearly, mut n_yearly, mut sum_month, mut n_month) = (0u128, 0u32, 0u128, 0u32);
        // 個人與宗門判定分月後，年底月不再特別重：另記全部月份的最大與逐月（12 個月各自）平均。
        let mut worst_any = (0u128, 0u32);
        let mut by_month_us = [0u128; 12];
        let mut by_month_n = [0u32; 12];
        let mut yearly_us: Vec<u128> = Vec::new();
        let mut notable_samples = Vec::new();
        let mut dynasty_samples: Vec<u32> = Vec::new();
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
                let mi = ((s.world().tick - 1) / 30 % 12) as usize;
                by_month_us[mi] += us;
                by_month_n[mi] += 1;
                if us > worst_any.0 {
                    worst_any = (us, s.world().tick);
                }
                if s.world().tick % 360 == 0 {
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
            dynasty_samples.push(s.world().dynasty.alive_ids().len() as u32);
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
        let ds = dynasty_stats(w, lo, hi, &dynasty_samples);
        println!("  {}", ds.line);
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
        println!(
            "  耗時（全部月份）均 {:.2} ms　最大 {:.2} ms（tick {}）　逐月(1–12 月)均 ms {}",
            (sum_yearly + sum_month) as f64 / (n_yearly + n_month).max(1) as f64 / 1000.0,
            worst_any.0 as f64 / 1000.0,
            worst_any.1,
            (0..12).map(|i| format!("{:.2}", by_month_us[i] as f64 / by_month_n[i].max(1) as f64 / 1000.0)).collect::<Vec<_>>().join("/"),
        );
        let mut by_cal_month = [0u32; 12];
        for e in w.ledger.events.iter().filter(|e| e.tick > lo && e.tick <= hi && e.importance >= 2) {
            by_cal_month[(calendar::date(e.tick).month - 1) as usize] += 1;
        }
        println!("  重要度≥2 事件按月(1–12 月) {by_cal_month:?}");
        {
            use siming_sim::EventKind as K;
            let dyn_kinds = [
                K::DynastyFounded,
                K::Succession,
                K::Usurpation,
                K::CivilWar,
                K::Secession,
                K::Rebellion,
                K::Annexed,
                K::WarDeclared,
                K::BanTao,
                K::Fengshan,
                K::Relief,
                K::DynastyCollapsed,
            ];
            let (mut d, mut f, mut r) = ([0u32; 12], [0u32; 12], [0u32; 12]);
            for e in w.ledger.events.iter().filter(|e| e.tick > lo && e.tick <= hi) {
                let m = (calendar::date(e.tick).month - 1) as usize;
                if e.kind == K::Died {
                    d[m] += 1;
                } else if e.kind == K::Famine {
                    f[m] += 1;
                } else if dyn_kinds.contains(&e.kind) {
                    r[m] += 1;
                }
            }
            println!(
                "  壽終 Died 按月 {d:?}
  Famine 按月 {f:?}
  王朝事件按月 {r:?}"
            );
        }
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
            "dynasty": ds.json,
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

/// 王朝量測（只量不改規則）：
/// 1. 每個本次推演中崩潰的王朝，在崩潰當年與其後 1、2、5、10、20、50 年，數「無主且人口 ≥ 立國門檻的州」
///    與其中「有才略 ≥ 立國門檻的散人或武將候補」的州；
/// 2. 每個已崩潰的王朝，崩潰前 50 年天命 ΔM 各項的逐王朝年平均，再對王朝平均。
fn dynasty_probe(seed: u64, yun: u32) -> Result<(), String> {
    use siming_sim::dynasty::{probe_start, probe_take};
    use siming_sim::params::{DYN_FOUND_POP, OFFICIAL_MIN_AGE};
    probe_start();
    let mut s = Sim::new(&WorldParams::new(seed)).map_err(|e| e.to_string())?;
    let mut records = probe_take();
    let measure = |s: &Sim| -> (u32, u32) {
        let w = s.world();
        let (mut free, mut talented) = (0u32, 0u32);
        let mut has = vec![false; w.provinces.len()];
        for &i in &w.alive_ids {
            let p = &w.persons[i as usize];
            if p.power.is_none()
                && matches!(p.role, Role::Commoner | Role::GeneralCandidate)
                && p.age(w.tick) >= OFFICIAL_MIN_AGE
                && p.attrs.talent >= 0.0
            {
                if let Some(h) = has.get_mut(p.province as usize) {
                    *h = true;
                }
            }
        }
        for pr in &w.provinces {
            if !pr.lost && w.dynasty.owner_of(pr.id).is_none() && pr.population >= DYN_FOUND_POP {
                free += 1;
                if has[pr.id as usize] {
                    talented += 1;
                }
            }
        }
        (free, talented)
    };
    let mut known_fallen: std::collections::BTreeSet<u16> =
        s.world().dynasty.list.iter().filter(|d| d.fallen.is_some()).map(|d| d.id).collect();
    let offsets: [u32; 7] = [0, 1, 2, 5, 10, 20, 50];
    // (崩潰 tick, 各抽樣 tick)；本次推演才崩潰者才抽樣。
    let mut pending: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut samples: Vec<Vec<(u32, u32)>> = vec![Vec::new(); offsets.len()];
    let end = yun * MONTHS_PER_YUN * 30;
    println!("種子 {}　開局無主大州/有人才 {:?}", s.world().params.seed, measure(&s));
    while s.world().tick < end {
        s.step();
        let w = s.world();
        let fresh: Vec<(u16, u32)> = w
            .dynasty
            .list
            .iter()
            .filter(|d| d.fallen.is_some() && !known_fallen.contains(&d.id))
            .map(|d| (d.id, d.fallen.unwrap()))
            .collect();
        for (id, f) in fresh {
            known_fallen.insert(id);
            pending.push((f, offsets.iter().map(|o| f + o * 360).collect()));
        }
        let tick = w.tick;
        let due: Vec<usize> =
            pending.iter().flat_map(|(_, ts)| ts.iter().enumerate().filter(|(_, &t)| t == tick).map(|(oi, _)| oi)).collect();
        for oi in due {
            samples[oi].push(measure(&s));
        }
    }
    records.extend(probe_take());
    println!("本次推演崩潰的王朝 {} 個（開局前已崩潰的不抽樣）", pending.len());
    println!("崩潰後 N 年　樣本數　無主大州均/最小/最大　有人才的州均/最小/最大");
    for (oi, o) in offsets.iter().enumerate() {
        let v = &samples[oi];
        if v.is_empty() {
            println!("  {o:>2} 年　0");
            continue;
        }
        let n = v.len() as f64;
        let (f, t): (Vec<u32>, Vec<u32>) = v.iter().copied().unzip();
        println!(
            "  {o:>2} 年　{}　{:.1}/{}/{}　{:.1}/{}/{}",
            v.len(),
            f.iter().sum::<u32>() as f64 / n,
            f.iter().min().unwrap(),
            f.iter().max().unwrap(),
            t.iter().sum::<u32>() as f64 / n,
            t.iter().min().unwrap(),
            t.iter().max().unwrap()
        );
    }
    let w = s.world();
    let mut per_dyn: Vec<[f64; 10]> = Vec::new();
    for d in w.dynasty.list.iter().filter(|d| d.fallen.is_some()) {
        let fall = d.fallen.unwrap();
        let rows: Vec<_> = records.iter().filter(|(t, id, _)| *id == d.id && *t <= fall && *t + 50 * 360 > fall).collect();
        if rows.is_empty() {
            continue;
        }
        let n = rows.len() as f64;
        let mut acc = [0f64; 10];
        for (_, _, t) in &rows {
            let v = [t.ruler, t.officials, t.righteous, t.karma, t.aura, t.famine, t.gain_loss, -t.cycle, t.broke, t.total()];
            for i in 0..10 {
                acc[i] += v[i] as f64 / n;
            }
        }
        per_dyn.push(acc);
    }
    println!("崩潰前 50 年天命 ΔM 各項的平均貢獻（先對各王朝取年平均，再對 {} 個已崩潰王朝平均）：", per_dyn.len());
    let names = ["君主德行", "文臣德行", "正氣", "業氣", "風水(都城靈氣)", "饑荒", "得失州", "週期項", "國庫不足", "合計ΔM"];
    for (i, nm) in names.iter().enumerate() {
        let m = per_dyn.iter().map(|a| a[i]).sum::<f64>() / per_dyn.len().max(1) as f64;
        println!("  {nm:<14} {m:+.3}");
    }
    println!("  德行合計(君主+文臣) {:+.3}", per_dyn.iter().map(|a| a[0] + a[1]).sum::<f64>() / per_dyn.len().max(1) as f64);
    Ok(())
}
