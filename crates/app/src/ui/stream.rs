//! GDD 4.7、5.7 傳聞簽與通知分級（7.9 #36）：事件由 bridge 送入 `EventFeed`，依「止／緩／簽／記」分級；
//! 右上角最多 8 張紙簽，Tab 展開為 50 則清單；停留浮出「飛」「溯」兩印，點擊飛往並開啟命簿。
use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_egui::egui::{self, Align2, Id, Order, RichText};
use siming_sim::{calendar, Event, EventId, EventKind, SoulId, World};

use super::deck::UiLayout;
use super::ink::{self, Ink, SealState};
use super::text::{EventText, BOOK_TOTAL};
use super::theme::*;
use super::{UiActions, UiText};
use crate::state::{InputMode, NoticeLevel, SimNotice, Speed, TimeControl};

/// 通知層級（5.7 通知優先序），由低到高。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// 只進 50 則清單。
    Log,
    /// 入 8 張簽。
    Slip,
    /// 緩：降到 1×。
    Slow,
    /// 止：暫停。
    Stop,
}

#[derive(Debug, Clone, Copy)]
pub struct FeedItem {
    pub id: EventId,
    pub level: Level,
    /// 牽涉關注的魂（左緣朱記）。
    pub followed: bool,
    /// 入簽的牆鐘時間（滑入動畫）。
    pub added: f64,
}

/// 傳聞簽的資料（最多 50 則，新到舊）。
#[derive(Resource, Default)]
pub struct EventFeed {
    pub items: VecDeque<FeedItem>,
    /// 最近一次自動止的年份（同一年只止一次）。
    pub last_stop_year: Option<u32>,
    /// 展開為 50 則清單。
    pub expanded: bool,
    /// 搖桿／`[` `]` 選中的簽（索引入 `items`）。
    pub cursor: Option<usize>,
    /// 自動止／緩時置頂高亮的事件。
    pub hot: Option<EventId>,
}

const FEED_CAP: usize = 50;
pub const SLIPS_SHOWN: usize = 8;

/// 事件是否牽涉關注的魂（人物經其魂判定）。
pub fn involves_followed(w: &World, e: &Event) -> bool {
    e.souls.iter().any(|s| w.follows.contains(s)) || e.persons.iter().any(|&p| w.persons.get(p as usize).is_some_and(|p| w.follows.contains(&p.soul)))
}

/// 牽涉的線是否為司命所牽（M5 前恆為假）。
fn involves_woven(w: &World, e: &Event) -> bool {
    match e.args {
        siming_sim::EventArgs::Thread { thread, .. } => w.threads.get(thread as usize).is_some_and(|t| t.woven),
        _ => false,
    }
}

/// GDD 5.7 通知優先序 + 4.7 自動暫停預設表。
pub fn classify(w: &World, e: &Event) -> (Level, bool) {
    let followed = involves_followed(w, e);
    let woven = involves_woven(w, e);
    let stop = match e.kind {
        EventKind::ThreadResolved | EventKind::ThreadOmen => woven || followed,
        EventKind::Ascended => true,
        EventKind::Died => followed,
        _ => false,
    };
    let level = if stop {
        Level::Stop
    } else if e.importance >= 5 && !followed {
        Level::Slow
    } else if e.importance >= 3 || (followed && e.importance >= 2) {
        Level::Slip
    } else {
        Level::Log
    };
    (level, followed)
}

/// 事件的地點格：事件本身的格、州郡治所，或第一位人物所在格。
pub fn event_cell(w: &World, e: &Event) -> Option<u32> {
    e.cell
        .or_else(|| e.province.and_then(|p| w.provinces.get(p as usize)).map(|p| p.seat))
        .or_else(|| e.persons.first().and_then(|&p| w.persons.get(p as usize)).map(|p| p.cell))
        .or_else(|| e.souls.first().and_then(|&s| w.souls.get(s as usize)).map(|s| s.last_place.1))
}

/// 由 bridge 在每次推進後呼叫：分級、入簽、自動止／緩（5.7）。`now` 為牆鐘秒。
pub fn ingest(feed: &mut EventFeed, w: &World, ids: &[EventId], tc: &mut TimeControl, notice: &mut SimNotice, etext: &EventText, now: f64) {
    if w.prerun {
        return;
    }
    let mut stops: Vec<EventId> = Vec::new();
    let mut slow: Option<EventId> = None;
    for &id in ids {
        let Some(e) = siming_sim::ledger::get(w, id) else { continue };
        if e.importance < 2 {
            continue;
        }
        let (level, followed) = classify(w, e);
        // 同一魂同一年合併：取較高層級
        let year = e.tick / 12;
        let dup = feed.items.iter_mut().find(|it| {
            siming_sim::ledger::get(w, it.id).is_some_and(|old| old.tick / 12 == year && !e.souls.is_empty() && old.souls.first() == e.souls.first() && old.kind == e.kind)
        });
        if let Some(d) = dup {
            d.level = d.level.max(level);
            d.followed |= followed;
        } else {
            feed.items.push_front(FeedItem { id, level, followed, added: now });
            if feed.items.len() > FEED_CAP {
                feed.items.pop_back();
            }
        }
        match level {
            Level::Stop => stops.push(id),
            Level::Slow => slow = slow.or(Some(id)),
            _ => {}
        }
    }
    let year = w.tick / 12;
    if let Some(&first) = stops.first() {
        if feed.last_stop_year != Some(year) {
            feed.last_stop_year = Some(year);
            tc.paused = true;
            let e = siming_sim::ledger::get(w, first);
            *notice = SimNotice {
                key: "",
                text: e.map(|e| etext.line(w, e)).unwrap_or_default(),
                level: NoticeLevel::Stop,
                cell: e.and_then(|e| event_cell(w, e)),
                event: Some(first),
                extra: stops.len().saturating_sub(1) as u32,
                remaining: f32::INFINITY,
            };
            feed.hot = Some(first);
        }
    } else if let Some(id) = slow {
        if !tc.paused && tc.speed != Speed::X1 {
            tc.speed = Speed::X1;
            let e = siming_sim::ledger::get(w, id);
            *notice = SimNotice {
                key: "",
                text: e.map(|e| etext.line(w, e)).unwrap_or_default(),
                level: NoticeLevel::Slow,
                cell: e.and_then(|e| event_cell(w, e)),
                event: Some(id),
                extra: 0,
                remaining: 6.0,
            };
            feed.hot = Some(id);
        }
    }
}

/// 續行（Space／☰）時由介面呼叫：止級原因簽開始倒數捲回。
pub fn resume(notice: &mut SimNotice) {
    if notice.level == NoticeLevel::Stop && notice.remaining.is_infinite() {
        notice.remaining = 1.0;
    }
}

/// 右上角傳聞簽；回傳佔用矩形（供地圖標籤避讓）。
#[allow(clippy::too_many_arguments)]
pub fn draw(
    ctx: &egui::Context,
    ink: &Ink,
    text: &UiText,
    etext: &EventText,
    world: &World,
    feed: &mut EventFeed,
    layout: UiLayout,
    input: InputMode,
    avail: egui::Rect,
    bottom_limit: f32,
    out: &mut UiActions,
) -> egui::Rect {
    let deck = layout == UiLayout::Deck;
    let now = ctx.input(|i| i.time);
    let width = ink.px(if feed.expanded { 360.0 } else { 300.0 });
    let body = ink.px(14.0).max(if deck { 16.0 } else { 0.0 });
    let small = ink.px(12.0).max(if deck { 16.0 } else { 0.0 });
    let shown = if feed.expanded {
        feed.items.len()
    } else if deck {
        1
    } else {
        SLIPS_SHOWN
    };
    let area = egui::Area::new(Id::new("rumor_stream"))
        .order(Order::Middle)
        .pivot(Align2::RIGHT_TOP)
        .fixed_pos(egui::pos2(avail.right() - 12.0, avail.top() + 10.0))
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(ink.px(6.0), ink.px(5.0));
            // 頁首：傳聞 ● 近 n 則 〔Tab〕｜天書 n／23（先 horizontal 定住列高，否則 right_to_left 會吃掉整個可用高度）
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let book = world.book.entries.len();
                let r = ui.label(RichText::new(format!("{} {book}／{BOOK_TOTAL}", text.t("book_count"))).size(small).color(if book > 0 { SEAL_RED } else { INK_LIGHT }));
                ink::hover_tip(ctx, ink, &r, text.t("book_tip"));
                ui.label(RichText::new("｜").size(small).color(INK_LIGHT));
                let key = if input == InputMode::Gamepad { super::prompts::Glyph::DPad } else { super::prompts::Glyph::key("Tab") };
                let r = super::prompts::glyph(ui, &key, layout);
                ink::hover_tip(ctx, ink, &r, text.t(if feed.expanded { "rumor_collapse" } else { "rumor_expand" }));
                let n = feed.items.len();
                let head = ui.label(RichText::new(format!("{} {n} {}", text.t("rumor_recent"), text.t("rumor_unit"))).size(small).color(INK_LIGHT));
                // 朱點：有新簽（最近 3 秒內）時實心，否則淡墨（字型子集無「●」，以向量繪製）
                let (dr, _) = ui.allocate_exact_size(egui::vec2(small * 0.8, small), egui::Sense::hover());
                let fresh = feed.items.front().is_some_and(|it| now - it.added < 3.0);
                ui.painter().circle_filled(dr.center(), small * 0.22, if fresh { SEAL_RED } else { INK_LIGHT.gamma_multiply(0.6) });
                let head2 = ui.label(RichText::new(text.t("rumor_title")).size(small).color(INK_LIGHT));
                if head.clicked() || head2.clicked() || r.clicked() {
                    feed.expanded = !feed.expanded;
                }
                });
            });
            let draw_items = |ui: &mut egui::Ui, feed: &mut EventFeed, out: &mut UiActions| {
                let items: Vec<(usize, FeedItem)> = feed.items.iter().copied().enumerate().filter(|(_, it)| feed.expanded || it.level >= Level::Slip).take(shown).collect();
                for (idx, it) in items {
                    let Some(e) = siming_sim::ledger::get(world, it.id) else { continue };
                    let hot = feed.hot == Some(it.id);
                    let cursor = feed.cursor == Some(idx);
                    let slide = ((now - it.added) / 0.25).clamp(0.0, 1.0) as f32;
                    rumor_slip(ui, ink, etext, world, e, it, hot, cursor, slide, body, small, out, feed);
                }
            };
            if feed.expanded {
                let max_h = (bottom_limit - ui.cursor().top() - ink.px(12.0)).max(ink.px(80.0));
                egui::ScrollArea::vertical().max_height(max_h).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                    ui.set_width(width);
                    draw_items(ui, feed, out);
                });
            } else {
                draw_items(ui, feed, out);
            }
            if feed.items.is_empty() {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(text.t("rumor_empty")).size(small).color(INK_LIGHT.gamma_multiply(0.8)));
                    });
                });
            }
        });
    area.response.rect
}

/// 一張傳聞簽：事件印 + 一句話 + 年份；停留時右端浮出「飛」「溯」。
#[allow(clippy::too_many_arguments)]
fn rumor_slip(
    ui: &mut egui::Ui,
    ink: &Ink,
    etext: &EventText,
    world: &World,
    e: &Event,
    it: FeedItem,
    hot: bool,
    cursor: bool,
    slide: f32,
    body: f32,
    small: f32,
    out: &mut UiActions,
    feed: &mut EventFeed,
) {
    let h = ink.px(32.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click());
    let rect = rect.translate(egui::vec2((1.0 - slide) * 40.0, 0.0));
    let hovered = resp.hovered() || ink::focused(ui.ctx(), &resp) || cursor;
    let p = ui.painter();
    let mut shapes = ink::slip_shapes(rect, ink, slide);
    if hot || hovered {
        shapes.push(egui::Shape::rect_filled(rect.shrink(1.0), 2.0, egui::Color32::from_rgba_unmultiplied(0xF7, 0xEE, 0xDD, 200)));
    }
    p.add(egui::Shape::Vec(shapes));
    if it.followed {
        ink::brush_rule(p, egui::pos2(rect.left() + ink.px(3.0), rect.top() + ink.px(5.0)), egui::pos2(rect.left() + ink.px(3.0), rect.bottom() - ink.px(5.0)), 3.0, SEAL_RED, it.id as u64);
    }
    if hot {
        p.rect_stroke(rect.shrink(0.5), 2.0, egui::Stroke::new(1.5_f32, SEAL_RED), egui::StrokeKind::Inside);
    }
    if cursor {
        ink::focus_ring(p, rect);
    }
    // 事件印
    let s = ink.px(24.0);
    let seal_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + ink.px(7.0), rect.center().y - s * 0.5), egui::vec2(s, s));
    ink::paint_seal(p, seal_rect, etext.seal(e.kind), false, if hot { SealState::Active } else { SealState::Idle }, ink::SealFx::default());
    // 年份（右端）與一句話（中段，超出截斷）
    let year = calendar::date(e.tick);
    let ytxt = format!("{}年", calendar::zh_num(year.total_year));
    let yg = p.layout_no_wrap(ytxt, egui::FontId::proportional(small), INK_LIGHT);
    let acts_w = if hovered { ink.px(52.0) } else { 0.0 };
    let yx = rect.right() - ink.px(8.0) - acts_w - yg.size().x;
    p.galley(egui::pos2(yx, rect.center().y - yg.size().y * 0.5), yg, INK_LIGHT);
    let line = etext.line(world, e);
    let x0 = seal_rect.right() + ink.px(8.0);
    let maxw = (yx - ink.px(6.0) - x0).max(10.0);
    let col = if hot { SEAL_RED } else { INK };
    let g = p.layout(line, egui::FontId::proportional(body), col, f32::INFINITY);
    let g = if g.size().x > maxw { p.layout_no_wrap(truncate(g.text(), maxw, g.size().x), egui::FontId::proportional(body), col) } else { g };
    p.galley(egui::pos2(x0, rect.center().y - g.size().y * 0.5), g, col);
    // 停留動作印：飛、溯
    if hovered {
        let s2 = ink.px(22.0);
        let fly_rect = egui::Rect::from_min_size(egui::pos2(rect.right() - ink.px(6.0) - s2 * 2.0 - ink.px(4.0), rect.center().y - s2 * 0.5), egui::vec2(s2, s2));
        let tr_rect = fly_rect.translate(egui::vec2(s2 + ink.px(4.0), 0.0));
        let fly = ui.interact(fly_rect, resp.id.with("fly"), egui::Sense::click());
        let tr = ui.interact(tr_rect, resp.id.with("trace"), egui::Sense::click());
        ink::paint_seal(ui.painter(), fly_rect, "飛", true, SealState::Idle, ink::seal_fx(ui.ctx(), &fly));
        ink::paint_seal(ui.painter(), tr_rect, "溯", true, SealState::Idle, ink::seal_fx(ui.ctx(), &tr));
        if fly.clicked() {
            out.fly_to_event(world, e, false);
            feed.hot = None;
        }
        if tr.clicked() {
            out.open_event = Some(e.id);
            out.fly_to_event(world, e, false);
            feed.hot = None;
        }
    }
    if resp.clicked() {
        out.fly_to_event(world, e, true);
        feed.hot = None;
    }
    let tip = calendar::short(e.tick);
    ink::hover_tip(ui.ctx(), ink, &resp, &tip);
}

/// 依比例截斷並加「…」。
fn truncate(s: &str, maxw: f32, fullw: f32) -> String {
    let n = s.chars().count();
    let keep = ((n as f32) * (maxw / fullw)).floor().max(1.0) as usize;
    let mut t: String = s.chars().take(keep.saturating_sub(1)).collect();
    t.push('…');
    t
}

/// 十字鍵 ◁▷／`[` `]`：移動游標並飛往該則。
pub fn step_cursor(feed: &mut EventFeed, world: &World, dir: i32, out: &mut UiActions) {
    if feed.items.is_empty() {
        return;
    }
    let n = feed.items.len() as i32;
    let i = feed.cursor.map_or(if dir > 0 { 0 } else { n - 1 }, |c| (c as i32 + dir).rem_euclid(n));
    feed.cursor = Some(i as usize);
    if let Some(e) = siming_sim::ledger::get(world, feed.items[i as usize].id) {
        out.fly_to_event(world, e, true);
    }
}

/// 命牌的提醒點：此魂在最近一年內是否有入簽事件。
pub fn recent_alert(feed: &EventFeed, world: &World, soul: SoulId) -> bool {
    feed.items.iter().any(|it| {
        siming_sim::ledger::get(world, it.id).is_some_and(|e| {
            e.tick + 12 >= world.tick && (e.souls.contains(&soul) || e.persons.iter().any(|&p| world.persons.get(p as usize).is_some_and(|p| p.soul == soul)))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered() {
        assert!(Level::Stop > Level::Slow && Level::Slow > Level::Slip && Level::Slip > Level::Log);
    }

    #[test]
    fn truncate_keeps_ellipsis() {
        let t = truncate("一二三四五六七八九十", 50.0, 100.0);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() <= 6);
    }
}
