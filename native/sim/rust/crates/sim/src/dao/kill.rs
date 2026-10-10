//! 一次殺人的完整處理（GDD 6.3「死亡」第 5 步、3.1、3.3）。

use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::people;
use crate::sim::World;
use crate::thread;

/// 7.2 `Killed` 重要度 3。
const KILLED_IMPORTANCE: u8 = 3;

/// `killer` 殺死 `victim`。兩者須在世且不同人；否則回傳 `false` 且不做事。
///
/// 順序與 `thread` 的衝突殺人一致：
/// 1. 寫入 `Killed` 事件（因：兇手最近事件、`cause_event`）。
/// 2. `people::die(victim, cause)`，死因事件為 `Killed`。
/// 3. `thread::kill_karma`：依 3.1 計業（死者是惡人 → 兇手 G += 5，否則 S += 10 × 無辜係數）。
/// 4. `thread::on_kill`：殺業線（復仇了結、冤冤相報、新線；斬殺惡人不建線）。
pub fn kill(w: &mut World, killer: PersonId, victim: PersonId, cause: DeathCause, cause_event: Option<EventId>) -> bool {
    let (Some(k), Some(v)) = (w.persons.get(killer as usize), w.persons.get(victim as usize)) else { return false };
    if !k.alive() || !v.alive() || killer == victim {
        return false;
    }
    let mut e = NewEvent::new(EventKind::Killed, KILLED_IMPORTANCE, EventArgs::None);
    e.province = Some(v.province);
    e.cell = Some(v.cell);
    e.souls = smallvec![k.soul, v.soul];
    e.persons = smallvec![killer, victim];
    if let Some(c) = k.last_event {
        e.causes.push(c);
    }
    if let Some(c) = cause_event {
        e.causes.push(c);
    }
    let ke = ledger::record(w, e);
    people::die(w, victim, cause, None, &[ke]);
    thread::kill_karma(w, killer, victim);
    thread::on_kill(w, killer, victim);
    true
}
