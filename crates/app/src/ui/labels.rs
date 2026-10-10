//! GDD 5.3 文字標籤：螢幕空間以 egui 繪製，貪婪避讓。
use bevy::prelude::*;
use bevy_egui::egui::{self, FontFamily, FontId, Id, LayerId, Order, Rect};

use super::theme::{INK, PAPER};
use crate::bridge::SimRes;
use crate::state::{cell_center, nearest_copy_x, terrain_height, MapMode, MapView};

/// 把格座標投影到螢幕（取離鏡頭焦點最近的東西副本；落在 `visible` 內者）。供溯因視圖、情境印環、朱環使用。
pub fn cell_to_screen(world: &siming_sim::World, view: &MapView, camera: &Camera, cam_tf: &GlobalTransform, cell: u32, visible: Rect) -> Option<egui::Pos2> {
    let geo = &world.geo;
    if view.globe > 0.02 {
        return None;
    }
    let w = geo.w as f32;
    let c = cell_center(geo, cell);
    let h = terrain_height(geo, cell, view.height_scale);
    let base_x = if geo.wrap_x { nearest_copy_x(c.x, view.focus.x, w) } else { c.x };
    let copies: &[f32] = if geo.wrap_x { &[0.0, -1.0, 1.0] } else { &[0.0] };
    for k in copies {
        let pos = Vec3::new(base_x + k * w, h, c.y);
        let Ok(vp) = camera.world_to_viewport(cam_tf, pos) else { continue };
        let p = egui::pos2(vp.x, vp.y);
        if visible.contains(p) {
            return Some(p);
        }
    }
    None
}

struct Label<'a> {
    text: &'a str,
    cell: u32,
    importance: f32,
    size: f32,
}

pub fn draw_labels(
    ctx: &egui::Context,
    sim: &SimRes,
    view: &MapView,
    mode: MapMode,
    camera: Option<(&Camera, &GlobalTransform)>,
    avoid: &[Rect],
) {
    let Some((camera, cam_tf)) = camera else { return };
    let world = sim.0.world();
    let geo = &world.geo;
    // 星球視角（含過渡中）不畫標籤
    if view.globe > 0.02 {
        return;
    }
    let z = view.camera_z;
    let w = geo.w as f32;
    let mut labels: Vec<Label> = Vec::new();

    // 州郡名：z < 地圖寬 × 0.3。
    if z < w * 0.3 {
        for p in &world.provinces {
            let imp = 1000.0 + p.city_level as f32 * 100.0 + (p.population as f32 / 1.0e4).min(99.0);
            labels.push(Label { text: &p.name, cell: p.seat, importance: imp, size: 17.0 });
        }
    }
    // 江河、山脈、洞天：中景（z < 地圖寬 × 0.6）。
    if z < w * 0.6 {
        if mode == MapMode::Shanhe {
            for r in &geo.rivers {
                if let Some(&c) = r.cells.get(r.cells.len() / 2) {
                    labels.push(Label { text: &r.name, cell: c, importance: 200.0 + r.level as f32 * 50.0, size: 14.0 });
                }
            }
            for r in &geo.ranges {
                labels.push(Label { text: &r.name, cell: r.peak, importance: 500.0, size: 15.0 });
            }
        }
        if matches!(mode, MapMode::Shanhe | MapMode::Lingmai) {
            for n in &geo.nodes {
                labels.push(Label { text: &n.name, cell: n.cell, importance: 300.0 + n.strength * 100.0, size: 14.0 });
            }
        }
    }
    labels.sort_by(|a, b| b.importance.total_cmp(&a.importance));
    let map_h = geo.h as f32;

    let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("map_labels")));
    // 可用區域（Deck 抽屜開啟時已扣除）再避開上下各 44 px 與呼叫者給的器物矩形（5.7）。
    let screen = ctx.available_rect();
    let visible = Rect::from_min_max(screen.min + egui::vec2(0.0, 44.0), screen.max - egui::vec2(0.0, 44.0));
    let mut placed: Vec<Rect> = Vec::new();
    const OUTLINE: [(f32, f32); 8] =
        [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];

    for l in labels {
        let c = cell_center(geo, l.cell);
        // 與著色器的墨線同一條極區淡出（62°–78°）：地圖在那裡已漸變為極冰，標籤所指的河、洞天看不到了。
        let sin_lat = (std::f32::consts::PI * c.y / map_h).cos().abs();
        let alpha = 1.0 - ((sin_lat - 0.883) / (0.978 - 0.883)).clamp(0.0, 1.0);
        let alpha = alpha * alpha * (3.0 - 2.0 * alpha);
        if alpha < 0.08 {
            continue;
        }
        let h = terrain_height(geo, l.cell, view.height_scale);
        // GDD 5.3：東西無縫——以離鏡頭焦點最近的副本位置繪製；視野較寬時相鄰副本也可能入鏡，
        // 取最近副本及其左右各一份，第一個落在畫面內的為準。
        let base_x = if geo.wrap_x { nearest_copy_x(c.x, view.focus.x, w) } else { c.x };
        let copies: &[f32] = if geo.wrap_x { &[0.0, -1.0, 1.0] } else { &[0.0] };
        let mut found = None;
        for k in copies {
            let pos = Vec3::new(base_x + k * w, h, c.y);
            let Ok(vp) = camera.world_to_viewport(cam_tf, pos) else { continue };
            let center = egui::pos2(vp.x, vp.y);
            if visible.contains(center) {
                found = Some(center);
                break;
            }
        }
        let Some(center) = found else { continue };
        let galley = ctx.fonts(|f| f.layout_no_wrap(l.text.to_owned(), FontId::new(l.size, FontFamily::Proportional), INK));
        let rect = Rect::from_center_size(center, galley.size());
        if !visible.contains_rect(rect) || avoid.iter().any(|r| r.intersects(rect)) || placed.iter().any(|r| r.expand(2.0).intersects(rect)) {
            continue;
        }
        placed.push(rect);
        for (dx, dy) in OUTLINE {
            painter.galley_with_override_text_color(rect.min + egui::vec2(dx, dy), galley.clone(), PAPER.gamma_multiply(alpha));
        }
        painter.galley_with_override_text_color(rect.min, galley, INK.gamma_multiply(alpha));
    }
}
