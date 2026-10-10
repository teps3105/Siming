//! GDD 5.7 命牌（7.9 #36）：左緣直列的小牌，每張一個關注的魂（`World.follows`），跨世追蹤：
//! 今生姓名、一行狀態、提醒點（朱＝最近一年有事、金＝預兆中）；點擊飛往並開命簿；停留浮出飛／簿／因／解。
use bevy_egui::egui::{self, Align2, Id, Order};
use siming_sim::{SoulId, SoulState, World};

use super::deck::UiLayout;
use super::ink::{self, Ink, SealState};
use super::stream::EventFeed;
use super::text;
use super::theme::*;
use super::{UiActions, UiText};
use crate::state::InputMode;

/// 命牌列的游標（搖桿十字鍵 △ 進入後上下選）。
#[derive(Default)]
pub struct FatesNav {
    pub active: bool,
    pub cursor: usize,
}

/// 魂目前所在格（在世者取肉身所在；否則最後所在）。
pub fn soul_cell(w: &World, sid: SoulId) -> Option<u32> {
    let s = w.souls.get(sid as usize)?;
    match s.state {
        SoulState::Incarnate(pid) => w.persons.get(pid as usize).map(|p| p.cell),
        _ => Some(s.last_place.1),
    }
}

/// 魂是否有預兆中的未了線（`omen_at == lives`）。
pub fn omen_pending(w: &World, sid: SoulId) -> bool {
    w.souls.get(sid as usize).is_some_and(|s| {
        s.threads.iter().filter_map(|&t| w.threads.get(t as usize)).any(|t| siming_sim::thread::is_open(t) && t.omen_at.is_some() && t.omen_at == Some(siming_sim::thread::lives(w, t)))
    })
}

/// 左緣命牌；回傳佔用矩形。
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ctx: &egui::Context,
    ink: &Ink,
    text: &UiText,
    world: &World,
    feed: &EventFeed,
    nav: &mut FatesNav,
    layout: UiLayout,
    input: InputMode,
    top: f32,
    out: &mut UiActions,
) -> egui::Rect {
    let deck = layout == UiLayout::Deck;
    let width = ink.px(if deck { 174.0 } else { 176.0 });
    let body = ink.px(14.0).max(if deck { 16.0 } else { 0.0 });
    let small = ink.px(11.0).max(if deck { 16.0 } else { 0.0 });
    let follows: Vec<SoulId> = world.follows.iter().copied().collect();
    if follows.is_empty() {
        nav.active = false;
        return egui::Rect::NOTHING;
    }
    nav.cursor = nav.cursor.min(follows.len() - 1);
    egui::Area::new(Id::new("fates_tray"))
        .order(Order::Middle)
        .fixed_pos(egui::pos2(12.0, top))
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(ink.px(6.0), ink.px(5.0));
            ui.label(egui::RichText::new(format!("{} {}／12", text.t("fates_title"), follows.len())).size(small).color(INK_LIGHT));
            for (i, &sid) in follows.iter().enumerate() {
                let alive = matches!(world.souls.get(sid as usize).map(|s| &s.state), Some(SoulState::Incarnate(_)));
                let name = text::soul_name(world, sid);
                let status = text::soul_status(world, sid);
                let alert = super::stream::recent_alert(feed, world, sid);
                let omen = omen_pending(world, sid);
                let focused_nav = nav.active && input == InputMode::Gamepad && nav.cursor == i;
                let h = ink.px(44.0);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::click());
                let hovered = resp.hovered() || ink::focused(ctx, &resp) || focused_nav;
                let p = ui.painter();
                // 牌身：左端切角的紙簽
                let a = if alive { 1.0 } else { 0.8 };
                let mut shapes = ink::slip_shapes(rect, ink, a);
                if !alive {
                    shapes.push(egui::Shape::rect_filled(rect.shrink(1.0), 2.0, PAPER_DARK.gamma_multiply(0.45)));
                }
                if hovered {
                    shapes.push(egui::Shape::rect_filled(rect.shrink(1.0), 2.0, egui::Color32::from_rgba_unmultiplied(0xF7, 0xEE, 0xDD, 160)));
                }
                p.add(egui::Shape::Vec(shapes));
                let notch = [rect.left_top(), egui::pos2(rect.left() + ink.px(7.0), rect.center().y), rect.left_bottom()];
                p.add(egui::Shape::convex_polygon(notch.to_vec(), PAPER.gamma_multiply(0.0), egui::Stroke::NONE));
                p.line_segment([notch[0], notch[1]], egui::Stroke::new(1.0_f32, INK_LIGHT.gamma_multiply(0.6)));
                p.line_segment([notch[1], notch[2]], egui::Stroke::new(1.0_f32, INK_LIGHT.gamma_multiply(0.6)));
                if focused_nav {
                    ink::focus_ring(p, rect);
                }
                // 提醒點
                let dot = egui::pos2(rect.left() + ink.px(16.0), rect.top() + ink.px(13.0));
                let (col, glow) = if alert { (SEAL_RED, true) } else if omen { (GOLD, true) } else { (INK_LIGHT.gamma_multiply(0.7), false) };
                if glow {
                    p.circle_filled(dot, ink.px(7.0), col.gamma_multiply(0.22));
                }
                p.circle_filled(dot, ink.px(4.0), col);
                // 名與狀態
                let name_col = if alive { INK } else { INK_LIGHT };
                p.text(egui::pos2(rect.left() + ink.px(26.0), rect.top() + ink.px(13.0)), Align2::LEFT_CENTER, &name, egui::FontId::proportional(body), name_col);
                if !alive {
                    p.text(egui::pos2(rect.right() - ink.px(8.0), rect.top() + ink.px(13.0)), Align2::RIGHT_CENTER, "輪", egui::FontId::proportional(small), INK_LIGHT);
                }
                let sg = p.layout_no_wrap(status, egui::FontId::proportional(small), INK_LIGHT);
                let maxw = width - ink.px(34.0);
                let sg = if sg.size().x > maxw {
                    let keep = ((sg.text().chars().count() as f32) * (maxw / sg.size().x)).floor().max(2.0) as usize - 1;
                    let t: String = sg.text().chars().take(keep).chain(std::iter::once('…')).collect();
                    p.layout_no_wrap(t, egui::FontId::proportional(small), INK_LIGHT)
                } else {
                    sg
                };
                p.galley(egui::pos2(rect.left() + ink.px(26.0), rect.top() + ink.px(24.0)), sg, INK_LIGHT);
                // 停留動作：飛、簿、因、解（右端）
                if hovered {
                    let s = ink.px(20.0);
                    let y = rect.bottom() - s - ink.px(3.0);
                    let acts = [("飛", "fates_fly"), ("簿", "fates_card"), ("因", "fates_threads"), ("解", "fates_unfollow")];
                    for (k, (ch, tip)) in acts.iter().enumerate() {
                        let r = egui::Rect::from_min_size(egui::pos2(rect.right() - ink.px(4.0) - s * (4 - k) as f32 - ink.px(3.0) * (3 - k) as f32, y), egui::vec2(s, s));
                        let rr = ui.interact(r, resp.id.with(k), egui::Sense::click());
                        ink::paint_seal(ui.painter(), r, ch, true, SealState::Idle, ink::seal_fx(ctx, &rr));
                        ink::hover_tip(ctx, ink, &rr, text.t(tip));
                        if rr.clicked() {
                            match k {
                                0 => out.fly_to = soul_cell(world, sid),
                                1 => out.open_soul(world, sid),
                                2 => out.show_threads = Some(sid),
                                _ => out.follow_toggle = Some(sid),
                            }
                        }
                    }
                }
                if resp.clicked() {
                    out.open_soul(world, sid);
                    out.fly_to = soul_cell(world, sid);
                }
            }
            if nav.active && input == InputMode::Gamepad {
                ui.label(egui::RichText::new(text.t("fates_nav_hint")).size(small).color(INK_LIGHT));
            }
        })
        .response
        .rect
}

/// 搖桿命牌列：上下選、A 飛往、Y 解除。回傳是否消費了該動作。
pub fn nav_input(nav: &mut FatesNav, world: &World, dir: i32, select: bool, unfollow: bool, out: &mut UiActions) {
    let follows: Vec<SoulId> = world.follows.iter().copied().collect();
    if follows.is_empty() {
        nav.active = false;
        return;
    }
    let n = follows.len();
    nav.cursor = ((nav.cursor as i32 + dir).rem_euclid(n as i32)) as usize;
    let sid = follows[nav.cursor];
    if select {
        out.open_soul(world, sid);
        out.fly_to = soul_cell(world, sid);
    }
    if unfollow {
        out.follow_toggle = Some(sid);
    }
}
