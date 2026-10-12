//! GDD 2.8「每世結算」第 2、7 步：寒暖與海平面改地形與海陸，連帶重算遊戲格與州郡。
//!
//! 演變在細格（GDD 2.2「地理解析度」）上進行，遊戲格照世界生成的降採樣規則由細格推得。
//! 生成時的高度、溫度、濕度與海平面是基準，不被覆寫；變化量以開局那一年為零點
//! （[`Evolution::warmth_delta`]、[`Evolution::sea_delta`]），所以變化量為零時地圖一格都不動，
//! 回到零之後地圖會一世一步地回到生成時的樣子。

use crate::evolution::Evolution;
use crate::flora::Flora;
use crate::model::PowerKind;
use crate::params::*;
use crate::sim::World;
use crate::types::{FineGeography, ProvinceId, Terrain, NO_PROVINCE};
use crate::worldgen::grid::{components, nb4, nb8, Sphere};
use crate::worldgen::provinces::{adjacency, cell_habitability};

/// 生成時的地形（效能快取之外也是基準：不納入雜湊與存檔，讀檔重播時在開天當下重建）。
#[derive(Debug, Clone)]
pub struct LandBasis {
    /// 細格：生成時的地形、河流等級與流量。
    terrain: Vec<Terrain>,
    river: Vec<u8>,
    flow: Vec<u8>,
    /// 細格：以量化後的高度、溫度、濕度在變化量為零時判出的地形。與目前變化量下的判定相同時，
    /// 目標取生成時的地形——量化誤差與去斑造成的出入因此不會讓地圖自己動起來。
    base: Vec<Terrain>,
    /// 遊戲格：生成時的地形與河流等級；格內細格全數回到生成時的樣子，遊戲格也原樣取回。
    cell_terrain: Vec<Terrain>,
    cell_river: Vec<u8>,
    /// 遊戲格：生成時所屬的州郡。
    cell_province: Vec<ProvinceId>,
    /// 細格：生成時海格所屬的連通分量（八鄰）；不是海的格不用。
    sea_part: Vec<u32>,
    sea_parts: usize,
    /// 細格：生成時不是海的格所屬的連通分量（八鄰）；海格不用。
    land_part: Vec<u32>,
    land_parts: usize,
    /// 生成時各條江河的幹流。
    rivers: Vec<Vec<u32>>,
}

/// 一世之內地形的變動。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LandShift {
    /// 改了地形的細格數。
    pub fine: u32,
    /// 改了地形或河流等級的遊戲格，升冪。
    pub cells: Vec<u32>,
    /// 沒入水中、浮出水面的遊戲格數。
    pub drowned: u32,
    pub emerged: u32,
    /// 治所遷移：州、原治所格、新治所格。
    pub seats: Vec<(ProvinceId, u32, u32)>,
    /// 全數沒入海中而消失的州郡。
    pub lost: Vec<LostProvince>,
    /// 陸鄰的變動：兩州（ID 小的在前）成為陸鄰（真）或不再是陸鄰（假）。
    pub links: Vec<(ProvinceId, ProvinceId, bool)>,
}

/// 一個消失的州郡：人口移往鄰州，容不下的折損。
#[derive(Debug, Clone, PartialEq)]
pub struct LostProvince {
    pub province: ProvinceId,
    /// 收容最多人的鄰州；沒有鄰州時為 `None`。
    pub refuge: Option<ProvinceId>,
    pub perished: u32,
}

/// GDD 2.2 步驟 5 的地形判定（單格）。海陸以目前的海平面 `sea` 判，陸地高度以生成時的海平面
/// `sea0` 算——山脈與丘陵不隨海平面移動。
fn classify(h: f32, sea: f32, sea0: f32, lake: bool, t: f32, q: f32) -> Terrain {
    if h < sea {
        return Terrain::Sea;
    }
    if lake {
        return Terrain::Lake;
    }
    let lh = (h - sea0) / (1.0 - sea0);
    if lh > WG_MOUNTAIN_H {
        Terrain::Mountain
    } else if t < WG_SNOW_T {
        Terrain::Snow
    } else if lh > WG_HILL_H {
        Terrain::Hills
    } else if q < WG_DESERT_Q && t > WG_DESERT_T {
        Terrain::Desert
    } else if q > WG_SWAMP_Q && lh < WG_SWAMP_H {
        Terrain::Swamp
    } else if q > WG_FOREST_Q {
        Terrain::Forest
    } else {
        Terrain::Plains
    }
}

/// 細格在寒暖偏移 `warmth`、海平面偏移 `sea` 下的地形判定；`lake` 為生成時是否為湖。
fn classify_fine(fine: &FineGeography, c: usize, lake: bool, warmth: f32, sea: f32) -> Terrain {
    let h = fine.height[c] as f32 / 65535.0;
    let t = fine.temperature[c] as f32 / 255.0 + EVO_WARMTH_TEMPERATURE * warmth;
    let q = fine.moisture[c] as f32 / 255.0 * (1.0 + EVO_WARMTH_MOISTURE * warmth);
    classify(h, fine.sea_level + sea, fine.sea_level, lake, t, q)
}

impl LandBasis {
    fn build(w: &World) -> Self {
        let fine = &w.fine;
        let base = (0..fine.len()).map(|c| classify_fine(fine, c, fine.terrain[c] == Terrain::Lake, 0.0, 0.0)).collect();
        let sea: Vec<bool> = fine.terrain.iter().map(|&t| t == Terrain::Sea).collect();
        let (sea_part, sizes) = components(fine.w as usize, fine.h as usize, &sea, true);
        let land: Vec<bool> = sea.iter().map(|&s| !s).collect();
        let (land_part, land_sizes) = components(fine.w as usize, fine.h as usize, &land, true);
        Self {
            terrain: fine.terrain.clone(),
            river: fine.river.clone(),
            flow: fine.flow.clone(),
            base,
            cell_terrain: w.geo.terrain.clone(),
            cell_river: w.geo.river.clone(),
            cell_province: w.geo.province_of.clone(),
            sea_part,
            sea_parts: sizes.len(),
            land_part,
            land_parts: land_sizes.len(),
            rivers: w.geo.rivers.iter().map(|r| r.cells.clone()).collect(),
        }
    }

    /// 遊戲格 `(x, y)` 由格內細格推得的地形與河流等級（世界生成的降採樣規則，GDD 2.2）。
    fn cell(&self, fine: &FineGeography, x: usize, y: usize, cell: usize) -> (Terrain, u8) {
        let scale = fine.scale as usize;
        let fw = fine.w as usize;
        let mut counts = [0u32; Terrain::COUNT];
        let mut level = 0u8;
        let mut origin_level = 0u8;
        let mut pristine = true;
        for dy in 0..scale {
            for dx in 0..scale {
                let f = (y * scale + dy) * fw + x * scale + dx;
                let t = fine.terrain[f];
                pristine &= t == self.terrain[f];
                counts[t as usize] += 1;
                if !t.is_water() {
                    level = level.max(fine.river[f]);
                }
                if !self.terrain[f].is_water() {
                    origin_level = origin_level.max(self.river[f]);
                }
            }
        }
        if pristine {
            return (self.cell_terrain[cell], self.cell_river[cell]);
        }
        let sea = counts[Terrain::Sea as usize];
        let half = (scale * scale / 2) as u32;
        if sea > half || (sea == half && (x + y) % 2 == 0) {
            return (Terrain::Sea, 0);
        }
        let most = |skip_lake: bool| {
            let mut best = (Terrain::Plains, 0u32);
            for t in Terrain::ALL {
                let skip = t == Terrain::Sea || (skip_lake && t == Terrain::Lake);
                if !skip && counts[t as usize] > best.1 {
                    best = (t, counts[t as usize]);
                }
            }
            best.0
        };
        let mut terrain = most(false);
        // 生成時不是湖的遊戲格不因格內的碎湖變成湖（世界生成去掉過小的粗格湖）。
        if terrain == Terrain::Lake && self.cell_terrain[cell] != Terrain::Lake {
            terrain = most(true);
        }
        // 世界生成把過小的粗格湖改成陸地時，河流等級留 0；這種格照舊不算有河。
        let riverless = origin_level > 0 && self.cell_river[cell] == 0 && !self.cell_terrain[cell].is_water();
        (terrain, if terrain.is_water() || riverless { 0 } else { level })
    }
}

impl LandBasis {
    /// 生成時遊戲格的地形（水域名以它為準沿用，GDD 7.3）。
    pub fn cell_terrain(&self) -> &[Terrain] {
        &self.cell_terrain
    }
}

/// 建立地形的基準（[`LandBasis`]）。必須在世界還是生成時的樣子時呼叫：開天時建好；
/// 自帶細格、以 `from_parts` 建立的世界在第一次每世結算時建立。回傳這個世界有沒有細格資料。
pub fn prepare(w: &mut World) -> bool {
    let (fine, geo) = (&w.fine, &w.geo);
    let scale = fine.scale as usize;
    let fits = scale > 0
        && fine.w as usize == geo.w as usize * scale
        && fine.h as usize == geo.h as usize * scale
        && fine.terrain.len() == fine.len()
        && fine.height.len() == fine.len()
        && fine.temperature.len() == fine.len()
        && fine.moisture.len() == fine.len()
        && geo.terrain.len() == geo.w as usize * geo.h as usize;
    if !fits {
        return false;
    }
    if w.land_basis.is_none() {
        w.land_basis = Some(LandBasis::build(w));
    }
    true
}

/// 細格這一世的目標地形：變化量下的判定與零變化量下的判定相同就取生成時的地形，否則取新判定。
/// 人改過的遊戲格再疊上去（GDD 2.8 草木）：該是森林而已墾的是平原，該是平原而已沙化的是沙漠。
fn desired(fine: &FineGeography, basis: &LandBasis, evolution: &Evolution, flora: &Flora) -> Vec<Terrain> {
    let (warmth, sea) = (evolution.warmth_delta(), evolution.sea_delta());
    let (fw, scale) = (fine.w as usize, fine.scale as usize);
    let gw = fw / scale;
    let flagged = flora.cleared.len() * scale * scale == fine.len() && flora.sanded.len() == flora.cleared.len();
    (0..fine.len())
        .map(|c| {
            let target = classify_fine(fine, c, basis.terrain[c] == Terrain::Lake, warmth, sea);
            let mut want = if target == basis.base[c] { basis.terrain[c] } else { target };
            if flagged {
                let cell = c / fw / scale * gw + c % fw / scale;
                if want == Terrain::Forest && flora.cleared[cell] {
                    want = Terrain::Plains;
                }
                if want == Terrain::Plains && flora.sanded[cell] {
                    want = Terrain::Desert;
                }
            }
            want
        })
        .collect()
}

/// 一世一步：細格每世走 `scale` 輪（合遊戲格一格），每輪都以該輪開始時的地形判斷鄰格。
///
/// - 陸格沒入：與海相鄰才成為海。生成時的一片海整片乾了（內陸的小海灣），才改由這一片生成時的岸邊回填。
/// - 海格浮出：與陸或湖相鄰才浮出，先成沼澤（生成時原是湖的回復為湖）；下一世起直接依氣候判定，
///   不看鄰格。生成時不是沼澤、後來才成為沼澤的格都這樣處理。生成時的一塊陸地整塊沒入了（小島），
///   才改由這一塊生成時的岸邊浮出。
/// - 氣候地形：與「將變成的那一類」相鄰才改。雪原另以山脈為源頭（雪從山上下來），沼澤另以水為源頭。
///   雪原與沙漠只這樣由邊緣向外推；退回平原、森林、沼澤、丘陵的格，位於所屬區塊的邊緣（八鄰中有
///   山脈以外的別種地形）也可以改——雪線後退時，露出來的不一定與鄰格同一種地形。
/// - 要回到生成時的地形時，生成時那一片的邊緣也算數：整片被周圍吞掉的小海灣與小片森林才回得來。
///
/// 回傳改了地形的細格。
fn advance(fine: &mut FineGeography, basis: &LandBasis, want: &[Terrain]) -> Vec<u32> {
    let (fw, fh) = (fine.w as usize, fine.h as usize);
    let mut pending: Vec<u32> = (0..fine.len()).filter(|&c| fine.terrain[c] != want[c]).map(|c| c as u32).collect();
    let mut changed: Vec<u32> = Vec::new();
    if pending.is_empty() {
        return changed;
    }
    let mut fresh = vec![false; fine.len()];
    // 生成時的每一片海現在還有沒有海格、每一塊陸地現在還有沒有不是海的格。
    let mut wet = vec![false; basis.sea_parts];
    let mut dry = vec![false; basis.land_parts];
    for c in 0..fine.len() {
        match (fine.terrain[c] == Terrain::Sea, basis.terrain[c] == Terrain::Sea) {
            (true, true) => wet[basis.sea_part[c] as usize] = true,
            (false, false) => dry[basis.land_part[c] as usize] = true,
            _ => {}
        }
    }
    for _ in 0..fine.scale {
        let before = fine.terrain.clone();
        for &cell in &pending {
            let c = cell as usize;
            let (now, to) = (before[c], want[c]);
            let near_sea = || nb8(fw, fh, c).any(|(nb, _)| before[nb] == Terrain::Sea);
            let home = basis.terrain[c];
            let on_edge = || nb8(fw, fh, c).any(|(nb, _)| before[nb] != now);
            let dried_up = || home == Terrain::Sea && !wet[basis.sea_part[c] as usize];
            let on_old_shore = || nb8(fw, fh, c).any(|(nb, _)| basis.terrain[nb] != Terrain::Sea);
            let drowned = || home != Terrain::Sea && !dry[basis.land_part[c] as usize];
            let on_old_coast = || nb8(fw, fh, c).any(|(nb, _)| basis.terrain[nb] == Terrain::Sea);
            let spreading = matches!(to, Terrain::Snow | Terrain::Desert);
            let reachable = || {
                nb8(fw, fh, c).any(|(nb, _)| {
                    let t = before[nb];
                    t == to
                        || (to == Terrain::Snow && t == Terrain::Mountain)
                        || (to == Terrain::Swamp && t.is_water())
                        || (!spreading && t != now && t != Terrain::Mountain)
                        || (to == home && basis.terrain[nb] != home)
                })
            };
            let next = if now == Terrain::Sea {
                if !(on_edge() || (drowned() && on_old_coast())) {
                    continue;
                }
                fresh[c] = true;
                if to == Terrain::Lake {
                    Terrain::Lake
                } else {
                    Terrain::Swamp
                }
            } else if to == Terrain::Sea {
                if !(near_sea() || (dried_up() && on_old_shore())) {
                    continue;
                }
                Terrain::Sea
            } else if now.is_water() || to.is_water() || now == Terrain::Mountain || fresh[c] {
                // 湖與山脈不動；這一世剛浮出的新地下一世才依氣候判定。
                continue;
            } else if (now == Terrain::Swamp && home != Terrain::Swamp) || reachable() {
                to
            } else {
                continue;
            };
            fine.terrain[c] = next;
            let water = next.is_water();
            fine.river[c] = if water { 0 } else { basis.river[c] };
            fine.flow[c] = if water { 0 } else { basis.flow[c] };
            changed.push(cell);
        }
        pending.retain(|&c| fine.terrain[c as usize] != want[c as usize]);
        if pending.is_empty() {
            break;
        }
    }
    changed.sort_unstable();
    changed.dedup();
    changed
}

/// 新浮出的陸格併入相鄰格數最多的州郡（同數取 ID 小的）；沒有相鄰州郡的暫不屬任何州。
/// 一輪以該輪開始時的歸屬判斷，併到沒有可併的為止。生成時就不屬任何州的陸格不併。
fn adopt(w: usize, h: usize, terrain: &[Terrain], basis: &LandBasis, of: &mut [ProvinceId]) {
    let unowned = |c: usize| !basis.cell_terrain[c].is_water() && basis.cell_province[c] == NO_PROVINCE;
    let mut orphans: Vec<usize> =
        (0..of.len()).filter(|&c| of[c] == NO_PROVINCE && !terrain[c].is_water() && !unowned(c)).collect();
    while !orphans.is_empty() {
        let mut joins: Vec<(usize, ProvinceId)> = Vec::new();
        for &c in &orphans {
            let mut votes: Vec<(ProvinceId, u32)> = Vec::new();
            for nb in nb4(w, h, c) {
                let p = of[nb];
                if p == NO_PROVINCE {
                    continue;
                }
                match votes.iter_mut().find(|v| v.0 == p) {
                    Some(v) => v.1 += 1,
                    None => votes.push((p, 1)),
                }
            }
            if let Some(&(p, _)) = votes.iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))) {
                joins.push((c, p));
            }
        }
        if joins.is_empty() {
            break;
        }
        for &(c, p) in &joins {
            of[c] = p;
        }
        orphans.retain(|&c| of[c] == NO_PROVINCE);
    }
}

/// GDD 2.8 第 7 步：遊戲格的海陸或地形變了之後，重算州郡的格、地形組成、宜居度、治所與相鄰關係；
/// `coast_moved` 為假時（只有氣候地形變了）歸屬與相鄰關係不變。
fn recompute_provinces(w: &mut World, basis: &LandBasis, coast_moved: bool, shift: &mut LandShift) {
    let geo = &mut w.geo;
    let (gw, gh) = (geo.w as usize, geo.h as usize);
    let sphere = Sphere::new(gw, gh);
    if coast_moved {
        for c in 0..geo.terrain.len() {
            if geo.terrain[c].is_water() {
                geo.province_of[c] = NO_PROVINCE;
            }
        }
        adopt(gw, gh, &geo.terrain, basis, &mut geo.province_of);
    }

    let habitability = cell_habitability(&sphere, &geo.terrain, &geo.river);
    let had_cells: Vec<bool> = w.provinces.iter().map(|p| !p.cells.is_empty()).collect();
    for p in &mut w.provinces {
        p.cells.clear();
        p.terrain_mix = [0; Terrain::COUNT];
        p.habitability = 0.0;
    }
    for c in 0..geo.terrain.len() {
        let Some(p) = w.provinces.get_mut(geo.province_of[c] as usize) else { continue };
        p.cells.push(c as u32);
        p.terrain_mix[geo.terrain[c] as usize] += 1;
        p.habitability += habitability[c];
    }
    for p in &mut w.provinces {
        if p.cells.is_empty() || geo.province_of[p.seat as usize] == p.id {
            continue;
        }
        // 治所被淹：遷到州內宜居度最高的格（同值取格索引小的）。
        let mut seat = p.cells[0];
        for &c in &p.cells {
            if habitability[c as usize] > habitability[seat as usize] {
                seat = c;
            }
        }
        shift.seats.push((p.id, p.seat, seat));
        p.seat = seat;
    }
    if !coast_moved {
        return;
    }

    // 州郡的格全數沒入海中：人口移往原本的陸鄰與海鄰（承載比低的先收，收到滿為止），容不下的折損。
    for id in 0..w.provinces.len() {
        if !had_cells[id] || !w.provinces[id].cells.is_empty() {
            continue;
        }
        let ratio = |w: &World, n: ProvinceId| {
            let p = &w.provinces[n as usize];
            p.population / p.capacity
        };
        let mut hosts: Vec<ProvinceId> = w.provinces[id]
            .neighbors
            .iter()
            .chain(&w.provinces[id].sea_neighbors)
            .copied()
            .filter(|&n| !w.provinces[n as usize].cells.is_empty() && w.provinces[n as usize].capacity > 0.0)
            .collect();
        hosts.sort_by(|&a, &b| ratio(w, a).total_cmp(&ratio(w, b)).then(a.cmp(&b)));
        hosts.dedup();
        let mut homeless = w.provinces[id].population;
        let mut refuge: Option<(ProvinceId, f64)> = None;
        for &n in &hosts {
            let host = &mut w.provinces[n as usize];
            let taken = (host.capacity - host.population).clamp(0.0, homeless);
            host.population += taken;
            homeless -= taken;
            if refuge.is_none_or(|(_, most)| taken > most) {
                refuge = Some((n, taken));
            }
        }
        let p = &mut w.provinces[id];
        p.population = 0.0;
        p.capacity = 0.0;
        p.city_level = 0;
        p.lost = true;
        (p.aura, p.karma_qi, p.merit, p.righteous, p.devastation) = (0.0, 0.0, 0.0, 0.0, 0.0);
        p.faith = [0.0; 3];
        shift.lost.push(LostProvince { province: id as ProvinceId, refuge: refuge.map(|r| r.0), perished: homeless.round() as u32 });
    }

    // 陸鄰與海鄰照世界生成的規則重算；陸鄰的增減就是陸橋的浮現與沉沒。
    let geo = &w.geo;
    let adjacency = adjacency(&sphere, &geo.terrain, &geo.province_of, w.provinces.len());
    for (p, (land, sea)) in w.provinces.iter_mut().zip(adjacency.land.into_iter().zip(adjacency.sea)) {
        let land: Vec<ProvinceId> = land.into_iter().collect();
        for &n in land.iter().filter(|&&n| n > p.id && !p.neighbors.contains(&n)) {
            shift.links.push((p.id, n, true));
        }
        for &n in p.neighbors.iter().filter(|&&n| n > p.id && !land.contains(&n)) {
            shift.links.push((p.id, n, false));
        }
        p.neighbors = land;
        p.sea_neighbors = sea.into_iter().collect();
    }
    // 消失的州郡不算陸橋沉沒。
    let provinces = &w.provinces;
    shift.links.retain(|&(a, b, _)| !provinces[a as usize].cells.is_empty() && !provinces[b as usize].cells.is_empty());

    rehome(w, &shift.lost);
}

/// 州郡消失或所在的格沒入水中之後，把記著位置的人、魂與勢力移到陸上：消失的州郡改記收容的鄰州
/// （沒有鄰州則取治所最近的州），沒入水中的格改記該州的治所。宗門的山門因此搬走時失去洞天福地。
fn rehome(w: &mut World, lost: &[LostProvince]) {
    let (gw, geo) = (w.geo.w as usize, &w.geo);
    let provinces = &w.provinces;
    let mut moved_to: Vec<Option<ProvinceId>> = vec![None; provinces.len()];
    for l in lost {
        let old_seat = provinces[l.province as usize].seat as usize;
        moved_to[l.province as usize] = l.refuge.or_else(|| {
            provinces
                .iter()
                .filter(|p| !p.cells.is_empty())
                .min_by_key(|p| (crate::worldgen::grid::cheb(gw, old_seat, p.seat as usize), p.id))
                .map(|p| p.id)
        });
    }
    let place = |province: ProvinceId, cell: u32| -> (ProvinceId, u32) {
        let Some(p) = provinces.get(province as usize) else { return (province, cell) };
        if let Some(to) = moved_to[province as usize] {
            return (to, provinces[to as usize].seat);
        }
        let afloat = geo.terrain.get(cell as usize).is_some_and(|t| !t.is_water());
        if afloat || p.cells.is_empty() {
            (province, cell)
        } else {
            (province, p.seat)
        }
    };
    for person in &mut w.persons {
        (person.province, person.cell) = place(person.province, person.cell);
        if let Some(to) = moved_to.get(person.home as usize).copied().flatten() {
            person.home = to;
        }
    }
    for soul in &mut w.souls {
        soul.last_place = place(soul.last_place.0, soul.last_place.1);
        // 引渡指定的投生地消失了，改指收容的鄰州。
        if let Some(province) = soul.guide.as_mut().and_then(|g| g.province.as_mut()) {
            if let Some(to) = moved_to.get(*province as usize).copied().flatten() {
                *province = to;
            }
        }
    }
    for power in &mut w.powers {
        let seat = place(power.seat_province, power.seat_cell);
        if seat == (power.seat_province, power.seat_cell) {
            continue;
        }
        (power.seat_province, power.seat_cell) = seat;
        // 山門搬走了，原本佔的洞天福地就此放開。
        let PowerKind::Sect(sect) = &mut power.kind;
        sect.seat_node = None;
    }
}

/// GDD 2.8「每世結算」第 2、7 步。沒有細格資料的手工世界不演變。
pub fn settle(w: &mut World) -> LandShift {
    settle_with(w, &[])
}

/// 同 [`settle`]；`touched` 是這一世草木旗標有變的遊戲格（[`crate::flora::settle`]），格內的平原、森林、
/// 沙漠當世就照旗標改，不等一世一步。
pub fn settle_with(w: &mut World, touched: &[u32]) -> LandShift {
    let mut shift = LandShift::default();
    if !prepare(w) {
        return shift;
    }
    let basis = w.land_basis.take().expect("prepare 剛建立");
    let want = desired(&w.fine, &basis, &w.evolution, &w.flora);
    let mut changed = advance(&mut w.fine, &basis, &want);
    let scale = w.fine.scale as usize;
    let (fw, gw) = (w.fine.w as usize, w.geo.w as usize);
    let soft = |t: Terrain| matches!(t, Terrain::Plains | Terrain::Forest | Terrain::Desert);
    for &cell in touched {
        let (x, y) = (cell as usize % gw, cell as usize / gw);
        for f in (0..scale * scale).map(|i| (y * scale + i / scale) * fw + x * scale + i % scale) {
            if w.fine.terrain[f] != want[f] && soft(w.fine.terrain[f]) && soft(want[f]) {
                w.fine.terrain[f] = want[f];
                changed.push(f as u32);
            }
        }
    }
    changed.sort_unstable();
    changed.dedup();
    shift.fine = changed.len() as u32;

    // 有細格變動的遊戲格照降採樣規則重推。
    let mut blocks: Vec<u32> = changed.iter().map(|&f| ((f as usize / fw / scale) * gw + f as usize % fw / scale) as u32).collect();
    blocks.dedup();
    blocks.sort_unstable();
    blocks.dedup();
    let mut reshaped = false;
    for &cell in &blocks {
        let c = cell as usize;
        let (terrain, river) = basis.cell(&w.fine, c % gw, c / gw, c);
        let before = w.geo.terrain[c];
        if terrain == before && river == w.geo.river[c] {
            continue;
        }
        reshaped |= terrain != before;
        if terrain.is_water() != before.is_water() {
            if terrain.is_water() {
                shift.drowned += 1;
            } else {
                shift.emerged += 1;
            }
        }
        w.geo.terrain[c] = terrain;
        w.geo.river[c] = river;
        shift.cells.push(cell);
    }
    if reshaped {
        // 江河的幹流取生成時的河道，去掉尾端沒入海中的格（GDD 2.2 步驟 6）。
        for (river, origin) in w.geo.rivers.iter_mut().zip(&basis.rivers) {
            let mut end = origin.len();
            while end > 0 && w.geo.terrain[origin[end - 1] as usize] == Terrain::Sea {
                end -= 1;
            }
            if river.cells.len() != end {
                river.cells = origin[..end].to_vec();
            }
        }
    }
    if !shift.cells.is_empty() {
        recompute_provinces(w, &basis, shift.drowned + shift.emerged > 0, &mut shift);
    }
    w.land_basis = Some(basis);
    shift
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 這裡的單格判定與世界生成的地形判定（GDD 2.2 步驟 5）是同一套規則。
    #[test]
    fn gdd_2_8_classification_matches_world_generation() {
        let sea = 0.4f32;
        let steps = |n: u32| (0..=n).map(move |i| i as f32 / n as f32);
        let mut checked = 0;
        for h in steps(40) {
            for t in steps(25) {
                for q in steps(25) {
                    for lake in [false, true] {
                        let expected = crate::worldgen::terrain::classify(&[h], sea, &[lake], &[t], &[q])[0];
                        assert_eq!(classify(h, sea, sea, lake, t, q), expected, "h {h} t {t} q {q} lake {lake}");
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 50_000);
    }
}
