//! GDD 2.8「每世結算」第 2、7 步：寒暖與海平面改地形與海陸，州郡跟著重算。

use siming_sim::{landform, EventArgs, EventKind, PowerKind, Sim, Terrain, World, WorldParams, NO_PROVINCE};

/// 測試用的短開天：人口期 0 年、人物期 0 年（預設長度太慢，且測試要看的是其中一段規則）。
fn quick(seed: u64) -> WorldParams {
    let mut p = WorldParams::new(seed);
    p.pop_years = 0;
    p.prerun_years = 0;
    p
}

const SEED: u64 = 20261011;

fn sim() -> Sim {
    Sim::new(&quick(SEED)).unwrap()
}

/// 把寒暖與海平面定在相對於開局的偏移上（測試直接呼叫地形的結算，不經慢變量的週期）。
fn pin(w: &mut World, warmth: f32, sea: f32) {
    w.evolution.warmth = w.evolution.opening_warmth + warmth;
    w.evolution.sea_shift = w.evolution.opening_sea_shift + sea;
}

fn count(terrain: &[Terrain], kind: Terrain) -> usize {
    terrain.iter().filter(|&&t| t == kind).count()
}

/// 州郡與格點互相對得上：格升冪、歸屬一致、地形組成與重數相同、治所在州內、水格不屬任何州。
fn assert_consistent(w: &World) {
    let geo = &w.geo;
    let mut owned = 0usize;
    for p in &w.provinces {
        assert!(p.cells.windows(2).all(|pair| pair[0] < pair[1]), "{} 的格未升冪", p.name);
        let mut mix = [0u32; Terrain::COUNT];
        for &c in &p.cells {
            assert_eq!(geo.province_of[c as usize], p.id, "{} 的格 {c} 歸屬不符", p.name);
            assert!(!geo.terrain[c as usize].is_water(), "{} 含水格 {c}", p.name);
            mix[geo.terrain[c as usize] as usize] += 1;
        }
        assert_eq!(mix, p.terrain_mix, "{} 的地形組成", p.name);
        if !p.cells.is_empty() {
            assert!(p.cells.binary_search(&p.seat).is_ok(), "{} 的治所不在州內", p.name);
            assert!(p.habitability > 0.0, "{} 有格而宜居度為零", p.name);
        }
        owned += p.cells.len();
    }
    assert_eq!(owned, geo.province_of.iter().filter(|&&p| p != NO_PROVINCE).count(), "有格的歸屬指向不存在的州");
    // 相鄰關係對稱；陸鄰確實共享邊界；消失的州郡沒有鄰州，也沒有人、魂、勢力還記在那裡。
    let (gw, gh) = (geo.w as usize, geo.h as usize);
    for p in &w.provinces {
        for &n in p.neighbors.iter().chain(&p.sea_neighbors) {
            let other = &w.provinces[n as usize];
            assert!(!other.cells.is_empty(), "{} 的鄰州 {} 已消失", p.name, other.name);
            assert!(other.neighbors.contains(&p.id) || other.sea_neighbors.contains(&p.id), "{} 與 {} 的相鄰不對稱", p.name, other.name);
        }
        for &n in &p.neighbors {
            let touches = p.cells.iter().any(|&c| {
                let (x, y) = (c as usize % gw, c as usize / gw);
                let mut around = vec![y * gw + (x + 1) % gw, y * gw + (x + gw - 1) % gw];
                if y > 0 {
                    around.push((y - 1) * gw + x);
                }
                if y + 1 < gh {
                    around.push((y + 1) * gw + x);
                }
                around.into_iter().any(|nb| geo.province_of[nb] == n)
            });
            assert!(touches, "{} 與陸鄰 {} 沒有共享邊界", p.name, w.provinces[n as usize].name);
        }
        assert_eq!(p.lost, p.cells.is_empty(), "{} 的消失旗標", p.name);
        if p.cells.is_empty() {
            assert!(p.neighbors.is_empty() && p.sea_neighbors.is_empty(), "消失的 {} 仍有鄰州", p.name);
            assert_eq!(p.population, 0.0, "消失的 {} 仍有人口", p.name);
        }
    }
    let gone = |province: u16| w.provinces[province as usize].cells.is_empty();
    let dry = |cell: u32| !geo.terrain[cell as usize].is_water();
    for person in &w.persons {
        assert!(!gone(person.province) && !gone(person.home), "{} 還記在消失的州郡", person.id);
        assert!(dry(person.cell), "{} 在水裡", person.id);
    }
    for soul in &w.souls {
        assert!(!gone(soul.last_place.0) && dry(soul.last_place.1), "魂 {} 的位置在消失的州郡或水裡", soul.id);
    }
    for power in &w.powers {
        assert!(!gone(power.seat_province) && dry(power.seat_cell), "{} 的駐地在消失的州郡或水裡", power.name);
        let PowerKind::Sect(sect) = &power.kind;
        let on_node = sect.seat_node.is_none_or(|n| geo.nodes[n as usize].cell == power.seat_cell);
        assert!(on_node || power.fallen.is_some(), "{} 的山門不在所佔的洞天福地上", power.name);
    }
    for (c, &t) in geo.terrain.iter().enumerate() {
        if t.is_water() {
            assert_eq!(geo.province_of[c], NO_PROVINCE, "水格 {c} 仍屬某州");
            assert_eq!(geo.river[c], 0, "水格 {c} 仍有河流");
        }
    }
    for (f, &t) in w.fine.terrain.iter().enumerate() {
        if t.is_water() {
            assert_eq!(w.fine.river[f], 0, "細格水格 {f} 仍有河流");
        }
    }
}

/// 一世一步：這一世改了地形的細格，結算前在兩格（細格，合遊戲格一格）之內必有不同的地形。
fn assert_one_step(before: &[Terrain], after: &[Terrain], fw: usize, fh: usize) {
    for c in 0..before.len() {
        if before[c] == after[c] {
            continue;
        }
        let (x, y) = ((c % fw) as i64, (c / fw) as i64);
        let mut near = false;
        for dy in -2i64..=2 {
            for dx in -2i64..=2 {
                let ny = y + dy;
                if ny < 0 || ny >= fh as i64 {
                    continue;
                }
                let nx = (x + dx).rem_euclid(fw as i64);
                near |= before[ny as usize * fw + nx as usize] != before[c];
            }
        }
        assert!(near, "細格 {c} 不在地形邊界兩格內卻改了（{:?} → {:?}）", before[c], after[c]);
    }
}

#[test]
fn gdd_2_8_land_stays_put_while_climate_stays_at_its_opening_value() {
    let mut sim = sim();
    let before = sim.world().clone();
    for _ in 0..5 {
        let shift = landform::settle(sim.world_mut());
        assert_eq!(shift, landform::LandShift::default());
    }
    let w = sim.world();
    assert_eq!(w.fine, before.fine);
    assert_eq!(w.geo, before.geo);
    assert_eq!(w.provinces, before.provinces);
}

#[test]
fn gdd_2_8_cold_spreads_snow_and_bares_the_shelf_one_step_per_shi() {
    let mut sim = sim();
    let (fw, fh) = (sim.world().fine.w as usize, sim.world().fine.h as usize);
    let snow_at_opening = count(&sim.world().geo.terrain, Terrain::Snow);
    let sea_at_opening = count(&sim.world().geo.terrain, Terrain::Sea);
    pin(sim.world_mut(), -1.0, -0.03);
    let mut emerged = 0;
    for shi in 0..8 {
        let before = sim.world().fine.terrain.clone();
        let shift = landform::settle(sim.world_mut());
        assert!(shift.fine > 0, "第 {shi} 世沒有變動");
        assert_eq!(shift.drowned, 0, "海退時沒有陸格沒入");
        emerged += shift.emerged;
        assert_one_step(&before, &sim.world().fine.terrain, fw, fh);
        assert_consistent(sim.world());
    }
    let w = sim.world();
    assert!(count(&w.geo.terrain, Terrain::Snow) > snow_at_opening, "大寒八世，雪原沒有擴張");
    assert!(emerged > 0 && count(&w.geo.terrain, Terrain::Sea) < sea_at_opening, "海退八世，沒有新地浮出");
    assert!(w.geo.terrain.iter().zip(&w.geo.province_of).any(|(t, &p)| !t.is_water() && p != NO_PROVINCE));
}

#[test]
fn gdd_2_8_new_land_joins_the_adjacent_province_and_starts_as_swamp() {
    let mut sim = sim();
    let origin = sim.world().geo.terrain.clone();
    let origin_fine = sim.world().fine.terrain.clone();
    pin(sim.world_mut(), 0.0, -0.03);
    landform::settle(sim.world_mut());
    let w = sim.world();
    let fresh: Vec<usize> =
        (0..origin_fine.len()).filter(|&f| origin_fine[f] == Terrain::Sea && w.fine.terrain[f] != Terrain::Sea).collect();
    assert!(!fresh.is_empty(), "海退一世沒有細格浮出");
    assert!(fresh.iter().all(|&f| w.fine.terrain[f] == Terrain::Swamp), "新地先是沼澤");
    let joined = (0..origin.len())
        .filter(|&c| origin[c] == Terrain::Sea && !w.geo.terrain[c].is_water() && w.geo.province_of[c] != NO_PROVINCE)
        .count();
    assert!(joined > 0, "浮出的遊戲格沒有併入州郡");
    assert_consistent(w);
}

#[test]
fn gdd_2_8_rising_sea_drowns_the_coast_and_provinces_lose_cells() {
    let mut sim = sim();
    let land_at_opening: usize = sim.world().provinces.iter().map(|p| p.cells.len()).sum();
    let habitability_at_opening: f64 = sim.world().provinces.iter().map(|p| p.habitability).sum();
    let (fw, fh) = (sim.world().fine.w as usize, sim.world().fine.h as usize);
    pin(sim.world_mut(), 1.0, 0.03);
    let mut drowned = 0;
    for _ in 0..8 {
        let before = sim.world().fine.terrain.clone();
        let shift = landform::settle(sim.world_mut());
        drowned += shift.drowned;
        assert_one_step(&before, &sim.world().fine.terrain, fw, fh);
        assert_consistent(sim.world());
    }
    let w = sim.world();
    assert!(drowned > 0, "海進八世，沒有陸格沒入");
    assert!(w.provinces.iter().map(|p| p.cells.len()).sum::<usize>() < land_at_opening, "州郡的格沒有減少");
    assert_ne!(w.provinces.iter().map(|p| p.habitability).sum::<f64>(), habitability_at_opening, "宜居度沒有重算");
    for river in &w.geo.rivers {
        assert!(river.cells.last().is_none_or(|&c| w.geo.terrain[c as usize] != Terrain::Sea), "{} 的尾端在海裡", river.name);
    }
}

#[test]
fn gdd_2_8_the_map_returns_once_the_climate_does() {
    let mut sim = sim();
    let before = sim.world().clone();
    pin(sim.world_mut(), -1.0, -0.03);
    for _ in 0..6 {
        landform::settle(sim.world_mut());
    }
    assert_ne!(sim.world().geo.terrain, before.geo.terrain);
    pin(sim.world_mut(), 0.0, 0.0);
    let mut shis = 0;
    while landform::settle(sim.world_mut()).fine > 0 {
        shis += 1;
        assert!(shis < 200, "回到開局的寒暖後 200 世還沒停");
    }
    let w = sim.world();
    let mut stuck = std::collections::BTreeMap::new();
    for (now, origin) in w.fine.terrain.iter().zip(&before.fine.terrain) {
        if now != origin {
            *stuck.entry((*now as u8, *origin as u8)).or_insert(0u32) += 1;
        }
    }
    assert!(stuck.is_empty(), "沒有回到生成時地形的細格（目前, 生成時）：{stuck:?}");
    assert!(w.fine == before.fine, "細格的河流或流量沒有復原");
    assert!(w.geo == before.geo, "遊戲格沒有復原");
    assert!(w.provinces == before.provinces, "州郡沒有復原");
}

#[test]
fn gdd_2_8_terrain_follows_the_slow_variables_during_play() {
    let mut sim = sim();
    let origin = sim.world().fine.terrain.clone();
    // 把寒暖的兩個週期拉到極長、相位定在谷底：每世結算的寒暖都是 −1。
    let evolution = &mut sim.world_mut().evolution;
    evolution.warmth_periods = [u32::MAX, u32::MAX];
    evolution.warmth_phases = [0.75, 0.75];
    for _ in 0..3 * 360 {
        sim.step();
    }
    let w = sim.world();
    assert!(w.evolution.warmth_delta() < -0.3, "寒暖偏移 {}", w.evolution.warmth_delta());
    assert_ne!(w.fine.terrain, origin, "三世大寒，地形沒有變");
    assert_consistent(w);
}

#[test]
fn gdd_2_8_a_province_wholly_under_the_sea_is_gone_and_its_people_move_on() {
    let mut sim = sim();
    pin(sim.world_mut(), 1.0, 0.03);
    let mut lost = Vec::new();
    let mut seats = 0;
    for _ in 0..30 {
        let people_before: f64 = sim.world().provinces.iter().map(|p| p.population).sum();
        let shift = landform::settle(sim.world_mut());
        let people_after: f64 = sim.world().provinces.iter().map(|p| p.population).sum();
        let perished: f64 = shift.lost.iter().map(|l| f64::from(l.perished)).sum();
        assert!((people_before - people_after - perished).abs() <= shift.lost.len() as f64, "人口只在鄰州容不下時折損");
        for &(province, from, to) in &shift.seats {
            let p = &sim.world().provinces[province as usize];
            assert_eq!(p.seat, to);
            assert!(sim.world().geo.terrain[from as usize].is_water(), "治所沒被淹卻遷了");
        }
        seats += shift.seats.len();
        for l in &shift.lost {
            assert!(l.refuge.is_none_or(|r| !sim.world().provinces[r as usize].lost), "收容的州郡也消失了");
        }
        lost.extend(shift.lost);
        assert_consistent(sim.world());
    }
    assert!(seats > 0, "海進三十世，沒有治所遷移");
    assert!(!lost.is_empty(), "海進三十世，這個種子應有州郡全數沒入");
    let w = sim.world();
    for l in &lost {
        let p = &w.provinces[l.province as usize];
        assert!(p.cells.is_empty() && p.habitability == 0.0 && p.capacity == 0.0);
        assert_eq!(p.id, l.province, "消失的州郡留在原位，ID 不變");
    }
}

#[test]
fn gdd_2_8_land_changes_enter_the_ledger() {
    let mut sim = sim();
    // 寒暖定在峰頂：海平面一世追一半，海岸跟著退。
    let evolution = &mut sim.world_mut().evolution;
    evolution.warmth_periods = [u32::MAX, u32::MAX];
    evolution.warmth_phases = [0.25, 0.25];
    for _ in 0..6 * 360 {
        sim.step();
    }
    let w = sim.world();
    assert!(w.evolution.sea_delta() > 0.02, "海平面偏移 {}", w.evolution.sea_delta());
    let moved: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::SeatMoved).collect();
    assert!(!moved.is_empty(), "海進六世，因果帳裡沒有治所遷移");
    // 這個種子在這六世裡有州郡全數沒入：之後的年度人口、投生與宗門照常推演（上面每一步都跑過）。
    let lost: Vec<_> = w.ledger.events.iter().filter(|e| e.kind == EventKind::ProvinceLost).collect();
    assert!(!lost.is_empty(), "海進六世，這個種子應有州郡消失");
    assert!(lost.iter().all(|e| e.province.is_some_and(|p| w.provinces[p as usize].lost)));
    assert!(lost.iter().any(|e| e.tick < w.tick - 10_800), "州郡消失後至少又推演了一世");
    for event in &moved {
        assert_eq!(event.tick % 10_800, 0, "只在每世結算時發生");
        let EventArgs::Seat { from, to } = event.args else { panic!("治所遷移的參數") };
        assert_ne!(from, to);
        assert_eq!(event.cell, Some(to));
        assert!(event.province.is_some());
    }
    for event in w.ledger.events.iter().filter(|e| e.kind == EventKind::LandLinked) {
        let EventArgs::Link { other, .. } = event.args else { panic!("陸橋的參數") };
        assert!(event.province.is_some_and(|p| p < other), "陸橋記在 ID 小的那一州");
    }
    assert_consistent(w);
}

/// 與 `cell` 相距兩格（細格）之內有沒有海。
fn sea_within_two(terrain: &[Terrain], fw: usize, fh: usize, cell: usize) -> bool {
    let (x, y) = ((cell % fw) as i64, (cell / fw) as i64);
    (-2i64..=2).any(|dy| {
        let ny = y + dy;
        ny >= 0
            && ny < fh as i64
            && (-2i64..=2).any(|dx| terrain[ny as usize * fw + (x + dx).rem_euclid(fw as i64) as usize] == Terrain::Sea)
    })
}

#[test]
fn gdd_2_8_bared_shelf_sinks_back_from_the_shoreline() {
    let mut sim = sim();
    let (fw, fh) = (sim.world().fine.w as usize, sim.world().fine.h as usize);
    pin(sim.world_mut(), -1.0, -0.03);
    for _ in 0..6 {
        landform::settle(sim.world_mut());
    }
    pin(sim.world_mut(), 0.0, 0.0);
    let before = sim.world().fine.terrain.clone();
    landform::settle(sim.world_mut());
    let after = &sim.world().fine.terrain;
    let sunk: Vec<usize> = (0..before.len()).filter(|&c| before[c] != Terrain::Sea && after[c] == Terrain::Sea).collect();
    assert!(sunk.len() > 1000, "海平面回升一世，只沉回 {} 個細格", sunk.len());
    // 整片乾掉的內陸小海灣由生成時的岸邊回填，不在此限；它們只佔零頭。
    let inland = sunk.iter().filter(|&&c| !sea_within_two(&before, fw, fh, c)).count();
    assert!(inland * 100 <= sunk.len(), "{inland}/{} 個細格不貼海就沉了", sunk.len());
}

#[test]
fn gdd_2_8_terrain_returns_after_the_sea_recedes_but_seats_stay_moved() {
    let mut sim = sim();
    let before = sim.world().clone();
    pin(sim.world_mut(), 1.0, 0.03);
    let mut moved = Vec::new();
    for _ in 0..3 {
        moved.extend(landform::settle(sim.world_mut()).seats);
    }
    assert!(!moved.is_empty());
    pin(sim.world_mut(), 0.0, 0.0);
    let mut shis = 0;
    loop {
        let shift = landform::settle(sim.world_mut());
        assert!(shift.seats.is_empty(), "海退時治所不搬回去");
        if shift.fine == 0 {
            break;
        }
        shis += 1;
        assert!(shis < 200, "回到開局的寒暖後 200 世還沒停");
    }
    let w = sim.world();
    assert!(w.fine == before.fine, "細格沒有復原");
    assert!(w.geo.terrain == before.geo.terrain && w.geo.river == before.geo.river, "遊戲格沒有復原");
    assert!(w.geo.rivers == before.geo.rivers, "江河的幹流沒有復原");
    for &(province, from, _) in &moved {
        let p = &w.provinces[province as usize];
        assert!(p.lost || p.seat != from || moved.iter().filter(|m| m.0 == province).count() > 1, "{} 的治所搬回去了", p.name);
    }
    assert_consistent(w);
}

#[test]
fn gdd_6_5_a_save_replays_across_a_shi_of_changing_land() {
    let mut sim = sim();
    let origin = sim.world().fine.terrain.clone();
    for _ in 0..2 * 360 {
        sim.step();
    }
    assert_ne!(sim.world().fine.terrain, origin, "這個種子開局兩世內地形應有變動");
    let loaded = Sim::load(&sim.save()).unwrap();
    assert_eq!(loaded.hash(), sim.hash());
    assert!(loaded.world().fine == sim.world().fine && loaded.world().provinces == sim.world().provinces);
}
