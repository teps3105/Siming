//! GDD 2.2 品質檢查（生成後自動驗證）。

use super::grid::{components, Sphere};
use super::Draft;
use crate::params::*;
use crate::types::Terrain;

/// GDD 2.2 品質檢查：全部通過回傳 `Ok`，否則回傳第一項失敗的說明。
pub(crate) fn check(d: &Draft, sp: &Sphere, land_ratio: f32, target: u32) -> Result<(), String> {
    let n = d.w * d.h;
    let land: Vec<bool> = d.terrain.iter().map(|&t| t != Terrain::Sea).collect();
    let total: f32 = (0..n).map(|c| sp.area(c)).sum();
    let land_area: f32 = (0..n).filter(|&c| land[c]).map(|c| sp.area(c)).sum();
    let ratio = land_area / total;
    if (ratio - land_ratio).abs() > WG_QC_LAND_TOL {
        return Err(format!(
            "陸地比例 {ratio:.3} 與目標 {land_ratio:.3} 相差過大"
        ));
    }
    let (label, sizes) = components(d.w, d.h, &land, true);
    let mut area = vec![0.0f32; sizes.len()];
    for c in 0..n {
        if land[c] {
            area[label[c] as usize] += sp.area(c);
        }
    }
    let biggest = area.iter().copied().fold(0.0f32, f32::max);
    if land_area <= 0.0 || biggest > WG_QC_MAIN_LANDMASS * land_area {
        return Err(format!("最大陸塊占陸地 {:.2}", biggest / land_area.max(1e-6)));
    }
    let big = area
        .iter()
        .filter(|&&a| a >= WG_QC_CONTINENT_SHARE * land_area)
        .count();
    if big < WG_QC_CONTINENTS {
        return Err(format!("≥ 5% 陸地的大洲只有 {big} 塊"));
    }
    let count = d.provinces.list.len() as f32;
    let t = target as f32;
    if (count - t).abs() > WG_QC_PROVINCE_TOL * t {
        return Err(format!("州郡數 {count} 不在目標 {t} 的 ±15% 內"));
    }
    if let Some(p) = d.provinces.list.iter().position(|p| {
        p.cells.iter().map(|&c| sp.area(c as usize)).sum::<f32>() < WG_PROVINCE_MIN_CELLS as f32
    }) {
        return Err(format!("州郡 {p} 面積不足 {WG_PROVINCE_MIN_CELLS} 格"));
    }
    let jiang = d.rivers.stems.iter().filter(|s| s.level >= 3).count();
    if jiang < WG_QC_MIN_JIANG {
        return Err(format!("入海的江只有 {jiang} 條"));
    }
    if d.nodes.len() < WG_QC_MIN_NODES {
        return Err(format!("洞天福地只有 {} 個", d.nodes.len()));
    }
    Ok(())
}
