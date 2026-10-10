//! GDD 3.4 婚配與子女（M2：只有一般婚配；王族生育在 M3）。

use rand::Rng;
use smallvec::smallvec;

use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
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
pub fn yearly_marriage(w: &mut World) {
    let tick = w.tick;
    // 以所在州郡分組的候選名單（ID 升冪）。
    let mut by_prov: Vec<Vec<PersonId>> = vec![Vec::new(); w.provinces.len()];
    for p in w.alive_ids.iter().map(|&i| &w.persons[i as usize]) {
        if eligible(p, tick) {
            if let Some(v) = by_prov.get_mut(p.province as usize) {
                v.push(p.id);
            }
        }
    }
    for i in w.alive_snapshot().into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !eligible(p, tick) {
            continue;
        }
        if w.rng.people.gen::<f32>() >= MARRY_CHANCE {
            continue;
        }
        let (pid, age, gender, power, prov) = (p.id, p.age(tick), p.gender, p.power, p.province);
        let mut cands: Vec<PersonId> = Vec::new();
        let near = std::iter::once(prov).chain(w.provinces[prov as usize].neighbors.iter().copied());
        for q in near {
            for &c in &by_prov[q as usize] {
                let o = &w.persons[c as usize];
                if c != pid && eligible(o, tick) && o.gender != gender && o.age(tick).abs_diff(age) <= MARRY_AGE_GAP_MAX {
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
