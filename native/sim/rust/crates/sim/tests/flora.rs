//! GDD 2.8「每世結算」第 3 步：草木——墾殖、復林、沙化與回復。

// 斷言寫成「不小於」的否定式：沒有可比的格時兩邊是 NaN，這時要算通過。
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use siming_sim::{flora, landform, EventKind, Sim, Terrain, World, WorldParams};

/// 測試用的短開天：人口期 `pop_years` 年、人物期 0 年（預設長度太慢，且測試要看的是其中一段規則）。
fn quick(seed: u64, pop_years: u32) -> WorldParams {
    let mut p = WorldParams::new(seed);
    p.pop_years = pop_years;
    p.prerun_years = 0;
    p
}

const SEED: u64 = 20261011;

/// 開天做完的世界。人口期的氣候一直在變，地形追氣候有時間差（每次結算只動一部分），
/// 所以先把地形結算到與氣候一致，後面的測試才看得到草木自己的變動。
fn sim() -> Sim {
    sim_after(40)
}

/// 人口期長到各州人口超過開局承載的世界：草木在人口期內已墾殖、沙化。
fn sim_long() -> Sim {
    sim_after(2000)
}

fn sim_after(pop_years: u32) -> Sim {
    let mut sim = Sim::new(&quick(SEED, pop_years)).unwrap();
    let w = sim.world_mut();
    for _ in 0..200 {
        if landform::settle(w).fine == 0 {
            break;
        }
    }
    assert_eq!(landform::settle(w).fine, 0, "地形沒有結算到與氣候一致");
    sim
}

/// 草木有在管的州（有格、未沉沒、開局承載為正）。
fn active(w: &World, i: usize) -> bool {
    let p = &w.provinces[i];
    !p.lost && !p.cells.is_empty() && w.flora.opening_capacity[i] > 0.0
}

/// 把各州的墾殖度定在 `level`：人口設為開局承載的 `level` 倍，墾殖度就停在那裡。
fn press(w: &mut World, level: f32) {
    for i in 0..w.provinces.len() {
        w.flora.cultivation[i] = level;
        w.provinces[i].population = w.flora.opening_capacity[i] * f64::from(level);
    }
}

/// 草木結算一世，地形照旗標改。
fn shi(w: &mut World) -> flora::FloraShift {
    let shift = flora::settle(w);
    landform::settle_with(w, &shift.touched);
    shift
}

fn cheb(gw: usize, a: u32, b: u32) -> u32 {
    let (a, b) = (a as usize, b as usize);
    let dx = (a % gw).abs_diff(b % gw);
    dx.min(gw - dx).max((a / gw).abs_diff(b / gw)) as u32
}

/// 遊戲格內的細格。
fn block(w: &World, cell: u32) -> Vec<usize> {
    let (scale, fw, gw) = (w.fine.scale as usize, w.fine.w as usize, w.geo.w as usize);
    let (x, y) = (cell as usize % gw, cell as usize / gw);
    (0..scale * scale).map(|i| (y * scale + i / scale) * fw + x * scale + i % scale).collect()
}

fn moisture(w: &World, cells: &[u32]) -> f32 {
    let sum: f32 = cells.iter().flat_map(|&c| block(w, c)).map(|f| f32::from(w.fine.moisture[f]) / 255.0).sum();
    sum / (cells.len() * block(w, 0).len()) as f32
}

fn flags(v: &[bool]) -> usize {
    v.iter().filter(|&&x| x).count()
}

/// 開天時天地間沒有人，草木的狀態在人口期的第一次每世結算建立：承載記為那時的承載，墾殖度之後每世追著
/// 人口 ÷ 開局承載走；人口期內人口多起來的州已經開始墾殖。
#[test]
fn gdd_2_8_flora_is_built_during_the_population_period_and_tracks_population() {
    let sim = sim_long();
    let w = sim.world();
    assert_eq!(w.flora.cultivation.len(), w.provinces.len());
    assert_eq!(w.flora.opening_capacity.len(), w.provinces.len());
    assert_eq!(w.flora.cleared.len(), w.geo.terrain.len());
    assert_eq!(w.flora.sanded.len(), w.geo.terrain.len());
    let mut tracking = 0;
    for (i, p) in w.provinces.iter().enumerate() {
        assert!((0.0..=1.0).contains(&w.flora.cultivation[i]), "州 {i} 墾殖度 {}", w.flora.cultivation[i]);
        if !active(w, i) {
            continue;
        }
        // 墾殖度每世只走四分之一的路，所以不等於當下的比值；但人口多的州墾殖度不會是 0。
        let ratio = (p.population / w.flora.opening_capacity[i]).clamp(0.0, 1.0);
        if ratio > 0.5 {
            assert!(w.flora.cultivation[i] > 0.05, "州 {i}：人口占開局承載 {ratio:.2}，墾殖度卻只有 {}", w.flora.cultivation[i]);
            tracking += 1;
        }
    }
    assert!(tracking > 0, "人口期 2000 年後沒有任何州人口過半");
}

#[test]
fn gdd_2_8_cultivation_approaches_a_quarter_per_shi_against_opening_capacity() {
    let mut sim = sim();
    let w = sim.world_mut();
    let before = w.flora.cultivation.clone();
    for (i, p) in w.provinces.iter_mut().enumerate() {
        // 承載翻倍不影響墾殖度：分母是開局時的承載。
        p.capacity *= 2.0;
        p.population = w.flora.opening_capacity[i] * if i % 2 == 0 { 0.9 } else { 0.1 };
    }
    flora::settle(w);
    for (i, &old) in before.iter().enumerate().filter(|(i, _)| active(w, *i)) {
        let target = if i % 2 == 0 { 0.9 } else { 0.1 };
        assert!((w.flora.cultivation[i] - (old + (target - old) * 0.25)).abs() < 1e-5, "州 {i}");
    }
}

#[test]
fn gdd_2_8_crowded_provinces_clear_the_forest_nearest_the_seat() {
    let mut sim = sim();
    let w = sim.world_mut();
    press(w, 0.9);
    let gw = w.geo.w as usize;
    let before = w.clone();
    let shift = shi(w);
    assert!(shift.cleared > 50 && shift.regrown == 0, "墾了 {} 格", shift.cleared);
    for (p, old) in w.provinces.iter().zip(&before.provinces) {
        let standing = |c: &u32| before.geo.terrain[*c as usize] == Terrain::Forest;
        // 靈氣達門檻的森林墾不動。
        let guarded: Vec<u32> = old.cells.iter().copied().filter(|c| standing(c) && before.geo.aura_cell[*c as usize] >= 0.6).collect();
        assert!(guarded.iter().all(|&c| !w.flora.cleared[c as usize]), "{}：靈氣護著的森林被墾了", p.name);
        let forest: Vec<u32> = old.cells.iter().copied().filter(|c| standing(c) && !guarded.contains(c)).collect();
        let cleared: Vec<u32> = forest.iter().copied().filter(|&c| w.flora.cleared[c as usize]).collect();
        let expect = if forest.is_empty() { 0 } else { ((forest.len() as f32 * 0.05).ceil() as usize).max(1) };
        assert_eq!(cleared.len(), expect, "{}：森林 {} 格", p.name, forest.len());
        // 離治所近的先墾。
        let far = cleared.iter().map(|&c| cheb(gw, old.seat, c)).max();
        let kept = forest.iter().filter(|c| !cleared.contains(c)).map(|&c| cheb(gw, old.seat, c)).min();
        assert!(far.zip(kept).is_none_or(|(far, kept)| far <= kept), "{}：墾了遠的、留了近的", p.name);
        for &c in &cleared {
            assert_ne!(w.geo.terrain[c as usize], Terrain::Forest);
            assert!(block(w, c).iter().all(|&f| w.fine.terrain[f] != Terrain::Forest), "格內還有森林的細格");
        }
        if !cleared.is_empty() {
            assert!(p.terrain_mix[Terrain::Forest as usize] < old.terrain_mix[Terrain::Forest as usize]);
            assert!(p.habitability > old.habitability, "{}：墾出平原，宜居度應上升", p.name);
        }
    }
    assert_eq!(flags(&w.flora.sanded), shift.sanded as usize);
}

#[test]
fn gdd_2_8_cleared_land_stays_cleared_while_the_climate_holds() {
    let mut sim = sim();
    let w = sim.world_mut();
    press(w, 0.9);
    shi(w);
    let cleared: Vec<u32> = (0..w.flora.cleared.len() as u32).filter(|&c| w.flora.cleared[c as usize]).collect();
    let after = w.fine.terrain.clone();
    for _ in 0..3 {
        let shift = landform::settle(w);
        assert_eq!(shift.fine, 0, "氣候沒變，地形不該自己動");
    }
    assert!(w.fine.terrain == after);
    assert!(cleared.iter().all(|&c| w.geo.terrain[c as usize] != Terrain::Forest));
}

#[test]
fn gdd_2_8_forest_grows_back_when_people_leave_and_the_map_returns() {
    let mut sim = sim();
    let w = sim.world_mut();
    let origin = w.clone();
    press(w, 0.9);
    for _ in 0..4 {
        shi(w);
    }
    let cleared = flags(&w.flora.cleared);
    assert!(cleared > 200);
    press(w, 0.1);
    let gw = w.geo.w as usize;
    let before = w.flora.cleared.clone();
    let shift = shi(w);
    assert!(shift.regrown > 0 && shift.cleared == 0);
    assert_eq!(flags(&w.flora.cleared) + shift.regrown as usize, cleared);
    for p in &w.provinces {
        // 離治所遠的先長回來。
        let was: Vec<u32> = p.cells.iter().copied().filter(|&c| before[c as usize]).collect();
        let back = was.iter().filter(|&&c| !w.flora.cleared[c as usize]).map(|&c| cheb(gw, p.seat, c)).min();
        let still = was.iter().filter(|&&c| w.flora.cleared[c as usize]).map(|&c| cheb(gw, p.seat, c)).max();
        assert!(back.zip(still).is_none_or(|(back, still)| back >= still), "{}：近的先長回來了", p.name);
    }
    let mut shis = 0;
    while flags(&w.flora.cleared) + flags(&w.flora.sanded) > 0 {
        shi(w);
        shis += 1;
        assert!(shis < 200, "200 世還沒長回來");
    }
    assert!(w.fine == origin.fine, "細格沒有復原");
    assert!(w.geo.terrain == origin.geo.terrain, "遊戲格沒有復原");
    for (p, old) in w.provinces.iter().zip(&origin.provinces) {
        assert_eq!(p.terrain_mix, old.terrain_mix);
        assert!((p.habitability - old.habitability).abs() < 1e-6);
    }
}

#[test]
fn gdd_2_8_aura_brings_the_forest_back_whatever_the_crowd() {
    let mut sim = sim();
    let w = sim.world_mut();
    let origin = w.clone();
    press(w, 0.9);
    for _ in 0..4 {
        shi(w);
    }
    let cleared = flags(&w.flora.cleared);
    assert!(cleared > 200);
    // 靈氣漲到處處都過門檻：人口照舊擁擠，森林不再被墾，已墾的每世長回一成，靈氣高的先長。
    let aura: Vec<f32> = (0..w.geo.aura_cell.len()).map(|c| 0.6 + (c % 7) as f32 * 0.1).collect();
    w.geo.aura_cell = aura.clone();
    let before = w.flora.cleared.clone();
    let shift = shi(w);
    assert_eq!(shift.cleared, 0, "靈氣護著，墾不動");
    assert!(shift.revived > 0 && shift.revived == shift.regrown);
    for p in &w.provinces {
        let was: Vec<u32> = p.cells.iter().copied().filter(|&c| before[c as usize]).collect();
        let back: Vec<u32> = was.iter().copied().filter(|&c| !w.flora.cleared[c as usize]).collect();
        let expect = if was.is_empty() { 0 } else { ((was.len() as f32 * 0.1).ceil() as usize).max(1) };
        assert_eq!(back.len(), expect, "{}：已墾 {} 格", p.name, was.len());
        let weakest_back = back.iter().map(|&c| aura[c as usize]).fold(f32::NAN, f32::min);
        let strongest_left = was.iter().filter(|c| !back.contains(c)).map(|&c| aura[c as usize]).fold(f32::NAN, f32::max);
        assert!(!(weakest_back < strongest_left), "{}：靈氣低的先長回來了", p.name);
    }
    let mut shis = 0;
    while flags(&w.flora.cleared) > 0 {
        assert_eq!(shi(w).cleared, 0);
        shis += 1;
        assert!(shis < 200, "200 世還沒長回來");
    }
    assert!(w.geo.terrain.iter().zip(&origin.geo.terrain).all(|(a, b)| (*a == Terrain::Forest) == (*b == Terrain::Forest)), "森林沒有復原");
    // 靈氣不到門檻的已墾格不會自己長回來。
    w.geo.aura_cell = origin.geo.aura_cell.clone();
    let shift = shi(w);
    assert!(shift.cleared > 0 && shift.revived == 0);
}

#[test]
fn gdd_2_8_dry_crowded_provinces_turn_to_sand_up_to_half_and_heal() {
    let mut sim = sim();
    let w = sim.world_mut();
    let origin = w.clone();
    press(w, 0.9);
    let dry: Vec<bool> = w.provinces.iter().map(|p| moisture(w, &p.cells) < 0.25).collect();
    assert!(dry.iter().any(|&d| d) && dry.iter().any(|&d| !d));
    let shift = shi(w);
    assert!(shift.sanded > 0);
    let mut started = 0;
    for (p, &dry) in w.provinces.iter().zip(&dry) {
        let sanded: Vec<u32> = p.cells.iter().copied().filter(|&c| w.flora.sanded[c as usize]).collect();
        if !dry {
            assert!(sanded.is_empty(), "{} 不乾，不該沙化", p.name);
            continue;
        }
        started += usize::from(!sanded.is_empty());
        assert_eq!(shift.desertified.contains(&p.id), !sanded.is_empty());
        for &c in &sanded {
            assert_eq!(origin.geo.terrain[c as usize], Terrain::Plains, "只有平原沙化");
            // 格內其餘的細格若是別種地形，遊戲格不一定成為沙漠，但不再是平原。
            assert_ne!(w.geo.terrain[c as usize], Terrain::Plains);
            assert!(block(w, c).iter().all(|&f| w.fine.terrain[f] != Terrain::Plains), "格內還有平原的細格");
        }
        // 最乾的先沙化。
        let driest = |cells: &[u32]| cells.iter().map(|&c| moisture(w, &[c])).fold(f32::NAN, f32::max);
        let kept: Vec<u32> =
            p.cells.iter().copied().filter(|&c| origin.geo.terrain[c as usize] == Terrain::Plains && !sanded.contains(&c)).collect();
        let wettest_sanded = driest(&sanded);
        let driest_kept = kept.iter().map(|&c| moisture(w, &[c])).fold(f32::NAN, f32::min);
        assert!(!(wettest_sanded > driest_kept), "{}：濕的先沙化了", p.name);
    }
    assert!(started > 0);
    // 沙化到州內平原的一半為止。
    for _ in 0..80 {
        let shift = shi(w);
        assert!(shift.desertified.is_empty(), "已在沙化的州不再記一次");
    }
    for (p, &dry) in w.provinces.iter().zip(&dry).filter(|(p, _)| active(w, p.id as usize)) {
        let sanded = p.cells.iter().filter(|&&c| w.flora.sanded[c as usize]).count();
        let plains = p.cells.iter().filter(|&&c| w.geo.terrain[c as usize] == Terrain::Plains).count();
        assert!(sanded <= plains + 1, "{}：沙化 {sanded} 格、平原剩 {plains} 格", p.name);
        assert!(!dry || plains == 0 || sanded > 0 || p.cells.is_empty());
    }
    press(w, 0.1);
    let mut shis = 0;
    while flags(&w.flora.cleared) + flags(&w.flora.sanded) > 0 {
        let shift = shi(w);
        assert_eq!(shift.sanded + shift.cleared, 0);
        shis += 1;
        assert!(shis < 300, "300 世還沒回復");
    }
    assert!(w.fine == origin.fine, "細格沒有復原");
}

#[test]
fn gdd_2_8_flora_changes_enter_the_ledger_and_replay() {
    // 不手動結算地形：存檔重播從種子重跑，世界得是純粹由 step 走出來的。
    let mut sim = Sim::new(&quick(SEED, 2000)).unwrap();
    // 人口期已把人口養到超過開局承載，接著九世內仍在墾殖。
    for _ in 0..9 * 360 {
        sim.step();
    }
    let w = sim.world();
    assert!(flags(&w.flora.cleared) > 0, "九世內沒有墾殖");
    assert!(w.flora.cultivation.iter().all(|c| (0.0..=1.0).contains(c)));
    for (c, &cleared) in w.flora.cleared.iter().enumerate() {
        assert!(!(cleared && w.geo.terrain[c] == Terrain::Forest), "已墾的格 {c} 還是森林");
    }
    for event in w.ledger.events.iter().filter(|e| matches!(e.kind, EventKind::ForestCleared | EventKind::Desertified)) {
        assert_eq!(event.tick % 360, 0, "只在每世結算時發生");
        let p = &w.provinces[event.province.expect("掛在州上") as usize];
        if event.kind == EventKind::Desertified {
            assert!(p.lost || p.cells.iter().any(|&c| w.flora.sanded[c as usize]));
        }
    }
    let loaded = Sim::load(&sim.save()).unwrap();
    assert_eq!(loaded.hash(), sim.hash());
    assert!(loaded.world().flora == sim.world().flora);
}
