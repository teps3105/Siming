//! GDD 6.7 平衡迴歸、路線圖 M2 驗收：3 個種子各 3 運（無量劫版本）的統計落在 7.1「平衡目標」內。
//!
//! 預設忽略（約 20 秒，需 `--release`）；CI 每晚以 `cargo test -p siming-sim --release -- --ignored balance` 執行。
//! 指標定義與 `siming-cli sim` 報表相同。「每運宿緣了結 30–120」尚待規則決策（6.10 #34），暫不斷言，只印出。

use siming_sim::{BookEntryKind, Era, PathState, Role, Sim, SoulState, WorldParams};

const MONTHS_PER_SHI: u32 = 360;
const SHI_PER_YUN: u32 = 12;
const SEEDS: [u64; 3] = [1, 2, 3];

struct YunStats {
    notables_min: u32,
    notables_max: u32,
    /// 4 個取樣點（第 3、6、9、12 世末）的境界人數平均。
    realm_mean: [f64; 9],
    ascensions: u32,
    threads_resolved: u32,
}

fn realms(sim: &Sim) -> [u32; 9] {
    let mut d = [0u32; 9];
    for p in sim.world().persons.iter().filter(|p| p.alive()) {
        if let (PathState::Cultivator { realm, .. }, Role::Cultivator | Role::Demonic) = (&p.path, p.role) {
            d[(*realm as usize).min(8)] += 1;
        }
    }
    d
}

fn run(seed: u64, yuns: u32) -> (Sim, Vec<YunStats>) {
    let mut sim = Sim::new(&WorldParams::new(seed)).expect("世界生成");
    let mut out = Vec::new();
    for _ in 0..yuns {
        let lo = sim.world().tick;
        let (mut nmin, mut nmax) = (u32::MAX, 0);
        let mut samples: Vec<[u32; 9]> = Vec::new();
        for shi in 0..SHI_PER_YUN {
            for _ in 0..MONTHS_PER_SHI {
                sim.step();
            }
            let n = sim.world().persons.iter().filter(|p| p.alive()).count() as u32;
            nmin = nmin.min(n);
            nmax = nmax.max(n);
            if (shi + 1) % (SHI_PER_YUN / 4) == 0 {
                samples.push(realms(&sim));
            }
        }
        let hi = sim.world().tick;
        let mut realm_mean = [0f64; 9];
        for s in &samples {
            for i in 0..9 {
                realm_mean[i] += s[i] as f64 / samples.len() as f64;
            }
        }
        let w = sim.world();
        let ascensions = w
            .souls
            .iter()
            .filter(|s| matches!(s.state, SoulState::Liberated { way: siming_sim::Liberation::Ascension, tick } if tick > lo && tick <= hi))
            .count() as u32;
        let threads_resolved =
            w.threads.iter().filter(|t| t.resolved.is_some_and(|(rt, _)| rt > lo && rt <= hi)).count() as u32;
        out.push(YunStats { notables_min: nmin, notables_max: nmax, realm_mean, ascensions, threads_resolved });
    }
    (sim, out)
}

/// 最小平方法擬合 ln(人數) 對境界的斜率，回傳逐境比 exp(斜率)；只取平均人數 ≥ 0.5 的境界。
fn fitted_ratio(mean: &[f64; 9]) -> f64 {
    let pts: Vec<(f64, f64)> = mean.iter().enumerate().filter(|(_, &v)| v >= 0.5).map(|(i, &v)| (i as f64, v.ln())).collect();
    let n = pts.len() as f64;
    let (sx, sy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let (sxx, sxy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0 * p.0, a.1 + p.0 * p.1));
    ((n * sxy - sx * sy) / (n * sxx - sx * sx)).exp()
}

#[test]
#[ignore = "約 20 秒；CI 每晚以 --ignored 執行"]
fn balance_three_seeds_three_yun() {
    let (mut ascensions_23, mut cells) = (0u32, 0u32);
    let mut three_lives_feud = false;
    for seed in SEEDS {
        let (sim, yuns) = run(seed, 3);
        let w = sim.world();
        assert_eq!(w.params.era, Era::Middle, "開局時代為中古");
        three_lives_feud |= w.book.entries.contains_key(&BookEntryKind::ThreeLivesFeud);

        // 在世有名之人 2,000–4,000（第 1 運含成長期，從第 2 運起檢查）。
        for (i, y) in yuns.iter().enumerate().skip(1) {
            assert!(y.notables_min >= 2_000 && y.notables_max <= 4_000, "seed {seed} 運{} 在世 {}..{}", i + 1, y.notables_min, y.notables_max);
        }
        // 境界分布每高一境約 ×0.3–0.6（第 2、3 運）。
        for (i, y) in yuns.iter().enumerate().skip(1) {
            let r = fitted_ratio(&y.realm_mean);
            assert!((0.3..=0.6).contains(&r), "seed {seed} 運{} 逐境比 {r:.2} 分布 {:?}", i + 1, y.realm_mean);
        }
        // 首次飛升 ≤ 2 運。
        assert!(yuns.iter().take(2).map(|y| y.ascensions).sum::<u32>() >= 1, "seed {seed} 前 2 運沒有飛升");
        for y in yuns.iter().skip(1) {
            ascensions_23 += y.ascensions;
            cells += 1;
        }
        eprintln!(
            "seed {seed} 飛升 {:?} 宿緣了結 {:?}（目標 30–120，待規則決策）",
            yuns.iter().map(|y| y.ascensions).collect::<Vec<_>>(),
            yuns.iter().map(|y| y.threads_resolved).collect::<Vec<_>>()
        );
    }
    // 第 2、3 運每運飛升 1–3：單一運的飛升數變異大（0–5），以所有種子的平均判定。
    let mean = ascensions_23 as f64 / cells as f64;
    assert!((1.0..=3.0).contains(&mean), "第 2、3 運平均每運飛升 {mean:.2}");
    assert!(three_lives_feud, "至少一個種子觸發「三世血仇」");
}
