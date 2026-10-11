//! GDD 2.3 人口年度更新。

use siming_sim::params::*;
use siming_sim::population::yearly;
use siming_sim::{Province, Terrain};

fn prov(id: u16, hab: f64, pop: f64, neighbors: Vec<u16>) -> Province {
    Province {
        id,
        name: format!("p{id}"),
        cells: vec![id as u32],
        seat: id as u32,
        neighbors,
        sea_neighbors: vec![],
        terrain_mix: [0; Terrain::COUNT],
        habitability: hab,
        capacity: 0.0,
        population: pop,
        aura: 0.0,
        karma_qi: 0.0,
        merit: 0.0,
        righteous: 0.0,
        devastation: 0.0,
        faith: [0.0; 3],
        city_level: 0,
        lost: false,
    }
}

#[test]
fn gdd_2_3_growth_converges_to_capacity() {
    let mut v = vec![prov(0, 10.0, 1000.0, vec![])];
    for _ in 0..3000 {
        yearly(&mut v);
    }
    let k = 10.0 * POP_CAP_PER_HAB;
    assert_eq!(v[0].capacity, k);
    assert!((v[0].population - k).abs() / k < 0.01, "P={}", v[0].population);
    assert_eq!(v[0].karma_qi, 0.0);
}

#[test]
fn gdd_2_3_capacity_uses_devastation_and_karma() {
    let mut p = prov(0, 10.0, 1000.0, vec![]);
    p.devastation = 0.5;
    p.karma_qi = 0.5;
    let mut v = vec![p];
    yearly(&mut v);
    let k = 10.0 * POP_CAP_PER_HAB * (1.0 - CAP_DEVASTATION * 0.5) * (1.0 - CAP_KARMA * 0.5);
    assert!((v[0].capacity - k).abs() < 1e-6);
    assert!((v[0].devastation - 0.4).abs() < 1e-6);
}

#[test]
fn gdd_2_3_famine_triggers() {
    let k = 10.0 * POP_CAP_PER_HAB;
    let mut v = vec![prov(0, 10.0, 2.0 * k, vec![])];
    yearly(&mut v);
    assert!(v[0].karma_qi >= FAMINE_KARMA);
    assert!(v[0].population < 1.6 * k);
    assert!(v[0].population > 0.0);
}

#[test]
fn gdd_2_3_no_famine_below_threshold() {
    let k = 10.0 * POP_CAP_PER_HAB;
    let mut v = vec![prov(0, 10.0, 1.02 * k, vec![])];
    yearly(&mut v);
    assert_eq!(v[0].karma_qi, 0.0);
}

#[test]
fn gdd_2_3_migration_conserves_population_and_targets_lowest_ratio() {
    let k = 10.0 * POP_CAP_PER_HAB;
    // P=0.95K 無饑荒；成長後仍 > 0.9K，觸發遷徙。
    let mut v = vec![
        prov(0, 10.0, 0.95 * k, vec![1, 2]),
        prov(1, 10.0, 0.5 * k, vec![0]),
        prov(2, 10.0, 0.2 * k, vec![0]),
    ];
    // 先單獨算成長後的人口（無鄰接），以便對照。
    let mut solo = v.clone();
    for p in solo.iter_mut() {
        p.neighbors.clear();
    }
    yearly(&mut solo);
    let total_solo: f64 = solo.iter().map(|p| p.population).sum();
    yearly(&mut v);
    let total: f64 = v.iter().map(|p| p.population).sum();
    assert!((total - total_solo).abs() < 1e-6);
    let moved = solo[0].population * POP_MIGRATE_RATE;
    assert!((v[0].population - (solo[0].population - moved)).abs() < 1e-6);
    assert!((v[2].population - (solo[2].population + moved)).abs() < 1e-6);
    assert!((v[1].population - solo[1].population).abs() < 1e-6);
}

#[test]
fn gdd_2_3_migration_independent_of_order() {
    let k = 10.0 * POP_CAP_PER_HAB;
    let mk = || {
        vec![
            prov(0, 10.0, 0.95 * k, vec![1]),
            prov(1, 10.0, 0.95 * k, vec![0]),
        ]
    };
    let mut v = mk();
    yearly(&mut v);
    // 對稱情境：雙方互相遷出同量，結果對稱。
    assert!((v[0].population - v[1].population).abs() < 1e-6);
}

#[test]
fn gdd_2_3_city_levels() {
    let cases = [
        (CITY_LEVEL_TOWN_POP * 0.99, 0),
        (CITY_LEVEL_TOWN_POP, 1),
        (CITY_LEVEL_COUNTY_POP, 2),
        (CITY_LEVEL_STATE_POP, 3),
    ];
    for (pop, lv) in cases {
        // 以極大承載讓成長可忽略，人口維持於門檻附近。
        let mut p = prov(0, 1.0e6, pop, vec![]);
        p.population = pop;
        let mut v = vec![p];
        yearly(&mut v);
        // 成長微量 (>0)，故只在恰為門檻或以上時等級不降。
        assert_eq!(v[0].city_level, lv, "pop {pop}");
    }
}

#[test]
fn gdd_2_3_population_never_negative_and_zero_capacity() {
    let mut v = vec![prov(0, 0.0, 100.0, vec![]), prov(1, 10.0, 0.0, vec![0])];
    yearly(&mut v);
    assert!(v.iter().all(|p| p.population >= 0.0 && p.population.is_finite()));
}

#[test]
fn gdd_2_4_famine_karma_stays_within_the_field_range() {
    // 連年饑荒：業氣逐年累加，但不超過值域上限 1（GDD 2.4「所有州郡氣場值域 0..1」）。
    let k = 10.0 * POP_CAP_PER_HAB;
    let mut v = vec![prov(0, 10.0, 2.0 * k, vec![])];
    v[0].karma_qi = 0.99;
    for _ in 0..5 {
        v[0].population = 2.0 * v[0].capacity.max(k);
        yearly(&mut v);
    }
    assert!(v[0].karma_qi <= 1.0, "業氣 {} 超過值域", v[0].karma_qi);
    assert!(v[0].karma_qi > 0.99);
}

