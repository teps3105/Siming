//! GDD 5.3 地形網格：單一網格，格點高度以 2×2 雙線性超取樣。
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use siming_sim::Geography;

use crate::state::terrain_height;

/// 超取樣倍率（GDD 5.3：2×2）。
pub const MESH_SS: usize = 2;

/// 格點高度場的 CPU 副本（高度縮放 = 1），供選取與靈脈線取高度。
#[derive(Resource)]
pub struct TerrainCpu {
    pub w: usize,
    pub h: usize,
    /// `terrain_height(geo, cell, 1.0)`，水面為 0。
    pub heights: Vec<f32>,
    /// 陸地最高點（高度縮放 = 1）。
    pub max_height: f32,
    #[allow(dead_code)]
    pub province_of: Vec<u16>,
    /// 東西環繞（GDD 2.2）：x 方向取樣以週期 `w` 循環。
    pub wrap_x: bool,
}

impl TerrainCpu {
    pub fn new(geo: &Geography) -> Self {
        let n = geo.len();
        let heights: Vec<f32> = (0..n as u32).map(|c| terrain_height(geo, c, 1.0)).collect();
        let max_height = heights.iter().cloned().fold(0.0, f32::max).max(0.001);
        Self { w: geo.w as usize, h: geo.h as usize, heights, max_height, province_of: geo.province_of.clone(), wrap_x: geo.wrap_x }
    }

    /// 世界座標 (x, z) 處的高度（縮放 = 1），格中心在 (x+0.5, z+0.5)，雙線性。
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let gx = if self.wrap_x { (x - 0.5).rem_euclid(self.w as f32) } else { (x - 0.5).clamp(0.0, (self.w - 1) as f32) };
        let gz = (z - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, z0) = ((gx.floor() as usize).min(self.w - 1), gz.floor() as usize);
        let x1 = if self.wrap_x { (x0 + 1) % self.w } else { (x0 + 1).min(self.w - 1) };
        let z1 = (z0 + 1).min(self.h - 1);
        let (tx, tz) = (gx - x0 as f32, gz - z0 as f32);
        let g = |x: usize, z: usize| self.heights[z * self.w + x];
        let a = g(x0, z0) * (1.0 - tx) + g(x1, z0) * tx;
        let b = g(x0, z1) * (1.0 - tx) + g(x1, z1) * tx;
        a * (1.0 - tz) + b * tz
    }
}

/// 頂點：位置（y = 縮放 1 的高度）、法線（縮放 1）、UV（整張地圖 0..1）。
/// 地形代碼與河流等級改由 `terrain` 貼圖逐像素提供（見 textures.rs），以得到平滑海岸與細河。
pub fn build_terrain_mesh(cpu: &TerrainCpu) -> Mesh {
    let (vw, vh) = (cpu.w * MESH_SS, cpu.h * MESH_SS);
    let (mw, mh) = (cpu.w as f32, cpu.h as f32);
    let mut pos = Vec::with_capacity(vw * vh);
    let mut uv = Vec::with_capacity(vw * vh);
    let mut curv = Vec::with_capacity(vw * vh);
    let sx = mw / (vw - 1) as f32;
    let sz = mh / (vh - 1) as f32;
    for j in 0..vh {
        for i in 0..vw {
            let (x, z) = (i as f32 * sx, j as f32 * sz);
            let y = cpu.height_at(x, z);
            // 曲率（高度場 Hessian 的 Frobenius 範數，步長 1 格）：山脊線用。逐頂點算好再內插，
            // 比在著色器對內插梯度取 fwidth 平滑（後者在每個三角形內為常數，會呈方塊狀）。
            let e = 1.0;
            let hxx = cpu.height_at(x + e, z) - 2.0 * y + cpu.height_at(x - e, z);
            let hzz = cpu.height_at(x, z + e) - 2.0 * y + cpu.height_at(x, z - e);
            let hxz = (cpu.height_at(x + e, z + e) - cpu.height_at(x + e, z - e) - cpu.height_at(x - e, z + e) + cpu.height_at(x - e, z - e)) / 4.0;
            curv.push([(hxx * hxx + hzz * hzz + 2.0 * hxz * hxz).sqrt(), 0.0]);
            pos.push([x, y, z]);
            uv.push([x / mw, z / mh]);
        }
    }
    let mut normals = Vec::with_capacity(vw * vh);
    for j in 0..vh {
        for i in 0..vw {
            // 法線以約 ±0.75 格的寬模板求中央差（直接取高度場，東西環繞由 `height_at` 處理）：
            // 比相鄰頂點差分平滑，避免格點階梯狀的山影與山脊線。
            let (x, z) = (i as f32 * sx, j as f32 * sz);
            let e = 0.75;
            let dx = (cpu.height_at(x + e, z) - cpu.height_at(x - e, z)) / (2.0 * e);
            let dz = (cpu.height_at(x, z + e) - cpu.height_at(x, z - e)) / (2.0 * e);
            normals.push(Vec3::new(-dx, 1.0, -dz).normalize().to_array());
        }
    }
    // 南北緣外的「極地邊疆」帶：高度 0、UV 夾在 0／1（星球上收成極點）、著色器畫極冰並淡出為紙色。
    let pad = mw * crate::state::FRONTIER_FRAC * 1.6;
    let n0 = (vw * vh) as u32;
    for (zz, v) in [(-pad, 0.0f32), (mh + pad, 1.0)] {
        for i in 0..vw {
            let x = i as f32 * sx;
            pos.push([x, 0.0, zz]);
            uv.push([x / mw, v]);
            normals.push([0.0, 1.0, 0.0]);
            curv.push([0.0, 0.0]);
        }
    }
    let mut idx: Vec<u32> = Vec::with_capacity((vw - 1) * (vh - 1) * 6 + (vw - 1) * 12);
    for j in 0..vh - 1 {
        for i in 0..vw - 1 {
            let a = (j * vw + i) as u32;
            let (b, c, d) = (a + 1, a + vw as u32, a + vw as u32 + 1);
            // 由上方（-y）看為逆時針 → 法線朝 +y
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    for i in 0..vw as u32 - 1 {
        // 北帶：外緣列 → 地圖首列；南帶：地圖末列 → 外緣列（與主網格同繞向）
        let (no, nm) = (n0 + i, i);
        idx.extend_from_slice(&[no, nm, no + 1, no + 1, nm, nm + 1]);
        let (sm, so) = (((vh - 1) * vw) as u32 + i, n0 + vw as u32 + i);
        idx.extend_from_slice(&[sm, so, sm + 1, sm + 1, so, so + 1]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, curv)
        .with_inserted_indices(Indices::U32(idx))
}
