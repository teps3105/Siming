//! GDD 2.2 步驟 4：氣候（溫度、濕度）。
//!
//! 溫度依緯度（赤道熱、兩極冷）減高度遞減加低頻雜訊；濕度 = 氣團濕度（沿三圈環流的風向掃描：信風自東、西風帶自西、
//! 極地東風自東，經海回濕、經陸失濕、超過隨高度降低的含水上限即降雨成雨影——mapgen4 的地形雨；再做南北平滑）+ 海岸項 + 低頻雜訊 − 副熱帶乾燥帶（中心緯度
//! 隨低頻雜訊擺動），再乘濕潤度。雜訊使生態區呈片狀而非帶狀（決策 #31）。

use super::grid::{nb8, stretch, Sphere, NONE, PI};
use super::noise::Noises;
use crate::params::*;
use std::collections::VecDeque;

pub struct Climate {
    /// 溫度 t。
    pub t: Vec<f32>,
    /// 濕度 q（已乘上濕潤度）。
    pub q: Vec<f32>,
}

/// 到海的步數距離（八鄰、對角算 1 步；x 環繞）；海格為 0。
fn sea_distance(w: usize, h: usize, is_sea: &[bool]) -> Vec<u32> {
    let n = w * h;
    let mut d = vec![NONE; n];
    let mut dq: VecDeque<usize> = VecDeque::new();
    for c in 0..n {
        if is_sea[c] {
            d[c] = 0;
            dq.push_back(c);
        }
    }
    while let Some(c) = dq.pop_front() {
        for (nb, _) in nb8(w, h, c) {
            if d[nb] == NONE {
                d[nb] = d[c] + 1;
                dq.push_back(nb);
            }
        }
    }
    d
}

/// 東風的權重：信風帶與極地東風為 1、西風帶為 0，風帶邊界 ±`WG_WIND_BLEND`° 內平滑過渡。
pub fn east_weight(abs_lat_deg: f32) -> f32 {
    let s = |edge: f32| super::height::smoothstep(edge - WG_WIND_BLEND, edge + WG_WIND_BLEND, abs_lat_deg);
    1.0 - s(WG_WIND_TRADE_LAT) + s(WG_WIND_WESTERLY_LAT)
}

/// GDD 2.2 步驟 4：溫度 `t = 1.0 − 1.1·|lat|/90° − 0.3·陸高 + 0.22·(noise − 0.5)`；
/// 濕度 `q = 0.4·noise + 0.4·氣團濕度 + 0.2·coast − 乾燥帶`，再乘濕潤度。
pub fn compute(sp: &Sphere, height: &[f32], sea: f32, humidity: f32, scale: usize, nz: &Noises) -> Climate {
    // 每格的增減量以遊戲格為單位定義；地理格較細時按 1/scale 換算（距離類參數乘 scale）。
    let k = 1.0 / scale as f32;
    let (w, h) = (sp.w, sp.h);
    let n = w * h;
    let is_sea: Vec<bool> = height.iter().map(|&v| v < sea).collect();
    let dist = sea_distance(w, h, &is_sea);
    let mut tn_f = vec![0.0f32; n];
    let mut qn_f = vec![0.0f32; n];
    let mut dn_f = vec![0.0f32; n];
    for c in 0..n {
        let p = sp.unit(c);
        tn_f[c] = nz.temp(p);
        qn_f[c] = nz.humid(p);
        dn_f[c] = nz.dry(p);
    }
    stretch(&mut tn_f);
    stretch(&mut qn_f);
    stretch(&mut dn_f);
    let land_h = |i: usize| ((height[i] - sea) / (1.0 - sea)).max(0.0);

    // 氣團濕度（mapgen4 的地形雨）：每列分別自東、自西掃描兩圈（第一圈只為建立起始狀態）：經海回濕；經陸失濕；
    // 空氣含水上限 cap = 1 − k·陸高，超過者降下 rain = r·(m − cap)（迎風坡多雨、背風坡乾，山越高越明顯）。
    // 兩方向的結果依風帶權重混合（風帶邊界 ±WG_WIND_BLEND° 內以 smoothstep 過渡，避免條帶斷裂）。
    let sweep = |y: usize, east: bool, out: &mut [f32]| {
        let mut m = WG_MOIST_START;
        for step in 0..2 * w {
            let x = if east { (2 * w - 1 - step) % w } else { step % w };
            let i = y * w + x;
            if is_sea[i] {
                m = (m + WG_MOIST_SEA_GAIN * k).min(1.0);
            } else {
                let cap = 1.0 - WG_OROG_CAP * land_h(i);
                let rain = WG_OROG_RAIN * k * (m - cap).max(0.0);
                m = (m - rain - WG_MOIST_LAND_LOSS * k).max(0.0);
            }
            if step >= w {
                out[x] = m;
            }
        }
    };
    let mut moist = vec![0.0f32; n];
    let mut from_e = vec![0.0f32; w];
    let mut from_w = vec![0.0f32; w];
    for y in 0..h {
        let lat_deg = sp.lat[y].abs() * 180.0 / PI;
        let we = east_weight(lat_deg);
        sweep(y, true, &mut from_e);
        sweep(y, false, &mut from_w);
        for x in 0..w {
            moist[y * w + x] = we * from_e[x] + (1.0 - we) * from_w[x];
        }
    }
    // 南北平滑（每欄取 2·WG_MOIST_SMOOTH_ROWS + 1 列的平均），消除逐列掃描的條紋。
    let raw = moist.clone();
    for y in 0..h {
        let y0 = y.saturating_sub(WG_MOIST_SMOOTH_ROWS * scale);
        let y1 = (y + WG_MOIST_SMOOTH_ROWS * scale).min(h - 1);
        for x in 0..w {
            let mut s = 0.0f32;
            for yy in y0..=y1 {
                s += raw[yy * w + x];
            }
            moist[y * w + x] = s / (y1 - y0 + 1) as f32;
        }
    }

    let mut t = vec![0.0f32; n];
    let mut q = vec![0.0f32; n];
    for y in 0..h {
        let lat_deg = sp.lat[y].abs() * 180.0 / PI;
        let lat_t = WG_TEMP_EQUATOR - WG_TEMP_LAT_COEF * (lat_deg / 90.0);
        for x in 0..w {
            let i = y * w + x;
            // 乾燥帶中心緯度隨低頻雜訊擺動 ±WG_DRY_BELT_WOBBLE。
            let center = WG_DRY_BELT_LAT + WG_DRY_BELT_WOBBLE * (2.0 * dn_f[i] - 1.0);
            let dz = (lat_deg - center) / WG_DRY_BELT_HALF;
            let dry = WG_DRY_BELT_DEPTH
                * (-dz * dz).exp()
                * (WG_DRY_BELT_PATCH_BASE + WG_DRY_BELT_PATCH_SPAN * qn_f[i]);
            t[i] = lat_t - WG_TEMP_ALT_COEF * land_h(i) + WG_TEMP_NOISE_COEF * (tn_f[i] - 0.5);
            let coast = (-(dist[i] as f32) / (WG_COAST_DECAY * scale as f32)).exp();
            q[i] = (WG_HUMID_NOISE_WEIGHT * qn_f[i]
                + WG_HUMID_MOIST_WEIGHT * moist[i]
                + WG_HUMID_COAST_WEIGHT * coast
                - dry)
                * humidity;
            q[i] = q[i].max(0.0);
        }
    }
    // 生態區連續：溫度與濕度場各做一次方框平滑（半徑 WG_BIOME_SMOOTH 遊戲格 × 解析度；x 環繞、y 夾邊）。
    let r = WG_BIOME_SMOOTH * scale;
    let t = box_blur(&t, w, h, r);
    let q = box_blur(&q, w, h, r);
    Climate { t, q }
}

/// 可分離的方框平滑（x 環繞、y 夾邊），固定順序。
fn box_blur(v: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    if r == 0 {
        return v.to_vec();
    }
    let mut tmp = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0f32;
            for dx in 0..=2 * r {
                s += v[y * w + (x + w + dx - r) % w];
            }
            tmp[y * w + x] = s / (2 * r + 1) as f32;
        }
    }
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        let y0 = y.saturating_sub(r);
        let y1 = (y + r).min(h - 1);
        for x in 0..w {
            let mut s = 0.0f32;
            for yy in y0..=y1 {
                s += tmp[yy * w + x];
            }
            out[y * w + x] = s / (y1 - y0 + 1) as f32;
        }
    }
    out
}
