//! GDD 5.8 命簿內容（7.9 #36）：人物／魂卡（前世今生、宿緣、生平）、事件卡／溯因視圖（因樹、漣漪）、
//! 州郡卡的「此地有名之人」與「近事」，以及溯因的地圖同步（事件印與赤線）。所有人名、地名、事件行都可點（點了就到）。
use bevy::prelude::*;
use bevy_egui::egui::{self, Align, Align2, Id, Layout, Order, Pos2, Rect, RichText, Vec2};
use siming_sim::{calendar, Event, EventId, PathState, PersonId, Province, ThreadKind, World};

use super::context;
use super::deck::UiLayout;
use super::ink::{self, Ink, SealState};
use super::labels::cell_to_screen;
use super::text::{self, EventText};
use super::theme::*;
use super::{CardOpts, UiActions, UiText};
use crate::state::{InputMode, MapView, Target};

/// 卡片標題列：名 + 副題 + 關閉印；回傳是否要求關閉。
fn header(ui: &mut egui::Ui, ink: &Ink, text: &UiText, title: &str, sub: &str, opts: CardOpts, focus_req: &mut bool) -> bool {
    let mut close = false;
    let body = ink.px(14.0).max(if opts.deck { 16.0 } else { 0.0 });
    ui.horizontal(|ui| {
        let now = ui.ctx().input(|i| i.time);
        let shown = ui.ctx().data_mut(|d| *d.get_temp_mut_or(Id::new("card_title").with(title), now));
        let a = (((now - shown) / 0.15).clamp(0.0, 1.0) as f32).max(0.15);
        ui.label(RichText::new(title).size(if opts.deck { 22.0 } else { 20.0 }).color(INK.gamma_multiply(a)));
        ui.label(RichText::new(sub).size(body).color(INK_LIGHT.gamma_multiply(a)));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let r = ink::seal(ui, text.t("card_close_seal"), true, SealState::Idle, ink.px(26.0));
            if *focus_req {
                r.request_focus();
                *focus_req = false;
            }
            ink::hover_tip(ui.ctx(), ink, &r, &format!("{}  {}", text.t("card_close"), if opts.pad { "B" } else { "Esc" }));
            if r.clicked() {
                close = true;
            }
        });
    });
    ink::title_rule(ui, ink);
    ui.add_space(ink.px(2.0));
    close
}

/// 可點的一行文字（人名、地名、事件）：停留墨暈底、按下即動作。
fn link(ui: &mut egui::Ui, text: &str, size: f32, col: egui::Color32) -> egui::Response {
    let r = ui.add(egui::Label::new(RichText::new(text).size(size).color(col)).sense(egui::Sense::click()));
    if r.hovered() || ink::focused(ui.ctx(), &r) {
        ui.painter().rect_filled(r.rect.expand2(egui::vec2(3.0, 1.0)), 2.0, SEAL_RED.gamma_multiply(0.12));
        if ink::focused(ui.ctx(), &r) {
            ink::focus_ring(ui.painter(), r.rect);
        }
    }
    r
}

fn person_sub(w: &World, pid: PersonId, text: &UiText) -> String {
    let Some(p) = w.persons.get(pid as usize) else { return String::new() };
    let mut parts: Vec<String> = Vec::new();
    if let Some(t) = &p.title {
        parts.push(t.clone());
    }
    if let Some(pw) = p.power.and_then(|x| w.powers.get(x as usize)) {
        parts.push(pw.name.clone());
    } else {
        parts.push(text::role_name(p.role).into());
    }
    if !p.alive() {
        parts.push(text.t("dead").into());
    }
    parts.join(" · ")
}

/// 人物／魂卡（5.8）。
#[allow(clippy::too_many_arguments)]
pub fn person_body(ui: &mut egui::Ui, ink: &Ink, text: &UiText, etext: &EventText, world: &World, pid: PersonId, opts: CardOpts, input: InputMode, layout: UiLayout, focus_req: &mut bool, out: &mut UiActions) -> bool {
    let Some(p) = world.persons.get(pid as usize) else { return true };
    let body = ink.px(15.0).max(if opts.deck { 16.0 } else { 0.0 });
    let small = ink.px(12.0).max(if opts.deck { 16.0 } else { 0.0 });
    ui.spacing_mut().item_spacing = egui::vec2(ink.px(6.0), ink.px(4.0));
    let close = header(ui, ink, text, &p.name, &person_sub(world, pid, text), opts, focus_req);
    // 一行：性別 · 年齡 · 境界 · 州郡
    let realm = match &p.path {
        PathState::Cultivator { realm, exp, .. } => {
            let e = siming_sim::params::REALM_EXP.get(*realm as usize).copied().unwrap_or(1.0).max(0.001);
            let layer = ((exp / e * 10.0).floor() as u32 + 1).min(10);
            Some(format!("{}{}", text::realm_name(*realm), if layer >= 10 { "巔峰".to_string() } else { format!("{}層", calendar::zh_num(layer)) }))
        }
        _ => None,
    };
    ui.horizontal_wrapped(|ui| {
        let g = text.t(if p.gender == siming_sim::Gender::Male { "male" } else { "female" });
        ui.label(RichText::new(format!("{g} · {}{}", calendar::zh_num(p.age(world.tick)), text.t("age_suffix"))).size(body).color(INK));
        if let Some(r) = realm {
            ui.label(RichText::new(format!("· {r}")).size(body).color(INK));
        }
        if let Some(prov) = world.provinces.get(p.province as usize) {
            ui.label(RichText::new("·").size(body).color(INK));
            if link(ui, &prov.name, body, INK).clicked() {
                out.open_province = Some(prov.id);
                out.fly_to = Some(prov.seat);
            }
        }
    });
    let a = &p.attrs;
    ui.label(RichText::new(format!("根骨 {:.0} · 悟性 {:.0} · 才略 {:.0} · 德行 {:.0} · 戾氣 {:.0}", a.root, a.insight, a.talent, a.virtue, a.ferocity)).size(small).color(INK_LIGHT));
    if let Some(s) = world.souls.get(p.soul as usize) {
        ink::ink_meter(ui, ink, text.t("card_merit"), (s.merit / 500.0).clamp(0.0, 1.0), GOLD, &format!("{:.0}", s.merit));
        ink::ink_meter(ui, ink, text.t("card_sin"), (s.sin / 200.0).clamp(0.0, 1.0), SEAL_RED, &format!("{:.0}", s.sin));
        // 前世今生：每世一個剪影
        ink::section(ui, ink, text.t("card_lives"));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = ink.px(4.0);
            let n = s.lives.len();
            let start = n.saturating_sub(5);
            for (i, &life) in s.lives.iter().enumerate().skip(start) {
                if i > start {
                    ui.label(RichText::new("→").size(small).color(INK_LIGHT));
                }
                let now = life == pid;
                let w = ink.px(46.0);
                let (rect, r) = ui.allocate_exact_size(egui::vec2(w, ink.px(50.0)), egui::Sense::click());
                let pt = ui.painter();
                let fig = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + ink.px(16.0)), egui::vec2(ink.px(18.0), ink.px(28.0)));
                let col = if now { SEAL_RED } else { INK_LIGHT.gamma_multiply(0.7) };
                pt.rect_filled(fig, egui::CornerRadius { nw: 9, ne: 9, sw: 3, se: 3 }, col);
                pt.circle_filled(egui::pos2(fig.center().x, fig.top() + ink.px(6.0)), ink.px(5.5), col);
                if r.hovered() || ink::focused(ui.ctx(), &r) {
                    pt.circle_filled(fig.center(), ink.px(18.0), SEAL_RED.gamma_multiply(0.12));
                }
                let d = text::life_desc(world, life);
                pt.text(egui::pos2(rect.center().x, rect.bottom()), Align2::CENTER_BOTTOM, d, egui::FontId::proportional(small), if now { INK } else { INK_LIGHT });
                ink::hover_tip(ui.ctx(), ink, &r, &text::person_name(world, life));
                if r.clicked() && !now {
                    out.open_person = Some(life);
                }
            }
        });
        // 宿緣
        ink::section(ui, ink, text.t("card_threads"));
        let mut threads: Vec<&siming_sim::Thread> = s.threads.iter().filter_map(|&t| world.threads.get(t as usize)).collect();
        threads.sort_by(|a, b| siming_sim::thread::is_open(b).cmp(&siming_sim::thread::is_open(a)).then(b.strength.total_cmp(&a.strength)));
        if threads.is_empty() {
            ui.label(RichText::new(text.t("card_none")).size(small).color(INK_LIGHT));
        }
        for t in threads.iter().take(6) {
            let other = if t.debtor == p.soul { t.creditor } else { t.debtor };
            let open = siming_sim::thread::is_open(t);
            let lives = siming_sim::thread::lives(world, t);
            let omen = open && t.omen_at.is_some() && t.omen_at == Some(lives);
            let rel = match (t.kind, t.debtor == p.soul) {
                (ThreadKind::Mentor, true) => text.t("thread_master"),
                (ThreadKind::Mentor, false) => text.t("thread_disciple"),
                (ThreadKind::Love, _) => text.t("thread_with"),
                (_, true) => text.t("thread_owes"),
                (_, false) => text.t("thread_owed"),
            };
            ui.horizontal(|ui| {
                let (lr, _) = ui.allocate_exact_size(egui::vec2(ink.px(26.0), body), egui::Sense::hover());
                let col = thread_color(t.kind).gamma_multiply(if open { 1.0 } else { 0.4 });
                ink::brush_rule(ui.painter(), egui::pos2(lr.left(), lr.center().y), egui::pos2(lr.right(), lr.center().y), 3.0, col, t.id as u64);
                ui.label(RichText::new(format!("{} {:.0} · {rel}", text::thread_name(t.kind), t.strength)).size(body).color(if open { INK } else { INK_LIGHT }));
                let name = text::soul_name(world, other);
                let status = text::soul_status(world, other);
                let r = link(ui, &name, body, if open { INK } else { INK_LIGHT });
                ink::hover_tip(ui.ctx(), ink, &r, &status);
                if r.clicked() {
                    out.open_soul(world, other);
                    out.fly_to = super::fates::soul_cell(world, other);
                    out.show_threads = Some(p.soul);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if omen {
                        ui.label(RichText::new(text.t("thread_omen")).size(small).color(SEAL_RED));
                    } else if !open {
                        ui.label(RichText::new(text.t("thread_resolved")).size(small).color(INK_LIGHT));
                    }
                    ui.label(RichText::new(format!("{} {}", calendar::zh_num(lives as u32), text.t("thread_lives"))).size(small).color(INK_LIGHT));
                });
            });
        }
    }
    // 生平：最近四筆 importance ≥ 2 的事件（反向掃描因果帳，上限 6000 筆）
    ink::section(ui, ink, text.t("card_bio"));
    let mut bio: Vec<&Event> = Vec::new();
    for e in world.ledger.events.iter().rev().take(6000) {
        if e.importance >= 2 && e.persons.contains(&pid) {
            bio.push(e);
            if bio.len() >= 4 {
                break;
            }
        }
    }
    if bio.is_empty() {
        ui.label(RichText::new(text.t("card_none")).size(small).color(INK_LIGHT));
    }
    for e in bio.iter().rev() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(calendar::short(e.tick)).size(small).color(INK_LIGHT));
            let r = link(ui, &etext.line(world, e), body, INK);
            ink::hover_tip(ui.ctx(), ink, &r, text.t("card_trace"));
            if r.clicked() {
                out.open_event = Some(e.id);
            }
        });
    }
    context::actions_row(ui, ink, text, world, Target::Person(pid), input, layout, out);
    close
}

pub fn thread_color(k: ThreadKind) -> egui::Color32 {
    match k {
        ThreadKind::Feud => SEAL_RED,
        ThreadKind::Grace => GOLD,
        ThreadKind::Love => egui::Color32::from_rgb(0xD0, 0x8A, 0x95),
        ThreadKind::Mentor => CYAN,
        ThreadKind::Grudge => egui::Color32::from_rgb(0x5A, 0x3E, 0x6B),
    }
}

/// 溯因樹的一列。
pub struct TraceNode {
    pub id: EventId,
    pub depth: u8,
    /// 是否為直接因（深度 1）；地圖上以實線連接。
    pub direct: bool,
}

/// 沿 `causes` 往上最多 6 層，每層每節點取 importance 最高的 4 筆，總數 ≤ 24（5.8）。
pub fn trace_tree(world: &World, root: EventId) -> Vec<TraceNode> {
    let mut out = vec![TraceNode { id: root, depth: 0, direct: false }];
    fn walk(world: &World, id: EventId, depth: u8, out: &mut Vec<TraceNode>) {
        if depth > 6 || out.len() >= 24 {
            return;
        }
        let Some(e) = siming_sim::ledger::get(world, id) else { return };
        let mut causes: Vec<&Event> = e.causes.iter().filter_map(|&c| siming_sim::ledger::get(world, c)).collect();
        causes.sort_by(|a, b| b.importance.cmp(&a.importance).then(b.tick.cmp(&a.tick)));
        for c in causes.into_iter().take(4) {
            if out.iter().any(|n| n.id == c.id) || out.len() >= 24 {
                continue;
            }
            out.push(TraceNode { id: c.id, depth, direct: depth == 1 });
            walk(world, c.id, depth + 1, out);
        }
    }
    walk(world, root, 1, &mut out);
    out
}

/// 漣漪數：沿反向索引往下，最多 300。
pub fn ripple_count(world: &World, root: EventId) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if seen.len() >= 300 {
            break;
        }
        for &e in world.ledger.effects.get(id as usize).map(|v| v.as_slice()).unwrap_or(&[]) {
            if seen.insert(e) {
                stack.push(e);
            }
        }
    }
    seen.len()
}

/// 事件卡／溯因視圖（5.8）。
#[allow(clippy::too_many_arguments)]
pub fn event_body(ui: &mut egui::Ui, ink: &Ink, text: &UiText, etext: &EventText, world: &World, eid: EventId, opts: CardOpts, input: InputMode, layout: UiLayout, focus_req: &mut bool, out: &mut UiActions) -> bool {
    let Some(e) = siming_sim::ledger::get(world, eid) else { return true };
    let body = ink.px(14.0).max(if opts.deck { 16.0 } else { 0.0 });
    let small = ink.px(12.0).max(if opts.deck { 16.0 } else { 0.0 });
    ui.spacing_mut().item_spacing = egui::vec2(ink.px(6.0), ink.px(4.0));
    let place = e.province.and_then(|p| world.provinces.get(p as usize)).map(|p| p.name.as_str()).unwrap_or("");
    let close = header(ui, ink, text, text.t("card_trace"), &format!("{} · {place}", calendar::short(e.tick)), opts, focus_req);
    ui.label(RichText::new(etext.line(world, e)).size(body).color(INK));
    ink::section(ui, ink, text.t("card_causes"));
    let tree = trace_tree(world, eid);
    let hot = ui.ctx().data(|d| d.get_temp::<EventId>(Id::new("trace_hot")));
    let mut new_hot = None;
    for n in &tree {
        let Some(ev) = siming_sim::ledger::get(world, n.id) else { continue };
        ui.horizontal(|ui| {
            ui.add_space(ink.px(14.0) * n.depth as f32);
            if n.depth > 0 {
                let (lr, _) = ui.allocate_exact_size(egui::vec2(ink.px(8.0), body), egui::Sense::hover());
                ui.painter().line_segment([egui::pos2(lr.left(), lr.center().y), egui::pos2(lr.right(), lr.center().y)], egui::Stroke::new(1.0_f32, INK_LIGHT));
            }
            let s = ink.px(20.0);
            let (sr, _) = ui.allocate_exact_size(egui::vec2(s, s), egui::Sense::hover());
            ink::paint_seal(ui.painter(), sr, etext.seal(ev.kind), false, if n.depth == 0 { SealState::Active } else { SealState::Idle }, ink::SealFx::default());
            let line = etext.line(world, ev);
            let r = link(ui, &line, body, INK);
            if r.hovered() || ink::focused(ui.ctx(), &r) {
                new_hot = Some(ev.id);
            }
            if r.clicked() && n.depth > 0 {
                out.open_event = Some(ev.id);
                out.fly_to_event(world, ev, false);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(short_year(ev.tick)).size(small).color(INK_LIGHT));
            });
        });
    }
    if new_hot != hot {
        ui.ctx().data_mut(|d| match new_hot {
            Some(h) => d.insert_temp(Id::new("trace_hot"), h),
            None => {
                d.remove_temp::<EventId>(Id::new("trace_hot"));
            }
        });
    }
    ink::section(ui, ink, text.t("card_ripples"));
    let n = ripple_count(world, eid);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{n}")).size(ink.px(18.0).max(if opts.deck { 18.0 } else { 0.0 })).color(SEAL_RED));
        if n == 0 {
            ui.label(RichText::new(text.t("card_no_ripples")).size(small).color(INK_LIGHT));
            ui.label(RichText::new(text.t("card_ripples_hint")).size(small).color(INK_LIGHT));
        } else {
            let mut shown = 0;
            let mut stack: Vec<EventId> = world.ledger.effects.get(eid as usize).map(|v| v.iter().copied().collect()).unwrap_or_default();
            while let Some(id) = stack.pop() {
                if shown >= 4 {
                    break;
                }
                if let Some(ev) = siming_sim::ledger::get(world, id) {
                    if ev.importance >= 2 {
                        let r = link(ui, &format!("{} · {}", short_year(ev.tick), etext.line(world, ev)), small, INK);
                        if r.clicked() {
                            out.open_event = Some(ev.id);
                            out.fly_to_event(world, ev, false);
                        }
                        shown += 1;
                    }
                    stack.extend(world.ledger.effects.get(id as usize).map(|v| v.iter().copied()).into_iter().flatten());
                }
            }
        }
    });
    context::actions_row(ui, ink, text, world, Target::Event(eid), input, layout, out);
    close
}

fn short_year(tick: u32) -> String {
    let d = calendar::date(tick);
    format!("{}世{}年", calendar::zh_num(d.shi), calendar::zh_num(d.year))
}

/// 州郡卡的「此地有名之人（前 10 名）」與「近事」（5.8）。
#[allow(clippy::too_many_arguments)]
pub fn province_extra(ui: &mut egui::Ui, ink: &Ink, text: &UiText, etext: &EventText, world: &World, p: &Province, opts: CardOpts, input: InputMode, layout: UiLayout, out: &mut UiActions) {
    let body = ink.px(14.0).max(if opts.deck { 16.0 } else { 0.0 });
    let small = ink.px(12.0).max(if opts.deck { 16.0 } else { 0.0 });
    ink::section(ui, ink, text.t("card_notables"));
    let mut here: Vec<&siming_sim::Person> = world.persons.iter().filter(|x| x.alive() && x.province == p.id && x.role != siming_sim::Role::Child).collect();
    let rank = |x: &siming_sim::Person| match &x.path {
        PathState::Cultivator { realm, exp, .. } => *realm as f32 * 100.0 + exp,
        _ => 0.0,
    };
    here.sort_by(|a, b| rank(b).total_cmp(&rank(a)));
    if here.is_empty() {
        ui.label(RichText::new(text.t("card_none")).size(small).color(INK_LIGHT));
    }
    for x in here.iter().take(10) {
        ui.horizontal(|ui| {
            let r = link(ui, &x.name, body, INK);
            if r.clicked() {
                out.open_person = Some(x.id);
            }
            ui.label(RichText::new(person_sub(world, x.id, text)).size(small).color(INK_LIGHT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let followed = world.follows.contains(&x.soul);
                let r = ink::seal(ui, "關", true, if followed { SealState::Active } else { SealState::Idle }, ink.px(20.0));
                ink::hover_tip(ui.ctx(), ink, &r, text.t(if followed { "act_unfollow" } else { "act_follow" }));
                if r.clicked() {
                    out.follow_toggle = Some(x.soul);
                }
            });
        });
    }
    ink::section(ui, ink, text.t("card_recent"));
    let mut recent: Vec<&Event> = Vec::new();
    for e in world.ledger.events.iter().rev().take(4000) {
        if e.importance >= 2 && e.province == Some(p.id) {
            recent.push(e);
            if recent.len() >= 3 {
                break;
            }
        }
    }
    if recent.is_empty() {
        ui.label(RichText::new(text.t("card_none")).size(small).color(INK_LIGHT));
    }
    for e in recent {
        ui.horizontal(|ui| {
            ui.label(RichText::new(short_year(e.tick)).size(small).color(INK_LIGHT));
            let r = link(ui, &etext.line(world, e), body, INK);
            if r.clicked() {
                out.open_event = Some(e.id);
            }
        });
    }
    context::actions_row(ui, ink, text, world, Target::Province(p.id), input, layout, out);
}

/// 溯因的地圖同步（5.8）：樹中每個有地點的事件放一枚事件印，赤線連到根事件（直接因實線、間接因虛線）。
#[allow(clippy::too_many_arguments)]
pub fn draw_trace_overlay(ctx: &egui::Context, ink: &Ink, etext: &EventText, world: &World, view: &MapView, camera: Option<(&Camera, &GlobalTransform)>, root: EventId, avoid: &[Rect]) {
    let Some((camera, tf)) = camera else { return };
    let tree = trace_tree(world, root);
    let painter = ctx.layer_painter(egui::LayerId::new(Order::Middle, Id::new("trace_overlay")));
    let hot = ctx.data(|d| d.get_temp::<EventId>(Id::new("trace_hot")));
    let mut pins: Vec<(Pos2, &TraceNode, &Event)> = Vec::new();
    let avail = ctx.available_rect();
    for n in &tree {
        let Some(e) = siming_sim::ledger::get(world, n.id) else { continue };
        let Some(cell) = super::stream::event_cell(world, e) else { continue };
        let Some(pos) = cell_to_screen(world, view, camera, tf, cell, avail) else { continue };
        if avoid.iter().any(|r| r.contains(pos)) {
            continue;
        }
        pins.push((pos, n, e));
    }
    let Some(&(rp, _, _)) = pins.iter().find(|(_, n, _)| n.depth == 0) else { return };
    for (pos, n, _) in pins.iter().filter(|(_, n, _)| n.depth > 0) {
        let mid = Pos2::new((rp.x + pos.x) * 0.5, rp.y.min(pos.y) - (rp.distance(*pos) * 0.25).min(120.0));
        let pts: Vec<Pos2> = (0..=24).map(|i| {
            let t = i as f32 / 24.0;
            let a = rp.lerp(mid, t);
            let b = mid.lerp(*pos, t);
            a.lerp(b, t)
        }).collect();
        let col = SEAL_RED.gamma_multiply(if hot == Some(n.id) { 1.0 } else { 0.75 });
        if n.direct {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(2.0_f32, col)));
        } else {
            for seg in pts.chunks(2) {
                if seg.len() == 2 {
                    painter.line_segment([seg[0], seg[1]], egui::Stroke::new(1.6_f32, col));
                }
            }
        }
    }
    let s = ink.px(26.0);
    let font = egui::FontId::proportional(ink.px(12.0).max(if ink.scale > 1.0 { 16.0 } else { 0.0 }));
    for (pos, n, e) in &pins {
        let rect = Rect::from_center_size(*pos, Vec2::splat(s));
        let state = if n.depth == 0 || hot == Some(n.id) { SealState::Active } else { SealState::Idle };
        painter.circle_filled(*pos, s * 0.62, PAPER.gamma_multiply(0.85));
        ink::paint_seal(&painter, rect, etext.seal(e.kind), false, state, ink::SealFx::default());
        let line = etext.line(world, e);
        let g = painter.layout_no_wrap(line, font.clone(), INK);
        let tp = Pos2::new(rect.right() + ink.px(6.0), rect.top() + 2.0);
        for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
            painter.galley_with_override_text_color(tp + d, g.clone(), PAPER);
        }
        painter.galley_with_override_text_color(tp, g, INK);
        painter.text(Pos2::new(rect.right() + ink.px(6.0), rect.top() + 2.0 + font.size + 2.0), Align2::LEFT_TOP, short_year(e.tick), egui::FontId::proportional(font.size * 0.85), INK_LIGHT);
    }
}

/// 自動止／緩時，事件兩端的朱環（2 秒）。
pub fn draw_alert_rings(ctx: &egui::Context, world: &World, view: &MapView, camera: Option<(&Camera, &GlobalTransform)>, e: &Event, age: f32) {
    let Some((camera, tf)) = camera else { return };
    if age > 2.0 {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(Order::Middle, Id::new("alert_rings")));
    let mut cells: Vec<u32> = e.persons.iter().filter_map(|&p| world.persons.get(p as usize)).map(|p| p.cell).collect();
    cells.extend(e.souls.iter().filter_map(|&s| super::fates::soul_cell(world, s)));
    if cells.is_empty() {
        cells.extend(super::stream::event_cell(world, e));
    }
    let avail = ctx.available_rect();
    let pulse = (age * 1.4).fract();
    let mut pts = Vec::new();
    for c in cells {
        if let Some(pos) = cell_to_screen(world, view, camera, tf, c, avail) {
            painter.circle_stroke(pos, 10.0 + pulse * 18.0, egui::Stroke::new(2.5_f32 * (1.0 - pulse), SEAL_RED));
            painter.circle_stroke(pos, 8.0, egui::Stroke::new(2.0_f32, SEAL_RED));
            pts.push(pos);
        }
    }
    if pts.len() == 2 {
        let (a, b) = (pts[0], pts[1]);
        let mid = Pos2::new((a.x + b.x) * 0.5, a.y.min(b.y) - (a.distance(b) * 0.3).min(140.0));
        let line: Vec<Pos2> = (0..=30).map(|i| {
            let t = i as f32 / 30.0;
            a.lerp(mid, t).lerp(mid.lerp(b, t), t)
        }).collect();
        for seg in line.chunks(2) {
            if seg.len() == 2 {
                painter.line_segment([seg[0], seg[1]], egui::Stroke::new(2.5_f32, SEAL_RED.gamma_multiply(0.9)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_colors_follow_gdd_5_1() {
        assert_eq!(thread_color(ThreadKind::Feud), SEAL_RED);
        assert_eq!(thread_color(ThreadKind::Grace), GOLD);
        assert_eq!(thread_color(ThreadKind::Mentor), CYAN);
    }

    #[test]
    fn short_year_uses_shi_and_year() {
        assert_eq!(short_year(1), "一世一年");
    }
}
