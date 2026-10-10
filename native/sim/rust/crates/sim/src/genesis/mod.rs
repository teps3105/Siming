//! GDD 4.6 開天（M2 子集）（U8）。
//!
//! 開天不建立王朝、寺院、書院（M3、M4）；開局時代只影響宗門數、初始有名之人數
//! （靈氣倍率與人口係數已在世界生成時套用）。
//!
//! # 流程（`run`）
//!
//! 1. [`build`]：天道魂 → `Genesis` 事件 → 宗門（山門、掌門、成員）→ 其餘有名之人 → 開局宿緣 → 道號。
//!    全部在 tick 0；人物的 `born` 為負值。
//! 2. 預跑：`Sim::step` 推進 tick 1..=`GENESIS_PRERUN_TICK_END`，期間 `World::prerun` 為真；
//!    事件寫入因果帳，但不進 `Sim::drain_events`（不進傳聞、不觸發奇觀、不給天機）。
//! 3. 開局保證：中古、末法的高境界保證（[`ensure_high_realm`]）→ 強制突破（[`ensure_forced_breakthrough`]）。
//!
//! # 亂數
//!
//! 先天魂、年齡、境界、修為、出生地、宿緣對象、山門以外的選擇一律取自 `genesis` 子流。
//! 為了重用既有規則，下列呼叫會另外消耗其他子流（仍為確定性）：`people::spawn`（屬性、性別、姓名、壽元）、
//! `people::enter_path`（入道抽樣）用 `people`（道號 `people::assign_title` 不消耗子流）；`dao::sect_name` 用 `powers`。
//!
//! # 介面
//!
//! | 函式 | 用途 |
//! |---|---|
//! | [`run`] | `Sim::new` 呼叫：整個開天 |
//! | [`build`]、[`ensure_high_realm`]、[`ensure_forced_breakthrough`] | 測試與分段使用 |
//! | [`camera_start`] | 鏡頭初始位置（聲望最高的宗門山門，4.6 步驟 7） |

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use rand::Rng;
use smallvec::smallvec;

use crate::dao;
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::people;
use crate::sim::{Sim, World};
use crate::soul::{self, pull};
use crate::thread;
use crate::types::{Era, ProvinceId, WorldSize};

/// 開局宿緣的種類（4.6 步驟 5：不含怨），依序輪流。
const THREAD_KINDS: [ThreadKind; 4] = [ThreadKind::Feud, ThreadKind::Grace, ThreadKind::Love, ThreadKind::Mentor];
/// 挑選開局宿緣雙方時，每條線最多嘗試的次數。
const THREAD_ATTEMPTS: u32 = 60;
/// 境界金字塔涵蓋的境界數（0..=8）。
const REALMS: usize = REALM_EXP.len();

/// 建立勢力、有名之人、魂、開局宿緣；預跑；開局保證。
pub fn run(sim: &mut Sim) {
    build(sim.world_mut());
    prerun(sim);
    let w = sim.world_mut();
    ensure_high_realm(w);
    ensure_forced_breakthrough(w);
}

/// 預跑 `GENESIS_PRERUN_TICK_END` 個月（4.6 步驟 6）。預跑期間的事件不輸出。
fn prerun(sim: &mut Sim) {
    sim.world_mut().prerun = true;
    for _ in 0..GENESIS_PRERUN_TICK_END {
        sim.step();
    }
    sim.world_mut().prerun = false;
    sim.drain_events();
}

fn era_index(era: Era) -> usize {
    match era {
        Era::Ancient => 0,
        Era::Middle => 1,
        Era::Decline => 2,
    }
}

/// 小、大尺寸的勢力數依州郡數等比例縮放（4.6）。
fn size_scale(size: WorldSize) -> f32 {
    let n = match size {
        WorldSize::Small => WG_PROVINCES_S,
        WorldSize::Medium => WG_PROVINCES_M,
        WorldSize::Large => WG_PROVINCES_L,
    };
    n as f32 / WG_PROVINCES_M as f32
}

/// 開局時代與尺寸決定的宗門數。
pub fn sect_count(era: Era, size: WorldSize) -> u32 {
    ((GENESIS_SECTS[era_index(era)] as f32 * size_scale(size)).round() as u32).max(1)
}

/// 開局時代與尺寸決定的初始有名之人數（不含上古另有的遊方僧、散儒）。
pub fn notable_count(era: Era, size: WorldSize) -> u32 {
    let cap = NOTABLE_CAP[match size {
        WorldSize::Small => 0,
        WorldSize::Medium => 1,
        WorldSize::Large => 2,
    }];
    (cap as f32 * GENESIS_NOTABLE_RATIO[era_index(era)]).round() as u32
}

// ── 抽樣輔助（genesis 子流）──────────────────────────────

fn uni(w: &mut World, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * w.rng.genesis.gen::<f32>()
}

/// 依分布抽一個州郡。
fn sample_province(w: &mut World, dist: &[f64]) -> ProvinceId {
    let total: f64 = dist.iter().sum();
    let r = w.rng.genesis.gen::<f64>() * total;
    let mut acc = 0.0;
    let mut last = 0usize;
    for (i, &p) in dist.iter().enumerate() {
        if p <= 0.0 {
            continue;
        }
        acc += p;
        last = i;
        if r < acc {
            return w.provinces[i].id;
        }
    }
    w.provinces[last].id
}

fn random_cell(w: &mut World, province: ProvinceId) -> u32 {
    let p = &w.provinces[province as usize];
    if p.cells.is_empty() {
        return p.seat;
    }
    let n = p.cells.len();
    let k = w.rng.genesis.gen_range(0..n);
    w.provinces[province as usize].cells[k]
}

/// 金字塔抽樣（4.6 步驟 3）：境界 r（`min..=8`）的權重 `GENESIS_PYRAMID^r`。
fn sample_realm(w: &mut World, min: u8) -> u8 {
    let weights: Vec<f32> = (min as usize..REALMS).map(|r| GENESIS_PYRAMID_RATIO.powi(r as i32)).collect();
    let total: f32 = weights.iter().sum();
    let mut x = w.rng.genesis.gen::<f32>() * total;
    for (i, wt) in weights.iter().enumerate() {
        if x < *wt {
            return min + i as u8;
        }
        x -= wt;
    }
    (REALMS - 1) as u8
}

// ── 山門 ─────────────────────────────────────────────────

/// 下一個宗門的山門（4.6 步驟 2）：以「含未佔用洞天福地、強度最高」（其次靈氣最高）的州郡為候選起點，
/// 套用 3.5 選址（`dao::choose_seat`）。起點的州郡 ID 小者優先。不消耗亂數。
fn next_seat(w: &World) -> Option<dao::Seat> {
    let used: BTreeSet<u16> = w
        .powers
        .iter()
        .filter(|p| p.fallen.is_none())
        .filter_map(|p| match &p.kind {
            PowerKind::Sect(s) => s.seat_node,
        })
        .collect();
    let mut best_node: Vec<f32> = vec![-1.0; w.provinces.len()];
    for (n, node) in w.geo.nodes.iter().enumerate() {
        if used.contains(&(n as u16)) {
            continue;
        }
        let Some(&prov) = w.geo.province_of.get(node.cell as usize) else { continue };
        if let Some(slot) = best_node.get_mut(prov as usize) {
            *slot = slot.max(node.strength);
        }
    }
    let mut order: Vec<(f32, f32, ProvinceId)> =
        w.provinces.iter().map(|p| (best_node[p.id as usize], p.aura, p.id)).collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.total_cmp(&a.1)).then(a.2.cmp(&b.2)));
    order.into_iter().find_map(|(_, _, id)| dao::choose_seat(w, id, None))
}

/// 以成員戰力即時計算的聲望（3.5：`Σ √戰力`；開天時尚無飛升者）。
fn live_prestige(w: &World, sect: PowerId) -> f32 {
    w.powers[sect as usize].members.iter().map(|&m| people::power_of(w, m).sqrt()).sum()
}

// ── 有名之人 ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// 宗門掌門（境界 ≥ 金丹）。
    Founder,
    /// 宗門成員。
    Member(PowerId),
    /// 依 3.4 入道規則決定身分。
    Free,
    /// 上古另有的遊方僧、散儒。
    Monk,
    Scholar,
}

/// 建立一名開天的有名之人：先天魂（4.6 步驟 4）、肉身、身分、境界、修為、年齡、`Born` 事件。
/// 道號與宗門掌門的創立由呼叫端處理。
fn make_person(w: &mut World, province: ProvinceId, slot: Slot) -> PersonId {
    let cell = random_cell(w, province);
    let sid = soul::create_soul(w, SoulOrigin::Primordial, province);
    let merit = uni(w, GENESIS_MERIT_MIN, GENESIS_MERIT_MAX);
    let sin = uni(w, GENESIS_SIN_MIN, GENESIS_SIN_MAX);
    let wisdom = uni(w, GENESIS_WISDOM_MIN, GENESIS_WISDOM_MAX);
    {
        let s = &mut w.souls[sid as usize];
        s.merit = merit;
        s.sin = sin;
        s.wisdom = wisdom;
    }
    // 屬性吃 W 與 G−S（3.4），所以先設魂再出生；身分決定後，修士的 S 減半，德行同步修正。
    let pid = people::spawn(w, sid, province, cell);
    soul::attach_person(w, sid, pid);

    let role = match slot {
        Slot::Founder | Slot::Member(_) => Role::Cultivator,
        Slot::Free => people::enter_path(w, pid),
        Slot::Monk => {
            let p = &mut w.persons[pid as usize];
            p.role = Role::Monk;
            p.path = PathState::Monk { fruit: 0, practice: 0.0 };
            p.lifespan += MONK_LIFESPAN_BONUS as u16;
            Role::Monk
        }
        Slot::Scholar => {
            let p = &mut w.persons[pid as usize];
            p.role = Role::Scholar;
            p.path = PathState::Scholar { virtue: 0.0, deed: 0.0, word: 0.0 };
            Role::Scholar
        }
    };
    let cultivator = matches!(role, Role::Cultivator | Role::Demonic);
    let bias = match role {
        Role::Cultivator => Some(Teaching::Tao),
        Role::Monk => Some(Teaching::Buddha),
        Role::Scholar => Some(Teaching::Ru),
        Role::Demonic => Some(Teaching::Demon),
        _ => None,
    };

    // 先天魂：修士 S × 0.5；道緣 = normalize(faith4 × 0.5 + 身分偏向 × 0.5)。
    let old_sin = w.souls[sid as usize].sin;
    if cultivator {
        w.souls[sid as usize].sin = old_sin * GENESIS_SIN_CULTIVATOR_MULT;
        let delta = (old_sin - w.souls[sid as usize].sin) / ATTR_VIRTUE_NET_DIV;
        let v = &mut w.persons[pid as usize].attrs.virtue;
        *v = (*v + delta).clamp(0.0, 10.0);
    }
    let f4 = pull::faith4(&w.provinces[province as usize]);
    let mut status = [0.25f32; 4];
    if let Some(t) = bias {
        status = [0.0; 4];
        status[t as usize] = GENESIS_AFFINITY_STATUS_BIAS;
    }
    let mut aff = [0.0f32; 4];
    for k in 0..4 {
        aff[k] = f4[k] * GENESIS_AFFINITY_FAITH_WEIGHT + status[k] * GENESIS_AFFINITY_STATUS_WEIGHT;
    }
    w.souls[sid as usize].affinity = pull::normalize4(aff);

    // 修士：境界（金字塔）、修為（境界內均勻）、壽元。
    if cultivator {
        let min = if slot == Slot::Founder { SECT_FOUND_MIN_REALM } else { 0 };
        let realm = sample_realm(w, min);
        let exp = uni(w, 0.0, REALM_EXP[realm as usize]).min(REALM_EXP[realm as usize] * 0.999);
        let p = &mut w.persons[pid as usize];
        p.role = role;
        p.path = PathState::Cultivator { realm, exp, last_attempt: None };
        p.lifespan = p.lifespan.max(REALM_LIFESPAN[realm as usize] as u16);
    }

    // 年齡：均勻分布在 12 到壽元的 60%。
    let lifespan = w.persons[pid as usize].lifespan;
    let max_age = ((lifespan as f32 * GENESIS_AGE_MAX_LIFESPAN_RATIO).floor() as u32).max(GENESIS_AGE_MIN);
    let age = w.rng.genesis.gen_range(GENESIS_AGE_MIN..=max_age);
    let months = w.rng.genesis.gen_range(0..12u32);
    w.persons[pid as usize].born = -((age * 12 + months) as i32);

    if let Slot::Member(sect) = slot {
        dao::join_sect(w, pid, sect);
    }

    let p = &w.persons[pid as usize];
    let mut e = NewEvent::new(EventKind::Born, 1, EventArgs::None);
    e.province = Some(p.province);
    e.cell = Some(p.cell);
    e.souls = smallvec![sid];
    e.persons = smallvec![pid];
    ledger::record(w, e);
    pid
}

fn living_count(w: &World) -> usize {
    w.persons.iter().filter(|p| p.alive()).count()
}

/// 4.6 步驟 2、3：宗門與其成員，再補足其餘有名之人。回傳開天事件的 ID 之前，先建立事件。
fn build_people(w: &mut World) {
    let era = w.params.era;
    let size = w.params.size;
    let pop = pull::population_dist(w);

    // 宗門：掌門 → 成員補滿至 SECT_SIZE。
    for _ in 0..sect_count(era, size) {
        let Some(seat) = next_seat(w) else { break };
        let founder = make_person(w, seat.province, Slot::Founder);
        let sect = dao::found_sect(w, founder, seat);
        while w.powers[sect as usize].members.len() < people::sect_capacity(live_prestige(w, sect)) {
            make_person(w, seat.province, Slot::Member(sect));
        }
        let pr = live_prestige(w, sect);
        w.powers[sect as usize].prestige = pr;
    }

    // 其餘有名之人：依人口分布出生，依 3.4 入道規則決定身分。
    let target = notable_count(era, size) as usize;
    while living_count(w) < target {
        let prov = sample_province(w, &pop);
        make_person(w, prov, Slot::Free);
    }

    // 上古另有遊方僧與散儒各 20（依州郡數縮放）。
    if era == Era::Ancient {
        let scale = size_scale(size);
        let monks = (GENESIS_ROAMING_MONKS_ANCIENT as f32 * scale).round() as u32;
        let scholars = (GENESIS_ROAMING_SCHOLARS_ANCIENT as f32 * scale).round() as u32;
        for _ in 0..monks {
            let prov = sample_province(w, &pop);
            make_person(w, prov, Slot::Monk);
        }
        for _ in 0..scholars {
            let prov = sample_province(w, &pop);
            make_person(w, prov, Slot::Scholar);
        }
    }
}

// ── 開局宿緣 ─────────────────────────────────────────────

fn has_open_thread(w: &World, kind: ThreadKind, a: SoulId, b: SoulId) -> bool {
    w.souls[a as usize].threads.iter().any(|&tid| {
        let t = &w.threads[tid as usize];
        t.kind == kind && thread::is_open(t) && ((t.debtor == a && t.creditor == b) || (t.debtor == b && t.creditor == a))
    })
}

/// 4.6 步驟 5：`GENESIS_THREADS` = 有名之人數 × 0.1 條線，種類依序輪流、雙方距離 3 步內、欠債方隨機。
fn build_threads(w: &mut World, genesis_event: EventId) {
    let persons: Vec<PersonId> = w.persons.iter().filter(|p| p.alive()).map(|p| p.id).collect();
    if persons.len() < 2 {
        return;
    }
    let count = (persons.len() as f32 * GENESIS_THREADS_PER_NOTABLE).round() as usize;
    let mut by_province: Vec<Vec<PersonId>> = vec![Vec::new(); w.provinces.len()];
    for &p in &persons {
        by_province[w.persons[p as usize].province as usize].push(p);
    }
    let mut near_cache: Vec<Option<Vec<ProvinceId>>> = vec![None; w.provinces.len()];
    for i in 0..count {
        let kind = THREAD_KINDS[i % THREAD_KINDS.len()];
        for _ in 0..THREAD_ATTEMPTS {
            let a = persons[w.rng.genesis.gen_range(0..persons.len())];
            let from = w.persons[a as usize].province;
            if near_cache[from as usize].is_none() {
                near_cache[from as usize] = Some(dao::provinces_within(w, from, GENESIS_THREAD_RANGE_STEPS));
            }
            let sa = w.persons[a as usize].soul;
            let cands: Vec<PersonId> = near_cache[from as usize]
                .as_ref()
                .into_iter()
                .flatten()
                .flat_map(|&pv| by_province[pv as usize].iter().copied())
                .filter(|&b| w.persons[b as usize].soul != sa)
                .collect();
            if cands.is_empty() {
                continue;
            }
            let b = cands[w.rng.genesis.gen_range(0..cands.len())];
            let sb = w.persons[b as usize].soul;
            if has_open_thread(w, kind, sa, sb) {
                continue;
            }
            let (debtor, creditor) = if w.rng.genesis.gen::<bool>() { (sa, sb) } else { (sb, sa) };
            thread::create_genesis(w, kind, debtor, creditor, genesis_event);
            break;
        }
    }
}

/// 4.6 步驟 1–5：天道魂、`Genesis` 事件、宗門、有名之人、先天魂、開局宿緣、道號。不含預跑與開局保證。
pub fn build(w: &mut World) {
    soul::ensure_heaven(w);
    w.tick = 0;
    let genesis_event = ledger::record(w, NewEvent::new(EventKind::Genesis, 1, EventArgs::None));
    build_people(w);
    build_threads(w, genesis_event);
    // 7.3：金丹以上者的道號（宗門已定，正邪可判）。
    for pid in 0..w.persons.len() as PersonId {
        people::assign_title(w, pid);
    }
}

// ── 開局保證 ─────────────────────────────────────────────

fn realm_of(p: &Person) -> Option<u8> {
    match p.path {
        PathState::Cultivator { realm, .. } => Some(realm),
        _ => None,
    }
}

/// 4.6 步驟 8：中古、末法至少 `GENESIS_HIGH_REALM_COUNT` 名在世的渡劫以上修士。
/// 不足時，把在世修士中境界最高者（同值取 ID 小者）提升為渡劫。回傳提升的人數。
pub fn ensure_high_realm(w: &mut World) -> u32 {
    if w.params.era == Era::Ancient {
        return 0;
    }
    let have = w
        .persons
        .iter()
        .filter(|p| p.alive() && realm_of(p).is_some_and(|r| r >= GENESIS_HIGH_REALM_MIN_REALM))
        .count() as u32;
    if have >= GENESIS_HIGH_REALM_COUNT {
        return 0;
    }
    let mut cands: Vec<(u8, PersonId)> = w
        .persons
        .iter()
        .filter(|p| p.alive() && p.role == Role::Cultivator && p.forced_breakthrough.is_none())
        .filter_map(|p| realm_of(p).filter(|&r| r < GENESIS_HIGH_REALM_MIN_REALM).map(|r| (r, p.id)))
        .collect();
    cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let need = (GENESIS_HIGH_REALM_COUNT - have) as usize;
    let mut done = 0;
    for &(_, pid) in cands.iter().take(need) {
        let realm = GENESIS_HIGH_REALM_MIN_REALM;
        let exp = uni(w, 0.0, REALM_EXP[realm as usize]).min(REALM_EXP[realm as usize] * 0.999);
        let p = &mut w.persons[pid as usize];
        p.path = PathState::Cultivator { realm, exp, last_attempt: None };
        p.lifespan = p.lifespan.max(REALM_LIFESPAN[realm as usize] as u16);
        people::assign_title(w, pid);
        done += 1;
    }
    done
}

/// 聲望最高的在世宗門（同值取 ID 小者）。
fn top_sect(w: &World) -> Option<PowerId> {
    let mut best: Option<(f32, PowerId)> = None;
    for p in &w.powers {
        if p.fallen.is_some() || !matches!(p.kind, PowerKind::Sect(_)) {
            continue;
        }
        let pr = dao::effective_prestige(p);
        if best.is_none_or(|(b, _)| pr > b) {
            best = Some((pr, p.id));
        }
    }
    best.map(|b| b.1)
}

/// 4.6 步驟 7：鏡頭初始位置（聲望最高的宗門山門）。沒有宗門時回傳 `None`。
pub fn camera_start(w: &World) -> Option<(ProvinceId, u32)> {
    top_sect(w).map(|id| (w.powers[id as usize].seat_province, w.powers[id as usize].seat_cell))
}

/// 築基期（索引 1）且修為最高者（同值取 ID 小者）。
fn best_foundation(w: &World, ids: impl Iterator<Item = PersonId>) -> Option<PersonId> {
    let mut best: Option<(f32, PersonId)> = None;
    for id in ids {
        let p = &w.persons[id as usize];
        if !p.alive() || p.forced_breakthrough.is_some() {
            continue;
        }
        if let PathState::Cultivator { realm: 1, exp, .. } = p.path {
            if best.is_none_or(|(b, _)| exp > b) {
                best = Some((exp, id));
            }
        }
    }
    best.map(|b| b.1)
}

/// 4.6 步驟 7：在聲望最高的宗門（沒有則鏡頭 2 州內）挑一名築基巔峰的修士，修為設為已滿、冷卻清除，
/// 排定 tick `GENESIS_FORCED_BT_TICK` 的強制突破；沒有合格者時建立一名（新魂，12 歲）。
/// 回傳該人物；世界沒有宗門時回傳 `None`。
pub fn ensure_forced_breakthrough(w: &mut World) -> Option<PersonId> {
    let sect = top_sect(w)?;
    let (seat_province, members) = {
        let p = &w.powers[sect as usize];
        (p.seat_province, p.members.clone())
    };
    let mut pick = best_foundation(w, members.iter().copied());
    if pick.is_none() {
        let near = dao::provinces_within(w, seat_province, 2);
        let rogues: Vec<PersonId> = w
            .persons
            .iter()
            .filter(|p| p.alive() && p.power.is_none() && p.role == Role::Cultivator && near.contains(&p.province))
            .map(|p| p.id)
            .collect();
        pick = best_foundation(w, rogues.into_iter());
    }
    let pid = match pick {
        Some(pid) => pid,
        None => {
            let (_, pid) = soul::spawn_new_life(w, SoulOrigin::New, seat_province);
            let tick = w.tick as i32;
            let p = &mut w.persons[pid as usize];
            p.born = tick - (GENESIS_AGE_MIN * 12) as i32;
            p.role = Role::Cultivator;
            p.lifespan = p.lifespan.max(REALM_LIFESPAN[1] as u16);
            dao::join_sect(w, pid, sect);
            pid
        }
    };
    let p = &mut w.persons[pid as usize];
    p.path = PathState::Cultivator { realm: 1, exp: REALM_EXP[1], last_attempt: None };
    p.forced_breakthrough = Some(GENESIS_FORCED_BT_TICK);
    Some(pid)
}
