//! GDD 2.6、4.6 開局落點：玩家接手的世界年落在住劫內（第 27,000–113,400 年），由種子或設定決定。

use std::collections::BTreeSet;
use std::time::Instant;

use siming_sim::params::{LANDING_YEAR_MAX, LANDING_YEAR_MIN};
use siming_sim::{landing_year, Landing, Sim, WorldParams};

/// 世界生成與開天各約 10–20 秒；三個種子並行，時間算最慢那個。
fn open(seed: u64, landing: Landing, rng: u64) -> (Sim, f64) {
    let mut p = WorldParams::new(seed);
    p.set_landing(landing);
    let t = Instant::now();
    let sim = Sim::new_seeded(&p, Some(rng)).unwrap();
    (sim, t.elapsed().as_secs_f64())
}

fn handover_year(sim: &Sim) -> u32 {
    sim.world().tick / 360
}

/// 紀元表在最長的世界（住劫終點）裡不重名、不靠序號補位，且開天總時間可接受。
fn check_chronicle(sim: &Sim) {
    let eras = &sim.world().eras;
    let names: BTreeSet<&str> = eras.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names.len(), eras.len(), "紀元重名");
    for e in eras {
        assert_eq!(e.name.chars().count(), 2, "紀元名「{}」不是兩個字（字庫用盡才會加序號）", e.name);
    }
    assert!(eras.windows(2).all(|w| w[0].start_year < w[1].start_year), "紀元起始年遞增");
}

#[test]
fn landing_year_of_a_seed_is_the_world_year_at_handover() {
    let results: Vec<_> = [(20261011u64, Landing::Late), (777, Landing::Random), (31415, Landing::Early)]
        .into_iter()
        .map(|(seed, landing)| {
            std::thread::spawn(move || {
                let (sim, secs) = open(seed, landing, 1);
                (seed, landing, sim, secs)
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    for (seed, landing, sim, secs) in &results {
        let year = handover_year(sim);
        assert_eq!(year, landing_year(*seed, *landing), "{seed} 的接手年");
        assert!((LANDING_YEAR_MIN..=LANDING_YEAR_MAX).contains(&year), "{seed}：{year}");
        check_chronicle(sim);
        eprintln!("種子 {seed} {landing:?} 接手於世界第 {year} 年 紀元 {} 個 開天 {secs:.1} 秒", sim.world().eras.len());
    }
}

#[test]
fn longest_world_has_a_clean_chronicle_and_a_fixed_land() {
    let seed = 20261011;
    let ((a, secs_a), (b, _)) = std::thread::scope(|s| {
        let x = s.spawn(|| open(seed, Landing::Year(LANDING_YEAR_MAX), 1));
        let y = s.spawn(|| open(seed, Landing::Year(LANDING_YEAR_MAX), 1));
        (x.join().unwrap(), y.join().unwrap())
    });
    assert_eq!(handover_year(&a), LANDING_YEAR_MAX);
    assert_eq!(a.hash(), b.hash(), "同種子、同落點、同亂數種子得到同一個世界");
    check_chronicle(&a);
    eprintln!("住劫終點：紀元 {} 個 開天 {secs_a:.1} 秒", a.world().eras.len());
}

#[test]
fn same_seed_same_land_whatever_the_rng() {
    let (a, _) = open(777, Landing::Year(LANDING_YEAR_MIN), 1);
    let (b, _) = open(777, Landing::Year(LANDING_YEAR_MIN), 2);
    let (ga, gb) = (&a.world().geo, &b.world().geo);
    assert_eq!(ga.height, gb.height);
    assert_eq!(ga.province_of, gb.province_of);
    assert_eq!(ga.river, gb.river);
    assert_eq!(a.world().eras.len(), b.world().eras.len(), "紀元表由種子與天地決定，與亂數種子無關");
    assert_eq!(a.world().eras.iter().map(|e| &e.name).collect::<Vec<_>>(), b.world().eras.iter().map(|e| &e.name).collect::<Vec<_>>());
}

/// 量測開天各段耗時（單獨執行：`cargo test --release --test landing timing -- --ignored --nocapture`）。
#[test]
#[ignore]
fn timing() {
    use siming_sim::genesis::Stage;
    for (seed, y) in [(20261011u64, LANDING_YEAR_MIN), (20261011, 70_000), (20261011, LANDING_YEAR_MAX), (777, LANDING_YEAR_MAX), (31415, LANDING_YEAR_MAX)] {
        let mut p = WorldParams::new(seed);
        p.set_landing(Landing::Year(y));
        let t = Instant::now();
        let mut marks: Vec<(String, f64)> = vec![];
        let mut last = None;
        let sim = Sim::new_with_progress_seeded(&p, Some(1), &mut |_, _| {}, &mut |st: Stage, _| {
            if last != Some(st) {
                marks.push((format!("{st:?}"), t.elapsed().as_secs_f64()));
                last = Some(st);
            }
        })
        .unwrap();
        let w = sim.world();
        let mut hist = [0u32; 16];
        for t in &w.geo.terrain {
            hist[*t as usize] += 1;
        }
        let pop: f64 = w.provinces.iter().map(|x| x.population).sum();
        eprintln!("統計 種子 {seed} 落點 {y}：人口 {pop:.0} 紀元 {} 地形格數 {hist:?} 海平面偏移 {:.4} 失陷州 {}", w.eras.len(), w.evolution.sea_shift, w.provinces.iter().filter(|p| p.lost).count());
        eprintln!("種子 {seed} 落點 {y}：各段起點 {marks:?} 總計 {:.1} 秒 紀元 {}", t.elapsed().as_secs_f64(), sim.world().eras.len());
    }
}
