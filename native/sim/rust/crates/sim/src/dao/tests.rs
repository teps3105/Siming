//! U6 單元測試：公式對 7.1、突破與天劫、強制突破、入魔、斬妖、宗門生命週期、繼任分裂、爭脈之戰、確定性。

use std::collections::BTreeMap;

use super::*;
use crate::model::*;
use crate::params::*;
use crate::people;
use crate::soul;
use crate::types::{Geography, LeyNode, Province, Terrain, WorldParams};

fn province(id: u16, neighbors: Vec<u16>, aura: f32) -> Province {
    Province {
        id,
        name: format!("p{id}"),
        cells: vec![id as u32 * 2, id as u32 * 2 + 1],
        seat: id as u32 * 2,
        neighbors,
        sea_neighbors: vec![],
        terrain_mix: [0; Terrain::COUNT],
        habitability: 1.0,
        capacity: 1000.0,
        population: 1000.0,
        aura,
        karma_qi: 0.0,
        merit: 0.0,
        righteous: 0.0,
        devastation: 0.0,
        faith: [0.3, 0.1, 0.1],
        city_level: 0,
        lost: false,
    }
}

/// 直線 n 州；節點：州 i 的第一格（cell = 2i）為節點，強度見 `nodes`。
fn line_world(seed: u64, n: usize, nodes: &[(u16, f32)]) -> World {
    let provs = (0..n)
        .map(|i| {
            let mut nb = vec![];
            if i > 0 {
                nb.push(i as u16 - 1);
            }
            if i + 1 < n {
                nb.push(i as u16 + 1);
            }
            province(i as u16, nb, 0.5)
        })
        .collect();
    let geo = Geography {
        w: 0,
        h: 0,
        wrap_x: true,
        sea_level: 0.0,
        height: vec![],
        terrain: vec![],
        river: vec![],
        province_of: (0..n as u16 * 2).map(|c| c / 2).collect(),
        aura_cell: vec![],
        nodes: nodes
            .iter()
            .map(|&(p, s)| LeyNode { cell: p as u32 * 2, strength: s, origin: s, name: format!("n{p}") })
            .collect(),
        ley: vec![],
        rivers: vec![],
        ranges: vec![],
        continents: vec![],
        continent_of: vec![],
    };
    let mut w = World::empty(WorldParams::new(seed), geo, provs);
    soul::ensure_heaven(&mut w);
    w.tick = 1200;
    w
}

/// 建立一名成年修士（`Role::Cultivator`、散修）。
fn cult(w: &mut World, prov: u16, realm: u8, exp: f32) -> PersonId {
    let (_, pid) = soul::spawn_new_life(w, SoulOrigin::New, prov);
    let tick = w.tick;
    let p = &mut w.persons[pid as usize];
    p.born = tick as i32 - 30 * 12;
    p.role = Role::Cultivator;
    p.path = PathState::Cultivator { realm, exp, last_attempt: None };
    p.lifespan = 10_000;
    p.attrs = Attrs { root: 5.0, insight: 5.0, talent: 5.0, virtue: 5.0, ferocity: 3.0 };
    pid
}

fn set_soul(w: &mut World, pid: PersonId, merit: f32, sin: f32) {
    let s = w.persons[pid as usize].soul as usize;
    w.souls[s].merit = merit;
    w.souls[s].sin = sin;
}

fn realm(w: &World, pid: PersonId) -> u8 {
    match w.persons[pid as usize].path {
        PathState::Cultivator { realm, .. } => realm,
        _ => panic!("not cultivator"),
    }
}

fn exp(w: &World, pid: PersonId) -> f32 {
    match w.persons[pid as usize].path {
        PathState::Cultivator { exp, .. } => exp,
        _ => panic!(),
    }
}

fn count(w: &World, k: EventKind) -> usize {
    w.ledger.events.iter().filter(|e| e.kind == k).count()
}

fn new_sect(w: &mut World, province: u16, members: &[PersonId], leader: PersonId) -> PowerId {
    let id = w.powers.len() as PowerId;
    w.powers.push(Power {
        id,
        kind: PowerKind::Sect(Sect { evil: false, seat_node: None, radius: 1, ascended: 0 }),
        name: format!("s{id}"),
        seat_province: province,
        seat_cell: province as u32 * 2,
        leader: Some(leader),
        members: vec![],
        founded: 0,
        fallen: None,
        prestige: 0.0,
        prestige_mod: 0.0,
        relations: BTreeMap::new(),
    });
    for &m in members {
        join_sect(w, m, id);
    }
    id
}

// ── 公式對 7.1 ────────────────────────────────────────────

#[test]
fn formulas_match_params() {
    // 突破成功率 B × (0.8 + 悟性 × 0.04) × (賜福 1.5)，上限 0.95
    assert!((breakthrough_chance(2, 5.0, false) - REALM_BT[2]).abs() < 1e-6);
    assert!((breakthrough_chance(0, 0.0, false) - REALM_BT[0] * 0.8).abs() < 1e-6);
    assert!((breakthrough_chance(0, 10.0, true) - (REALM_BT[0] * 1.2 * 1.5).min(BT_MAX)).abs() < 1e-6);
    assert!((breakthrough_chance(3, 5.0, true) - REALM_BT[3] * 1.5).abs() < 1e-6);
    // 上限 0.95：所有境界、任何悟性與賜福都不超過。
    for r in 0..=ASCENSION_REALM {
        assert!(breakthrough_chance(r, 10.0, true) <= BT_MAX + 1e-6);
    }
    // 走火身死 0.1 + 0.03 × 當前境界（≥ 2）
    assert_eq!(fail_death_chance(1), 0.0);
    assert!((fail_death_chance(2) - 0.16).abs() < 1e-6);
    assert!((fail_death_chance(7) - 0.31).abs() < 1e-6);
    // 天劫
    assert!((tribulation_survival(2, 0.0) - 0.92).abs() < 1e-6);
    assert!((tribulation_survival(3, 100.0) - 0.92 * (-100.0f32 / 180.0).exp()).abs() < 1e-6);
    assert!((ascension_survival(300.0) - 0.80 * (-1.0f32).exp()).abs() < 1e-6);
    // 冷卻 36 個月
    assert!(cooldown_ready(None, 5));
    assert!(!cooldown_ready(Some(100), 135));
    assert!(cooldown_ready(Some(100), 136));
    // 心魔
    assert_eq!(heart_demon_chance(50.0, 0.0, false), 0.0);
    assert!((heart_demon_chance(280.0, 0.0, false) - 0.2).abs() < 1e-6);
    assert!((heart_demon_chance(280.0, 0.0, true) - 0.4).abs() < 1e-6);
}

#[test]
fn cultivation_coefficient_values() {
    let mut w = line_world(1, 3, &[(0, 1.0)]);
    let a = cult(&mut w, 0, 0, 0.0);
    w.persons[a as usize].attrs.root = 5.0;
    w.souls[w.persons[a as usize].soul as usize].wisdom = 100.0;
    // 散修：(0.6 + 1.2) × (0.5 + 0.75) × 1.0 × 1.5 × 1
    let expect = 1.8 * 1.25 * 1.0 * 1.5;
    assert!((cultivation_coefficient(&w, &w.persons[a as usize]) - expect).abs() < 1e-5);
    // 賜福 ×2
    w.persons[a as usize].blessed_until = Some(w.tick + 5);
    assert!((cultivation_coefficient(&w, &w.persons[a as usize]) - expect * 2.0).abs() < 1e-5);
    w.persons[a as usize].blessed_until = None;
    // 宗門弟子 1.2，山門在節點（強度 1.0）上再 × 1.3
    let s = new_sect(&mut w, 0, &[a], a);
    assert!((cultivation_coefficient(&w, &w.persons[a as usize]) - expect * 1.2).abs() < 1e-5);
    let PowerKind::Sect(sc) = &mut w.powers[s as usize].kind;
    sc.seat_node = Some(0);
    assert!((cultivation_coefficient(&w, &w.persons[a as usize]) - expect * 1.2 * 1.3).abs() < 1e-5);
    // 魔修：靈氣係數改業氣係數 0.5 + 2 × 業氣，無宗門加成
    w.persons[a as usize].role = Role::Demonic;
    w.provinces[0].karma_qi = 0.5;
    let demon = 1.8 * 1.5 * 1.0 * 1.5;
    assert!((cultivation_coefficient(&w, &w.persons[a as usize]) - demon).abs() < 1e-5);
}

#[test]
fn cultivation_caps_at_realm_exp() {
    let mut w = line_world(2, 2, &[]);
    let a = cult(&mut w, 0, 0, 7.9);
    w.persons[a as usize].last_attempt_block();
    cultivate::yearly_actions(&mut w);
    assert!(exp(&w, a) <= REALM_EXP[0] + 1e-4);
}

trait Block {
    fn last_attempt_block(&mut self);
}
impl Block for Person {
    /// 讓冷卻未過，避免測試中途突破。
    fn last_attempt_block(&mut self) {
        if let PathState::Cultivator { realm, exp, .. } = self.path {
            self.path = PathState::Cultivator { realm, exp, last_attempt: Some(1199) };
        }
    }
}

// ── 突破與天劫 ────────────────────────────────────────────

#[test]
fn breakthrough_rate_at_fixed_seed() {
    let mut w = line_world(7, 2, &[]);
    let ids: Vec<_> = (0..2000).map(|_| cult(&mut w, 0, 0, REALM_EXP[0])).collect();
    yearly(&mut w);
    let up = ids.iter().filter(|&&i| w.persons[i as usize].alive() && realm(&w, i) == 1).count() as f32;
    let rate = up / ids.len() as f32;
    // B = REALM_BT[0]，悟性 5 → ×1.0
    assert!((rate - REALM_BT[0]).abs() < 0.02, "rate {rate}");
    assert_eq!(count(&w, EventKind::Breakthrough), up as usize);
    // 成功者：修為歸零後再修煉一次；壽元不減
    for &i in &ids {
        if realm(&w, i) == 1 {
            assert!(w.persons[i as usize].lifespan >= 10_000);
        }
    }
}

#[test]
fn failed_breakthrough_keeps_85_percent_and_may_kill_from_golden_core() {
    let mut w = line_world(8, 2, &[]);
    // 築基（1）失敗不會身死
    let ids: Vec<_> = (0..400).map(|_| cult(&mut w, 0, 1, REALM_EXP[1])).collect();
    for &i in &ids {
        w.persons[i as usize].attrs.insight = 0.0;
    }
    yearly(&mut w);
    assert!(ids
        .iter()
        .all(|&i| !matches!(w.persons[i as usize].died, Some((_, DeathCause::BreakthroughFailed)))));
    let failed = ids.iter().filter(|&&i| w.persons[i as usize].alive() && realm(&w, i) == 1).count();
    assert!(failed > 100);
    for &i in &ids {
        if w.persons[i as usize].alive() && realm(&w, i) == 1 {
            assert!(matches!(w.persons[i as usize].path, PathState::Cultivator { last_attempt: Some(1200), .. }));
            assert!(exp(&w, i) >= REALM_EXP[1] * BT_FAIL_KEEP - 1e-3);
        }
    }
    // 金丹（2）失敗有 16% 走火身死
    let mut w = line_world(9, 2, &[]);
    let ids: Vec<_> = (0..2000).map(|_| cult(&mut w, 0, 2, REALM_EXP[2])).collect();
    for &i in &ids {
        w.persons[i as usize].attrs.insight = 0.0;
        set_soul(&mut w, i, 0.0, 0.0);
    }
    yearly(&mut w);
    let failed_death = ids
        .iter()
        .filter(|&&i| matches!(w.persons[i as usize].died, Some((_, DeathCause::BreakthroughFailed))))
        .count() as f32;
    let fails = ids.len() as f32 * (1.0 - 0.35 * 0.8) * 0.0 + ids.iter().filter(|&&i| realm(&w, i) == 2 || !w.persons[i as usize].alive()).count() as f32;
    // 失敗者中的身死比例 ≈ 0.16（含天劫死亡者不在此列）
    let rate = failed_death / (failed_death + ids.iter().filter(|&&i| w.persons[i as usize].alive() && realm(&w, i) == 2).count() as f32);
    assert!((rate - 0.16).abs() < 0.04, "rate {rate} fails {fails}");
}

#[test]
fn tribulation_outcomes_follow_sin() {
    // S = 0：存活率 0.92；高業障：大多死亡，兵解佔失敗的 40%
    for (sin, lo, hi) in [(0.0f32, 0.88f32, 0.96f32), (400.0, 0.0, 0.2)] {
        let mut w = line_world(11, 2, &[]);
        let ids: Vec<_> = (0..8000).map(|_| cult(&mut w, 0, 1, REALM_EXP[1])).collect();
        for &i in &ids {
            w.persons[i as usize].attrs.insight = 10.0;
            set_soul(&mut w, i, 0.0, sin);
            w.persons[i as usize].forced_breakthrough = None;
        }
        yearly(&mut w);
        let mut passed = 0;
        let mut shed = 0;
        let mut perished = 0;
        for e in &w.ledger.events {
            if let EventArgs::Tribulation { realm, outcome } = e.args {
                assert_eq!(realm, 2);
                match outcome {
                    TribulationOutcome::Passed => passed += 1,
                    TribulationOutcome::ShedBody => shed += 1,
                    TribulationOutcome::Perished => perished += 1,
                }
            }
        }
        let total = (passed + shed + perished) as f32;
        assert!(total > 500.0);
        let r = passed as f32 / total;
        let expect = 0.92 * (-sin / (60.0 + 80.0)).exp();
        assert!(r >= lo && r <= hi && (r - expect).abs() < 0.04, "sin {sin} rate {r} expect {expect}");
        let fails = (shed + perished) as f32;
        // 失敗樣本數隨 B 校準而變：容許 4 個標準差。
        let tol = 4.0 * (0.4f32 * 0.6 / fails).sqrt();
        assert!((shed as f32 / fails - 0.4).abs() < tol, "shed {shed} perished {perished}");
        // 死因與魂印
        assert_eq!(
            ids.iter().filter(|&&i| matches!(w.persons[i as usize].died, Some((_, DeathCause::ShedBody)))).count(),
            shed
        );
        assert_eq!(
            ids.iter().filter(|&&i| matches!(w.persons[i as usize].died, Some((_, DeathCause::Tribulation)))).count(),
            perished
        );
    }
}

#[test]
fn passing_tribulation_resets_exp_and_extends_lifespan() {
    let mut w = line_world(12, 2, &[]);
    let a = cult(&mut w, 0, 1, REALM_EXP[1]);
    w.persons[a as usize].lifespan = 100;
    w.persons[a as usize].forced_breakthrough = Some(w.tick);
    monthly_forced(&mut w);
    assert_eq!(realm(&w, a), 2);
    assert_eq!(exp(&w, a), 0.0);
    assert_eq!(w.persons[a as usize].lifespan, REALM_LIFESPAN[2] as u16);
    assert_eq!(w.persons[a as usize].forced_breakthrough, None);
    assert_eq!(count(&w, EventKind::Breakthrough), 1);
    assert_eq!(count(&w, EventKind::Tribulation), 1);
    // 事件的境界為新境界
    assert!(w.ledger.events.iter().any(|e| e.kind == EventKind::Breakthrough && e.args == EventArgs::Realm { realm: 2 }));
    assert!(w
        .ledger
        .events
        .iter()
        .any(|e| e.args == EventArgs::Tribulation { realm: 2, outcome: TribulationOutcome::Passed }));
}

/// 轉世大能：渡過天劫同時寫 `Tribulation{Passed}` 與 `Breakthrough`（同境界），天書只計一次。
#[test]
fn reincarnated_master_not_double_counted_on_passed_tribulation() {
    let mut w = line_world(14, 2, &[]);
    let a = cult(&mut w, 0, 1, REALM_EXP[1]);
    let sid = w.persons[a as usize].soul as usize;
    w.souls[sid].best_prev.realm = Some(2);
    w.persons[a as usize].forced_breakthrough = Some(w.tick);
    monthly_forced(&mut w);
    assert_eq!(realm(&w, a), 2);
    assert_eq!(count(&w, EventKind::Tribulation), 1);
    assert_eq!(count(&w, EventKind::Breakthrough), 1);
    assert_eq!(w.book.entries[&BookEntryKind::ReincarnatedMaster].count, 1);
    assert_eq!(count(&w, EventKind::BookEntry), 1);
}

#[test]
fn forced_breakthrough_ignores_sin_and_does_not_draw() {
    let mut w = line_world(13, 2, &[]);
    let a = cult(&mut w, 0, 1, REALM_EXP[1]);
    set_soul(&mut w, a, 0.0, 5000.0);
    w.persons[a as usize].forced_breakthrough = Some(w.tick);
    let before = w.rng.people.clone();
    monthly_forced(&mut w);
    assert_eq!(realm(&w, a), 2);
    assert_eq!(w.rng.people, before);
    // 排定在未來者不動
    let b = cult(&mut w, 0, 1, REALM_EXP[1]);
    w.persons[b as usize].forced_breakthrough = Some(w.tick + 3);
    monthly_forced(&mut w);
    assert_eq!(realm(&w, b), 1);
    assert_eq!(w.persons[b as usize].forced_breakthrough, Some(w.tick + 3));
}

#[test]
fn forced_ascension_liberates_soul_and_counts_for_sect() {
    let mut w = line_world(14, 2, &[]);
    let a = cult(&mut w, 0, ASCENSION_REALM, REALM_EXP[8]);
    let s = new_sect(&mut w, 0, &[a], a);
    w.persons[a as usize].forced_breakthrough = Some(w.tick);
    monthly_forced(&mut w);
    let p = &w.persons[a as usize];
    assert!(matches!(p.died, Some((_, DeathCause::Ascended))));
    assert!(matches!(
        w.souls[p.soul as usize].state,
        SoulState::Liberated { way: Liberation::Ascension, .. }
    ));
    assert_eq!(count(&w, EventKind::Ascended), 1);
    let PowerKind::Sect(sc) = &w.powers[s as usize].kind;
    assert_eq!(sc.ascended, 1);
    // 首領缺位已在同一步處理（成員 0 → 覆滅）
    assert!(w.powers[s as usize].fallen.is_some());
}

#[test]
fn omen_for_yuanying_with_full_exp_and_cooldown_ending() {
    let mut w = line_world(15, 2, &[]);
    let a = cult(&mut w, 0, 3, REALM_EXP[3]);
    w.persons[a as usize].path = PathState::Cultivator { realm: 3, exp: REALM_EXP[3], last_attempt: Some(w.tick - 30) };
    let b = cult(&mut w, 0, 2, REALM_EXP[2]);
    w.persons[b as usize].path = PathState::Cultivator { realm: 2, exp: REALM_EXP[2], last_attempt: Some(w.tick - 30) };
    let c = cult(&mut w, 0, 3, REALM_EXP[3]);
    w.persons[c as usize].path = PathState::Cultivator { realm: 3, exp: REALM_EXP[3], last_attempt: Some(w.tick - 5) };
    yearly(&mut w);
    let omens: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::TribulationOmen).collect();
    assert_eq!(omens.len(), 1);
    assert_eq!(omens[0].persons.as_slice(), &[a]);
}

// ── 心魔、魔修、斬妖除魔 ──────────────────────────────────

#[test]
fn forced_heart_demon_path() {
    let mut w = line_world(21, 3, &[]);
    let a = cult(&mut w, 0, 1, 0.0);
    let b = cult(&mut w, 0, 1, 0.0);
    let s = new_sect(&mut w, 0, &[a, b], a);
    w.persons[b as usize].master = Some(a);
    w.persons[a as usize].disciples.push(b);
    set_soul(&mut w, b, 0.0, 1200.0); // 機率 > 1
    let soul_b = w.persons[b as usize].soul as usize;
    yearly(&mut w);
    let p = &w.persons[b as usize];
    assert_eq!(p.role, Role::Demonic);
    assert_eq!(p.power, None);
    assert_eq!(p.master, None);
    assert!(!w.powers[s as usize].members.contains(&b));
    assert!(w.persons[a as usize].disciples.is_empty());
    assert!(w.souls[soul_b].marks.has(SoulMarks::FELL_TO_DEMON));
    assert_eq!(count(&w, EventKind::FellToDemon), 1);
    // 魔修不再被視為宗門成員，也不會再入魔一次
    yearly(&mut w);
    assert_eq!(count(&w, EventKind::FellToDemon), 1);
    // 魔修修為使用業氣係數
    assert!(exp(&w, b) > 0.0);
}

#[test]
fn heart_demon_rate_matches_formula_and_asura_doubles() {
    for asura in [false, true] {
        let mut w = line_world(22, 2, &[]);
        let ids: Vec<_> = (0..3000).map(|_| cult(&mut w, 0, 0, 0.0)).collect();
        for &i in &ids {
            set_soul(&mut w, i, 0.0, 180.0); // (180 − 80)/1000 = 0.1
            if asura {
                let s = w.persons[i as usize].soul as usize;
                w.souls[s].marks.set(SoulMarks::ASURA);
            }
        }
        yearly(&mut w);
        let n = ids.iter().filter(|&&i| w.persons[i as usize].role == Role::Demonic).count() as f32;
        let expect = if asura { 0.2 } else { 0.1 };
        assert!((n / 3000.0 - expect).abs() < 0.025, "asura {asura} rate {}", n / 3000.0);
    }
}

#[test]
fn demon_slaying_evil_target_gives_merit_without_thread() {
    let mut w = line_world(23, 3, &[]);
    let hunter = cult(&mut w, 0, 5, 0.0);
    let demon = cult(&mut w, 1, 0, 0.0);
    w.persons[demon as usize].role = Role::Demonic;
    set_soul(&mut w, hunter, 10.0, 0.0);
    set_soul(&mut w, demon, 0.0, 200.0); // S > G + 50
    let index = demon::PersonIndex::build(&w);
    for _ in 0..200 {
        if !w.persons[demon as usize].alive() {
            break;
        }
        demon::hunt(&mut w, &index, hunter);
    }
    assert!(!w.persons[demon as usize].alive(), "10% 每年，200 次內必中");
    assert!(matches!(w.persons[demon as usize].died, Some((_, DeathCause::Killed))));
    let hs = w.persons[hunter as usize].soul as usize;
    assert!((w.souls[hs].merit - (10.0 + MERIT_SLAY_EVIL)).abs() < 1e-4);
    assert_eq!(w.souls[hs].sin, 0.0);
    assert!(w.threads.is_empty());
    assert_eq!(count(&w, EventKind::Killed), 1);
}

#[test]
fn hunt_requires_realm_and_net_karma() {
    let mut w = line_world(24, 2, &[]);
    let low = cult(&mut w, 0, 1, 0.0);
    let sinner = cult(&mut w, 0, 5, 0.0);
    set_soul(&mut w, sinner, 0.0, 10.0);
    let demon = cult(&mut w, 0, 0, 0.0);
    w.persons[demon as usize].role = Role::Demonic;
    let index = demon::PersonIndex::build(&w);
    for _ in 0..300 {
        demon::hunt(&mut w, &index, low);
        demon::hunt(&mut w, &index, sinner);
    }
    assert!(w.persons[demon as usize].alive());
}

#[test]
fn demon_assassination_gains_exp_and_sin() {
    let mut w = line_world(26, 2, &[]);
    let d = cult(&mut w, 0, 7, 0.0);
    w.persons[d as usize].role = Role::Demonic;
    w.persons[d as usize].attrs.ferocity = 10.0; // 20%
    let victim = cult(&mut w, 1, 0, 0.0);
    let index = demon::PersonIndex::build(&w);
    for _ in 0..200 {
        if !w.persons[victim as usize].alive() {
            break;
        }
        demon::assassinate(&mut w, &index, d);
    }
    
    assert!(!w.persons[victim as usize].alive());
    // 修為 += 對方戰力（1.1）× 2
    assert!((exp(&w, d) - 1.1 * DEMON_KILL_EXP).abs() < 1e-3, "{}", exp(&w, d));
    let ds = w.persons[d as usize].soul as usize;
    assert!(w.souls[ds].sin > 0.0);
}

// ── 宗門 ──────────────────────────────────────────────────

#[test]
fn choose_seat_prefers_free_node_then_aura() {
    let mut w = line_world(31, 6, &[(1, 0.5), (2, 1.0), (5, 1.0)]);
    // 從州 0 出發，2 步內為 0,1,2：選強度最高的節點州 2
    let s = choose_seat(&w, 0, None).unwrap();
    assert_eq!((s.province, s.node, s.cell), (2, Some(1), 4));
    // 州 2 的節點被宗門佔用後（山門在州 2），州 0 的候選須離州 2 ≥ 2 步 → 只剩州 0；無空閒節點 → 靈氣最高州
    let a = cult(&mut w, 2, 2, 0.0);
    let id = new_sect(&mut w, 2, &[a], a);
    let PowerKind::Sect(sc) = &mut w.powers[id as usize].kind;
    sc.seat_node = Some(1);
    let s = choose_seat(&w, 0, None).unwrap();
    assert_eq!((s.province, s.node), (0, None));
    // 山門間距：州 3 緊鄰州 2，不可
    assert!(choose_seat(&w, 3, None).is_some_and(|s| s.province != 3 && s.province != 1));
    // 無候選州（起點及 2 步內全被擋）
    let b = cult(&mut w, 0, 2, 0.0);
    new_sect(&mut w, 0, &[b], b);
    assert!(choose_seat(&w, 1, None).is_none());
}

#[test]
fn yearly_found_creates_sects_with_spacing() {
    let mut w = line_world(32, 12, &[(3, 1.0), (9, 0.5)]);
    for i in 0..12 {
        for _ in 0..30 {
            cult(&mut w, i, 2, 0.0);
        }
    }
    sect::yearly_found(&mut w);
    let sects: Vec<_> = w.powers.iter().collect();
    assert!(!sects.is_empty());
    for (i, a) in sects.iter().enumerate() {
        for b in &sects[i + 1..] {
            assert!(a.seat_province.abs_diff(b.seat_province) >= 2, "{} {}", a.seat_province, b.seat_province);
        }
        assert_eq!(a.members.len(), 1);
        assert_eq!(w.persons[a.leader.unwrap() as usize].power, Some(a.id));
    }
    assert_eq!(count(&w, EventKind::PowerFounded), sects.len());
    // 名稱不重複
    let mut names: Vec<_> = sects.iter().map(|s| s.name.clone()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), sects.len());
}

#[test]
fn evil_founder_makes_evil_sect_with_hostile_relations() {
    let mut w = line_world(33, 8, &[]);
    let a = cult(&mut w, 0, 3, 0.0);
    let b = cult(&mut w, 6, 3, 0.0);
    set_soul(&mut w, b, 0.0, 100.0);
    let seat = choose_seat(&w, 0, None).unwrap();
    let sa = found_sect(&mut w, a, seat);
    let seat = choose_seat(&w, 6, None).unwrap();
    let sb = found_sect(&mut w, b, seat);
    assert!(matches!(&w.powers[sb as usize].kind, PowerKind::Sect(s) if s.evil));
    assert!(matches!(&w.powers[sa as usize].kind, PowerKind::Sect(s) if !s.evil));
    assert_eq!(w.powers[sa as usize].relations.get(&sb), Some(&-50));
    assert_eq!(w.powers[sb as usize].relations.get(&sa), Some(&-50));
}

#[test]
fn recruit_masters_fill_and_join() {
    let mut w = line_world(34, 4, &[]);
    let leader = cult(&mut w, 0, 4, 0.0);
    let elder = cult(&mut w, 0, 2, 0.0);
    let novice = cult(&mut w, 0, 0, 0.0);
    let s = new_sect(&mut w, 0, &[leader, elder, novice], leader);
    let rogue_near = cult(&mut w, 1, 0, 0.0);
    w.persons[novice as usize].master = None;
    let g0 = 0.0;
    sect::yearly_recruit(&mut w);
    // 新人拜長老（境界高於自己者），長老無師父 → 拜掌門
    assert!([Some(leader), Some(elder)].contains(&w.persons[novice as usize].master));
    assert_eq!(w.persons[elder as usize].master, Some(leader));
    assert!(w.persons[leader as usize].master.is_none());
    let m = w.persons[novice as usize].master.unwrap();
    assert!(w.persons[m as usize].disciples.contains(&novice));
    assert!(w.souls[w.persons[m as usize].soul as usize].merit >= g0 + MERIT_DISCIPLE);
    // 補足：規模 5 + 0 = 5 人
    assert_eq!(w.powers[s as usize].members.len(), people::sect_capacity(0.0));
    // 新補足的成員都有師父、12 歲
    for &m in &w.powers[s as usize].members {
        if m != leader {
            assert!(w.persons[m as usize].master.is_some());
        }
    }
    let newest = *w.powers[s as usize].members.last().unwrap();
    assert_eq!(w.persons[newest as usize].age(w.tick), 12);
    assert!(w.persons[rogue_near as usize].power.is_none() || w.persons[rogue_near as usize].power == Some(s));
}

#[test]
fn rogues_join_highest_prestige_unfilled_sect_in_range() {
    let mut w = line_world(35, 6, &[]);
    let l1 = cult(&mut w, 0, 3, 0.0);
    let l2 = cult(&mut w, 2, 3, 0.0);
    let s1 = new_sect(&mut w, 0, &[l1], l1);
    let s2 = new_sect(&mut w, 2, &[l2], l2);
    w.powers[s1 as usize].prestige = 10.0;
    w.powers[s2 as usize].prestige = 50.0;
    let rogues: Vec<_> = (0..600).map(|_| cult(&mut w, 1, 0, 0.0)).collect(); // 州 1 同在兩宗範圍（半徑 1）
    let demons: Vec<_> = (0..100)
        .map(|_| {
            let d = cult(&mut w, 1, 0, 0.0);
            w.persons[d as usize].role = Role::Demonic;
            d
        })
        .collect();
    sect::join_rogues(&mut w);
    let joined1 = rogues.iter().filter(|&&r| w.persons[r as usize].power == Some(s1)).count();
    let joined2 = rogues.iter().filter(|&&r| w.persons[r as usize].power == Some(s2)).count();
    // 聲望較高的 s2 先收滿，溢出者才進 s1；兩宗都不超過各自規模
    assert_eq!(w.powers[s2 as usize].members.len(), people::sect_capacity(50.0));
    assert_eq!(w.powers[s1 as usize].members.len(), people::sect_capacity(10.0));
    assert!(joined1 > 0 && joined2 > 0);
    assert!(demons.iter().all(|&d| w.persons[d as usize].power.is_none()));
}

#[test]
fn prestige_and_radius() {
    let mut w = line_world(36, 10, &[]);
    let l = cult(&mut w, 0, 2, 0.0);
    let m = cult(&mut w, 0, 1, 0.0);
    let s = new_sect(&mut w, 0, &[l, m], l);
    {
        let PowerKind::Sect(sc) = &mut w.powers[s as usize].kind;
        sc.ascended = 2;
    }
    w.powers[s as usize].prestige_mod = 100.0;
    sect::yearly_prestige(&mut w);
    let pw = &w.powers[s as usize];
    let expect = people::power_of(&w, l).sqrt() + people::power_of(&w, m).sqrt() + 100.0;
    assert!((pw.prestige - expect).abs() < 1e-4);
    assert!((pw.prestige_mod - 95.0).abs() < 1e-4);
    // eff ≈ 205 → 1 + floor(log2(1 + 205/30)) = 1 + 2 = 3
    let eff = effective_prestige(pw);
    let r = 1 + (1.0f32 + eff / 30.0).log2().floor() as u8;
    assert_eq!(sect_of_radius(&w, s), r.min(4));
    assert_eq!(sect_range(&w, s), vec![0, 1, 2, 3]);
}

fn sect_of_radius(w: &World, s: PowerId) -> u8 {
    let PowerKind::Sect(sc) = &w.powers[s as usize].kind;
    sc.radius
}

// ── 繼任、分裂、覆滅 ──────────────────────────────────────

#[test]
fn succession_picks_strongest_member() {
    let mut w = line_world(41, 4, &[]);
    let l = cult(&mut w, 0, 5, 0.0);
    let a = cult(&mut w, 0, 3, 0.0);
    let b = cult(&mut w, 0, 4, 0.0);
    let s = new_sect(&mut w, 0, &[l, a, b], l);
    people::die(&mut w, l, DeathCause::Age, None, &[]);
    assert_eq!(w.powers[s as usize].leader, Some(l)); // die 不改 leader
    resolve_leaders(&mut w);
    assert_eq!(w.powers[s as usize].leader, Some(b));
    assert_eq!(count(&w, EventKind::Succession), 1);
    let succ = w.ledger.events.iter().find(|e| e.kind == EventKind::Succession).unwrap();
    let died = w.ledger.events.iter().find(|e| e.kind == EventKind::Died).unwrap();
    assert!(succ.causes.contains(&died.id));
}

#[test]
fn leader_who_falls_to_demon_is_replaced() {
    let mut w = line_world(42, 4, &[]);
    let l = cult(&mut w, 0, 5, 0.0);
    let a = cult(&mut w, 0, 3, 0.0);
    let s = new_sect(&mut w, 0, &[l, a], l);
    fall_to_demon(&mut w, l);
    resolve_leaders(&mut w);
    assert_eq!(w.powers[s as usize].leader, Some(a));
}

#[test]
fn schism_happens_sometimes_with_rival_disciples() {
    let mut split = 0;
    for seed in 0..60u64 {
        let mut w = line_world(1000 + seed, 8, &[]);
        let l = cult(&mut w, 0, 5, 0.0);
        let mut mem = vec![l];
        let rival = cult(&mut w, 0, 5, 0.0);
        mem.push(rival);
        let mut followers = vec![];
        for _ in 0..3 {
            let d = cult(&mut w, 0, 1, 0.0);
            followers.push(d);
            mem.push(d);
        }
        for _ in 0..3 {
            mem.push(cult(&mut w, 0, 0, 0.0));
        }
        let s = new_sect(&mut w, 0, &mem, l);
        for &d in &followers {
            w.persons[d as usize].master = Some(rival);
            w.persons[rival as usize].disciples.push(d);
        }
        // 讓掌門略強，次強者 ≥ 70%
        w.persons[l as usize].path = PathState::Cultivator { realm: 5, exp: REALM_EXP[5] * 0.4, last_attempt: None };
        people::die(&mut w, l, DeathCause::Age, None, &[]);
        // 補一人使成員達 8（l 已移除，加入一名同境界者作新掌門）
        let strong = cult(&mut w, 0, 5, REALM_EXP[5] * 0.2);
        join_sect(&mut w, strong, s);
        resolve_leaders(&mut w);
        if count(&w, EventKind::SectSplit) == 1 {
            split += 1;
            assert_eq!(w.powers.len(), 2);
            let ns = &w.powers[1];
            assert_eq!(ns.leader, Some(rival));
            assert_eq!(ns.relations.get(&s), Some(&-80));
            assert_eq!(w.powers[s as usize].relations.get(&1), Some(&-80));
            for &d in &followers {
                assert_eq!(w.persons[d as usize].power, Some(1));
                assert_eq!(w.persons[d as usize].master, Some(rival));
            }
            assert_eq!(w.persons[rival as usize].power, Some(1));
            assert!(!w.powers[s as usize].members.contains(&rival));
        } else {
            assert_eq!(w.powers.len(), 1);
        }
    }
    assert!(split > 8 && split < 40, "split {split}/60 (期望 35%)");
}

#[test]
fn empty_sect_falls_and_members_become_rogues() {
    let mut w = line_world(43, 4, &[]);
    let l = cult(&mut w, 0, 3, 0.0);
    let s = new_sect(&mut w, 0, &[l], l);
    people::die(&mut w, l, DeathCause::Age, None, &[]);
    yearly_sects(&mut w);
    assert!(w.powers[s as usize].fallen.is_some());
    assert_eq!(count(&w, EventKind::PowerFallen), 1);
    // 覆滅後不再被處理
    yearly_sects(&mut w);
    assert_eq!(count(&w, EventKind::PowerFallen), 1);
    // 另一宗覆滅：成員轉散修
    let a = cult(&mut w, 2, 3, 0.0);
    let b = cult(&mut w, 2, 2, 0.0);
    let s2 = new_sect(&mut w, 2, &[a, b], a);
    sect::fall_sect(&mut w, s2, None);
    assert!(w.persons[a as usize].power.is_none() && w.persons[b as usize].power.is_none());
}

// ── 爭脈之戰 ──────────────────────────────────────────────

#[test]
fn sect_war_resolves_deaths_with_karma_and_may_destroy() {
    let mut falls = 0;
    for seed in 0..40u64 {
        let mut w = line_world(2000 + seed, 4, &[(1, 1.0)]);
        let mut a = vec![];
        let mut b = vec![];
        for i in 0..6 {
            a.push(cult(&mut w, 0, 3 + (i % 2) as u8, 0.0));
            b.push(cult(&mut w, 2, 1, 0.0));
        }
        let sa = new_sect(&mut w, 0, &a, a[0]);
        let sb = new_sect(&mut w, 2, &b, b[0]);
        {
            let PowerKind::Sect(x) = &mut w.powers[sb as usize].kind;
            x.evil = true;
        }
        for s in [sa, sb] {
            let PowerKind::Sect(x) = &mut w.powers[s as usize].kind;
            x.radius = 2;
        }
        let strong_before = a.len() + b.len();
        war::fight(&mut w, sa, sb, 0);
        assert_eq!(count(&w, EventKind::SectWar), 1);
        // 強者 a 勝；敗方 b 前 5 名各 30% 戰死
        let dead_b = b.iter().filter(|&&m| !w.persons[m as usize].alive()).count();
        assert!(dead_b <= 5);
        assert!(a.len() + b.len() == strong_before);
        for &m in &b {
            if let Some((_, c)) = w.persons[m as usize].died {
                assert_eq!(c, DeathCause::War);
            }
        }
        // 殺人者計業：Killed 事件數 = 戰死數（皆有殺者）
        let dead_all = a.iter().chain(&b).filter(|&&m| !w.persons[m as usize].alive()).count();
        assert_eq!(count(&w, EventKind::Killed), dead_all);
        if w.powers[sb as usize].fallen.is_some() {
            falls += 1;
        }
    }
    let _ = falls;
}

#[test]
fn yearly_wars_need_shared_node_and_hostility() {
    // 兩宗皆正道、關係 0 → 不開戰；其一為邪道 → 有機會
    let mut fought = [0u32, 0u32];
    for (case, evil) in [false, true].into_iter().enumerate() {
        for seed in 0..120u64 {
            let mut w = line_world(3000 + seed, 4, &[(1, 1.0)]);
            let a = cult(&mut w, 0, 3, 0.0);
            let b = cult(&mut w, 2, 3, 0.0);
            let sa = new_sect(&mut w, 0, &[a], a);
            let sb = new_sect(&mut w, 2, &[b], b);
            for s in [sa, sb] {
                let PowerKind::Sect(x) = &mut w.powers[s as usize].kind;
                x.radius = 1;
            }
            let PowerKind::Sect(x) = &mut w.powers[sb as usize].kind;
            x.evil = evil;
            war::yearly_wars(&mut w);
            fought[case] += count(&w, EventKind::SectWar) as u32;
        }
    }
    assert_eq!(fought[0], 0);
    assert!(fought[1] >= 1 && fought[1] < 30, "{:?}", fought); // 4% × 120 ≈ 5
}

// ── 確定性 ────────────────────────────────────────────────

fn scenario(seed: u64) -> u64 {
    let mut w = line_world(seed, 10, &[(2, 1.0), (6, 0.75)]);
    for i in 0..10 {
        for k in 0..25 {
            let c = cult(&mut w, i, (k % 4) as u8, 0.0);
            if k % 5 == 0 {
                set_soul(&mut w, c, 0.0, 300.0);
            }
        }
    }
    for y in 0..40 {
        w.tick += 12;
        yearly(&mut w);
        yearly_sects(&mut w);
        if y % 7 == 0 {
            monthly_forced(&mut w);
        }
    }
    crate::hash::world_hash(&w)
}

#[test]
fn deterministic_over_many_years() {
    assert_eq!(scenario(5), scenario(5));
    assert_ne!(scenario(5), scenario(6));
}

/// 煙霧測試（手動）：從空世界（U8 開天尚未接上）跑 3 運，印出境界分布與飛升數。
/// `cargo test -p siming-sim --release smoke_three_yun -- --ignored --nocapture`
#[test]
#[ignore]
fn smoke_three_yun() {
    use crate::Sim;
    let mut sim = Sim::new(&WorldParams::new(42)).unwrap();
    for yun in 0..3 {
        for _ in 0..4320 {
            sim.step();
        }
        let w = sim.world();
        let mut dist = [0u32; 9];
        let mut demons = 0;
        for p in w.persons.iter().filter(|p| p.alive()) {
            if let PathState::Cultivator { realm, .. } = p.path {
                dist[realm as usize] += 1;
                if p.role == Role::Demonic {
                    demons += 1;
                }
            }
        }
        let asc = w.persons.iter().filter(|p| matches!(p.died, Some((_, DeathCause::Ascended)))).count();
        let sects = w.powers.iter().filter(|p| p.fallen.is_none()).count();
        eprintln!(
            "yun {} realms {:?} demons {} sects {} fallen {} ascended(total) {} alive {}",
            yun + 1,
            dist,
            demons,
            sects,
            w.powers.len() - sects,
            asc,
            w.persons.iter().filter(|p| p.alive()).count()
        );
    }
}
