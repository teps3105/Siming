//! GDD 3.5 爭脈之戰。

use rand::Rng;
use smallvec::smallvec;

use super::kill::kill;
use super::sect::{fall_sect, sect_of, RangeCache};
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::people::power_of;
use crate::schedule::{Domain, Round};
use crate::sim::World;
use crate::types::{NodeId, ProvinceId};

/// 7.2 `SectWar` 重要度 4。
const WAR_IMPORTANCE: u8 = 4;

fn live(w: &World, id: PowerId) -> bool {
    let p = &w.powers[id as usize];
    p.fallen.is_none() && sect_of(p).is_some()
}

/// 戰力前 `n` 名的在世成員 `(ID, 戰力)`，戰力降冪、同值 ID 升冪。
fn top(w: &World, id: PowerId, n: u32) -> Vec<(PersonId, f32)> {
    let mut v: Vec<(PersonId, f32)> =
        w.powers[id as usize].members.iter().filter(|&&m| w.persons[m as usize].alive()).map(|&m| (m, power_of(w, m))).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n as usize);
    v
}

/// GDD 6.3 步驟 17：兩宗勢力範圍同含一座洞天福地，且關係 < 0 或任一方為邪道時，每年 4% 開戰。
/// 一對宗門每年判定一次，落在編號較小的一宗的判定月。
pub(super) fn yearly_wars(w: &mut World, round: Round, ranges: &mut RangeCache) {
    let seed = w.params.seed;
    let ids: Vec<PowerId> = w.powers.iter().filter(|p| p.fallen.is_none() && sect_of(p).is_some()).map(|p| p.id).collect();
    // 洞天福地所在的州郡；本輪有宗門輪到時才算。
    let mut node_prov: Option<Vec<Option<ProvinceId>>> = None;
    for x in 0..ids.len() {
        if !round.takes(seed, Domain::Sect, ids[x]) {
            continue;
        }
        let node_prov = node_prov
            .get_or_insert_with(|| w.geo.nodes.iter().map(|n| w.geo.province_of.get(n.cell as usize).copied()).collect::<Vec<_>>());
        for y in x + 1..ids.len() {
            let (a, b) = (ids[x], ids[y]);
            if !live(w, a) || !live(w, b) {
                continue;
            }
            // 沒有敵意就不會開戰，先檢查（不耗亂數，結果與先找共同洞天相同）。
            let (pa, pb) = (&w.powers[a as usize], &w.powers[b as usize]);
            let hostile = pa.relations.get(&b).is_some_and(|&r| r < 0)
                || pb.relations.get(&a).is_some_and(|&r| r < 0)
                || sect_of(pa).is_some_and(|s| s.evil)
                || sect_of(pb).is_some_and(|s| s.evil);
            if !hostile {
                continue;
            }
            ranges.ensure(w, a);
            ranges.ensure(w, b);
            let (rx, ry) = (ranges.at(a), ranges.at(b));
            let Some(node) =
                (0..node_prov.len()).find(|&n| node_prov[n].is_some_and(|p| rx.binary_search(&p).is_ok() && ry.binary_search(&p).is_ok()))
            else {
                continue;
            };
            if w.rng.powers.gen::<f32>() >= SECT_WAR_CHANCE {
                continue;
            }
            fight(w, a, b, node as NodeId);
        }
    }
}

pub(super) fn fight(w: &mut World, a: PowerId, b: PowerId, node: NodeId) {
    let (ta, tb) = (top(w, a, SECT_WAR_TOP), top(w, b, SECT_WAR_TOP));
    let mul = |rng: &mut rand_chacha::ChaCha8Rng| SECT_WAR_MUL_MIN + (SECT_WAR_MUL_MAX - SECT_WAR_MUL_MIN) * rng.gen::<f32>();
    let sa: f32 = ta.iter().map(|x| x.1).sum::<f32>() * mul(&mut w.rng.powers);
    let sb: f32 = tb.iter().map(|x| x.1).sum::<f32>() * mul(&mut w.rng.powers);
    let (win, lose, tw, tl) = if sa >= sb { (a, b, ta, tb) } else { (b, a, tb, ta) };
    let before: f32 = tl.iter().map(|x| x.1).sum();

    let mut e = NewEvent::new(EventKind::SectWar, WAR_IMPORTANCE, EventArgs::Sect { node: Some(node) });
    e.province = Some(w.powers[win as usize].seat_province);
    e.powers = smallvec![win, lose];
    for id in [win, lose] {
        if let Some(c) = w.powers[id as usize].leader.and_then(|l| w.persons[l as usize].last_event) {
            e.causes.push(c);
        }
    }
    let war = ledger::record(w, e);

    // 戰死判定（敗方先、勝方後），再逐一結算；殺者自對方前 3 名在世者中隨機挑選。
    let mut doomed: Vec<(PersonId, PowerId)> = Vec::new();
    for &(m, _) in &tl {
        if w.rng.powers.gen::<f32>() < SECT_WAR_LOSER_DEATH_CHANCE {
            doomed.push((m, win));
        }
    }
    for &(m, _) in &tw {
        if w.rng.powers.gen::<f32>() < SECT_WAR_WINNER_DEATH_CHANCE {
            doomed.push((m, lose));
        }
    }
    for (victim, killer_side) in doomed {
        if !w.persons[victim as usize].alive() {
            continue;
        }
        let pool = if killer_side == win { &tw } else { &tl };
        let killers: Vec<PersonId> =
            pool.iter().take(SECT_WAR_KILLERS as usize).map(|x| x.0).filter(|&k| k != victim && w.persons[k as usize].alive()).collect();
        if killers.is_empty() {
            crate::people::die(w, victim, DeathCause::War, None, &[war]);
        } else {
            let k = killers[w.rng.powers.gen_range(0..killers.len())];
            kill(w, k, victim, DeathCause::War, Some(war));
        }
    }
    let after: f32 = tl.iter().map(|x| power_of(w, x.0)).sum();
    if after < before * SECT_WAR_FALL {
        fall_sect(w, lose, Some(war));
    }
}
