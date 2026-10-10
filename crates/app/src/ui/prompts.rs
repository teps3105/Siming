//! GDD 5.9、9.8 按鍵提示圖示（egui 向量繪製，不用點陣圖）、畫面右下角的情境提示、搖桿準星。
use bevy_egui::egui::{self, Align2, Color32, FontId, Id, LayerId, Order, Pos2, Rect, Stroke, StrokeKind, Vec2};

use super::deck::UiLayout;
use super::ink::{self, Ink};
use super::theme::*;
use super::UiText;
use crate::input::actions::Reticle;
use crate::state::InputMode;

/// 搖桿按鈕配色：Xbox 慣例（A 綠、B 紅、X 藍、Y 黃），降低彩度配合水墨風格（9.8）。
const PAD_A: Color32 = Color32::from_rgb(0x6F, 0x8A, 0x5E);
const PAD_B: Color32 = Color32::from_rgb(0xA8, 0x5A, 0x4E);
const PAD_X: Color32 = Color32::from_rgb(0x5B, 0x7B, 0x96);
const PAD_Y: Color32 = Color32::from_rgb(0xC0, 0xA0, 0x58);

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // LT／RT／搖桿圖示於日後的提示使用
pub enum Glyph {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Lt,
    Rt,
    DPad,
    LStick,
    RStick,
    Menu,
    View,
    /// 鍵盤鍵帽，內含按鍵名稱。
    Key(String),
}

impl Glyph {
    pub fn key(name: impl Into<String>) -> Self {
        Glyph::Key(name.into())
    }
}

/// 圖示高度（邏輯像素）。
fn glyph_h(layout: UiLayout) -> f32 {
    match layout {
        UiLayout::Deck => 26.0,
        UiLayout::Desktop => 22.0,
    }
}

fn label_font(layout: UiLayout) -> FontId {
    FontId::proportional(if layout == UiLayout::Deck { 16.0 } else { 14.0 })
}

/// 圖示寬度：圓鈕與搖桿為正方形；肩鍵、扳機與鍵帽依文字寬度。
fn glyph_width(ui: &egui::Ui, g: &Glyph, h: f32) -> f32 {
    match g {
        Glyph::A | Glyph::B | Glyph::X | Glyph::Y | Glyph::DPad | Glyph::LStick | Glyph::RStick | Glyph::Menu | Glyph::View => h,
        Glyph::Lb | Glyph::Rb | Glyph::Lt | Glyph::Rt => h * 1.7,
        Glyph::Key(name) => {
            let w = ui.painter().layout_no_wrap(name.clone(), FontId::proportional(h * 0.6), INK).size().x;
            (w + h * 0.7).max(h)
        }
    }
}

/// 在 `rect` 內繪製圖示。
pub fn paint_glyph(p: &egui::Painter, rect: Rect, g: &Glyph) {
    let c = rect.center();
    let h = rect.height();
    let r = h * 0.5;
    let ink = Stroke::new(1.0_f32, INK);
    let letter = |p: &egui::Painter, s: &str, col: Color32| {
        p.text(c, Align2::CENTER_CENTER, s, FontId::proportional(h * 0.62), col);
    };
    match g {
        Glyph::A | Glyph::B | Glyph::X | Glyph::Y => {
            let (col, s) = match g {
                Glyph::A => (PAD_A, "A"),
                Glyph::B => (PAD_B, "B"),
                Glyph::X => (PAD_X, "X"),
                _ => (PAD_Y, "Y"),
            };
            p.circle(c, r - 0.5, col, Stroke::new(1.0_f32, INK));
            letter(p, s, PAPER);
        }
        Glyph::Lb | Glyph::Rb | Glyph::Lt | Glyph::Rt => {
            let trigger = matches!(g, Glyph::Lt | Glyph::Rt);
            let s = match g {
                Glyph::Lb => "LB",
                Glyph::Rb => "RB",
                Glyph::Lt => "LT",
                _ => "RT",
            };
            let rr = if trigger { egui::CornerRadius::same((h * 0.45) as u8) } else { egui::CornerRadius::same((h * 0.25) as u8) };
            p.rect(rect.shrink(0.5), rr, if trigger { INK } else { INK_LIGHT }, ink, StrokeKind::Inside);
            letter(p, s, PAPER);
        }
        Glyph::DPad => {
            p.circle(c, r - 0.5, PAPER, Stroke::new(1.0_f32, INK_LIGHT));
            let arm = h * 0.18;
            let len = h * 0.36;
            p.rect_filled(Rect::from_center_size(c, Vec2::new(arm * 2.0, len * 2.0)), 1.0, INK);
            p.rect_filled(Rect::from_center_size(c, Vec2::new(len * 2.0, arm * 2.0)), 1.0, INK);
            p.circle_filled(c, arm * 0.6, PAPER);
        }
        Glyph::LStick | Glyph::RStick => {
            p.circle(c, r - 0.5, PAPER_DARK, Stroke::new(1.0_f32, INK_LIGHT));
            p.circle(c, r * 0.62, INK_LIGHT, ink);
            p.text(c, Align2::CENTER_CENTER, if *g == Glyph::LStick { "L" } else { "R" }, FontId::proportional(h * 0.5), PAPER);
        }
        Glyph::Menu => {
            p.circle(c, r - 0.5, INK_LIGHT, ink);
            for dy in [-0.2_f32, 0.0, 0.2] {
                let y = c.y + h * dy;
                p.line_segment([Pos2::new(c.x - h * 0.22, y), Pos2::new(c.x + h * 0.22, y)], Stroke::new(1.6_f32, PAPER));
            }
        }
        Glyph::View => {
            p.circle(c, r - 0.5, INK_LIGHT, ink);
            let s = Stroke::new(1.4_f32, PAPER);
            let a = Rect::from_center_size(c + Vec2::new(h * 0.06, -h * 0.06), Vec2::splat(h * 0.3));
            let b = Rect::from_center_size(c + Vec2::new(-h * 0.06, h * 0.06), Vec2::splat(h * 0.3));
            p.rect_stroke(a, 0.0, s, StrokeKind::Middle);
            p.rect_filled(b, 0.0, INK_LIGHT);
            p.rect_stroke(b, 0.0, s, StrokeKind::Middle);
        }
        Glyph::Key(name) => {
            // 鍵帽：紙色面 + 較深的底邊
            let top = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.max.y - 2.0));
            p.rect_filled(rect.translate(Vec2::new(0.0, 0.0)), 3.0, INK_LIGHT);
            p.rect(top, 3.0, PAPER, Stroke::new(1.0_f32, INK_LIGHT), StrokeKind::Inside);
            p.text(top.center(), Align2::CENTER_CENTER, name, FontId::proportional(h * 0.6), INK);
        }
    }
}

/// 以 widget 形式放入圖示。
pub fn glyph(ui: &mut egui::Ui, g: &Glyph, layout: UiLayout) -> egui::Response {
    let h = glyph_h(layout);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(glyph_width(ui, g, h), h), egui::Sense::hover());
    paint_glyph(ui.painter(), rect, g);
    resp
}

/// 一組圖示 + 說明文字（例如 「A 選取」「LB RB 地圖模式」）。
pub fn prompt(ui: &mut egui::Ui, glyphs: &[Glyph], label: &str, layout: UiLayout) {
    ui.spacing_mut().item_spacing.x = 3.0;
    for g in glyphs {
        glyph(ui, g, layout);
    }
    ui.add_space(3.0);
    ui.label(egui::RichText::new(label).font(label_font(layout)).color(INK));
    ui.spacing_mut().item_spacing.x = 6.0;
}

/// 提示的情境（5.7）：地圖、卡片開啟、原因簽「止」、命牌列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptCtx {
    Map,
    Card,
    Notice,
    Fates,
}

/// 目前情境下的提示：`(圖示, 文字鍵)`，最多 4 組（5.9：常駐 2–4 個）。
pub fn context_prompts(mode: InputMode, ctx: PromptCtx) -> Vec<(Vec<Glyph>, &'static str)> {
    match (mode, ctx) {
        (InputMode::Gamepad, PromptCtx::Card) => vec![(vec![Glyph::A], "prompt_confirm"), (vec![Glyph::B], "prompt_back"), (vec![Glyph::X], "prompt_context"), (vec![Glyph::Y], "prompt_follow")],
        (InputMode::Gamepad, PromptCtx::Notice) => vec![(vec![Glyph::A], "prompt_fly"), (vec![Glyph::Menu], "prompt_resume"), (vec![Glyph::DPad], "prompt_rumor")],
        (InputMode::Gamepad, PromptCtx::Fates) => vec![(vec![Glyph::A], "prompt_fly"), (vec![Glyph::Y], "prompt_unfollow"), (vec![Glyph::B], "prompt_back")],
        (InputMode::Gamepad, PromptCtx::Map) => {
            vec![(vec![Glyph::A], "prompt_select"), (vec![Glyph::DPad], "prompt_rumor"), (vec![Glyph::Lb, Glyph::Rb], "prompt_mode"), (vec![Glyph::Menu], "prompt_pause")]
        }
        (InputMode::KeyboardMouse, PromptCtx::Card) => vec![(vec![Glyph::key("F")], "prompt_follow"), (vec![Glyph::key("Esc")], "prompt_back")],
        (InputMode::KeyboardMouse, PromptCtx::Notice) => vec![(vec![Glyph::key("Enter")], "prompt_fly"), (vec![Glyph::key("Space")], "prompt_resume")],
        (InputMode::KeyboardMouse, PromptCtx::Fates) => vec![],
        (InputMode::KeyboardMouse, PromptCtx::Map) => {
            vec![(vec![Glyph::key("Space")], "prompt_pause"), (vec![Glyph::key("F1–F4")], "prompt_mode"), (vec![Glyph::key("Esc")], "prompt_back")]
        }
        (InputMode::Touch, _) => vec![],
    }
}

/// 畫面右下角的常駐按鍵提示（紙簽形式）。桌機版面只在搖桿模式顯示；Deck 版面一律顯示（5.9）。
/// `avail` 為可用區域（Deck 抽屜開啟時已扣除）；`bottom_inset` 為印匣高度，提示浮在其上。
#[allow(clippy::too_many_arguments)]
pub fn prompt_bar(ctx: &egui::Context, ink: &Ink, text: &UiText, mode: InputMode, layout: UiLayout, pctx: PromptCtx, avail: Rect, bottom_inset: f32) {
    if layout == UiLayout::Desktop && mode != InputMode::Gamepad {
        return;
    }
    let prompts = context_prompts(mode, pctx);
    if prompts.is_empty() {
        return;
    }
    egui::Area::new(Id::new("prompt_bar"))
        .order(Order::Foreground)
        .pivot(Align2::RIGHT_BOTTOM)
        .fixed_pos(avail.right_bottom() - Vec2::new(12.0, bottom_inset))
        .interactable(false)
        .show(ctx, |ui| {
            let bg = ui.painter().add(egui::Shape::Noop);
            let pad = Vec2::new(ink.px(12.0), ink.px(6.0));
            let inner = ui
                .scope_builder(egui::UiBuilder::new().max_rect(ui.max_rect().translate(pad)), |ui| {
                    ui.horizontal(|ui| {
                        for (i, (glyphs, key)) in prompts.iter().enumerate() {
                            if i > 0 {
                                ui.add_space(10.0);
                            }
                            prompt(ui, glyphs, text.t(key), layout);
                        }
                    });
                })
                .response
                .rect;
            let rect = Rect::from_min_max(inner.min - pad, inner.max + pad);
            ui.painter().set(bg, egui::Shape::Vec(ink::slip_shapes(rect, ink, 0.96)));
            ui.allocate_rect(rect, egui::Sense::hover());
        });
}

/// 搖桿準星（5.9）：朱紅小圈，吸附時收緊並加中心點。
pub fn draw_reticle(ctx: &egui::Context, reticle: &Reticle) {
    let Some(pos) = reticle.pos else { return };
    let p = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("reticle")));
    let pos = Pos2::new(pos.x, pos.y);
    let r = if reticle.snapped { 9.0 } else { 14.0 };
    // 紙色外描邊讓朱紅在深色地形上仍可辨
    p.circle_stroke(pos, r, Stroke::new(4.5_f32, PAPER.gamma_multiply(0.85)));
    p.circle_stroke(pos, r, Stroke::new(2.5_f32, SEAL_RED));
    let tick = |d: Vec2| {
        let a = pos + d * (r + 2.0);
        let b = pos + d * (r + 8.0);
        p.line_segment([a, b], Stroke::new(2.0_f32, SEAL_RED));
    };
    for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
        tick(d);
    }
    if reticle.snapped {
        p.circle_filled(pos, 2.5, SEAL_RED);
    }
}
