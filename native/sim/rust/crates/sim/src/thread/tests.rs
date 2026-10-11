//! U7 單元測試：宿緣建立、相遇、了結、冤冤相報、預兆、因果帳修剪、天書五條。

use super::*;
use crate::book;
use crate::soul::{ensure_heaven, spawn_new_life};
use crate::types::{Geography, Province, Terrain, Tick, WorldParams};

fn province(id: u16, neighbors: Vec<u16>) -> Province {
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

/// 一條直線上的 4 個州郡：i 與 i±1 為陸鄰。tick = 40 年。
fn line_world(seed: u64) -> World {
    let n = 4usize;
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
    let mut w = World::empty(WorldParams::new(seed), geo(), provs);
    w.tick = 12 * 40;
    ensure_heaven(&mut w);
    w
}

/// 在州郡 `prov` 建立一位 40 歲的修士。
fn adult(w: &mut World, prov: u16, realm: u8) -> (SoulId, PersonId) {
    let (s, p) = spawn_new_life(w, SoulOrigin::New, prov);
    let pr = &mut w.persons[p as usize];
    pr.born = 0;
    pr.role = Role::Cultivator;
    pr.path = PathState::Cultivator { realm, exp: 0.0, last_attempt: None };
    pr.attrs = Attrs { talent: 5.0, virtue: 5.0, ..Default::default() };
    (s, p)
}

/// 模擬轉世 n 次（只增加 `lives`，不經輪迴系統）。
fn reborn(w: &mut World, sid: SoulId, n: usize) {
    let p = w.souls[sid as usize].lives[0];
    for _ in 0..n {
        w.souls[sid as usize].lives.push(p);
    }
}

fn events_of(w: &World, kind: EventKind) -> Vec<&Event> {
    w.ledger.events.iter().filter(|e| e.kind == kind).collect()
}

fn kill_simple(w: &mut World, killer: PersonId, victim: PersonId) -> Option<ThreadId> {
    // 只用於測試殺業線：寫入死亡事件但不走中陰。
    let mut e = NewEvent::new(EventKind::Died, 2, EventArgs::Death { cause: DeathCause::Killed, age: 40 });
    e.persons = smallvec![victim];
    e.souls = smallvec![w.persons[victim as usize].soul];
    ledger::record(w, e);
    on_kill(w, killer, victim)
}

// ── 建立 ───────────────────────────────────────────────────

#[test]
fn gdd_3_3_kill_creates_feud_with_realm_bonus_and_registers_in_both_souls() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 0);
    let (sb, pb) = adult(&mut w, 0, 3);
    let tid = kill_simple(&mut w, pa, pb).expect("建立殺業線");
    let t = &w.threads[tid as usize];
    assert_eq!((t.kind, t.debtor, t.creditor), (ThreadKind::Feud, sa, sb));
    assert_eq!(t.strength, 40.0 + 3.0 * 5.0);
    assert_eq!(t.lives_base, (0, 0));
    assert!(w.souls[sa as usize].threads.contains(&tid));
    assert!(w.souls[sb as usize].threads.contains(&tid));
    assert_eq!(w.kill_pairs.get(&(sa.min(sb), sa.max(sb))), Some(&1));
    // 建線事件：重要度 2，因為死亡事件，t.created 指向它。
    let e = &w.ledger.events[t.created.1 as usize];
    assert_eq!((e.kind, e.importance), (EventKind::ThreadCreated, 2));
    assert_eq!(e.causes.len(), 1);
    assert_eq!(w.ledger.events[e.causes[0] as usize].kind, EventKind::Died);
}

#[test]
fn gdd_3_3_ruler_victim_bonus_and_evil_victim_exempt() {
    let mut w = line_world(1);
    let (_, pa) = adult(&mut w, 0, 0);
    let (_, pb) = adult(&mut w, 0, 0);
    w.persons[pb as usize].role = Role::Ruler;
    w.persons[pb as usize].path = PathState::Mortal;
    let tid = kill_simple(&mut w, pa, pb).unwrap();
    assert_eq!(w.threads[tid as usize].strength, 60.0);

    let (_, pc) = adult(&mut w, 0, 0);
    let (sd, pd) = adult(&mut w, 0, 0);
    w.souls[sd as usize].sin = 100.0; // S > G + 50
    assert_eq!(kill_simple(&mut w, pc, pd), None);
    assert!(w.threads.len() == 1);
}

#[test]
fn gdd_3_1_kill_karma() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 0);
    let (sb, pb) = adult(&mut w, 0, 0);
    kill_karma(&mut w, pa, pb);
    assert_eq!(w.souls[sa as usize].sin, 10.0);
    w.souls[sb as usize].sin = 5.0; // 淨業力 < 0
    kill_karma(&mut w, pa, pb);
    assert!((w.souls[sa as usize].sin - 13.0).abs() < 1e-4);
    w.souls[sb as usize].sin = 200.0; // 惡人
    kill_karma(&mut w, pa, pb);
    assert_eq!(w.souls[sa as usize].merit, 5.0);
}

#[test]
fn create_merges_same_open_pair_and_caps() {
    let mut w = line_world(1);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, 0, 0);
    let t1 = create(&mut w, ThreadKind::Love, sa, sb, 30.0, false, None);
    let t2 = create(&mut w, ThreadKind::Love, sb, sa, 30.0, false, None); // 情緣不分方向
    assert_eq!(t1, t2);
    assert_eq!(w.threads[t1 as usize].strength, 60.0);
    create(&mut w, ThreadKind::Love, sa, sb, 90.0, false, None);
    assert_eq!(w.threads[t1 as usize].strength, 100.0);
    // 師徒有方向：相反方向是另一條線。
    let m1 = create(&mut w, ThreadKind::Mentor, sa, sb, 20.0, false, None);
    let m2 = create(&mut w, ThreadKind::Mentor, sb, sa, 20.0, false, None);
    assert_ne!(m1, m2);
    // 司命所牽：重要度 5。
    let wv = create(&mut w, ThreadKind::Grace, sa, sb, 50.0, true, None);
    assert_eq!(w.ledger.events[w.threads[wv as usize].created.1 as usize].importance, 5);
}

// ── 復仇與冤冤相報 ───────────────────────────────────────────

#[test]
fn gdd_3_3_revenge_resolves_without_new_thread() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 0);
    let (sb, pb) = adult(&mut w, 0, 0);
    let tid = kill_simple(&mut w, pa, pb).unwrap(); // A 殺 B：A 欠 B
    w.souls[sa as usize].sin = 30.0;
    let n_threads = w.threads.len();
    assert_eq!(kill_simple(&mut w, pb, pa), None); // B 復仇殺 A
    assert_eq!(w.threads.len(), n_threads, "復仇不建立新線");
    let t = &w.threads[tid as usize];
    assert!(t.resolved.is_some());
    assert!((w.souls[sa as usize].sin - (30.0 - 40.0 * 0.5)).abs() < 1e-4 || w.souls[sa as usize].sin == 10.0);
    let _ = sb;
    let ev = &w.ledger.events[t.resolved.unwrap().1 as usize];
    assert_eq!((ev.kind, ev.importance), (EventKind::ThreadResolved, 4));
    assert!(ev.causes.contains(&t.created.1));
    assert!(!book::achieved(&w, BookEntryKind::ThreeLivesFeud));
}

#[test]
fn gdd_3_3_escalation_makes_independent_new_thread_and_third_gives_book_entry() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 0);
    let (sb, pb) = adult(&mut w, 0, 0);
    let t1 = kill_simple(&mut w, pa, pb).unwrap();
    assert_eq!(w.threads[t1 as usize].strength, 40.0);
    // 欠債方 A 再殺債主 B。
    let t2 = kill_simple(&mut w, pa, pb).expect("另建新線");
    assert_ne!(t1, t2);
    assert_eq!(w.threads[t1 as usize].strength, 70.0, "舊線 +30");
    assert!(w.threads[t1 as usize].resolved.is_none());
    assert_eq!(w.threads[t2 as usize].strength, 40.0);
    assert_eq!((w.threads[t2 as usize].debtor, w.threads[t2 as usize].creditor), (sa, sb));
    let esc = events_of(&w, EventKind::FeudEscalated);
    assert_eq!(esc.len(), 1);
    assert_eq!(esc[0].importance, 4);
    assert!(esc[0].causes.contains(&w.threads[t1 as usize].created.1));
    // 新線的因是 FeudEscalated。
    assert_eq!(w.ledger.events[w.threads[t2 as usize].created.1 as usize].causes[0], esc[0].id);
    assert!(!book::achieved(&w, BookEntryKind::EndlessVengeance));
    // 第三條：累計 3 條 → 冤冤相報。
    kill_simple(&mut w, pa, pb).unwrap();
    assert_eq!(w.kill_pairs[&(sa.min(sb), sa.max(sb))], 3);
    assert!(book::achieved(&w, BookEntryKind::EndlessVengeance));
    assert_eq!(w.book.entries[&BookEntryKind::EndlessVengeance].count, 1);
    // 第四條不再重複觸發。
    kill_simple(&mut w, pa, pb).unwrap();
    assert_eq!(w.book.entries[&BookEntryKind::EndlessVengeance].count, 1);
    // BookEntry 事件：重要度 4，因為觸發的 ThreadCreated。
    let be = events_of(&w, EventKind::BookEntry);
    assert_eq!(be.len(), 1);
    assert_eq!(be[0].importance, 4);
    assert_eq!(w.ledger.events[be[0].causes[0] as usize].kind, EventKind::ThreadCreated);
}

#[test]
fn endless_vengeance_counts_both_directions() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 0);
    let (sb, pb) = adult(&mut w, 0, 0);
    create_feud_raw(&mut w, sa, sb);
    create_feud_raw(&mut w, sb, sa);
    assert!(!book::achieved(&w, BookEntryKind::EndlessVengeance));
    create_feud_raw(&mut w, sa, sb);
    assert!(book::achieved(&w, BookEntryKind::EndlessVengeance));
    let _ = (pa, pb);
}

fn create_feud_raw(w: &mut World, d: SoulId, c: SoulId) -> ThreadId {
    insert_thread(w, ThreadKind::Feud, d, c, 40.0, false, None)
}

#[test]
fn gdd_4_5_three_lives_feud_on_resolve_with_lives_2() {
    let mut w = line_world(1);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, 0, 0);
    let t = create_feud_raw(&mut w, sa, sb);
    reborn(&mut w, sa, 1);
    resolve(&mut w, t, Resolution::Revenge, None).unwrap();
    assert!(!book::achieved(&w, BookEntryKind::ThreeLivesFeud), "lives = 1 不算");

    let t = create_feud_raw(&mut w, sa, sb);
    // lives_base 以建線時為準：再轉世 2 次。
    reborn(&mut w, sa, 2);
    let eid = resolve(&mut w, t, Resolution::Atone, None).unwrap();
    assert_eq!(w.ledger.events[eid as usize].importance, 5);
    assert!(book::achieved(&w, BookEntryKind::ThreeLivesFeud));
}

#[test]
fn gdd_3_3_atone_clears_full_strength() {
    let mut w = line_world(1);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, 0, 0);
    w.souls[sa as usize].sin = 100.0;
    let t = create_feud_raw(&mut w, sa, sb);
    resolve(&mut w, t, Resolution::Atone, None).unwrap();
    assert_eq!(w.souls[sa as usize].sin, 60.0);
    assert_eq!(resolve(&mut w, t, Resolution::Atone, None), None, "不重複了結");
}

// ── 相遇 ───────────────────────────────────────────────────

/// 兩位修士，愛情線強度 50，同州（prov_b = 0）或相隔。不婚、不同世 → 每次相遇強度 −10。
fn love_world(seed: u64, prov_b: u16) -> (World, ThreadId) {
    let mut w = line_world(seed);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, prov_b, 0);
    let t = create(&mut w, ThreadKind::Love, sa, sb, 50.0, false, None);
    (w, t)
}

#[test]
fn gdd_3_3_meeting_probability_and_range() {
    let trials = 3000u32;
    let count = |prov_b: u16| -> u32 {
        (0..trials)
            .filter(|&s| {
                let (mut w, t) = love_world(s as u64 + 100, prov_b);
                yearly(&mut w);
                w.threads[t as usize].strength < 50.0
            })
            .count() as u32
    };
    // 同州：0.15 × 50/50 × 2 = 0.3。
    let same = count(0) as f32 / trials as f32;
    assert!((same - 0.30).abs() < 0.04, "同州 {same}");
    // 陸鄰：0.15。
    let near = count(1) as f32 / trials as f32;
    assert!((near - 0.15).abs() < 0.03, "陸鄰 {near}");
    // 距離 2：不相遇。
    assert_eq!(count(2), 0);
}

#[test]
fn heaven_and_child_and_spouses_never_meet() {
    // 天道。
    let mut w = line_world(3);
    let (sa, _) = adult(&mut w, 0, 0);
    let t = create(&mut w, ThreadKind::Love, sa, HEAVEN_SOUL, 100.0, false, None);
    for _ in 0..50 {
        yearly(&mut w);
    }
    assert_eq!(w.threads[t as usize].strength, 100.0);
    // 幼年者。
    let (mut w, t) = love_world(5, 0);
    let pb = match w.souls[w.threads[t as usize].creditor as usize].state {
        SoulState::Incarnate(p) => p,
        _ => unreachable!(),
    };
    w.persons[pb as usize].role = Role::Child;
    w.persons[pb as usize].path = PathState::Child;
    for _ in 0..50 {
        yearly(&mut w);
    }
    assert_eq!(w.threads[t as usize].strength, 50.0);
    // 配偶。
    let (mut w, t) = love_world(6, 0);
    let (a, b) = match (&w.souls[w.threads[t as usize].debtor as usize].state, &w.souls[w.threads[t as usize].creditor as usize].state) {
        (SoulState::Incarnate(a), SoulState::Incarnate(b)) => (*a, *b),
        _ => unreachable!(),
    };
    w.persons[a as usize].spouse = Some(b);
    w.persons[b as usize].spouse = Some(a);
    for _ in 0..50 {
        yearly(&mut w);
    }
    assert_eq!(w.threads[t as usize].strength, 50.0);
}

#[test]
fn at_most_one_meeting_per_soul_per_year() {
    // A 有兩條強度 100 的情緣線（同州，機率 0.6）：同一年不會兩條都被處理。
    for seed in 0..400u64 {
        let mut w = line_world(seed);
        let (sa, _) = adult(&mut w, 0, 0);
        let (sb, _) = adult(&mut w, 0, 0);
        let (sc, _) = adult(&mut w, 0, 0);
        let t1 = create(&mut w, ThreadKind::Love, sa, sb, 100.0, false, None);
        let t2 = create(&mut w, ThreadKind::Love, sa, sc, 90.0, false, None);
        yearly(&mut w);
        let hit = |t: ThreadId| w.threads[t as usize].strength < w.threads[t as usize].strength.max(f32::MAX);
        let _ = hit;
        let dropped = [t1, t2].iter().filter(|&&t| w.threads[t as usize].strength < [100.0, 90.0][(t == t2) as usize]).count();
        assert!(dropped <= 1, "seed {seed}");
    }
}

#[test]
fn gdd_3_3_grace_repay_resolves_and_grants_cultivation() {
    let mut found = false;
    for seed in 0..200u64 {
        let mut w = line_world(seed);
        let (sa, _) = adult(&mut w, 0, 0);
        let (sb, pb) = adult(&mut w, 0, 0);
        w.persons[pb as usize].path = PathState::Cultivator { realm: 1, exp: 10.0, last_attempt: None };
        let t = create(&mut w, ThreadKind::Grace, sa, sb, 100.0, false, None);
        yearly(&mut w);
        if let Some((_, eid)) = w.threads[t as usize].resolved {
            found = true;
            assert_eq!(w.souls[sa as usize].merit, 20.0);
            assert_eq!(w.souls[sb as usize].merit, 20.0);
            let PathState::Cultivator { exp, .. } = w.persons[pb as usize].path else { panic!() };
            assert!((exp - 11.0).abs() < 1e-4);
            assert_eq!(w.ledger.events[eid as usize].kind, EventKind::ThreadResolved);
            break;
        }
    }
    assert!(found);
}

#[test]
fn gdd_3_3_clash_stronger_creditor_takes_revenge() {
    // 債主境界 3、欠債方境界 0：債主勝率約 97%；找到爆發衝突的種子。
    let mut revenge = 0;
    let mut checked = 0;
    for seed in 0..400u64 {
        let mut w = line_world(seed);
        let (sa, pa) = adult(&mut w, 0, 0);
        let (sb, pb) = adult(&mut w, 0, 3);
        let t = create(&mut w, ThreadKind::Feud, sa, sb, 100.0, false, None);
        yearly(&mut w);
        let dead_a = w.persons[pa as usize].died.is_some();
        let dead_b = w.persons[pb as usize].died.is_some();
        if !dead_a && !dead_b {
            continue;
        }
        checked += 1;
        assert!(dead_a ^ dead_b);
        if dead_a {
            revenge += 1;
            assert!(w.threads[t as usize].resolved.is_some());
            assert_eq!(w.threads.len(), 1, "復仇不建新線");
            assert!(matches!(w.souls[sa as usize].state, SoulState::Bardo { .. }));
            // 兇手 B 計業：A 淨業力 ≥ 0 → S += 10。
            assert!(w.souls[sb as usize].sin >= 10.0 - 1e-4);
        } else {
            // 欠債方勝：冤冤相報。
            assert!(w.threads[t as usize].resolved.is_none());
            assert_eq!(w.threads.len(), 2);
            assert_eq!(w.threads[t as usize].strength, 100.0);
            assert_eq!(events_of(&w, EventKind::FeudEscalated).len(), 1);
        }
    }
    assert!(checked > 20, "衝突次數 {checked}");
    assert!(revenge as f32 / checked as f32 > 0.85, "{revenge}/{checked}");
}

#[test]
fn gdd_3_3_clash_debtor_win_escalates() {
    let mut escalated = 0;
    for seed in 0..800u64 {
        let mut w = line_world(seed);
        let (sa, pa) = adult(&mut w, 0, 3);
        let (sb, pb) = adult(&mut w, 0, 0);
        let t = create(&mut w, ThreadKind::Feud, sa, sb, 50.0, false, None);
        yearly(&mut w);
        if w.persons[pb as usize].died.is_some() {
            escalated += 1;
            assert!(w.persons[pa as usize].died.is_none());
            assert_eq!(w.threads[t as usize].strength, 80.0);
            let new = &w.threads[1];
            assert_eq!((new.debtor, new.creditor), (sa, sb));
            assert_eq!(new.strength, 40.0);
            assert_eq!(w.kill_pairs[&(sa.min(sb), sa.max(sb))], 2);
            break;
        }
    }
    assert!(escalated > 0);
}

#[test]
fn gdd_3_3_atone_rescue_by_debtor() {
    let mut rescued = 0;
    for seed in 0..300u64 {
        let mut w = line_world(seed);
        let (sa, _) = adult(&mut w, 0, 0);
        let (sb, pb) = adult(&mut w, 0, 0);
        let SoulState::Incarnate(pa) = w.souls[sa as usize].state else { unreachable!() };
        w.persons[pa as usize].attrs.virtue = 10.0; // 欠債方德行 → 0.3
        let _ = pb;
        w.souls[sa as usize].sin = 50.0;
        let t = create_feud_raw(&mut w, sa, sb);
        // 債主 B 遇險。
        if on_peril(&mut w, pb, None).is_some() {
            rescued += 1;
            assert!(w.threads[t as usize].resolved.is_some());
            assert_eq!(w.souls[sa as usize].sin, 10.0); // 50 − 40
        }
    }
    assert!((rescued as f32 / 300.0 - 0.3).abs() < 0.08, "{rescued}");
}

// ── 情緣 ───────────────────────────────────────────────────

#[test]
fn gdd_3_3_marriage_creates_then_resolves_after_rebirth_reunited_love() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 0);
    let (sb, pb) = adult(&mut w, 0, 0);
    let t = on_marriage(&mut w, pa, pb, None).expect("建立情緣線");
    assert_eq!(w.threads[t as usize].kind, ThreadKind::Love);
    // 同一世再婚配：累加，不了結。
    assert_eq!(on_marriage(&mut w, pa, pb, None), Some(t));
    assert_eq!(w.threads[t as usize].strength, 60.0);
    // 轉世 1 次後再結：了結（再續前緣），但 lives = 1 不算天書。
    reborn(&mut w, sa, 1);
    assert_eq!(on_marriage(&mut w, pa, pb, None), None);
    assert!(w.threads[t as usize].resolved.is_some());
    assert!(!book::achieved(&w, BookEntryKind::ReunitedLove));
    assert_eq!(w.souls[sa as usize].merit, 60.0 * 0.2);
    assert_eq!(w.souls[sb as usize].merit, 60.0 * 0.2);

    // lives ≥ 2 了結 → 再續前緣。
    let t2 = create(&mut w, ThreadKind::Love, sa, sb, 30.0, false, None);
    assert_ne!(t, t2);
    reborn(&mut w, sb, 2);
    on_marriage(&mut w, pa, pb, None);
    assert!(w.threads[t2 as usize].resolved.is_some());
    assert!(book::achieved(&w, BookEntryKind::ReunitedLove));
}

#[test]
fn gdd_3_3_love_meeting_reunites_after_rebirth_else_fades() {
    // lives ≥ 1、皆未婚、年齡相近：相遇後有 60% 結為夫妻並了結。
    let mut reunited = 0;
    for seed in 0..300u64 {
        let mut w = line_world(seed);
        let (sa, pa) = adult(&mut w, 0, 0);
        let (sb, pb) = adult(&mut w, 0, 0);
        let t = create(&mut w, ThreadKind::Love, sa, sb, 100.0, false, None);
        reborn(&mut w, sa, 1);
        yearly(&mut w);
        if w.threads[t as usize].resolved.is_some() {
            reunited += 1;
            assert_eq!(w.persons[pa as usize].spouse, Some(pb));
            assert_eq!(w.persons[pb as usize].spouse, Some(pa));
            assert_eq!(events_of(&w, EventKind::Married).len(), 1);
        }
    }
    // 相遇 0.6 × 結合 0.6 = 0.36。
    assert!((reunited as f32 / 300.0 - 0.36).abs() < 0.08, "{reunited}");
}

#[test]
fn gdd_3_3_love_fades_at_zero_not_resolved() {
    let mut w = line_world(1);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, 0, 0);
    let t = create(&mut w, ThreadKind::Love, sa, sb, 10.0, false, None);
    for _ in 0..200 {
        yearly(&mut w);
        if w.threads[t as usize].strength == 0.0 {
            break;
        }
    }
    let th = &w.threads[t as usize];
    assert_eq!(th.strength, 0.0);
    assert!(th.resolved.is_none());
    assert!(!is_open(th));
    let f = events_of(&w, EventKind::ThreadFaded);
    assert_eq!((f.len(), f[0].importance), (1, 1));
    assert!(events_of(&w, EventKind::ThreadResolved).is_empty());
}

// ── 師徒 ───────────────────────────────────────────────────

#[test]
fn gdd_3_3_mentor_records_master_best_and_past_and_present() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 0, 2);
    let (sb, _pb) = adult(&mut w, 0, 4);
    // A 拜 B 為師。
    let t1 = on_apprentice(&mut w, sa, sb, None);
    assert_eq!(w.threads[t1 as usize].master_best.unwrap().realm, Some(4));
    assert_eq!(w.ledger.events[w.threads[t1 as usize].created.1 as usize].importance, 1);
    assert!(!book::achieved(&w, BookEntryKind::PastAndPresent));
    // 同一世再結：累加。
    assert_eq!(on_apprentice(&mut w, sa, sb, None), t1);
    assert_eq!(w.threads[t1 as usize].strength, 40.0);
    // 昔日弟子今為人師：B 拜 A，方向相反的新線。
    let t2 = on_apprentice(&mut w, sb, sa, None);
    assert_ne!(t1, t2);
    assert!(w.threads[t1 as usize].resolved.is_none(), "相反方向的舊線並存");
    assert!(book::achieved(&w, BookEntryKind::PastAndPresent));
    let _ = pa;
}

#[test]
fn gdd_3_3_mentor_same_direction_after_rebirth_resolves_and_recreates() {
    let mut w = line_world(1);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, 0, 4);
    let t1 = on_apprentice(&mut w, sa, sb, None);
    reborn(&mut w, sa, 1);
    let t2 = on_apprentice(&mut w, sa, sb, None);
    assert_ne!(t1, t2);
    assert!(w.threads[t1 as usize].resolved.is_some());
    assert!(w.threads[t2 as usize].resolved.is_none());
    assert!(!book::achieved(&w, BookEntryKind::PastAndPresent), "同向不算前世今生");
}

#[test]
fn gdd_3_3_mentor_resolves_when_disciple_surpasses_master_best() {
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 3, 0);
    let (sb, _) = adult(&mut w, 0, 2);
    let t = on_apprentice(&mut w, sa, sb, None);
    // 同世：不了結。
    w.persons[pa as usize].path = PathState::Cultivator { realm: 5, exp: 0.0, last_attempt: None };
    yearly(&mut w);
    assert!(w.threads[t as usize].resolved.is_none());
    // 轉世後（lives ≥ 1）今生成就超過 master_best（金丹）。
    reborn(&mut w, sa, 1);
    w.persons[pa as usize].path = PathState::Cultivator { realm: 2, exp: 0.0, last_attempt: None };
    yearly(&mut w);
    assert!(w.threads[t as usize].resolved.is_none(), "尚未超過");
    w.persons[pa as usize].path = PathState::Cultivator { realm: 3, exp: 0.0, last_attempt: None };
    yearly(&mut w);
    assert!(w.threads[t as usize].resolved.is_some());
}

#[test]
fn gdd_3_3_mentor_surpass_compares_same_axis_only() {
    // 師父是修士（只有境界）。徒弟轉世為僧（果位 0）：不同軸，不算超越。
    let mut w = line_world(1);
    let (sa, pa) = adult(&mut w, 3, 0);
    let (sb, _) = adult(&mut w, 0, 2);
    let t = on_apprentice(&mut w, sa, sb, None);
    reborn(&mut w, sa, 1);
    w.persons[pa as usize].role = Role::Monk;
    w.persons[pa as usize].path = PathState::Monk { fruit: 0, practice: 0.0 };
    yearly(&mut w);
    assert!(w.threads[t as usize].resolved.is_none(), "果位 Some(0) 對師父無果位不算超越");
    // 轉世為儒士且三不朽 > 0：同理。
    w.persons[pa as usize].role = Role::Scholar;
    w.persons[pa as usize].path = PathState::Scholar { virtue: 5.0, deed: 0.0, word: 0.0 };
    yearly(&mut w);
    assert!(w.threads[t as usize].resolved.is_none());
}

#[test]
fn gdd_3_3_mentor_meeting_joins_stronger_as_master() {
    let mut joined = 0;
    for seed in 0..300u64 {
        let mut w = line_world(seed);
        let (sa, pa) = adult(&mut w, 0, 0);
        let (sb, pb) = adult(&mut w, 0, 3);
        // 債主（師父）是較弱者 A，欠債方 B 較強：戰力較高者為師 → B 為師。
        create(&mut w, ThreadKind::Mentor, sb, sa, 100.0, false, None);
        yearly(&mut w);
        if w.persons[pa as usize].master.is_some() {
            joined += 1;
            assert_eq!(w.persons[pa as usize].master, Some(pb));
            assert_eq!(w.persons[pb as usize].disciples, vec![pa]);
            assert_eq!(w.souls[sb as usize].merit, 2.0);
            // 新線方向：A（徒）欠 B（師）。
            let new = w.threads.last().unwrap();
            assert_eq!((new.debtor, new.creditor, new.kind), (sa, sb, ThreadKind::Mentor));
        }
    }
    // 相遇 0.6 × 拜師 0.7 = 0.42。
    assert!((joined as f32 / 300.0 - 0.42).abs() < 0.08, "{joined}");
}

// ── 預兆 ───────────────────────────────────────────────────

#[test]
fn gdd_3_3_omen_once_per_life_within_range_and_strength() {
    let mut w = line_world(1);
    let (sa, _) = adult(&mut w, 0, 0);
    let (sb, _) = adult(&mut w, 2, 0); // 距離 2
    let t = create(&mut w, ThreadKind::Grace, sa, sb, 49.0, false, None);
    yearly(&mut w);
    assert!(events_of(&w, EventKind::ThreadOmen).is_empty(), "強度 < 50");
    w.threads[t as usize].strength = 50.0;
    yearly(&mut w);
    let omens = events_of(&w, EventKind::ThreadOmen);
    assert_eq!(omens.len(), 1);
    assert_eq!(omens[0].importance, 3);
    assert_eq!(omens[0].causes.as_slice(), &[w.threads[t as usize].created.1]);
    assert_eq!(w.threads[t as usize].omen_at, Some(0));
    yearly(&mut w);
    assert_eq!(events_of(&w, EventKind::ThreadOmen).len(), 1, "每次轉世最多一次");
    // 轉世後可再次預兆（若未被相遇了結）。
    w.threads[t as usize].strength = 100.0;
    reborn(&mut w, sb, 0);
    w.threads[t as usize].lives_base.0 = 0;
    // 欠債方 sa 轉世：lives 變 1。
    reborn(&mut w, sa, 1);
    // 防止恩義被報恩了結：直接呼叫預兆。
    yearly::omens(&mut w);
    assert_eq!(events_of(&w, EventKind::ThreadOmen).len(), 2);
    assert_eq!(w.threads[t as usize].omen_at, Some(1));
    // 距離 3：不預兆。
    let (sc, _) = adult(&mut w, 3, 0);
    let (sd, _) = adult(&mut w, 0, 0);
    let t3 = create(&mut w, ThreadKind::Grace, sc, sd, 80.0, false, None);
    yearly::omens(&mut w);
    assert_eq!(w.threads[t3 as usize].omen_at, None);
}

// ── 天書：轉世大能 ───────────────────────────────────────────

fn breakthrough(w: &mut World, sid: SoulId, pid: PersonId, realm: u8) {
    let mut e = NewEvent::new(EventKind::Breakthrough, 3, EventArgs::Realm { realm });
    e.souls = smallvec![sid];
    e.persons = smallvec![pid];
    ledger::record(w, e);
}

#[test]
fn gdd_4_5_reincarnated_master_conditions() {
    // 前世最高金丹，今生 30 歲再達金丹。
    let mut w = line_world(1);
    let (sid, pid) = adult(&mut w, 0, 2);
    w.persons[pid as usize].born = w.tick as i32 - 12 * 30;
    w.souls[sid as usize].best_prev.realm = Some(2);
    breakthrough(&mut w, sid, pid, 1);
    assert!(!book::achieved(&w, BookEntryKind::ReincarnatedMaster), "尚未達到舊境界");
    breakthrough(&mut w, sid, pid, 2);
    assert!(book::achieved(&w, BookEntryKind::ReincarnatedMaster));
    let rec = &w.book.entries[&BookEntryKind::ReincarnatedMaster];
    assert_eq!((rec.count, rec.first_tick), (1, w.tick));
    assert_eq!(w.ledger.events[rec.first_event as usize].kind, EventKind::BookEntry);

    // 50 歲後才達到：不算。
    let mut w = line_world(1);
    w.tick = 12 * 80;
    let (sid, pid) = adult(&mut w, 0, 2);
    w.persons[pid as usize].born = w.tick as i32 - 12 * 50;
    w.souls[sid as usize].best_prev.realm = Some(2);
    breakthrough(&mut w, sid, pid, 2);
    assert!(!book::achieved(&w, BookEntryKind::ReincarnatedMaster));

    // 前世只到築基：不算。
    let mut w = line_world(1);
    let (sid, pid) = adult(&mut w, 0, 1);
    w.souls[sid as usize].best_prev.realm = Some(1);
    breakthrough(&mut w, sid, pid, 1);
    assert!(!book::achieved(&w, BookEntryKind::ReincarnatedMaster));

    // 渡過天劫本身不偵測（隨後的 Breakthrough 才偵測，避免重複計次；見 dao 的 reincarnated_master_not_double_counted 測試）。
    let mut w = line_world(1);
    let (sid, pid) = adult(&mut w, 0, 3);
    w.souls[sid as usize].best_prev.realm = Some(3);
    let mut e = NewEvent::new(
        EventKind::Tribulation,
        3,
        EventArgs::Tribulation { realm: 3, outcome: TribulationOutcome::Passed },
    );
    e.souls = smallvec![sid];
    e.persons = smallvec![pid];
    ledger::record(&mut w, e);
    assert!(!book::achieved(&w, BookEntryKind::ReincarnatedMaster));
}

// ── 因果帳 ───────────────────────────────────────────────────

fn ev(w: &mut World, imp: u8, causes: &[EventId], tick: Tick) -> EventId {
    w.tick = tick;
    let mut e = NewEvent::new(EventKind::Traveled, imp, EventArgs::None);
    e.causes = causes.iter().copied().collect();
    ledger::record(w, e)
}

#[test]
fn gdd_3_3_ledger_effects_index_and_ripple() {
    let mut w = line_world(1);
    let a = ev(&mut w, 2, &[], 1);
    let b = ev(&mut w, 2, &[a], 2);
    let c = ev(&mut w, 2, &[a, b, a], 3);
    assert_eq!(w.ledger.effects[a as usize].as_slice(), &[b, c]);
    assert_eq!(w.ledger.effects[b as usize].as_slice(), &[c]);
    assert_eq!(w.ledger.events[c as usize].causes.as_slice(), &[a, b], "去重排序");
    assert_eq!(ledger::take_new(&mut w).len(), 3);
    assert!(ledger::take_new(&mut w).is_empty());
    ev(&mut w, 1, &[], 4);
    assert_eq!(ledger::take_new(&mut w), vec![3]);
}

#[test]
fn gdd_3_3_prune_old_trivial_events_rewires_causes() {
    let mut w = line_world(1);
    let a = ev(&mut w, 2, &[], 10); // 保留
    let b = ev(&mut w, 1, &[a], 20); // 修剪
    let c = ev(&mut w, 1, &[b], 30); // 修剪
    let d = ev(&mut w, 3, &[c, b], 40); // 保留，改指向 a
    let x = ev(&mut w, 1, &[d], 12 * 100 + 20); // 未滿 100 年（相對 now），保留
    let y = ev(&mut w, 2, &[x], 12 * 100 + 30);
    // 人物最近事件指向被修剪的 c。
    let (_, pid) = adult(&mut w, 0, 0);
    w.persons[pid as usize].last_event = Some(c);
    w.tick = 12 * 100 + 50;
    ledger::prune(&mut w);
    assert!(ledger::get(&w, b).is_none() && ledger::get(&w, c).is_none());
    assert_eq!(w.ledger.events[b as usize].importance, 0);
    assert!(ledger::get(&w, a).is_some() && ledger::get(&w, d).is_some());
    assert_eq!(w.ledger.events[d as usize].causes.as_slice(), &[a], "溯因不斷鏈");
    assert_eq!(w.ledger.effects[a as usize].as_slice(), &[d]);
    assert!(w.ledger.effects[b as usize].is_empty() && w.ledger.effects[c as usize].is_empty());
    assert_eq!(w.persons[pid as usize].last_event, Some(a));
    // 近期的 importance 1 與 importance ≥ 2 都保留。
    assert!(ledger::get(&w, x).is_some() && ledger::get(&w, y).is_some());
    // 之後滿 100 年再修剪 x。
    w.tick += 12 * 100;
    ledger::prune(&mut w);
    assert!(ledger::get(&w, x).is_none());
    assert_eq!(w.ledger.events[y as usize].causes.as_slice(), &[d]);
    assert_eq!(w.ledger.effects[d as usize].as_slice(), &[y]);
    // 重複修剪無副作用。
    let snapshot = w.ledger.clone();
    ledger::prune(&mut w);
    assert_eq!(w.ledger, snapshot);
}

#[test]
fn prune_via_sim_year_boundary_is_idempotent_for_empty_ledger() {
    let mut w = line_world(1);
    ledger::prune(&mut w);
    assert!(w.ledger.events.len() < 100);
}

// ── 確定性 ───────────────────────────────────────────────────

#[test]
fn yearly_is_deterministic() {
    let run = || {
        let mut w = line_world(42);
        let mut ids = vec![];
        for i in 0..8u16 {
            ids.push(adult(&mut w, i % 4, (i % 4) as u8).0);
        }
        for i in 0..8usize {
            let kind = [ThreadKind::Feud, ThreadKind::Love, ThreadKind::Grace, ThreadKind::Mentor][i % 4];
            create(&mut w, kind, ids[i], ids[(i + 3) % 8], 60.0, false, None);
        }
        for _ in 0..40 {
            w.tick += 12;
            yearly(&mut w);
            ledger::prune(&mut w);
        }
        (w.threads.clone(), w.ledger.clone(), w.book.clone(), w.kill_pairs.clone())
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    assert!(!a.1.events.is_empty());
}
