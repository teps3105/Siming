//! GDD 3.4 婚配與子女（M2：只有一般婚配；王族生育在 M3）。

use rand::Rng;
use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::schedule::Round;
use crate::sim::World;

/// 7.2 `Married` 重要度。
const MARRIED_IMPORTANCE: u8 = 2;
/// 可婚配者：在世、年滿 18、未婚、非僧人（僧人不婚）、非幼年。
fn eligible(p: &Person, tick: u32) -> bool {
    p.alive() && p.spouse.is_none() && p.age(tick) >= MARRY_AGE && !matches!(p.role, Role::Child | Role::Monk)
}

/// 結為夫妻：互設 `spouse`，寫入 `Married`（因：雙方最近事件），再由 `thread::on_marriage` 結算情緣線。回傳 `Married` 事件。
pub fn marry(w: &mut World, a: PersonId, b: PersonId) -> EventId {
    w.persons[a as usize].spouse = Some(b);
    w.persons[b as usize].spouse = Some(a);
    let (pa, pb) = (&w.persons[a as usize], &w.persons[b as usize]);
    let mut e = NewEvent::new(EventKind::Married, MARRIED_IMPORTANCE, EventArgs::None);
    e.province = Some(pa.province);
    e.cell = Some(pa.cell);
    e.souls = smallvec![pa.soul, pb.soul];
    e.persons = smallvec![a, b];
    for p in [pa, pb] {
        if let Some(c) = p.last_event {
            e.causes.push(c);
        }
        if let Some(pw) = p.power {
            if !e.powers.contains(&pw) {
                e.powers.push(pw);
            }
        }
    }
    let eid = ledger::record(w, e);
    // 情緣線的建立與了結（再續前緣）統一由 thread 處理（3.3、4.5）。
    crate::thread::on_marriage(w, a, b, Some(eid));
    eid
}

/// GDD 6.3 步驟 13：婚配。年滿 18 且未婚者每年 10%，與同州或陸鄰、年齡差 ≤ 10 的未婚異性
/// （同樣年滿 18、非僧人）結為夫妻或道侶；有同宗門（`power` 相同）的候選時優先從中挑。
/// 依人物 ID 升冪；每個符合資格者恆消耗 1 次機率抽樣，成功者另抽 1 次挑選候選。
///
/// 發起判定的人依自己的判定月輪到才處理；候選對象則是全體符合資格者，不限判定月。`Sim::step` 每月呼叫。
pub fn monthly_marriage(w: &mut World) {
    marriage_round(w, Round::at(w.tick));
}

/// 同 [`monthly_marriage`]，但一次判定全部人（整年份；測試與工具用）。
pub fn yearly_marriage(w: &mut World) {
    marriage_round(w, Round::All);
}

fn marriage_round(w: &mut World, round: Round) {
    let tick = w.tick;
    // 以所在州郡分組的候選名單（ID 升冪）；第一次有人通過機率抽樣時才建（單月輪次多半沒有人要配對）。
    // 每筆附性別與年齡（同一個月內不變），先以它們篩掉大半，不必每個都去讀人物資料。
    let mut by_prov: Option<Vec<Vec<(PersonId, Gender, u32)>>> = None;
    for i in w.round_alive(round).into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !eligible(p, tick) {
            continue;
        }
        if w.rng.people.gen::<f32>() >= MARRY_CHANCE {
            continue;
        }
        let by_prov = by_prov.get_or_insert_with(|| {
            let mut v: Vec<Vec<(PersonId, Gender, u32)>> = vec![Vec::new(); w.provinces.len()];
            for p in w.alive_ids.iter().map(|&i| &w.persons[i as usize]) {
                if eligible(p, tick) {
                    if let Some(x) = v.get_mut(p.province as usize) {
                        x.push((p.id, p.gender, p.age(tick)));
                    }
                }
            }
            v
        });
        let (pid, age, gender, power, prov) = (p.id, p.age(tick), p.gender, p.power, p.province);
        let mut cands: Vec<PersonId> = Vec::new();
        let near = std::iter::once(prov).chain(w.provinces[prov as usize].neighbors.iter().copied());
        for q in near {
            for &(c, c_gender, c_age) in &by_prov[q as usize] {
                if c != pid && c_gender != gender && c_age.abs_diff(age) <= MARRY_AGE_GAP_MAX && eligible(&w.persons[c as usize], tick) {
                    cands.push(c);
                }
            }
        }
        if cands.is_empty() {
            continue;
        }
        cands.sort_unstable();
        if power.is_some() {
            let same: Vec<PersonId> = cands.iter().copied().filter(|&c| w.persons[c as usize].power == power).collect();
            if !same.is_empty() {
                cands = same;
            }
        }
        let other = cands[w.rng.people.gen_range(0..cands.len())];
        marry(w, pid, other);
    }
}
