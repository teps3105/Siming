//! GDD 5.9 動作層（T14）：鍵鼠、搖桿、觸控三個 writer 每幀寫入 `InputActions`，
//! 並依最後使用的裝置切換 `InputMode`；搖桿模式的準星與吸附也在此。
//! 其他系統（鏡頭、選取、介面快捷鍵）只讀 `InputActions`，不讀原始按鍵。
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::input::InputSystem;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui;

use crate::state::{InputActions, InputMode, MapMode, Selection, Speed};

/// 搖桿徑向死區（5.9）。
pub const DEADZONE: f32 = 0.15;
/// 推滿時每秒平移的畫面寬倍數（5.9）。
pub const PAN_FULL_WIDTHS_PER_SEC: f32 = 0.8;
/// 推滿時每秒縮放的高度倍數（5.9：2 倍）。
pub const ZOOM_FULL_FACTOR_PER_SEC: f32 = 2.0;
/// 滾輪一刻度的縮放係數（5.2），搖桿／觸控的縮放換算成同一單位。
pub const ZOOM_NOTCH: f32 = 0.88;
/// 搖桿右搖桿左右推滿時的旋轉速度（弧度／秒）；5.9 未規定，取 1 rad/s。
pub const YAW_FULL_RAD_PER_SEC: f32 = 1.0;
/// 準星吸附半徑（邏輯像素，乘介面縮放；5.9）。
pub const SNAP_RADIUS_PX: f32 = 48.0;
/// 已吸附時脫離的半徑倍數（避免邊界抖動）。
pub const SNAP_RELEASE_FACTOR: f32 = 1.25;
/// 邊緣捲動的觸發帶寬（邏輯像素）。
pub const EDGE_BAND_PX: f32 = 6.0;
/// 超過此像素距離視為拖曳而非點擊。
const CLICK_SLOP_PX: f32 = 5.0;
/// 觸控點按的位移上限（較粗的手指）與時間上限。
const TAP_SLOP_PX: f32 = 12.0;
const TAP_MAX_SECS: f32 = 0.5;
/// 右鍵／中鍵拖曳旋轉的靈敏度（度／像素）。
const YAW_DEG_PER_PX: f32 = 0.25;
/// 鍵盤 1–4 對應的速度。
const SPEED_KEYS: [(KeyCode, Speed); 4] =
    [(KeyCode::Digit1, Speed::X1), (KeyCode::Digit2, Speed::X5), (KeyCode::Digit3, Speed::X20), (KeyCode::Digit4, Speed::X100)];
/// Ctrl+1–9 書籤鍵（索引 0–8）。
const BOOKMARK_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
const MODE_KEYS: [(KeyCode, MapMode); 6] = [
    (KeyCode::F1, MapMode::Shanhe),
    (KeyCode::F2, MapMode::Tianxia),
    (KeyCode::F3, MapMode::Sanjiao),
    (KeyCode::F4, MapMode::Lingmai),
    (KeyCode::F5, MapMode::Yinguo),
    (KeyCode::F6, MapMode::Yeqi),
];

/// 動作寫入系統的集合；讀取者請排在其後（`PreUpdate` 之後的 `Update` 自然滿足）。
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionSet;

// ── 純函式（可單元測試）─────────────────────────────

/// 徑向死區 + 平方曲線（5.9）：長度先重新映射到 0..1，再取平方，方向不變。
pub fn stick_curve(v: Vec2) -> Vec2 {
    let len = v.length();
    if len <= DEADZONE {
        return Vec2::ZERO;
    }
    let t = ((len - DEADZONE) / (1.0 - DEADZONE)).min(1.0);
    v / len * (t * t)
}

/// 依「最後使用的裝置」決定輸入模式：目前裝置之外有任何活動即切過去；多個同時活動時觸控 > 搖桿 > 鍵鼠。
pub fn next_mode(current: InputMode, kbm: bool, pad: bool, touch: bool) -> InputMode {
    let active = |m: InputMode| match m {
        InputMode::KeyboardMouse => kbm,
        InputMode::Gamepad => pad,
        InputMode::Touch => touch,
    };
    for m in [InputMode::Touch, InputMode::Gamepad, InputMode::KeyboardMouse] {
        if m != current && active(m) {
            return m;
        }
    }
    current
}

/// 邊緣捲動（5.9）：游標進入視窗邊緣帶時回傳朝該邊的方向（y 向下，每軸 -1/0/1）。
pub fn edge_scroll_dir(cursor: Vec2, size: Vec2, band: f32) -> Vec2 {
    let axis = |c: f32, n: f32| if c <= band { -1.0 } else if c >= n - band { 1.0 } else { 0.0 };
    Vec2::new(axis(cursor.x, size.x), axis(cursor.y, size.y))
}

/// 搖桿方向（介面焦點導航用）：主軸方向、超過 0.5 才算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Up,
    Down,
    Left,
    Right,
}

/// 搖桿座標（y 向上）→ 導航方向。
pub fn stick_dir(v: Vec2) -> Option<NavDir> {
    if v.length() < 0.5 {
        return None;
    }
    Some(if v.x.abs() >= v.y.abs() {
        if v.x > 0.0 {
            NavDir::Right
        } else {
            NavDir::Left
        }
    } else if v.y > 0.0 {
        NavDir::Up
    } else {
        NavDir::Down
    })
}

fn nav_key(d: NavDir) -> egui::Key {
    match d {
        NavDir::Up => egui::Key::ArrowUp,
        NavDir::Down => egui::Key::ArrowDown,
        NavDir::Left => egui::Key::ArrowLeft,
        NavDir::Right => egui::Key::ArrowRight,
    }
}

/// 吸附目標的種類（優先序：人物 > 建築 > 軍隊；州郡不吸附，5.9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // 人物、建築、軍隊於 M2 以後出現
pub enum SnapKind {
    Person,
    Building,
    Army,
}

/// 可吸附的目標（螢幕座標）。M2 以後由人物／建築／軍隊的系統每幀寫入 `SnapTargets`。
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct SnapTarget {
    pub screen: Vec2,
    pub kind: SnapKind,
    pub id: u64,
}

/// 吸附目標清單（M1 為空）。每幀由別的系統在 `ActionSet` 之前填入。
#[derive(Resource, Default)]
pub struct SnapTargets(pub Vec<SnapTarget>);

/// 在 `radius` 內挑最近的吸附目標；`current` 為上一幀吸附的目標 id（有遲滯，半徑放寬 25%）。
pub fn pick_snap(center: Vec2, targets: &[SnapTarget], radius: f32, current: Option<u64>) -> Option<usize> {
    if let Some(id) = current {
        if let Some((i, t)) = targets.iter().enumerate().find(|(_, t)| t.id == id) {
            if t.screen.distance(center) <= radius * SNAP_RELEASE_FACTOR {
                return Some(i);
            }
        }
    }
    targets
        .iter()
        .enumerate()
        .filter(|(_, t)| t.screen.distance(center) <= radius)
        .min_by(|a, b| a.1.screen.distance(center).total_cmp(&b.1.screen.distance(center)))
        .map(|(i, _)| i)
}

// ── 資源 ────────────────────────────────────────────

/// 準星狀態（搖桿模式；5.9）。
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct Reticle {
    /// 準星目前的螢幕位置（吸附後）；非搖桿模式為 `None`。
    pub pos: Option<Vec2>,
    pub snapped: bool,
    snap_id: Option<u64>,
}

/// 介面縮放（吸附半徑隨之調整，5.9）；Deck 版面與桌機皆為 1.0，保留給設定。
#[derive(Resource)]
pub struct UiScaleFactor(pub f32);

impl Default for UiScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

/// 邊緣捲動開關（設定可關；預設開）。
#[derive(Resource)]
pub struct EdgeScroll(pub bool);

/// `SIMING_FAKE_GAMEPAD=1`：強制搖桿模式且不自動切走（無實體手把時驗收截圖用）。
#[derive(Resource, Default)]
pub struct FakeGamepad(pub bool);

/// egui 的指標／鍵盤狀態（由 `read_egui_state` 每幀更新；無 egui 時全為 false）。
#[derive(Resource, Default)]
pub struct UiPointerState {
    pub over_ui: bool,
    pub kb_ui: bool,
}

/// 待送入 egui 的焦點導航事件（方向鍵、Enter）。
#[derive(Resource, Default)]
pub struct UiNavQueue(pub Vec<egui::Event>);

/// 搖桿模式下開啟卡片時，要求卡片把焦點放到第一個可互動元件（卡片消費後清除）。
#[derive(Resource, Default)]
pub struct UiFocusRequest(pub bool);

#[derive(Resource, Default)]
struct KbmState {
    last_cursor: Option<Vec2>,
    left_down_at: Option<Vec2>,
    left_dragging: bool,
    left_started_on_ui: bool,
    right_down_at: Option<Vec2>,
    right_dragging: bool,
}

#[derive(Resource, Default)]
struct TouchState {
    start: Option<(Vec2, f32)>,
    dragging: bool,
    multi: bool,
    started_on_ui: bool,
}

#[derive(Resource, Default)]
struct StickNav {
    dir: Option<NavDir>,
    timer: f32,
}

// ── 外掛 ────────────────────────────────────────────

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputActions>()
            .init_resource::<InputMode>()
            .init_resource::<Reticle>()
            .init_resource::<SnapTargets>()
            .init_resource::<UiPointerState>()
            .init_resource::<UiNavQueue>()
            .init_resource::<UiFocusRequest>()
            .init_resource::<KbmState>()
            .init_resource::<TouchState>()
            .init_resource::<StickNav>()
            .init_resource::<Selection>()
            .init_resource::<UiScaleFactor>()
            .insert_resource(EdgeScroll(std::env::var("SIMING_NO_EDGE_SCROLL").is_err()))
            .insert_resource(FakeGamepad(std::env::var("SIMING_FAKE_GAMEPAD").is_ok()))
            .add_systems(Startup, apply_fake_gamepad)
            .add_systems(
                PreUpdate,
                (clear_actions, detect_mode, kbm_writer, gamepad_writer, touch_writer, update_reticle, apply_cursor_visibility)
                    .chain()
                    .in_set(ActionSet)
                    .after(InputSystem),
            );
        if app.is_plugin_added::<bevy_egui::EguiPlugin>() {
            app.add_systems(PreUpdate, read_egui_state.before(ActionSet).after(InputSystem))
                .add_systems(PreUpdate, forward_nav_events.after(ActionSet));
        }
    }
}

fn apply_fake_gamepad(fake: Res<FakeGamepad>, mut mode: ResMut<InputMode>) {
    if fake.0 {
        *mode = InputMode::Gamepad;
    }
}

fn clear_actions(mut act: ResMut<InputActions>) {
    *act = InputActions::default();
}

fn read_egui_state(mut egui: bevy_egui::EguiContexts, mode: Res<InputMode>, mut st: ResMut<UiPointerState>) {
    match egui.try_ctx_mut() {
        Some(ctx) => {
            st.over_ui = ctx.wants_pointer_input() || ctx.is_pointer_over_area();
            // egui 把「任何元件有焦點」都算作 wants_keyboard_input（連滑鼠點過的按鈕也是），
            // 會誤擋快捷鍵。目前沒有文字輸入框，因此非搖桿模式一律放掉焦點、kb_ui 恆為 false；
            // 日後加入文字輸入框時改為只認該輸入框。
            if *mode != InputMode::Gamepad {
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                });
            }
            st.kb_ui = false;
        }
        None => *st = UiPointerState::default(),
    }
}

fn forward_nav_events(
    mut queue: ResMut<UiNavQueue>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut out: EventWriter<bevy_egui::input::EguiInputEvent>,
) {
    let Ok(context) = window.single() else {
        queue.0.clear();
        return;
    };
    for event in queue.0.drain(..) {
        out.write(bevy_egui::input::EguiInputEvent { context, event });
    }
}

#[allow(clippy::too_many_arguments)]
fn detect_mode(
    mut mode: ResMut<InputMode>,
    fake: Res<FakeGamepad>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    pads: Query<&Gamepad>,
    touches: Res<Touches>,
) {
    if fake.0 {
        *mode = InputMode::Gamepad;
        return;
    }
    let kbm = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || motion.delta != Vec2::ZERO
        || scroll.delta != Vec2::ZERO;
    let pad = pads.iter().any(|g| {
        g.get_just_pressed().next().is_some() || g.left_stick().length() > DEADZONE || g.right_stick().length() > DEADZONE
    });
    let touch = touches.iter().next().is_some() || touches.iter_just_released().next().is_some();
    let next = next_mode(*mode, kbm, pad, touch);
    if next != *mode {
        *mode = next;
    }
}

fn apply_cursor_visibility(mode: Res<InputMode>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if !mode.is_changed() {
        return;
    }
    for mut c in &mut windows {
        c.cursor_options.visible = *mode != InputMode::Gamepad;
    }
}

// ── 鍵鼠 ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn kbm_writer(
    mode: Res<InputMode>,
    mut act: ResMut<InputActions>,
    mut st: ResMut<KbmState>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui: Res<UiPointerState>,
    edge: Res<EdgeScroll>,
) {
    if *mode != InputMode::KeyboardMouse {
        st.last_cursor = None;
        return;
    }
    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    let over_ui = ui.over_ui;
    act.pointer = cursor;
    act.pointer_over_ui = over_ui;

    // 鍵盤（輸入框有焦點時不吃字元鍵）
    if !ui.kb_ui {
        let mut v = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            v.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            v.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            v.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            v.x += 1.0;
        }
        act.pan_keys = v;
        act.pause_toggle_pressed = keys.just_pressed(KeyCode::Space);
        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
        for (i, k) in BOOKMARK_KEYS.iter().enumerate() {
            if keys.just_pressed(*k) {
                if ctrl {
                    act.bookmark_set = Some(i as u8);
                } else if alt {
                    act.bookmark_goto = Some(i as u8);
                } else if let Some((_, s)) = SPEED_KEYS.iter().find(|(sk, _)| sk == k) {
                    act.speed_set = Some(*s);
                }
            }
        }
        for (k, m) in MODE_KEYS {
            if keys.just_pressed(k) {
                act.mode_set = Some(m);
            }
        }
        // GDD 5.9 互動層鍵位：F 關注、X 溯因、Enter 飛往、Tab 傳聞清單、[ ] 上下則傳聞
        act.follow_pressed = keys.just_pressed(KeyCode::KeyF);
        act.trace_pressed = keys.just_pressed(KeyCode::KeyX);
        act.goto_notice_pressed = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);
        act.rumor_toggle_pressed = keys.just_pressed(KeyCode::Tab);
        act.rumor_prev_pressed = keys.just_pressed(KeyCode::BracketLeft);
        act.rumor_next_pressed = keys.just_pressed(KeyCode::BracketRight);
    }
    act.back_pressed = keys.just_pressed(KeyCode::Escape);

    // 邊緣捲動：游標貼近視窗邊緣、未在拖曳、未在介面上、視窗有焦點
    if edge.0 && !over_ui && !buttons.pressed(MouseButton::Left) && !buttons.pressed(MouseButton::Right) && !buttons.pressed(MouseButton::Middle) {
        if let (Some(c), Ok(w)) = (cursor, windows.single()) {
            if w.focused {
                let d = edge_scroll_dir(c, Vec2::new(w.width(), w.height()), EDGE_BAND_PX);
                if d != Vec2::ZERO {
                    act.pan_keys = (act.pan_keys + d).clamp_length_max(1.0);
                }
            }
        }
    }

    // 滾輪縮放（以游標為中心）；卡片上不縮放
    if scroll.delta.y != 0.0 && !over_ui {
        act.zoom = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        act.zoom_anchor = cursor;
    }

    // 左鍵：拖曳平移／點擊選取
    if buttons.just_pressed(MouseButton::Left) {
        st.left_started_on_ui = over_ui;
        st.left_down_at = cursor;
        st.left_dragging = false;
    }
    if buttons.pressed(MouseButton::Left) && !st.left_started_on_ui {
        if let (Some(c), Some(prev), Some(down)) = (cursor, st.last_cursor, st.left_down_at) {
            if !st.left_dragging && c.distance(down) > CLICK_SLOP_PX {
                st.left_dragging = true;
            }
            if st.left_dragging {
                act.pan_pixels = c - prev;
            }
        }
    }
    if buttons.just_released(MouseButton::Left) {
        if !st.left_dragging && !st.left_started_on_ui && st.left_down_at.is_some() {
            act.select_pressed = true;
        }
        st.left_down_at = None;
        st.left_dragging = false;
    }

    // 右鍵／中鍵拖曳：旋轉方位角（滑鼠右移 → 方位角減少）；右鍵點擊（未拖曳）：情境印環（5.9）
    if buttons.just_pressed(MouseButton::Right) {
        st.right_down_at = if over_ui { None } else { cursor };
        st.right_dragging = false;
    }
    if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
        if let (Some(c), Some(prev)) = (cursor, st.last_cursor) {
            if let Some(down) = st.right_down_at {
                if c.distance(down) > CLICK_SLOP_PX {
                    st.right_dragging = true;
                }
            }
            if !over_ui && (st.right_dragging || buttons.pressed(MouseButton::Middle)) {
                act.yaw = -(c.x - prev.x) * YAW_DEG_PER_PX.to_radians();
            }
        }
    }
    if buttons.just_released(MouseButton::Right) {
        if !st.right_dragging && st.right_down_at.is_some() {
            act.context_pressed = true;
        }
        st.right_down_at = None;
        st.right_dragging = false;
    }
    st.last_cursor = cursor;
}

// ── 搖桿 ────────────────────────────────────────────

fn nav_event(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers: egui::Modifiers::NONE }
}

#[allow(clippy::too_many_arguments)]
fn gamepad_writer(
    mode: Res<InputMode>,
    time: Res<Time>,
    mut act: ResMut<InputActions>,
    pads: Query<&Gamepad>,
    sel: Res<Selection>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nav: ResMut<StickNav>,
    mut queue: ResMut<UiNavQueue>,
    mut focus_req: ResMut<UiFocusRequest>,
    mut prev_card: Local<bool>,
) {
    let card = sel.target.is_some();
    if *mode != InputMode::Gamepad {
        *prev_card = card;
        nav.dir = None;
        return;
    }
    let dt = time.delta_secs().min(0.1);
    // 卡片剛開啟 → 焦點落在第一個可互動元件
    if card && !*prev_card {
        focus_req.0 = true;
    }
    *prev_card = card;

    let (mut ls, mut rs) = (Vec2::ZERO, Vec2::ZERO);
    let btn = |b: GamepadButton| pads.iter().any(|g| g.just_pressed(b));
    for g in &pads {
        ls += g.left_stick();
        rs += g.right_stick();
    }
    ls = ls.clamp_length_max(1.0);
    rs = rs.clamp_length_max(1.0);
    use GamepadButton as B;

    // 無論焦點在哪：速度、暫停、總覽
    act.speed_down_pressed = btn(B::LeftTrigger2);
    act.speed_up_pressed = btn(B::RightTrigger2);
    act.pause_toggle_pressed = btn(B::Start);
    act.menu_pressed = btn(B::Select);
    act.back_pressed = btn(B::East);

    if card {
        // 介面焦點：左搖桿／十字鍵移動焦點，A 確認，B 關閉（由消費者處理），LB／RB 切分頁
        let mut dir = stick_dir(ls);
        for (b, d) in [(B::DPadUp, NavDir::Up), (B::DPadDown, NavDir::Down), (B::DPadLeft, NavDir::Left), (B::DPadRight, NavDir::Right)] {
            if btn(b) {
                queue.0.push(nav_event(nav_key(d), true));
                queue.0.push(nav_event(nav_key(d), false));
            }
        }
        match dir.take() {
            Some(d) => {
                nav.timer -= dt;
                if nav.dir != Some(d) {
                    nav.dir = Some(d);
                    nav.timer = 0.35;
                    queue.0.push(nav_event(nav_key(d), true));
                    queue.0.push(nav_event(nav_key(d), false));
                } else if nav.timer <= 0.0 {
                    nav.timer = 0.12;
                    queue.0.push(nav_event(nav_key(d), true));
                    queue.0.push(nav_event(nav_key(d), false));
                }
            }
            None => nav.dir = None,
        }
        if btn(B::South) {
            queue.0.push(nav_event(egui::Key::Enter, true));
            queue.0.push(nav_event(egui::Key::Enter, false));
        }
        act.tab_prev_pressed = btn(B::LeftTrigger);
        act.tab_next_pressed = btn(B::RightTrigger);
        act.context_pressed = btn(B::West);
        act.follow_pressed = btn(B::North);
    } else {
        nav.dir = None;
        // 地圖：左搖桿平移、右搖桿縮放（上下）與旋轉（左右）
        let p = stick_curve(ls);
        act.pan = Vec2::new(p.x, -p.y) * PAN_FULL_WIDTHS_PER_SEC;
        let r = stick_curve(rs);
        act.zoom = r.y * ZOOM_FULL_FACTOR_PER_SEC.ln() / (1.0 / ZOOM_NOTCH).ln() * dt;
        act.yaw = -r.x * YAW_FULL_RAD_PER_SEC * dt;
        if let Ok(w) = windows.single() {
            act.zoom_anchor = Some(Vec2::new(w.width(), w.height()) * 0.5);
        }
        act.select_pressed = btn(B::South);
        act.follow_pressed = btn(B::North);
        act.context_pressed = btn(B::West);
        act.mode_prev_pressed = btn(B::LeftTrigger);
        act.mode_next_pressed = btn(B::RightTrigger);
        act.rumor_prev_pressed = btn(B::DPadLeft);
        act.rumor_next_pressed = btn(B::DPadRight);
        act.rumor_toggle_pressed = btn(B::DPadUp) || btn(B::DPadDown);
        act.labels_toggle_pressed = btn(B::LeftThumb);
        act.recenter_pressed = btn(B::RightThumb);
    }
}

/// 準星：搖桿模式下固定在畫面中央，吸附半徑內有目標則吸過去；`pointer` = 準星位置。
fn update_reticle(
    mode: Res<InputMode>,
    mut act: ResMut<InputActions>,
    mut reticle: ResMut<Reticle>,
    targets: Res<SnapTargets>,
    ui_scale: Res<UiScaleFactor>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if *mode != InputMode::Gamepad {
        *reticle = Reticle::default();
        return;
    }
    // 無視窗（測試）時以 1280×800 的中央代替
    let size = windows.single().map(|w| Vec2::new(w.width(), w.height())).unwrap_or(Vec2::new(1280.0, 800.0));
    let center = size * 0.5;
    let radius = SNAP_RADIUS_PX * ui_scale.0;
    let hit = pick_snap(center, &targets.0, radius, reticle.snap_id);
    reticle.snapped = hit.is_some();
    reticle.snap_id = hit.map(|i| targets.0[i].id);
    let pos = hit.map_or(center, |i| targets.0[i].screen);
    reticle.pos = Some(pos);
    act.pointer = Some(pos);
    act.pointer_over_ui = false;
}

// ── 觸控 ────────────────────────────────────────────

fn touch_writer(
    mode: Res<InputMode>,
    time: Res<Time>,
    touches: Res<Touches>,
    ui: Res<UiPointerState>,
    mut act: ResMut<InputActions>,
    mut st: ResMut<TouchState>,
) {
    if *mode != InputMode::Touch {
        *st = TouchState::default();
        return;
    }
    let now = time.elapsed_secs();
    let active: Vec<&bevy::input::touch::Touch> = touches.iter().collect();
    match active.len() {
        1 => {
            let t = active[0];
            if touches.just_pressed(t.id()) {
                st.start = Some((t.position(), now));
                st.dragging = false;
                st.started_on_ui = ui.over_ui;
            }
            if let Some((start, _)) = st.start {
                if !st.dragging && t.position().distance(start) > TAP_SLOP_PX {
                    st.dragging = true;
                }
            }
            if st.dragging && !st.started_on_ui && !st.multi {
                act.pan_pixels = t.position() - t.previous_position();
                act.pointer = Some(t.position());
            }
        }
        2 => {
            st.multi = true;
            let (a, b) = (active[0], active[1]);
            let (d, pd) = (a.position().distance(b.position()), a.previous_position().distance(b.previous_position()));
            if d > 1.0 && pd > 1.0 {
                act.zoom = (d / pd).ln() / (1.0 / ZOOM_NOTCH).ln();
            }
            let mid = (a.position() + b.position()) * 0.5;
            let pmid = (a.previous_position() + b.previous_position()) * 0.5;
            act.zoom_anchor = Some(mid);
            act.pointer = Some(mid);
            act.pan_pixels = mid - pmid;
            let ang = |p: Vec2, q: Vec2| (q - p).to_angle();
            let mut da = ang(a.position(), b.position()) - ang(a.previous_position(), b.previous_position());
            da = (da + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            act.yaw = -da;
        }
        _ => {}
    }
    // 放開：未拖曳、未多指、短時間內 → 點按選取
    for t in touches.iter_just_released() {
        if let Some((start, t0)) = st.start {
            if !st.dragging && !st.multi && !st.started_on_ui && now - t0 <= TAP_MAX_SECS && t.position().distance(start) <= TAP_SLOP_PX {
                act.select_pressed = true;
                act.pointer = Some(t.position());
            }
        }
    }
    if active.is_empty() {
        st.start = None;
        st.dragging = false;
        st.multi = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::gamepad::{
        GamepadAxis, GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
    };
    use bevy::input::InputPlugin as BevyInputPlugin;

    #[test]
    fn deadzone_and_curve() {
        assert_eq!(stick_curve(Vec2::new(0.1, 0.1)), Vec2::ZERO); // |v| = 0.141 < 0.15
        assert_eq!(stick_curve(Vec2::new(0.15, 0.0)), Vec2::ZERO);
        // 推滿 = 1
        assert!((stick_curve(Vec2::new(1.0, 0.0)) - Vec2::new(1.0, 0.0)).length() < 1e-6);
        // 超過 1 被夾到 1（對角線 > 1）
        assert!((stick_curve(Vec2::new(1.0, 1.0)).length() - 1.0).abs() < 1e-6);
        // 平方曲線：重新映射後 0.5 → 0.25
        let v = 0.15 + 0.5 * 0.85;
        assert!((stick_curve(Vec2::new(0.0, -v)) - Vec2::new(0.0, -0.25)).length() < 1e-5);
        // 單調遞增且連續（死區邊界附近從 0 起）
        assert!(stick_curve(Vec2::new(0.16, 0.0)).x < 0.001 && stick_curve(Vec2::new(0.16, 0.0)).x > 0.0);
        assert!(stick_curve(Vec2::new(-0.6, 0.0)).x < 0.0);
    }

    #[test]
    fn edge_scroll_bands() {
        let size = Vec2::new(1000.0, 600.0);
        assert_eq!(edge_scroll_dir(Vec2::new(500.0, 300.0), size, 6.0), Vec2::ZERO);
        assert_eq!(edge_scroll_dir(Vec2::new(2.0, 300.0), size, 6.0), Vec2::new(-1.0, 0.0));
        assert_eq!(edge_scroll_dir(Vec2::new(999.0, 599.0), size, 6.0), Vec2::new(1.0, 1.0));
        assert_eq!(edge_scroll_dir(Vec2::new(500.0, 0.0), size, 6.0), Vec2::new(0.0, -1.0));
    }

    #[test]
    fn stick_directions() {
        assert_eq!(stick_dir(Vec2::new(0.3, 0.3)), None);
        assert_eq!(stick_dir(Vec2::new(0.9, 0.2)), Some(NavDir::Right));
        assert_eq!(stick_dir(Vec2::new(-0.2, 0.9)), Some(NavDir::Up));
        assert_eq!(stick_dir(Vec2::new(0.1, -0.8)), Some(NavDir::Down));
    }

    #[test]
    fn mode_switching_follows_last_device() {
        use InputMode::*;
        assert_eq!(next_mode(KeyboardMouse, false, false, false), KeyboardMouse);
        assert_eq!(next_mode(KeyboardMouse, true, false, false), KeyboardMouse);
        assert_eq!(next_mode(KeyboardMouse, false, true, false), Gamepad);
        assert_eq!(next_mode(Gamepad, true, false, false), KeyboardMouse);
        assert_eq!(next_mode(Gamepad, false, false, true), Touch);
        assert_eq!(next_mode(Touch, true, true, false), Gamepad); // 同時活動：搖桿優先於鍵鼠
        assert_eq!(next_mode(Gamepad, true, true, false), KeyboardMouse); // 目前裝置不算「切換」
    }

    #[test]
    fn snapping_picks_nearest_with_hysteresis() {
        let c = Vec2::new(100.0, 100.0);
        let t = |x: f32, id| SnapTarget { screen: Vec2::new(x, 100.0), kind: SnapKind::Person, id };
        assert_eq!(pick_snap(c, &[], 48.0, None), None);
        assert_eq!(pick_snap(c, &[t(200.0, 1)], 48.0, None), None);
        assert_eq!(pick_snap(c, &[t(140.0, 1), t(120.0, 2)], 48.0, None), Some(1));
        // 已吸附者在 48..60 px 之間仍保持
        assert_eq!(pick_snap(c, &[t(155.0, 1), t(120.0, 2)], 48.0, Some(1)), Some(0));
        assert_eq!(pick_snap(c, &[t(170.0, 1)], 48.0, Some(1)), None);
    }

    fn test_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BevyInputPlugin)).add_plugins(InputPlugin);
        let pad = app.world_mut().spawn_empty().id();
        app.world_mut().send_event(GamepadConnectionEvent::new(
            pad,
            GamepadConnection::Connected { name: "test".into(), vendor_id: None, product_id: None },
        ));
        app.update();
        (app, pad)
    }

    fn axis(app: &mut App, pad: Entity, a: GamepadAxis, v: f32) {
        app.world_mut().send_event(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(pad, a, v)));
    }
    fn button(app: &mut App, pad: Entity, b: GamepadButton, v: f32) {
        app.world_mut().send_event(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(pad, b, v)));
    }
    fn actions(app: &App) -> InputActions {
        app.world().resource::<InputActions>().clone()
    }

    #[test]
    fn synthetic_gamepad_events_drive_actions() {
        let (mut app, pad) = test_app();
        assert_eq!(*app.world().resource::<InputMode>(), InputMode::KeyboardMouse);

        // 左搖桿推右：自動切到搖桿模式，pan 朝右（螢幕 +x），上推 = 螢幕 -y
        axis(&mut app, pad, GamepadAxis::LeftStickX, 1.0);
        axis(&mut app, pad, GamepadAxis::LeftStickY, 0.0);
        app.update();
        assert_eq!(*app.world().resource::<InputMode>(), InputMode::Gamepad);
        let a = actions(&app);
        assert!((a.pan.x - PAN_FULL_WIDTHS_PER_SEC).abs() < 1e-4, "{:?}", a.pan);
        assert!(a.pan.y.abs() < 1e-4);
        // 準星 = 畫面中央，指標跟著準星
        assert_eq!(a.pointer, Some(Vec2::new(640.0, 400.0)));
        assert!(app.world().resource::<Reticle>().pos.is_some());

        // 搖桿在死區內：無平移
        axis(&mut app, pad, GamepadAxis::LeftStickX, 0.1);
        axis(&mut app, pad, GamepadAxis::LeftStickY, 0.1);
        app.update();
        assert_eq!(actions(&app).pan, Vec2::ZERO);

        // 右搖桿上推 → 拉近；左推 → 方位角增加（符號：左 = +）
        axis(&mut app, pad, GamepadAxis::LeftStickX, 0.0);
        axis(&mut app, pad, GamepadAxis::LeftStickY, 0.0);
        axis(&mut app, pad, GamepadAxis::RightStickY, 1.0);
        axis(&mut app, pad, GamepadAxis::RightStickX, -1.0);
        app.update();
        let a = actions(&app);
        assert!(a.zoom >= 0.0 && a.yaw >= 0.0);
        axis(&mut app, pad, GamepadAxis::RightStickY, 0.0);
        axis(&mut app, pad, GamepadAxis::RightStickX, 0.0);
        app.update();

        // 按鈕：A 選取、B 返回、RT 加速、LB 上一個地圖模式，且只在按下那一幀為真
        button(&mut app, pad, GamepadButton::South, 1.0);
        button(&mut app, pad, GamepadButton::East, 1.0);
        button(&mut app, pad, GamepadButton::RightTrigger2, 1.0);
        button(&mut app, pad, GamepadButton::LeftTrigger, 1.0);
        button(&mut app, pad, GamepadButton::Start, 1.0);
        app.update();
        let a = actions(&app);
        assert!(a.select_pressed && a.back_pressed && a.speed_up_pressed && a.mode_prev_pressed && a.pause_toggle_pressed);
        assert!(!a.speed_down_pressed && !a.mode_next_pressed);
        app.update();
        let a = actions(&app);
        assert!(!a.select_pressed && !a.back_pressed && !a.speed_up_pressed);
    }

    #[test]
    fn keyboard_activity_switches_back_and_hides_reticle() {
        let (mut app, pad) = test_app();
        button(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        assert_eq!(*app.world().resource::<InputMode>(), InputMode::Gamepad);
        assert!(app.world().resource::<Reticle>().pos.is_some());
        let win = app.world_mut().spawn_empty().id();
        app.world_mut().send_event(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: bevy::input::keyboard::Key::Space,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: win,
        });
        app.update();
        assert_eq!(*app.world().resource::<InputMode>(), InputMode::KeyboardMouse);
        assert!(app.world().resource::<Reticle>().pos.is_none());
        assert!(actions(&app).pause_toggle_pressed);
    }

    #[test]
    fn card_open_routes_gamepad_to_ui_focus() {
        let (mut app, pad) = test_app();
        app.world_mut().resource_mut::<Selection>().open(crate::state::Target::Province(3), Some(3));
        axis(&mut app, pad, GamepadAxis::LeftStickX, 1.0);
        app.update();
        // 卡片開啟：左搖桿不平移，改送焦點導航事件
        let a = actions(&app);
        assert_eq!(a.pan, Vec2::ZERO);
        assert!(!app.world().resource::<UiNavQueue>().0.is_empty());
        assert!(app.world().resource::<UiFocusRequest>().0);
        button(&mut app, pad, GamepadButton::South, 1.0);
        app.update();
        assert!(!actions(&app).select_pressed);
    }
}
