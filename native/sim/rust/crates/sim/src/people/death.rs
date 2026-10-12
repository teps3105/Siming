//! GDD 3.4 壽元、6.3「死亡」處理。

use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::sim::World;
use crate::soul;

/// 7.2 `Died` 重要度：君主 4；修士依境界——煉氣至金丹 2、元嬰至煉虛 3、合體以上 4；其他 2。
const DIED_IMPORTANCE_DEFAULT: u8 = 2;
const DIED_IMPORTANCE_RULER: u8 = 4;
const DIED_MID_REALM_MIN: u8 = 3;
const DIED_HIGH_REALM_MIN: u8 = 6;
const DIED_IMPORTANCE_MID: u8 = 3;
const DIED_IMPORTANCE_HIGH: u8 = 4;
/// 7.2 `Ascended` 重要度。
const ASCENDED_IMPORTANCE: u8 = 5;

fn died_importance(p: &Person) -> u8 {
    if p.role == Role::Ruler {
        return DIED_IMPORTANCE_RULER;
    }
    match p.path {
        PathState::Cultivator { realm, .. } if realm >= DIED_HIGH_REALM_MIN => DIED_IMPORTANCE_HIGH,
        PathState::Cultivator { realm, .. } if realm >= DIED_MID_REALM_MIN => DIED_IMPORTANCE_MID,
        _ => DIED_IMPORTANCE_DEFAULT,
    }
}

/// GDD 6.3「死亡」：人物在任何步驟死亡時立即呼叫。已死或不存在者回傳 `None`。
///
/// 依序：
/// 1. 設定 `died`，寫入死亡事件（飛升用 `Ascended`，其餘 `Died`；因 = 本人最近事件 + `extra_causes`）。
/// 2. `soul::on_death`：`liberation` 有值 → 超脫，否則入中陰。
/// 3. 勢力：自 `members` 移除。**首領死亡的繼承不在這裡處理**（U6 `dao`）：`leader` 欄位保持原值，
///    由繼承邏輯偵測 `leader` 已死者並換人。
/// 4. 人際：配偶的 `spouse`、徒弟的 `master`、師父的 `disciples` 中指向死者的部分解除（宿緣保留）。
///    死者自己的 `spouse`／`master`／`disciples`／`children` 保留作為生平記錄。
/// 5. 殺業計算與殺業線（3.1、3.3）由呼叫端（衝突、襲殺的處理）負責，本函式不計業。
/// 6. 會戰統帥（M3）。
pub fn die(w: &mut World, pid: PersonId, cause: DeathCause, liberation: Option<Liberation>, extra_causes: &[EventId]) -> Option<EventId> {
    let tick = w.tick;
    let p = w.persons.get_mut(pid as usize)?;
    if !p.alive() {
        return None;
    }
    p.died = Some((tick, cause));
    if let Ok(k) = w.alive_ids.binary_search(&pid) {
        w.alive_ids.remove(k);
    }
    let p = &mut w.persons[pid as usize];
    p.travel_until = None;
    p.blessed_until = None;
    p.forced_breakthrough = None;

    let p = &w.persons[pid as usize];
    let ascended = liberation == Some(Liberation::Ascension);
    let (kind, importance) = if ascended { (EventKind::Ascended, ASCENDED_IMPORTANCE) } else { (EventKind::Died, died_importance(p)) };
    let mut e = NewEvent::new(kind, importance, EventArgs::Death { cause, age: p.age(tick).min(u16::MAX as u32) as u16 });
    e.province = Some(p.province);
    e.cell = Some(p.cell);
    e.souls = smallvec![p.soul];
    e.persons = smallvec![pid];
    if let Some(pw) = p.power {
        e.powers = smallvec![pw];
    }
    if let Some(c) = p.last_event {
        e.causes.push(c);
    }
    e.causes.extend_from_slice(extra_causes);
    let eid = ledger::record(w, e);

    soul::on_death(w, pid, liberation);

    let (power, spouse, master, disciples) = {
        let p = &w.persons[pid as usize];
        (p.power, p.spouse, p.master, p.disciples.clone())
    };
    if let Some(pw) = power.and_then(|id| w.powers.get_mut(id as usize)) {
        pw.members.retain(|&m| m != pid);
    }
    if let Some(s) = spouse.and_then(|id| w.persons.get_mut(id as usize)) {
        if s.spouse == Some(pid) {
            s.spouse = None;
        }
    }
    if let Some(m) = master.and_then(|id| w.persons.get_mut(id as usize)) {
        m.disciples.retain(|&d| d != pid);
    }
    for d in disciples {
        if let Some(d) = w.persons.get_mut(d as usize) {
            if d.master == Some(pid) {
                d.master = None;
            }
        }
    }
    Some(eid)
}

/// GDD 6.3 步驟 10：壽元已盡者（年齡 ≥ 壽元）死亡，死因為壽終。依人物 ID 升冪。
pub fn yearly_aging(w: &mut World) {
    let tick = w.tick;
    for i in w.alive_snapshot().into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if p.alive() && p.age(tick) >= p.lifespan as u32 {
            die(w, p.id, DeathCause::Age, None, &[]);
        }
    }
}
