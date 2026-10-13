//! GDD 3.1 宿慧與道緣傳承、利息；GDD 3.2 出生；GDD 3.4 來源（新魂）。

use rand::Rng;
use smallvec::smallvec;

use super::pull::{faith4, normalize4};
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::ProvinceId;

fn merge_best(a: Achievements, b: Achievements) -> Achievements {
    Achievements { realm: a.realm.max(b.realm), fruit: a.fruit.max(b.fruit), immortality: a.immortality.max(b.immortality) }
}

/// GDD 3.1 前世成就帶來的宿慧加成（不含兵解）。
fn achievement_wisdom(a: &Achievements) -> f32 {
    a.realm.map_or(0.0, |r| r as f32 * WISDOM_GAIN_REALM)
        + a.fruit.map_or(0.0, |f| f as f32 * WISDOM_GAIN_FRUIT)
        + a.immortality / WISDOM_GAIN_IMMORTAL_DIV
}

/// 轉世處理（GDD 6.3「出生」第 2 項），不改變魂的狀態：
/// 1. 宿慧：`W_new = W_old × 0.5 + 前世成就`（兵解另 +30），截斷 0..200。
/// 2. `best_now` 併入 `best_prev`（逐項取大），再清空。
/// 3. 道緣：`normalize(0.7 × 前世道緣 + 0.3 × 出生州 faith4)`，前世入魔魔 +0.2、被度化釋 +0.3，再正規化。
/// 4. 利息：欠債方的未了殺業線、怨線，先 `S += 強度 × 0.1`（舊強度），再強度 ×1.1（上限 100）。
/// 5. 清除前世暫存旗標與修羅印記（修羅印記由六道判定重新設定）。
pub fn prepare_rebirth(w: &mut World, sid: SoulId, birth_province: ProvinceId) {
    let f4 = w.provinces.get(birth_province as usize).map_or([0.25; 4], faith4);
    let thread_ids = w.souls[sid as usize].threads.clone();

    let soul = &mut w.souls[sid as usize];
    let mut wisdom = soul.wisdom * WISDOM_DECAY + achievement_wisdom(&soul.best_now);
    if soul.marks.has(SoulMarks::LAST_LIFE_SHED) {
        wisdom += WISDOM_GAIN_SHED_BODY;
    }
    soul.wisdom = wisdom.clamp(0.0, 200.0);
    soul.best_prev = merge_best(soul.best_prev, soul.best_now);
    soul.best_now = Achievements::default();

    let mut aff = [0.0f32; 4];
    for (k, a) in aff.iter_mut().enumerate() {
        *a = AFFINITY_KEEP * soul.affinity[k] + (1.0 - AFFINITY_KEEP) * f4[k];
    }
    let mut aff = normalize4(aff);
    if soul.marks.has(SoulMarks::LAST_LIFE_DEMON) {
        aff[Teaching::Demon as usize] += AFFINITY_DEMON_GAIN;
    }
    if soul.marks.has(SoulMarks::LAST_LIFE_CONVERTED) {
        aff[Teaching::Buddha as usize] += AFFINITY_CONVERTED_BUDDHA_GAIN;
    }
    soul.affinity = normalize4(aff);
    soul.marks.clear(SoulMarks::LAST_LIFE_DEMON | SoulMarks::LAST_LIFE_SHED | SoulMarks::LAST_LIFE_CONVERTED);

    let mut sin_gain = 0.0f32;
    for tid in thread_ids {
        let Some(t) = w.threads.get_mut(tid as usize) else { continue };
        if t.debtor != sid || t.resolved.is_some() || !matches!(t.kind, ThreadKind::Feud | ThreadKind::Grudge) {
            continue;
        }
        sin_gain += t.strength * THREAD_INTEREST_SIN_COEF;
        t.strength = (t.strength * THREAD_INTEREST_MULT).min(THREAD_STRENGTH_CAP);
    }
    w.souls[sid as usize].sin += sin_gain;
}

/// 在州郡內隨機選一格（魂子流）；州郡沒有格時用治所。
fn pick_cell(w: &mut World, province: ProvinceId) -> u32 {
    let Some(p) = w.provinces.get(province as usize) else { return 0 };
    if p.cells.is_empty() {
        return p.seat;
    }
    let k = w.rng.soul.gen_range(0..p.cells.len());
    w.provinces[province as usize].cells[k]
}

/// 建立年齡 0 的有名之人：委派 `people::spawn`（GDD 3.4 屬性、壽元、姓名）。
fn spawn_person(w: &mut World, soul: SoulId, province: ProvinceId, cell: u32) -> PersonId {
    crate::people::spawn(w, soul, province, cell)
}

/// 把剛建立的肉身掛到魂上：`state = Incarnate`、`lives` 追加、`last_place` 更新。
///
/// U5 若自行建立有名之人（補足、王族生育），在取得魂（[`take_for_birth`](super::take_for_birth)
/// 或 [`create_soul`]）並呼叫 [`prepare_rebirth`] 之後，用它完成綁定。
pub fn attach_person(w: &mut World, sid: SoulId, pid: PersonId) {
    let (province, cell) = {
        let p = &w.persons[pid as usize];
        (p.province, p.cell)
    };
    let soul = &mut w.souls[sid as usize];
    soul.state = SoulState::Incarnate(pid);
    soul.lives.push(pid);
    soul.last_place = (province, cell);
}

/// 輪迴出生（GDD 3.2 出生）：轉世處理 → 建立肉身 → 綁定 → 寫入 `Rebirth` 事件。
/// 有未了宿緣者事件重要度 3，其他 1（GDD 7.2）。
pub fn rebirth(w: &mut World, sid: SoulId, province: ProvinceId) -> PersonId {
    let cause = w.souls[sid as usize].lives.last().and_then(|&p| w.persons.get(p as usize)).and_then(|p| p.last_event);
    prepare_rebirth(w, sid, province);
    let cell = pick_cell(w, province);
    let pid = spawn_person(w, sid, province, cell);
    attach_person(w, sid, pid);

    let has_thread =
        w.souls[sid as usize].threads.iter().any(|&t| w.threads.get(t as usize).is_some_and(|t| t.resolved.is_none() && t.strength > 0.0));
    let mut e = NewEvent::new(EventKind::Rebirth, if has_thread { 3 } else { 1 }, EventArgs::None);
    e.province = Some(province);
    e.cell = Some(cell);
    e.souls = smallvec![sid];
    e.persons = smallvec![pid];
    if let Some(c) = cause {
        e.causes = smallvec![c];
    }
    ledger::record(w, e);
    pid
}

/// 確保 `souls[0]` 是天道（GDD 3.1），已有魂時不做事。開天（U8）通常已建立；這裡只是防線。
/// 開天以外的呼叫端不需要直接呼叫；[`create_soul`] 會自動確保。
pub fn ensure_heaven(w: &mut World) {
    if w.souls.is_empty() {
        w.souls.push(Soul {
            id: HEAVEN_SOUL,
            merit: 0.0,
            sin: 0.0,
            wisdom: 0.0,
            affinity: [0.25; 4],
            threads: Vec::new(),
            lives: Vec::new(),
            best_prev: Achievements::default(),
            best_now: Achievements::default(),
            state: SoulState::Liberated { way: Liberation::Ascension, tick: 0 },
            origin: SoulOrigin::Primordial,
            marks: SoulMarks::default(),
            last_place: (0, 0),
            guide: None,
        });
    }
}

/// 建立新魂（GDD 3.1 來源「新魂」、「怨靈」）：G = S = W = 0，道緣取 `province` 的 faith4。
///
/// 狀態暫為 `Bardo { until: 現在 }`，呼叫端應立即以 [`attach_person`] 綁定肉身；
/// 否則下個月它會被當成死者判定。不檢查上限（呼叫端自行檢查，GDD 3.2「出生的延後」）。
pub fn create_soul(w: &mut World, origin: SoulOrigin, province: ProvinceId) -> SoulId {
    ensure_heaven(w);
    let id = w.souls.len() as SoulId;
    let affinity = w.provinces.get(province as usize).map_or([0.25; 4], faith4);
    let mut marks = SoulMarks::default();
    if origin == SoulOrigin::Wraith {
        marks.set(SoulMarks::WRAITH);
    }
    w.souls.push(Soul {
        id,
        merit: 0.0,
        sin: 0.0,
        wisdom: 0.0,
        affinity,
        threads: Vec::new(),
        lives: Vec::new(),
        best_prev: Achievements::default(),
        best_now: Achievements::default(),
        state: SoulState::Bardo { until: w.tick },
        origin,
        marks,
        last_place: (province, 0),
        guide: None,
    });
    super::arm(w, w.souls.len() as SoulId - 1);
    id
}

/// 新魂降生：[`create_soul`] + 建立肉身 + 寫入 `Born` 事件（GDD 7.2：新魂用 `Born`，無因）。
pub fn spawn_new_life(w: &mut World, origin: SoulOrigin, province: ProvinceId) -> (SoulId, PersonId) {
    let sid = create_soul(w, origin, province);
    let cell = pick_cell(w, province);
    let pid = spawn_person(w, sid, province, cell);
    attach_person(w, sid, pid);
    let mut e = NewEvent::new(EventKind::Born, 1, EventArgs::None);
    e.province = Some(province);
    e.cell = Some(cell);
    e.souls = smallvec![sid];
    e.persons = smallvec![pid];
    ledger::record(w, e);
    (sid, pid)
}
