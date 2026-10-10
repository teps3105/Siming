//! GDD 2.2 步驟 1.5：河流侵蝕（Braun & Willett 2013 的隱式 stream-power 解法；Cordonnier et al. 2016 的
//! 「隆起圖 + 侵蝕」大尺度地形）。
//!
//! 每次迭代（流向每 `WG_EROSION_ROUTE_EVERY` 次重算）：Priority-Flood（Barnes et al. 2014，`hydrology::fill_and_lakes`）求流向（receiver）與拓撲順序 →
//! 沿逆序累積面積加權（cos(lat)）的集水面積 A → 依順序（receiver 先於上游）隱式解
//! `h_i^{t+1} = (h_i^t + U_i·dt + f·h_r^{t+1}) / (1 + f)`，`f = K·A^m·dt / d(i, r)`（坡度指數 n = 1，無條件穩定、O(n)）。
//! 只在 receiver 較低時下切（窪地保留，最後成湖）；海格為基準面不變。全部用固定順序與基本浮點運算（GDD 6.5）。

use super::grid::NONE;
use super::hydrology::{fill_and_lakes, step_len};
use crate::params::*;

/// 對陸地做 `iters` 次隱式 stream-power 侵蝕。`uplift` 為每格隆起率（0..1 比例，海格忽略），`area` 為格面積權重。
pub fn erode(h: &mut [f32], uplift: &[f32], area: &[f32], w: usize, hh: usize, sea: f32, iters: u32) {
    let n = w * hh;
    let dt = WG_EROSION_DT;
    let mut acc = vec![0.0f32; n];
    let mut routing: Option<super::hydrology::Filled> = None;
    for it in 0..iters {
        // 流向與拓撲順序每 WG_EROSION_ROUTE_EVERY 次迭代重算一次（Priority-Flood 是主要成本）。
        if it % WG_EROSION_ROUTE_EVERY == 0 || routing.is_none() {
            let f = fill_and_lakes(h, w, hh, sea, u32::MAX);
            // 集水面積：沿拓撲逆序累加（海格不累加）。
            acc[..n].copy_from_slice(&area[..n]);
            for &c in f.order.iter().rev() {
                let c = c as usize;
                let r = f.flow[c];
                if r != NONE && h[r as usize] >= sea {
                    let a = acc[c];
                    acc[r as usize] += a;
                }
            }
            routing = Some(f);
        }
        let f = routing.as_ref().unwrap();
        // 隱式更新：receiver 先於上游。
        for &c in f.order.iter() {
            let c = c as usize;
            let r = f.flow[c];
            let raised = h[c] + uplift[c] * WG_EROSION_UPLIFT * dt;
            if r == NONE {
                h[c] = raised.min(1.0);
                continue;
            }
            let r = r as usize;
            let hr = h[r];
            if hr >= h[c] {
                h[c] = raised.min(1.0);
                continue;
            }
            let fac = WG_EROSION_K * acc[c].sqrt() * dt / step_len(w, hh, c, r);
            h[c] = ((raised + fac * hr) / (1.0 + fac)).clamp(0.0, 1.0);
        }
    }
}
