//! U8 單元測試：開天人數與宗門數（4.6）、金字塔、宿緣、開局保證、道號、確定性。

use super::*;
use crate::types::WorldParams;

/// 世界生成後只執行 [`build`]（不預跑、不保證）。
fn built(seed: u64, era: Era) -> World {
    let mut p = WorldParams::new(seed);
    p.era = era;
    let g = crate::worldgen::generate(&p).expect("worldgen");
    let mut w = World::empty(g.params, g.geo, g.provinces);
    build(&mut w);
    w
}

/// 完整開天（含預跑與保證）。
fn full(seed: u64, era: Era) -> Sim {
    let mut p = WorldParams::new(seed);
    p.era = era;
    Sim::new(&p).expect("sim")
}

fn live_sects(w: &World) -> Vec<&Power> {
    w.powers.iter().filter(|p| p.fallen.is_none() && matches!(p.kind, PowerKind::Sect(_))).collect()
}

fn realm(p: &Person) -> Option<u8> {
    match p.path {
        PathState::Cultivator { realm, .. } => Some(realm),
        _ => None,
    }
}

#[test]
fn gdd_4_6_middle_era_counts() {
    let w = built(1, Era::Middle);
    // 中古：宗門 10；有名之人 NOTABLE_CAP × 0.75（中尺寸 4000 → 3000）。
    assert_eq!(live_sects(&w).len(), 10);
    assert_eq!(w.persons.len(), 3000);
    assert!(w.persons.iter().all(|p| p.alive()));
    // 勢力（powers）只有宗門；王朝另存於 `World::dynasty`（3.7，開天時建立 4–6 個），寺院、書院尚未實作。
    assert_eq!(w.powers.len(), 10);
    assert!((4..=6).contains(&w.dynasty.list.len()));
    // 每個宗門有掌門且在 SECT_SIZE 規模內補滿。
    for s in live_sects(&w) {
        assert!(s.leader.is_some());
        assert!(s.members.len() >= SECT_SIZE_BASE as usize, "{} 成員 {}", s.name, s.members.len());
        let leader = &w.persons[s.leader.unwrap() as usize];
        assert!(realm(leader).unwrap() >= SECT_FOUND_MIN_REALM);
        for &m in &s.members {
            assert_eq!(w.persons[m as usize].power, Some(s.id));
        }
    }
    // 中古沒有遊方僧、散儒以外的限定：身分只會是 M2 允許的。
    assert!(w
        .persons
        .iter()
        .all(|p| matches!(p.role, Role::Cultivator | Role::Demonic | Role::Monk | Role::Scholar | Role::Commoner | Role::Ruler | Role::GeneralCandidate)),
        "{:?}",
        w.persons.iter().map(|p| p.role).find(|r| !matches!(r, Role::Cultivator | Role::Demonic | Role::Monk | Role::Scholar | Role::Commoner | Role::Ruler | Role::GeneralCandidate))
    );
}

#[test]
fn gdd_4_6_ancient_and_decline_counts() {
    let a = built(2, Era::Ancient);
    // 上古：宗門 12；有名之人 4000 × 0.3 = 1200，另有遊方僧與散儒各 20。
    assert_eq!(live_sects(&a).len(), 12);
    assert_eq!(a.persons.len(), 1200 + 40);
    assert!(a.persons.iter().filter(|p| p.role == Role::Monk).count() >= 20);
    assert!(a.persons.iter().filter(|p| p.role == Role::Scholar).count() >= 20);
    let d = built(2, Era::Decline);
    // 末法：宗門 5；有名之人 3000。
    assert_eq!(live_sects(&d).len(), 5);
    assert_eq!(d.persons.len(), 3000);
}

#[test]
fn gdd_4_6_sect_seats_follow_site_rule() {
    let w = built(3, Era::Middle);
    let sects = live_sects(&w);
    // 山門互不相同、相距 ≥ 2 步（3.5 選址）；有節點者不重複佔用同一個洞天福地。
    let mut nodes = BTreeSet::new();
    for s in &sects {
        let PowerKind::Sect(k) = &s.kind;
        if let Some(n) = k.seat_node {
            assert!(nodes.insert(n), "洞天福地被重複佔用");
            assert_eq!(s.seat_cell, w.geo.nodes[n as usize].cell);
        }
        for o in &sects {
            if o.id != s.id {
                assert!(!dao::provinces_within(&w, s.seat_province, 1).contains(&o.seat_province));
            }
        }
    }
    // 掌門與成員在山門。
    for s in &sects {
        let l = &w.persons[s.leader.unwrap() as usize];
        assert_eq!((l.province, l.cell), (s.seat_province, s.seat_cell));
    }
}

#[test]
fn gdd_4_6_souls_and_ages() {
    let w = built(4, Era::Middle);
    assert_eq!(w.souls[0].id, HEAVEN_SOUL);
    for p in &w.persons {
        let s = &w.souls[p.soul as usize];
        assert_eq!(s.state, SoulState::Incarnate(p.id));
        assert_eq!(s.lives, vec![p.id]);
        assert_eq!(s.origin, SoulOrigin::Primordial);
        assert!((0.0..=GENESIS_MERIT_MAX).contains(&s.merit));
        assert!((0.0..=GENESIS_SIN_MAX).contains(&s.sin));
        assert!((0.0..=GENESIS_WISDOM_MAX).contains(&s.wisdom));
        assert!((s.affinity.iter().sum::<f32>() - 1.0).abs() < 1e-4);
        let cult = matches!(p.role, Role::Cultivator | Role::Demonic);
        if cult {
            assert!(s.sin <= GENESIS_SIN_MAX * GENESIS_SIN_CULTIVATOR_MULT + 1e-3);
            assert!(s.affinity[Teaching::Tao as usize] > 0.5 || p.role == Role::Demonic);
        }
        // 年齡 12..=壽元 × 0.6；born 為負。
        let age = p.age(0);
        assert!(p.born < 0);
        assert!(age >= GENESIS_AGE_MIN, "age {age}");
        assert!(age as f32 <= p.lifespan as f32 * GENESIS_AGE_MAX_LIFESPAN_RATIO + 1.0, "age {age} life {}", p.lifespan);
        if let PathState::Cultivator { realm, exp, .. } = p.path {
            assert!(exp >= 0.0 && exp < REALM_EXP[realm as usize]);
            assert!(p.lifespan as u32 >= REALM_LIFESPAN[realm as usize]);
        }
    }
}

#[test]
fn gdd_4_6_pyramid() {
    // 三個種子合併，降低抽樣雜訊：每高一境人數約 × 0.35。
    let mut c = [0u32; 9];
    for seed in [5, 6, 7] {
        let w = built(seed, Era::Middle);
        for p in &w.persons {
            if let Some(r) = realm(p) {
                c[r as usize] += 1;
            }
        }
    }
    assert!(c[0] > c[1] && c[1] > c[2] && c[2] > c[3], "{c:?}");
    for r in 0..3 {
        let ratio = c[r + 1] as f32 / c[r] as f32;
        assert!((0.2..0.55).contains(&ratio), "境界 {r}→{}：{ratio}（{c:?}）", r + 1);
    }
}

#[test]
fn gdd_4_6_threads_invariants() {
    let w = built(8, Era::Middle);
    let n = w.persons.len();
    let expect = (n as f32 * GENESIS_THREADS_PER_NOTABLE).round() as usize;
    assert_eq!(w.threads.len(), expect);
    // 全世界只有一個 Genesis 事件；開局宿緣不另寫 ThreadCreated。
    let genesis: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::Genesis).collect();
    assert_eq!(genesis.len(), 1);
    assert_eq!(genesis[0].importance, 1);
    assert!(!w.ledger.events.iter().any(|e| e.kind == EventKind::ThreadCreated));
    let mut kinds = std::collections::BTreeMap::new();
    for t in &w.threads {
        *kinds.entry(t.kind).or_insert(0usize) += 1;
        assert_eq!(t.created, (0, genesis[0].id));
        assert!(t.resolved.is_none());
        assert_ne!(t.debtor, t.creditor);
        assert_eq!(t.strength, thread::default_strength(t.kind));
        assert_eq!(thread::lives(&w, t), 0);
        // 雙方魂的 threads 都含此線。
        assert!(w.souls[t.debtor as usize].threads.contains(&t.id));
        assert!(w.souls[t.creditor as usize].threads.contains(&t.id));
        // 雙方距離 3 步內。
        let pa = w.persons[match w.souls[t.debtor as usize].state {
            SoulState::Incarnate(p) => p as usize,
            _ => panic!(),
        }]
        .province;
        let pb = w.persons[match w.souls[t.creditor as usize].state {
            SoulState::Incarnate(p) => p as usize,
            _ => panic!(),
        }]
        .province;
        assert!(dao::provinces_within(&w, pa, GENESIS_THREAD_RANGE_STEPS).contains(&pb));
    }
    // 種類只有殺業、恩義、情緣、師徒，且均分；不含怨。
    assert_eq!(kinds.len(), 4);
    assert!(!kinds.contains_key(&ThreadKind::Grudge));
    for (&k, &c) in &kinds {
        assert!(c == expect / 4 || c == expect / 4 + 1, "{k:?}: {c}");
    }
    // 反向：每個魂的 threads 都指向以它為一方的線，且沒有重複。
    for s in &w.souls {
        let mut seen = BTreeSet::new();
        for &tid in &s.threads {
            let t = &w.threads[tid as usize];
            assert!(t.debtor == s.id || t.creditor == s.id);
            assert!(seen.insert(tid));
        }
    }
    // 殺業線計入 kill_pairs。
    assert_eq!(w.kill_pairs.values().map(|&v| v as usize).sum::<usize>(), kinds[&ThreadKind::Feud]);
}

#[test]
fn gdd_8_3_titles_from_golden_core() {
    let w = built(9, Era::Middle);
    let mut with_title = 0;
    for p in &w.persons {
        let high = realm(p).is_some_and(|r| r >= SECT_FOUND_MIN_REALM);
        assert_eq!(p.title.is_some(), high, "{} 境界 {:?}", p.name, realm(p));
        if let Some(t) = &p.title {
            assert!(t.chars().count() >= 3, "{t}");
            with_title += 1;
        }
    }
    assert!(with_title > 50);
    // 掌門都有道號。
    for s in live_sects(&w) {
        assert!(w.persons[s.leader.unwrap() as usize].title.is_some());
    }
}

#[test]
fn gdd_8_3_title_updates_on_breakthrough_and_keeps_core() {
    let mut w = built(10, Era::Middle);
    // 找一名境界恰為金丹、有道號者，手動令其突破（走 dao 的 advance 路徑：強制突破）。
    let pid = w
        .persons
        .iter()
        .find(|p| realm(p) == Some(2) && p.alive())
        .map(|p| p.id)
        .expect("有金丹");
    let before = w.persons[pid as usize].title.clone().unwrap();
    if let PathState::Cultivator { realm, last_attempt, .. } = w.persons[pid as usize].path {
        w.persons[pid as usize].path = PathState::Cultivator { realm, exp: REALM_EXP[realm as usize], last_attempt };
    }
    w.tick = 100;
    w.persons[pid as usize].forced_breakthrough = Some(100);
    dao::monthly_forced(&mut w);
    if w.persons[pid as usize].alive() {
        assert_eq!(realm(&w.persons[pid as usize]), Some(3));
        let after = w.persons[pid as usize].title.clone().unwrap();
        assert_eq!(after.chars().take(2).collect::<String>(), before.chars().take(2).collect::<String>());
    }
}

#[test]
fn gdd_4_6_high_realm_guarantee() {
    for era in [Era::Middle, Era::Decline] {
        let sim = full(11, era);
        let w = sim.world();
        let n = w.persons.iter().filter(|p| p.alive() && realm(p).is_some_and(|r| r >= GENESIS_HIGH_REALM_MIN_REALM)).count();
        assert!(n >= GENESIS_HIGH_REALM_COUNT as usize, "{era:?}: {n}");
        // 其道號為渡劫期尊稱之一（金丹起皆有）。
        for p in w.persons.iter().filter(|p| p.alive() && realm(p).is_some_and(|r| r >= 7)) {
            assert!(p.title.is_some());
        }
    }
}

#[test]
fn gdd_4_6_high_realm_guarantee_promotes_highest_first() {
    let mut w = built(12, Era::Middle);
    // 清掉所有渡劫以上，再保證：恰補到 2 名。
    for p in w.persons.iter_mut() {
        if let PathState::Cultivator { realm, .. } = &mut p.path {
            if *realm >= 7 {
                *realm = 6;
            }
        }
    }
    let top: Vec<PersonId> = {
        let mut v: Vec<(u8, PersonId)> =
            w.persons.iter().filter(|p| p.role == Role::Cultivator).filter_map(|p| realm(p).map(|r| (r, p.id))).collect();
        v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        v.iter().take(2).map(|x| x.1).collect()
    };
    assert_eq!(ensure_high_realm(&mut w), 2);
    for pid in top {
        assert_eq!(realm(&w.persons[pid as usize]), Some(7));
    }
    assert_eq!(ensure_high_realm(&mut w), 0);
    // 上古不保證。
    let mut a = built(12, Era::Ancient);
    for p in a.persons.iter_mut() {
        if let PathState::Cultivator { realm, .. } = &mut p.path {
            *realm = 0;
        }
    }
    assert_eq!(ensure_high_realm(&mut a), 0);
}

#[test]
fn gdd_4_6_prerun_and_opening_guarantee() {
    let mut sim = full(13, Era::Middle);
    {
        let w = sim.world();
        assert_eq!(w.tick, GENESIS_PRERUN_TICK_END);
        assert!(!w.prerun);
        // 預跑事件不輸出。
    }
    assert!(sim.drain_events().is_empty());
    let (pid, sect, seat) = {
        let w = sim.world();
        let forced: Vec<&Person> = w.persons.iter().filter(|p| p.forced_breakthrough.is_some()).collect();
        assert_eq!(forced.len(), 1);
        let p = forced[0];
        assert_eq!(p.forced_breakthrough, Some(GENESIS_FORCED_BT_TICK));
        let PathState::Cultivator { realm, exp, last_attempt } = p.path else { panic!() };
        assert_eq!((realm, exp, last_attempt), (1, REALM_EXP[1], None));
        // 聲望最高的宗門的成員，或其山門 2 州內的散修。
        let top_prestige = live_sects(w).into_iter().map(dao::effective_prestige).fold(f32::MIN, f32::max);
        let sect = top_sect(w).unwrap();
        assert_eq!(dao::effective_prestige(&w.powers[sect as usize]), top_prestige);
        assert!(
            p.power == Some(sect)
                || (p.power.is_none()
                    && dao::provinces_within(w, w.powers[sect as usize].seat_province, 2).contains(&p.province))
        );
        let (pr, cell) = camera_start(w).unwrap();
        assert_eq!((pr, cell), (w.powers[sect as usize].seat_province, w.powers[sect as usize].seat_cell));
        (p.id, sect, (pr, cell))
    };
    let _ = (sect, seat);
    // 接手後第 3 個月：強制突破成功，築基 → 金丹，取得道號。
    for _ in 0..3 {
        sim.step();
    }
    let p = &sim.world().persons[pid as usize];
    assert_eq!(sim.world().tick, GENESIS_FORCED_BT_TICK);
    assert!(p.forced_breakthrough.is_none());
    assert!(p.alive());
    assert_eq!(realm(p), Some(2));
    assert!(p.title.is_some());
}

#[test]
fn gdd_4_6_opening_guarantee_creates_new_soul_when_none_qualified() {
    let mut w = built(6, Era::Middle);
    for p in w.persons.iter_mut() {
        if let PathState::Cultivator { realm, .. } = &mut p.path {
            if *realm == 1 {
                *realm = 0;
            }
        }
    }
    let n_before = w.persons.len();
    let pid = ensure_forced_breakthrough(&mut w).unwrap();
    assert_eq!(w.persons.len(), n_before + 1);
    assert_eq!(pid as usize, n_before);
    let p = &w.persons[pid as usize];
    assert_eq!(p.age(w.tick), GENESIS_AGE_MIN);
    assert!(p.power.is_some());
    assert_eq!(realm(p), Some(1));
    assert_eq!(p.forced_breakthrough, Some(GENESIS_FORCED_BT_TICK));
}

#[test]
fn gdd_6_5_genesis_is_deterministic() {
    let a = full(7, Era::Middle);
    let b = full(7, Era::Middle);
    assert_eq!(a.world().persons, b.world().persons);
    assert_eq!(a.world().souls, b.world().souls);
    assert_eq!(a.world().threads, b.world().threads);
    assert_eq!(a.world().powers, b.world().powers);
    assert_eq!(a.world().ledger, b.world().ledger);
    // 不同種子不同。
    let c = full(1, Era::Middle);
    assert_ne!(a.world().persons, c.world().persons);
}
