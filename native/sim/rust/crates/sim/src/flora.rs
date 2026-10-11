//! GDD 2.8「每世結算」第 3 步：草木——人改天地。墾殖、復林、沙化與回復。
//!
//! 每州有一個墾殖度，每世向「人口 ÷ 開局時的承載」靠近；墾出來的新承載不算進分母，人口隨承平與
//! 災荒起伏，墾殖度才有上有下。墾殖與沙化記在遊戲格的旗標上，由 [`crate::landform`] 疊到氣候判出的
//! 目標地形上：氣候要它是森林而已墾，就是平原；要它是平原而已沙化，就是沙漠。

use serde::{Deserialize, Serialize};

use crate::params::*;
use crate::sim::World;
use crate::types::{FineGeography, ProvinceId, Terrain};
use crate::worldgen::grid::cheb;

/// 草木的狀態（納入雜湊；存檔是種子加指令紀錄，重播時自然重建）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Flora {
    /// 每州的墾殖度，0..1。
    pub cultivation: Vec<f32>,
    /// 每州開局時的承載：墾殖度的分母。
    pub opening_capacity: Vec<f64>,
    /// 遊戲格：森林被墾為平原。
    pub cleared: Vec<bool>,
    /// 遊戲格：平原沙化為沙漠。
    pub sanded: Vec<bool>,
}

/// 這一世草木的變動。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FloraShift {
    /// 旗標有變的遊戲格，升冪。
    pub touched: Vec<u32>,
    pub cleared: u32,
    pub regrown: u32,
    pub sanded: u32,
    pub healed: u32,
    /// 這一世把森林墾盡的州。
    pub exhausted: Vec<ProvinceId>,
    /// 這一世開始沙化的州（原本沒有沙化的格）。
    pub desertified: Vec<ProvinceId>,
}

/// 建立草木的狀態：墾殖度取當下的人口 ÷ 承載，承載記為開局時的承載。開天之後呼叫；
/// 以 `from_parts` 建立的世界在第一次每世結算時建立。回傳這個世界有沒有細格資料。
pub fn prepare(w: &mut World) -> bool {
    if !crate::landform::prepare(w) {
        return false;
    }
    let cells = w.geo.terrain.len();
    let flora = &mut w.flora;
    if flora.cultivation.len() != w.provinces.len() || flora.cleared.len() != cells {
        flora.opening_capacity = w.provinces.iter().map(|p| p.capacity).collect();
        flora.cultivation = w.provinces.iter().map(|p| ratio(p.population, p.capacity)).collect();
        flora.cleared = vec![false; cells];
        flora.sanded = vec![false; cells];
    }
    true
}

fn ratio(population: f64, capacity: f64) -> f32 {
    if capacity > 0.0 {
        (population / capacity).clamp(0.0, 1.0) as f32
    } else {
        0.0
    }
}

/// `n` 格裡這一世要動的格數：比例無條件進位，至少一格。
fn share(n: usize, part: f32) -> usize {
    ((n as f32 * part).ceil() as usize).clamp(1, n)
}

/// 遊戲格生成時的濕度：格內細格的平均，0..1。
fn cell_moisture(fine: &FineGeography, gw: usize, cell: usize) -> f32 {
    let (scale, fw) = (fine.scale as usize, fine.w as usize);
    let (x, y) = (cell % gw, cell / gw);
    let mut sum = 0u32;
    for dy in 0..scale {
        for dx in 0..scale {
            sum += u32::from(fine.moisture[(y * scale + dy) * fw + x * scale + dx]);
        }
    }
    sum as f32 / (255 * scale * scale) as f32
}

/// GDD 2.8「每世結算」第 3 步：更新各州的墾殖度，選出這一世墾殖、復林、沙化與回復的格並改旗標。
/// 地形由緊接著的 [`crate::landform::settle_with`] 依旗標改。沒有細格資料的手工世界不演變。
pub fn settle(w: &mut World) -> FloraShift {
    let mut shift = FloraShift::default();
    if !prepare(w) {
        return shift;
    }
    let gw = w.geo.w as usize;
    let terrain = &w.geo.terrain;
    let fine = &w.fine;
    let wet = 1.0 + EVO_WARMTH_MOISTURE * w.evolution.warmth_delta();
    let flora = &mut w.flora;
    for (i, p) in w.provinces.iter().enumerate() {
        if p.lost || p.cells.is_empty() || flora.opening_capacity[i] <= 0.0 {
            continue;
        }
        let target = ratio(p.population, flora.opening_capacity[i]);
        flora.cultivation[i] += (target - flora.cultivation[i]) * EVO_CULTIVATION_APPROACH;
        let cultivation = flora.cultivation[i];
        let seat = p.seat as usize;
        let near = |c: u32| cheb(gw, seat, c as usize);

        // 墾殖：離治所近的森林先墾。復林：離治所遠的先長回來。
        if cultivation > EVO_CLEAR_ABOVE {
            let mut forest: Vec<u32> =
                p.cells.iter().copied().filter(|&c| terrain[c as usize] == Terrain::Forest && !flora.cleared[c as usize]).collect();
            if !forest.is_empty() {
                forest.sort_by_key(|&c| (near(c), c));
                let k = share(forest.len(), EVO_CLEAR_SHARE);
                for &c in &forest[..k] {
                    flora.cleared[c as usize] = true;
                    shift.touched.push(c);
                }
                shift.cleared += k as u32;
                if k == forest.len() {
                    shift.exhausted.push(p.id);
                }
            }
        } else if cultivation < EVO_REGROW_BELOW {
            let mut cleared: Vec<u32> = p.cells.iter().copied().filter(|&c| flora.cleared[c as usize]).collect();
            if !cleared.is_empty() {
                cleared.sort_by_key(|&c| (std::cmp::Reverse(near(c)), c));
                let k = share(cleared.len(), EVO_REGROW_SHARE);
                for &c in &cleared[..k] {
                    flora.cleared[c as usize] = false;
                    shift.touched.push(c);
                }
                shift.regrown += k as u32;
            }
        }

        // 沙化：乾燥的州裡最乾的平原先沙化，沙化到平原的一定比例為止。回復：最濕的先回復。
        let mut sanded: Vec<u32> = p.cells.iter().copied().filter(|&c| flora.sanded[c as usize]).collect();
        if cultivation > EVO_SAND_ABOVE {
            let mut plains: Vec<u32> =
                p.cells.iter().copied().filter(|&c| terrain[c as usize] == Terrain::Plains && !flora.sanded[c as usize]).collect();
            let limit = ((plains.len() + sanded.len()) as f32 * EVO_SAND_LIMIT) as usize;
            let room = limit.saturating_sub(sanded.len());
            if plains.is_empty() || room == 0 {
                continue;
            }
            let moisture: f32 = p.cells.iter().map(|&c| cell_moisture(fine, gw, c as usize)).sum::<f32>() / p.cells.len() as f32;
            if moisture * wet >= EVO_SAND_MOISTURE {
                continue;
            }
            plains.sort_by(|&a, &b| {
                cell_moisture(fine, gw, a as usize).total_cmp(&cell_moisture(fine, gw, b as usize)).then(a.cmp(&b))
            });
            let k = share(plains.len(), EVO_SAND_SHARE).min(room);
            for &c in &plains[..k] {
                flora.sanded[c as usize] = true;
                shift.touched.push(c);
            }
            shift.sanded += k as u32;
            if sanded.is_empty() {
                shift.desertified.push(p.id);
            }
        } else if cultivation < EVO_REGROW_BELOW && !sanded.is_empty() {
            sanded.sort_by(|&a, &b| {
                cell_moisture(fine, gw, b as usize).total_cmp(&cell_moisture(fine, gw, a as usize)).then(a.cmp(&b))
            });
            let k = share(sanded.len(), EVO_REGROW_SHARE);
            for &c in &sanded[..k] {
                flora.sanded[c as usize] = false;
                shift.touched.push(c);
            }
            shift.healed += k as u32;
        }
    }
    shift.touched.sort_unstable();
    shift.touched.dedup();
    shift
}
