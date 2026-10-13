//! GDD 3.5 宗門：創立、選址、招收、補足、拜師、聲望與範圍、繼任與分裂、覆滅。

use std::collections::BTreeSet;
use std::sync::OnceLock;

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use smallvec::smallvec;

use super::demon::leave_sect;
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::people::{self, sect_capacity};
use crate::schedule::{Domain, Round};
use crate::sim::World;
use crate::thread;
use crate::types::{NodeId, ProvinceId};

const ORGS: &str = include_str!("../../data/orgs.zh-Hant.json");
/// 7.3 命名重名時最多重抽 20 次。
const NAME_RETRIES: u32 = 20;
/// 7.3 命名仍重名時加的前綴。
const NAME_FALLBACK_PREFIX: &str = "新";
/// 7.2 `PowerFounded`、`Succession`、`SectSplit` 重要度 3；`PowerFallen` 4。
const FOUNDED_IMPORTANCE: u8 = 3;
const SUCCESSION_IMPORTANCE: u8 = 3;
const SPLIT_IMPORTANCE: u8 = 3;
const FALLEN_IMPORTANCE: u8 = 4;
/// 入門年齡：補足的新魂以 12 歲入道（3.4 來源第 2 項）。
const FILL_AGE_YEARS: u32 = COMING_OF_AGE_YEARS;

/// 山門選址結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seat {
    pub province: ProvinceId,
    pub cell: u32,
    pub node: Option<NodeId>,
}

pub fn sect_of(p: &Power) -> Option<&Sect> {
    match &p.kind {
        PowerKind::Sect(s) => Some(s),
    }
}

fn is_live_sect(p: &Power) -> bool {
    p.fallen.is_none() && sect_of(p).is_some()
}

/// 有效聲望 = 公式值 + 修正值（GDD 3.5）。
pub fn effective_prestige(p: &Power) -> f32 {
    p.prestige + p.prestige_mod
}

/// 沿陸鄰 `steps` 步內（含起點）的州郡，依 ID 升冪。
pub fn provinces_within(w: &World, from: ProvinceId, steps: u32) -> Vec<ProvinceId> {
    let n = w.provinces.len();
    if from as usize >= n {
        return Vec::new();
    }
    let mut seen = vec![false; n];
    seen[from as usize] = true;
    let mut frontier = vec![from];
    for _ in 0..steps {
        let mut next = Vec::new();
        for &p in &frontier {
            for &q in &w.provinces[p as usize].neighbors {
                if !seen[q as usize] {
                    seen[q as usize] = true;
                    next.push(q);
                }
            }
        }
        frontier = next;
    }
    (0..n as ProvinceId).filter(|&i| seen[i as usize]).collect()
}

/// 宗門勢力範圍（GDD 3.5）：山門州郡出發 `radius` 步內的州郡，依 ID 升冪。
pub fn sect_range(w: &World, id: PowerId) -> Vec<ProvinceId> {
    let Some(p) = w.powers.get(id as usize) else { return Vec::new() };
    let r = sect_of(p).map_or(0, |s| s.radius as u32);
    provinces_within(w, p.seat_province, r)
}

/// 宗門勢力範圍的快取（以宗門編號為索引，用到才算；效能快取，不納入雜湊與存檔）。
/// 範圍只由山門州郡、半徑與州郡鄰接決定，所以每項記下算出它時的（山門州郡，半徑，州郡數），
/// 對不上就重算；州郡鄰接在開天後不變。`World` 保存一份，月與月之間沿用。
#[derive(Debug, Clone, Default)]
pub struct RangeCache(Vec<Option<RangeEntry>>);

#[derive(Debug, Clone)]
struct RangeEntry {
    seat: ProvinceId,
    radius: u8,
    provinces: usize,
    range: Vec<ProvinceId>,
}

impl RangeCache {
    pub(super) fn ensure(&mut self, w: &World, id: PowerId) {
        let i = id as usize;
        if self.0.len() <= i {
            self.0.resize(i + 1, None);
        }
        let Some(p) = w.powers.get(i) else {
            self.0[i] = Some(RangeEntry { seat: 0, radius: 0, provinces: usize::MAX, range: Vec::new() });
            return;
        };
        let radius = sect_of(p).map_or(0, |s| s.radius);
        let valid = self.0[i].as_ref().is_some_and(|e| e.seat == p.seat_province && e.radius == radius && e.provinces == w.provinces.len());
        if !valid {
            self.0[i] = Some(RangeEntry { seat: p.seat_province, radius, provinces: w.provinces.len(), range: sect_range(w, id) });
        }
    }

    /// 須先 [`RangeCache::ensure`]。
    pub(super) fn at(&self, id: PowerId) -> &[ProvinceId] {
        self.0.get(id as usize).and_then(|x| x.as_ref()).map_or(&[], |e| e.range.as_slice())
    }
}

// ── 命名 ─────────────────────────────────────────────────

struct OrgNames {
    good: (Vec<String>, Vec<String>),
    evil: (Vec<String>, Vec<String>),
    suffixes: Vec<(String, u32)>,
}

fn strs(v: &serde_json::Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

fn org_names() -> &'static OrgNames {
    static N: OnceLock<OrgNames> = OnceLock::new();
    N.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(ORGS).expect("orgs.zh-Hant.json");
        let pre = &v["prefix_chars_by_type_and_faction"]["修煉"];
        let suffixes = v["type_suffixes"]["修煉"]["suffixes"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| Some((x["text"].as_str()?.to_owned(), x["weight"].as_u64()? as u32))).collect())
            .unwrap_or_default();
        OrgNames {
            good: (strs(&pre["正道"]["first"]), strs(&pre["正道"]["second"])),
            evil: (strs(&pre["魔門"]["first"]), strs(&pre["魔門"]["second"])),
            suffixes,
        }
    })
}

/// GDD 7.3 宗門名：兩字 + 宗／門／派／閣／谷／洞（權重）；邪道取魔門字庫。與現有勢力重名時重抽，最多 20 次，
/// 仍重名加「新」。只用傳入的亂數子流（`powers`）。
pub fn sect_name(w: &mut World, evil: bool) -> String {
    let n = org_names();
    let (first, second) = if evil { &n.evil } else { &n.good };
    let total: u32 = n.suffixes.iter().map(|s| s.1).sum();
    let mut name = String::new();
    for _ in 0..NAME_RETRIES {
        let rng: &mut ChaCha8Rng = &mut w.rng.powers;
        let a = &first[rng.gen_range(0..first.len())];
        let b = &second[rng.gen_range(0..second.len())];
        let mut x = rng.gen_range(0..total);
        let mut suf = &n.suffixes[0].0;
        for (t, wt) in &n.suffixes {
            if x < *wt {
                suf = t;
                break;
            }
            x -= wt;
        }
        name = format!("{a}{b}{suf}");
        if !w.powers.iter().any(|p| p.name == name) {
            return name;
        }
    }
    format!("{NAME_FALLBACK_PREFIX}{name}")
}

// ── 選址 ─────────────────────────────────────────────────

fn node_province(w: &World, n: usize) -> Option<ProvinceId> {
    w.geo.province_of.get(w.geo.nodes[n].cell as usize).copied()
}

/// GDD 3.5 宗門創立選址（`SECT_SITE`；創立與開天共用）。
///
/// - 候選：`origin` 及 2 步內的州郡，且山門與其他在世宗門（`ignore` 除外）山門相距 ≥ 2 步。
/// - 優先「含未被在世宗門佔用的洞天福地」的州，山門格 = 強度最高的節點格（強度相同取節點 ID 小者；
///   跨州時取最強者，再同則州 ID 小者）。
/// - 否則取靈氣最高的州（同值取 ID 小者），山門格 = 該州格點靈氣最高的格（無格點資料取州治格）。
/// - 無候選州回傳 `None`。不消耗亂數。
pub fn choose_seat(w: &World, origin: ProvinceId, ignore: Option<PowerId>) -> Option<Seat> {
    let mut blocked: BTreeSet<ProvinceId> = BTreeSet::new();
    let mut used_nodes: BTreeSet<NodeId> = BTreeSet::new();
    for p in w.powers.iter().filter(|p| is_live_sect(p)) {
        if let Some(n) = sect_of(p).and_then(|s| s.seat_node) {
            used_nodes.insert(n);
        }
        if Some(p.id) != ignore {
            blocked.extend(provinces_within(w, p.seat_province, SECT_FOUND_MIN_SPACING_STEPS - 1));
        }
    }
    let cands: Vec<ProvinceId> =
        provinces_within(w, origin, SECT_FOUND_MIN_SPACING_STEPS).into_iter().filter(|c| !blocked.contains(c)).collect();
    if cands.is_empty() {
        return None;
    }
    // 含空閒節點的州
    let mut best: Option<(f32, ProvinceId, NodeId)> = None;
    for n in 0..w.geo.nodes.len() {
        let Some(prov) = node_province(w, n) else { continue };
        if !cands.contains(&prov) || used_nodes.contains(&(n as NodeId)) {
            continue;
        }
        let s = w.geo.nodes[n].strength;
        let better = match best {
            None => true,
            Some((bs, bp, _)) => s > bs || (s == bs && prov < bp),
        };
        if better {
            best = Some((s, prov, n as NodeId));
        }
    }
    if let Some((_, province, node)) = best {
        return Some(Seat { province, cell: w.geo.nodes[node as usize].cell, node: Some(node) });
    }
    let mut top: Option<(f32, ProvinceId)> = None;
    for &c in &cands {
        let a = w.provinces[c as usize].aura;
        if top.is_none_or(|(ba, _)| a > ba) {
            top = Some((a, c));
        }
    }
    let province = top?.1;
    let pr = &w.provinces[province as usize];
    let mut cell = pr.seat;
    let mut ba = f32::MIN;
    for &c in &pr.cells {
        if let Some(&a) = w.geo.aura_cell.get(c as usize) {
            if a > ba {
                ba = a;
                cell = c;
            }
        }
    }
    Some(Seat { province, cell, node: None })
}

// ── 創立、加入 ───────────────────────────────────────────

/// 加入宗門：設定 `power`、`home`，非遊歷者移到山門格；加入成員。不拜師、不寫事件。
pub fn join_sect(w: &mut World, pid: PersonId, sect: PowerId) {
    let (seat_province, seat_cell) = {
        let pw = &w.powers[sect as usize];
        (pw.seat_province, pw.seat_cell)
    };
    w.powers[sect as usize].members.push(pid);
    let p = &mut w.persons[pid as usize];
    p.power = Some(sect);
    p.home = seat_province;
    if p.travel_until.is_none() {
        p.province = seat_province;
        p.cell = seat_cell;
    }
}

fn init_relations(w: &mut World, id: PowerId, evil: bool) {
    let others: Vec<(PowerId, bool)> =
        w.powers.iter().filter(|p| p.id != id && is_live_sect(p)).map(|p| (p.id, sect_of(p).is_some_and(|s| s.evil))).collect();
    for (oid, oevil) in others {
        if oevil != evil {
            w.powers[id as usize].relations.insert(oid, EVIL_SECT_RELATION as i8);
            w.powers[oid as usize].relations.insert(id, EVIL_SECT_RELATION as i8);
        }
    }
}

fn formula_prestige(w: &World, p: &Power) -> f32 {
    let members: f32 = p.members.iter().map(|&m| people::power_of(w, m).sqrt()).sum();
    members + sect_of(p).map_or(0.0, |s| s.ascended as f32) * PRESTIGE_ASCENDED
}

/// 建立宗門：`founder` 為掌門與首位成員，邪正依其魂的淨業力（< 0 為邪道）。
/// 寫入 `PowerFounded`，與異道宗門關係初值 −50。回傳新宗門 ID。
pub fn found_sect(w: &mut World, founder: PersonId, seat: Seat) -> PowerId {
    let id = create_sect(w, founder, seat);
    let p = &w.persons[founder as usize];
    let mut e = NewEvent::new(EventKind::PowerFounded, FOUNDED_IMPORTANCE, EventArgs::Sect { node: seat.node });
    e.province = Some(seat.province);
    e.cell = Some(seat.cell);
    e.souls = smallvec![p.soul];
    e.persons = smallvec![founder];
    e.powers = smallvec![id];
    if let Some(c) = p.last_event {
        e.causes = smallvec![c];
    }
    ledger::record(w, e);
    id
}

/// 建立宗門本體（不寫事件）：掌門、成員、邪正、與異道宗門的關係。
fn create_sect(w: &mut World, founder: PersonId, seat: Seat) -> PowerId {
    let tick = w.tick;
    let soul = w.persons[founder as usize].soul;
    let evil = w.souls[soul as usize].net_karma() < 0.0;
    let name = sect_name(w, evil);
    let id = w.powers.len() as PowerId;
    w.powers.push(Power {
        id,
        kind: PowerKind::Sect(Sect { evil, seat_node: seat.node, radius: SECT_RADIUS_BASE as u8, ascended: 0 }),
        name,
        seat_province: seat.province,
        seat_cell: seat.cell,
        leader: Some(founder),
        members: Vec::new(),
        founded: tick,
        fallen: None,
        prestige: 0.0,
        prestige_mod: 0.0,
        relations: Default::default(),
    });
    join_sect(w, founder, id);
    // 創立者原有的師徒關係保留；掌門不需拜師
    let pw = &w.powers[id as usize];
    let p0 = formula_prestige(w, pw);
    w.powers[id as usize].prestige = p0;
    init_relations(w, id, evil);
    id
}

/// GDD 6.3 步驟 15：散修（金丹以上）每年 3% 創立宗門。
pub(super) fn yearly_found(w: &mut World, round: Round) {
    for i in w.round_alive(round).into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !p.alive() || p.role != Role::Cultivator || p.power.is_some() {
            continue;
        }
        let PathState::Cultivator { realm, .. } = p.path else { continue };
        if realm < SECT_FOUND_MIN_REALM {
            continue;
        }
        if w.rng.powers.gen::<f32>() >= SECT_FOUND_CHANCE {
            continue;
        }
        let (pid, prov) = (p.id, p.province);
        if let Some(seat) = choose_seat(w, prov, None) {
            found_sect(w, pid, seat);
        }
    }
}

// ── 招收、補足、拜師 ─────────────────────────────────────

fn realm_of(w: &World, pid: PersonId) -> u8 {
    match w.persons[pid as usize].path {
        PathState::Cultivator { realm, .. } => realm,
        _ => 0,
    }
}

/// GDD 6.3 步驟 16：散修投靠 → 補足 → 拜師。
pub(super) fn yearly_recruit(w: &mut World, round: Round, ranges: &mut RangeCache) {
    join_rogues(w, round, ranges);
    fill_sects(w, round);
    assign_masters(w, round);
}

/// 散修每年 2% 投靠：勢力範圍涵蓋其所在州、未滿員的宗門中，有效聲望最高者（同值取 ID 小者）。魔修不投靠。
pub(super) fn join_rogues(w: &mut World, round: Round, ranges: &mut RangeCache) {
    for i in w.round_alive(round).into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !p.alive() || p.role != Role::Cultivator || p.power.is_some() || !matches!(p.path, PathState::Cultivator { .. }) {
            continue;
        }
        if w.rng.powers.gen::<f32>() >= SECT_JOIN_ROGUE_CHANCE {
            continue;
        }
        let (pid, prov) = (p.id, p.province);
        let mut best: Option<(f32, PowerId)> = None;
        // 勢力範圍只在第一次有人通過機率抽樣時才算（單月輪次多半沒有人要投靠）；加入宗門不改變範圍。
        for idx in 0..w.powers.len() {
            let pw = &w.powers[idx];
            if !is_live_sect(pw) {
                continue;
            }
            let id = pw.id;
            ranges.ensure(w, id);
            let pw = &w.powers[idx];
            if pw.fallen.is_some()
                || ranges.at(id).binary_search(&prov).is_err()
                || pw.members.len() >= sect_capacity(effective_prestige(pw))
            {
                continue;
            }
            let pr = effective_prestige(pw);
            if best.is_none_or(|(bp, _)| pr > bp) {
                best = Some((pr, id));
            }
        }
        if let Some((_, id)) = best {
            join_sect(w, pid, id);
        }
    }
}

/// 補足的人選上限年齡：凡人要年輕到還能當弟子。
const FILL_MAX_AGE_YEARS: u32 = 30;

/// 補足的人選：山門州郡內、沒有勢力的在世者——已滿 12 歲仍是幼年的，或 30 歲以下的凡人（散人）；
/// 取根骨最高者，同值取 ID 小者。不憑空造人：挑不到就是 `None`。
fn local_recruit(w: &World, province: ProvinceId) -> Option<PersonId> {
    let tick = w.tick;
    let mut best: Option<(f32, PersonId)> = None;
    for &i in &w.alive_ids {
        let p = &w.persons[i as usize];
        if p.province != province || p.power.is_some() || p.travel_until.is_some() {
            continue;
        }
        let age = p.age(tick);
        let fits = match p.role {
            Role::Child => age >= FILL_AGE_YEARS,
            Role::Commoner => matches!(p.path, PathState::Mortal) && age >= FILL_AGE_YEARS && age <= FILL_MAX_AGE_YEARS,
            _ => false,
        };
        if fits && best.is_none_or(|(r, _)| p.attrs.root > r) {
            best = Some((p.attrs.root, p.id));
        }
    }
    best.map(|b| b.1)
}

/// 補足（3.4 來源第 2 項）：成員少於規模的宗門，從山門州郡既有的人裡挑弟子入門（[`local_recruit`]），
/// 挑不到就不補；不再建立新魂。沒有成員的宗門不補（道統已斷，見覆滅）。
fn fill_sects(w: &mut World, round: Round) {
    let seed = w.params.seed;
    for idx in 0..w.powers.len() {
        let pw = &w.powers[idx];
        if !is_live_sect(pw) || !round.takes(seed, Domain::Sect, pw.id) || pw.members.is_empty() {
            continue;
        }
        let cap = sect_capacity(effective_prestige(pw));
        let (id, seat_province, seat_cell) = (pw.id, pw.seat_province, pw.seat_cell);
        let mut have = pw.members.len();
        while have < cap {
            let Some(pid) = local_recruit(w, seat_province) else { break };
            let p = &mut w.persons[pid as usize];
            p.role = Role::Cultivator;
            p.path = PathState::Cultivator { realm: 0, exp: 0.0, last_attempt: None };
            p.lifespan = p.lifespan.max(REALM_LIFESPAN[0] as u16);
            join_sect(w, pid, id);
            w.persons[pid as usize].cell = seat_cell;
            let p = &w.persons[pid as usize];
            let mut e = NewEvent::new(EventKind::ComeOfAge, 1, EventArgs::Role { role: Role::Cultivator });
            e.province = Some(p.province);
            e.cell = Some(p.cell);
            e.souls = smallvec![p.soul];
            e.persons = smallvec![pid];
            e.powers = smallvec![id];
            if let Some(c) = p.last_event {
                e.causes = smallvec![c];
            }
            ledger::record(w, e);
            have += 1;
        }
    }
}

/// 拜師：沒有師父的非掌門成員，拜本宗境界高於自己的長老（金丹以上、徒弟 < 5）；
/// 沒有長老時拜掌門。建立師徒關係，師父 G += 2，建師徒線。
fn assign_masters(w: &mut World, round: Round) {
    let seed = w.params.seed;
    for idx in 0..w.powers.len() {
        if !is_live_sect(&w.powers[idx]) || !round.takes(seed, Domain::Sect, w.powers[idx].id) {
            continue;
        }
        let members = w.powers[idx].members.clone();
        let leader = w.powers[idx].leader;
        for &m in &members {
            let p = &w.persons[m as usize];
            if !p.alive() || p.master.is_some() || Some(m) == leader {
                continue;
            }
            let my_realm = realm_of(w, m);
            let elders: Vec<PersonId> = w.powers[idx]
                .members
                .iter()
                .copied()
                .filter(|&e| {
                    let q = &w.persons[e as usize];
                    e != m
                        && q.alive()
                        && realm_of(w, e) >= SECT_ELDER_MIN_REALM
                        && realm_of(w, e) > my_realm
                        && (q.disciples.len() as u32) < MAX_DISCIPLES
                        && q.master != Some(m)
                })
                .collect();
            let master = if elders.is_empty() {
                match leader {
                    Some(l) if w.persons[l as usize].alive() && w.persons[l as usize].master != Some(m) => l,
                    _ => continue,
                }
            } else {
                elders[w.rng.powers.gen_range(0..elders.len())]
            };
            take_master(w, m, master);
        }
    }
}

fn take_master(w: &mut World, disciple: PersonId, master: PersonId) {
    w.persons[disciple as usize].master = Some(master);
    w.persons[master as usize].disciples.push(disciple);
    let (ds, ms) = (w.persons[disciple as usize].soul, w.persons[master as usize].soul);
    w.souls[ms as usize].merit += MERIT_DISCIPLE;
    let cause = w.persons[disciple as usize].last_event;
    thread::on_apprentice(w, ds, ms, cause);
}

// ── 繼任與分裂、覆滅 ─────────────────────────────────────

fn leader_valid(w: &World, p: &Power) -> bool {
    p.leader.is_some_and(|l| {
        let q = &w.persons[l as usize];
        q.alive() && q.power == Some(p.id)
    })
}

/// 戰力最高的在世成員（同值取 ID 小者），排除 `skip`。
fn strongest(w: &World, members: &[PersonId], skip: Option<PersonId>) -> Option<(PersonId, f32)> {
    let mut best: Option<(PersonId, f32)> = None;
    for &m in members {
        if Some(m) == skip || !w.persons[m as usize].alive() {
            continue;
        }
        let pw = people::power_of(w, m);
        if best.is_none_or(|(_, bp)| pw > bp) {
            best = Some((m, pw));
        }
    }
    best
}

/// 宗門覆滅：成員成為散修，寫入 `PowerFallen`。
pub(super) fn fall_sect(w: &mut World, id: PowerId, cause: Option<EventId>) {
    let tick = w.tick;
    let members = std::mem::take(&mut w.powers[id as usize].members);
    for m in members {
        let p = &mut w.persons[m as usize];
        if p.power == Some(id) {
            p.power = None;
        }
    }
    let former_leader = w.powers[id as usize].leader.take();
    w.powers[id as usize].fallen = Some(tick);
    let pw = &w.powers[id as usize];
    let mut e = NewEvent::new(EventKind::PowerFallen, FALLEN_IMPORTANCE, EventArgs::None);
    e.province = Some(pw.seat_province);
    e.cell = Some(pw.seat_cell);
    e.powers = smallvec![id];
    if let Some(c) = cause {
        e.causes.push(c);
    } else if let Some(c) = former_leader.and_then(|l| w.persons[l as usize].last_event) {
        e.causes.push(c);
    }
    ledger::record(w, e);
}

/// GDD 3.5 傳承：首領缺位（`None`、已死或已離宗）的宗門，由戰力最高的成員繼任；
/// 然後若成員 ≥ 8 且次強者戰力 ≥ 繼任者 70%，35% 機率分裂。成員歸零者覆滅。
pub fn resolve_leaders(w: &mut World) {
    resolve_leaders_round(w, Round::All);
}

/// 同 [`resolve_leaders`]，只處理本輪輪到判定月的宗門（`Sim::step` 的每月宗門事務用）。
pub(super) fn resolve_leaders_round(w: &mut World, round: Round) {
    let seed = w.params.seed;
    for idx in 0..w.powers.len() {
        let pw = &w.powers[idx];
        if !is_live_sect(pw) || !round.takes(seed, Domain::Sect, pw.id) || leader_valid(w, pw) {
            continue;
        }
        let id = pw.id;
        let old = pw.leader;
        let members = pw.members.clone();
        let Some((new, new_power)) = strongest(w, &members, None) else {
            fall_sect(w, id, None);
            continue;
        };
        w.powers[idx].leader = Some(new);
        let cause = old.and_then(|o| w.persons[o as usize].last_event);
        let p = &w.persons[new as usize];
        let mut e = NewEvent::new(EventKind::Succession, SUCCESSION_IMPORTANCE, EventArgs::None);
        e.province = Some(w.powers[idx].seat_province);
        e.cell = Some(p.cell);
        e.souls = smallvec![p.soul];
        e.persons = smallvec![new];
        e.powers = smallvec![id];
        if let Some(c) = cause {
            e.causes.push(c);
        }
        let succ = ledger::record(w, e);
        // 分裂
        if members.len() as u32 >= SECT_SPLIT_MIN_MEMBERS {
            if let Some((second, sp)) = strongest(w, &members, Some(new)) {
                if sp >= new_power * SECT_SPLIT_SECOND_RATIO && w.rng.powers.gen::<f32>() < SECT_SPLIT_CHANCE {
                    split_sect(w, id, second, succ);
                }
            }
        }
    }
}

fn split_sect(w: &mut World, parent: PowerId, second: PersonId, cause: EventId) {
    let origin = w.persons[second as usize].province;
    let Some(seat) = choose_seat(w, origin, Some(parent)) else { return };
    let followers: Vec<PersonId> = w.persons[second as usize]
        .disciples
        .iter()
        .copied()
        .filter(|&d| w.persons[d as usize].alive() && w.persons[d as usize].power == Some(parent))
        .collect();
    // 次強者脫離本宗（師徒解除，但門下徒弟隨行，故先記下再還原）
    leave_sect(w, second);
    let id = create_sect(w, second, seat);
    for d in followers {
        w.powers[parent as usize].members.retain(|&m| m != d);
        w.persons[d as usize].power = None;
        join_sect(w, d, id);
        w.persons[d as usize].master = Some(second);
        w.persons[second as usize].disciples.push(d);
    }
    let rel = SECT_SPLIT_RELATION as i8;
    w.powers[id as usize].relations.insert(parent, rel);
    w.powers[parent as usize].relations.insert(id, rel);
    let p = &w.persons[second as usize];
    let mut e = NewEvent::new(EventKind::SectSplit, SPLIT_IMPORTANCE, EventArgs::Sect { node: seat.node });
    e.province = Some(seat.province);
    e.cell = Some(seat.cell);
    e.souls = smallvec![p.soul];
    e.persons = smallvec![second];
    e.powers = smallvec![id, parent];
    e.causes = smallvec![cause];
    ledger::record(w, e);
}

/// GDD 6.3 步驟 18：覆滅檢查（成員 0）→ 聲望重算 → 勢力範圍 → 修正值衰減。
pub(super) fn yearly_prestige(w: &mut World, round: Round) {
    let seed = w.params.seed;
    for idx in 0..w.powers.len() {
        if !is_live_sect(&w.powers[idx]) || !round.takes(seed, Domain::Sect, w.powers[idx].id) {
            continue;
        }
        if w.powers[idx].members.is_empty() {
            fall_sect(w, idx as PowerId, None);
            continue;
        }
        let f = formula_prestige(w, &w.powers[idx]);
        let pw = &mut w.powers[idx];
        pw.prestige = f;
        pw.prestige_mod *= PRESTIGE_MOD_DECAY;
        let eff = pw.prestige + pw.prestige_mod;
        let r = (SECT_RADIUS_BASE + (1.0 + eff.max(0.0) / SECT_RADIUS_PRESTIGE_DIV).log2().floor() as u32).min(SECT_RADIUS_MAX);
        let PowerKind::Sect(s) = &mut pw.kind;
        s.radius = r as u8;
    }
}
