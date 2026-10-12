//! GDD 3.7 疆域：開國、立國、崩潰、割據、內戰、擴張、關係、天命的效果。

use std::collections::BTreeSet;
use std::sync::OnceLock;

use rand::Rng;

use super::*;
use crate::soul;
use crate::types::Era;

/// 不可達的距離。
const FAR: u32 = u32::MAX;

fn set_owner(w: &mut World, p: ProvinceId, d: DynastyId) {
    w.dynasty.ensure(w.provinces.len());
    w.dynasty.owner[p as usize] = d;
}

/// `src` 起算的步數，只走 `allowed` 的州郡（陸鄰）；走不到者為 [`FAR`]。
pub(super) fn hops(w: &World, src: ProvinceId, allowed: &dyn Fn(ProvinceId) -> bool) -> Vec<u32> {
    let mut dist = vec![FAR; w.provinces.len()];
    if (src as usize) >= dist.len() {
        return dist;
    }
    dist[src as usize] = 0;
    let mut frontier = vec![src];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for &p in &frontier {
            for &n in &w.provinces[p as usize].neighbors {
                if dist[n as usize] == FAR && allowed(n) {
                    dist[n as usize] = dist[p as usize] + 1;
                    next.push(n);
                }
            }
        }
        frontier = next;
    }
    dist
}

/// 與 `provs` 相鄰、無主且未沉沒的陸鄰，升冪。
fn free_neighbors(w: &World, provs: &[ProvinceId]) -> Vec<ProvinceId> {
    let mut set = BTreeSet::new();
    for &p in provs {
        for &n in &w.provinces[p as usize].neighbors {
            if w.dynasty.owner_of(n).is_none() && !w.provinces[n as usize].lost {
                set.insert(n);
            }
        }
    }
    set.into_iter().collect()
}

fn names() -> &'static [String] {
    static N: OnceLock<Vec<String>> = OnceLock::new();
    N.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(include_str!("../../data/orgs.zh-Hant.json")).expect("orgs.zh-Hant.json");
        v["prefix_chars_by_type_and_faction"]["國家"]["names_1char"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
            .unwrap_or_default()
    })
}

/// GDD 7.3 國號一字：從字庫挑一個目前在世王朝沒用的；全用盡時加「新」。
fn dynasty_name(w: &mut World) -> String {
    let pool = names();
    let free: Vec<&String> = pool.iter().filter(|n| !w.dynasty.list.iter().any(|d| d.fallen.is_none() && &d.name == *n)).collect();
    if free.is_empty() {
        let k = w.rng.dynasty.gen_range(0..pool.len().max(1));
        return format!("新{}", pool.get(k).map_or("", String::as_str));
    }
    free[w.rng.dynasty.gen_range(0..free.len())].clone()
}

/// 讓人物成為君主：身分、駐在都城。
pub(super) fn make_ruler(w: &mut World, pid: PersonId, capital: ProvinceId) {
    move_to(w, pid, capital);
    let p = &mut w.persons[pid as usize];
    p.role = Role::Ruler;
    p.path = PathState::Mortal;
}

/// 人物改駐某州（治所格）。
pub(super) fn move_to(w: &mut World, pid: PersonId, province: ProvinceId) {
    let cell = w.provinces[province as usize].seat;
    let p = &mut w.persons[pid as usize];
    p.province = province;
    p.cell = cell;
    let sid = p.soul;
    w.souls[sid as usize].last_place = (province, cell);
}

/// 建立王朝但不寫事件：`provinces` 全部改歸新王朝（可能自舊王朝奪來），`founder` 為君主。
fn found_raw(w: &mut World, founder: PersonId, capital: ProvinceId, provinces: &[ProvinceId]) -> DynastyId {
    w.dynasty.ensure(w.provinces.len());
    let id = w.dynasty.list.len() as DynastyId;
    let name = dynasty_name(w);
    let mut provs = provinces.to_vec();
    provs.sort_unstable();
    provs.dedup();
    for &p in &provs {
        set_owner(w, p, id);
    }
    make_ruler(w, founder, capital);
    let policy = court::policy_of(w, founder);
    let tick = w.tick;
    w.dynasty.list.push(Dynasty {
        id,
        name,
        founder: Some(founder),
        ruler: Some(founder),
        heirs: Vec::new(),
        capital,
        provinces: provs,
        mandate: MANDATE_INIT,
        treasury: 0.0,
        policy,
        decree: Decree::None,
        officials: Vec::new(),
        generals: Vec::new(),
        standing_army: 0.0,
        wars: Vec::new(),
        purges: 0,
        prestige: 0.0,
        prestige_mod: 0.0,
        founded: tick,
        fallen: None,
        relations: Default::default(),
        sect_relations: Default::default(),
        ruler_since: tick,
        high_years: 0,
        fengshan_by: None,
        gained: 0,
        lost: 0,
        famine: 0,
        window: Vec::new(),
    });
    // 國庫初值為一年收入（GDD 3.7）。
    let fin = finance(w, id as usize);
    let d = &mut w.dynasty.list[id as usize];
    d.treasury = fin.income;
    d.standing_army = fin.standing;
    d.prestige = d.provinces.len() as f32 * DYN_PRESTIGE_PER_PROVINCE + d.mandate;
    id
}

/// GDD 3.7 立國：`founder` 在 `capital` 立國並取得 `provinces`；寫入 `DynastyFounded`。
pub fn found(w: &mut World, founder: PersonId, capital: ProvinceId, provinces: &[ProvinceId]) -> DynastyId {
    let cause: Vec<EventId> = w.persons[founder as usize].last_event.into_iter().collect();
    let id = found_raw(w, founder, capital, provinces);
    rec(w, EventKind::DynastyFounded, 4, Some(capital), &[founder], id, None, 0, &cause);
    id
}

/// 新建一名成年的散人（新魂，年齡 `age`），君主或起義首領不夠時使用。
fn new_adult(w: &mut World, province: ProvinceId, age: u32) -> PersonId {
    // 開天（tick 0）時建立的是先天魂（GDD 4.6）。
    let origin = if w.tick == 0 { SoulOrigin::Primordial } else { SoulOrigin::New };
    let (_, pid) = soul::spawn_new_life(w, origin, province);
    let tick = w.tick as i32;
    let p = &mut w.persons[pid as usize];
    p.born = tick - (age * 12) as i32;
    p.role = Role::Commoner;
    p.path = PathState::Mortal;
    pid
}

/// 州內才略最高的散人或武將候補（成年、無勢力、才略 ≥ `min_talent`）；同值取 ID 小者。
fn best_commoner(w: &World, province: ProvinceId, min_talent: f32) -> Option<PersonId> {
    let tick = w.tick;
    let mut best: Option<(f32, PersonId)> = None;
    for &i in &w.alive_ids {
        let p = &w.persons[i as usize];
        if p.province != province
            || p.power.is_some()
            || !matches!(p.role, Role::Commoner | Role::GeneralCandidate)
            || p.age(tick) < OFFICIAL_MIN_AGE
            || p.attrs.talent < min_talent
        {
            continue;
        }
        if best.is_none_or(|(t, _)| p.attrs.talent > t) {
            best = Some((p.attrs.talent, p.id));
        }
    }
    best.map(|b| b.1)
}

/// GDD 3.7 每年的立國判定：無主且人口 ≥ 50,000 的州，若有才略 ≥ 8 的散人或武將候補，2% 機率立國，
/// 取得該州及無主的陸鄰。依州郡 ID 升冪；只有條件齊備的州才抽樣。
pub fn yearly_found(w: &mut World, round: Round) {
    let seed = w.params.seed;
    for p in 0..w.provinces.len() as ProvinceId {
        let pr = &w.provinces[p as usize];
        if !round.takes(seed, Domain::Province, p as u32) || pr.lost || w.dynasty.owner_of(p).is_some() || pr.population < DYN_FOUND_POP {
            continue;
        }
        let Some(founder) = best_commoner(w, p, DYN_FOUND_TALENT) else { continue };
        if w.rng.dynasty.gen::<f32>() >= DYN_FOUND_CHANCE {
            continue;
        }
        let mut provs = free_neighbors(w, &[p]);
        provs.push(p);
        found(w, founder, p, &provs);
    }
}

/// GDD 3.7 開國：中古 4–6 個、末法 2–3 個王朝，合計覆蓋約 70% 的陸地（州郡數）；上古無。
/// 都城取彼此（陸路）最遠的州（人口多者優先），疆域由各都城輪流向無主陸鄰擴張。
/// 君主取都城的散人或武將候補中才略最高者，沒有則新建一名 25 歲的散人。
pub fn found_initial(w: &mut World) {
    let count = match w.params.era {
        Era::Ancient => return,
        Era::Middle => w.rng.genesis.gen_range(DYN_INIT_MIDDLE_MIN..=DYN_INIT_MIDDLE_MAX),
        Era::Decline => w.rng.genesis.gen_range(DYN_INIT_DECLINE_MIN..=DYN_INIT_DECLINE_MAX),
    } as usize;
    let land: Vec<ProvinceId> = w.provinces.iter().filter(|p| !p.lost).map(|p| p.id).collect();
    if land.is_empty() {
        return;
    }
    let pop = |p: ProvinceId| w.provinces[p as usize].population;
    let by_pop = |a: &ProvinceId, b: &ProvinceId| pop(*a).total_cmp(&pop(*b)).then(b.cmp(a));
    let first = *land.iter().max_by(|a, b| by_pop(a, b)).expect("land");
    let not_lost = |p: ProvinceId| !w.provinces[p as usize].lost;
    let mut seeds = vec![first];
    let mut mind = hops(w, first, &not_lost);
    while seeds.len() < count.min(land.len()) {
        let pick = land
            .iter()
            .copied()
            .filter(|p| !seeds.contains(p))
            .max_by(|a, b| mind[*a as usize].cmp(&mind[*b as usize]).then(by_pop(a, b)))
            .expect("seed");
        seeds.push(pick);
        for (m, d) in mind.iter_mut().zip(hops(w, pick, &not_lost)) {
            *m = (*m).min(d);
        }
    }
    // 輪流擴張。
    let target = ((land.len() as f32 * DYN_INIT_COVER).round() as usize).max(seeds.len());
    let mut owner: Vec<Option<usize>> = vec![None; w.provinces.len()];
    let mut terr: Vec<Vec<ProvinceId>> = seeds.iter().map(|&s| vec![s]).collect();
    for (k, &s) in seeds.iter().enumerate() {
        owner[s as usize] = Some(k);
    }
    let mut total = seeds.len();
    'grow: loop {
        let mut progress = false;
        for (k, t) in terr.iter_mut().enumerate() {
            if total >= target {
                break 'grow;
            }
            let mut best: Option<ProvinceId> = None;
            for &p in t.iter() {
                for &n in &w.provinces[p as usize].neighbors {
                    if owner[n as usize].is_none() && !w.provinces[n as usize].lost && best.is_none_or(|b| by_pop(&n, &b).is_gt()) {
                        best = Some(n);
                    }
                }
            }
            if let Some(n) = best {
                owner[n as usize] = Some(k);
                t.push(n);
                total += 1;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    for (k, &cap) in seeds.iter().enumerate() {
        let founder = best_commoner(w, cap, 0.0).unwrap_or_else(|| new_adult(w, cap, REBEL_AGE));
        found(w, founder, cap, &terr[k]);
    }
}

/// GDD 3.7 崩潰（天命 0，或無人可繼）：所有州郡無主；王族、君主成為散人，文臣回到散儒，武將回到武將候補。
pub fn collapse(w: &mut World, di: usize) {
    let id = di as DynastyId;
    let tick = w.tick;
    let (provs, capital, ruler, heirs, officials, generals) = {
        let d = &w.dynasty.list[di];
        (d.provinces.clone(), d.capital, d.ruler, d.heirs.clone(), d.officials.clone(), d.generals.iter().map(|g| g.0).collect::<Vec<_>>())
    };
    for &p in &provs {
        set_owner(w, p, NO_OWNER);
    }
    let revert = |w: &mut World, pids: &[PersonId], role: Role| {
        for &x in pids {
            let p = &mut w.persons[x as usize];
            if p.alive() {
                p.role = role;
            }
        }
    };
    revert(w, &ruler.into_iter().chain(heirs).collect::<Vec<_>>(), Role::Commoner);
    revert(w, &officials, Role::Scholar);
    revert(w, &generals, Role::GeneralCandidate);
    {
        let d = &mut w.dynasty.list[di];
        d.fallen = Some(tick);
        d.provinces.clear();
        d.heirs.clear();
        d.officials.clear();
        d.generals.clear();
        d.wars.clear();
        d.ruler = None;
        d.standing_army = 0.0;
    }
    for o in w.dynasty.list.iter_mut() {
        o.wars.retain(|&x| x != id);
    }
    rec(w, EventKind::DynastyCollapsed, 5, Some(capital), &[], id, None, 0, &[]);
}

/// 疆域內 `from` 到各州的步數（只走本國州郡）。
fn hops_in(w: &World, di: usize, from: ProvinceId) -> Vec<u32> {
    let id = di as DynastyId;
    hops(w, from, &|p| w.dynasty.owner_of(p) == Some(id))
}

/// GDD 3.7 割據：武將據駐地自立為新王朝。取得駐地及相連的本國州郡中，距駐地比距都城近者。
/// 武將駐在都城時無法割據，回傳 `None`。
pub fn secede(w: &mut World, di: usize, general: PersonId) -> Option<DynastyId> {
    let id = di as DynastyId;
    let (station, capital) = {
        let d = &w.dynasty.list[di];
        (d.generals.iter().find(|g| g.0 == general)?.1, d.capital)
    };
    if station == capital {
        return None;
    }
    let hs = hops_in(w, di, station);
    let hc = hops_in(w, di, capital);
    let set: Vec<ProvinceId> =
        w.dynasty.list[di].provinces.iter().copied().filter(|&p| hs[p as usize] != FAR && hs[p as usize] < hc[p as usize]).collect();
    {
        let tick = w.tick;
        let d = &mut w.dynasty.list[di];
        d.generals.retain(|g| g.0 != general);
        d.provinces.retain(|p| !set.contains(p));
        d.note_flow(tick, 0, set.len() as u16, 0);
    }
    let cause: Vec<EventId> = w.persons[general as usize].last_event.into_iter().collect();
    let nid = found_raw(w, general, station, &set);
    rec(w, EventKind::Secession, 4, Some(station), &[general], nid, Some(id), 0, &cause);
    Some(nid)
}

/// GDD 3.7 內戰：以兩人的駐地為核心，依距離瓜分疆域成兩國（`a` 留在原王朝，`b` 另立新王朝），互相宣戰
/// （3.9 之前只記關係值與 `wars`）。`core_a` 與 `core_b` 同一州時，`b` 的核心改取離 `core_a` 最遠的本國州。
/// 疆域只有一州或分不出兩邊時不內戰，回傳 `false`。
pub fn civil_war(w: &mut World, di: usize, core_a: ProvinceId, b: PersonId, core_b: ProvinceId) -> bool {
    let id = di as DynastyId;
    let dom = w.dynasty.list[di].provinces.clone();
    if dom.len() < 2 {
        return false;
    }
    let ha = hops_in(w, di, core_a);
    let mut core_b = core_b;
    if core_b == core_a {
        core_b = dom
            .iter()
            .copied()
            .filter(|&p| ha[p as usize] != FAR)
            .max_by(|&x, &y| ha[x as usize].cmp(&ha[y as usize]).then(y.cmp(&x)))
            .unwrap_or(core_a);
        if core_b == core_a {
            return false;
        }
    }
    let hb = hops_in(w, di, core_b);
    let side_b: Vec<ProvinceId> = dom.iter().copied().filter(|&p| hb[p as usize] < ha[p as usize]).collect();
    if side_b.is_empty() || side_b.len() == dom.len() {
        return false;
    }
    let cause: Vec<EventId> = w.persons[b as usize].last_event.into_iter().collect();
    // 分家：b 的一邊（含駐在該邊的武將）。
    let moved: Vec<(PersonId, ProvinceId)> = w.dynasty.list[di].generals.iter().copied().filter(|g| side_b.contains(&g.1)).collect();
    let tick_now = w.tick;
    let share_a = {
        let d = &mut w.dynasty.list[di];
        d.officials.retain(|&o| o != b);
        d.generals.retain(|g| g.0 != b && !side_b.contains(&g.1));
        d.provinces.retain(|p| !side_b.contains(p));
        d.note_flow(tick_now, 0, side_b.len() as u16, 0);
        if !d.provinces.contains(&d.capital) {
            d.capital = core_a;
        }
        d.provinces.len() as f64 / dom.len() as f64
    };
    let nid = found_raw(w, b, core_b, &side_b);
    {
        let old_treasury = w.dynasty.list[di].treasury;
        w.dynasty.list[di].treasury = old_treasury * share_a;
        let nd = &mut w.dynasty.list[nid as usize];
        nd.generals = moved.into_iter().filter(|g| g.0 != b).collect();
        nd.relations.insert(id, CIVIL_WAR_RELATION);
        nd.wars.push(id);
        let od = &mut w.dynasty.list[di];
        od.relations.insert(nid, CIVIL_WAR_RELATION);
        od.wars.push(nid);
    }
    rec(w, EventKind::CivilWar, 4, Some(core_b), &[b], id, Some(nid), 0, &cause);
    true
}

/// GDD 3.7 擴張（天命 ≥ 50 且國庫 ≥ 兩年支出）：開疆 20%（吞併一個相鄰無主州，取人口最多者，同值 ID 小者）；
/// 宣戰 10%（對相鄰王朝：我方軍力 ≥ 對方 1.3 倍、關係 < 0、尚未交戰；只記錄與發事件，戰鬥屬 3.9）。
pub(super) fn expansion(w: &mut World, di: usize, fin: &Finance) {
    let id = di as DynastyId;
    {
        let d = &w.dynasty.list[di];
        if d.mandate < EXPAND_MANDATE || d.treasury < fin.expenses * EXPAND_TREASURY_YEARS {
            return;
        }
    }
    if w.rng.dynasty.gen::<f32>() < EXPAND_ANNEX_CHANCE {
        let cands = free_neighbors(w, &w.dynasty.list[di].provinces.clone());
        let pick = cands
            .into_iter()
            .max_by(|&a, &b| w.provinces[a as usize].population.total_cmp(&w.provinces[b as usize].population).then(b.cmp(&a)));
        if let Some(p) = pick {
            set_owner(w, p, id);
            let tick = w.tick;
            let d = &mut w.dynasty.list[di];
            d.provinces.push(p);
            d.provinces.sort_unstable();
            d.note_flow(tick, 1, 0, 0);
            rec(w, EventKind::Annexed, 2, Some(p), &[], id, None, 0, &[]);
        }
    }
    let neighbors = neighbor_dynasties(w, di);
    let mine = military_power(w, di);
    for o in neighbors {
        let (rel, at_war) = {
            let d = &w.dynasty.list[di];
            (d.relations.get(&o).copied().unwrap_or(0), d.wars.contains(&o))
        };
        if rel >= 0 || at_war || mine < EXPAND_WAR_RATIO * military_power(w, o as usize) {
            continue;
        }
        if w.rng.dynasty.gen::<f32>() < EXPAND_WAR_CHANCE {
            w.dynasty.list[di].wars.push(o);
            w.dynasty.list[o as usize].wars.push(id);
            rec(w, EventKind::WarDeclared, 3, None, &[], id, Some(o), 0, &[]);
            break;
        }
    }
}

/// 疆域接壤的其他在世王朝，ID 升冪。
fn neighbor_dynasties(w: &World, di: usize) -> Vec<DynastyId> {
    let id = di as DynastyId;
    let mut set = BTreeSet::new();
    for &p in &w.dynasty.list[di].provinces {
        for &n in &w.provinces[p as usize].neighbors {
            if let Some(o) = w.dynasty.owner_of(n) {
                if o != id {
                    set.insert(o);
                }
            }
        }
    }
    set.into_iter().collect()
}

/// GDD 3.7 關係：每年向 0 回歸 10%；與接壤王朝 −5；同國策（非「無」）+5。
pub(super) fn update_relations(w: &mut World, di: usize) {
    let neighbors = neighbor_dynasties(w, di);
    let policy = w.dynasty.list[di].policy;
    let delta: Vec<(DynastyId, i32)> = neighbors
        .iter()
        .map(|&o| {
            let same = policy != Policy::None && w.dynasty.list[o as usize].policy == policy;
            (o, RELATION_BORDER + if same { RELATION_SAME_POLICY } else { 0 })
        })
        .collect();
    let d = &mut w.dynasty.list[di];
    for r in d.relations.values_mut() {
        *r -= (*r as i32 / RELATION_DRIFT_RATE) as i8;
    }
    for (o, dv) in delta {
        let r = d.relations.entry(o).or_insert(0);
        *r = (*r as i32 + dv).clamp(-100, 100) as i8;
    }
    d.relations.retain(|_, r| *r != 0);
}

/// GDD 3.7 天命的效果（天命 > 0 時）：天命 < 20 的割據、天命 < 40 的起義判定；國庫不足時才略最高的武將 50% 叛離。
pub(super) fn unrest(w: &mut World, di: usize, broke: bool) {
    let id = di as DynastyId;
    let mandate = w.dynasty.list[di].mandate;
    if mandate < SECEDE_MANDATE {
        let p = (SECEDE_MANDATE - mandate) / SECEDE_DIV;
        let gens = w.dynasty.list[di].generals.clone();
        for (g, st) in gens {
            if st == w.dynasty.list[di].capital || !w.dynasty.list[di].generals.iter().any(|x| x.0 == g) {
                continue;
            }
            if w.rng.dynasty.gen::<f32>() < p && secede(w, di, g).is_some() {
                break;
            }
        }
    }
    if broke {
        // 才略最高的武將（同值取 ID 小者）50% 機率叛離；駐在都城者無地可據。
        let top = w.dynasty.list[di]
            .generals
            .iter()
            .copied()
            .max_by(|a, b| w.persons[a.0 as usize].attrs.talent.total_cmp(&w.persons[b.0 as usize].attrs.talent).then(b.0.cmp(&a.0)));
        if let Some((g, _)) = top {
            if w.rng.dynasty.gen::<f32>() < BROKE_DEFECT_CHANCE {
                secede(w, di, g);
            }
        }
    }
    let mandate = w.dynasty.list[di].mandate;
    if mandate < REBEL_MANDATE {
        let p = (REBEL_MANDATE - mandate) / REBEL_DIV;
        for prov in w.dynasty.list[di].provinces.clone() {
            if w.rng.dynasty.gen::<f32>() >= p {
                continue;
            }
            // 首領：該州才略最高的散人或武將候補；沒有時建立新魂，25 歲出生在該州（叛軍與戰鬥屬 3.9）。
            let leader = best_commoner(w, prov, 0.0).unwrap_or_else(|| new_adult(w, prov, REBEL_AGE));
            let cause: Vec<EventId> = w.persons[leader as usize].last_event.into_iter().collect();
            rec(w, EventKind::Rebellion, 3, Some(prov), &[leader], id, None, 0, &cause);
        }
    }
}
