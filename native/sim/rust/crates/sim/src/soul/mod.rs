//! GDD 3.1 魂、3.2 六道輪迴（U4）。
//!
//! 狀態機：`Incarnate` →（死亡）→ `Bardo` →（判定）→ 人道／阿修羅道出生，或 `Realm`（天道、為獸、餓鬼、地獄）
//! →（期滿）→ 重新判定；人道判定時若有值得等的宿緣對象則先 `Waiting`。超脫者為 `Liberated`，永久離開。
//! 所有亂數取自 `World::rng.soul`；同一步內依魂 ID 升冪處理（GDD 6.3、6.5）。
//!
//! # 給其他系統的介面
//!
//! | 函式 | 誰呼叫 | 用途 |
//! |---|---|---|
//! | [`monthly`] | `Sim::step` 步驟 5 | 中陰到期判定、他道期滿重判、等待逾時、配對與出生 |
//! | [`yearly_refill`] | `Sim::step` 年度（人間補魂） | GDD 3.4 來源第 3 項 |
//! | [`on_death`] | people／dao／war（任何步驟中人物死亡時） | GDD 6.3「死亡」第 2 步：魂入中陰或超脫；併入今生成就 |
//! | [`take_for_birth`] | people（王族生育） | 找出「已判入人道、在等待、宿緣對象在本州」的魂 |
//! | [`create_soul`]、[`spawn_new_life`] | people（補足）、genesis、war（怨靈） | 建立新魂（先檢查 [`living_notables`] 與 [`notable_cap`]） |
//! | [`prepare_rebirth`]、[`attach_person`] | people（自行建立肉身時） | 宿慧／道緣傳承與利息；把肉身掛到魂上 |
//! | [`pull::faith4`]、[`pull::pull_distribution`] | genesis、people | faith4、因果牽引分布 |
//!
//! 出生時輪迴系統自己建立肉身：`rebirth::spawn_person` 委派 `people::spawn`（屬性吃宿慧、G−S、修羅印記，
//! 所以必須在 [`prepare_rebirth`] 之後呼叫）。
//!
//! # 約定
//!
//! - `souls[i].id == i`，`souls[0]` 為天道（GDD 3.1）；`persons[i].id == i`；`provinces[i].id == i`。
//! - U7 建線時須把 ThreadId 加進雙方魂的 `threads`（宿緣分布、利息、等待皆經由它找線）。
//! - 呼叫 [`on_death`] 前，呼叫端須已設定 `Person::died`、寫入死亡事件（使 `Person::last_event` 指向它，
//!   `Judged`、`Rebirth` 以它為因）。
//! - 暫存旗標 `SoulMarks::LAST_LIFE_*`：前世入魔／兵解／被度化，由死亡處理設定，下次轉世消耗。
//!   「被度化」由度化系統（M4）設定。
//! - 超度加成需要州郡香火（M4 寺院）；M2 沒有寺院，恆為 0（見 `incense`）。

mod judge;
pub mod pull;
mod rebirth;
#[cfg(test)]
mod tests;

use smallvec::smallvec;

pub use judge::{judge_path, judge_value, realm_months, wait_target};
pub use rebirth::{attach_person, create_soul, ensure_heaven, prepare_rebirth, rebirth, spawn_new_life};

use rand::Rng;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::{ProvinceId, Tick, WorldSize};

/// 在世有名之人上限 `NOTABLE_CAP`（依世界尺寸，GDD 7.1）。
pub fn notable_cap(w: &World) -> u32 {
    NOTABLE_CAP[match w.params.size {
        WorldSize::Small => 0,
        WorldSize::Medium => 1,
        WorldSize::Large => 2,
    }]
}

/// 在世有名之人數（含幼年者）。
pub fn living_notables(w: &World) -> u32 {
    w.alive_ids.iter().filter(|&&i| w.persons.get(i as usize).is_some_and(|p| p.alive())).count() as u32
}

/// 州郡寺院香火（GDD 3.6）。M2 沒有寺院，恆為 0；M4 起改讀州郡的香火欄位。
fn incense(_w: &World, _province: ProvinceId) -> f32 {
    0.0
}

/// GDD 6.3「死亡」第 2 步：人物死亡時由死亡處理呼叫。
///
/// - `liberation` 有值（飛升、涅槃、成聖）→ `Liberated`，不經中陰。
/// - 否則 → `Bardo { until: tick + BARDO_MONTHS }`。
/// - 把這一世的成就（境界、果位、三不朽總和）併入 `best_now`；入魔者、兵解者設暫存旗標；
///   `last_place` 記下死亡地點。
///
/// 該人物不是魂目前的肉身時不做事。
pub fn on_death(w: &mut World, pid: PersonId, liberation: Option<Liberation>) {
    let Some(p) = w.persons.get(pid as usize) else { return };
    let sid = p.soul;
    let place = (p.province, p.cell);
    let demon = p.role == Role::Demonic;
    let shed = matches!(p.died, Some((_, DeathCause::ShedBody)));
    let now = Achievements {
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
    };
    let tick = w.tick;
    let Some(soul) = w.souls.get_mut(sid as usize) else { return };
    if soul.state != SoulState::Incarnate(pid) {
        return;
    }
    soul.best_now = Achievements {
        realm: soul.best_now.realm.max(now.realm),
        fruit: soul.best_now.fruit.max(now.fruit),
        immortality: soul.best_now.immortality.max(now.immortality),
    };
    if demon {
        soul.marks.set(SoulMarks::LAST_LIFE_DEMON);
    }
    if shed {
        soul.marks.set(SoulMarks::LAST_LIFE_SHED);
    }
    soul.last_place = place;
    soul.state = match liberation {
        Some(way) => SoulState::Liberated { way, tick },
        None => SoulState::Bardo { until: tick + BARDO_MONTHS },
    };
}

/// GDD 3.4 王族子女：取一個「已判入人道、在等待中、宿緣對象在世且在 `province`」的魂（依魂 ID 升冪取第一個）。
/// 不改變狀態；呼叫端建立肉身後以 [`prepare_rebirth`] + [`attach_person`] 完成出生。
/// 沒有時回傳 `None`，呼叫端改用 [`create_soul`]。
///
/// 只考慮 `Waiting` 的魂：中陰中的魂尚未判定，不知會去哪一道。
pub fn take_for_birth(w: &World, province: ProvinceId) -> Option<SoulId> {
    for soul in &w.souls {
        let SoulState::Waiting { for_souls, .. } = &soul.state else { continue };
        let near = for_souls.iter().any(|&o| {
            matches!(
                w.souls.get(o as usize).map(|s| &s.state),
                Some(SoulState::Incarnate(pid)) if w.persons.get(*pid as usize).is_some_and(|p| p.province == province)
            )
        });
        if near {
            return Some(soul.id);
        }
    }
    None
}

/// 月份數轉換用：從 `[lo, hi]`（含）抽一個月數（魂子流）。
fn draw_months(w: &mut World, (lo, hi): (u32, u32)) -> u32 {
    w.rng.soul.gen_range(lo..=hi)
}

/// 決定去處：引渡令 > 菩薩願 > 判定表。回傳 (去處, 出生州郡覆寫)。
/// 不論走哪條路都先抽雜訊，使亂數消耗與路徑無關。
fn choose_path(w: &mut World, sid: SoulId) -> (SixPath, Option<ProvinceId>) {
    let noise = w.rng.soul.gen_range(-PATH_NOISE..PATH_NOISE);
    let soul = &w.souls[sid as usize];
    if let Some(g) = soul.guide {
        return (g.path, g.province);
    }
    if soul.marks.has(SoulMarks::BODHISATTVA_VOW) {
        return (SixPath::Human, Some(pull::heaviest_karma_province(w)));
    }
    let salvation = SALVATION_PER_INCENSE * incense(w, soul.last_place.0);
    let x = judge_value(soul, salvation, noise);
    (judge_path(soul, x), None)
}

fn record_judged(w: &mut World, sid: SoulId, path: SixPath) {
    let soul = &w.souls[sid as usize];
    let cause = soul.lives.last().and_then(|&p| w.persons.get(p as usize)).and_then(|p| p.last_event);
    let mut e = NewEvent::new(EventKind::Judged, 1, EventArgs::Judged { path });
    e.province = Some(soul.last_place.0);
    e.cell = Some(soul.last_place.1);
    e.souls = smallvec![sid];
    if let Some(c) = cause {
        e.causes = smallvec![c];
    }
    ledger::record(w, e);
}

/// 一個月的魂處理內共用的暫存：都是由世界狀態導出的，只在這個月內有效，不進存檔與雜湊。
struct Month {
    cap: u32,
    /// 在世有名之人數；第一次用到時才算（見 [`monthly`]）。
    living: Option<u32>,
    /// 狀態為 `Waiting` 的魂（升冪）。第一次要找等待者時才掃描建立，之後隨狀態轉換維護。
    waiting: Option<Vec<SoulId>>,
    /// 人口分布與各州郡的 faith4；第一次抽出生州郡時才算（月內州郡人口與信仰不變）。
    pop: Option<(Vec<f64>, Vec<[f32; 4]>)>,
}

impl Month {
    fn living(&mut self, w: &World) -> &mut u32 {
        self.living.get_or_insert_with(|| living_notables(w))
    }

    fn waiting(&mut self, w: &World) -> &mut Vec<SoulId> {
        self.waiting.get_or_insert_with(|| w.souls.iter().filter(|s| matches!(s.state, SoulState::Waiting { .. })).map(|s| s.id).collect())
    }

    fn birth_province(&mut self, w: &mut World, sid: SoulId) -> ProvinceId {
        let (pop, faith) = self.pop.get_or_insert_with(|| (pull::population_dist(w), pull::faith_table(w)));
        pull::sample_birth_province_with(w, sid, pop, faith)
    }

    /// 魂 `sid` 剛離開等待（出生）。
    fn left_waiting(&mut self, sid: SoulId) {
        if let Some(v) = &mut self.waiting {
            if let Ok(k) = v.binary_search(&sid) {
                v.remove(k);
            }
        }
    }

    /// 魂 `sid` 剛進入等待。
    fn entered_waiting(&mut self, sid: SoulId) {
        if let Some(v) = &mut self.waiting {
            if let Err(k) = v.binary_search(&sid) {
                v.insert(k, sid);
            }
        }
    }
}

/// 對一個該判定的魂（中陰到期、他道期滿）做六道判定並安置。
fn judge_and_place(w: &mut World, sid: SoulId, m: &mut Month) {
    let tick = w.tick;
    let (path, province_override) = choose_path(w, sid);

    if let Some((lo, hi)) = realm_months(path) {
        let dur = draw_months(w, (lo, hi));
        let soul = &mut w.souls[sid as usize];
        soul.guide = None;
        soul.marks.clear(SoulMarks::ASURA);
        soul.state = SoulState::Realm { path, until: tick + dur };
        record_judged(w, sid, path);
        return;
    }

    // 人道、阿修羅道。
    let waiters: Vec<SoulId> = m
        .waiting(w)
        .iter()
        .copied()
        .filter(|&o| matches!(&w.souls[o as usize].state, SoulState::Waiting { for_souls, .. } if for_souls.contains(&sid)))
        .collect();

    // 有人在等本魂時一律配對出生；否則看本魂是否要等別人。
    if waiters.is_empty() && province_override.is_none() {
        if let Some(other) = wait_target(w, &w.souls[sid as usize], tick) {
            let soul = &mut w.souls[sid as usize];
            soul.guide = None;
            soul.marks.clear(SoulMarks::ASURA);
            if path == SixPath::Asura {
                soul.marks.set(SoulMarks::ASURA);
            }
            soul.state = SoulState::Waiting { until: tick + WAIT_MAX_YEARS * 12, for_souls: smallvec![other] };
            m.entered_waiting(sid);
            record_judged(w, sid, path);
            return;
        }
    }

    // 先配對，再檢查上限；達上限時整組一起延後（等待者維持原狀）。
    let group = 1 + waiters.len() as u32;
    if *m.living(w) + group > m.cap {
        w.souls[sid as usize].state = SoulState::Bardo { until: tick + BIRTH_DEFER_YEARS * 12 };
        return;
    }

    let province = match province_override {
        Some(p) => p,
        None => m.birth_province(w, sid),
    };
    {
        let soul = &mut w.souls[sid as usize];
        soul.guide = None;
        soul.marks.clear(SoulMarks::ASURA);
        if path == SixPath::Asura {
            soul.marks.set(SoulMarks::ASURA);
        }
    }
    record_judged(w, sid, path);
    rebirth(w, sid, province);
    for wid in waiters {
        let p = pull::sample_near(w, province);
        rebirth(w, wid, p);
        m.left_waiting(wid);
    }
    *m.living(w) += group;
}

/// 等待逾時：照常以因果牽引出生；達上限則再等 1 年。
fn birth_after_wait(w: &mut World, sid: SoulId, m: &mut Month) {
    if *m.living(w) + 1 > m.cap {
        if let SoulState::Waiting { until, .. } = &mut w.souls[sid as usize].state {
            *until += BIRTH_DEFER_YEARS * 12;
        }
        return;
    }
    let province = m.birth_province(w, sid);
    rebirth(w, sid, province);
    m.left_waiting(sid);
    *m.living(w) += 1;
}

/// GDD 6.3 步驟 5：中陰到期的魂六道判定；他道期滿的魂重判（天道 G × 0.5、為獸 S × 0.9）；
/// 等待逾時的魂出生；餓鬼、地獄每年衰減 S；配對出生。
pub fn monthly(w: &mut World) {
    if w.souls.is_empty() {
        return;
    }
    let tick: Tick = w.tick;

    if tick.is_multiple_of(12) {
        for soul in &mut w.souls {
            if let SoulState::Realm { path, .. } = soul.state {
                match path {
                    SixPath::Hungry => soul.sin *= PATH_HUNGRY_S_DECAY_PER_YEAR,
                    SixPath::Hell => soul.sin *= PATH_HELL_S_DECAY_PER_YEAR,
                    _ => {}
                }
            }
        }
    }

    let mut month = Month { cap: notable_cap(w), living: None, waiting: None, pop: None };
    // 在世有名之人數要掃全部在世者，多數月份沒有任何魂到期，所以第一次用到時才算
    // （第一個到期之前世界沒有改變，數值與一開始就算相同）。
    // 狀態會在迴圈中被配對改變，因此每次重讀。
    for i in 0..w.souls.len() {
        let sid = i as SoulId;
        // 先以參照判斷是否到期（多數魂沒有到期，不複製狀態）。
        let due = match &w.souls[i].state {
            SoulState::Bardo { until } if *until <= tick => 1,
            SoulState::Realm { until, .. } if *until <= tick => 2,
            SoulState::Waiting { until, .. } if *until <= tick => 3,
            _ => 0,
        };
        if due == 0 {
            continue;
        }
        match due {
            1 => judge_and_place(w, sid, &mut month),
            2 => {
                let SoulState::Realm { path, .. } = w.souls[i].state else { continue };
                let soul = &mut w.souls[i];
                match path {
                    SixPath::Heaven => soul.merit *= PATH_HEAVEN_RETURN_G_MULT,
                    SixPath::Beast => soul.sin *= PATH_BEAST_RETURN_S_MULT,
                    _ => {}
                }
                judge_and_place(w, sid, &mut month);
            }
            _ => birth_after_wait(w, sid, &mut month),
        }
    }
}

/// GDD 3.4 人間補魂：每年，在世有名之人 < `NOTABLE_FLOOR` 時，
/// 建立 `ceil((NOTABLE_FLOOR − 在世數) × REFILL_RATE)` 個新魂（不超過上限），依州郡人口分布選出生地，年齡 0。
pub fn yearly_refill(w: &mut World) {
    if w.provinces.is_empty() {
        return;
    }
    let cap = notable_cap(w);
    let floor = cap as f32 * NOTABLE_FLOOR_RATIO;
    let living = living_notables(w);
    if (living as f32) >= floor {
        return;
    }
    let want = ((floor - living as f32) * REFILL_RATE).ceil() as u32;
    let n = want.min(cap.saturating_sub(living));
    let dist = pull::population_dist(w);
    for _ in 0..n {
        let province = pull::sample_dist(w, &dist);
        spawn_new_life(w, SoulOrigin::New, province);
    }
}
