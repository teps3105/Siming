//! GDD 5.7「HUD 是互動層」（7.9 #34、#36）：天時儀與題簽、原因簽、傳聞簽、命牌、情境印環、命簿（州郡／人物／事件）、
//! 印匣與例簽、按鍵提示、引導簽。所有器物以 `ink.rs` 繪製；介面的動作先收進 [`UiActions`]，於幀末統一套用。
mod cards;
mod context;
mod deck;
mod fates;
mod ink;
mod labels;
mod prompts;
pub mod stream;
pub mod text;
mod theme;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_egui::egui::{self, Align2, Id, Order, RichText};
use bevy_egui::{EguiContextPass, EguiContexts};
use siming_sim::{calendar, Command, Event, EventId, PersonId, Province, ProvinceId, SoulId, SoulState, Terrain, World};

use crate::bridge::SimRes;
use crate::input::actions::{Reticle, UiFocusRequest};
use crate::state::{FlyTo, InputActions, InputMode, MapMode, MapView, NoticeLevel, Selection, SimNotice, Speed, Target, ThreadFocus, TimeControl};
use deck::UiLayout;
use ink::{Ink, SealState};
use stream::EventFeed;
use text::EventText;
use theme::*;

/// L3 切換的地圖標籤顯示（5.9）。
#[derive(Resource)]
pub struct LabelsVisible(pub bool);

/// 介面字串（`assets/text/ui.zh-Hant.json`；GDD 10.5）。
#[derive(Resource)]
pub struct UiText(HashMap<String, String>);

impl UiText {
    fn load() -> Self {
        let raw = include_str!("../../assets/text/ui.zh-Hant.json");
        Self(serde_json::from_str(raw).expect("ui.zh-Hant.json 格式錯誤"))
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.0.get(key).map(String::as_str).unwrap_or(key)
    }
}

/// 命牌列的搖桿游標（資源，供輸入消費者與繪製共用）。
#[derive(Resource, Default)]
pub struct FatesNavRes(pub fates::FatesNav);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiText::load())
            .insert_resource(EventText::load())
            .init_resource::<EventFeed>()
            .init_resource::<FatesNavRes>()
            .init_resource::<ThreadFocus>()
            .init_resource::<UiLayout>()
            .insert_resource(LabelsVisible(true))
            .add_systems(Startup, dev_startup)
            .add_systems(Update, (deck::detect_layout, actions_consumer, dev_cursor))
            .add_systems(EguiContextPass, ui_system);
    }
}

/// 介面要做的事（由傳聞簽、命牌、情境印環、命簿與輸入消費者寫入；`apply_actions` 統一套用）。
#[derive(Default)]
pub struct UiActions {
    pub fly_to: Option<u32>,
    pub open_person: Option<PersonId>,
    pub open_event: Option<EventId>,
    pub open_province: Option<ProvinceId>,
    pub follow_toggle: Option<SoulId>,
    pub follow_many: Vec<SoulId>,
    /// 在因果模式中只顯示此魂的線（U12 消費 `ThreadFocus`）。
    pub show_threads: Option<SoulId>,
    pub mode_yinguo: bool,
    pub recenter: bool,
    pub close_ring: bool,
}

impl UiActions {
    /// 飛往事件地點；`open` 時連帶開啟相關命簿（人物 > 魂 > 州郡）。
    pub fn fly_to_event(&mut self, w: &World, e: &Event, open: bool) {
        self.fly_to = stream::event_cell(w, e);
        if open {
            if let Some(&pid) = e.persons.first() {
                self.open_person = Some(pid);
            } else if let Some(&sid) = e.souls.first() {
                self.open_soul(w, sid);
            } else if let Some(p) = e.province {
                self.open_province = Some(p);
            }
            if e.kind == siming_sim::EventKind::BookEntry || e.kind == siming_sim::EventKind::ThreadResolved || e.kind == siming_sim::EventKind::ThreadOmen {
                self.open_event = Some(e.id);
            }
        }
    }

    /// 開啟魂的卡：在世取今生，否則取最後一世（唯讀）。
    pub fn open_soul(&mut self, w: &World, sid: SoulId) {
        if let Some(s) = w.souls.get(sid as usize) {
            match s.state {
                SoulState::Incarnate(pid) => self.open_person = Some(pid),
                _ => self.open_person = s.lives.last().copied(),
            }
        }
    }
}

/// 統一套用介面動作（5.7「點了就到」）。
fn apply_actions(out: UiActions, sim: &mut SimRes, sel: &mut Selection, fly: &mut FlyTo, mode: &mut MapMode, focus: &mut ThreadFocus) {
    let world = sim.0.world();
    if let Some(eid) = out.open_event {
        let prov = siming_sim::ledger::get(world, eid).and_then(|e| e.province);
        sel.open(Target::Event(eid), prov);
    } else if let Some(pid) = out.open_person {
        let prov = world.persons.get(pid as usize).map(|p| p.province);
        sel.open(Target::Person(pid), prov);
    } else if let Some(p) = out.open_province {
        sel.open(Target::Province(p), Some(p));
    }
    if let Some(c) = out.fly_to {
        fly.0 = Some(c);
    }
    if out.recenter {
        let cell = match sel.target {
            Some(Target::Person(pid)) => world.persons.get(pid as usize).map(|p| p.cell),
            Some(Target::Event(eid)) => siming_sim::ledger::get(world, eid).and_then(|e| stream::event_cell(world, e)),
            Some(Target::Province(p)) => world.provinces.get(p as usize).map(|p| p.seat),
            None => None,
        };
        if cell.is_some() {
            fly.0 = cell;
        }
    }
    if let Some(sid) = out.show_threads {
        *mode = MapMode::Yinguo;
        focus.0 = Some(sid);
    }
    if out.mode_yinguo {
        *mode = MapMode::Yinguo;
    }
    if out.close_ring {
        sel.ring_open = false;
    }
    let mut toggles: Vec<SoulId> = out.follow_toggle.into_iter().collect();
    if !out.follow_many.is_empty() {
        // 關注雙方：全已關注則全部解除，否則把未關注者補上
        let all = out.follow_many.iter().all(|s| world.follows.contains(s));
        toggles.extend(out.follow_many.iter().copied().filter(|s| all || !world.follows.contains(s)));
    }
    for sid in toggles {
        if sid == siming_sim::HEAVEN_SOUL {
            continue;
        }
        let on = !sim.0.world().follows.contains(&sid);
        if on && sim.0.world().follows.len() >= 12 {
            continue;
        }
        // 指令於下一 tick 生效（6.5）；暫停時為了立即回饋，先直接寫入關注集合——
        // Follow 的套用是冪等的，重播時結果相同。
        sim.0.apply(Command::Follow { soul: sid, on });
        let w = sim.0.world_mut();
        if on {
            w.follows.insert(sid);
        } else {
            w.follows.remove(&sid);
        }
    }
}

/// 起始狀態：開局即以 1× 運行（4.1）；`SIMING_*` 供驗證截圖用。
#[allow(clippy::too_many_arguments)]
fn dev_startup(
    mut commands: Commands,
    mut sim: ResMut<SimRes>,
    mut view: ResMut<MapView>,
    mut tc: ResMut<TimeControl>,
    mut sel: ResMut<Selection>,
    mut mode: ResMut<MapMode>,
    mut feed: ResMut<EventFeed>,
    mut notice: ResMut<SimNotice>,
    mut fly: ResMut<FlyTo>,
    etext: Res<EventText>,
) {
    if std::env::var("SIMING_UI_TEST_CAMERA").is_ok() {
        let w = sim.0.world().geo.w as f32;
        view.camera_z = w * 0.25;
        commands.spawn((Camera3d::default(), Transform::from_xyz(w * 0.5, view.camera_z * 1.6, w * 0.5 + 1.0).looking_at(Vec3::new(w * 0.5, 0.0, w * 0.5), Vec3::NEG_Z)));
    }
    tc.paused = std::env::var("SIMING_PAUSED").is_ok();
    tc.speed = Speed::X1;
    if let Ok(s) = std::env::var("SIMING_SPEED") {
        tc.paused = false;
        tc.speed = match s.to_lowercase().trim_start_matches('x') {
            "5" => Speed::X5,
            "20" => Speed::X20,
            "100" => Speed::X100,
            _ => Speed::X1,
        };
    }
    if let Some(id) = std::env::var("SIMING_SELECT_PROVINCE").ok().and_then(|s| s.parse().ok()) {
        sel.open(Target::Province(id), Some(id));
    }
    match std::env::var("SIMING_MAP_MODE").as_deref() {
        Ok("tianxia") => *mode = MapMode::Tianxia,
        Ok("lingmai") => *mode = MapMode::Lingmai,
        _ => {}
    }
    // 驗證用：SIMING_PRERUN=<月數> 先推進模擬，讓傳聞簽與命簿有內容；SIMING_FOLLOW=<n> 關注前 n 位境界最高的在世修士；
    // SIMING_SELECT_PERSON=<第 k 位>、SIMING_SELECT_EVENT=latest、SIMING_RING=1、SIMING_NOTICE=1（以最新的高重要度事件模擬自動止）
    if let Some(n) = std::env::var("SIMING_PRERUN").ok().and_then(|s| s.parse::<u32>().ok()) {
        for _ in 0..n {
            sim.0.step();
        }
        let ids = sim.0.drain_events();
        let mut dummy_tc = TimeControl::default();
        let mut dummy_notice = SimNotice::default();
        stream::ingest(&mut feed, sim.0.world(), &ids, &mut dummy_tc, &mut dummy_notice, &etext, 0.0);
        feed.hot = None;
    }
    let notables = |w: &World| -> Vec<PersonId> {
        let mut v: Vec<&siming_sim::Person> = w.persons.iter().filter(|p| p.alive() && matches!(p.path, siming_sim::PathState::Cultivator { .. })).collect();
        v.sort_by(|a, b| {
            let r = |p: &siming_sim::Person| match p.path {
                siming_sim::PathState::Cultivator { realm, exp, .. } => realm as f32 * 100.0 + exp,
                _ => 0.0,
            };
            r(b).total_cmp(&r(a))
        });
        v.iter().map(|p| p.id).collect()
    };
    if let Some(n) = std::env::var("SIMING_FOLLOW").ok().and_then(|s| s.parse::<usize>().ok()) {
        let ids = notables(sim.0.world());
        for pid in ids.into_iter().take(n) {
            let soul = sim.0.world().persons[pid as usize].soul;
            sim.0.apply(Command::Follow { soul, on: true });
            sim.0.world_mut().follows.insert(soul);
        }
    }
    if let Some(k) = std::env::var("SIMING_SELECT_PERSON").ok().and_then(|s| s.parse::<usize>().ok()) {
        if let Some(&pid) = notables(sim.0.world()).get(k) {
            let prov = sim.0.world().persons[pid as usize].province;
            sel.open(Target::Person(pid), Some(prov));
        }
    }
    if std::env::var("SIMING_SELECT_EVENT").is_ok() {
        let w = sim.0.world();
        if let Some(e) = w.ledger.events.iter().rev().find(|e| e.importance >= 3 && !e.causes.is_empty()) {
            sel.open(Target::Event(e.id), e.province);
        }
    }
    if std::env::var("SIMING_RING").is_ok() {
        sel.ring_open = true;
    }
    // 驗證用：鏡頭飛到選取的主體
    if std::env::var("SIMING_FLY").is_ok() {
        let w = sim.0.world();
        fly.0 = match sel.target {
            Some(Target::Person(pid)) => w.persons.get(pid as usize).map(|p| p.cell),
            Some(Target::Event(eid)) => siming_sim::ledger::get(w, eid).and_then(|e| stream::event_cell(w, e)),
            Some(Target::Province(p)) => w.provinces.get(p as usize).map(|p| p.seat),
            None => notice.cell.or_else(|| w.follows.iter().next().and_then(|&s| fates::soul_cell(w, s))),
        };
    }
    if std::env::var("SIMING_NOTICE").is_ok() {
        let w = sim.0.world();
        if let Some(e) = w.ledger.events.iter().rev().find(|e| e.importance >= 4) {
            tc.paused = true;
            *notice = SimNotice { key: "", text: etext.line(w, e), level: NoticeLevel::Stop, cell: stream::event_cell(w, e), event: Some(e.id), extra: 0, remaining: f32::INFINITY };
            feed.hot = Some(e.id);
        }
    }
}

/// 僅供驗證截圖：`SIMING_TEST_CURSOR=x,y` 於第 40 幀把游標移到該處（停留提示、墨暈）。
fn dev_cursor(mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>, mut frame: Local<u32>) {
    *frame += 1;
    if *frame != 40 {
        return;
    }
    let Some((x, y)) = std::env::var("SIMING_TEST_CURSOR").ok().and_then(|s| {
        let (x, y) = s.split_once(',')?;
        Some((x.trim().parse::<f32>().ok()?, y.trim().parse::<f32>().ok()?))
    }) else {
        return;
    };
    if let Ok(mut w) = windows.single_mut() {
        w.set_cursor_position(Some(Vec2::new(x, y)));
        info!("SIMING_TEST_CURSOR：游標移至 ({x}, {y})");
    }
}

/// 速度階梯（5.9）：暫停 ↔ 1× ↔ 5× ↔ 20× ↔ 100×。`dir` > 0 加速、< 0 減速。
pub fn speed_step(tc: &mut TimeControl, dir: i32) {
    if dir > 0 {
        if tc.paused {
            tc.paused = false;
        } else if let Some(i) = Speed::ALL.iter().position(|s| *s == tc.speed) {
            if let Some(next) = Speed::ALL.get(i + 1) {
                tc.speed = *next;
            }
        }
    } else if !tc.paused {
        match tc.speed.slower() {
            Some(s) => tc.speed = s,
            None => tc.paused = true,
        }
    }
}

/// 在 M1 啟用的地圖模式中循環（LB／RB）。
pub fn cycle_mode(cur: MapMode, dir: i32) -> MapMode {
    let n = MapMode::M1.len() as i32;
    let i = MapMode::M1.iter().position(|m| *m == cur).unwrap_or(0) as i32;
    MapMode::M1[(i + dir).rem_euclid(n) as usize]
}

/// 動作層的消費者（GDD 5.2、5.7、5.9）：時間、地圖模式、標籤，以及互動層的鍵位——
/// Esc／B 回上一張、右鍵／X 情境印環、F／Y 關注、X 溯因、Enter 飛往、Tab 與 ◁▷ 傳聞、△ 命牌列。
#[allow(clippy::too_many_arguments)]
fn actions_consumer(
    act: Res<InputActions>,
    input: Res<InputMode>,
    mut tc: ResMut<TimeControl>,
    mut mode: ResMut<MapMode>,
    mut sel: ResMut<Selection>,
    mut labels: ResMut<LabelsVisible>,
    mut sim: ResMut<SimRes>,
    mut fly: ResMut<FlyTo>,
    mut feed: ResMut<EventFeed>,
    notice: Res<SimNotice>,
    mut nav: ResMut<FatesNavRes>,
    mut focus: ResMut<ThreadFocus>,
) {
    if act.pause_toggle_pressed {
        tc.paused = !tc.paused;
    }
    if let Some(s) = act.speed_set {
        tc.speed = s;
        tc.paused = false;
    }
    if act.speed_up_pressed {
        speed_step(&mut tc, 1);
    }
    if act.speed_down_pressed {
        speed_step(&mut tc, -1);
    }
    if let Some(m) = act.mode_set {
        if MapMode::M1.contains(&m) || m == MapMode::Yinguo {
            *mode = m;
        }
    }
    if act.mode_next_pressed {
        *mode = cycle_mode(*mode, 1);
    }
    if act.mode_prev_pressed {
        *mode = cycle_mode(*mode, -1);
    }
    if act.labels_toggle_pressed {
        labels.0 = !labels.0;
    }

    let mut out = UiActions::default();
    let world = sim.0.world();
    let pad = *input == InputMode::Gamepad;
    let notice_stop = notice.visible() && notice.level == NoticeLevel::Stop && notice.event.is_some();
    if act.back_pressed {
        if nav.0.active {
            nav.0.active = false;
        } else {
            sel.back();
        }
    }
    if act.context_pressed {
        if sel.target.is_some() {
            sel.ring_open = !sel.ring_open;
        } else if let Some(p) = sel.hovered {
            sel.open(Target::Province(p), Some(p));
            sel.ring_open = true;
        }
    }
    if act.follow_pressed {
        if nav.0.active && pad {
            fates::nav_input(&mut nav.0, world, 0, false, true, &mut out);
        } else if let Some(t) = sel.target {
            context::apply(context::ActionKind::Follow, world, t, &mut out);
        }
    }
    if act.trace_pressed {
        if let Some(t) = sel.target {
            context::apply(context::ActionKind::Trace, world, t, &mut out);
        }
    }
    // 原因簽：Enter（鍵鼠）或 A（搖桿，無卡片時）飛往
    if act.goto_notice_pressed || (pad && notice_stop && act.select_pressed && sel.target.is_none() && !nav.0.active) {
        if let Some(e) = notice.event.and_then(|id| siming_sim::ledger::get(world, id)) {
            out.fly_to_event(world, e, true);
            feed.hot = None;
        }
    }
    if act.rumor_toggle_pressed {
        if pad {
            // 十字鍵 △：先進命牌列，再按一次展開傳聞清單；▽ 關閉
            if !nav.0.active && !feed.expanded && !world.follows.is_empty() {
                nav.0.active = true;
            } else if nav.0.active && !feed.expanded {
                nav.0.active = false;
                feed.expanded = true;
            } else {
                feed.expanded = false;
                nav.0.active = false;
            }
        } else {
            feed.expanded = !feed.expanded;
        }
    }
    if nav.0.active && pad {
        if act.rumor_prev_pressed {
            fates::nav_input(&mut nav.0, world, -1, false, false, &mut out);
        }
        if act.rumor_next_pressed {
            fates::nav_input(&mut nav.0, world, 1, false, false, &mut out);
        }
        if act.select_pressed {
            fates::nav_input(&mut nav.0, world, 0, true, false, &mut out);
        }
    } else {
        if act.rumor_prev_pressed {
            stream::step_cursor(&mut feed, world, -1, &mut out);
        }
        if act.rumor_next_pressed {
            stream::step_cursor(&mut feed, world, 1, &mut out);
        }
    }
    if act.recenter_pressed && pad {
        out.recenter = false; // 鏡頭系統已處理 R3
    }
    apply_actions(out, &mut sim, &mut sel, &mut fly, &mut mode, &mut focus);
}

/// 介面動畫與隨需圖例的跨幀狀態（5.7）。
#[derive(Default)]
struct HudState {
    themed: bool,
    applied_layout: Option<UiLayout>,
    paper: Option<egui::TextureHandle>,
    /// 最後一次開啟的主體（捲回動畫仍需要它）。
    last_target: Option<Target>,
    legend_pinned: bool,
    legend_mode: Option<MapMode>,
    legend_changed_at: f64,
    hover_id: Option<ProvinceId>,
    hover_since: f64,
    /// 原因簽目前事件與開始時間（朱環 2 秒）。
    notice_event: Option<EventId>,
    notice_since: f64,
    /// 引導簽（4.1 節拍）：0 未開始；1 已看過人物卡；2 已關注過。
    guide_step: u8,
    first_frame_at: Option<f64>,
}

/// 停留提示延遲（5.9）。
const HOVER_DELAY: f64 = 0.3;
/// 桌機印匣高度（印 38 + 邊距）。
const DESK_TRAY_H: f32 = 50.0;

/// 卡片內容的版面選項：Deck 抽屜（字級 ≥ 16）與搖桿模式（提示顯示 B 而非 Esc）。
#[derive(Clone, Copy)]
pub(crate) struct CardOpts {
    pub deck: bool,
    pub pad: bool,
}

/// `ui_system` 讀取的資源（Bevy 系統參數上限 16，故分組）。
#[derive(bevy::ecs::system::SystemParam)]
struct UiRead<'w, 's> {
    text: Res<'w, UiText>,
    etext: Res<'w, EventText>,
    view: Res<'w, MapView>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<Camera3d>>,
    layout: Res<'w, UiLayout>,
    input: Res<'w, InputMode>,
    reticle: Res<'w, Reticle>,
    labels_visible: Res<'w, LabelsVisible>,
}

/// `ui_system` 寫入的狀態。
#[derive(bevy::ecs::system::SystemParam)]
struct UiWrite<'w> {
    sim: ResMut<'w, SimRes>,
    notice: ResMut<'w, SimNotice>,
    tc: ResMut<'w, TimeControl>,
    mode: ResMut<'w, MapMode>,
    sel: ResMut<'w, Selection>,
    fly: ResMut<'w, FlyTo>,
    feed: ResMut<'w, EventFeed>,
    nav: ResMut<'w, FatesNavRes>,
    focus: ResMut<'w, ThreadFocus>,
    focus_req: ResMut<'w, UiFocusRequest>,
}

fn ui_system(mut contexts: EguiContexts, mut hud: Local<HudState>, r: UiRead, w: UiWrite) {
    let UiRead { text, etext, view, cameras, layout, input, reticle, labels_visible } = r;
    let UiWrite { mut sim, mut notice, mut tc, mut mode, mut sel, mut fly, mut feed, mut nav, mut focus, mut focus_req } = w;
    let (text, etext, view, layout, input, reticle, labels_visible) = (&*text, &*etext, &*view, &*layout, &*input, &*reticle, &*labels_visible);
    let ctx = contexts.ctx_mut();
    if !hud.themed {
        theme::apply(ctx);
        hud.themed = true;
        hud.applied_layout = None;
    }
    let layout: UiLayout = *layout;
    if hud.applied_layout != Some(layout) {
        deck::apply_text_sizes(ctx, layout);
        hud.applied_layout = Some(layout);
    }
    let paper = hud.paper.get_or_insert_with(|| {
        ctx.load_texture("ui_paper", ink::paper_tile(), egui::TextureOptions { wrap_mode: egui::TextureWrapMode::Repeat, ..egui::TextureOptions::LINEAR })
    });
    let deck = layout == UiLayout::Deck;
    let paper_id = paper.id();
    let ink = Ink { paper: paper_id, scale: if deck { 1.15 } else { 1.0 } };
    let now = ctx.input(|i| i.time);
    if hud.first_frame_at.is_none() {
        hud.first_frame_at = Some(now);
    }
    let pad = *input == InputMode::Gamepad;
    let mut out = UiActions::default();
    let camera = cameras.single().ok();

    // ── 左上：天時儀、題簽、原因簽 ──
    let instruments_rect = {
        let world = sim.0.world();
        instruments(ctx, &ink, text, world, world.tick, &mut tc, &mut notice, &mut feed, layout, pad, &mut out)
    };
    if notice.event != hud.notice_event {
        hud.notice_event = notice.event;
        hud.notice_since = now;
    }

    // ── 命簿：主體改變時展卷／捲回（5.8） ──
    if sel.target.is_some() {
        hud.last_target = sel.target;
    }
    let open = sel.target.is_some();
    let t = ctx.animate_bool_with_time(Id::new("card_t"), open, if open { 0.25 } else { 0.18 });
    let card_target = if t > 0.001 { hud.last_target } else { None };
    if let Some(Target::Person(_)) = sel.target {
        if hud.guide_step == 0 {
            hud.guide_step = 1;
        }
    }
    if hud.guide_step == 1 && !sim.0.world().follows.is_empty() {
        hud.guide_step = 2;
    }
    let prompt_ctx = if nav.0.active && pad {
        prompts::PromptCtx::Fates
    } else if open {
        prompts::PromptCtx::Card
    } else if notice.visible() && notice.level == NoticeLevel::Stop {
        prompts::PromptCtx::Notice
    } else {
        prompts::PromptCtx::Map
    };
    let mut close = false;
    let mut card_rect = egui::Rect::NOTHING;
    let tray_rect;
    let stream_rect;
    let fates_rect;
    let opts = CardOpts { deck, pad };
    let fates_top = instruments_rect.bottom() + ink.px(18.0);
    if deck {
        if let Some(target) = card_target {
            close = deck::card_drawer(ctx, &ink, t, open, |ui| card_body_for(ui, &ink, text, etext, sim.0.world(), target, opts, *input, layout, &mut focus_req.0, &mut out));
        }
        let avail = ctx.available_rect();
        tray_rect = mode_tray(ctx, &ink, text, &mut mode, layout, *input, &mut hud, avail);
        let limit = avail.bottom() - deck::DECK_TRAY_H - 30.0;
        stream_rect = stream::draw(ctx, &ink, text, etext, sim.0.world(), &mut feed, layout, *input, avail, limit, &mut out);
        fates_rect = fates::draw(ctx, &ink, text, sim.0.world(), &feed, &mut nav.0, layout, *input, fates_top, &mut out);
        prompts::prompt_bar(ctx, &ink, text, *input, layout, prompt_ctx, avail, deck::DECK_TRAY_H + 20.0);
    } else {
        let avail = ctx.screen_rect();
        tray_rect = mode_tray(ctx, &ink, text, &mut mode, layout, *input, &mut hud, avail);
        let lift = if pad { 56.0 } else { 0.0 };
        if let Some(target) = card_target {
            let (c, r) = desk_card(ctx, &ink, t, open, lift, |ui| card_body_for(ui, &ink, text, etext, sim.0.world(), target, opts, *input, layout, &mut focus_req.0, &mut out));
            close = c;
            card_rect = r;
        }
        let limit = avail.bottom() - DESK_TRAY_H - 30.0;
        stream_rect = stream::draw(ctx, &ink, text, etext, sim.0.world(), &mut feed, layout, *input, avail, limit, &mut out);
        fates_rect = fates::draw(ctx, &ink, text, sim.0.world(), &feed, &mut nav.0, layout, *input, fates_top, &mut out);
        prompts::prompt_bar(ctx, &ink, text, *input, layout, prompt_ctx, avail, DESK_TRAY_H + 20.0);
    }
    if close {
        sel.back();
    }
    let avoid = [instruments_rect.expand(8.0), tray_rect.expand(8.0), stream_rect.expand(8.0), fates_rect.expand(8.0), card_rect.expand(8.0)];

    // ── 地圖上的疊層：溯因事件印與赤線、朱環、情境印環 ──
    {
        let world = sim.0.world();
        if let Some(Target::Event(eid)) = sel.target {
            cards::draw_trace_overlay(ctx, &ink, etext, world, view, camera, eid, &avoid);
        }
        if let Some(e) = notice.event.filter(|_| notice.visible()).and_then(|id| siming_sim::ledger::get(world, id)) {
            // 止級且仍暫停時朱環持續脈動（玩家還沒去看）；否則 2 秒
            let age = if notice.level == NoticeLevel::Stop && tc.paused { ((now - hud.notice_since) as f32).rem_euclid(2.0) } else { (now - hud.notice_since) as f32 };
            cards::draw_alert_rings(ctx, world, view, camera, e, age);
        }
        let ring_t = ctx.animate_bool_with_time(Id::new("ring_t"), sel.ring_open && sel.target.is_some(), 0.15);
        if ring_t > 0.001 {
            if let Some(target) = sel.target.or(hud.last_target) {
                let cell = match target {
                    Target::Person(pid) => world.persons.get(pid as usize).map(|p| p.cell),
                    Target::Province(p) => world.provinces.get(p as usize).map(|p| p.seat),
                    Target::Event(eid) => siming_sim::ledger::get(world, eid).and_then(|e| stream::event_cell(world, e)),
                };
                let avail = ctx.available_rect();
                let center = cell
                    .and_then(|c| camera.and_then(|(cam, tf)| labels::cell_to_screen(world, view, cam, tf, c, avail)))
                    .unwrap_or(avail.center());
                context::ring(ctx, &ink, text, world, target, center, *input, layout, ring_t, &mut out);
            }
        }
    }

    // ── 引導簽（4.1 節拍；M7 完整實作） ──
    {
        let elapsed = now - hud.first_frame_at.unwrap_or(now);
        let key = match hud.guide_step {
            0 if elapsed > 20.0 && !open => Some("guide_1"),
            1 if open => Some("guide_2"),
            _ => None,
        };
        let want = key.is_some() && std::env::var("SIMING_NO_GUIDE").is_err();
        let gt = ctx.animate_bool_with_time(Id::new("guide_t"), want, 0.3);
        if gt > 0.001 {
            let k = key.unwrap_or(if hud.guide_step == 0 { "guide_1" } else { "guide_2" });
            guide_slip(ctx, &ink, text.t(k), layout, gt, if deck { deck::DECK_TRAY_H } else { DESK_TRAY_H });
        }
    }

    // ── 地圖標籤（最底層），避開器物 ──
    if labels_visible.0 {
        labels::draw_labels(ctx, &sim, view, *mode, camera, &avoid);
    }

    // ── 州郡停留提示 ──
    let hovered = sel.hovered.filter(|_| !ctx.is_pointer_over_area());
    if hud.hover_id != hovered {
        hud.hover_id = hovered;
        hud.hover_since = now;
    }
    if let Some(id) = hovered {
        if now - hud.hover_since >= HOVER_DELAY {
            if let (Some(p), Some(pos)) = (sim.0.world().provinces.get(id as usize), ctx.pointer_hover_pos()) {
                ink::tooltip(ctx, &ink, Id::new("hover_tip").with(id), egui::Rect::from_min_size(pos, egui::Vec2::ZERO), &p.name, true);
            }
        }
    }
    prompts::draw_reticle(ctx, reticle);

    apply_actions(out, &mut sim, &mut sel, &mut fly, &mut mode, &mut focus);
}

/// 依主體選擇卡片內容。
#[allow(clippy::too_many_arguments)]
fn card_body_for(ui: &mut egui::Ui, ink: &Ink, text: &UiText, etext: &EventText, world: &World, target: Target, opts: CardOpts, input: InputMode, layout: UiLayout, focus_req: &mut bool, out: &mut UiActions) -> bool {
    match target {
        Target::Province(id) => match world.provinces.get(id as usize) {
            Some(p) => province_body(ui, ink, text, etext, world, p, opts, input, layout, focus_req, out),
            None => true,
        },
        Target::Person(pid) => cards::person_body(ui, ink, text, etext, world, pid, opts, input, layout, focus_req, out),
        Target::Event(eid) => cards::event_body(ui, ink, text, etext, world, eid, opts, input, layout, focus_req, out),
    }
}

/// 左上角器物：天時儀、題簽（日期）、原因簽（5.7）。不寫產品名（7.9 #36）。
#[allow(clippy::too_many_arguments)]
fn instruments(
    ctx: &egui::Context,
    ink: &Ink,
    text: &UiText,
    world: &World,
    tick: u32,
    tc: &mut TimeControl,
    notice: &mut SimNotice,
    feed: &mut EventFeed,
    layout: UiLayout,
    pad: bool,
    out: &mut UiActions,
) -> egui::Rect {
    let deck = layout == UiLayout::Deck;
    let r = egui::Area::new(Id::new("instruments")).order(Order::Middle).anchor(Align2::LEFT_TOP, egui::vec2(12.0, 10.0)).show(ctx, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(ink.px(10.0), ink.px(6.0));
        ui.horizontal_top(|ui| {
            let month = calendar::date(tick).month.saturating_sub(1) % 12;
            let idx = Speed::ALL.iter().position(|s| *s == tc.speed).unwrap_or(0);
            let tips: [String; 4] = std::array::from_fn(|i| format!("{}  {}", Speed::ALL[i].label(), i + 1));
            let pause_tip = format!("{}  Space", text.t(if tc.paused { "resume" } else { "pause" }));
            let label = if tc.paused { text.t("pause").to_owned() } else { tc.speed.label().to_owned() };
            let dial = ink::time_dial(ui, ink, month, tc.paused, idx, (text.t("dial_run"), text.t("dial_stop")), &pause_tip, &tips, &label);
            if dial.toggle_pause {
                tc.paused = !tc.paused;
            }
            if let Some(i) = dial.set_speed {
                tc.speed = Speed::ALL[i];
                tc.paused = false;
            }
            ui.vertical(|ui| {
                ink::title_slip(ui, ink, &calendar::short(tick), if deck { 18.0 } else { 17.0 }, 1.0);
                let show = notice.visible();
                let t = ui.ctx().animate_bool_with_time(Id::new("notice_t"), show, 0.2);
                if t > 0.001 {
                    reason_slip(ui, ink, text, world, notice, feed, tc, t, deck, pad, out);
                }
            });
        });
    });
    r.response.rect
}

/// 原因簽（5.7）：朱字「止」／「緩」＋一句話＋〔飛往〕〔續行〕；系統提示（降速）只有文字。
#[allow(clippy::too_many_arguments)]
fn reason_slip(ui: &mut egui::Ui, ink: &Ink, text: &UiText, world: &World, notice: &mut SimNotice, feed: &mut EventFeed, tc: &mut TimeControl, a: f32, deck: bool, pad: bool, out: &mut UiActions) {
    let font = ink.px(14.0).max(if deck { 16.0 } else { 0.0 });
    let fid = egui::FontId::proportional(font);
    let body = if notice.text.is_empty() { text.t(notice.key).to_string() } else { notice.text.clone() };
    let body = if notice.extra > 0 { format!("{body} ＋{}", notice.extra) } else { body };
    let level = match notice.level {
        NoticeLevel::Stop => Some(text.t("notice_stop")),
        NoticeLevel::Slow => Some(text.t("notice_slow")),
        NoticeLevel::Info => None,
    };
    let g = ui.painter().layout_no_wrap(body, fid.clone(), INK);
    let maxw = ink.px(400.0);
    let g = if g.size().x > maxw {
        let keep = ((g.text().chars().count() as f32) * (maxw / g.size().x)).floor().max(2.0) as usize - 1;
        ui.painter().layout_no_wrap(g.text().chars().take(keep).chain(std::iter::once('…')).collect(), fid.clone(), INK)
    } else {
        g
    };
    let pad_v = egui::vec2(ink.px(12.0), ink.px(6.0));
    let lvl_w = if level.is_some() { font * 1.3 } else { 0.0 };
    let has_acts = notice.event.is_some() && notice.level != NoticeLevel::Info;
    let small = egui::FontId::proportional(ink.px(12.0).max(if deck { 16.0 } else { 0.0 }));
    let act_defs = [("notice_resume", if pad { "☰" } else { "Space" }), ("notice_goto", if pad { "A" } else { "Enter" })];
    let act_galleys: Vec<std::sync::Arc<egui::Galley>> = if has_acts {
        act_defs.iter().map(|(lbl, key)| ui.painter().layout_no_wrap(format!("{} {}", key, text.t(lbl)), small.clone(), INK_LIGHT)).collect()
    } else {
        Vec::new()
    };
    let acts_w = act_galleys.iter().map(|g| g.size().x + ink.px(12.0)).sum::<f32>();
    let size = egui::vec2(g.size().x + pad_v.x * 2.0 + ink.px(8.0) + lvl_w + acts_w, g.size().y.max(font) + pad_v.y * 2.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter();
    ink::slip(p, rect, ink, a);
    let x = rect.left() + ink.px(6.0);
    ink::brush_rule(p, egui::pos2(x, rect.top() + ink.px(5.0)), egui::pos2(x, rect.bottom() - ink.px(5.0)), 2.5, SEAL_RED.gamma_multiply(a), 7);
    let mut tx = rect.left() + pad_v.x + ink.px(8.0);
    if let Some(l) = level {
        p.text(egui::pos2(tx, rect.center().y), Align2::LEFT_CENTER, l, egui::FontId::proportional(font * 1.1), SEAL_RED.gamma_multiply(a));
        tx += lvl_w;
    }
    p.galley(egui::pos2(tx, rect.center().y - g.size().y * 0.5), g, INK.gamma_multiply(a));
    if has_acts {
        let mut ax = rect.right() - pad_v.x;
        for (i, gg) in act_galleys.into_iter().enumerate() {
            ax -= gg.size().x;
            let r = egui::Rect::from_min_size(egui::pos2(ax, rect.center().y - gg.size().y * 0.5), gg.size());
            let resp = ui.interact(r.expand(3.0), Id::new("notice_act").with(i), egui::Sense::click());
            if resp.hovered() || ink::focused(ui.ctx(), &resp) {
                ui.painter().rect_filled(r.expand(3.0), 2.0, SEAL_RED.gamma_multiply(0.12));
            }
            ui.painter().galley(r.min, gg, INK_LIGHT.gamma_multiply(a));
            ax -= ink.px(12.0);
            if resp.clicked() {
                if i == 0 {
                    tc.paused = false;
                    stream::resume(notice);
                } else if let Some(e) = notice.event.and_then(|id| siming_sim::ledger::get(world, id)) {
                    out.fly_to_event(world, e, true);
                    feed.hot = None;
                }
            }
        }
    }
}

/// 引導簽：畫面下方中央一句話（5.7）。
fn guide_slip(ctx: &egui::Context, ink: &Ink, line: &str, layout: UiLayout, a: f32, tray_h: f32) {
    let deck = layout == UiLayout::Deck;
    let font = ink.px(15.0).max(if deck { 16.0 } else { 0.0 });
    egui::Area::new(Id::new("guide_slip"))
        .order(Order::Middle)
        .pivot(Align2::CENTER_BOTTOM)
        .fixed_pos(egui::pos2(ctx.available_rect().center().x, ctx.available_rect().bottom() - 10.0 - tray_h - 24.0))
        .interactable(false)
        .show(ctx, |ui| {
            ink::title_slip(ui, ink, line, font, a);
        });
}

/// 地圖模式一字印（9.8）。
fn mode_char(m: MapMode) -> &'static str {
    match m {
        MapMode::Shanhe => "地",
        MapMode::Tianxia => "國",
        MapMode::Sanjiao => "教",
        MapMode::Lingmai => "脈",
        MapMode::Yinguo => "因",
        MapMode::Yeqi => "業",
    }
}

/// 右下角印匣：例簽 + 六枚地圖模式印；Deck 版面只列 M1 模式與「因」並帶 LB／RB 提示（5.7、5.9）。
#[allow(clippy::too_many_arguments)]
fn mode_tray(ctx: &egui::Context, ink: &Ink, text: &UiText, mode: &mut MapMode, layout: UiLayout, input: InputMode, hud: &mut HudState, avail: egui::Rect) -> egui::Rect {
    let deck = layout == UiLayout::Deck;
    let size = if deck { 44.0 } else { 38.0 };
    let now = ctx.input(|i| i.time);
    if hud.legend_mode != Some(*mode) {
        hud.legend_mode = Some(*mode);
        hud.legend_changed_at = now;
    }
    let mut legend_anchor = egui::Rect::NOTHING;
    let mut legend_hot = false;
    let tray = egui::Area::new(Id::new("mode_tray")).order(Order::Middle).pivot(Align2::RIGHT_BOTTOM).fixed_pos(avail.right_bottom() - egui::vec2(12.0, 10.0)).show(ctx, |ui| {
        ui.spacing_mut().item_spacing.x = ink.px(6.0);
        ui.horizontal(|ui| {
            let r = ink::seal(ui, text.t("legend_seal"), false, if hud.legend_pinned { SealState::Active } else { SealState::Idle }, size * 0.8);
            if r.clicked() {
                hud.legend_pinned = !hud.legend_pinned;
            }
            legend_hot = r.hovered() || ink::focused(ctx, &r);
            legend_anchor = r.rect;
            ui.add_space(ink.px(6.0));
            if deck {
                prompts::glyph(ui, &if input == InputMode::Gamepad { prompts::Glyph::Lb } else { prompts::Glyph::key("F1") }, layout);
            }
            let modes: &[MapMode] = if deck {
                &[MapMode::Shanhe, MapMode::Tianxia, MapMode::Lingmai, MapMode::Yinguo]
            } else {
                &[MapMode::Shanhe, MapMode::Tianxia, MapMode::Sanjiao, MapMode::Lingmai, MapMode::Yinguo, MapMode::Yeqi]
            };
            for m in modes {
                // 因果模式的印在 M2 可按（情境印「因」會切到它）；渲染端的線與魂光由 U12 接上
                let enabled = MapMode::M1.contains(m) || *m == MapMode::Yinguo;
                let state = if *mode == *m {
                    SealState::Active
                } else if enabled {
                    SealState::Idle
                } else {
                    SealState::Disabled
                };
                let r = ink::seal(ui, mode_char(*m), false, state, size);
                let key = ["F1", "F2", "F3", "F4", "F5", "F6"][*m as usize];
                let tip = if enabled { format!("{}  {}", m.name_zh(), key) } else { format!("{}  {}", m.name_zh(), text.t("mode_disabled_hint")) };
                ink::hover_tip(ctx, ink, &r, &tip);
                if r.clicked() && enabled {
                    *mode = *m;
                }
            }
            if deck {
                prompts::glyph(ui, &if input == InputMode::Gamepad { prompts::Glyph::Rb } else { prompts::Glyph::key("F5") }, layout);
            }
        });
    });
    let want = legend_hot || hud.legend_pinned || now - hud.legend_changed_at < 4.0;
    let t = ctx.animate_bool_with_time(Id::new("legend_t"), want, 0.2);
    if t > 0.001 {
        legend_slip(ctx, ink, text, *mode, legend_anchor, t);
    }
    tray.response.rect
}

/// 圖例紙簽：浮在例簽上方，淡入淡出（5.4、5.7）。
fn legend_slip(ctx: &egui::Context, ink: &Ink, text: &UiText, mode: MapMode, anchor: egui::Rect, t: f32) {
    let small = ink.px(13.0).max(if ink.scale > 1.0 { 16.0 } else { 0.0 });
    egui::Area::new(Id::new("legend_slip"))
        .order(Order::Middle)
        .pivot(Align2::LEFT_BOTTOM)
        .fixed_pos(egui::pos2(anchor.left(), anchor.top() - ink.px(14.0)))
        .interactable(false)
        .show(ctx, |ui| {
            let bg = ui.painter().add(egui::Shape::Noop);
            let pad = egui::vec2(ink.px(12.0), ink.px(8.0));
            let inner = ui
                .scope_builder(egui::UiBuilder::new().max_rect(ui.max_rect().translate(pad)), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(ink.px(5.0), ink.px(4.0));
                    ui.horizontal(|ui| match mode {
                        MapMode::Shanhe => {
                            for terr in Terrain::ALL {
                                ink::swatch(ui, ink, terrain_color(terr).gamma_multiply(t));
                                ui.label(RichText::new(terr.name_zh()).size(small).color(INK.gamma_multiply(t)));
                                ui.add_space(ink.px(4.0));
                            }
                        }
                        MapMode::Tianxia => {
                            ui.label(RichText::new(text.t("legend_tianxia")).size(small).color(INK.gamma_multiply(t)));
                        }
                        MapMode::Lingmai => {
                            ui.label(RichText::new(text.t("legend_lingmai")).size(small).color(INK.gamma_multiply(t)));
                            ui.add_space(ink.px(4.0));
                            for (level, name) in FIELD_INK_LEVELS.iter().zip(FIELD_INK_NAMES) {
                                ink::swatch(ui, ink, aura_ramp(*level).gamma_multiply(t));
                                ui.label(RichText::new(name).size(small).color(INK.gamma_multiply(t)));
                                ui.add_space(ink.px(2.0));
                            }
                        }
                        MapMode::Yinguo => {
                            for (k, name) in [(siming_sim::ThreadKind::Feud, "殺業"), (siming_sim::ThreadKind::Grace, "恩義"), (siming_sim::ThreadKind::Love, "情緣"), (siming_sim::ThreadKind::Mentor, "師徒"), (siming_sim::ThreadKind::Grudge, "怨")] {
                                ink::swatch(ui, ink, cards::thread_color(k).gamma_multiply(t));
                                ui.label(RichText::new(name).size(small).color(INK.gamma_multiply(t)));
                                ui.add_space(ink.px(4.0));
                            }
                        }
                        _ => {
                            ui.label(RichText::new(text.t("mode_disabled_hint")).size(small).color(INK_LIGHT.gamma_multiply(t)));
                        }
                    });
                })
                .response
                .rect;
            let rect = egui::Rect::from_min_max(inner.min - pad, inner.max + pad);
            ui.painter().set(bg, egui::Shape::Vec(ink::slip_shapes(rect, ink, t)));
            ui.allocate_rect(rect, egui::Sense::hover());
        });
}

/// 人口格式：滿萬以「萬」計，一位小數；否則取整數。
fn fmt_count(v: f64, wan: &str) -> String {
    if v >= 10_000.0 {
        format!("{:.1}{}", v / 10_000.0, wan)
    } else {
        format!("{}", v.round() as u64)
    }
}

/// 桌機版面的命簿：浮在右下的卷軸（GDD 5.8）。`t` 為展卷量；回傳 `true` 表示使用者要求關閉。
fn desk_card(ctx: &egui::Context, ink: &Ink, t: f32, open: bool, lift: f32, body: impl FnOnce(&mut egui::Ui) -> bool) -> (bool, egui::Rect) {
    let mut close = false;
    let width = 380.0;
    let h = ink::scroll_height(ctx, Id::new("desk_card_body"), ink);
    let screen = ctx.screen_rect();
    // 卷軸最高到傳聞簽下方（約 110 px），內容超出時軸頭固定、內頁可捲
    let max_h = screen.height() - 110.0 - 10.0 - DESK_TRAY_H - lift;
    let h = h.min(max_h);
    let y = screen.bottom() - 10.0 - DESK_TRAY_H - lift - h;
    let inner_max = max_h - ink.px(14.0) * 2.0 - ink.px(6.0) * 2.0 - ink.px(12.0) * 2.0;
    let r = egui::Area::new(Id::new("desk_card")).order(Order::Middle).fixed_pos(egui::pos2(screen.right() - 12.0 - width, y)).interactable(open).show(ctx, |ui| {
        ink::scroll_body(ui, ink, Id::new("desk_card_body"), t, width, |ui| {
            egui::ScrollArea::vertical().max_height(inner_max).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                close = body(ui);
            });
        });
    });
    (close, r.response.rect)
}

/// 州郡卡內容（M1 欄位 + M2 的有名之人與近事）。
#[allow(clippy::too_many_arguments)]
fn province_body(ui: &mut egui::Ui, ink: &Ink, text: &UiText, etext: &EventText, world: &World, p: &Province, opts: CardOpts, input: InputMode, layout: UiLayout, focus_req: &mut bool, out: &mut UiActions) -> bool {
    let CardOpts { deck, pad } = opts;
    let mut close = false;
    let body = ink.px(15.0).max(if deck { 16.0 } else { 0.0 });
    ui.spacing_mut().item_spacing = egui::vec2(ink.px(6.0), ink.px(4.0));
    ui.horizontal(|ui| {
        let now = ui.ctx().input(|i| i.time);
        let shown = ui.ctx().data_mut(|d| *d.get_temp_mut_or(Id::new("card_title").with(p.id), now));
        let a = (((now - shown) / 0.15).clamp(0.0, 1.0) as f32).max(0.15);
        ui.label(RichText::new(&p.name).size(if deck { 22.0 } else { 20.0 }).color(INK.gamma_multiply(a)));
        ui.label(RichText::new(text.t(&format!("city_level_{}", p.city_level.min(4)))).size(body).color(INK_LIGHT.gamma_multiply(a)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let r = ink::seal(ui, text.t("card_close_seal"), true, SealState::Idle, ink.px(26.0));
            if *focus_req {
                r.request_focus();
                *focus_req = false;
            }
            ink::hover_tip(ui.ctx(), ink, &r, &format!("{}  {}", text.t("card_close"), if pad { "B" } else { "Esc" }));
            if r.clicked() {
                close = true;
            }
        });
    });
    ink::title_rule(ui, ink);
    ui.add_space(ink.px(2.0));
    ink::section(ui, ink, text.t("card_terrain"));
    let total: u32 = p.terrain_mix.iter().sum();
    let mut mix: Vec<(Terrain, u32)> = Terrain::ALL.iter().map(|&t| (t, p.terrain_mix[t as usize])).filter(|x| x.1 > 0).collect();
    mix.sort_by_key(|x| std::cmp::Reverse(x.1));
    ui.horizontal_wrapped(|ui| {
        for (t, n) in mix.into_iter().take(4) {
            ink::swatch(ui, ink, terrain_color(t));
            ui.label(RichText::new(format!("{} {:.0}%", t.name_zh(), n as f32 * 100.0 / total.max(1) as f32)).size(body).color(INK));
            ui.add_space(ink.px(6.0));
        }
    });
    ui.add_space(ink.px(4.0));
    let wan = text.t("unit_wan");
    ink::section(ui, ink, text.t("card_people"));
    ink::ink_meter(ui, ink, text.t("card_population"), (p.population / p.capacity.max(1.0)) as f32, GOLD, &format!("{} / {}", fmt_count(p.population, wan), fmt_count(p.capacity, wan)));
    ink::ink_meter(ui, ink, text.t("card_aura"), p.aura, CYAN, &format!("{}", (p.aura * 100.0).round() as i32));
    cards::province_extra(ui, ink, text, etext, world, p, opts, input, layout, out);
    close
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_ladder_includes_pause() {
        let mut tc = TimeControl { paused: false, speed: Speed::X1 };
        speed_step(&mut tc, -1);
        assert!(tc.paused);
        speed_step(&mut tc, -1);
        assert!(tc.paused);
        speed_step(&mut tc, 1);
        assert!(!tc.paused && tc.speed == Speed::X1);
        for want in [Speed::X5, Speed::X20, Speed::X100, Speed::X100] {
            speed_step(&mut tc, 1);
            assert_eq!(tc.speed, want);
        }
        speed_step(&mut tc, -1);
        assert_eq!(tc.speed, Speed::X20);
    }

    #[test]
    fn mode_cycle_wraps_over_m1() {
        assert_eq!(cycle_mode(MapMode::Shanhe, -1), MapMode::Lingmai);
        assert_eq!(cycle_mode(MapMode::Lingmai, 1), MapMode::Shanhe);
        assert_eq!(cycle_mode(MapMode::Shanhe, 1), MapMode::Tianxia);
    }

    #[test]
    fn mode_seals_are_distinct_single_chars() {
        let all = [MapMode::Shanhe, MapMode::Tianxia, MapMode::Sanjiao, MapMode::Lingmai, MapMode::Yinguo, MapMode::Yeqi];
        let chars: Vec<&str> = all.iter().map(|m| mode_char(*m)).collect();
        assert!(chars.iter().all(|c| c.chars().count() == 1));
        let mut dedup = chars.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(dedup.len(), chars.len());
    }

    #[test]
    fn ui_text_has_no_product_name() {
        let t = UiText::load();
        assert!(!t.0.contains_key("product_name"), "產品名不是遊戲內容，不進介面字串（7.9 #36）");
        for k in ["rumor_title", "fates_title", "notice_stop", "act_follow", "card_trace"] {
            assert!(t.0.contains_key(k), "{k}");
        }
    }

    #[test]
    fn selection_back_pops_history_then_closes() {
        let mut s = Selection::default();
        s.open(Target::Province(1), Some(1));
        s.open(Target::Person(7), Some(2));
        s.ring_open = true;
        s.back();
        assert!(!s.ring_open && s.target == Some(Target::Person(7)));
        s.back();
        assert_eq!(s.target, Some(Target::Province(1)));
        assert_eq!(s.selected, Some(1));
        s.back();
        assert!(s.target.is_none() && s.selected.is_none());
    }
}
