//! U4 單元測試：判定表、牽引分布、配對、延後、傳承、確定性。

use smallvec::smallvec;

use super::pull::*;
use super::*;
use crate::types::{Geography, Province, Terrain, WorldParams};

fn province(id: u16, pop: f64, neighbors: Vec<u16>, faith: [f32; 3]) -> Province {
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
        aura: 0.5,
        karma_qi: 0.0,
        merit: 0.0,
        righteous: 0.0,
        devastation: 0.0,
        faith,
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

/// 一條直線上的 n 個州郡：i 與 i±1 為陸鄰。
fn line_world(seed: u64, pops: &[f64]) -> World {
    let n = pops.len();
    let provs = (0..n)
        .map(|i| {
            let mut nb = vec![];
            if i > 0 {
                nb.push(i as u16 - 1);
            }
            if i + 1 < n {
                nb.push(i as u16 + 1);
            }
            province(i as u16, pops[i], nb, [0.3, 0.1, 0.1])
        })
        .collect();
    let mut w = World::empty(WorldParams::new(seed), geo(), provs);
    ensure_heaven(&mut w);
    w
}

/// 在州郡 `prov` 建立一個在世的魂與肉身。
fn living(w: &mut World, prov: u16, g: f32, s: f32) -> (SoulId, PersonId) {
    let (sid, pid) = spawn_new_life(w, SoulOrigin::New, prov);
    w.souls[sid as usize].merit = g;
    w.souls[sid as usize].sin = s;
    (sid, pid)
}

fn kill(w: &mut World, pid: PersonId) {
    w.persons[pid as usize].died = Some((w.tick, DeathCause::Age));
    on_death(w, pid, None);
}

fn add_thread(w: &mut World, kind: ThreadKind, debtor: SoulId, creditor: SoulId, strength: f32) -> ThreadId {
    let id = w.threads.len() as ThreadId;
    w.threads.push(Thread {
        id,
        kind,
        debtor,
        creditor,
        strength,
        created: (0, 0),
        lives_base: (0, 0),
        woven: false,
        resolved: None,
        omen_at: None,
        master_best: None,
    });
    w.souls[debtor as usize].threads.push(id);
    w.souls[creditor as usize].threads.push(id);
    id
}

fn run_to(w: &mut World, tick: Tick) {
    while w.tick < tick {
        w.tick += 1;
        monthly(w);
    }
}

fn soul_with(g: f32, s: f32, wisdom: f32, demon: f32) -> Soul {
    let mut w = line_world(1, &[1.0]);
    let (sid, _) = living(&mut w, 0, g, s);
    let mut soul = w.souls[sid as usize].clone();
    soul.wisdom = wisdom;
    soul.affinity = [(1.0 - demon) / 3.0, (1.0 - demon) / 3.0, (1.0 - demon) / 3.0, demon];
    soul
}

// ── 判定表（3.2）───────────────────────────────────────────

#[test]
fn gdd_3_2_judgement_table_boundaries() {
    let j = |g: f32, s: f32, wis: f32, demon: f32, noise: f32| {
        let soul = soul_with(g, s, wis, demon);
        judge_path(&soul, judge_value(&soul, 0.0, noise))
    };
    // 天道：x ≥ 200 且 W ≥ 50。
    assert_eq!(j(200.0, 0.0, 50.0, 0.0, 0.0), SixPath::Heaven);
    assert_eq!(j(200.0, 0.0, 49.9, 0.0, 0.0), SixPath::Human);
    assert_eq!(j(199.9, 0.0, 100.0, 0.0, 0.0), SixPath::Human);
    // 雜訊可以把人推過天道門檻。
    assert_eq!(j(190.0, 0.0, 60.0, 0.0, 15.0), SixPath::Heaven);
    // 阿修羅：S ≥ 150 且 魔 ≥ 0.4，優先於人道。
    assert_eq!(j(300.0, 150.0, 0.0, 0.4, 0.0), SixPath::Asura);
    assert_eq!(j(300.0, 150.0, 0.0, 0.39, 0.0), SixPath::Human);
    assert_eq!(j(300.0, 149.9, 0.0, 0.9, 0.0), SixPath::Human);
    // 天道優先於阿修羅。
    assert_eq!(j(500.0, 150.0, 60.0, 0.5, 0.0), SixPath::Heaven);
    // 人道 x ≥ −50。
    assert_eq!(j(0.0, 50.0, 0.0, 0.0, 0.0), SixPath::Human);
    assert_eq!(j(0.0, 50.1, 0.0, 0.0, 0.0), SixPath::Beast);
    // 畜生道 x ≥ −300；M2 暫行：W ≥ 30 也一律為獸。
    assert_eq!(j(0.0, 300.0, 0.0, 0.0, 0.0), SixPath::Beast);
    assert_eq!(j(0.0, 300.0, 120.0, 0.0, 0.0), SixPath::Beast);
    // 餓鬼道 x ≥ −600；其餘地獄。
    assert_eq!(j(0.0, 300.1, 0.0, 0.0, 0.0), SixPath::Hungry);
    assert_eq!(j(0.0, 600.0, 0.0, 0.0, 0.0), SixPath::Hungry);
    assert_eq!(j(0.0, 600.1, 0.0, 0.0, 0.0), SixPath::Hell);
}

#[test]
fn gdd_3_2_realm_durations_and_returns() {
    assert_eq!(realm_months(SixPath::Heaven), Some((360, 1200)));
    assert_eq!(realm_months(SixPath::Beast), Some((120, 360)));
    assert_eq!(realm_months(SixPath::Hungry), Some((600, 1800)));
    assert_eq!(realm_months(SixPath::Hell), Some((1800, 6000)));
    assert_eq!(realm_months(SixPath::Human), None);

    // 為獸：期滿回來 S × 0.9，重新判定。
    let mut w = line_world(7, &[100.0, 100.0]);
    let (sid, pid) = living(&mut w, 0, 0.0, 250.0);
    w.tick = 10;
    kill(&mut w, pid);
    run_to(&mut w, 12);
    let SoulState::Realm { path, until } = w.souls[sid as usize].state else { panic!("{:?}", w.souls[sid as usize].state) };
    assert_eq!(path, SixPath::Beast);
    assert!((12 + 120..=12 + 360).contains(&until));
    let s_before = w.souls[sid as usize].sin;
    w.tick = until - 1;
    run_to(&mut w, until);
    // 重判：S = 225 → x ≈ −225 → 仍是畜生道（或因雜訊相同），S 已被 × 0.9。
    assert!((w.souls[sid as usize].sin - s_before * 0.9).abs() < 1e-3);
    assert!(matches!(w.souls[sid as usize].state, SoulState::Realm { path: SixPath::Beast, .. }));

    // 天道：回來 G × 0.5。
    let mut w = line_world(8, &[100.0]);
    let (sid, _) = living(&mut w, 0, 400.0, 0.0);
    w.souls[sid as usize].wisdom = 80.0;
    w.souls[sid as usize].state = SoulState::Realm { path: SixPath::Heaven, until: 5 };
    w.persons.clear();
    w.alive_ids.clear();
    w.tick = 4;
    run_to(&mut w, 5);
    // 400 × 0.5 = 200，加雜訊後可能 < 200 而判入人道；不論如何 G 都減半。
    assert!((w.souls[sid as usize].merit - 200.0).abs() < 1e-3);
}

#[test]
fn gdd_3_2_hungry_and_hell_decay_yearly() {
    let mut w = line_world(3, &[10.0]);
    let (a, _) = living(&mut w, 0, 0.0, 100.0);
    let (b, _) = living(&mut w, 0, 0.0, 100.0);
    w.souls[a as usize].state = SoulState::Realm { path: SixPath::Hungry, until: 9999 };
    w.souls[b as usize].state = SoulState::Realm { path: SixPath::Hell, until: 9999 };
    run_to(&mut w, 24);
    assert!((w.souls[a as usize].sin - 100.0 * 0.98 * 0.98).abs() < 1e-3);
    assert!((w.souls[b as usize].sin - 100.0 * 0.99 * 0.99).abs() < 1e-3);
}

#[test]
fn gdd_3_2_death_bardo_then_rebirth_flow() {
    let mut w = line_world(11, &[100.0, 100.0, 100.0]);
    let (sid, pid) = living(&mut w, 1, 100.0, 0.0);
    w.tick = 100;
    kill(&mut w, pid);
    assert_eq!(w.souls[sid as usize].state, SoulState::Bardo { until: 102 });
    run_to(&mut w, 101);
    assert!(matches!(w.souls[sid as usize].state, SoulState::Bardo { .. }));
    run_to(&mut w, 102);
    let SoulState::Incarnate(np) = w.souls[sid as usize].state else { panic!() };
    assert_ne!(np, pid);
    assert_eq!(w.souls[sid as usize].lives, vec![pid, np]);
    assert_eq!(w.persons[np as usize].born, 102);
    // Judged、Rebirth 事件，Rebirth 的因是 Judged 之前的死亡事件（此測試沒寫死亡事件，故無因）。
    let kinds: Vec<_> = w.ledger.events.iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&EventKind::Judged) && kinds.contains(&EventKind::Rebirth));

    // 超脫：不經中陰。
    let (s2, p2) = living(&mut w, 0, 0.0, 0.0);
    w.persons[p2 as usize].died = Some((102, DeathCause::Ascended));
    on_death(&mut w, p2, Some(Liberation::Ascension));
    assert_eq!(w.souls[s2 as usize].state, SoulState::Liberated { way: Liberation::Ascension, tick: 102 });
}

#[test]
fn gdd_3_2_bodhisattva_vow_and_asura_mark() {
    let mut w = line_world(5, &[100.0, 100.0, 100.0]);
    w.provinces[2].karma_qi = 0.9;
    let (sid, pid) = living(&mut w, 0, 0.0, 1000.0);
    w.souls[sid as usize].marks.set(SoulMarks::BODHISATTVA_VOW);
    kill(&mut w, pid);
    run_to(&mut w, 2);
    let SoulState::Incarnate(np) = w.souls[sid as usize].state else { panic!("{:?}", w.souls[sid as usize].state) };
    assert_eq!(w.persons[np as usize].province, 2);

    let mut w = line_world(5, &[100.0, 100.0]);
    let (sid, pid) = living(&mut w, 0, 900.0, 200.0);
    w.souls[sid as usize].affinity = [0.1, 0.1, 0.1, 0.7];
    kill(&mut w, pid);
    run_to(&mut w, 2);
    assert!(matches!(w.souls[sid as usize].state, SoulState::Incarnate(_)));
    assert!(w.souls[sid as usize].marks.has(SoulMarks::ASURA));
}

// ── 因果牽引（3.2）─────────────────────────────────────────

#[test]
fn gdd_3_2_pull_distribution_formula() {
    // 0-1-2 直線。對方在州 2（陸鄰只有 1）。
    let mut w = line_world(1, &[600.0, 300.0, 100.0]);
    w.provinces[0].faith = [0.0, 1.0, 0.0]; // 釋
    w.provinces[1].faith = [1.0, 0.0, 0.0]; // 道
    w.provinces[2].faith = [0.0, 0.0, 1.0]; // 儒
    let (me, _) = living(&mut w, 0, 0.0, 0.0);
    let (other, _) = living(&mut w, 2, 0.0, 0.0);
    w.souls[me as usize].state = SoulState::Bardo { until: 0 };
    w.souls[me as usize].affinity = [1.0, 0.0, 0.0, 0.0]; // 純道緣
    add_thread(&mut w, ThreadKind::Feud, me, other, 60.0);

    let p = pull_distribution(&w, &w.souls[me as usize]);
    // 人口 0.6, 0.3, 0.1；道緣 0,1,0；宿緣：強度 60 平分給州 2 與其鄰 1 → 0, .5, .5。
    let expect = [0.5 * 0.6, 0.5 * 0.3 + 0.2 * 1.0 + 0.3 * 0.5, 0.5 * 0.1 + 0.3 * 0.5];
    for i in 0..3 {
        assert!((p[i] - expect[i]).abs() < 1e-6, "{i}: {} vs {}", p[i], expect[i]);
    }
    assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-6);

    // 對方不在世（中陰）→ 宿緣分布為空，權重 0.7 / 0.3。
    w.souls[other as usize].state = SoulState::Bardo { until: 0 };
    let p = pull_distribution(&w, &w.souls[me as usize]);
    let expect = [0.7 * 0.6, 0.7 * 0.3 + 0.3, 0.7 * 0.1];
    for i in 0..3 {
        assert!((p[i] - expect[i]).abs() < 1e-6);
    }
}

#[test]
fn gdd_3_2_pull_sampling_matches_distribution() {
    let mut w = line_world(2024, &[600.0, 300.0, 100.0]);
    let (me, _) = living(&mut w, 0, 0.0, 0.0);
    let (other, _) = living(&mut w, 2, 0.0, 0.0);
    add_thread(&mut w, ThreadKind::Grace, me, other, 80.0);
    w.souls[me as usize].state = SoulState::Bardo { until: 0 };
    let dist = pull_distribution(&w, &w.souls[me as usize]);
    let n = 40_000;
    let mut count = [0u32; 3];
    for _ in 0..n {
        count[sample_birth_province(&mut w, me) as usize] += 1;
    }
    for i in 0..3 {
        let f = count[i] as f64 / n as f64;
        assert!((f - dist[i]).abs() < 0.012, "{i}: {f} vs {}", dist[i]);
    }
}

// ── 等待與配對（3.2）───────────────────────────────────────

#[test]
fn gdd_3_2_waiting_and_pairing_same_month() {
    let mut w = line_world(99, &[100.0, 100.0, 100.0, 100.0, 100.0]);
    let (a, pa) = living(&mut w, 0, 100.0, 0.0);
    let (b, pb) = living(&mut w, 4, 100.0, 0.0);
    add_thread(&mut w, ThreadKind::Feud, a, b, 70.0);
    // a 先死，b 晚 6 個月死。
    w.tick = 10;
    kill(&mut w, pa);
    w.tick = 16;
    kill(&mut w, pb);
    w.tick = 11;
    run_to(&mut w, 12);
    let SoulState::Waiting { until, ref for_souls } = w.souls[a as usize].state else { panic!("{:?}", w.souls[a as usize].state) };
    assert_eq!(until, 12 + 360);
    assert_eq!(for_souls.as_slice(), &[b]);
    // b 判定那個月，兩者同月出生，a 在 b 的州或其陸鄰。
    run_to(&mut w, 18);
    let (SoulState::Incarnate(na), SoulState::Incarnate(nb)) = (w.souls[a as usize].state.clone(), w.souls[b as usize].state.clone())
    else {
        panic!()
    };
    assert_eq!(w.persons[na as usize].born, 18);
    assert_eq!(w.persons[nb as usize].born, 18);
    let pb_prov = w.persons[nb as usize].province;
    let pa_prov = w.persons[na as usize].province;
    assert!(pa_prov == pb_prov || w.provinces[pb_prov as usize].neighbors.contains(&pa_prov));
    // 利息：a 是欠債方，轉世後線強度 70 × 1.1，S += 7。
    assert!((w.threads[0].strength - 77.0).abs() < 1e-3);
    assert!((w.souls[a as usize].sin - 7.0).abs() < 1e-3);
}

#[test]
fn gdd_3_2_no_wait_cases() {
    let mut w = line_world(1, &[100.0, 100.0]);
    let (a, _) = living(&mut w, 0, 100.0, 0.0);
    let (b, _) = living(&mut w, 1, 100.0, 0.0);
    let t = add_thread(&mut w, ThreadKind::Feud, a, b, 70.0);
    let sa = w.souls[a as usize].clone();
    // 對方在世：不等。
    assert_eq!(wait_target(&w, &sa, 0), None);
    // 對方在中陰：等。
    w.souls[b as usize].state = SoulState::Bardo { until: 2 };
    assert_eq!(wait_target(&w, &sa, 0), Some(b));
    // 強度不足：不等。
    w.threads[t as usize].strength = 49.9;
    assert_eq!(wait_target(&w, &sa, 0), None);
    w.threads[t as usize].strength = 50.0;
    assert_eq!(wait_target(&w, &sa, 0), Some(b));
    // 他道 30 年內期滿：等；尚遠：不等。
    w.souls[b as usize].state = SoulState::Realm { path: SixPath::Hell, until: 360 };
    assert_eq!(wait_target(&w, &sa, 0), Some(b));
    w.souls[b as usize].state = SoulState::Realm { path: SixPath::Hell, until: 361 };
    assert_eq!(wait_target(&w, &sa, 0), None);
    // 對方已超脫、在等待中：不等。
    w.souls[b as usize].state = SoulState::Liberated { way: Liberation::Nirvana, tick: 1 };
    assert_eq!(wait_target(&w, &sa, 0), None);
    w.souls[b as usize].state = SoulState::Waiting { until: 5, for_souls: smallvec![99] };
    assert_eq!(wait_target(&w, &sa, 0), None);
    // 多條線取最強者。
    let c = {
        let (c, _) = living(&mut w, 1, 0.0, 0.0);
        c
    };
    w.souls[b as usize].state = SoulState::Bardo { until: 2 };
    w.souls[c as usize].state = SoulState::Bardo { until: 2 };
    add_thread(&mut w, ThreadKind::Grace, a, c, 90.0);
    let sa = w.souls[a as usize].clone();
    assert_eq!(wait_target(&w, &sa, 0), Some(c));
}

#[test]
fn gdd_3_2_waiting_timeout_births_alone() {
    let mut w = line_world(4, &[100.0, 100.0]);
    let (a, _) = living(&mut w, 0, 100.0, 0.0);
    w.persons.clear();
    w.alive_ids.clear();
    w.souls[a as usize].state = SoulState::Waiting { until: 360, for_souls: smallvec![5] };
    run_to(&mut w, 359);
    assert!(matches!(w.souls[a as usize].state, SoulState::Waiting { .. }));
    run_to(&mut w, 360);
    assert!(matches!(w.souls[a as usize].state, SoulState::Incarnate(_)));
}

#[test]
fn gdd_3_2_birth_cap_defers_group() {
    let mut w = line_world(6, &[100.0, 100.0]);
    let cap = notable_cap(&w);
    let (a, pa) = living(&mut w, 0, 100.0, 0.0);
    w.tick = 1;
    kill(&mut w, pa);
    // 填滿到上限。
    for _ in 0..cap {
        spawn_new_life(&mut w, SoulOrigin::New, 1);
    }
    assert_eq!(living_notables(&w), cap);
    run_to(&mut w, 3);
    // 判定當月因上限延後 1 年。
    assert_eq!(w.souls[a as usize].state, SoulState::Bardo { until: 3 + 12 });
    assert!(!w.ledger.events.iter().any(|e| e.kind == EventKind::Judged));
    // 空出一個位置後出生。
    let victim = w.persons.iter().rposition(|p| p.alive()).unwrap() as PersonId;
    w.persons[victim as usize].died = Some((3, DeathCause::Age));
    run_to(&mut w, 15);
    assert!(matches!(w.souls[a as usize].state, SoulState::Incarnate(_)));
}

// ── 傳承（3.1）────────────────────────────────────────────

#[test]
fn gdd_3_1_wisdom_and_affinity_inheritance() {
    let mut w = line_world(1, &[100.0]);
    w.provinces[0].faith = [0.6, 0.2, 0.2];
    let (sid, pid) = living(&mut w, 0, 50.0, 0.0);
    w.souls[sid as usize].wisdom = 40.0;
    w.souls[sid as usize].affinity = [0.0, 0.0, 0.0, 1.0];
    w.persons[pid as usize].role = Role::Demonic;
    w.persons[pid as usize].path = PathState::Cultivator { realm: 5, exp: 0.0, last_attempt: None };
    w.persons[pid as usize].died = Some((0, DeathCause::ShedBody));
    on_death(&mut w, pid, None);
    assert_eq!(w.souls[sid as usize].best_now.realm, Some(5));
    prepare_rebirth(&mut w, sid, 0);
    let s = &w.souls[sid as usize];
    // 40 × 0.5 + 5 × 8 + 30 = 90。
    assert!((s.wisdom - 90.0).abs() < 1e-4);
    assert_eq!(s.best_prev.realm, Some(5));
    assert_eq!(s.best_now, Achievements::default());
    // 0.7 × (0,0,0,1) + 0.3 × faith4(0.6,0.2,0.2,0) → 魔 0.7 + 0.2 = 0.9 之前正規化：
    // 先 normalize 得 (.18,.06,.06,.7)，魔 +0.2 → (.18,.06,.06,.9)，再除以 1.2。
    let expect = [0.18 / 1.2, 0.06 / 1.2, 0.06 / 1.2, 0.9 / 1.2];
    for (k, e) in expect.iter().enumerate() {
        assert!((s.affinity[k] - e).abs() < 1e-5, "{k}");
    }
    assert!((s.affinity.iter().sum::<f32>() - 1.0).abs() < 1e-5);
    assert!(!s.marks.has(SoulMarks::LAST_LIFE_DEMON) && !s.marks.has(SoulMarks::LAST_LIFE_SHED));
}

#[test]
fn gdd_3_1_wisdom_gain_by_identity_and_clamp() {
    let mut w = line_world(1, &[100.0]);
    // 僧：果位 × 15；儒：三不朽 / 10；上限 200。
    let (m, pm) = living(&mut w, 0, 0.0, 0.0);
    w.persons[pm as usize].path = PathState::Monk { fruit: 3, practice: 0.0 };
    w.persons[pm as usize].died = Some((0, DeathCause::Age));
    on_death(&mut w, pm, None);
    prepare_rebirth(&mut w, m, 0);
    assert!((w.souls[m as usize].wisdom - 45.0).abs() < 1e-4);

    let (r, pr) = living(&mut w, 0, 0.0, 0.0);
    w.persons[pr as usize].path = PathState::Scholar { virtue: 100.0, deed: 50.0, word: 30.0 };
    w.persons[pr as usize].died = Some((0, DeathCause::Age));
    on_death(&mut w, pr, None);
    prepare_rebirth(&mut w, r, 0);
    assert!((w.souls[r as usize].wisdom - 18.0).abs() < 1e-4);

    w.souls[r as usize].wisdom = 200.0;
    w.souls[r as usize].best_now.realm = Some(8);
    w.souls[r as usize].best_now.fruit = Some(4);
    prepare_rebirth(&mut w, r, 0);
    assert_eq!(w.souls[r as usize].wisdom, 200.0);
}

#[test]
fn gdd_3_1_interest_only_for_debtor_feud_and_grudge() {
    let mut w = line_world(1, &[100.0]);
    let (a, _) = living(&mut w, 0, 0.0, 0.0);
    let (b, _) = living(&mut w, 0, 0.0, 0.0);
    let feud = add_thread(&mut w, ThreadKind::Feud, a, b, 95.0);
    let grudge = add_thread(&mut w, ThreadKind::Grudge, a, b, 60.0);
    let grace = add_thread(&mut w, ThreadKind::Grace, a, b, 30.0);
    let as_creditor = add_thread(&mut w, ThreadKind::Feud, b, a, 40.0);
    let done = add_thread(&mut w, ThreadKind::Feud, a, b, 40.0);
    w.threads[done as usize].resolved = Some((1, 0));
    prepare_rebirth(&mut w, a, 0);
    assert!((w.threads[feud as usize].strength - 100.0).abs() < 1e-4); // 95 × 1.1 封頂
    assert!((w.threads[grudge as usize].strength - 66.0).abs() < 1e-4);
    assert_eq!(w.threads[grace as usize].strength, 30.0);
    assert_eq!(w.threads[as_creditor as usize].strength, 40.0);
    assert_eq!(w.threads[done as usize].strength, 40.0);
    // S += 95 × 0.1 + 60 × 0.1（以舊強度）。
    assert!((w.souls[a as usize].sin - 15.5).abs() < 1e-4);
}

// ── 補魂與新魂 ─────────────────────────────────────────────

#[test]
fn gdd_3_4_refill_below_floor() {
    let mut w = line_world(1, &[100.0, 100.0]);
    let cap = notable_cap(&w);
    let floor = cap as f32 * NOTABLE_FLOOR_RATIO;
    // 在世 0 個：補 ceil(floor × 0.1)。
    yearly_refill(&mut w);
    let want = (floor * REFILL_RATE).ceil() as u32;
    assert_eq!(living_notables(&w), want);
    assert!(w.persons.iter().all(|p| p.born == 0 && p.role == Role::Child));
    assert!(w.souls.iter().skip(1).all(|s| s.origin == SoulOrigin::New && s.lives.len() == 1));
    // 達到 floor 後不再補。
    while (living_notables(&w) as f32) < floor {
        yearly_refill(&mut w);
    }
    let n = living_notables(&w);
    yearly_refill(&mut w);
    assert_eq!(living_notables(&w), n);
}

#[test]
fn gdd_3_2_take_for_birth_finds_waiting_with_partner_in_province() {
    let mut w = line_world(1, &[100.0, 100.0]);
    let (a, _) = living(&mut w, 0, 0.0, 0.0);
    let (b, _) = living(&mut w, 1, 0.0, 0.0);
    assert_eq!(take_for_birth(&w, 1), None);
    w.souls[a as usize].state = SoulState::Waiting { until: 50, for_souls: smallvec![b] };
    assert_eq!(take_for_birth(&w, 1), Some(a));
    assert_eq!(take_for_birth(&w, 0), None);
}

// ── 確定性（6.5）───────────────────────────────────────────

fn scenario(seed: u64) -> World {
    let mut w = line_world(seed, &[500.0, 300.0, 200.0, 100.0, 50.0, 400.0]);
    let mut ids = vec![];
    for i in 0..60u16 {
        let (s, p) = living(&mut w, i % 6, (i as f32 * 37.0) % 300.0, (i as f32 * 53.0) % 400.0);
        ids.push((s, p));
    }
    for k in 0..20 {
        add_thread(&mut w, ThreadKind::Feud, ids[k].0, ids[k + 20].0, 55.0 + k as f32);
    }
    for (i, &(_, p)) in ids.iter().enumerate() {
        w.tick = (i as Tick % 24) + 1;
        kill(&mut w, p);
    }
    w.tick = 0;
    for t in 1..=3000 {
        w.tick = t;
        monthly(&mut w);
        if t % 12 == 0 {
            yearly_refill(&mut w);
        }
    }
    w
}

#[test]
fn gdd_6_5_soul_system_is_deterministic() {
    let a = scenario(2024);
    let b = scenario(2024);
    assert_eq!(format!("{:?}", a.souls), format!("{:?}", b.souls));
    assert_eq!(format!("{:?}", a.persons), format!("{:?}", b.persons));
    assert_eq!(format!("{:?}", a.threads), format!("{:?}", b.threads));
    assert_eq!(a.ledger, b.ledger);
    assert_eq!(crate::hash::world_hash(&a), crate::hash::world_hash(&b));
    // 不同種子結果不同，且確實發生了輪迴。
    let c = scenario(2025);
    assert_ne!(format!("{:?}", a.souls), format!("{:?}", c.souls));
    assert!(a.ledger.events.iter().any(|e| e.kind == EventKind::Rebirth));
    assert!(a.ledger.events.iter().any(|e| e.kind == EventKind::Judged));
}
