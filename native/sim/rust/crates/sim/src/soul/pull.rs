//! GDD 3.2 因果牽引：出生地的機率分布與抽樣；GDD 3.1 faith4。

use rand::Rng;

use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::{Province, ProvinceId};

/// GDD 3.1 faith4：`(道, 釋, 儒, 濁氣 × 0.5)`，總和為 0 取均等，再正規化。
/// 濁氣 = clamp(業氣 − 0.5·功德 − 0.5·正氣, 0, 1)（GDD 2.4）。
pub fn faith4(p: &Province) -> [f32; 4] {
    let turbid = (p.karma_qi - TURBID_MIX_MERIT * p.merit - TURBID_MIX_RIGHT * p.righteous).clamp(0.0, 1.0);
    normalize4([p.faith[0], p.faith[1], p.faith[2], turbid * FAITH4_TURBID])
}

/// 和為 1；總和 ≤ 0 時取均等。
pub fn normalize4(v: [f32; 4]) -> [f32; 4] {
    let sum: f32 = v.iter().sum();
    if sum > 0.0 {
        [v[0] / sum, v[1] / sum, v[2] / sum, v[3] / sum]
    } else {
        [0.25; 4]
    }
}

fn normalized(mut v: Vec<f64>) -> Vec<f64> {
    let sum: f64 = v.iter().sum();
    if sum > 0.0 {
        for x in &mut v {
            *x /= sum;
        }
    } else if !v.is_empty() {
        let u = 1.0 / v.len() as f64;
        v.iter_mut().for_each(|x| *x = u);
    }
    v
}

/// 人口分布：`人口(p) / Σ人口`；總人口為 0 時取均等。以州郡在 `provinces` 中的位置為索引。
pub fn population_dist(w: &World) -> Vec<f64> {
    normalized(w.provinces.iter().map(|p| p.population.max(0.0)).collect())
}

/// 道緣分布：`faith4(p) · 道緣`，再正規化。
pub fn affinity_dist(w: &World, affinity: [f32; 4]) -> Vec<f64> {
    normalized(
        w.provinces
            .iter()
            .map(|p| {
                // 已沒入海中的州郡不投生（GDD 2.8）。
                if p.lost {
                    return 0.0;
                }
                let f = faith4(p);
                (0..4).map(|k| f[k] as f64 * affinity[k] as f64).sum::<f64>()
            })
            .collect(),
    )
}

/// 宿緣分布：對每條未了宿緣，若對方在世，將強度平均分給對方所在州郡及其陸鄰；再正規化。
/// 沒有任何在世的宿緣對象時回傳 `None`。
///
/// 對方的魂依 `Soul::threads` 找線（U7 須在建線時把線 ID 加進雙方的 `threads`）。
pub fn thread_dist(w: &World, soul: &Soul) -> Option<Vec<f64>> {
    let mut d = vec![0.0f64; w.provinces.len()];
    let mut any = false;
    for &tid in &soul.threads {
        let Some(t) = w.threads.get(tid as usize) else { continue };
        if t.resolved.is_some() || t.strength <= 0.0 {
            continue;
        }
        let other = if t.debtor == soul.id {
            t.creditor
        } else if t.creditor == soul.id {
            t.debtor
        } else {
            continue;
        };
        if other == soul.id || other == HEAVEN_SOUL {
            continue;
        }
        let Some(os) = w.souls.get(other as usize) else { continue };
        let SoulState::Incarnate(pid) = os.state else { continue };
        let Some(person) = w.persons.get(pid as usize) else { continue };
        let Some(home) = w.provinces.get(person.province as usize) else { continue };
        // 對方所在州郡沒有陸鄰時，只計本州。
        let share = t.strength as f64 / (1 + home.neighbors.len()) as f64;
        d[home.id as usize] += share;
        for &n in &home.neighbors {
            d[n as usize] += share;
        }
        any = true;
    }
    any.then(|| normalized(d))
}

/// GDD 3.2 `P(p) = 0.5 × 人口 + 0.2 × 道緣 + 0.3 × 宿緣`；無在世宿緣對象時 `0.7 × 人口 + 0.3 × 道緣`。
/// 回傳與 `w.provinces` 同長、和為 1 的分布。
pub fn pull_distribution(w: &World, soul: &Soul) -> Vec<f64> {
    let pop = population_dist(w);
    let aff = affinity_dist(w, soul.affinity);
    match thread_dist(w, soul) {
        Some(th) => (0..pop.len())
            .map(|i| PULL_MIX_POP as f64 * pop[i] + PULL_MIX_AFFINITY as f64 * aff[i] + PULL_MIX_THREAD as f64 * th[i])
            .collect(),
        None => (0..pop.len())
            .map(|i| PULL_MIX_NO_THREAD_POP as f64 * pop[i] + PULL_MIX_NO_THREAD_AFFINITY as f64 * aff[i])
            .collect(),
    }
}

/// 依分布抽樣一個州郡（使用魂子流，抽一個 f64）。
pub fn sample_dist(w: &mut World, dist: &[f64]) -> ProvinceId {
    let total: f64 = dist.iter().sum();
    let r = w.rng.soul.gen::<f64>() * total;
    let mut acc = 0.0;
    let mut last = 0usize;
    for (i, &p) in dist.iter().enumerate() {
        if p <= 0.0 {
            continue;
        }
        acc += p;
        last = i;
        if r < acc {
            return w.provinces[i].id;
        }
    }
    w.provinces.get(last).map_or(0, |p| p.id)
}

/// 因果牽引抽樣出生州郡。
pub fn sample_birth_province(w: &mut World, soul: SoulId) -> ProvinceId {
    let dist = pull_distribution(w, &w.souls[soul as usize]);
    sample_dist(w, &dist)
}

/// 配對出生：在 `anchor` 州或其陸鄰中均勻抽一個（GDD 3.2 配對）。
pub fn sample_near(w: &mut World, anchor: ProvinceId) -> ProvinceId {
    let Some(p) = w.provinces.get(anchor as usize) else { return anchor };
    let n = p.neighbors.len() + 1;
    let k = w.rng.soul.gen_range(0..n);
    if k == 0 { anchor } else { w.provinces[anchor as usize].neighbors[k - 1] }
}

/// 菩薩願（GDD 3.2）：業氣最重的州郡；同值取 ID 小者。
pub fn heaviest_karma_province(w: &World) -> ProvinceId {
    let mut best: Option<(ProvinceId, f32)> = None;
    for p in w.provinces.iter().filter(|p| !p.lost) {
        if best.is_none_or(|(_, k)| p.karma_qi > k) {
            best = Some((p.id, p.karma_qi));
        }
    }
    best.map_or(0, |b| b.0)
}
