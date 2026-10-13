//! GDD 3.3 宿緣（因果線）：建立、相遇、了結、冤冤相報、預兆（U7）。
//!
//! 所有亂數取自 `World::rng.people`（6.5：宿緣步驟與有名之人共用此子流；輪迴判定專用 `soul`）。
//! 迭代順序皆為 `Vec`／`BTreeSet`，依魂 ID、ThreadId 升冪。
//!
//! # 契約
//!
//! - 每個新 ThreadId 都會被推進**雙方魂**的 `Soul::threads`（U4 的因果牽引、利息、等待都靠它找線）。
//! - 「未了」＝ [`is_open`]：`resolved == None` 且 `strength > 0`。強度降到 0 的線已消散（`ThreadFaded`），不算了結。
//! - `lives` 由 [`lives`] 即時推得（3.3）。
//! - 事件寫入經 `ledger::record`，天書偵測（`book::detect`）自動進行。
//!
//! # 給其他系統的介面
//!
//! | 函式 | 誰呼叫 | 用途 |
//! |---|---|---|
//! | [`monthly`]、[`yearly`] | `Sim::step` 步驟 14 | 師徒超越檢查、相遇與了結、預兆；`monthly` 逐魂在自己的判定月處理，`yearly` 一次處理全部（測試與工具用） |
//! | [`on_kill`] | 任何人殺死有名之人後（people 死亡處理、dao 魔修襲殺與斬妖、war） | 殺業線：建立、復仇了結、冤冤相報；斬殺惡人不建線。**不**改 G／S，見 [`kill_karma`] |
//! | [`kill_karma`] | 同上（呼叫端統一計業時） | 依 3.1 計殺人的業力（惡人 G+5，否則 S+10×無辜係數） |
//! | [`on_peril`] | 會戰、天劫、妖襲、被襲殺時的債主 | 殺業化解：同州的欠債方依德行機率出手相救 |
//! | [`on_marriage`] | people 婚配 | 情緣線：lives ≥ 1 的未了線了結（再續前緣），否則建立／累加 |
//! | [`on_apprentice`] | dao／people 拜師（建立 `Person::master` 後） | 師徒線：lives ≥ 1 的同向未了線了結（再結師徒），並建立新線 |
//! | [`create_genesis`] | genesis | 開局宿緣：`created` 為 `Genesis` 事件，不寫 `ThreadCreated` |
//! | [`create`] | 司命牽線（M5）、怨靈（war）、恩義來源（M3+） | 建立或累加任一種線 |
//! | [`resolve`] | 其他了結來源（怨：攻滅仇人的勢力等） | 清算業力、寫入 `ThreadResolved`、偵測天書 |
//! | [`is_open`]、[`lives`]、[`default_strength`]、[`distance`] | 全部 | 查詢輔助 |
//!
//! # 與 people 的分工
//!
//! 死亡處理（`people::die`）、戰力（`people::combat_power`）、婚配（`people::marry`）由 people 提供；
//! 本模組在相遇衝突時呼叫它們。`people::die` 不計業、不建線，由 `kill_person` 的呼叫順序
//! （Killed 事件 → die → [`kill_karma`] → [`on_kill`]）補上；其他場景（war、dao 斬妖與魔修襲殺）的呼叫端
//! 請照同一順序，並以 `die` 回傳的死亡事件為因。`people::marry` 內部呼叫 [`on_marriage`]。

#[cfg(test)]
mod tests;
mod yearly;

pub use yearly::{monthly, yearly};

use rand::Rng;
use smallvec::{smallvec, SmallVec};

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::ProvinceId;

/// 了結的方式（決定業力清算，3.3 了結第 1 步）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// 殺業被復仇：欠債方 S −= 強度 × 0.5。
    Revenge,
    /// 殺業被化解：欠債方 S −= 強度。
    Atone,
    /// 其他種類：雙方各 G += 強度 × 0.2。
    Other,
}

// ── 查詢輔助 ─────────────────────────────────────────────

/// 未了：未了結且強度 > 0。
pub fn is_open(t: &Thread) -> bool {
    t.resolved.is_none() && t.strength > 0.0
}

/// 宿緣的 `lives`（3.3）：欠債方轉世增量；對稱線（情緣、師徒）取雙方增量較大者。
pub fn lives(w: &World, t: &Thread) -> u16 {
    let inc = |sid: SoulId, base: u16| w.souls.get(sid as usize).map_or(0, |s| s.rebirths().saturating_sub(base));
    let d = inc(t.debtor, t.lives_base.0);
    match t.kind {
        ThreadKind::Love | ThreadKind::Mentor => d.max(inc(t.creditor, t.lives_base.1)),
        _ => d,
    }
}

/// 各種類的初始強度（3.3；司命牽線另用 `THREAD_INIT_TIANJI`）。殺業的初始強度依死者身分，見 [`on_kill`]。
pub fn default_strength(kind: ThreadKind) -> f32 {
    match kind {
        ThreadKind::Feud => THREAD_INIT_KILL,
        ThreadKind::Grace => THREAD_INIT_GRACE,
        ThreadKind::Love => THREAD_INIT_LOVE,
        ThreadKind::Mentor => THREAD_INIT_MASTER,
        ThreadKind::Grudge => THREAD_INIT_FEUD,
    }
}

/// 沿陸鄰的步數（不含海鄰）；超過 `max`（目前支援 0..=2）回傳 `None`。
pub fn distance(w: &World, a: ProvinceId, b: ProvinceId, max: u32) -> Option<u32> {
    if a == b {
        return Some(0);
    }
    let na = &w.provinces.get(a as usize)?.neighbors;
    if na.contains(&b) {
        return (max >= 1).then_some(1);
    }
    if max >= 2 {
        for &m in na {
            if w.provinces.get(m as usize).is_some_and(|p| p.neighbors.contains(&b)) {
                return Some(2);
            }
        }
    }
    None
}

/// 魂目前在世的肉身（含幼年者）。
fn any_body(w: &World, sid: SoulId) -> Option<&Person> {
    match w.souls.get(sid as usize)?.state {
        SoulState::Incarnate(pid) => w.persons.get(pid as usize).filter(|p| p.alive()),
        _ => None,
    }
}

fn is_child(p: &Person) -> bool {
    p.role == Role::Child || matches!(p.path, PathState::Child)
}

/// 魂目前在世且非幼年的肉身（相遇、預兆的參與者）。
fn body(w: &World, sid: SoulId) -> Option<&Person> {
    any_body(w, sid).filter(|p| !is_child(p))
}

fn place(w: &World, sid: SoulId) -> (Option<ProvinceId>, Option<u32>) {
    match any_body(w, sid) {
        Some(p) => (Some(p.province), Some(p.cell)),
        None => w.souls.get(sid as usize).map_or((None, None), |s| (Some(s.last_place.0), Some(s.last_place.1))),
    }
}

/// 魂最近一世肉身的最近事件。
fn last_event_of(w: &World, sid: SoulId) -> Option<EventId> {
    let pid = *w.souls.get(sid as usize)?.lives.last()?;
    w.persons.get(pid as usize)?.last_event
}

fn roll(w: &mut World, p: f32) -> bool {
    w.rng.people.gen_range(0.0f32..1.0) < p
}

fn body_achievements(p: &Person) -> Achievements {
    Achievements {
        realm: match p.path {
            PathState::Cultivator { realm, .. } => Some(realm),
            _ => None,
        },
        fruit: match p.path {
            PathState::Monk { fruit, .. } => Some(fruit),
            _ => None,
        },
        immortality: match p.path {
            PathState::Scholar { virtue, deed, word } => virtue + deed + word,
            _ => 0.0,
        },
    }
}

/// 魂的最高成就：前世、已併入的今生與目前肉身。
fn soul_peak(w: &World, sid: SoulId) -> Achievements {
    let Some(s) = w.souls.get(sid as usize) else { return Achievements::default() };
    let mut a = Achievements {
        realm: s.best_prev.realm.max(s.best_now.realm),
        fruit: s.best_prev.fruit.max(s.best_now.fruit),
        immortality: s.best_prev.immortality.max(s.best_now.immortality),
    };
    if let Some(p) = any_body(w, sid) {
        let b = body_achievements(p);
        a.realm = a.realm.max(b.realm);
        a.fruit = a.fruit.max(b.fruit);
        a.immortality = a.immortality.max(b.immortality);
    }
    a
}

/// 師徒線「徒弟今生成就超過 `master_best`」的判定：逐項比較，且只比師父確實有成就的那一項
/// （師父沒有境界／果位／三不朽，就沒有可超越的基準；`Some(0) > None` 不算超越）。
fn exceeds(now: Achievements, best: Achievements) -> bool {
    let beats = |n: Option<u8>, b: Option<u8>| matches!((n, b), (Some(n), Some(b)) if n > b);
    beats(now.realm, best.realm) || beats(now.fruit, best.fruit) || (best.immortality > 0.0 && now.immortality > best.immortality)
}

fn open_between(w: &World, kind: ThreadKind, a: SoulId, b: SoulId, directed: bool) -> Option<ThreadId> {
    let s = w.souls.get(a as usize)?;
    s.threads.iter().copied().find(|&tid| {
        w.threads.get(tid as usize).is_some_and(|t| {
            t.kind == kind && is_open(t) && ((t.debtor == a && t.creditor == b) || (!directed && t.debtor == b && t.creditor == a))
        })
    })
}

// ── 建立 ─────────────────────────────────────────────────

/// 把一條線的資料推進世界：`threads`、雙方魂的 `threads`、殺業對計數。不寫事件。
fn push_thread(
    w: &mut World,
    kind: ThreadKind,
    debtor: SoulId,
    creditor: SoulId,
    strength: f32,
    woven: bool,
    created: EventId,
) -> ThreadId {
    let tid = w.threads.len() as ThreadId;
    let lives_base = (w.souls[debtor as usize].rebirths(), w.souls[creditor as usize].rebirths());
    let master_best = (kind == ThreadKind::Mentor).then(|| soul_peak(w, creditor));
    w.threads.push(Thread {
        id: tid,
        kind,
        debtor,
        creditor,
        strength: strength.clamp(0.0, THREAD_STRENGTH_CAP),
        created: (w.tick, created),
        lives_base,
        woven,
        resolved: None,
        omen_at: None,
        master_best,
    });
    w.souls[debtor as usize].threads.push(tid);
    w.souls[creditor as usize].threads.push(tid);
    if kind == ThreadKind::Feud {
        let key = (debtor.min(creditor), debtor.max(creditor));
        let c = w.kill_pairs.entry(key).or_insert(0);
        *c = c.saturating_add(1);
    }
    tid
}

/// 開局宿緣（GDD 4.6 步驟 5）：以既有的 `Genesis` 事件為 `created`，不另寫 `ThreadCreated`；
/// 強度取 [`default_strength`]、`lives` = 0。呼叫端負責挑選雙方並避免重複。
/// 魂不存在或欠債方＝債主時回傳 `ThreadId::MAX`。
pub fn create_genesis(w: &mut World, kind: ThreadKind, debtor: SoulId, creditor: SoulId, genesis_event: EventId) -> ThreadId {
    if debtor == creditor || debtor as usize >= w.souls.len() || creditor as usize >= w.souls.len() {
        return ThreadId::MAX;
    }
    push_thread(w, kind, debtor, creditor, default_strength(kind), false, genesis_event)
}

/// 一律新建一條線（不合併），寫入 `ThreadCreated` 事件，並推進雙方魂的 `threads`。
fn insert_thread(
    w: &mut World,
    kind: ThreadKind,
    debtor: SoulId,
    creditor: SoulId,
    strength: f32,
    woven: bool,
    cause: Option<EventId>,
) -> ThreadId {
    let eid = w.ledger.events.len() as EventId;
    let tid = push_thread(w, kind, debtor, creditor, strength, woven, eid);
    let importance = if woven {
        5
    } else if kind == ThreadKind::Mentor {
        1
    } else {
        2
    };
    let (province, cell) = place(w, debtor);
    let mut e = NewEvent::new(EventKind::ThreadCreated, importance, EventArgs::Thread { thread: tid, kind, lives: 0 });
    e.province = province;
    e.cell = cell;
    e.souls = smallvec![debtor, creditor];
    e.persons = [debtor, creditor].iter().filter_map(|&s| any_body(w, s).map(|p| p.id)).collect();
    if let Some(c) = cause {
        e.causes = smallvec![c];
    }
    let got = ledger::record(w, e);
    debug_assert_eq!(got, eid);
    tid
}

/// 建立宿緣（3.3）。同一對魂、同一種類已有未了線時，強度累加（上限 100）並回傳既有的 ThreadId，不寫事件。
/// 情緣線不分方向配對，其他種類以 (欠債方, 債主) 為鍵（師徒的方向因此有別）。
/// 否則新建並寫入 `ThreadCreated`；`strength` 通常取 [`default_strength`]，司命牽線取 `THREAD_INIT_TIANJI` 並 `woven = true`。
/// 殺業的新建請走 [`on_kill`]（含復仇與冤冤相報）。魂不存在、欠債方＝債主時回傳 `ThreadId::MAX`。
pub fn create(
    w: &mut World,
    kind: ThreadKind,
    debtor: SoulId,
    creditor: SoulId,
    strength: f32,
    woven: bool,
    cause: Option<EventId>,
) -> ThreadId {
    if debtor == creditor || debtor as usize >= w.souls.len() || creditor as usize >= w.souls.len() {
        return ThreadId::MAX;
    }
    let directed = kind != ThreadKind::Love;
    if let Some(tid) = open_between(w, kind, debtor, creditor, directed) {
        let t = &mut w.threads[tid as usize];
        t.strength = (t.strength + strength).min(THREAD_STRENGTH_CAP);
        t.woven |= woven;
        return tid;
    }
    // 大劫之後一段時間，新的宿緣線不易長出（司命牽線不受影響）。
    if !woven && !crate::cataclysm::thread_allowed(w) {
        return ThreadId::MAX;
    }
    insert_thread(w, kind, debtor, creditor, strength, woven, cause)
}

// ── 了結 ─────────────────────────────────────────────────

/// 了結（3.3 了結）：清算業力 → 寫入 `ThreadResolved`（因：建立事件、雙方最近事件、`cause`）→ 天書偵測。
/// 司命天機（M5）與地圖奇觀（呈現層）不在此處。線已了結或已消散時回傳 `None`。
pub fn resolve(w: &mut World, tid: ThreadId, how: Resolution, cause: Option<EventId>) -> Option<EventId> {
    let why = match how {
        Resolution::Revenge => Why::Avenged,
        Resolution::Atone => Why::Repaid,
        Resolution::Other => Why::ResolvedOther,
    };
    resolve_why(w, tid, how, cause, why)
}

/// 同 [`resolve`]，並指定了結的成因（寫入 `ThreadResolved` 的 `why`）。
pub fn resolve_why(w: &mut World, tid: ThreadId, how: Resolution, cause: Option<EventId>, why: Why) -> Option<EventId> {
    let t = w.threads.get(tid as usize)?.clone();
    if !is_open(&t) {
        return None;
    }
    match how {
        Resolution::Revenge => {
            let s = &mut w.souls[t.debtor as usize];
            s.sin = (s.sin - t.strength * RESOLVE_REVENGE_S_COEF).max(0.0);
        }
        Resolution::Atone => {
            let s = &mut w.souls[t.debtor as usize];
            s.sin = (s.sin - t.strength * RESOLVE_ATONE_S_COEF).max(0.0);
        }
        Resolution::Other => {
            for sid in [t.debtor, t.creditor] {
                w.souls[sid as usize].merit += t.strength * RESOLVE_OTHER_G_COEF;
            }
        }
    }
    let n = lives(w, &t);
    let (province, cell) = place(w, t.creditor);
    let mut e =
        NewEvent::new(EventKind::ThreadResolved, if n >= 2 { 5 } else { 4 }, EventArgs::Thread { thread: tid, kind: t.kind, lives: n });
    e.why = why;
    e.province = province;
    e.cell = cell;
    e.souls = smallvec![t.debtor, t.creditor];
    e.persons = [t.debtor, t.creditor].iter().filter_map(|&s| any_body(w, s).map(|p| p.id)).collect();
    let mut causes: SmallVec<[EventId; 3]> = smallvec![t.created.1];
    for s in [t.debtor, t.creditor] {
        causes.extend(last_event_of(w, s));
    }
    causes.extend(cause);
    e.causes = causes;
    let eid = ledger::record(w, e);
    w.threads[tid as usize].resolved = Some((w.tick, eid));
    Some(eid)
}

/// 強度降到 0：線消散（不算了結），寫入 `ThreadFaded`。
pub(crate) fn fade(w: &mut World, tid: ThreadId) {
    let t = w.threads[tid as usize].clone();
    w.threads[tid as usize].strength = 0.0;
    let n = lives(w, &t);
    let (province, cell) = place(w, t.debtor);
    let mut e = NewEvent::new(EventKind::ThreadFaded, 1, EventArgs::Thread { thread: tid, kind: t.kind, lives: n });
    e.province = province;
    e.cell = cell;
    e.souls = smallvec![t.debtor, t.creditor];
    e.causes = smallvec![t.created.1];
    ledger::record(w, e);
}

// ── 殺業 ─────────────────────────────────────────────────

/// 3.1 斬殺惡人：死者 S > G + 50。
fn is_evildoer(w: &World, sid: SoulId) -> bool {
    w.souls.get(sid as usize).is_some_and(|s| s.sin > s.merit + EVIL_THRESHOLD_OFFSET)
}

/// 3.1 殺有名之人的業力：死者是惡人 → 兇手 G += 5；否則兇手 S += 10 × 無辜係數。
/// 判斷看死者的魂。呼叫端（死亡處理）統一計業時呼叫；[`on_kill`] 本身不改 G／S。
pub fn kill_karma(w: &mut World, killer: PersonId, victim: PersonId) {
    let (Some(k), Some(v)) = (w.persons.get(killer as usize), w.persons.get(victim as usize)) else { return };
    let (ks, vs) = (k.soul, v.soul);
    if is_evildoer(w, vs) {
        w.souls[ks as usize].merit += MERIT_SLAY_EVIL;
    } else {
        let coef = if w.souls[vs as usize].net_karma() >= 0.0 { SIN_KILL_INNOCENT_COEF } else { SIN_KILL_GUILTY_COEF };
        w.souls[ks as usize].sin += SIN_KILL_BASE * coef;
    }
}

/// 有名之人 `killer` 殺死 `victim` 後呼叫（`victim.died` 與死亡事件已寫入，其 `last_event` 為因）。
///
/// 1. 兇手是死者所欠的債主（死者為某條未了殺業／怨線的欠債方）→ 復仇：該線了結，**不**建立新線。
/// 2. 兇手是某條未了殺業／怨線的欠債方、死者為債主 → 冤冤相報：舊線強度 +30、寫入 `FeudEscalated`，
///    另建一條獨立的新線（同種類、同方向）。
/// 3. 死者是惡人（S > G + 50）→ 不建線。
/// 4. 否則建立殺業線：兇手欠死者，初始強度 40 + 死者修士境界 × 5（君主 20）。
///
/// 回傳新建的線；復仇、不建線時回傳 `None`。同魂、天道不處理。
pub fn on_kill(w: &mut World, killer: PersonId, victim: PersonId) -> Option<ThreadId> {
    let (k, v) = (w.persons.get(killer as usize)?, w.persons.get(victim as usize)?);
    let (ks, vs) = (k.soul, v.soul);
    if ks == vs || ks == HEAVEN_SOUL || vs == HEAVEN_SOUL {
        return None;
    }
    let cause = v.last_event;
    let victim_province = v.province;
    let ids: Vec<ThreadId> = w.souls[vs as usize].threads.clone();

    // 1. 復仇。
    let mut revenged = false;
    for &tid in &ids {
        let t = &w.threads[tid as usize];
        if matches!(t.kind, ThreadKind::Feud | ThreadKind::Grudge) && is_open(t) && t.debtor == vs && t.creditor == ks {
            resolve(w, tid, Resolution::Revenge, cause);
            revenged = true;
        }
    }
    if revenged {
        return None;
    }

    // 2. 冤冤相報（僅處理第一條）。
    for &tid in &ids {
        let t = &w.threads[tid as usize];
        if matches!(t.kind, ThreadKind::Feud | ThreadKind::Grudge) && is_open(t) && t.debtor == ks && t.creditor == vs {
            let (kind, created) = (t.kind, t.created.1);
            let snapshot = t.clone();
            let n = lives(w, &snapshot);
            let t = &mut w.threads[tid as usize];
            t.strength = (t.strength + FEUD_ESCALATE_OLD_BONUS).min(THREAD_STRENGTH_CAP);
            let mut e = NewEvent::new(EventKind::FeudEscalated, 4, EventArgs::Thread { thread: tid, kind, lives: n });
            e.province = Some(victim_province);
            e.souls = smallvec![ks, vs];
            e.persons = smallvec![killer, victim];
            e.causes = match cause {
                Some(c) => smallvec![created, c],
                None => smallvec![created],
            };
            let esc = ledger::record(w, e);
            let strength = victim_strength(w, victim);
            return Some(insert_thread(w, kind, ks, vs, strength, false, Some(esc)));
        }
    }

    // 3. 斬殺惡人。
    if is_evildoer(w, vs) {
        return None;
    }
    // 4. 新建。
    let strength = victim_strength(w, victim);
    Some(insert_thread(w, ThreadKind::Feud, ks, vs, strength, false, cause))
}

/// 殺業初始強度：40 + 死者身分加成（修士：境界 × 5；君主：20）。
fn victim_strength(w: &World, victim: PersonId) -> f32 {
    let p = &w.persons[victim as usize];
    let bonus = match (&p.path, p.role) {
        (PathState::Cultivator { realm, .. }, _) => *realm as f32 * THREAD_INIT_KILL_REALM_MULT,
        (_, Role::Ruler) => THREAD_INIT_KILL_MONARCH_BONUS,
        _ => 0.0,
    };
    THREAD_INIT_KILL + bonus
}

/// 化解（3.3 殺業化解）：債主 `endangered` 遇險（會戰、天劫、妖襲、被襲殺）時由場景呼叫。
/// 該魂為債主的每條未了殺業線，欠債方若在世、非幼年且與債主同州，以 `0.3 × 德行 / 10` 機率出手相救，
/// 線了結（Atone，欠債方 S −= 強度）。只救一次（取 ThreadId 最小者）。
pub fn on_peril(w: &mut World, endangered: PersonId, cause: Option<EventId>) -> Option<EventId> {
    let p = w.persons.get(endangered as usize)?;
    let (sid, prov) = (p.soul, p.province);
    let ids = w.souls[sid as usize].threads.clone();
    for tid in ids {
        let t = &w.threads[tid as usize];
        if t.kind != ThreadKind::Feud || !is_open(t) || t.creditor != sid || t.debtor == HEAVEN_SOUL {
            continue;
        }
        let Some(d) = body(w, t.debtor).filter(|d| d.province == prov) else { continue };
        let chance = ATONE_RESCUE_COEF * d.attrs.virtue / ATONE_RESCUE_VIRTUE_DIV;
        if roll(w, chance) {
            return resolve(w, tid, Resolution::Atone, cause);
        }
    }
    None
}

// ── 情緣、師徒 ───────────────────────────────────────────

/// 婚配成立（3.4）：若兩魂已有 `lives ≥ 1` 的未了情緣線，該線了結（再續前緣）並回傳 `None`；
/// 否則建立情緣線或累加既有未了線，回傳該線。`cause` 通常是 `Married` 事件。
/// 呼叫端須已設定雙方的 `spouse`。
pub fn on_marriage(w: &mut World, a: PersonId, b: PersonId, cause: Option<EventId>) -> Option<ThreadId> {
    let (sa, sb) = (w.persons.get(a as usize)?.soul, w.persons.get(b as usize)?.soul);
    if sa == sb || sa == HEAVEN_SOUL || sb == HEAVEN_SOUL {
        return None;
    }
    if let Some(tid) = open_between(w, ThreadKind::Love, sa, sb, false) {
        let t = w.threads[tid as usize].clone();
        if lives(w, &t) >= 1 {
            resolve_why(w, tid, Resolution::Other, cause, Why::WedSettled);
            return None;
        }
    }
    Some(create(w, ThreadKind::Love, sa, sb, THREAD_INIT_LOVE, false, cause))
}

/// 拜師成立（3.3 師徒；`Person::master`／`disciples` 已由呼叫端設定）。徒弟魂 `disciple` 是欠債方，師父魂 `master` 是債主。
/// 同向已有 `lives ≥ 1` 的未了師徒線 → 該線了結（再結師徒）；之後建立新線（或累加同世既有線）。
/// 方向相反的舊線不受影響（可並存；已有則觸發天書「前世今生」）。
pub fn on_apprentice(w: &mut World, disciple: SoulId, master: SoulId, cause: Option<EventId>) -> ThreadId {
    if let Some(tid) = open_between(w, ThreadKind::Mentor, disciple, master, true) {
        let t = w.threads[tid as usize].clone();
        if lives(w, &t) >= 1 {
            resolve(w, tid, Resolution::Other, cause);
        }
    }
    create(w, ThreadKind::Mentor, disciple, master, THREAD_INIT_MASTER, false, cause)
}

// ── 衝突殺人 ─────────────────────────────────────────────

/// 相遇衝突的殺人：`killer` 殺死 `victim`（皆在世）。寫入 `Killed`，`people::die` 處理死亡，計業，再 [`on_kill`]。
/// 回傳 [`on_kill`] 的結果。`cause` 為衝突的起因事件。
fn kill_person(w: &mut World, killer: PersonId, victim: PersonId, cause: Option<EventId>) -> Option<ThreadId> {
    let (ks, vs) = (w.persons[killer as usize].soul, w.persons[victim as usize].soul);
    let (province, cell) = {
        let v = &w.persons[victim as usize];
        (v.province, v.cell)
    };
    // 殺人事件為死亡事件的因；死亡處理（Died、魂入中陰、人際、勢力）由 people::die 負責。
    let mut ke = NewEvent::new(EventKind::Killed, 3, EventArgs::None);
    let rel = ledger::rel_of_persons(w, killer, victim);
    ke.why = if rel.has(Rel::THREAD_FEUD | Rel::THREAD_GRUDGE) { Why::Vendetta } else { Why::KillOther };
    ke.rel = Some(rel);
    ke.province = Some(province);
    ke.cell = Some(cell);
    ke.souls = smallvec![ks, vs];
    ke.persons = smallvec![killer, victim];
    ke.causes = cause.into_iter().collect();
    let killed = ledger::record(w, ke);
    crate::people::die(w, victim, DeathCause::Killed, None, &[killed]);
    kill_karma(w, killer, victim);
    on_kill(w, killer, victim)
}
