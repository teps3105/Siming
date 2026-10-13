//! GDD 6.9 運算核心：州域場的傳輸運算子。
//!
//! 全部是「快照 → 計算 → 回傳新向量」的純函數：輸入向量不被改動，結果與州郡的處理順序無關
//! （加總順序固定為 ID 升冪）。向量索引即 `ProvinceId`；寫回與截斷由 [`super::scatter`] 負責。

use crate::types::ProvinceId;

/// 衰減 `v'_i = v_i × (1 − r)`（不守恆）。
pub fn decay(v: &[f32], r: f32) -> Vec<f32> {
    v.iter().map(|x| x * (1.0 - r)).collect()
}

/// 擴散：每州把 `k` 倍的「與較低陸鄰的差值」依陸鄰數均分流出（守恆；`k ≤ 1` 時結果恆非負）。
///
/// `out_i = Σ_{j∈N(i)} k·max(0, v_i − v_j) / deg(i)`；`v'_i = v_i − out_i + Σ_{j: i∈N(j)} k·max(0, v_j − v_i) / deg(j)`。
/// `neighbors[i]` 為州 `i` 的陸鄰，依 ID 升冪。
pub fn diffuse<N: AsRef<[ProvinceId]>>(v: &[f32], neighbors: &[N], k: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; v.len()];
    let mut inflow = vec![0.0f32; v.len()];
    for (j, nb) in neighbors.iter().enumerate() {
        let nb = nb.as_ref();
        if nb.is_empty() {
            continue;
        }
        let deg = nb.len() as f32;
        for &i in nb {
            let i = i as usize;
            let d = k * (v[j] - v[i]).max(0.0) / deg;
            out[j] += d;
            inflow[i] += d;
        }
    }
    v.iter().enumerate().map(|(i, x)| x - out[i] + inflow[i]).collect()
}

/// 順流：每州把 `k` 倍的值移往下游州（守恆）；下游為無（入海、入湖、仍屬本州）者不移動。
///
/// `m_i = k·v_i`；`v'_i = v_i − m_i + Σ_{j: downstream(j)=i} m_j`。
pub fn drift(v: &[f32], downstream: &[Option<ProvinceId>], k: f32) -> Vec<f32> {
    let mut moved = vec![0.0f32; v.len()];
    let mut inflow = vec![0.0f32; v.len()];
    for (i, d) in downstream.iter().enumerate() {
        let Some(d) = *d else { continue };
        let d = d as usize;
        if d == i || d >= v.len() {
            continue;
        }
        let m = k * v[i];
        moved[i] = m;
        inflow[d] += m;
    }
    v.iter().enumerate().map(|(i, x)| x - moved[i] + inflow[i]).collect()
}

/// 靠攏 `v'_i = v_i + r × (target_i − v_i)`（不守恆）。
pub fn relax(v: &[f32], target: &[f32], r: f32) -> Vec<f32> {
    v.iter().zip(target).map(|(x, t)| x + r * (t - x)).collect()
}

/// 截斷：依 `FieldSpec.range` 的 `(下限, 上限)`。
pub fn clamp(v: &[f32], range: (f32, f32)) -> Vec<f32> {
    v.iter().map(|x| x.clamp(range.0, range.1)).collect()
}
