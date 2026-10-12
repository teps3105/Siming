//! GDD 6.9 測試：運算子的守恆與順序無關、規格表與雜湊順序、統一入口的截斷與往返。

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::*;
use crate::types::{Geography, Terrain, WorldParams};

fn prov(id: u16, neighbors: Vec<u16>) -> Province {
    Province {
        id,
        name: format!("p{id}"),
        cells: vec![],
        seat: 0,
        neighbors,
        sea_neighbors: vec![],
        terrain_mix: [0; Terrain::COUNT],
        habitability: 10.0,
        capacity: 0.0,
        population: 100.0,
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

fn empty_geo() -> Geography {
    Geography {
        w: 0,
        h: 0,
        wrap_x: true,
        sea_level: 0.3,
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

fn world(n: usize) -> World {
    let provs = (0..n).map(|i| prov(i as u16, vec![])).collect();
    World::empty(WorldParams::new(7), empty_geo(), provs)
}

/// 隨機對稱圖：n 個點，邊以機率 p 產生；鄰居依 ID 升冪。
fn random_graph(rng: &mut ChaCha8Rng, n: usize, p: f64) -> Vec<Vec<u16>> {
    let mut nb = vec![Vec::new(); n];
    for i in 0..n {
        for j in i + 1..n {
            if rng.gen_bool(p) {
                nb[i].push(j as u16);
                nb[j].push(i as u16);
            }
        }
    }
    for l in &mut nb {
        l.sort_unstable();
    }
    nb
}

fn sum(v: &[f32]) -> f64 {
    v.iter().map(|x| *x as f64).sum()
}

/// 以 `perm`（舊 ID → 新 ID）重新編號：值、鄰居（升冪）。
fn renumber(v: &[f32], nb: &[Vec<u16>], perm: &[usize]) -> (Vec<f32>, Vec<Vec<u16>>) {
    let n = v.len();
    let mut v2 = vec![0.0; n];
    let mut nb2 = vec![Vec::new(); n];
    for i in 0..n {
        v2[perm[i]] = v[i];
        let mut l: Vec<u16> = nb[i].iter().map(|&j| perm[j as usize] as u16).collect();
        l.sort_unstable();
        nb2[perm[i]] = l;
    }
    (v2, nb2)
}

fn shuffled_perm(rng: &mut ChaCha8Rng, n: usize) -> Vec<usize> {
    let mut perm: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        perm.swap(i, rng.gen_range(0..=i));
    }
    perm
}

#[test]
fn gdd_6_9_diffuse_conserves() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    for round in 0..20 {
        let n = 30 + round;
        let nb = random_graph(&mut rng, n, 0.12);
        let v: Vec<f32> = (0..n).map(|_| rng.gen_range(0.0..1.0)).collect();
        for k in [KARMA_DIFFUSE, MERIT_DIFFUSE, 1.0] {
            let out = kernel::diffuse(&v, &nb, k);
            assert!((sum(&out) - sum(&v)).abs() <= 1e-5, "擴散總和不守恆：{} → {}", sum(&v), sum(&out));
            assert!(out.iter().all(|x| *x >= 0.0), "擴散後出現負值");
        }
    }
}

#[test]
fn gdd_6_9_drift_conserves() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    for round in 0..20 {
        let n = 30 + round;
        let v: Vec<f32> = (0..n).map(|_| rng.gen_range(0.0..1.0)).collect();
        // 約三成的州下游為海（None），其餘指向隨機他州。
        let down: Vec<Option<u16>> =
            (0..n).map(|_| if rng.gen_bool(0.3) { None } else { Some(rng.gen_range(0..n) as u16) }).collect();
        let out = kernel::drift(&v, &down, KARMA_DRIFT);
        assert!((sum(&out) - sum(&v)).abs() <= 1e-5, "順流總和不守恆");
        let mut is_target = vec![false; n];
        for d in down.iter().flatten() {
            is_target[*d as usize] = true;
        }
        for i in 0..n {
            // 下游為海且無人流入者：完全不動。
            if down[i].is_none() && !is_target[i] {
                assert_eq!(out[i], v[i], "下游為海的州 {i} 不應移動");
            }
        }
    }
    // 兩州一線：0 → 1，1 入海；0 的 4% 移到 1。
    let out = kernel::drift(&[1.0, 0.0], &[Some(1), None], KARMA_DRIFT);
    assert!((out[0] - 0.96).abs() < 1e-6 && (out[1] - 0.04).abs() < 1e-6);
}

#[test]
fn gdd_6_9_order_independent() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    for _ in 0..10 {
        let n = 40;
        let nb = random_graph(&mut rng, n, 0.1);
        let v: Vec<f32> = (0..n).map(|_| rng.gen_range(0.0..1.0)).collect();
        let down: Vec<Option<u16>> =
            (0..n).map(|_| if rng.gen_bool(0.3) { None } else { Some(rng.gen_range(0..n) as u16) }).collect();
        let perm = shuffled_perm(&mut rng, n);
        let (v2, nb2) = renumber(&v, &nb, &perm);
        let mut down2 = vec![None; n];
        for i in 0..n {
            down2[perm[i]] = down[i].map(|d| perm[d as usize] as u16);
        }
        let a = kernel::diffuse(&v, &nb, KARMA_DIFFUSE);
        let b = kernel::diffuse(&v2, &nb2, KARMA_DIFFUSE);
        let c = kernel::drift(&v, &down, KARMA_DRIFT);
        let d = kernel::drift(&v2, &down2, KARMA_DRIFT);
        for i in 0..n {
            // 加總順序因重新編號而異，浮點結果允許最後幾位的差。
            assert!((a[i] - b[perm[i]]).abs() <= 1e-6, "擴散與編號有關：州 {i}");
            assert!((c[i] - d[perm[i]]).abs() <= 1e-6, "順流與編號有關：州 {i}");
        }
    }
}

/// 解析 `hash.rs` 的 `province()` 餵入州域場的順序，以 `FieldId` 的州域場名表示；`faith` 三維合為 `FaithTao`。
fn hash_feed_order() -> Vec<FieldId> {
    let src = include_str!("../hash.rs");
    let start = src.find("fn province(").expect("hash.rs 應有 province()");
    let body = &src[start..];
    let body = &body[..body.find("\n}").expect("province() 應有結尾")];
    let mut order: Vec<FieldId> = Vec::new();
    for (name, id) in [
        ("p.population", FieldId::Population),
        ("p.devastation", FieldId::Devastation),
        ("p.faith", FieldId::FaithTao),
        ("p.aura", FieldId::Aura),
        ("p.karma_qi", FieldId::KarmaQi),
        ("p.merit", FieldId::Merit),
        ("p.righteous", FieldId::Righteous),
    ] {
        assert!(body.contains(name), "hash.rs 的 province() 應餵入 {name}");
        order.push(id);
    }
    // 依各欄位在原始碼中第一次出現的位置排序，即實際餵入順序。
    let pos = |id: FieldId| {
        let name = match id {
            FieldId::Population => "p.population",
            FieldId::Devastation => "p.devastation",
            FieldId::FaithTao => "p.faith",
            FieldId::Aura => "p.aura",
            FieldId::KarmaQi => "p.karma_qi",
            FieldId::Merit => "p.merit",
            _ => "p.righteous",
        };
        body.find(name).unwrap()
    };
    order.sort_by_key(|id| pos(*id));
    order
}

#[test]
fn gdd_6_9_hash_order() {
    // 6.9 規定 `World::hash()` 依 FieldId 順序餵入州域場，但現行 `hash.rs` 的 `province()` 順序是
    // 人口、靈氣、業氣、功德、正氣、殘破、教化；FieldId 順序（只計已有儲存者）是
    // 人口、殘破、教化、靈氣、業氣、功德、正氣。兩者不一致：殘破與教化在雜湊中排在氣場之後。
    // 改雜湊順序會改變所有世界雜湊並須遞增 sim_version，這一項不動雜湊，只把現況釘在這裡；
    // 日後調整雜湊順序時，同步更新此表與 6.9。
    let actual = hash_feed_order();
    let pinned = [
        FieldId::Population,
        FieldId::Aura,
        FieldId::KarmaQi,
        FieldId::Merit,
        FieldId::Righteous,
        FieldId::Devastation,
        FieldId::FaithTao,
    ];
    assert_eq!(actual, pinned, "hash.rs 的 province() 餵入順序已變，請同步本測試與 6.9");

    let mut by_field_id: Vec<FieldId> = pinned.to_vec();
    by_field_id.sort();
    assert_ne!(actual, by_field_id, "若兩者已一致，請把本測試改成斷言一致並刪除上方說明");

    // 規格表的列序即 FieldId 的數值順序。
    for (i, s) in FIELD_SPECS.iter().enumerate() {
        assert_eq!(s.id as usize, i, "FIELD_SPECS 第 {i} 列的 id 與列舉順序不符");
        assert_eq!(FieldId::ALL[i] as usize, i);
    }
    assert_eq!(FieldId::COUNT, 22);
}

#[test]
fn field_specs_use_params() {
    let k = FieldId::KarmaQi.spec();
    assert_eq!((k.decay_per_year, k.diffuse_per_season, k.drift_per_season), (KARMA_DECAY, KARMA_DIFFUSE, KARMA_DRIFT));
    let m = FieldId::Merit.spec();
    assert_eq!((m.decay_per_year, m.diffuse_per_season, m.drift_per_season), (MERIT_DECAY, MERIT_DIFFUSE, 0.0));
    let r = FieldId::Righteous.spec();
    assert_eq!((r.decay_per_year, r.diffuse_per_season, r.drift_per_season), (RIGHT_DECAY, RIGHT_DIFFUSE, 0.0));
    assert_eq!(FieldId::FaithBuddha.spec().relax_per_year, FAITH_DRIFT);
}

#[test]
fn add_clamps_into_range() {
    let mut w = world(2);
    add(&mut w, FieldId::KarmaQi, 0, 0.7);
    add(&mut w, FieldId::KarmaQi, 0, 0.7);
    assert_eq!(w.provinces[0].karma_qi, 1.0, "上限截斷");
    add(&mut w, FieldId::KarmaQi, 0, -5.0);
    assert_eq!(w.provinces[0].karma_qi, 0.0, "下限截斷");
    add(&mut w, FieldId::FaithRu, 1, 0.4);
    assert_eq!(w.provinces[1].faith, [0.0, 0.0, 0.4]);
    add(&mut w, FieldId::Population, 1, -1000.0);
    assert_eq!(w.provinces[1].population, 0.0, "人口不為負");
    // 與原本的 (x + FAMINE_KARMA).min(1.0) 逐位元相同。
    for x in [0.0f32, 0.013, 0.5, 0.981, 0.99, 1.0] {
        let mut p = prov(0, vec![]);
        p.karma_qi = x;
        add_to(&mut p, FieldId::KarmaQi, FAMINE_KARMA);
        assert_eq!(p.karma_qi.to_bits(), (x + FAMINE_KARMA).min(1.0).to_bits());
    }
    // 不存在的州、尚無儲存的維、衍生場：不動作也不 panic。
    let before = w.provinces.clone();
    add(&mut w, FieldId::KarmaQi, 99, 0.5);
    add(&mut w, FieldId::Incense, 0, 0.5);
    add(&mut w, FieldId::Turbid, 0, 0.5);
    assert_eq!(w.provinces, before);
    assert_eq!(get(&w, FieldId::Incense, 0), 0.0);
}

#[test]
fn gather_scatter_roundtrip() {
    let mut w = world(5);
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    for p in &mut w.provinces {
        p.aura = rng.gen_range(0.0..1.0);
        p.karma_qi = rng.gen_range(0.0..1.0);
        p.merit = rng.gen_range(0.0..1.0);
        p.righteous = rng.gen_range(0.0..1.0);
        p.devastation = rng.gen_range(0.0..1.0);
        p.faith = [rng.gen_range(0.0..0.3), rng.gen_range(0.0..0.3), rng.gen_range(0.0..0.3)];
    }
    let before = w.provinces.clone();
    for id in FieldId::ALL {
        let v = gather(&w, id);
        assert_eq!(v.len(), 5);
        scatter(&mut w, id, &v);
    }
    assert_eq!(w.provinces, before, "往返後州郡應逐位元相同");
    // scatter 截斷在值域內；衍生場不動作。
    scatter(&mut w, FieldId::Merit, &[2.0, -1.0, 0.5, 0.5, 0.5]);
    assert_eq!((w.provinces[0].merit, w.provinces[1].merit), (1.0, 0.0));
    let b = w.provinces.clone();
    scatter(&mut w, FieldId::Turbid, &[1.0; 5]);
    assert_eq!(w.provinces, b);
}

#[test]
fn turbid_matches_formula() {
    let mut w = world(1);
    w.provinces[0].karma_qi = 0.9;
    w.provinces[0].merit = 0.2;
    w.provinces[0].righteous = 0.4;
    assert!((get(&w, FieldId::Turbid, 0) - 0.6).abs() < 1e-6);
    w.provinces[0].karma_qi = 0.1;
    assert_eq!(get(&w, FieldId::Turbid, 0), 0.0, "下限截斷");
    assert_eq!(gather(&w, FieldId::Turbid), vec![0.0]);
}

#[test]
fn kernel_decay_relax_clamp() {
    assert_eq!(kernel::decay(&[1.0, 0.5], 0.5), vec![0.5, 0.25]);
    assert_eq!(kernel::relax(&[0.0, 1.0], &[1.0, 1.0], 0.05), vec![0.05, 1.0]);
    assert_eq!(kernel::clamp(&[-1.0, 0.5, 2.0], (0.0, 1.0)), vec![0.0, 0.5, 1.0]);
}

/// 印出 3 個種子各 1 運後的世界雜湊（接入前後比對用）：`cargo test --release -p siming-sim --lib -- --ignored --nocapture field_print_hashes`。
#[test]
#[ignore]
fn field_print_hashes() {
    for seed in [7u64, 42, 20261011] {
        let mut s = crate::Sim::new(&WorldParams::new(seed)).expect("世界生成");
        for _ in 0..4320 {
            s.step();
        }
        println!("種子 {seed} 1 運後世界雜湊 {:016x}", s.hash());
    }
}
