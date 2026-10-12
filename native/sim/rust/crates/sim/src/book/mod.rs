//! GDD 4.5 天書偵測（U7）。
//!
//! 偵測框架：[`crate::ledger::record`] 每寫入一筆事件就呼叫 [`detect`]（6.3「事件寫入」第 2 步）。
//! `detect` 依事件種類與參數判斷是否達成條目；達成時 [`grant`] 更新世界天書（次數 +1，首次記下 tick 與事件）
//! 並寫入 `BookEntry` 事件（importance 4，以觸發事件為因）。`BookEntry` 事件自身不再偵測。
//! 「首次達成給天機」在 M2 為空操作（天機在 M5）。
//!
//! # M2 的五條
//!
//! | 條目 | 觸發事件 | 條件 |
//! |---|---|---|
//! | 三世血仇 | `ThreadResolved`（殺業） | `lives ≥ 2` |
//! | 冤冤相報 | `ThreadCreated`（殺業） | 該對魂（不分方向）的 `kill_pairs` 恰好累計到 3；每對魂只觸發一次 |
//! | 前世今生 | `ThreadCreated`（師徒） | 同兩魂間已有方向相反的師徒線（不論是否了結） |
//! | 再續前緣 | `ThreadResolved`（情緣） | `lives ≥ 2` |
//! | 轉世大能 | `Breakthrough`（渡過天劫的突破同樣寫入 `Breakthrough`，故不另在 `Tribulation` 偵測） | 魂的 `best_prev.realm ≥ 金丹`，今生未滿 50 歲，新境界恰等於 `best_prev.realm` |
//!
//! 新增條目時：在 [`detect`] 加一個分支，並在此表補一列。

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use smallvec::smallvec;

/// 世界天書是否已有此條目。
pub fn achieved(w: &World, entry: BookEntryKind) -> bool {
    w.book.entries.contains_key(&entry)
}

/// 達成條目：更新世界天書並寫入 `BookEntry` 事件。回傳 `BookEntry` 事件的 ID。
pub fn grant(w: &mut World, entry: BookEntryKind, trigger: EventId) -> EventId {
    let tick = w.tick;
    let (souls, persons, province, cell) = {
        let e = &w.ledger.events[trigger as usize];
        (e.souls.clone(), e.persons.clone(), e.province, e.cell)
    };
    let mut ne = NewEvent::new(EventKind::BookEntry, 4, EventArgs::Book { entry });
    ne.causes = smallvec![trigger];
    ne.souls = souls;
    ne.province = province;
    ne.cell = cell;
    let _ = persons; // 不併入人物：避免改寫人物的最近事件。
    let id = ledger::record(w, ne);
    w.book.entries.entry(entry).and_modify(|r| r.count += 1).or_insert(BookRecord { count: 1, first_tick: tick, first_event: id });
    // 首次達成給天機（TIANJI_BOOK）：天機在 M5，M2 為空操作。
    id
}

/// 偵測剛寫入的事件 `id` 是否達成天書條目。
pub fn detect(w: &mut World, id: EventId) {
    let Some(e) = w.ledger.events.get(id as usize) else { return };
    let souls = e.souls.clone();
    let persons = e.persons.clone();
    let ek = e.kind;
    match e.args.clone() {
        EventArgs::Thread { thread, kind, lives } => match ek {
            EventKind::ThreadResolved => match kind {
                ThreadKind::Feud if lives >= BOOK_THREE_LIVES_MIN => grant(w, BookEntryKind::ThreeLivesFeud, id),
                ThreadKind::Love if lives >= BOOK_REUNITED_MIN_LIVES => grant(w, BookEntryKind::ReunitedLove, id),
                _ => return,
            },
            EventKind::ThreadCreated => match kind {
                ThreadKind::Feud => {
                    let Some(t) = w.threads.get(thread as usize) else { return };
                    let key = (t.debtor.min(t.creditor), t.debtor.max(t.creditor));
                    if u32::from(w.kill_pairs.get(&key).copied().unwrap_or(0)) != FEUD_CHAIN_COUNT {
                        return;
                    }
                    grant(w, BookEntryKind::EndlessVengeance, id)
                }
                ThreadKind::Mentor => {
                    let Some(t) = w.threads.get(thread as usize) else { return };
                    let reverse = w.souls.get(t.debtor as usize).is_some_and(|s| {
                        s.threads.iter().any(|&o| {
                            o != thread
                                && w.threads
                                    .get(o as usize)
                                    .is_some_and(|x| x.kind == ThreadKind::Mentor && x.debtor == t.creditor && x.creditor == t.debtor)
                        })
                    });
                    if !reverse {
                        return;
                    }
                    grant(w, BookEntryKind::PastAndPresent, id)
                }
                _ => return,
            },
            _ => return,
        },
        EventArgs::Realm { realm } if ek == EventKind::Breakthrough => reincarnated_master(w, id, &souls, &persons, realm),
        // 渡過天劫時 `Tribulation{Passed}` 與緊接著的 `Breakthrough` 帶同一境界；只在 `Breakthrough` 偵測，避免重複計次。
        _ => return,
    };
}

/// 轉世大能：前世最高境界 ≥ 金丹，今生在 50 歲前再達到該境界。
fn reincarnated_master(w: &mut World, id: EventId, souls: &[SoulId], persons: &[PersonId], realm: u8) -> EventId {
    let (Some(&sid), Some(&pid)) = (souls.first(), persons.first()) else { return id };
    let (Some(soul), Some(p)) = (w.souls.get(sid as usize), w.persons.get(pid as usize)) else { return id };
    let Some(prev) = soul.best_prev.realm else { return id };
    if prev >= BOOK_REINCARNATED_MIN_REALM && realm == prev && p.age(w.tick) < BOOK_REINCARNATED_MAX_AGE {
        grant(w, BookEntryKind::ReincarnatedMaster, id)
    } else {
        id
    }
}
