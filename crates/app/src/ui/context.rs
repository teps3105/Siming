//! GDD 5.7 情境印環（7.9 #36）：圍繞被選取主體的一圈圓印——關（關注／解除）、溯（溯因）、因（顯線）、中（置中）。
//! M5 於同環加入權能印。右鍵／X 開啟；Esc／B 收起。卡片頁尾的動作印列與此共用 [`actions_row`]。
use bevy_egui::egui::{self, Align2, Id, Order, Pos2, Rect, Vec2};
use siming_sim::World;

use super::deck::UiLayout;
use super::ink::{self, Ink, SealState};
use super::{UiActions, UiText};
use crate::state::{InputMode, Target};

/// 一枚動作印。
pub struct Action {
    pub ch: &'static str,
    pub label: &'static str,
    pub key_kbm: &'static str,
    pub key_pad: &'static str,
    pub state: SealState,
    pub kind: ActionKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Follow,
    Trace,
    Threads,
    Center,
    Fly,
    Card,
    /// M5 權能（未開放）。
    Power,
}

/// 主體可做的動作（5.7、5.8 頁尾動作印）。
pub fn actions_for(world: &World, target: Target) -> Vec<Action> {
    let mut v = Vec::new();
    match target {
        Target::Person(pid) => {
            let followed = world.persons.get(pid as usize).is_some_and(|p| world.follows.contains(&p.soul));
            v.push(Action { ch: "關", label: if followed { "act_unfollow" } else { "act_follow" }, key_kbm: "F", key_pad: "Y", state: if followed { SealState::Active } else { SealState::Idle }, kind: ActionKind::Follow });
            v.push(Action { ch: "溯", label: "act_trace", key_kbm: "X", key_pad: "A", state: SealState::Idle, kind: ActionKind::Trace });
            v.push(Action { ch: "因", label: "act_threads", key_kbm: "F5", key_pad: "A", state: SealState::Idle, kind: ActionKind::Threads });
            v.push(Action { ch: "中", label: "act_center", key_kbm: "R3", key_pad: "R3", state: SealState::Idle, kind: ActionKind::Center });
            v.push(Action { ch: "權", label: "act_power", key_kbm: "", key_pad: "", state: SealState::Disabled, kind: ActionKind::Power });
        }
        Target::Province(_) => {
            v.push(Action { ch: "中", label: "act_center", key_kbm: "R3", key_pad: "R3", state: SealState::Idle, kind: ActionKind::Center });
            v.push(Action { ch: "因", label: "act_threads_here", key_kbm: "F5", key_pad: "A", state: SealState::Idle, kind: ActionKind::Threads });
            v.push(Action { ch: "權", label: "act_power", key_kbm: "", key_pad: "", state: SealState::Disabled, kind: ActionKind::Power });
        }
        Target::Event(eid) => {
            let e = siming_sim::ledger::get(world, eid);
            let any_soul = e.is_some_and(|e| !e.souls.is_empty() || !e.persons.is_empty());
            v.push(Action { ch: "飛", label: "act_fly", key_kbm: "Enter", key_pad: "A", state: SealState::Idle, kind: ActionKind::Fly });
            if any_soul {
                let all_followed = e.is_some_and(|e| super::stream::involves_followed(world, e));
                v.push(Action { ch: "因", label: "act_threads", key_kbm: "F5", key_pad: "A", state: SealState::Idle, kind: ActionKind::Threads });
                v.push(Action { ch: "關", label: "act_follow_all", key_kbm: "F", key_pad: "Y", state: if all_followed { SealState::Active } else { SealState::Idle }, kind: ActionKind::Follow });
                v.push(Action { ch: "簿", label: "act_card", key_kbm: "", key_pad: "A", state: SealState::Idle, kind: ActionKind::Card });
            }
        }
    }
    v
}

/// 套用一個動作到 `UiActions`。
pub fn apply(kind: ActionKind, world: &World, target: Target, out: &mut UiActions) {
    match (kind, target) {
        (ActionKind::Follow, Target::Person(pid)) => {
            if let Some(p) = world.persons.get(pid as usize) {
                out.follow_toggle = Some(p.soul);
            }
        }
        (ActionKind::Follow, Target::Event(eid)) => {
            if let Some(e) = siming_sim::ledger::get(world, eid) {
                let mut souls: Vec<_> = e.souls.iter().copied().collect();
                souls.extend(e.persons.iter().filter_map(|&p| world.persons.get(p as usize)).map(|p| p.soul));
                souls.sort_unstable();
                souls.dedup();
                out.follow_many = souls;
            }
        }
        (ActionKind::Trace, Target::Person(pid)) => {
            if let Some(ev) = world.persons.get(pid as usize).and_then(|p| p.last_event) {
                out.open_event = Some(ev);
            }
        }
        (ActionKind::Threads, Target::Person(pid)) => {
            if let Some(p) = world.persons.get(pid as usize) {
                out.show_threads = Some(p.soul);
            }
        }
        (ActionKind::Threads, Target::Event(eid)) => {
            if let Some(e) = siming_sim::ledger::get(world, eid) {
                let sid = e.souls.first().copied().or_else(|| e.persons.first().and_then(|&p| world.persons.get(p as usize)).map(|p| p.soul));
                if let Some(sid) = sid {
                    out.show_threads = Some(sid);
                }
            }
        }
        (ActionKind::Threads, Target::Province(_)) => out.mode_yinguo = true,
        (ActionKind::Center, _) => out.recenter = true,
        (ActionKind::Fly, Target::Event(eid)) => {
            if let Some(e) = siming_sim::ledger::get(world, eid) {
                out.fly_to_event(world, e, false);
            }
        }
        (ActionKind::Card, Target::Event(eid)) => {
            if let Some(e) = siming_sim::ledger::get(world, eid) {
                if let Some(&pid) = e.persons.first() {
                    out.open_person = Some(pid);
                } else if let Some(&sid) = e.souls.first() {
                    out.open_soul(world, sid);
                }
            }
        }
        _ => {}
    }
}

/// 卡片頁尾的動作印列（印 + 下方小字）。
#[allow(clippy::too_many_arguments)]
pub fn actions_row(ui: &mut egui::Ui, ink: &Ink, text: &UiText, world: &World, target: Target, input: InputMode, layout: UiLayout, out: &mut UiActions) {
    let deck = layout == UiLayout::Deck;
    let size = ink.px(32.0);
    let small = ink.px(11.0).max(if deck { 16.0 } else { 0.0 });
    ui.add_space(ink.px(6.0));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = ink.px(10.0);
        for a in actions_for(world, target) {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = ink.px(1.0);
                let r = ink::seal(ui, a.ch, true, a.state, size);
                let key = if input == InputMode::Gamepad { a.key_pad } else { a.key_kbm };
                let tip = if key.is_empty() { text.t(a.label).to_string() } else { format!("{}  {}", text.t(a.label), key) };
                ink::hover_tip(ui.ctx(), ink, &r, &tip);
                let (lr, _) = ui.allocate_exact_size(egui::vec2(size, small + 2.0), egui::Sense::hover());
                ui.painter().text(lr.center_top(), Align2::CENTER_TOP, text.t(a.label), egui::FontId::proportional(small), if a.state == SealState::Disabled { INK_LIGHT.gamma_multiply(0.6) } else { INK_LIGHT });
                if r.clicked() && a.state != SealState::Disabled {
                    apply(a.kind, world, target, out);
                }
            });
        }
    });
}

use super::theme::*;

/// 地圖上的情境印環：以 `center`（主體的螢幕位置）為中心，四至五印成環；靠邊時整環內推。
#[allow(clippy::too_many_arguments)]
pub fn ring(ctx: &egui::Context, ink: &Ink, text: &UiText, world: &World, target: Target, center: Pos2, input: InputMode, layout: UiLayout, t: f32, out: &mut UiActions) -> Rect {
    let deck = layout == UiLayout::Deck;
    let radius = ink.px(52.0) * (0.6 + 0.4 * t);
    let size = ink.px(34.0);
    let small = ink.px(10.0).max(if deck { 14.0 } else { 0.0 });
    let label_font = ink.px(12.0).max(if deck { 16.0 } else { 0.0 });
    // 以可用區域（Deck 抽屜已扣除）夾住，整環與標籤都不壓到抽屜
    let screen = ctx.available_rect();
    let pad = radius + size;
    let c = Pos2::new(center.x.clamp(screen.left() + pad, screen.right() - pad - ink.px(70.0)), center.y.clamp(screen.top() + pad, screen.bottom() - pad));
    let acts = actions_for(world, target);
    let bounds = Rect::from_center_size(c, Vec2::splat((radius + size) * 2.0 + ink.px(140.0)));
    egui::Area::new(Id::new("context_ring")).order(Order::Foreground).fixed_pos(bounds.min).show(ctx, |ui| {
        ui.set_min_size(bounds.size());
        let p = ui.painter();
        p.circle_filled(c, ink.px(3.0), SEAL_RED.gamma_multiply(t));
        let n = acts.len().max(1) as f32;
        for (i, a) in acts.iter().enumerate() {
            let ang = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n;
            let dir = Vec2::new(ang.cos(), ang.sin());
            p.line_segment([c + dir * ink.px(8.0), c + dir * (radius - size * 0.55)], egui::Stroke::new(1.0_f32, SEAL_RED.gamma_multiply(0.45 * t)));
            let rect = Rect::from_center_size(c + dir * radius, Vec2::splat(size));
            let resp = ui.interact(rect, Id::new("ctx_seal").with(i), if a.state == SealState::Disabled { egui::Sense::hover() } else { egui::Sense::click() });
            let fx = ink::seal_fx(ctx, &resp);
            ink::paint_seal(ui.painter(), rect, a.ch, true, a.state, fx);
            let key = if input == InputMode::Gamepad { a.key_pad } else { a.key_kbm };
            if !key.is_empty() {
                let kr = Rect::from_center_size(rect.center_bottom() + Vec2::new(0.0, ink.px(6.0)), Vec2::new(small * key.chars().count().max(1) as f32 * 0.7 + ink.px(6.0), small + ink.px(4.0)));
                ui.painter().rect_filled(kr, 2.0, ink::PAPER_LIGHT.gamma_multiply(0.95 * t));
                ui.painter().rect_stroke(kr, 2.0, egui::Stroke::new(1.0_f32, INK_LIGHT.gamma_multiply(t)), egui::StrokeKind::Inside);
                ui.painter().text(kr.center(), Align2::CENTER_CENTER, key, egui::FontId::proportional(small), INK.gamma_multiply(t));
            }
            // 標籤：印的外側
            // 標籤放在印的外側：右半環靠右、左半環靠左、正上正下一律靠右
            let right = dir.x >= -0.3;
            let side = if right { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER };
            let lp = if right { Pos2::new(rect.right() + ink.px(6.0), rect.center().y) } else { Pos2::new(rect.left() - ink.px(6.0), rect.center().y) };
            let g = ui.painter().layout_no_wrap(text.t(a.label).to_string(), egui::FontId::proportional(label_font), INK);
            let tr = side.anchor_size(lp, g.size());
            for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
                ui.painter().galley_with_override_text_color(tr.min + d, g.clone(), PAPER.gamma_multiply(0.9 * t));
            }
            ui.painter().galley_with_override_text_color(tr.min, g, if a.state == SealState::Disabled { INK_LIGHT.gamma_multiply(t) } else { INK.gamma_multiply(t) });
            if resp.clicked() && a.state != SealState::Disabled {
                apply(a.kind, world, target, out);
                out.close_ring = true;
            }
        }
    });
    bounds
}
