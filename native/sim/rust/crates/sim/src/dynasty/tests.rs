//! GDD 3.7 王朝與天命的單元測試。每條規則一個 `gdd_3_7_` 測試。

use super::*;
use crate::soul;
use crate::types::{Geography, Province, Terrain, WorldParams};

const POP: f64 = 100_000.0;

fn province(id: u16, neighbors: Vec<u16>) -> Province {
    Province {
        id,
        name: format!("p{id}"),
        cells: vec![id as u32 * 2, id as u32 * 2 + 1],
        seat: id as u32 * 2,
        neighbors,
        sea_neighbors: vec![],
        terrain_mix: [0; Terrain::COUNT],
        habitability: 100.0,
        capacity: 300_000.0,
        population: POP,
        aura: 0.5,
        karma_qi: 0.0,
        merit: 0.0,
        righteous: 0.0,
        devastation: 0.0,
        faith: [0.3, 0.1, 0.1],
        city_level: 0,
        lost: false,
    }
}

fn geo() -> Geography {
    Geography {
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
    }
}

/// 直線 n 州，每州 10 萬人。
fn line(n: usize) -> World {
    let provs = (0..n)
        .map(|i| {
            let mut nb = vec![];
            if i > 0 {
                nb.push(i as u16 - 1);
            }
            if i + 1 < n {
                nb.push(i as u16 + 1);
            }
            province(i as u16, nb)
        })
        .collect();
    let mut w = World::empty(WorldParams::new(7), geo(), provs);
    soul::ensure_heaven(&mut w);
    w
}

/// 在 `province` 建立一名成年人物（新魂）。
fn person(w: &mut World, province: ProvinceId, role: Role, talent: f32, virtue: f32) -> PersonId {
    let (_, pid) = soul::spawn_new_life(w, SoulOrigin::New, province);
    let tick = w.tick as i32;
    let p = &mut w.persons[pid as usize];
    p.born = tick - 30 * 12;
    p.role = role;
    p.path = PathState::Mortal;
    p.attrs.talent = talent;
    p.attrs.virtue = virtue;
    p.lifespan = 90;
    pid
}

/// 一個佔有 `provs`、都城 `cap` 的王朝，君主德行 5、才略 5。
fn dynasty(w: &mut World, cap: ProvinceId, provs: &[ProvinceId]) -> usize {
    let ruler = person(w, cap, Role::Commoner, 5.0, 5.0);
    found(w, ruler, cap, provs) as usize
}

fn kill(w: &mut World, pid: PersonId) {
    crate::people::die(w, pid, DeathCause::Age, None, &[]);
}

fn kinds(w: &World, k: EventKind) -> usize {
    w.ledger.events.iter().filter(|e| e.kind == k).count()
}

#[test]
fn gdd_3_7_found_sets_initial_values() {
    let mut w = line(4);
    let di = dynasty(&mut w, 1, &[0, 1, 2]);
    let d = &w.dynasty.list[di];
    assert_eq!(d.mandate, MANDATE_INIT);
    assert_eq!(d.provinces, vec![0, 1, 2]);
    assert_eq!(d.name.chars().count(), 1);
    // 國庫初值為一年收入。
    let income = 3.0 * POP * TAX_RATE;
    assert!((d.treasury - income).abs() < 1e-6);
    assert_eq!(w.persons[d.ruler.unwrap() as usize].role, Role::Ruler);
    assert_eq!(kinds(&w, EventKind::DynastyFounded), 1);
    // 歸屬可由 Owner 場讀出：王朝 ID + 1，0 為無主。
    assert_eq!(field::get(&w, FieldId::Owner, 1), 1.0);
    assert_eq!(field::get(&w, FieldId::Owner, 3), 0.0);
}

#[test]
fn gdd_3_7_mandate_terms() {
    let mut w = line(4);
    let di = dynasty(&mut w, 1, &[0, 1, 2]);
    let r = w.dynasty.list[di].ruler.unwrap();
    w.persons[r as usize].attrs.virtue = 9.0;
    // 君主德行：0.5 × (9 − 5) = 2。
    assert!((mandate_delta(&w, di, 0, false) - 2.0).abs() < 1e-5);
    // 文臣平均德行：0.4 × (7 − 5) = 0.8。
    let o = person(&mut w, 1, Role::Official, 5.0, 7.0);
    w.dynasty.list[di].officials.push(o);
    assert!((mandate_delta(&w, di, 0, false) - 2.8).abs() < 1e-5);
    // 正氣 +10 × 0.1、業氣 −15 × 0.2。
    for p in 0..3 {
        w.provinces[p].righteous = 0.1;
        w.provinces[p].karma_qi = 0.2;
    }
    assert!((mandate_delta(&w, di, 0, false) - (2.8 + 1.0 - 3.0)).abs() < 1e-4);
    // 風水：4 × (0.75 − 0.5) = 1。
    w.provinces[1].aura = 0.75;
    let base = mandate_delta(&w, di, 0, false);
    // 饑荒州 −0.5 / 州；國庫不足 −5。
    assert!((mandate_delta(&w, di, 2, false) - (base - 1.0)).abs() < 1e-5);
    assert!((mandate_delta(&w, di, 0, true) - (base - 5.0)).abs() < 1e-5);
    // 得州 +1、失州 −3。
    w.dynasty.list[di].gained = 2;
    w.dynasty.list[di].lost = 1;
    assert!((mandate_delta(&w, di, 0, false) - (base + 2.0 - 3.0)).abs() < 1e-5);
}

#[test]
fn gdd_3_7_mandate_cycle_term_caps_at_three() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    w.persons[r as usize].attrs.virtue = 5.0;
    w.tick = 50 * 12;
    assert!((mandate_delta(&w, di, 0, false) + 0.5).abs() < 1e-5);
    w.tick = 900 * 12;
    assert!((mandate_delta(&w, di, 0, false) + 3.0).abs() < 1e-5);
}

#[test]
fn gdd_3_7_mandate_clamped_to_0_100() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    w.persons[r as usize].attrs.virtue = 10.0;
    w.provinces[0].righteous = 1.0;
    w.provinces[1].righteous = 1.0;
    w.dynasty.list[di].mandate = 99.0;
    w.tick = 12;
    yearly(&mut w, &[]);
    assert_eq!(w.dynasty.list[di].mandate, 100.0);
}

#[test]
fn gdd_3_7_golden_age_growth_bonus() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1]);
    w.dynasty.list[di].mandate = 80.0;
    let m = growth_mult(&w);
    assert_eq!(m, vec![POP_GROWTH_PROSPEROUS_MULT, POP_GROWTH_PROSPEROUS_MULT, 1.0]);
    w.dynasty.list[di].mandate = 79.9;
    assert_eq!(growth_mult(&w), vec![1.0; 3]);
    // 成長率確實乘上倍率。
    let mut a = vec![province(0, vec![])];
    let mut b = a.clone();
    a[0].population = 50_000.0;
    b[0].population = 50_000.0;
    crate::population::yearly_with(&mut a, &[1.0]);
    crate::population::yearly_with(&mut b, &[POP_GROWTH_PROSPEROUS_MULT]);
    let (ga, gb) = (a[0].population - 50_000.0, b[0].population - 50_000.0);
    assert!((gb / ga - 1.5).abs() < 1e-6);
}

#[test]
fn gdd_3_7_collapse_at_zero_mandate() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    let ruler = w.dynasty.list[di].ruler.unwrap();
    let off = person(&mut w, 0, Role::Official, 5.0, 5.0);
    let gen = person(&mut w, 1, Role::General, 6.0, 5.0);
    let heir = person(&mut w, 0, Role::Royal, 3.0, 5.0);
    {
        let d = &mut w.dynasty.list[di];
        d.officials.push(off);
        d.generals.push((gen, 1));
        d.heirs.push(heir);
        d.mandate = 0.1;
    }
    w.persons[ruler as usize].attrs.virtue = 0.0;
    w.tick = 12;
    yearly(&mut w, &[]);
    let d = &w.dynasty.list[di];
    assert!(d.fallen.is_some());
    assert!(d.provinces.is_empty());
    assert!((0..3).all(|p| w.dynasty.owner_of(p).is_none()));
    assert_eq!(w.persons[ruler as usize].role, Role::Commoner);
    assert_eq!(w.persons[heir as usize].role, Role::Commoner);
    assert_eq!(w.persons[off as usize].role, Role::Scholar);
    assert_eq!(w.persons[gen as usize].role, Role::GeneralCandidate);
    assert_eq!(kinds(&w, EventKind::DynastyCollapsed), 1);
}

#[test]
fn gdd_3_7_succession_eldest_heir() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    let ruler = w.dynasty.list[di].ruler.unwrap();
    let young = person(&mut w, 0, Role::Royal, 5.0, 5.0);
    let old = person(&mut w, 0, Role::Royal, 5.0, 5.0);
    w.persons[old as usize].born -= 120;
    w.dynasty.list[di].heirs = vec![young, old];
    w.dynasty.list[di].decree = Decree::BanTao;
    kill(&mut w, ruler);
    w.tick = 12;
    yearly(&mut w, &[]);
    let d = &w.dynasty.list[di];
    assert_eq!(d.ruler, Some(old));
    assert_eq!(d.heirs, vec![young]);
    assert_eq!(d.decree, Decree::None, "駕崩後詔令解除");
    assert_eq!(w.persons[old as usize].role, Role::Ruler);
    assert_eq!(kinds(&w, EventKind::Succession), 1);
    assert_eq!(kinds(&w, EventKind::Usurpation), 0);
}

#[test]
fn gdd_3_7_usurpation_without_heir() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    let ruler = w.dynasty.list[di].ruler.unwrap();
    let strong = person(&mut w, 1, Role::General, 9.0, 5.0);
    let weak = person(&mut w, 0, Role::Official, 2.0, 5.0);
    {
        let d = &mut w.dynasty.list[di];
        d.generals.push((strong, 1));
        d.officials.push(weak);
        d.mandate = 70.0;
    }
    kill(&mut w, ruler);
    court::succession(&mut w, di);
    let d = &w.dynasty.list[di];
    assert_eq!(d.ruler, Some(strong));
    assert!((d.mandate - 50.0).abs() < 1e-5, "篡位天命 −20");
    assert!(d.generals.is_empty() && d.officials == vec![weak]);
    assert_eq!(kinds(&w, EventKind::Usurpation), 1);
    assert_eq!(kinds(&w, EventKind::CivilWar), 0, "才略差 7 不內戰");
}

#[test]
fn gdd_3_7_no_candidates_collapses() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let ruler = w.dynasty.list[di].ruler.unwrap();
    kill(&mut w, ruler);
    court::succession(&mut w, di);
    assert!(w.dynasty.list[di].fallen.is_some());
}

#[test]
fn gdd_3_7_civil_war_splits_by_distance() {
    // 0-1-2-3-4-5：甲駐 0、乙駐 5。
    let mut w = line(6);
    let di = dynasty(&mut w, 0, &[0, 1, 2, 3, 4, 5]);
    let b = person(&mut w, 5, Role::General, 8.0, 5.0);
    let c = person(&mut w, 4, Role::General, 6.0, 5.0);
    w.dynasty.list[di].generals = vec![(b, 5), (c, 4)];
    w.dynasty.list[di].treasury = 600.0;
    assert!(civil_war(&mut w, di, 0, b, 5));
    let old = &w.dynasty.list[di];
    let new = &w.dynasty.list[1];
    // 距離相同的第 2、3 州之間（3 步對 3 步）：2 距甲 2、距乙 3 → 甲；3 距甲 3、距乙 2 → 乙。
    assert_eq!(old.provinces, vec![0, 1, 2]);
    assert_eq!(new.provinces, vec![3, 4, 5]);
    assert_eq!(new.ruler, Some(b));
    assert_eq!(new.capital, 5);
    assert_eq!(new.generals, vec![(c, 4)], "駐在乙一邊的武將隨乙");
    assert_eq!(old.relations.get(&1), Some(&CIVIL_WAR_RELATION));
    assert_eq!(new.relations.get(&0), Some(&CIVIL_WAR_RELATION));
    assert!(old.wars.contains(&1) && new.wars.contains(&0));
    assert_eq!(new.mandate, MANDATE_INIT);
    assert!((old.treasury - 300.0).abs() < 1e-6);
    assert_eq!(w.dynasty.owner_of(4), Some(1));
    assert_eq!(kinds(&w, EventKind::CivilWar), 1);
}

#[test]
fn gdd_3_7_civil_war_same_station_uses_farthest() {
    let mut w = line(5);
    let di = dynasty(&mut w, 0, &[0, 1, 2, 3, 4]);
    let b = person(&mut w, 0, Role::Official, 8.0, 5.0);
    assert!(civil_war(&mut w, di, 0, b, 0));
    assert_eq!(w.dynasty.list[1].capital, 4, "同駐地時乙取離甲最遠的州");
    // 單州疆域無法內戰。
    let mut w = line(1);
    let di = dynasty(&mut w, 0, &[0]);
    let b = person(&mut w, 0, Role::Official, 8.0, 5.0);
    assert!(!civil_war(&mut w, di, 0, b, 0));
}

#[test]
fn gdd_3_7_secede_takes_closer_provinces() {
    // 都城 0，武將駐 4；0-1-2-3-4-5，州 5 在 4 之外。
    let mut w = line(6);
    let di = dynasty(&mut w, 0, &[0, 1, 2, 3, 4, 5]);
    let g = person(&mut w, 4, Role::General, 7.0, 5.0);
    w.dynasty.list[di].generals = vec![(g, 4)];
    let nid = secede(&mut w, di, g).unwrap() as usize;
    // 距駐地 4 比距都城 0 近：3（1 對 3）、4、5；2 距兩邊皆 2，不算。
    assert_eq!(w.dynasty.list[nid].provinces, vec![3, 4, 5]);
    assert_eq!(w.dynasty.list[di].provinces, vec![0, 1, 2]);
    assert_eq!(w.dynasty.list[di].lost, 3);
    assert_eq!(w.dynasty.list[nid].ruler, Some(g));
    assert_eq!(w.dynasty.list[nid].capital, 4);
    assert_eq!(kinds(&w, EventKind::Secession), 1);
    // 駐在都城者無地可據。
    let g2 = person(&mut w, 0, Role::General, 7.0, 5.0);
    w.dynasty.list[di].generals = vec![(g2, 0)];
    assert!(secede(&mut w, di, g2).is_none());
}

#[test]
fn gdd_3_7_secession_roll_below_20() {
    // 天命 1：每位駐守非都城州的武將每年 19/200；多位武將、多年必有人割據。
    let mut w = line(8);
    let di = dynasty(&mut w, 0, &(0..8).collect::<Vec<_>>());
    for p in 1..8u16 {
        let g = person(&mut w, p, Role::General, 6.0, 5.0);
        w.dynasty.list[di].generals.push((g, p));
    }
    w.dynasty.list[di].mandate = 1.0;
    let ruler = w.dynasty.list[di].ruler.unwrap();
    w.persons[ruler as usize].attrs.virtue = 10.0;
    for year in 1..=20u32 {
        w.tick = year * 12;
        w.dynasty.list[di].mandate = 1.0;
        realm::unrest(&mut w, di, false);
    }
    assert!(kinds(&w, EventKind::Secession) >= 1);
}

#[test]
fn gdd_3_7_rebellion_below_40_emits_event() {
    let mut w = line(10);
    let di = dynasty(&mut w, 0, &(0..10).collect::<Vec<_>>());
    w.dynasty.list[di].mandate = 10.0;
    for _ in 0..10 {
        realm::unrest(&mut w, di, false);
    }
    let n = kinds(&w, EventKind::Rebellion);
    assert!(n >= 1, "每州每年 7.5%，10 州 10 年應至少一次");
    let e = w.ledger.events.iter().find(|e| e.kind == EventKind::Rebellion).unwrap();
    assert!(!e.persons.is_empty() && e.province.is_some());
    // 天命 ≥ 40 不起義。
    let before = kinds(&w, EventKind::Rebellion);
    w.dynasty.list[di].mandate = 40.0;
    for _ in 0..50 {
        realm::unrest(&mut w, di, false);
    }
    assert_eq!(kinds(&w, EventKind::Rebellion), before);
}

#[test]
fn gdd_3_7_staffing_counts_and_posts() {
    let mut w = line(10);
    let di = dynasty(&mut w, 0, &(0..8).collect::<Vec<_>>());
    // 8 州：文臣 2 + 8/5 = 3，武將 1 + 8/4 = 3。
    for p in 0..5 {
        person(&mut w, p, Role::Scholar, 5.0, 5.0 + p as f32 * 0.1);
        person(&mut w, p, Role::GeneralCandidate, 5.0 + p as f32, 5.0);
    }
    person(&mut w, 9, Role::GeneralCandidate, 9.9, 5.0); // 疆域外
    court::staff(&mut w, di);
    let d = &w.dynasty.list[di];
    assert_eq!(d.officials.len(), 3);
    assert_eq!(d.generals.len(), 3);
    // 文臣依德行 + 才略：德行 5.4、5.3、5.2 的三位。
    let virtues: Vec<f32> = d.officials.iter().map(|&o| w.persons[o as usize].attrs.virtue).collect();
    assert!(virtues.iter().all(|&v| v >= 5.2 - 1e-5), "{virtues:?}");
    // 武將依才略，且不取疆域外者；與無主州接壤的州 7（鄰 8）是駐地，其餘回都城。
    let gens: Vec<(f32, ProvinceId)> = d.generals.iter().map(|g| (w.persons[g.0 as usize].attrs.talent, g.1)).collect();
    assert!(gens.iter().all(|g| g.0 < 9.5));
    assert_eq!(gens[0].1, 7, "才略最高者駐與無主州接壤的州");
    assert_eq!(gens[1].1, 0);
    // 文臣駐都城。
    assert!(d.officials.iter().all(|&o| w.persons[o as usize].province == 0));
    // 員額上限。
    let mut w = line(60);
    let di = dynasty(&mut w, 0, &(0..60).collect::<Vec<_>>());
    for _ in 0..20 {
        person(&mut w, 0, Role::Scholar, 5.0, 5.0);
        person(&mut w, 0, Role::GeneralCandidate, 5.0, 5.0);
    }
    court::staff(&mut w, di);
    assert_eq!(w.dynasty.list[di].officials.len(), 10);
    assert_eq!(w.dynasty.list[di].generals.len(), 12);
    // 無人可補維持缺額。
    let mut w = line(10);
    let di = dynasty(&mut w, 0, &(0..10).collect::<Vec<_>>());
    court::staff(&mut w, di);
    assert!(w.dynasty.list[di].officials.is_empty() && w.dynasty.list[di].generals.is_empty());
}

#[test]
fn gdd_3_7_generals_post_enemy_border_first() {
    // 0-1-2 | 3-4：王朝甲 {0,1,2}、王朝乙 {3,4}；州 2 與乙接壤。
    let mut w = line(5);
    let a = dynasty(&mut w, 0, &[0, 1, 2]);
    let _b = dynasty(&mut w, 4, &[3, 4]);
    for p in 0..3 {
        person(&mut w, p, Role::GeneralCandidate, 6.0, 5.0);
    }
    court::staff(&mut w, a);
    let posts: Vec<ProvinceId> = w.dynasty.list[a].generals.iter().map(|g| g.1).collect();
    assert_eq!(posts[0], 2);
    assert!(posts[1..].iter().all(|&p| p == 0));
}

#[test]
fn gdd_3_7_policy_by_ruler_affinity() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    let sid = w.persons[r as usize].soul as usize;
    for (aff, want) in [
        ([0.5, 0.2, 0.2, 0.1], Policy::Tao),
        ([0.1, 0.5, 0.2, 0.2], Policy::Buddha),
        ([0.1, 0.2, 0.5, 0.2], Policy::Ru),
        ([0.1, 0.2, 0.2, 0.5], Policy::None),
    ] {
        w.souls[sid].affinity = aff;
        assert_eq!(policy_of(&w, r), want);
    }
}

#[test]
fn gdd_3_7_policy_set_on_accession_and_fixed_while_reigning() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let ruler = w.dynasty.list[di].ruler.unwrap();
    let heir = person(&mut w, 0, Role::Royal, 5.0, 5.0);
    let hs = w.persons[heir as usize].soul as usize;
    w.souls[hs].affinity = [0.1, 0.1, 0.7, 0.1];
    w.dynasty.list[di].heirs.push(heir);
    w.dynasty.list[di].policy = Policy::Tao;
    // 在位期間改變道緣不改國策。
    let rs = w.persons[ruler as usize].soul as usize;
    w.souls[rs].affinity = [0.1, 0.1, 0.1, 0.7];
    w.tick = 12;
    yearly(&mut w, &[]);
    assert_eq!(w.dynasty.list[di].policy, Policy::Tao);
    kill(&mut w, ruler);
    w.tick = 24;
    yearly(&mut w, &[]);
    assert_eq!(w.dynasty.list[di].policy, Policy::Ru);
}

#[test]
fn gdd_3_7_revere_ru_and_tao_effects() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    w.dynasty.list[di].policy = Policy::Ru;
    court::policy_effects(&mut w, di);
    assert!((w.provinces[0].righteous - REVERE_RU_RIGHT).abs() < 1e-6);
    assert!((w.provinces[1].righteous - REVERE_RU_RIGHT).abs() < 1e-6);
    w.dynasty.list[di].policy = Policy::Tao;
    w.powers.push(sect_power(0));
    w.powers.push(sect_power(1));
    court::policy_effects(&mut w, di);
    assert!((w.powers[0].prestige_mod - REVERE_TAO_SECT_MOD).abs() < 1e-6);
    assert!((w.powers[1].prestige_mod - REVERE_TAO_SECT_MOD).abs() < 1e-6);
}

fn sect_power(seat: ProvinceId) -> Power {
    Power {
        id: seat as PowerId,
        kind: PowerKind::Sect(Sect { evil: false, seat_node: None, radius: 1, ascended: 0 }),
        name: "宗".into(),
        seat_province: seat,
        seat_cell: 0,
        leader: None,
        members: vec![],
        founded: 0,
        fallen: None,
        prestige: 150.0,
        prestige_mod: 0.0,
        relations: Default::default(),
    }
}

#[test]
fn gdd_3_7_ban_tao_decree() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    let rs = w.persons[r as usize].soul as usize;
    w.powers.push(sect_power(1));
    // 道緣 ≥ 0.15 不禁道。
    w.souls[rs].affinity = [0.15, 0.4, 0.4, 0.05];
    for _ in 0..200 {
        court::decree(&mut w, di);
    }
    assert_eq!(w.dynasty.list[di].decree, Decree::None);
    // 道緣 < 0.15 且有聲望 ≥ 100 的宗門：每年 5%，200 次必成。
    w.souls[rs].affinity = [0.1, 0.4, 0.4, 0.1];
    for _ in 0..200 {
        court::decree(&mut w, di);
    }
    let d = &w.dynasty.list[di];
    assert_eq!(d.decree, Decree::BanTao);
    assert_eq!(d.sect_relations.get(&1), Some(&BAN_TAO_RELATION));
    assert_eq!(kinds(&w, EventKind::BanTao), 1, "頒布後鎖定，不重複");
    // 禁道期間宗門聲望修正每年 −5（國策取「無」以免混入尊道的 +2）。
    w.dynasty.list[di].policy = Policy::None;
    court::policy_effects(&mut w, di);
    assert!((w.powers[0].prestige_mod - BAN_TAO_SECT_MOD).abs() < 1e-6);
    // 聲望不足 100 不禁道。
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    let rs = w.persons[r as usize].soul as usize;
    w.souls[rs].affinity = [0.1, 0.4, 0.4, 0.1];
    let mut s = sect_power(1);
    s.prestige = 99.0;
    w.powers.push(s);
    for _ in 0..200 {
        court::decree(&mut w, di);
    }
    assert_eq!(w.dynasty.list[di].decree, Decree::None);
}

#[test]
fn gdd_3_7_finance_formulas() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    let o = person(&mut w, 0, Role::Official, 5.0, 5.0);
    let g = person(&mut w, 0, Role::General, 5.0, 5.0);
    w.dynasty.list[di].officials.push(o);
    w.dynasty.list[di].generals.push((g, 0));
    w.provinces[2].devastation = 0.5;
    let f = finance(&w, di);
    let income = POP * 0.001 * (1.0 + 1.0 + 0.5);
    assert!((f.income - income).abs() < 1e-6);
    let standing = 3.0 * POP * 0.005;
    assert!((f.standing - standing).abs() < 1e-6);
    assert!((f.military - standing / 1000.0 * 20.0).abs() < 1e-6);
    assert!((f.officers - 10.0).abs() < 1e-9);
    assert!((f.court - income * 0.1).abs() < 1e-6);
    assert!((f.expenses - (f.military + f.officers + f.court)).abs() < 1e-9);
}

#[test]
fn gdd_3_7_treasury_settles_and_caps_at_five_years_income() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let f = finance(&w, di);
    w.dynasty.list[di].treasury = 0.0;
    let (_, broke) = state_finance(&mut w, di, &[]);
    assert!((w.dynasty.list[di].treasury - (f.income - f.expenses)).abs() < 1e-6);
    assert!(!broke);
    // 超出 5 年收入的部分耗散。
    w.dynasty.list[di].treasury = 100.0 * f.income;
    state_finance(&mut w, di, &[]);
    assert!((w.dynasty.list[di].treasury - 5.0 * f.income).abs() < 1e-6);
    assert!((w.dynasty.list[di].standing_army - f.standing).abs() < 1e-9);
}

#[test]
fn gdd_3_7_broke_penalties() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    let r = w.dynasty.list[di].ruler.unwrap();
    w.persons[r as usize].attrs.virtue = 5.0;
    // 官俸遠超收入 → 國庫不足。
    for _ in 0..200 {
        let o = person(&mut w, 0, Role::Official, 5.0, 5.0);
        w.dynasty.list[di].officials.push(o);
    }
    w.dynasty.list[di].treasury = 0.0;
    let (_, broke) = state_finance(&mut w, di, &[]);
    assert!(broke);
    assert!((mandate_delta(&w, di, 0, true) - mandate_delta(&w, di, 0, false) - MANDATE_BROKE).abs() < 1e-5);
    // 才略最高的武將 50% 叛離：多次嘗試必有發生，且叛離者是才略最高者。
    let g1 = person(&mut w, 2, Role::General, 9.0, 5.0);
    let g2 = person(&mut w, 1, Role::General, 6.0, 5.0);
    w.dynasty.list[di].generals = vec![(g1, 2), (g2, 1)];
    w.dynasty.list[di].mandate = 60.0;
    let mut hit = false;
    for _ in 0..30 {
        realm::unrest(&mut w, di, true);
        if kinds(&w, EventKind::Secession) > 0 {
            hit = true;
            break;
        }
    }
    assert!(hit);
    let nd = w.dynasty.list.last().unwrap();
    assert_eq!(nd.ruler, Some(g1));
}

#[test]
fn gdd_3_7_relief_spends_and_saves_half() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let o = person(&mut w, 0, Role::Official, 5.0, 8.0);
    w.dynasty.list[di].officials.push(o);
    let sid = w.persons[o as usize].soul as usize;
    let g0 = w.souls[sid].merit;
    let f = finance(&w, di);
    w.dynasty.list[di].treasury = f.income * 3.0;
    let before_pop = w.provinces[1].population;
    let before_t = w.dynasty.list[di].treasury;
    state_finance(&mut w, di, &[(1, 4_000)]);
    // 救回一半：+2,000 人；支出 4,000 × 0.001 = 4。
    assert!((w.provinces[1].population - before_pop - 2_000.0).abs() < 1e-6);
    assert!((w.dynasty.list[di].treasury - (before_t + f.income - f.expenses - 4.0)).abs() < 1e-6);
    // 主事文臣 G += 救活人數 / 2,000 = 1。
    assert!((w.souls[sid].merit - g0 - 1.0).abs() < 1e-5);
    assert_eq!(kinds(&w, EventKind::Relief), 1);
    // 天命 < 40 或國庫不足一年支出不賑災。
    w.dynasty.list[di].mandate = 39.0;
    let p = w.provinces[1].population;
    state_finance(&mut w, di, &[(1, 4_000)]);
    assert_eq!(w.provinces[1].population, p);
    w.dynasty.list[di].mandate = 60.0;
    w.dynasty.list[di].treasury = f.expenses * 0.5;
    state_finance(&mut w, di, &[(1, 4_000)]);
    assert_eq!(w.provinces[1].population, p);
}

#[test]
fn gdd_3_7_annex_needs_mandate_and_treasury() {
    let mut w = line(4);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let f = finance(&w, di);
    let probe = |w: &mut World, di: usize, mandate: f32, treasury: f64| -> bool {
        w.dynasty.list[di].mandate = mandate;
        w.dynasty.list[di].treasury = treasury;
        for _ in 0..200 {
            realm::expansion(w, di, &f);
            if w.dynasty.list[di].provinces.len() > 2 {
                return true;
            }
        }
        false
    };
    assert!(!probe(&mut w, di, 49.0, f.expenses * 10.0), "天命不足");
    assert!(!probe(&mut w, di, 80.0, f.expenses * 1.9), "國庫不足兩年支出");
    assert!(probe(&mut w, di, 80.0, f.expenses * 2.0));
    // 吞併的是相鄰無主州（2），並記入本年得州。
    assert_eq!(w.dynasty.list[di].provinces, vec![0, 1, 2]);
    assert_eq!(w.dynasty.list[di].gained, 1);
    assert_eq!(w.dynasty.owner_of(2), Some(di as DynastyId));
    assert_eq!(kinds(&w, EventKind::Annexed), 1);
}

#[test]
fn gdd_3_7_declare_war_conditions() {
    // 兩個相鄰王朝：甲強（人口多）、關係 < 0。
    let mut w = line(4);
    let a = dynasty(&mut w, 0, &[0, 1]);
    let b = dynasty(&mut w, 3, &[2, 3]);
    w.provinces[0].population = 400_000.0;
    w.provinces[1].population = 400_000.0;
    let f = finance(&w, a);
    for d in [a, b] {
        w.dynasty.list[d].mandate = 80.0;
        w.dynasty.list[d].treasury = 1e12;
    }
    let try_war = |w: &mut World| -> bool {
        for _ in 0..300 {
            realm::expansion(w, a, &f);
            if !w.dynasty.list[a].wars.is_empty() {
                return true;
            }
            w.dynasty.list[a].provinces.retain(|&p| p <= 1);
        }
        false
    };
    // 關係 ≥ 0：不宣戰。
    assert!(!try_war(&mut w));
    w.dynasty.list[a].relations.insert(b as DynastyId, -10);
    assert!(try_war(&mut w));
    assert_eq!(w.dynasty.list[a].wars, vec![b as DynastyId]);
    assert_eq!(w.dynasty.list[b].wars, vec![a as DynastyId]);
    assert_eq!(kinds(&w, EventKind::WarDeclared), 1, "已交戰不重複宣戰");
}

#[test]
fn gdd_3_7_relations_drift_border_and_same_policy() {
    let mut w = line(4);
    let a = dynasty(&mut w, 0, &[0, 1]);
    let b = dynasty(&mut w, 3, &[2, 3]);
    w.dynasty.list[a].policy = Policy::Tao;
    w.dynasty.list[b].policy = Policy::Ru;
    realm::update_relations(&mut w, a);
    assert_eq!(w.dynasty.list[a].relations.get(&(b as DynastyId)), Some(&-5));
    // 向 0 回歸 10%：−5 → −5 − (−5/10 = 0) 再 −5；−50 → −45 → −50。
    w.dynasty.list[a].relations.insert(b as DynastyId, -50);
    realm::update_relations(&mut w, a);
    assert_eq!(w.dynasty.list[a].relations.get(&(b as DynastyId)), Some(&-50));
    // 同國策 +5 抵銷接壤 −5。
    w.dynasty.list[b].policy = Policy::Tao;
    w.dynasty.list[a].relations.clear();
    realm::update_relations(&mut w, a);
    assert_eq!(w.dynasty.list[a].relations.get(&(b as DynastyId)), None);
    // 不接壤的只回歸 0。
    w.dynasty.list[a].relations.insert(7, 40);
    realm::update_relations(&mut w, a);
    assert_eq!(w.dynasty.list[a].relations.get(&7), Some(&36));
}

#[test]
fn gdd_3_7_fengshan_conditions_and_effects() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    let sid = w.persons[r as usize].soul as usize;
    w.persons[r as usize].attrs.virtue = 8.0;
    w.dynasty.list[di].mandate = 91.0;
    w.dynasty.list[di].high_years = 29;
    court::fengshan(&mut w, di);
    assert_eq!(kinds(&w, EventKind::Fengshan), 0, "未滿 30 年");
    w.dynasty.list[di].high_years = 30;
    w.persons[r as usize].attrs.virtue = 7.9;
    court::fengshan(&mut w, di);
    assert_eq!(kinds(&w, EventKind::Fengshan), 0, "德行不足");
    w.persons[r as usize].attrs.virtue = 8.0;
    let g0 = w.souls[sid].merit;
    court::fengshan(&mut w, di);
    assert_eq!(kinds(&w, EventKind::Fengshan), 1);
    assert!((w.souls[sid].merit - g0 - 300.0).abs() < 1e-4);
    assert!((w.dynasty.list[di].mandate - 100.0).abs() < 1e-5, "91 + 10 截斷到 100");
    assert!((w.provinces[0].righteous - 0.2).abs() < 1e-6, "都城正氣 +0.2");
    court::fengshan(&mut w, di);
    assert_eq!(kinds(&w, EventKind::Fengshan), 1, "一位君主一生一次");
}

#[test]
fn gdd_3_7_high_years_counter() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    w.persons[r as usize].attrs.virtue = 5.0;
    w.provinces[0].righteous = 0.5;
    w.provinces[1].righteous = 0.5;
    w.dynasty.list[di].mandate = 95.0;
    w.tick = 12;
    yearly(&mut w, &[]);
    assert_eq!(w.dynasty.list[di].high_years, 1);
    w.dynasty.list[di].mandate = 50.0;
    w.provinces[0].righteous = 0.0;
    w.provinces[1].righteous = 0.0;
    w.tick = 24;
    yearly(&mut w, &[]);
    assert_eq!(w.dynasty.list[di].high_years, 0);
}

#[test]
fn gdd_3_7_prestige_formula() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    w.dynasty.list[di].prestige_mod = 20.0;
    w.tick = 12;
    yearly(&mut w, &[]);
    let d = &w.dynasty.list[di];
    assert!((d.prestige - (3.0 * 10.0 + d.mandate)).abs() < 1e-4);
    assert!((d.prestige_mod - 19.0).abs() < 1e-5);
}

#[test]
fn gdd_3_7_lost_province_leaves_dynasty() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    w.provinces[0].lost = true;
    w.tick = 12;
    yearly(&mut w, &[]);
    let d = &w.dynasty.list[di];
    assert_eq!(d.provinces, vec![1, 2]);
    assert_eq!(d.capital, 1, "都城沉沒遷到人口最多的剩餘州（同值 ID 小者）");
    assert!(w.dynasty.owner_of(0).is_none());
}

#[test]
fn gdd_3_7_yearly_founding_needs_pop_talent_and_free_land() {
    let mut w = line(6);
    // 州 4 有才略 9 的散人且人口足；州 0 才略不足。
    person(&mut w, 0, Role::Commoner, 7.9, 5.0);
    let hero = person(&mut w, 4, Role::Commoner, 9.0, 5.0);
    for y in 1..=400u32 {
        w.tick = y * 12;
        realm::yearly_found(&mut w, crate::schedule::Round::All);
        if !w.dynasty.list.is_empty() {
            break;
        }
    }
    assert_eq!(w.dynasty.list.len(), 1);
    let d = &w.dynasty.list[0];
    assert_eq!(d.ruler, Some(hero));
    assert_eq!(d.capital, 4);
    // 取得該州及無主的陸鄰。
    assert_eq!(d.provinces, vec![3, 4, 5]);
    assert_eq!(w.persons[hero as usize].role, Role::Ruler);
    // 人口不足 50,000 不立國。
    let mut w = line(2);
    w.provinces[1].population = 49_999.0;
    person(&mut w, 1, Role::Commoner, 9.5, 5.0);
    for y in 1..=400u32 {
        w.tick = y * 12;
        realm::yearly_found(&mut w, crate::schedule::Round::All);
    }
    assert!(w.dynasty.list.is_empty());
}

#[test]
fn gdd_3_7_royal_birth_needs_spouse_and_caps_heirs() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let r = w.dynasty.list[di].ruler.unwrap();
    for _ in 0..100 {
        court::royal_birth(&mut w, di);
    }
    assert!(w.dynasty.list[di].heirs.is_empty(), "無配偶不生育");
    let s = person(&mut w, 0, Role::Commoner, 5.0, 5.0);
    w.persons[r as usize].spouse = Some(s);
    for _ in 0..300 {
        court::royal_birth(&mut w, di);
    }
    let d = &w.dynasty.list[di];
    assert_eq!(d.heirs.len(), ROYAL_MAX, "至多 5 名");
    for &h in &d.heirs {
        let p = &w.persons[h as usize];
        assert_eq!(p.role, Role::Royal);
        assert!(w.persons[r as usize].children.contains(&h));
        assert_eq!(p.age(w.tick), 0);
    }
}

#[test]
fn gdd_3_7_prune_removes_dead_and_changed() {
    let mut w = line(2);
    let di = dynasty(&mut w, 0, &[0, 1]);
    let a = person(&mut w, 0, Role::Official, 5.0, 5.0);
    let b = person(&mut w, 0, Role::Official, 5.0, 5.0);
    let g = person(&mut w, 1, Role::General, 5.0, 5.0);
    let h = person(&mut w, 0, Role::Royal, 5.0, 5.0);
    {
        let d = &mut w.dynasty.list[di];
        d.officials = vec![a, b];
        d.generals = vec![(g, 1)];
        d.heirs = vec![h];
    }
    kill(&mut w, a);
    kill(&mut w, g);
    kill(&mut w, h);
    court::prune(&mut w, di);
    let d = &w.dynasty.list[di];
    assert_eq!(d.officials, vec![b]);
    assert!(d.generals.is_empty() && d.heirs.is_empty());
}

#[test]
fn gdd_3_7_determinism_two_runs_same_state() {
    fn run() -> u64 {
        let mut w = line(10);
        let di = dynasty(&mut w, 0, &(0..10).collect::<Vec<_>>());
        for p in 0..10 {
            person(&mut w, p, Role::Scholar, 5.0, 6.0);
            person(&mut w, p, Role::GeneralCandidate, 6.0 + p as f32 * 0.1, 5.0);
        }
        w.dynasty.list[di].mandate = 30.0;
        for y in 1..=60u32 {
            w.tick = y * 12;
            yearly(&mut w, &[]);
        }
        crate::hash::world_hash(&w)
    }
    assert_eq!(run(), run());
}

/// GDD 6.3 判定月：每個在世王朝一年恰好結算一次，落在自己的判定月；新生王朝滿 12 個月後才開始。
#[test]
fn gdd_3_7_each_dynasty_settles_exactly_once_per_year_in_monthly_rounds() {
    let mut w = line(6);
    let a = dynasty(&mut w, 0, &[0, 1, 2]);
    let b = dynasty(&mut w, 5, &[3, 4, 5]);
    for di in [a, b] {
        w.dynasty.list[di].mandate = 60.0;
        w.dynasty.list[di].treasury = 1.0e12;
    }
    super::probe_start();
    let start = w.tick;
    for _ in 0..36 {
        w.tick += 1;
        monthly(&mut w);
    }
    let records = super::probe_take();
    for di in [a, b] {
        let ticks: Vec<_> = records.iter().filter(|r| r.1 == di as DynastyId).map(|r| r.0).collect();
        let month = crate::schedule::judge_month(w.params.seed, crate::schedule::Domain::Dynasty, di as u32);
        assert!(ticks.iter().all(|&t| crate::schedule::month_of(t) == month), "{ticks:?}");
        // 36 個月內：新生王朝要滿 12 個月才開始，第 12–36 個月之間的判定月各一次。
        let expected = (start + 12..=start + 36).filter(|&t| crate::schedule::month_of(t) == month).count();
        assert_eq!(ticks.len(), expected, "王朝 {di} 的結算 tick {ticks:?}");
        assert!((1..=3).contains(&ticks.len()));
        let gaps: Vec<_> = ticks.windows(2).map(|p| p[1] - p[0]).collect();
        assert!(gaps.iter().all(|&g| g == 12), "{gaps:?}");
    }
}

/// 天命的饑荒、得失州項是滾動 12 個月的累計：12 個月後從累計扣除。
#[test]
fn gdd_3_7_famine_and_flow_terms_roll_over_twelve_months() {
    let mut w = line(3);
    let di = dynasty(&mut w, 0, &[0, 1, 2]);
    w.tick = 1000;
    w.dynasty.list[di].note_flow(1000, 1, 0, 0);
    super::on_famines(&mut w, &[(1, 100), (2, 100)]);
    assert_eq!(w.dynasty.list[di].famine, 2);
    assert_eq!(w.dynasty.list[di].gained, 1);
    w.tick = 1011;
    w.dynasty.list[di].expire_flow(1011);
    assert_eq!((w.dynasty.list[di].famine, w.dynasty.list[di].gained), (2, 1), "第 12 個月仍在窗口內");
    w.tick = 1012;
    w.dynasty.list[di].expire_flow(1012);
    assert_eq!((w.dynasty.list[di].famine, w.dynasty.list[di].gained, w.dynasty.list[di].lost), (0, 0, 0));
}
