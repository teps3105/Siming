//! U5 單元測試：屬性分布、壽元、入道、婚配、死亡與魂、戰力、遊歷、確定性。

use std::collections::BTreeMap;

use super::*;
use crate::model::*;
use crate::params::*;
use crate::soul;
use crate::types::{Geography, Province, Terrain, WorldParams};

fn province(id: u16, pop: f64, neighbors: Vec<u16>, aura: f32) -> Province {
    Province {
        id,
        name: format!("p{id}"),
        cells: vec![id as u32 * 2, id as u32 * 2 + 1],
        seat: id as u32 * 2,
        neighbors,
        sea_neighbors: vec![],
        terrain_mix: [0; Terrain::COUNT],
        habitability: 1.0,
        capacity: pop,
        population: pop,
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

/// 一條直線上的 n 個州郡：i 與 i±1 為陸鄰；靈氣 0.5。
fn line_world(seed: u64, n: usize) -> World {
    let provs = (0..n)
        .map(|i| {
            let mut nb = vec![];
            if i > 0 {
                nb.push(i as u16 - 1);
            }
            if i + 1 < n {
                nb.push(i as u16 + 1);
            }
            province(i as u16, 1_000_000.0, nb, 0.5)
        })
        .collect();
    let mut w = World::empty(WorldParams::new(seed), geo(), provs);
    soul::ensure_heaven(&mut w);
    w
}

fn new_life(w: &mut World, prov: u16) -> (SoulId, PersonId) {
    soul::spawn_new_life(w, SoulOrigin::New, prov)
}

/// 讓人物直接成為指定年齡的成年人（改出生時間），並定為指定身分。
fn make_adult(w: &mut World, pid: PersonId, age: u32, role: Role, path: PathState) {
    let p = &mut w.persons[pid as usize];
    p.born = w.tick as i32 - (age * 12) as i32;
    p.role = role;
    p.path = path;
}

fn cultivator(realm: u8) -> PathState {
    PathState::Cultivator { realm, exp: 0.0, last_attempt: None }
}

fn sect(id: PowerId, province: u16, prestige: f32) -> Power {
    Power {
        id,
        kind: PowerKind::Sect(Sect { evil: false, seat_node: None, radius: 1, ascended: 0 }),
        name: format!("s{id}"),
        seat_province: province,
        seat_cell: province as u32 * 2,
        leader: None,
        members: vec![],
        founded: 0,
        fallen: None,
        prestige,
        prestige_mod: 0.0,
        relations: BTreeMap::new(),
    }
}

fn kinds(w: &World, k: EventKind) -> usize {
    w.ledger.events.iter().filter(|e| e.kind == k).count()
}

// ── 出生 ──────────────────────────────────────────────────

#[test]
fn attribute_distributions_at_fixed_seed() {
    let mut w = line_world(7, 1);
    let mut soul = w.souls[0].clone();
    soul.wisdom = 0.0;
    let n = 4000;
    let (mut r, mut i, mut t, mut v, mut f) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for _ in 0..n {
        let a = roll_attrs(&mut w.rng.people, &soul);
        for x in [a.root, a.insight, a.talent, a.virtue, a.ferocity] {
            assert!((0.0..=10.0).contains(&x));
        }
        assert!(a.root <= 6.0 && a.insight <= 6.0 && a.talent <= 8.0 && a.ferocity <= 6.0);
        assert!((2.0..=8.0).contains(&a.virtue));
        r += a.root;
        i += a.insight;
        t += a.talent;
        v += a.virtue;
        f += a.ferocity;
    }
    let m = |x: f32| x / n as f32;
    assert!((m(r) - 3.0).abs() < 0.15, "root {}", m(r));
    assert!((m(i) - 3.0).abs() < 0.15);
    assert!((m(t) - 4.0).abs() < 0.2);
    assert!((m(v) - 5.0).abs() < 0.15);
    assert!((m(f) - 3.0).abs() < 0.15);
}

#[test]
fn attributes_follow_soul() {
    let mut w = line_world(8, 1);
    let mut s = w.souls[0].clone();
    s.wisdom = 80.0; // 根骨、悟性 +2
    s.merit = 0.0;
    s.sin = 200.0; // 德行 −2
    s.marks.set(SoulMarks::ASURA);
    for _ in 0..500 {
        let a = roll_attrs(&mut w.rng.people, &s);
        assert!(a.root >= 2.0 && a.root <= 8.0);
        assert!(a.insight >= 2.0 && a.insight <= 8.0);
        assert!((0.0..=6.0).contains(&a.virtue));
        assert!(a.ferocity >= 4.0);
    }
    s.wisdom = 200.0; // +5，截斷到 10 以內
    let a = roll_attrs(&mut w.rng.people, &s);
    assert!(a.root <= 10.0 && a.root >= 5.0);
}

#[test]
fn lifespan_distribution_truncated() {
    let mut w = line_world(9, 1);
    let n = 4000;
    let mut sum = 0.0;
    for _ in 0..n {
        let l = spawn::mortal_lifespan(&mut w.rng.people);
        assert!((MORTAL_LIFESPAN_MIN..=MORTAL_LIFESPAN_MAX).contains(&(l as u32)));
        sum += l as f32;
    }
    assert!((sum / n as f32 - 62.0).abs() < 1.0);
}

#[test]
fn spawn_via_soul_births() {
    let mut w = line_world(10, 2);
    let (sid, pid) = new_life(&mut w, 1);
    let p = &w.persons[pid as usize];
    assert_eq!(p.soul, sid);
    assert_eq!(p.role, Role::Child);
    assert_eq!(p.path, PathState::Child);
    assert!(!p.name.is_empty());
    assert!(p.attrs.root > 0.0 || p.attrs.talent >= 0.0);
    assert_eq!(p.born, w.tick as i32);
    assert_eq!(w.souls[sid as usize].state, SoulState::Incarnate(pid));
    // 名字允許 2–4 字（姓 1–2 + 名 1–2）
    let n = p.name.chars().count();
    assert!((2..=4).contains(&n), "{}", p.name);
}

// ── 入道 ──────────────────────────────────────────────────

fn force_attrs(w: &mut World, pid: PersonId, root: f32, insight: f32, talent: f32) {
    let a = &mut w.persons[pid as usize].attrs;
    a.root = root;
    a.insight = insight;
    a.talent = talent;
}

fn set_aff(w: &mut World, sid: SoulId, aff: [f32; 4]) {
    w.souls[sid as usize].affinity = aff;
}

#[test]
fn enter_path_tao_with_and_without_sect() {
    let mut w = line_world(11, 3);
    let (s, p) = new_life(&mut w, 1);
    force_attrs(&mut w, p, 5.0, 0.0, 0.0);
    set_aff(&mut w, s, [1.0, 0.0, 0.0, 0.0]);
    // 無宗門 → 散修
    assert_eq!(enter_path(&mut w, p), Role::Cultivator);
    assert_eq!(w.persons[p as usize].power, None);
    assert!(w.persons[p as usize].lifespan >= REALM_LIFESPAN[0] as u16);
    assert_eq!(w.persons[p as usize].path, cultivator(0));

    // 陸鄰有兩個宗門 → 聲望高者
    w.powers.push(sect(0, 0, 10.0));
    w.powers.push(sect(1, 2, 80.0));
    let (s, p) = new_life(&mut w, 1);
    force_attrs(&mut w, p, 5.0, 0.0, 0.0);
    set_aff(&mut w, s, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(enter_path(&mut w, p), Role::Cultivator);
    assert_eq!(w.persons[p as usize].power, Some(1));
    assert_eq!(w.powers[1].members, vec![p]);
}

#[test]
fn enter_path_full_sect_becomes_rogue() {
    let mut w = line_world(12, 2);
    let mut sc = sect(0, 1, 0.0);
    sc.members = (100..100 + sect_capacity(0.0) as u32).collect();
    w.powers.push(sc);
    let (s, p) = new_life(&mut w, 1);
    force_attrs(&mut w, p, 5.0, 0.0, 0.0);
    set_aff(&mut w, s, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(enter_path(&mut w, p), Role::Cultivator);
    assert_eq!(w.persons[p as usize].power, None);
    assert_eq!(sect_capacity(0.0), SECT_SIZE_BASE as usize);
    assert_eq!(sect_capacity(1e6), SECT_SIZE_MAX as usize);
}

#[test]
fn enter_path_monk_scholar_demon_commoner() {
    let mut w = line_world(13, 2);
    // 釋道緣 ≥ 0.2，根骨與悟性不足 → 遊方僧，壽元 +10
    let (s, p) = new_life(&mut w, 0);
    force_attrs(&mut w, p, 1.0, 1.0, 0.0);
    set_aff(&mut w, s, [0.0, 1.0, 0.0, 0.0]);
    let before = w.persons[p as usize].lifespan;
    assert_eq!(enter_path(&mut w, p), Role::Monk);
    assert_eq!(w.persons[p as usize].path, PathState::Monk { fruit: 0, practice: 0.0 });
    assert_eq!(w.persons[p as usize].lifespan, before + MONK_LIFESPAN_BONUS as u16);
    // 悟性 ≥ 3 → 散儒
    let (s, p) = new_life(&mut w, 0);
    force_attrs(&mut w, p, 1.0, 4.0, 0.0);
    set_aff(&mut w, s, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(enter_path(&mut w, p), Role::Scholar);
    // 修羅印記 → 魔修
    let (s, p) = new_life(&mut w, 0);
    force_attrs(&mut w, p, 1.0, 1.0, 0.0);
    set_aff(&mut w, s, [0.0, 0.0, 0.0, 1.0]);
    w.souls[s as usize].marks.set(SoulMarks::ASURA);
    assert_eq!(enter_path(&mut w, p), Role::Demonic);
    assert_eq!(w.persons[p as usize].path, cultivator(0));
    // 濁氣 > 0.5 的州郡亦可入魔
    w.provinces[1].karma_qi = 0.9;
    let (s, p) = new_life(&mut w, 1);
    force_attrs(&mut w, p, 1.0, 1.0, 0.0);
    set_aff(&mut w, s, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(enter_path(&mut w, p), Role::Demonic);
    // 無門可入 → 散人
    let (s, p) = new_life(&mut w, 0);
    force_attrs(&mut w, p, 1.0, 1.0, 9.0);
    set_aff(&mut w, s, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(enter_path(&mut w, p), Role::Commoner);
    assert_eq!(w.persons[p as usize].path, PathState::Mortal);
}

#[test]
fn enter_path_weighted_by_affinity() {
    let mut w = line_world(14, 1);
    let (mut tao, mut ru) = (0, 0);
    for _ in 0..400 {
        let (s, p) = new_life(&mut w, 0);
        force_attrs(&mut w, p, 5.0, 5.0, 0.0);
        set_aff(&mut w, s, [0.75, 0.0, 0.25, 0.0]);
        match enter_path(&mut w, p) {
            Role::Cultivator => tao += 1,
            Role::Scholar => ru += 1,
            r => panic!("{r:?}"),
        }
    }
    assert!(tao > ru * 2 && ru > 50, "tao {tao} ru {ru}");
}

#[test]
fn coming_of_age_at_twelve() {
    let mut w = line_world(15, 1);
    let (s, p) = new_life(&mut w, 0);
    force_attrs(&mut w, p, 5.0, 0.0, 0.0);
    set_aff(&mut w, s, [1.0, 0.0, 0.0, 0.0]);
    w.tick = 12 * 12 - 1;
    yearly_coming_of_age(&mut w);
    assert_eq!(w.persons[p as usize].role, Role::Child);
    w.tick = 12 * 12;
    yearly_coming_of_age(&mut w);
    assert_eq!(w.persons[p as usize].role, Role::Cultivator);
    assert_eq!(kinds(&w, EventKind::ComeOfAge), 1);
    let born = w.ledger.events.iter().find(|e| e.kind == EventKind::Born).unwrap().id;
    let coa = w.ledger.events.iter().find(|e| e.kind == EventKind::ComeOfAge).unwrap();
    assert_eq!(coa.causes.as_slice(), &[born]);
    assert_eq!(coa.args, EventArgs::Role { role: Role::Cultivator });
    // 不重複入道
    yearly_coming_of_age(&mut w);
    assert_eq!(kinds(&w, EventKind::ComeOfAge), 1);
}

// ── 婚配 ──────────────────────────────────────────────────

fn adults(w: &mut World, specs: &[(u16, u32, Role)]) -> Vec<PersonId> {
    let mut out = vec![];
    for &(prov, age, role) in specs {
        let (_, p) = new_life(w, prov);
        let path = if role == Role::Monk { PathState::Monk { fruit: 0, practice: 0.0 } } else { PathState::Mortal };
        make_adult(w, p, age, role, path);
        out.push(p);
    }
    out
}

fn run_marriage_years(w: &mut World, years: u32) {
    for _ in 0..years {
        w.tick += 12;
        yearly_marriage(w);
    }
}

#[test]
fn marriage_pairs_opposite_gender_adults() {
    let mut w = line_world(16, 3);
    w.tick = 1200;
    let ids = adults(
        &mut w,
        &[
            (0, 20, Role::Commoner),
            (1, 22, Role::Commoner),
            (1, 45, Role::Commoner), // 年齡差過大
            (2, 21, Role::Commoner), // 與 0 不相鄰
            (0, 30, Role::Monk),     // 僧人不婚
            (0, 10, Role::Child),    // 幼年
        ],
    );
    w.persons[ids[0] as usize].gender = Gender::Male;
    w.persons[ids[1] as usize].gender = Gender::Female;
    w.persons[ids[2] as usize].gender = Gender::Female;
    w.persons[ids[3] as usize].gender = Gender::Female;
    w.persons[ids[4] as usize].gender = Gender::Female;
    w.persons[ids[5] as usize].gender = Gender::Female;
    w.persons[ids[5] as usize].path = PathState::Child;
    run_marriage_years(&mut w, 200);
    // 0 與 1 結為夫妻（互為配偶）
    assert_eq!(w.persons[ids[0] as usize].spouse, Some(ids[1]));
    assert_eq!(w.persons[ids[1] as usize].spouse, Some(ids[0]));
    for &i in &ids[2..] {
        // 2 年齡差 25；3 與 0 不相鄰，且與 1 相鄰但 1 已婚或同為女性；僧人、幼年不婚
        assert!(w.persons[i as usize].spouse.is_none() || i == ids[2] || i == ids[3]);
    }
    assert!(w.persons[ids[4] as usize].spouse.is_none());
    assert!(w.persons[ids[5] as usize].spouse.is_none());
    assert_eq!(kinds(&w, EventKind::Married), 1);
    // 情緣線建立
    let (a, b) = (w.persons[ids[0] as usize].soul, w.persons[ids[1] as usize].soul);
    assert_eq!(w.threads.len(), 1);
    let t = &w.threads[0];
    assert_eq!(t.kind, ThreadKind::Love);
    assert_eq!(t.strength, THREAD_INIT_LOVE);
    assert!(w.souls[a as usize].threads.contains(&0) && w.souls[b as usize].threads.contains(&0));
}

#[test]
fn marriage_prefers_same_sect() {
    let mut w = line_world(17, 1);
    w.tick = 1200;
    w.powers.push(sect(0, 0, 0.0));
    let ids = adults(&mut w, &[(0, 20, Role::Cultivator), (0, 20, Role::Cultivator), (0, 20, Role::Cultivator)]);
    w.persons[ids[0] as usize].gender = Gender::Male;
    w.persons[ids[1] as usize].gender = Gender::Female;
    w.persons[ids[2] as usize].gender = Gender::Female;
    for &i in &ids[0..2] {
        w.persons[i as usize].power = Some(0);
    }
    run_marriage_years(&mut w, 300);
    assert_eq!(w.persons[ids[0] as usize].spouse, Some(ids[1]));
    assert_eq!(w.persons[ids[2] as usize].spouse, None);
}

#[test]
fn remarriage_resolves_existing_love_thread() {
    let mut w = line_world(18, 1);
    w.tick = 1200;
    let ids = adults(&mut w, &[(0, 20, Role::Commoner), (0, 20, Role::Commoner)]);
    let (sa, sb) = (w.persons[ids[0] as usize].soul, w.persons[ids[1] as usize].soul);
    // 前世建立的情緣線：兩魂各自已轉世一次
    let birth = kinds(&w, EventKind::Born) as EventId;
    w.threads.push(Thread {
        id: 0,
        kind: ThreadKind::Love,
        debtor: sa,
        creditor: sb,
        strength: 30.0,
        created: (0, birth),
        lives_base: (0, 0),
        woven: false,
        resolved: None,
        omen_at: None,
        master_best: None,
    });
    w.souls[sa as usize].threads.push(0);
    w.souls[sb as usize].threads.push(0);
    w.souls[sa as usize].lives.insert(0, 999);
    w.souls[sb as usize].lives.insert(0, 998);
    marry(&mut w, ids[0], ids[1]);
    assert!(w.threads[0].resolved.is_some());
    assert_eq!(w.threads.len(), 1);
    assert_eq!(w.souls[sa as usize].merit, 30.0 * RESOLVE_OTHER_G_COEF);
    assert_eq!(kinds(&w, EventKind::ThreadResolved), 1);

    // 同一世建立的線（lives = 0）→ 累加強度
    let mut w = line_world(19, 1);
    w.tick = 1200;
    let ids = adults(&mut w, &[(0, 20, Role::Commoner), (0, 20, Role::Commoner)]);
    marry(&mut w, ids[0], ids[1]);
    w.persons[ids[0] as usize].spouse = None;
    w.persons[ids[1] as usize].spouse = None;
    marry(&mut w, ids[0], ids[1]);
    assert_eq!(w.threads.len(), 1);
    assert_eq!(w.threads[0].strength, 2.0 * THREAD_INIT_LOVE);
    assert!(w.threads[0].resolved.is_none());
}

// ── 死亡 ──────────────────────────────────────────────────

#[test]
fn death_writes_event_then_soul_bardo() {
    let mut w = line_world(20, 2);
    w.tick = 1000;
    w.powers.push(sect(0, 0, 0.0));
    let ids = adults(&mut w, &[(0, 30, Role::Cultivator), (0, 30, Role::Commoner), (0, 50, Role::Cultivator), (0, 15, Role::Cultivator)]);
    let (a, b, master, disciple) = (ids[0], ids[1], ids[2], ids[3]);
    w.persons[a as usize].power = Some(0);
    w.powers[0].members = vec![a, master, disciple];
    w.persons[a as usize].spouse = Some(b);
    w.persons[b as usize].spouse = Some(a);
    w.persons[a as usize].master = Some(master);
    w.persons[master as usize].disciples = vec![a];
    w.persons[a as usize].disciples = vec![disciple];
    w.persons[disciple as usize].master = Some(a);
    w.persons[a as usize].path = cultivator(3);
    let sid = w.persons[a as usize].soul;
    let prev = w.persons[a as usize].last_event;

    let eid = die(&mut w, a, DeathCause::Age, None, &[]).unwrap();
    let e = &w.ledger.events[eid as usize];
    assert_eq!(e.kind, EventKind::Died);
    assert_eq!(e.importance, 3); // 元嬰
    assert_eq!(e.causes.as_slice(), prev.as_slice());
    assert_eq!(e.args, EventArgs::Death { cause: DeathCause::Age, age: 30 });
    assert_eq!(w.persons[a as usize].last_event, Some(eid));
    assert_eq!(w.persons[a as usize].died, Some((1000, DeathCause::Age)));
    // 魂
    assert_eq!(w.souls[sid as usize].state, SoulState::Bardo { until: 1000 + BARDO_MONTHS });
    assert_eq!(w.souls[sid as usize].best_now.realm, Some(3));
    // 勢力、人際
    assert!(!w.powers[0].members.contains(&a));
    assert_eq!(w.persons[b as usize].spouse, None);
    assert!(w.persons[master as usize].disciples.is_empty());
    assert_eq!(w.persons[disciple as usize].master, None);
    // 重複死亡不做事
    assert_eq!(die(&mut w, a, DeathCause::Age, None, &[]), None);
    assert_eq!(kinds(&w, EventKind::Died), 1);
}

#[test]
fn ascension_liberates_soul() {
    let mut w = line_world(21, 1);
    w.tick = 5000;
    let ids = adults(&mut w, &[(0, 300, Role::Cultivator)]);
    let sid = w.persons[ids[0] as usize].soul;
    let eid = die(&mut w, ids[0], DeathCause::Ascended, Some(Liberation::Ascension), &[]).unwrap();
    assert_eq!(w.ledger.events[eid as usize].kind, EventKind::Ascended);
    assert_eq!(w.ledger.events[eid as usize].importance, 5);
    assert_eq!(w.souls[sid as usize].state, SoulState::Liberated { way: Liberation::Ascension, tick: 5000 });
}

#[test]
fn demon_death_marks_soul() {
    let mut w = line_world(22, 1);
    let ids = adults(&mut w, &[(0, 40, Role::Demonic)]);
    let sid = w.persons[ids[0] as usize].soul;
    die(&mut w, ids[0], DeathCause::Killed, None, &[]);
    assert!(w.souls[sid as usize].marks.has(SoulMarks::LAST_LIFE_DEMON));
}

#[test]
fn aging_kills_at_lifespan_and_soul_reincarnates() {
    let mut w = line_world(23, 2);
    let (sid, pid) = new_life(&mut w, 0);
    w.persons[pid as usize].lifespan = 40;
    w.tick = 40 * 12 - 12;
    monthly_aging(&mut w);
    assert!(w.persons[pid as usize].alive());
    w.tick = 40 * 12;
    monthly_aging(&mut w);
    assert_eq!(w.persons[pid as usize].died, Some((480, DeathCause::Age)));
    assert!(matches!(w.souls[sid as usize].state, SoulState::Bardo { .. }));
    // 兩個月後中陰到期，魂判定；人道則轉世並建立新肉身（人道以外則入他道）
    for _ in 0..BARDO_MONTHS {
        w.tick += 1;
        soul::monthly(&mut w);
    }
    match &w.souls[sid as usize].state {
        SoulState::Incarnate(p) => {
            assert_ne!(*p, pid);
            assert_eq!(w.persons[*p as usize].soul, sid);
            assert_eq!(w.persons[*p as usize].role, Role::Child);
            assert_eq!(w.souls[sid as usize].lives.len(), 2);
        }
        SoulState::Realm { .. } | SoulState::Waiting { .. } | SoulState::Bardo { .. } => {}
        s => panic!("{s:?}"),
    }
    assert!(kinds(&w, EventKind::Judged) >= 1);
}

// ── 戰力 ──────────────────────────────────────────────────

#[test]
fn combat_power_table() {
    let mut w = line_world(24, 1);
    let ids = adults(&mut w, &[(0, 30, Role::Cultivator); 6]);
    let t = |w: &mut World, i: usize, role: Role, path: PathState, talent: f32| {
        let p = &mut w.persons[ids[i] as usize];
        p.role = role;
        p.path = path;
        p.attrs.talent = talent;
    };
    t(&mut w, 0, Role::Cultivator, PathState::Cultivator { realm: 2, exp: 0.0, last_attempt: None }, 0.0);
    assert!((power_of(&w, ids[0]) - 9.0 * 1.1).abs() < 1e-4);
    // 修滿：層 10
    t(&mut w, 0, Role::Cultivator, PathState::Cultivator { realm: 2, exp: REALM_EXP[2], last_attempt: None }, 0.0);
    assert!((power_of(&w, ids[0]) - 9.0 * 2.0).abs() < 1e-4);
    t(&mut w, 1, Role::Monk, PathState::Monk { fruit: 2, practice: 0.0 }, 0.0);
    assert_eq!(power_of(&w, ids[1]), 18.0);
    t(&mut w, 2, Role::General, PathState::Mortal, 5.0);
    assert_eq!(power_of(&w, ids[2]), 8.0);
    t(&mut w, 3, Role::Ruler, PathState::Mortal, 6.0);
    assert_eq!(power_of(&w, ids[3]), 5.0);
    assert_eq!(combat_power_as_target(&w.persons[ids[3] as usize]), 15.0);
    t(&mut w, 4, Role::Scholar, PathState::Scholar { virtue: 0.0, deed: 0.0, word: 0.0 }, 5.0);
    assert_eq!(power_of(&w, ids[4]), 2.0);
    t(&mut w, 5, Role::Child, PathState::Child, 5.0);
    assert_eq!(power_of(&w, ids[5]), 0.0);
    assert_eq!(layer(0, 0.0), 1);
    assert_eq!(layer(0, 1e9), LAYER_MAX);
}

// ── 遊歷 ──────────────────────────────────────────────────

#[test]
fn travel_candidates_within_three_steps() {
    let w = line_world(25, 8);
    assert_eq!(travel_candidates(&w, 0), vec![1, 2, 3]);
    assert_eq!(travel_candidates(&w, 4), vec![1, 2, 3, 5, 6, 7]);
}

#[test]
fn travel_departs_and_returns() {
    let mut w = line_world(26, 6);
    w.powers.push(sect(0, 2, 0.0));
    let ids = adults(
        &mut w,
        &[
            (2, 40, Role::Cultivator), // 築基，可出遊
            (2, 40, Role::Cultivator), // 煉氣，不遊
            (2, 40, Role::Cultivator), // 掌門，不遊
            (2, 40, Role::Monk),       // 僧人，不遊
        ],
    );
    w.persons[ids[0] as usize].path = cultivator(1);
    w.persons[ids[1] as usize].path = cultivator(0);
    w.persons[ids[2] as usize].path = cultivator(3);
    w.persons[ids[2] as usize].power = Some(0);
    w.powers[0].leader = Some(ids[2]);
    w.tick = 1200;
    let start_cell = w.persons[ids[0] as usize].cell;
    let mut departed = None;
    for _ in 0..200 {
        w.tick += 12;
        yearly_travel(&mut w);
        if w.persons[ids[0] as usize].travel_until.is_some() {
            departed = Some(w.tick);
            break;
        }
    }
    let t0 = departed.expect("應在 200 年內出遊");
    let p = &w.persons[ids[0] as usize];
    assert_ne!(p.province, 2);
    assert!(p.province.abs_diff(2) <= TRAVEL_RANGE_STEPS as u16);
    assert_eq!(p.home, 2);
    let stay = p.travel_until.unwrap() - t0;
    assert!((12..=60).contains(&stay), "{stay}");
    assert!(w.provinces[p.province as usize].cells.contains(&p.cell));
    let ev = w.ledger.events.iter().rev().find(|e| e.kind == EventKind::Traveled).unwrap();
    assert_eq!(ev.args, EventArgs::Travel { from: 2, to: p.province });
    assert_eq!(ev.cell, Some(start_cell));
    for &i in &ids[1..] {
        assert_eq!(w.persons[i as usize].province, 2);
        assert!(w.persons[i as usize].travel_until.is_none());
    }
    // 遊歷期間不再出遊；到期返回
    let until = p.travel_until.unwrap();
    w.tick = until - 1;
    monthly_due(&mut w);
    assert_ne!(w.persons[ids[0] as usize].province, 2);
    w.tick = until;
    monthly_due(&mut w);
    let p = &w.persons[ids[0] as usize];
    assert_eq!((p.province, p.travel_until), (2, None));
    assert_eq!(kinds(&w, EventKind::Traveled), 2);
    let back = w.ledger.events.last().unwrap();
    assert_eq!(back.kind, EventKind::Traveled);
    assert_eq!(back.args, EventArgs::Travel { from: back.province.unwrap(), to: 2 });
}

#[test]
fn travel_weighted_by_aura() {
    let mut w = line_world(27, 3);
    w.provinces[0].aura = 1.0;
    w.provinces[2].aura = 0.0;
    let ids = adults(&mut w, &[(1, 40, Role::Cultivator)]);
    w.persons[ids[0] as usize].path = cultivator(1);
    for _ in 0..300 {
        w.tick += 12;
        yearly_travel(&mut w);
        let p = &mut w.persons[ids[0] as usize];
        if p.travel_until.is_some() {
            assert_eq!(p.province, 0, "靈氣為 0 的州郡權重為 0");
            p.travel_until = None;
            p.province = 1;
        }
    }
}

#[test]
fn blessing_expires() {
    let mut w = line_world(28, 1);
    let (_, p) = new_life(&mut w, 0);
    w.persons[p as usize].blessed_until = Some(10);
    w.tick = 9;
    monthly_due(&mut w);
    assert_eq!(w.persons[p as usize].blessed_until, Some(10));
    w.tick = 10;
    monthly_due(&mut w);
    assert_eq!(w.persons[p as usize].blessed_until, None);
}

// ── 確定性 ────────────────────────────────────────────────

/// 以 `Sim::step` 的順序（只含 people 與 soul）推進 `years` 年。
fn drive(seed: u64, years: u32) -> World {
    let mut w = line_world(seed, 5);
    for p in 0..5 {
        for _ in 0..30 {
            new_life(&mut w, p);
        }
    }
    for _ in 0..years * 12 {
        w.tick += 1;
        monthly_due(&mut w);
        soul::monthly(&mut w);
        if w.tick.is_multiple_of(12) {
            monthly_aging(&mut w);
            yearly_travel(&mut w);
            yearly_coming_of_age(&mut w);
            yearly_marriage(&mut w);
            soul::yearly_refill(&mut w);
        }
    }
    w
}

#[test]
fn deterministic_and_lives_a_full_cycle() {
    let a = drive(99, 150);
    let b = drive(99, 150);
    assert_eq!(a.persons, b.persons);
    assert_eq!(a.souls, b.souls);
    assert_eq!(a.threads, b.threads);
    assert_eq!(a.ledger, b.ledger);
    let c = drive(100, 150);
    assert_ne!(a.persons, c.persons);

    // 整個流程實際發生過
    assert!(kinds(&a, EventKind::ComeOfAge) > 50);
    assert!(kinds(&a, EventKind::Married) > 10);
    assert!(kinds(&a, EventKind::Died) > 50);
    assert!(kinds(&a, EventKind::Rebirth) > 20);
    assert!(a.souls.iter().any(|s| s.lives.len() >= 2));
    // 在世者的魂與肉身互相對應
    for p in a.persons.iter().filter(|p| p.alive()) {
        assert_eq!(a.souls[p.soul as usize].state, SoulState::Incarnate(p.id));
        assert!(p.attrs.root >= 0.0 && p.attrs.root <= 10.0);
        if let Some(s) = p.spouse {
            assert_eq!(a.persons[s as usize].spouse, Some(p.id));
            assert!(a.persons[s as usize].alive());
        }
    }
    // 沒有人壽元超過
    for p in a.persons.iter().filter(|p| p.alive()) {
        assert!(p.age(a.tick) < p.lifespan as u32);
    }
}

// ── 分月判定（GDD 6.3 判定月）────────────────────────────

/// 入道在自己的判定月：滿 12 歲前不入道；滿 12 歲後最近一次自己的判定月入道（至多晚 12 個月），且只入一次。
/// 年中才滿 12 歲者等到下一個自己的判定月。
#[test]
fn gdd_3_4_coming_of_age_waits_for_own_judge_month() {
    for seed in [15u64, 16, 17] {
        let mut w = line_world(seed, 1);
        w.tick = 1;
        let (s, p) = new_life(&mut w, 0);
        force_attrs(&mut w, p, 5.0, 0.0, 0.0);
        set_aff(&mut w, s, [1.0, 0.0, 0.0, 0.0]);
        let born = w.persons[p as usize].born.max(0) as u32;
        let month = crate::schedule::judge_month(seed, crate::schedule::Domain::Person, p);
        let mut entered_at = None;
        for t in 2..=(born + 12 * 14) {
            w.tick = t;
            monthly_coming_of_age(&mut w);
            if entered_at.is_none() && w.persons[p as usize].role != Role::Child {
                entered_at = Some(t);
            }
        }
        let t = entered_at.expect("應已入道");
        assert!(w.persons[p as usize].age(t) >= COMING_OF_AGE_YEARS, "未滿 12 歲不入道");
        assert_eq!(crate::schedule::month_of(t), month, "入道月須是自己的判定月");
        assert!(t <= born + 12 * 12 + 12, "至多晚 12 個月");
        assert_eq!(kinds(&w, EventKind::ComeOfAge), 1);
    }
}

/// 婚配的機率抽樣一年恰好一次：連續 12 個月的單月輪次，消耗的亂數量與一次整年輪次相同
/// （人物同性別，候選為空，亂數只來自各人各一次的抽樣）。
#[test]
fn gdd_6_3_marriage_roll_once_per_year() {
    let mut w = line_world(33, 2);
    w.tick = 1200;
    for i in 0..20 {
        let (_, p) = new_life(&mut w, (i % 2) as u16);
        make_adult(&mut w, p, 30, Role::Commoner, PathState::Mortal);
        w.persons[p as usize].gender = Gender::Male;
    }
    let mut yearly_world = w.clone();
    yearly_marriage(&mut yearly_world);
    for _ in 0..12 {
        w.tick += 1;
        monthly_marriage(&mut w);
    }
    assert_eq!(yearly_world.rng.people.get_word_pos(), w.rng.people.get_word_pos());
    assert!(yearly_world.rng.people.get_word_pos() > 0);
}
