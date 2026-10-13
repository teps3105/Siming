//! 門人層的測試（GDD 3.4「凡人、門人與出頭」）：入道率、五門分布、人口驟減、濁氣、確定性、加成。

use super::*;
use crate::soul;
use crate::types::{Geography, Province, Terrain, WorldParams};

const POP: f64 = 1_000_000.0;

fn province(id: u16, n: usize) -> Province {
    let mut neighbors = vec![];
    if id > 0 {
        neighbors.push(id - 1);
    }
    if (id as usize) + 1 < n {
        neighbors.push(id + 1);
    }
    Province {
        id,
        name: format!("p{id}"),
        cells: vec![id as u32],
        seat: id as u32,
        neighbors,
        sea_neighbors: vec![],
        terrain_mix: [0; Terrain::COUNT],
        habitability: 1.0,
        capacity: POP * 2.0,
        population: POP,
        aura: 0.5,
        karma_qi: 0.0,
        merit: 0.0,
        righteous: 0.0,
        devastation: 0.0,
        faith: [0.0; 3],
        city_level: 0,
        lost: false,
    }
}

/// 直線 n 州、各 100 萬人、靈潮 1.0、無王朝無宗門的承平世界。
fn calm_world(seed: u64, n: usize) -> World {
    let geo = Geography {
        w: 0,
        h: 0,
        wrap_x: true,
        sea_level: 0.0,
        height: vec![],
        terrain: vec![],
        river: vec![],
        province_of: vec![],
        aura_cell: vec![],
        nodes: vec![],
        ley: vec![],
        rivers: vec![],
        ranges: vec![],
        continents: vec![],
        continent_of: vec![],
    };
    let mut w = World::empty(WorldParams::new(seed), geo, (0..n as u16).map(|i| province(i, n)).collect());
    soul::ensure_heaven(&mut w);
    w.evolution.tide = 1.0;
    w
}

/// 跑 `years` 年；每年把人口補回 `POP`（人口自己的成長不在本模組）。
fn run(w: &mut World, years: u32) {
    for _ in 0..years {
        w.tick += 360;
        yearly(w);
        for p in &mut w.provinces {
            p.population = POP;
        }
    }
}

fn shares_of(w: &World, i: usize) -> [f64; DOORS] {
    let a = &w.adepts.provinces[i];
    a.n.map(|x| x / a.total())
}

#[test]
fn enter_rate_pins_three_points_and_is_monotonic() {
    assert!((enter_rate(1.0) - 0.003).abs() < 1e-12);
    assert!((enter_rate(EVO_TIDE_MAX) - 0.01).abs() < 1e-12);
    assert!((enter_rate(EVO_TIDE_MIN) - 0.0003).abs() < 1e-12);
    // 超出範圍的靈潮夾在端點。
    assert_eq!(enter_rate(9.0), enter_rate(EVO_TIDE_MAX));
    assert_eq!(enter_rate(-1.0), enter_rate(EVO_TIDE_MIN));
    let mut prev = 0.0;
    for i in 0..=90 {
        let r = enter_rate(0.5 + i as f32 * 0.01);
        assert!(r > prev, "靈潮 {} 時入道率沒有上升", 0.5 + i as f32 * 0.01);
        prev = r;
    }
}

#[test]
fn peaceful_world_settles_at_base_rate_with_balanced_doors() {
    let mut w = calm_world(7, 12);
    init(&mut w);
    run(&mut w, 1000);
    let s = summary(&w);
    assert!((0.002..=0.005).contains(&s.rate()), "入道率 {}", s.rate());
    for (d, sh) in s.shares().iter().enumerate() {
        assert!((0.12..=0.30).contains(sh), "{} 占 {}", DOOR_NAMES[d], sh);
    }
}

#[test]
fn tide_extremes_drive_rate_to_one_percent_and_three_hundredths() {
    for (tide, want) in [(EVO_TIDE_MAX, 0.01), (EVO_TIDE_MIN, 0.0003)] {
        let mut w = calm_world(7, 4);
        w.evolution.tide = tide;
        init(&mut w);
        run(&mut w, 400);
        let r = summary(&w).rate();
        assert!((r / want - 1.0).abs() < 0.05, "靈潮 {tide}：入道率 {r}，應趨近 {want}");
    }
}

#[test]
fn turbid_province_raises_demon_share_while_other_doors_stay_close() {
    let mut w = calm_world(7, 3);
    w.provinces[1].karma_qi = 0.8;
    init(&mut w);
    run(&mut w, 400);
    let (calm, turbid) = (shares_of(&w, 0), shares_of(&w, 1));
    assert!(turbid[DEMON] > calm[DEMON] + 0.1, "濁州魔門 {} 對照 {}", turbid[DEMON], calm[DEMON]);
    let rest: Vec<f64> = (0..DOORS).filter(|&d| d != DEMON).map(|d| turbid[d]).collect();
    let (lo, hi) = rest.iter().fold((1.0f64, 0.0f64), |(l, h), &x| (l.min(x), h.max(x)));
    assert!(hi - lo <= 0.05, "其餘四門差 {} 個百分點：{rest:?}", (hi - lo) * 100.0);
}

#[test]
fn same_seed_same_result() {
    let make = || {
        let mut w = calm_world(42, 6);
        w.provinces[2].karma_qi = 0.5;
        init(&mut w);
        run(&mut w, 200);
        w
    };
    let (a, b) = (make(), make());
    assert_eq!(a.adepts, b.adepts);
    assert_eq!(crate::hash::world_hash(&a), crate::hash::world_hash(&b));
}

#[test]
fn never_negative_and_never_above_population_under_shocks() {
    let mut w = calm_world(7, 5);
    init(&mut w);
    for y in 0..600u32 {
        w.tick += 360;
        // 人口忽高忽低，含驟減與歸零。
        for (i, p) in w.provinces.iter_mut().enumerate() {
            p.population = match (y as usize + i) % 7 {
                0 => 0.0,
                1 => POP * 0.05,
                2 => POP * 3.0,
                _ => POP,
            };
        }
        w.evolution.tide = [0.5, 1.4, 1.0, 0.7][(y / 50 % 4) as usize];
        yearly(&mut w);
        for (p, a) in w.provinces.iter().zip(&w.adepts.provinces) {
            assert!(a.n.iter().all(|&x| x >= 0.0 && x.is_finite()), "門人為負或非數：{:?}", a.n);
            assert!(a.total() <= p.population + 1e-6, "門人 {} 超過人口 {}", a.total(), p.population);
            assert!(a.nourish.iter().all(|&x| (0.0..=1.0).contains(&x)));
        }
    }
}

#[test]
fn population_drop_cuts_adepts_in_same_proportion() {
    let mut w = calm_world(7, 2);
    init(&mut w);
    run(&mut w, 600);
    let before = w.adepts.provinces[0].total();
    w.tick += 360;
    w.provinces[0].population = POP * 0.5;
    w.provinces[1].population = POP;
    yearly(&mut w);
    let after = w.adepts.provinces[0].total();
    // 穩態下目標也跟著減半，所以總數約為原來的一半（再扣魔道消耗的一點點）。
    assert!((0.48..=0.52).contains(&(after / before)), "比例 {}", after / before);
    // 沒減人口的州不受影響。
    assert!((w.adepts.provinces[1].total() / before - 1.0).abs() < 0.02);
}

#[test]
fn demon_adepts_eat_commoners_on_their_own_books_without_touching_population() {
    let mut w = calm_world(7, 1);
    init(&mut w);
    w.tick += 360;
    yearly(&mut w);
    assert_eq!(w.provinces[0].population, POP, "門人層不得改動州郡人口");
    let a = &w.adepts.provinces[0];
    assert!(a.n[DEMON] > 0.0);
    assert!((a.eaten - a.n[DEMON] * ADEPT_DEMON_EAT).abs() < 1e-9, "消耗帳 {}", a.eaten);
    // 吃得多的門養分較低：魔門比等量門人但不吃人的門低。
    let (civilians, per) = (POP - a.total() - a.eaten, ADEPT_NOURISH_BASE);
    assert!((a.nourish[DEMON] - (civilians / a.n[DEMON] / per).min(1.0)).abs() < 1e-12);
}

#[test]
fn nourish_is_commoners_per_adept_against_base_capped_at_one() {
    let mut w = calm_world(7, 1);
    init(&mut w);
    let a = &mut w.adepts.provinces[0];
    a.n = [100.0, 1000.0, 0.0, 0.0, 0.0];
    a.eaten = 0.0;
    let got = nourish_of(a, 1_000_000.0);
    let civilians = 1_000_000.0 - 1100.0;
    assert_eq!(got[TAO], 1.0);
    assert!((got[BUDDHA] - civilians / 1000.0 / ADEPT_NOURISH_BASE).abs() < 1e-12);
    assert!(got[BUDDHA] < 1.0);
    assert_eq!(got[RU], 1.0, "沒有門人的門養分記 1");
}

/// 建一名在世者並設定身分。
fn person(w: &mut World, role: Role, path: PathState) -> PersonId {
    let (_, pid) = soul::spawn_new_life(w, SoulOrigin::New, 0);
    let tick = w.tick;
    let p = &mut w.persons[pid as usize];
    p.born = tick as i32 - 30 * 360;
    p.role = role;
    p.path = path;
    pid
}

#[test]
fn transcender_boosts_its_door_for_one_yun_everywhere() {
    let mut w = calm_world(7, 3);
    w.tick = 360 * 100;
    let base = door_coefs(&w, 0);
    let pid = person(&mut w, Role::Monk, PathState::Monk { fruit: 3, practice: 0.0 });
    crate::people::die(&mut w, pid, DeathCause::Nirvana, Some(Liberation::Nirvana), &[]);
    refresh_world(&mut w);
    for province in 0..3 {
        let c = door_coefs(&w, province);
        assert!((c[BUDDHA] / base[BUDDHA] - ADEPT_TOP_BOOST).abs() < 1e-9, "佛門係數 {} 對 {}", c[BUDDHA], base[BUDDHA]);
        assert_eq!(c[TAO], base[TAO]);
    }
    // 滿一運（360 年）後失效。
    w.tick += 360 * (ADEPT_TOP_YEARS + 1);
    assert_eq!(door_coefs(&w, 0)[BUDDHA], base[BUDDHA]);
}

#[test]
fn highest_living_achiever_boosts_its_door() {
    let mut w = calm_world(7, 2);
    w.tick = 360 * 100;
    let base = door_coefs(&w, 0);
    person(&mut w, Role::Cultivator, PathState::Cultivator { realm: 2, exp: 0.0, last_attempt: None });
    person(&mut w, Role::Scholar, PathState::Scholar { virtue: 200.0, deed: 100.0, word: 100.0 });
    refresh_world(&mut w);
    assert_eq!(w.adepts.top_door, RU as u8);
    let c = door_coefs(&w, 0);
    assert!((c[RU] / base[RU] - ADEPT_TOP_BOOST).abs() < 1e-9);
    assert_eq!(c[TAO], base[TAO]);
}
