//! 呈現層共用狀態：render（T8）、ui（T9）、bridge 之間的契約。由主控維護。

use bevy::prelude::*;
use siming_sim::{EventId, Geography, PersonId, ProvinceId};

/// GDD 5.4 地圖模式。M1 只啟用山河、天下、靈脈。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // 三教、因果、氣運為 M2+ 模式
pub enum MapMode {
    #[default]
    Shanhe,
    Tianxia,
    Sanjiao,
    Lingmai,
    Yinguo,
    Yeqi,
}

impl MapMode {
    pub const M1: [MapMode; 3] = [MapMode::Shanhe, MapMode::Tianxia, MapMode::Lingmai];

    pub fn name_zh(self) -> &'static str {
        match self {
            MapMode::Shanhe => "山河",
            MapMode::Tianxia => "天下",
            MapMode::Sanjiao => "三教",
            MapMode::Lingmai => "靈脈",
            MapMode::Yinguo => "因果",
            MapMode::Yeqi => "業氣",
        }
    }
}

/// GDD 2.6 速度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    X1,
    X5,
    X20,
    X100,
}

impl Speed {
    /// 每秒推進的月數（GDD 2.6）。
    pub fn months_per_second(self) -> f32 {
        match self {
            Speed::X1 => 2.0,
            Speed::X5 => 10.0,
            Speed::X20 => 40.0,
            Speed::X100 => 200.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Speed::X1 => "1×",
            Speed::X5 => "5×",
            Speed::X20 => "20×",
            Speed::X100 => "100×",
        }
    }
}

/// 時間控制（GDD 2.6、6.4）。
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct TimeControl {
    pub paused: bool,
    pub speed: Speed,
}

/// 選取的對象（GDD 5.7 情境印環與命簿的主體）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Province(ProvinceId),
    Person(PersonId),
    Event(EventId),
}

/// 滑鼠停留與選取（由 render/input 寫入州郡部分；人物與事件由介面寫入）。
#[derive(Resource, Debug, Clone, Default)]
pub struct Selection {
    pub hovered: Option<ProvinceId>,
    /// 高亮的州郡（地圖著色用）；選取人物或事件時為其所在州郡。
    pub selected: Option<ProvinceId>,
    /// 命簿與情境印環的主體。
    pub target: Option<Target>,
    /// 情境印環是否展開（右鍵／X）。
    pub ring_open: bool,
    /// 「點了就到」的回溯堆疊（B／Esc 回上一張，最多 8 層）。
    pub history: Vec<Target>,
}

impl Selection {
    /// 開啟新的主體：目前主體推入堆疊，州郡高亮改為主體所在州郡。
    pub fn open(&mut self, target: Target, province: Option<ProvinceId>) {
        if let Some(cur) = self.target {
            if cur != target {
                self.history.push(cur);
                if self.history.len() > 8 {
                    self.history.remove(0);
                }
            }
        }
        self.target = Some(target);
        self.ring_open = false;
        if let Some(p) = province {
            self.selected = Some(p);
        }
    }

    /// B／Esc：先收印環，再回上一張，最後關閉。
    pub fn back(&mut self) {
        if self.ring_open {
            self.ring_open = false;
        } else if let Some(prev) = self.history.pop() {
            self.target = Some(prev);
            if let Target::Province(p) = prev {
                self.selected = Some(p);
            }
        } else {
            self.target = None;
            self.selected = None;
        }
    }

    pub fn clear(&mut self) {
        self.target = None;
        self.selected = None;
        self.ring_open = false;
        self.history.clear();
    }
}

/// 介面要求鏡頭飛往某格（GDD 5.7「點了就到」）；由鏡頭系統取走。
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct FlyTo(pub Option<u32>);

/// 地圖的顯示狀態（由鏡頭系統寫入，標籤與網格讀取；GDD 5.2、5.3）。
#[derive(Resource, Debug, Clone, Copy)]
pub struct MapView {
    /// 地形高度縮放，1 → 0.05（古地圖化）。
    pub height_scale: f32,
    /// 古地圖化權重 0..1（1 = 完全平塗）。
    pub flatten: f32,
    /// 鏡頭離地高度（世界單位）。
    pub camera_z: f32,
    /// 鏡頭焦點（世界 xz；x 已正規化到 `[0, W)`，y 為列座標）。
    pub focus: Vec2,
    /// 星球視角的過渡權重 0..1（0 = 平面地圖、1 = 球面；已緩動，GDD 5.3 過渡 0.6 秒）。
    pub globe: f32,
}

impl Default for MapView {
    fn default() -> Self {
        Self { height_scale: 1.0, flatten: 0.0, camera_z: 100.0, focus: Vec2::ZERO, globe: 0.0 }
    }
}

/// 模擬推進後送出（ui 用來刷新顯示）。
#[derive(Event, Debug, Clone, Copy)]
pub struct SimAdvanced {
    pub months: u32,
}

/// 原因簽的層級（GDD 5.7 通知優先序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoticeLevel {
    /// 系統提示（推演降速），無動作。
    #[default]
    Info,
    /// 緩：降到 1×。
    Slow,
    /// 止：暫停。
    Stop,
}

/// 原因簽（GDD 5.7）：自動止／緩的理由，或系統提示（推演降速）。`key` 為 `ui.zh-Hant.json` 的鍵；
/// `text` 非空時直接顯示。`cell` 為「飛往」的目的地。
#[derive(Resource, Debug, Clone, Default)]
pub struct SimNotice {
    pub key: &'static str,
    pub text: String,
    pub level: NoticeLevel,
    /// 「飛往」的目的地格（`event` 無地點時的備援；介面以事件優先）。
    #[allow(dead_code)]
    pub cell: Option<u32>,
    pub event: Option<EventId>,
    /// 同時發生的其他「止」級事件數（簽上顯示「＋n」）。
    pub extra: u32,
    /// 剩餘顯示秒數；<= 0 表示無提示。止級的簽在續行前不會倒數。
    pub remaining: f32,
}

impl SimNotice {
    pub fn visible(&self) -> bool {
        self.remaining > 0.0
    }
}

impl Speed {
    pub const ALL: [Speed; 4] = [Speed::X1, Speed::X5, Speed::X20, Speed::X100];

    /// 下一個較慢的速度；已是最慢則 `None`。
    pub fn slower(self) -> Option<Speed> {
        match self {
            Speed::X1 => None,
            Speed::X5 => Some(Speed::X1),
            Speed::X20 => Some(Speed::X5),
            Speed::X100 => Some(Speed::X20),
        }
    }
}

/// GDD 5.2 鏡頭高度範圍 `[地圖寬 × 0.04, 地圖寬 × 1.1]`。
pub fn camera_z_range(map_w: f32) -> (f32, f32) {
    (map_w * 0.04, map_w * 1.1)
}

/// GDD 5.2 俯角（弧度）：`z_min` 時 50°，`z_max` 時 85°，線性內插。
pub fn camera_pitch(z: f32, map_w: f32) -> f32 {
    let (lo, hi) = camera_z_range(map_w);
    let t = ((z - lo) / (hi - lo)).clamp(0.0, 1.0);
    (50.0 + (85.0 - 50.0) * t).to_radians()
}

/// GDD 5.2 古地圖化權重：`z > 地圖寬 × 0.6` 起由 0 漸變到 1（於 `z_max` 達 1）。
pub fn flatten_weight(z: f32, map_w: f32) -> f32 {
    let (_, hi) = camera_z_range(map_w);
    let lo = map_w * 0.6;
    let t = ((z - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// GDD 5.2 星球視角門檻：`z > 地圖寬 × 0.9`。
pub const GLOBE_Z_FRAC: f32 = 0.9;

/// GDD 5.2 南北限制：鏡頭焦點的列座標範圍（南北緯 ±75°；等距圓柱投影，y = 0 為北極）。
pub fn focus_row_range(map_h: f32) -> (f32, f32) {
    // 開發驗收：`SIMING_POLE_DEBUG` 解除緯度限制，以便正對極點檢視（正式遊玩不設）。
    let t = if std::env::var("SIMING_POLE_DEBUG").is_ok() { 0.0 } else { (90.0 - 75.0) / 180.0 };
    (map_h * t, map_h * (1.0 - t))
}

/// GDD 5.2：垂直視角 30°。
pub const FOV_DEG: f32 = 30.0;
/// GDD 5.2：平面視角下，畫面在南北緣之外最多可見的寬度（地圖寬的比例；極地邊疆的淡出帶）。
pub const FRONTIER_FRAC: f32 = 0.05;

/// GDD 5.2：平面視角下，鏡頭焦點的列座標範圍。除 ±75° 外，再要求畫面可見的地面不超出南北緣
/// 太多（`FRONTIER_FRAC`）；縮得越遠、俯角越低，可看的緯度越靠近赤道。星球視角不受此限。
pub fn focus_row_limits(map_w: f32, map_h: f32, z: f32) -> (f32, f32) {
    let (lo, hi) = focus_row_range(map_h);
    if z > map_w * GLOBE_Z_FRAC || std::env::var("SIMING_POLE_DEBUG").is_ok() {
        return (lo, hi);
    }
    let pitch = camera_pitch(z, map_w);
    let half = FOV_DEG.to_radians() * 0.5;
    // 畫面頂／底緣的視線與地面交點，相對焦點的南北距離（北為正）。
    let north = z * (1.0 / (pitch - half).tan() - 1.0 / pitch.tan());
    let south = z * (1.0 / pitch.tan() - 1.0 / (pitch + half).tan());
    let allow = map_w * FRONTIER_FRAC;
    let (lo2, hi2) = (lo.max(north - allow), hi.min(map_h - (south - allow)));
    if lo2 > hi2 {
        (map_h * 0.5, map_h * 0.5)
    } else {
        (lo2, hi2)
    }
}

/// GDD 5.3：把 `x` 搬到離 `reference` 最近的東西副本（週期 `w`）。
pub fn nearest_copy_x(x: f32, reference: f32, w: f32) -> f32 {
    x + w * ((reference - x) / w).round()
}

/// 世界座標慣例：一格 = 1 單位；格 (x, y) 的中心在 (x + 0.5, 高度, y + 0.5)。
pub fn cell_center(geo: &Geography, cell: u32) -> Vec2 {
    let (x, y) = geo.xy(cell);
    Vec2::new(x as f32 + 0.5, y as f32 + 0.5)
}

/// GDD 5.3 地形高度：陸地 `(h − sea) × 地圖寬 × 0.04 × 高度縮放`；水面為 0。
pub fn terrain_height(geo: &Geography, cell: u32, height_scale: f32) -> f32 {
    let h = geo.height[cell as usize];
    if geo.terrain[cell as usize].is_water() || h < geo.sea_level {
        0.0
    } else {
        (h - geo.sea_level) * geo.w as f32 * 0.04 * height_scale
    }
}

// ── GDD 5.9 動作層：鍵鼠與搖桿分別寫入，其他系統只讀 ─────────

/// 目前的輸入模式（依最後使用的裝置自動切換，GDD 5.9）。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    KeyboardMouse,
    Gamepad,
    Touch,
}

/// 每幀由輸入系統（T14，`input/actions.rs`）寫入、其他系統讀取的動作（GDD 5.9）。
/// 連續量以「本幀」為單位（搖桿已乘上 dt，故各裝置同一單位）；
/// 觸發量（`*_pressed`）只在按下的那一幀為真。每幀在 `PreUpdate` 先清空再寫入。
#[derive(Resource, Debug, Clone, Default)]
pub struct InputActions {
    /// 搖桿平移：螢幕座標方向（y 向下），單位為「畫面寬的比例／秒」（推滿 0.8，已套死區與平方曲線）。
    pub pan: Vec2,
    /// 鍵盤（WASD／方向鍵）平移的單位方向（螢幕座標，y 向下）；鏡頭以高度 × 0.9／秒換算（5.2）。
    pub pan_keys: Vec2,
    /// 拖曳（滑鼠左鍵、單指、雙指中點）本幀的像素位移（鏡頭依地面投影換算，以 `pointer` 為終點）。
    pub pan_pixels: Vec2,
    /// 縮放：本幀的「刻度數」，正值拉近（高度 × 0.88^刻度）。滾輪為整數刻度；搖桿為 `強度 × dt` 換算；觸控為捏合比。
    pub zoom: f32,
    /// 縮放的錨點（螢幕座標）；搖桿為畫面中央，觸控為雙指中點。
    pub zoom_anchor: Option<Vec2>,
    /// 旋轉方位角：本幀弧度，正值增加鏡頭方位角（搖桿已乘 dt）。
    pub yaw: f32,
    /// 指向位置：鍵鼠為游標；搖桿為準星（含吸附後的位置）；觸控為點擊／拖曳位置。
    pub pointer: Option<Vec2>,
    /// `pointer` 目前在介面（egui）之上；此時地圖不應懸停或選取。
    pub pointer_over_ui: bool,
    pub select_pressed: bool,
    pub back_pressed: bool,
    /// `F`／Y：關注目前主體。
    pub follow_pressed: bool,
    /// 右鍵點擊／X：開啟情境印環（5.7）。
    pub context_pressed: bool,
    /// `X`：溯因（開啟主體的最近事件）。
    pub trace_pressed: bool,
    /// `Enter`：原因簽的「飛往」。
    pub goto_notice_pressed: bool,
    pub pause_toggle_pressed: bool,
    pub speed_up_pressed: bool,
    pub speed_down_pressed: bool,
    /// 鍵盤 1–4 直接指定速度（同時解除暫停）。
    pub speed_set: Option<Speed>,
    pub mode_next_pressed: bool,
    pub mode_prev_pressed: bool,
    /// 鍵盤 F1–F6 直接指定地圖模式。
    pub mode_set: Option<MapMode>,
    pub labels_toggle_pressed: bool,
    pub recenter_pressed: bool,
    /// Ctrl + 1–9：記下目前視角為書籤（0–8）。
    pub bookmark_set: Option<u8>,
    /// Alt + 1–9：回到書籤（0–8）。
    pub bookmark_goto: Option<u8>,
    /// 檢視鍵（⧉）：開啟總覽選單。
    pub menu_pressed: bool,
    /// 十字鍵（地圖上）：上一則／下一則傳聞並飛過去；上下開啟傳聞欄。M3 以後有消費者。
    pub rumor_prev_pressed: bool,
    pub rumor_next_pressed: bool,
    pub rumor_toggle_pressed: bool,
    /// 介面有焦點時的 LB／RB：切換卡片分頁。
    pub tab_prev_pressed: bool,
    pub tab_next_pressed: bool,
}

/// 因果模式只顯示此魂的線（GDD 5.4「選取時」；由介面的「因」印寫入、渲染端讀取）。
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ThreadFocus(pub Option<siming_sim::SoulId>);
