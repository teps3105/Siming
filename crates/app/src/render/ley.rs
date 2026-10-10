//! GDD 5.4 靈脈模式的額外圖層：靈脈線（線帶網格 + 流動光點）與洞天標記（圓盤）。
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use siming_sim::Geography;

use super::mesh::TerrainCpu;

fn chaikin(pts: &[Vec2], rounds: usize) -> Vec<Vec2> {
    let mut cur = pts.to_vec();
    for _ in 0..rounds {
        if cur.len() < 3 {
            break;
        }
        let mut next = vec![cur[0]];
        for w in cur.windows(2) {
            next.push(w[0] * 0.75 + w[1] * 0.25);
            next.push(w[0] * 0.25 + w[1] * 0.75);
        }
        next.push(*cur.last().unwrap());
        cur = next;
    }
    cur
}

fn push_tri(idx: &mut Vec<u32>, pos: &[[f32; 3]], a: u32, b: u32, c: u32) {
    let (pa, pb, pc) = (pos[a as usize], pos[b as usize], pos[c as usize]);
    let ny = (pb[2] - pa[2]) * (pc[0] - pa[0]) - (pb[0] - pa[0]) * (pc[2] - pa[2]);
    if ny >= 0.0 {
        idx.extend_from_slice(&[a, b, c]);
    } else {
        idx.extend_from_slice(&[a, c, b]);
    }
}

/// 回傳 (靈脈線帶網格, 洞天圓盤網格)。高度為縮放 = 1 的地形高度，著色器乘上高度縮放。
pub fn build_ley_meshes(geo: &Geography, cpu: &TerrainCpu) -> (Mesh, Mesh) {
    // 線帶
    let mut pos: Vec<[f32; 3]> = vec![];
    let mut nrm: Vec<[f32; 3]> = vec![];
    let mut uv: Vec<[f32; 2]> = vec![];
    let mut uvb: Vec<[f32; 2]> = vec![];
    let mut idx: Vec<u32> = vec![];
    for e in &geo.ley {
        let mut cells: Vec<Vec2> = e
            .path
            .iter()
            .map(|&c| {
                let (x, y) = geo.xy(c);
                Vec2::new(x as f32 + 0.5, y as f32 + 0.5)
            })
            .collect();
        // 東西環繞：沿路徑展開 x（相鄰點相差不超過半個地圖寬），線帶可跨過接縫；
        // 各副本各自繪出整條線，接縫兩側因此都看得到。
        if geo.wrap_x {
            let w = geo.w as f32;
            for i in 1..cells.len() {
                cells[i].x = crate::state::nearest_copy_x(cells[i].x, cells[i - 1].x, w);
            }
        }
        if cells.len() < 2 {
            continue;
        }
        let pts = chaikin(&cells, 2);
        let half = (0.6 + 0.8 * e.strength.clamp(0.0, 1.0)) * 0.5;
        let base = pos.len() as u32;
        let mut along = 0.0f32;
        for i in 0..pts.len() {
            let prev = pts[i.saturating_sub(1)];
            let next = pts[(i + 1).min(pts.len() - 1)];
            let dir = (next - prev).normalize_or_zero();
            let side = Vec2::new(-dir.y, dir.x);
            if i > 0 {
                along += (pts[i] - pts[i - 1]).length();
            }
            let y = cpu.height_at(pts[i].x, pts[i].y);
            for s in [-1.0f32, 1.0] {
                // 位置含名目寬度 1 的側向位移（僅供三角形朝向判斷；實際位移在頂點著色器）
                pos.push([pts[i].x + side.x * s * half, y, pts[i].y + side.y * s * half]);
                nrm.push([side.x, 0.0, side.y]);
                uv.push([0.0, 0.0]);
                uvb.push([s, along]);
            }
        }
        // 著色器以 position + normal * uv.y * width 位移，故中心線位置需還原
        for i in 0..pts.len() {
            let y = pos[(base as usize) + i * 2][1];
            for s in 0..2 {
                let k = base as usize + i * 2 + s;
                pos[k] = [pts[i].x, y, pts[i].y];
                uv[k] = [0.0, if s == 0 { -half } else { half }];
            }
        }
        // 三角形朝向以名目位置判斷
        let nominal: Vec<[f32; 3]> = (0..pts.len() * 2)
            .map(|k| {
                let i = k / 2;
                let s = if k % 2 == 0 { -1.0 } else { 1.0 };
                let n = nrm[base as usize + k];
                [pts[i].x + n[0] * s * half, 0.0, pts[i].y + n[2] * s * half]
            })
            .collect();
        for i in 0..pts.len() as u32 - 1 {
            let (a, b, c, d) = (i * 2, i * 2 + 1, i * 2 + 2, i * 2 + 3);
            let mut tri = vec![];
            push_tri(&mut tri, &nominal, a, b, c);
            push_tri(&mut tri, &nominal, b, d, c);
            idx.extend(tri.iter().map(|t| t + base));
        }
    }
    let line = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, uvb)
        .with_inserted_indices(Indices::U32(idx));

    // 洞天圓盤
    const SEG: u32 = 24;
    let mut pos: Vec<[f32; 3]> = vec![];
    let mut nrm: Vec<[f32; 3]> = vec![];
    let mut uv: Vec<[f32; 2]> = vec![];
    let mut uvb: Vec<[f32; 2]> = vec![];
    let mut idx: Vec<u32> = vec![];
    for n in &geo.nodes {
        let (x, y) = geo.xy(n.cell);
        let (cx, cz) = (x as f32 + 0.5, y as f32 + 0.5);
        let h = cpu.height_at(cx, cz);
        let radius = 0.9 + 1.4 * n.strength.clamp(0.0, 1.0);
        let base = pos.len() as u32;
        pos.push([cx, h, cz]);
        nrm.push([0.0, 0.0, 0.0]);
        uv.push([0.0, 0.0]);
        uvb.push([0.0, 0.0]);
        for k in 0..SEG {
            let a = k as f32 / SEG as f32 * std::f32::consts::TAU;
            pos.push([cx, h, cz]);
            nrm.push([a.cos(), 0.0, a.sin()]);
            uv.push([0.0, radius]);
            uvb.push([1.0, 0.0]);
        }
        for k in 0..SEG {
            let (b, c) = (base + 1 + k, base + 1 + (k + 1) % SEG);
            // 由上方看逆時針
            idx.extend_from_slice(&[base, c, b]);
        }
    }
    let disc = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, uvb)
        .with_inserted_indices(Indices::U32(idx));
    (line, disc)
}
