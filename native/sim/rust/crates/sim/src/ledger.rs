//! GDD 3.3 因果帳：事件寫入、反向索引、修剪、輸出游標。（U7）
//!
//! - [`record`]：指派 ID、寫入、維護反向索引（`effects`）、更新人物最近事件，並呼叫天書偵測（`book::detect`）。
//! - [`prune`]：每年修剪 importance 1 且超過 100 年的事件。為保持 `EventId` 即索引，被修剪的事件
//!   留下**墓碑**（`importance == 0`，保留 `tick`、`kind`、`causes`，清空其餘），指向它的 `causes`
//!   改指向它的因（遞迴到最近被保留的祖先）。讀取事件請用 [`get`]（墓碑回傳 `None`）。
//! - [`take_new`]：自上次呼叫以來新增的事件 ID。

use smallvec::SmallVec;

use crate::model::*;
use crate::params::LEDGER_TRIVIAL_KEEP_YEARS;
use crate::sim::World;
use crate::types::ProvinceId;

/// 新事件的建構參數；`id`、`tick` 由 [`record`] 補上。
pub struct NewEvent {
    pub kind: EventKind,
    pub province: Option<ProvinceId>,
    pub cell: Option<u32>,
    pub souls: SmallVec<[SoulId; 2]>,
    pub persons: SmallVec<[PersonId; 2]>,
    pub powers: SmallVec<[PowerId; 2]>,
    pub causes: SmallVec<[EventId; 3]>,
    pub importance: u8,
    pub args: EventArgs,
    pub why: Why,
    /// 兩方關係快照；留 `None` 時，事件有兩個人物（或兩個魂）就在寫入時自動查。
    pub rel: Option<Rel>,
}

impl NewEvent {
    pub fn new(kind: EventKind, importance: u8, args: EventArgs) -> Self {
        Self {
            kind,
            province: None,
            cell: None,
            souls: SmallVec::new(),
            persons: SmallVec::new(),
            powers: SmallVec::new(),
            causes: SmallVec::new(),
            importance,
            args,
            why: Why::None,
            rel: None,
        }
    }
}

/// 兩個人物之間的關係旗標。師徒、同宗門、配偶、親子、同鄉看人物資料，宿緣種類與世數看兩魂之間未了的線。
pub fn rel_of_persons(w: &World, a: PersonId, b: PersonId) -> Rel {
    let (Some(pa), Some(pb)) = (w.persons.get(a as usize), w.persons.get(b as usize)) else { return Rel(0) };
    let mut r = 0u16;
    if pa.master == Some(b) || pb.master == Some(a) {
        r |= Rel::MASTER_DISCIPLE;
    }
    if pa.power.is_some() && pa.power == pb.power {
        r |= Rel::SAME_SECT;
    }
    if pa.spouse == Some(b) || pb.spouse == Some(a) {
        r |= Rel::SPOUSES;
    }
    if pa.children.contains(&b) || pb.children.contains(&a) {
        r |= Rel::PARENT_CHILD;
    }
    if pa.home == pb.home {
        r |= Rel::SAME_HOMETOWN;
    }
    Rel(r | rel_of_souls(w, pa.soul, pb.soul).0)
}

/// 兩個魂之間未了宿緣線的種類與世數。
pub fn rel_of_souls(w: &World, a: SoulId, b: SoulId) -> Rel {
    let Some(s) = w.souls.get(a as usize) else { return Rel(0) };
    let (mut kinds, mut lives) = (0u16, 0u16);
    for &tid in &s.threads {
        let Some(t) = w.threads.get(tid as usize) else { continue };
        if !((t.debtor == a && t.creditor == b) || (t.debtor == b && t.creditor == a)) || !crate::thread::is_open(t) {
            continue;
        }
        kinds |= match t.kind {
            ThreadKind::Feud => Rel::THREAD_FEUD,
            ThreadKind::Grace => Rel::THREAD_GRACE,
            ThreadKind::Love => Rel::THREAD_LOVE,
            ThreadKind::Mentor => Rel::THREAD_MENTOR,
            ThreadKind::Grudge => Rel::THREAD_GRUDGE,
        };
        lives = lives.max(crate::thread::lives(w, t).min(7));
    }
    Rel(kinds | lives << Rel::LIVES_SHIFT)
}

/// GDD 6.3「事件寫入」：指派 ID、寫入因果帳、維護反向索引、更新人物的最近事件，然後偵測天書條目
/// （`book::detect`；新條目會遞迴寫入 `BookEntry` 事件）。
pub fn record(w: &mut World, e: NewEvent) -> EventId {
    let id = w.ledger.events.len() as EventId;
    let mut causes = e.causes;
    causes.sort_unstable();
    causes.dedup();
    for &c in &causes {
        if let Some(list) = w.ledger.effects.get_mut(c as usize) {
            list.push(id);
        }
    }
    for &p in &e.persons {
        if let Some(person) = w.persons.get_mut(p as usize) {
            person.last_event = Some(id);
        }
    }
    let kind = e.kind;
    let rel = e.rel.unwrap_or_else(|| match (e.persons.as_slice(), e.souls.as_slice()) {
        ([a, b], _) => rel_of_persons(w, *a, *b),
        (_, [a, b]) => rel_of_souls(w, *a, *b),
        _ => Rel(0),
    });
    w.ledger.events.push(Event {
        id,
        tick: w.tick,
        kind,
        province: e.province,
        cell: e.cell,
        souls: e.souls,
        persons: e.persons,
        powers: e.powers,
        causes,
        importance: e.importance,
        args: e.args,
        why: e.why,
        rel,
    });
    w.ledger.effects.push(SmallVec::new());
    if kind != EventKind::BookEntry {
        crate::book::detect(w, id);
    }
    id
}

/// 讀取事件；已被修剪（墓碑）或不存在時回傳 `None`。
pub fn get(w: &World, id: EventId) -> Option<&Event> {
    w.ledger.events.get(id as usize).filter(|e| e.importance > 0)
}

/// 從事件 `id` 起，沿第一個因往上找到最近仍保留的事件（自己保留則回傳自己）。
pub fn live_ancestor(w: &World, mut id: EventId) -> Option<EventId> {
    loop {
        let e = w.ledger.events.get(id as usize)?;
        if e.importance > 0 {
            return Some(id);
        }
        id = *e.causes.first()?;
    }
}

/// GDD 2.3 饑荒事件（M2 起寫入）。
pub fn record_famines(w: &mut World, famines: &[(ProvinceId, u32)]) {
    for &(p, deaths) in famines {
        let mut e = NewEvent::new(EventKind::Famine, 3, EventArgs::Famine { deaths });
        e.province = Some(p);
        record(w, e);
    }
}

/// 自上次呼叫以來新增事件的 ID（由 Sim 收集，GDD 6.3 事件輸出）。
pub fn take_new(w: &mut World) -> Vec<EventId> {
    let from = w.ledger.emit_cursor;
    let to = w.ledger.events.len() as EventId;
    w.ledger.emit_cursor = to;
    (from..to).collect()
}

/// 修剪一個事件：把指向它的事件改指向它的因，再留下墓碑。
fn prune_one(w: &mut World, p: EventId) {
    let pi = p as usize;
    let causes = w.ledger.events[pi].causes.clone();
    let effects = std::mem::take(&mut w.ledger.effects[pi]);
    for &x in &effects {
        let ev = &mut w.ledger.events[x as usize];
        ev.causes.retain(|c| *c != p);
        ev.causes.extend(causes.iter().copied());
        ev.causes.sort_unstable();
        ev.causes.dedup();
        for &a in &causes {
            let list = &mut w.ledger.effects[a as usize];
            if let Err(pos) = list.binary_search(&x) {
                list.insert(pos, x);
            }
        }
    }
    for &a in &causes {
        w.ledger.effects[a as usize].retain(|e| *e != p);
    }
    let ev = &mut w.ledger.events[pi];
    ev.importance = 0;
    ev.province = None;
    ev.cell = None;
    ev.souls.clear();
    ev.persons.clear();
    ev.powers.clear();
    ev.args = EventArgs::None;
}

/// 這筆重要度 1 的舊事件現在可不可以修剪：在世者的歷程（降生、入道等）與未了線的建線事件要留著。
fn prunable(w: &World, e: &Event) -> bool {
    if e.persons.iter().any(|&p| w.persons.get(p as usize).is_some_and(|p| p.alive())) {
        return false;
    }
    if let EventArgs::Thread { thread, .. } = e.args {
        if w.threads.get(thread as usize).is_some_and(crate::thread::is_open) {
            return false;
        }
    }
    true
}

/// GDD 3.3 修剪 importance 1 且超過 100 年的事件（6.3 步驟 25）。
/// 事件依 ID 升冪即依 tick 升冪，因此只需從修剪游標掃到「剛好滿 100 年」的位置。
/// 在世者從降生起的歷程與未了線的建線事件不修剪，改記入 `prune_deferred`，每十年複查一次，條件解除才修剪。
/// 修剪後，`Person::last_event` 若指向被修剪的事件，改指向最近保留的祖先。
pub fn prune(w: &mut World) {
    let keep = LEDGER_TRIVIAL_KEEP_YEARS * crate::calendar::DAYS_PER_YEAR;
    let now = w.tick;
    let end = w.ledger.events.len() as EventId;
    let mut i = w.ledger.prune_cursor;
    let mut any = false;
    while i < end {
        let e = &w.ledger.events[i as usize];
        if e.tick + keep >= now {
            break;
        }
        if e.importance == 1 {
            if prunable(w, e) {
                prune_one(w, i);
                any = true;
            } else {
                w.ledger.prune_deferred.push(i);
            }
        }
        i += 1;
    }
    w.ledger.prune_cursor = i;
    if now.is_multiple_of(10 * crate::calendar::DAYS_PER_YEAR) && !w.ledger.prune_deferred.is_empty() {
        let list = std::mem::take(&mut w.ledger.prune_deferred);
        let mut kept = Vec::with_capacity(list.len());
        for id in list {
            if w.ledger.events[id as usize].importance == 1 && !prunable(w, &w.ledger.events[id as usize]) {
                kept.push(id);
            } else if w.ledger.events[id as usize].importance == 1 {
                prune_one(w, id);
                any = true;
            }
        }
        w.ledger.prune_deferred = kept;
    }
    if any {
        for k in 0..w.persons.len() {
            if let Some(le) = w.persons[k].last_event {
                if w.ledger.events.get(le as usize).is_some_and(|e| e.importance == 0) {
                    w.persons[k].last_event = live_ancestor(w, le);
                }
            }
        }
    }
}
