//! GDD 2.3 州郡人口的年度更新。

use crate::field::{self, FieldId};
use crate::params::*;
use crate::schedule::{Domain, Round};
use crate::types::{Province, ProvinceId};

/// 承載上限 K（GDD 2.3）。M1：暫時修正 = 1。
fn capacity(p: &Province) -> f64 {
    p.habitability * POP_CAP_PER_HAB * (1.0 - CAP_DEVASTATION * p.devastation as f64) * (1.0 - CAP_KARMA * p.karma_qi as f64)
}

fn city_level(pop: f64) -> u8 {
    if pop >= CITY_LEVEL_STATE_POP {
        3
    } else if pop >= CITY_LEVEL_COUNTY_POP {
        2
    } else if pop >= CITY_LEVEL_TOWN_POP {
        1
    } else {
        0
    }
}

/// 每年執行一次（GDD 6.3 第 7 步，M1 只含人口成長、饑荒、遷徙、殘破恢復、城池等級）。
///
/// 假設 `provinces[i].id == i`。回傳本年饑荒的州郡與死亡人數（GDD 2.3，供寫入「饑荒」事件）。
pub fn yearly(provinces: &mut [Province]) -> Vec<(ProvinceId, u32)> {
    yearly_with(provinces, &[])
}

/// 同 [`yearly`]，但各州成長率乘上 `growth_mult[州 ID]`（GDD 3.7 盛世 +50%；缺項視為 1）。
pub fn yearly_with(provinces: &mut [Province], growth_mult: &[f64]) -> Vec<(ProvinceId, u32)> {
    let mut famines = Vec::new();
    // 成長與饑荒：各州獨立，依 id 順序。M1 無盛世，r 不乘倍率。
    for p in provinces.iter_mut() {
        let k = capacity(p).max(0.0);
        p.capacity = k;
        if k > 0.0 {
            let mult = growth_mult.get(p.id as usize).copied().unwrap_or(1.0);
            p.population += p.population * POP_GROWTH * mult * (1.0 - p.population / k);
        }
        if p.population > FAMINE_THRESHOLD * k {
            let deaths = (p.population - k) * FAMINE_DEATH_RATE;
            p.population -= deaths;
            // 州郡氣場的值域是 0..1，寫入經 `field::add_to` 截斷（GDD 2.4、6.9）。
            field::add_to(p, FieldId::KarmaQi, FAMINE_KARMA);
            famines.push((p.id, deaths.round() as u32));
        }
        p.population = p.population.max(0.0);
    }

    // 遷徙：以遷徙前的快照決定所有流向，再一併套用，結果與處理順序無關。
    let snapshot: Vec<(f64, f64)> = provinces.iter().map(|p| (p.population, p.capacity)).collect();
    let mut delta = vec![0.0f64; provinces.len()];
    for (i, p) in provinces.iter().enumerate() {
        let (pop, k) = snapshot[i];
        if pop <= POP_MIGRATE_THRESHOLD * k {
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for &n in &p.neighbors {
            let n = n as usize;
            let (np, nk) = snapshot[n];
            if nk <= 0.0 {
                continue;
            }
            let ratio = np / nk;
            if best.is_none_or(|(bid, br)| ratio < br || (ratio == br && n < bid)) {
                best = Some((n, ratio));
            }
        }
        // GDD 2.3：只在鄰州的承載比低於本州時遷徙。
        if let Some((n, _)) = best.filter(|&(_, br)| br < pop / k) {
            let moved = pop * POP_MIGRATE_RATE;
            delta[i] -= moved;
            delta[n] += moved;
        }
    }

    for (p, d) in provinces.iter_mut().zip(delta) {
        p.population = (p.population + d).max(0.0);
        p.devastation = (p.devastation - DEVASTATION_RECOVER).max(0.0);
        p.city_level = city_level(p.population);
    }
    famines
}

/// 分月版（GDD 6.3 判定月）：每個州郡在自己的判定月做一次年度更新（成長與饑荒 → 遷徙 → 殘破恢復 → 城池等級），
/// 公式仍是年率，一年恰好一次。依州郡 ID 升冪；遷徙以處理當下的鄰州人口承載比決定，立即套用。
/// 回傳本月饑荒的州郡與死亡人數。`growth_mult` 同 [`yearly_with`]。
pub fn monthly_with(provinces: &mut [Province], growth_mult: &[f64], seed: u64, round: Round) -> Vec<(ProvinceId, u32)> {
    let mut famines = Vec::new();
    for i in 0..provinces.len() {
        if !round.takes(seed, Domain::Province, provinces[i].id as u32) {
            continue;
        }
        let p = &mut provinces[i];
        let k = capacity(p).max(0.0);
        p.capacity = k;
        if k > 0.0 {
            let mult = growth_mult.get(p.id as usize).copied().unwrap_or(1.0);
            p.population += p.population * POP_GROWTH * mult * (1.0 - p.population / k);
        }
        if p.population > FAMINE_THRESHOLD * k {
            let deaths = (p.population - k) * FAMINE_DEATH_RATE;
            p.population -= deaths;
            field::add_to(p, FieldId::KarmaQi, FAMINE_KARMA);
            famines.push((p.id, deaths.round() as u32));
        }
        p.population = p.population.max(0.0);

        // 遷徙：本州人口過滿時，流向承載比最低的鄰州。
        let (pop, k) = (p.population, p.capacity);
        if pop > POP_MIGRATE_THRESHOLD * k {
            let mut best: Option<(usize, f64)> = None;
            for &n in &provinces[i].neighbors {
                let n = n as usize;
                let nk = capacity(&provinces[n]).max(0.0);
                if nk <= 0.0 {
                    continue;
                }
                let ratio = provinces[n].population / nk;
                if best.is_none_or(|(bid, br)| ratio < br || (ratio == br && n < bid)) {
                    best = Some((n, ratio));
                }
            }
            if let Some((n, _)) = best.filter(|&(_, br)| br < pop / k) {
                let moved = pop * POP_MIGRATE_RATE;
                provinces[i].population -= moved;
                provinces[n].population += moved;
                provinces[n].city_level = city_level(provinces[n].population);
            }
        }
        let p = &mut provinces[i];
        p.population = p.population.max(0.0);
        p.devastation = (p.devastation - DEVASTATION_RECOVER).max(0.0);
        p.city_level = city_level(p.population);
    }
    famines
}
