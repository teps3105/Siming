//! GDD 5.1 色票、5.7 主題（字型、字級、基本色；可見元件由 `ink.rs` 繪製）、9.2 字型。
use bevy_egui::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use siming_sim::Terrain;

use crate::render::palette;

/// 把色票的 sRGB 十六進位轉為 egui 顏色。
const fn hex(h: u32) -> Color32 {
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

pub const PAPER: Color32 = hex(palette::PAPER);
pub const PAPER_DARK: Color32 = hex(palette::PAPER_DARK);
pub const INK: Color32 = hex(palette::INK);
pub const INK_LIGHT: Color32 = hex(palette::INK_LIGHT);
pub const SEAL_RED: Color32 = hex(palette::SEAL_RED);
pub const GOLD: Color32 = hex(palette::GOLD);
pub const CYAN: Color32 = hex(palette::CYAN);

/// 墨分五色的五階量化位準（8.1 `FIELD_INK_LEVELS`；5.4）。
pub const FIELD_INK_LEVELS: [f32; 5] = [0.1, 0.3, 0.5, 0.7, 0.9];
/// 五階名稱，由清到焦。
pub const FIELD_INK_NAMES: [&str; 5] = ["清", "淡", "重", "濃", "焦"];

/// 山河模式的地形色：與地形著色器同一來源（`render::palette::terrain_colors`，線性 → sRGB），
/// 圖例因此與地圖所見一致。
pub fn terrain_color(t: Terrain) -> Color32 {
    let v = palette::terrain_colors()[t as usize];
    let c = bevy::prelude::Color::linear_rgb(v.x, v.y, v.z).to_srgba();
    Color32::from_rgb((c.red * 255.0).round() as u8, (c.green * 255.0).round() as u8, (c.blue * 255.0).round() as u8)
}

/// 靈脈模式色階（GDD 5.4）：0 → paper，0.5 → cyan，1 → gold。
pub fn aura_ramp(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let (a, b, u) = if t < 0.5 { (PAPER, CYAN, t * 2.0) } else { (CYAN, GOLD, (t - 0.5) * 2.0) };
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * u).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// 載入字型並套用主題。
pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "wenkai".into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/wenkai_tc_subset.ttf"))),
    );
    for fam in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(fam).or_default().insert(0, "wenkai".into());
    }
    ctx.set_fonts(fonts);

    ctx.style_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
        ]
        .into();
        let r = CornerRadius::same(2);
        let border = Stroke::new(1.0_f32, INK_LIGHT);
        let v = &mut style.visuals;
        *v = egui::Visuals::light();
        v.override_text_color = Some(INK);
        v.panel_fill = PAPER_DARK;
        v.window_fill = PAPER_DARK;
        v.extreme_bg_color = PAPER;
        v.faint_bg_color = PAPER;
        v.window_stroke = border;
        v.window_corner_radius = r;
        v.menu_corner_radius = r;
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
        v.selection.bg_fill = SEAL_RED;
        v.selection.stroke = Stroke::new(1.0_f32, PAPER);
        for (w, fill) in [
            (&mut v.widgets.noninteractive, PAPER_DARK),
            (&mut v.widgets.inactive, PAPER),
            (&mut v.widgets.hovered, hex(palette::SNOW)),
            (&mut v.widgets.active, SEAL_RED),
            (&mut v.widgets.open, PAPER),
        ] {
            w.corner_radius = r;
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = border;
            w.fg_stroke = Stroke::new(1.0_f32, INK);
        }
        v.widgets.active.fg_stroke = Stroke::new(1.0_f32, PAPER);
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        // 開發建置預設會在未對齊像素的元件旁印出「Unaligned」，介面元件多為自繪形狀，關閉之
        #[cfg(debug_assertions)]
        {
            style.debug.show_unaligned = false;
        }
    });
}
