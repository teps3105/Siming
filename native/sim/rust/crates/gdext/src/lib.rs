//! GDD 6.4 Godot 橋接：`SimWorld` 持有一個 [`Sim`]，Godot 端只經由本類別讀取世界、推進時間。
//! 本 crate 不含遊戲規則；規則一律在 siming-sim（GDD 6.1 依賴原則）。

mod karma;
mod mapbake;
mod relief;
mod stats_api;

use std::sync::mpsc::{self, Receiver, TryRecvError};

use godot::prelude::*;
use relief::Relief;
use siming_sim::save::{Replay, SaveFile};
use std::sync::atomic::{AtomicU32, Ordering};

use siming_sim::{calendar, Command, Era, FineGeography, Geography, Sim, WorldParams, WorldSize, NO_PROVINCE};

/// 山脈標籤的路徑：沿主軸每幾格取一段（GDD 5.3「文字標籤」）。
const RANGE_LABEL_SEGMENT: f32 = 4.0;

/// 治所的全名（GDD 7.3）：兩字名加上城池等級的字（0 村 .. 4 都）。
fn seat_name(stem: &str, city_level: u8) -> String {
    const SUFFIX: [&str; 5] = ["村", "鎮", "縣", "城", "都"];
    format!("{stem}{}", SUFFIX[(city_level as usize).min(SUFFIX.len() - 1)])
}

struct SimingExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SimingExtension {}

fn vec3s(v: &[[f32; 3]]) -> PackedVector3Array {
    v.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect()
}

fn vec2s(v: &[[f32; 2]]) -> PackedVector2Array {
    v.iter().map(|p| Vector2::new(p[0], p[1])).collect()
}

fn bytes(v: &[u8]) -> PackedByteArray {
    PackedByteArray::from(v)
}

/// 一個模擬世界。未呼叫 `create` 前所有查詢回傳空值。
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct SimWorld {
    sim: Option<Sim>,
    /// 細化後的高度場與跟著它走的貼圖、河流網格（選取也讀它）。
    land: Option<Land>,
    /// 靈氣與州界貼圖（每世結算後重算）。
    textures: Option<mapbake::WorldTextures>,
    /// 各州郡治所的兩字名（世界生成後算一次）。
    seat_stems: Vec<String>,
    /// 事件發生當月月底時主角的身分（推進時逐月記下，`drain_events` 取走）；快轉一次推進多月，
    /// 取走時人物的現況可能已不是事件當時的樣子（升了境界、換了掌門）。
    identities: std::collections::HashMap<siming_sim::EventId, Identity>,
    /// 有名字的海區與湖泊（世界生成後算一次）。
    waters: Vec<siming_sim::worldgen::NamedWater>,
    /// 進行中的讀檔重播（`load_begin` 之後、`load_step` 回報完成之前）。
    replay: Option<Replay>,
    /// `land` 是依哪一份水陸算出來的（`land_stamp`）。
    baked: u64,
    /// 背景執行緒上重算中的一批（GDD 2.8「畫面」）。
    rebake: Option<Rebake>,
    /// 高度場換過幾次；地圖顯示的資料（含州界、靈氣）換過幾次。
    /// `create_with` 已明確指定預跑長度（不套用 `set_default_prerun`）。
    explicit_prerun: bool,
    terrain_revision: i64,
    map_revision: i64,
    base: Base<RefCounted>,
}

fn size_from(name: &str) -> WorldSize {
    match name {
        "small" => WorldSize::Small,
        "large" => WorldSize::Large,
        _ => WorldSize::Medium,
    }
}

fn size_name(size: WorldSize) -> &'static str {
    match size {
        WorldSize::Small => "small",
        WorldSize::Medium => "medium",
        WorldSize::Large => "large",
    }
}

fn era_name(era: Era) -> &'static str {
    match era {
        Era::Ancient => "ancient",
        Era::Middle => "middle",
        Era::Decline => "decline",
    }
}

/// 跟著水陸走的一整套呈現資料：高度場、已轉好的高度貼圖與河流網格。
struct Land {
    relief: Relief,
    /// 高度貼圖：16 位元半精度，含整串多級縮圖。
    elevation: Vec<u8>,
    river: mapbake::RiverArrays,
}

impl Land {
    fn build(geo: &Geography, fine: &FineGeography) -> Self {
        let relief = relief::build_relief(geo, fine);
        let elevation = relief::half_mips(&relief.elevation, relief.w, relief.h);
        let river = mapbake::build_river_mesh(geo, &relief);
        Self { relief, elevation, river }
    }
}

/// 背景執行緒算好的一批：水陸沒變時 `land` 為空，只有靈氣與州界。
struct Fresh {
    land: Option<Land>,
    /// 水陸變了才重取的水域名。
    waters: Option<Vec<siming_sim::worldgen::NamedWater>>,
    textures: mapbake::WorldTextures,
}

/// 重算中的一批：`stamp` 是它依據的水陸，`with_land` 表示這一批含高度場。
struct Rebake {
    stamp: u64,
    with_land: bool,
    done: Receiver<Fresh>,
}

/// 高度場與河流網格讀到的水陸與河道的指紋（FNV-1a）：每世結算後拿來判斷要不要重算高度場。
fn land_stamp(world: &siming_sim::World) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |byte: u8| hash = (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
    world.fine.terrain.iter().for_each(|&t| feed(t as u8));
    world.geo.terrain.iter().for_each(|&t| feed(t as u8));
    world.geo.river.iter().for_each(|&r| feed(r));
    hash
}

/// 水域名（GDD 7.3）：以生成時的水域為準沿用；沒有生成時地形的手工世界就照目前的地形取名。
fn water_names(seed: u64, geo: &siming_sim::Geography, origin: Option<&[siming_sim::Terrain]>) -> Vec<siming_sim::worldgen::NamedWater> {
    match origin {
        Some(origin) => siming_sim::worldgen::name_waters_since(seed, geo, origin),
        None => siming_sim::worldgen::name_waters(seed, geo),
    }
}

/// 大洲名標在哪一格：重心還是陸地就用重心；被海淹了就改標在這個大洲離重心最近的陸格。
fn continent_label_cell(geo: &siming_sim::Geography, index: usize) -> u32 {
    let centroid = geo.continents[index].centroid as usize;
    if !geo.terrain[centroid].is_water() {
        return centroid as u32;
    }
    let w = geo.w as usize;
    let apart = |c: usize| {
        let dx = (c % w).abs_diff(centroid % w);
        let dy = (c / w).abs_diff(centroid / w);
        let dx = dx.min(w - dx);
        dx * dx + dy * dy
    };
    (0..geo.terrain.len())
        .filter(|&c| geo.continent_of[c] as usize == index && !geo.terrain[c].is_water())
        .min_by_key(|&c| (apart(c), c))
        .unwrap_or(centroid) as u32
}

impl SimWorld {
    /// 接手一個開好的世界：算出呈現端要用的高度場、貼圖與地名。
    fn adopt(&mut self, sim: Sim) {
        self.land = Some(Land::build(&sim.world().geo, &sim.world().fine));
        self.textures = Some(mapbake::build_world_textures(&sim.world().geo));
        self.baked = land_stamp(sim.world());
        self.rebake = None;
        self.terrain_revision += 1;
        self.map_revision += 1;
        let origin = sim.world().land_basis.as_ref().map(|basis| basis.cell_terrain());
        self.waters = water_names(sim.world().params.seed, &sim.world().geo, origin);
        self.seat_stems = siming_sim::worldgen::seat_stems(sim.world().params.seed, &sim.world().provinces, &sim.world().geo);
        self.identities.clear();
        self.sim = Some(sim);
    }

    /// 過了一世的結算之後，在背景執行緒重算呈現資料（GDD 2.8「畫面」）：靈氣與州界每次都算，
    /// 水陸變了才連高度場一起算。算好之前地圖維持舊的一整套，由 `poll_terrain` 一起換上；
    /// 還沒換上就又過了一世，舊的一批作廢。
    fn after_settle(&mut self) {
        let Some(sim) = self.sim.as_ref() else {
            return;
        };
        let stamp = land_stamp(sim.world());
        let with_land = stamp != self.baked;
        let geo = sim.world().geo.clone();
        let fine = with_land.then(|| sim.world().fine.clone());
        let seed = sim.world().params.seed;
        let origin = sim.world().land_basis.as_ref().map(|basis| basis.cell_terrain().to_vec());
        let (send, done) = mpsc::channel();
        std::thread::spawn(move || {
            let waters = fine.is_some().then(|| water_names(seed, &geo, origin.as_deref()));
            let land = fine.map(|fine| Land::build(&geo, &fine));
            let _ = send.send(Fresh { land, waters, textures: mapbake::build_world_textures(&geo) });
        });
        self.rebake = Some(Rebake { stamp, with_land, done });
    }

    fn take_rebake(&mut self, fresh: Fresh) {
        let Some(job) = self.rebake.take() else {
            return;
        };
        if let Some(waters) = fresh.waters {
            self.waters = waters;
        }
        if let Some(land) = fresh.land {
            self.land = Some(land);
            self.baked = job.stamp;
            self.terrain_revision += 1;
        }
        self.textures = Some(fresh.textures);
        self.map_revision += 1;
    }

    fn open(&mut self, mut params: WorldParams) -> GString {
        if params.name.is_empty() {
            params.name = siming_sim::worldgen::world_name(params.seed);
        }
        let (pop, pre) = (DEFAULT_POP_YEARS.load(Ordering::Relaxed), DEFAULT_PRERUN_YEARS.load(Ordering::Relaxed));
        if pop != UNSET && !self.explicit_prerun {
            params.pop_years = pop;
        }
        if pre != UNSET && !self.explicit_prerun {
            params.prerun_years = pre;
        }
        self.explicit_prerun = false;
        PROGRESS_STAGE.store(0, Ordering::Relaxed);
        PROGRESS_MILLI.store(0, Ordering::Relaxed);
        let mut progress = |stage: siming_sim::genesis::Stage, f: f32| {
            PROGRESS_STAGE.store(stage as u32 + 1, Ordering::Relaxed);
            PROGRESS_MILLI.store((f.clamp(0.0, 1.0) * 1000.0) as u32, Ordering::Relaxed);
        };
        match Sim::new_with_progress(&params, &mut |_, _| {}, &mut progress) {
            Ok(sim) => {
                self.adopt(sim);
                GString::new()
            }
            Err(e) => GString::from(e.to_string().as_str()),
        }
    }
}

const UNSET: u32 = u32::MAX;
static DEFAULT_POP_YEARS: AtomicU32 = AtomicU32::new(UNSET);
static DEFAULT_PRERUN_YEARS: AtomicU32 = AtomicU32::new(UNSET);
static PROGRESS_STAGE: AtomicU32 = AtomicU32::new(0);
static PROGRESS_MILLI: AtomicU32 = AtomicU32::new(0);

#[godot_api]
impl SimWorld {
    /// GDD 4.6 開天：世界生成 → 開天 → 預跑。`size` 為 `"small"`、`"medium"`、`"large"`。
    /// 失敗時回傳原因；成功回傳空字串。
    #[func]
    fn create(&mut self, seed: i64, size: GString) -> GString {
        let mut params = WorldParams::new(seed as u64);
        params.size = size_from(size.to_string().as_str());
        self.open(params)
    }

    /// GDD 4.6 開天，開局選項全部由 `options` 指定：`seed`、`name`、`size`、`land_ratio`、`relief`、
    /// `humidity`、`era`（`"ancient"`、`"middle"`、`"decline"`）；沒給的取預設值，數值夾在 4.6 的範圍內。
    #[func]
    fn create_with(&mut self, options: VarDictionary) -> GString {
        let seed = options.get("seed").and_then(|v| v.try_to::<i64>().ok()).unwrap_or(0);
        let mut params = WorldParams::new(seed as u64);
        let text = |key: &str| options.get(key).and_then(|v| v.try_to::<GString>().ok()).map(|s| s.to_string());
        let number = |key: &str| options.get(key).and_then(|v| v.try_to::<f64>().ok()).map(|n| n as f32);
        if let Some(name) = text("name") {
            params.name = name.trim().to_string();
        }
        if let Some(size) = text("size") {
            params.size = size_from(&size);
        }
        if let Some(era) = text("era") {
            params.era = match era.as_str() {
                "ancient" => Era::Ancient,
                "decline" => Era::Decline,
                _ => Era::Middle,
            };
        }
        let count = |key: &str| options.get(key).and_then(|v| v.try_to::<i64>().ok()).map(|n| n.clamp(0, 20_000) as u32);
        if let Some(n) = count("pop_years") {
            params.pop_years = n;
            self.explicit_prerun = true;
        }
        if let Some(n) = count("prerun_years") {
            params.prerun_years = n;
            self.explicit_prerun = true;
        }
        if let Some(v) = number("land_ratio") {
            params.land_ratio = v.clamp(0.25, 0.55);
        }
        if let Some(v) = number("relief") {
            params.relief = v.clamp(0.0, 1.0);
        }
        if let Some(v) = number("humidity") {
            params.humidity = v.clamp(0.6, 1.4);
        }
        self.open(params)
    }

    /// 之後開天的預設預跑長度（人口期年數、人物期年數）；測試用，讓每個案例的世界快速建好。
    /// 傳負數表示還原成世界參數的預設值。`create_with` 明確給的 `pop_years`、`prerun_years` 優先。
    #[func]
    fn set_default_prerun(pop_years: i64, prerun_years: i64) {
        let f = |n: i64| if n < 0 { UNSET } else { n.min(20_000) as u32 };
        DEFAULT_POP_YEARS.store(f(pop_years), Ordering::Relaxed);
        DEFAULT_PRERUN_YEARS.store(f(prerun_years), Ordering::Relaxed);
    }

    /// 開天進度（可在另一條執行緒建世界時輪詢）：`stage` 0 未開始、1 聚落、2 人口期、3 人物期；`fraction` 0..1。
    #[func]
    fn genesis_progress() -> VarDictionary {
        let mut d = VarDictionary::new();
        d.set("stage", PROGRESS_STAGE.load(Ordering::Relaxed) as i64);
        d.set("fraction", PROGRESS_MILLI.load(Ordering::Relaxed) as f64 / 1000.0);
        d
    }

    /// 這個世界的開局選項（鍵與 `create_with` 相同；`seed` 是品質檢查重試後實際使用的種子）。
    #[func]
    fn world_options(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some(sim) = self.sim.as_ref() {
            let p = &sim.world().params;
            d.set("seed", p.seed as i64);
            d.set("name", &GString::from(p.name.as_str()));
            d.set("size", &GString::from(size_name(p.size)));
            d.set("era", &GString::from(era_name(p.era)));
            d.set("land_ratio", p.land_ratio);
            d.set("relief", p.relief);
            d.set("humidity", p.humidity);
        }
        d
    }

    /// GDD 7.3 由種子取的世界名（開局設定畫面的「隨機」）。
    #[func]
    fn random_world_name(seed: i64) -> GString {
        GString::from(siming_sim::worldgen::world_name(seed as u64).as_str())
    }

    /// GDD 6.5 存檔的內容：種子、開局設定、指令紀錄、目標 tick 與雜湊（JSON 文字）。
    #[func]
    fn save_text(&self) -> GString {
        match self.sim.as_ref() {
            Some(sim) => GString::from(sim.save().to_json().as_str()),
            None => GString::new(),
        }
    }

    /// GDD 4.8 讀檔第一步：檢查存檔並重新開天。失敗回傳原因，成功回傳空字串；接著反覆呼叫 `load_step`。
    /// 讀檔完成前，原本的世界照舊可用。
    #[func]
    fn load_begin(&mut self, text: GString) -> GString {
        self.replay = None;
        let file = match SaveFile::from_json(text.to_string().as_str()) {
            Ok(file) => file,
            Err(e) => return GString::from(e.to_string().as_str()),
        };
        match Replay::start(file, Sim::new) {
            Ok(replay) => {
                self.replay = Some(replay);
                GString::new()
            }
            Err(e) => GString::from(e.to_string().as_str()),
        }
    }

    /// 讀檔第二步：最多重播 `max_ticks` 個月。回傳 `done`、`progress`（0..1）與 `error`（空字串為無誤）；
    /// 重播到存檔的時刻並通過雜湊比對後，世界換成讀出來的這一個。
    #[func]
    fn load_step(&mut self, max_ticks: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let mut error = String::new();
        let mut done = true;
        let mut progress = 1.0f32;
        match self.replay.as_mut() {
            None => error = "沒有進行中的讀檔".to_string(),
            Some(replay) => match replay.advance(max_ticks.max(1) as u32) {
                Ok(false) => {
                    done = false;
                    progress = replay.progress();
                }
                Ok(true) => match self.replay.take().expect("replay").finish() {
                    Ok(sim) => self.adopt(sim),
                    Err(e) => error = e.to_string(),
                },
                Err(e) => error = e.to_string(),
            },
        }
        if done {
            self.replay = None;
        }
        d.set("done", done);
        d.set("progress", progress);
        d.set("error", &GString::from(error.as_str()));
        d
    }

    #[func]
    fn is_ready(&self) -> bool {
        self.sim.is_some()
    }

    /// GDD 6.3 推進 `months` 個月。
    #[func]
    fn step(&mut self, months: i32) {
        let Some(sim) = self.sim.as_mut() else {
            return;
        };
        let mut settled = false;
        for _ in 0..months.max(0) {
            sim.step();
            settled |= siming_sim::sim::is_shi(sim.world().tick);
            note_identities(sim.world(), &mut self.identities);
        }
        if settled {
            self.after_settle();
        }
    }

    /// 高度場換過幾次（開天、讀檔與每次重算完成各加一）。高度、湖面、底色、覆蓋四張貼圖與河流網格跟著它換。
    #[func]
    fn terrain_revision(&self) -> i64 {
        self.terrain_revision
    }

    /// 地圖顯示的資料換過幾次：開天、讀檔，以及每世結算後的一批（州界、靈氣，水陸變了再加高度場）換上時各加一。
    #[func]
    fn map_revision(&self) -> i64 {
        self.map_revision
    }

    /// 高度場是否正在背景重算（GDD 2.8「畫面」）。
    #[func]
    fn terrain_pending(&self) -> bool {
        self.rebake.as_ref().is_some_and(|job| job.with_land)
    }

    /// 是否有一批呈現資料正在背景重算（含只算靈氣與州界的）。
    #[func]
    fn map_pending(&self) -> bool {
        self.rebake.is_some()
    }

    /// 每幀呼叫：背景重算好了就換上新的一批，回傳這次有沒有換。不等待。
    #[func]
    fn poll_terrain(&mut self) -> bool {
        let Some(job) = self.rebake.as_ref() else {
            return false;
        };
        match job.done.try_recv() {
            Ok(fresh) => {
                self.take_rebake(fresh);
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                godot_error!("高度場重算中斷，地圖維持原樣");
                self.rebake = None;
                false
            }
        }
    }

    /// 等背景重算完成並換上（測試與工具用；平時用 `poll_terrain`）。
    #[func]
    fn flush_terrain(&mut self) {
        let Some(job) = self.rebake.as_ref() else {
            return;
        };
        match job.done.recv() {
            Ok(fresh) => self.take_rebake(fresh),
            Err(_) => {
                godot_error!("高度場重算中斷，地圖維持原樣");
                self.rebake = None;
            }
        }
    }

    #[func]
    fn tick(&self) -> i64 {
        self.sim.as_ref().map_or(0, |s| s.world().tick as i64)
    }

    /// GDD 2.6 曆法全稱。
    #[func]
    fn date_text(&self) -> GString {
        match self.sim.as_ref() {
            Some(s) => GString::from(calendar::full(s.world().tick, s.world().evolution.history_years, &s.world().eras).as_str()),
            None => GString::new(),
        }
    }

    /// GDD 2.6 曆法簡稱（題簽用）。
    #[func]
    fn date_short(&self) -> GString {
        match self.sim.as_ref() {
            Some(s) => GString::from(calendar::era_date(s.world().tick, s.world().evolution.history_years, &s.world().eras).as_str()),
            None => GString::new(),
        }
    }

    /// GDD 2.8 寒暖與靈潮（天時儀的兩道弧）：`warmth`（−1..1）、`tide` 與下一世結算後的 `warmth_next`、`tide_next`；
    /// 所在時期 `climate`（Cold／Mild／Warm）、`aura`（Decline／Plain／Revival）；靈潮的範圍 `tide_min`、`tide_max`；
    /// 進入各時期的門檻 `warmth_enter`、`decline_enter`、`revival_enter`。
    #[func]
    fn climate(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(sim) = self.sim.as_ref() else {
            return d;
        };
        let w = sim.world();
        let e = &w.evolution;
        let next_tick = (w.tick / 10_800 + 1) * 10_800;
        let (warmth_next, tide_next) = e.forecast(e.history_years + siming_sim::evolution::year_of(next_tick));
        d.set("warmth", e.warmth);
        d.set("tide", e.tide);
        d.set("warmth_next", warmth_next);
        d.set("tide_next", tide_next);
        d.set("climate", &GString::from(format!("{:?}", e.climate).as_str()));
        d.set("aura", &GString::from(format!("{:?}", e.aura).as_str()));
        d.set("tide_min", siming_sim::params::EVO_TIDE_MIN);
        d.set("tide_max", siming_sim::params::EVO_TIDE_MAX);
        d.set("warmth_enter", siming_sim::params::EVO_WARMTH_ENTER);
        d.set("decline_enter", siming_sim::params::EVO_DECLINE_ENTER);
        d.set("revival_enter", siming_sim::params::EVO_REVIVAL_ENTER);
        d
    }

    /// GDD 2.8 開局前已過的年數：開天以來的年數 = 這個數 + 開局後的年數。
    #[func]
    fn history_years(&self) -> i64 {
        self.sim.as_ref().map_or(0, |s| i64::from(s.world().evolution.history_years))
    }

    /// GDD 6.5 世界雜湊，十六進位 16 字元。
    #[func]
    fn hash_hex(&self) -> GString {
        match self.sim.as_ref() {
            Some(s) => GString::from(format!("{:016x}", s.hash()).as_str()),
            None => GString::new(),
        }
    }

    // ── 地圖資料（GDD 5.3）──────────────────────────────────────

    /// 遊戲格的寬、高。地圖的世界座標以格為單位：x 向東、z 向南。
    #[func]
    fn grid_size(&self) -> Vector2i {
        self.sim.as_ref().map_or(Vector2i::ZERO, |s| Vector2i::new(s.world().geo.w as i32, s.world().geo.h as i32))
    }

    /// 陸地最高點的網格高度（高度縮放 = 1）。
    #[func]
    fn terrain_max_height(&self) -> f32 {
        self.land.as_ref().map_or(0.0, |l| l.relief.max_height())
    }

    /// 世界座標 (x, z) 的地形高度（高度縮放 = 1）。
    #[func]
    fn terrain_height_at(&self, x: f32, z: f32) -> f32 {
        self.land.as_ref().map_or(0.0, |l| l.relief.height_at(x, z))
    }

    /// 地形網格陣列：`positions`（平面格網，高度由著色器取高度貼圖）、`uvs`、`indices`。
    #[func]
    fn terrain_mesh(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(sim) = self.sim.as_ref() else {
            return d;
        };
        let geo = &sim.world().geo;
        let m = mapbake::build_terrain_mesh(geo.w as usize, geo.h as usize);
        d.set("positions", &vec3s(&m.positions));
        d.set("uvs", &vec2s(&m.uvs));
        d.set("indices", &PackedInt32Array::from(m.indices.as_slice()));
        d
    }

    /// 河流帶狀網格陣列：`positions`、`tangents`、`side_level`、`taper`、`indices`（GDD 5.3「河流」）。
    #[func]
    fn river_mesh(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(land) = self.land.as_ref() else {
            return d;
        };
        let r = &land.river;
        d.set("positions", &vec3s(&r.positions));
        d.set("tangents", &vec3s(&r.tangents));
        d.set("side_level", &vec2s(&r.side_level));
        d.set("taper", &vec2s(&r.taper));
        d.set("indices", &PackedInt32Array::from(r.indices.as_slice()));
        d
    }

    /// 世界貼圖的原始資料（GDD 5.3「貼圖」，工具與測試用）：
    /// `elevation`（相對海平面的高度，`PackedFloat32Array`）與 `elevation_size`、`lake`（R8，同尺寸）、
    /// `climate`（RGB8）、`cover`（RG8，尺寸同 `climate`：R 冰雪、G 沙土）與 `climate_size`、`sea_level`、`aura`（R8，格 × `aura_scale`）、`province_id`（RG8，格）。
    #[func]
    fn map_textures(&self) -> VarDictionary {
        let mut d = self.relief_textures();
        let Some(land) = self.land.as_ref() else {
            return d;
        };
        d.set("elevation", &PackedFloat32Array::from(land.relief.elevation.as_slice()));
        d.extend_dictionary(&self.data_textures(), true);
        d
    }

    /// 跟著高度場換的貼圖（地圖用）：`elevation_half`（16 位元半精度，含整串多級縮圖）與 `elevation_size`、
    /// `lake`（R8，同尺寸）、`climate`（RGB8）、`cover`（RG8）與 `climate_size`、`sea_level`。
    #[func]
    fn relief_textures(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(land) = self.land.as_ref() else {
            return d;
        };
        let relief = &land.relief;
        d.set("elevation_half", &bytes(&land.elevation));
        d.set("elevation_size", Vector2i::new(relief.w as i32, relief.h as i32));
        d.set("lake", &bytes(&relief.lake));
        d.set("climate", &bytes(&relief.climate));
        d.set("cover", &bytes(&relief.cover));
        d.set("climate_size", Vector2i::new(relief.climate_w as i32, relief.climate_h as i32));
        d.set("sea_level", relief.sea_level);
        d
    }

    /// 每世結算後換的兩張貼圖：`aura`（R8，格 × `aura_scale`）與 `province_id`（RG8，格）。
    #[func]
    fn data_textures(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(t) = self.textures.as_ref() else {
            return d;
        };
        d.set("aura_scale", mapbake::SS as i32);
        d.set("aura", &bytes(&t.aura));
        d.set("province_id", &bytes(&t.province_id));
        d
    }

    /// 紙紋：512×512 RGBA8（與世界種子無關，GDD 8.3）。
    #[func]
    fn paper_texture() -> PackedByteArray {
        bytes(&mapbake::gen_paper())
    }

    /// 州郡 ID → 淡彩的 256×1 RGBA8 查表（天下模式，GDD 5.4）。
    #[func]
    fn province_colors(&self) -> PackedByteArray {
        self.sim.as_ref().map_or_else(PackedByteArray::new, |s| bytes(&mapbake::province_color_table(&s.world().provinces)))
    }

    /// 三教模式的州郡著色表（GDD 5.4 F3）：州郡 ID → 淡彩的 256×1 RGBA8；alpha 為 0 的州郡不上色。
    #[func]
    fn faith_colors(&self) -> PackedByteArray {
        self.sim.as_ref().map_or_else(PackedByteArray::new, |s| bytes(&mapbake::faith_color_table(&s.world().provinces)))
    }

    /// 世界座標 (x, z) 所在格的州郡 ID；水域或圖外為 -1。
    #[func]
    fn province_at(&self, x: f32, z: f32) -> i32 {
        let Some(s) = self.sim.as_ref() else {
            return -1;
        };
        let geo = &s.world().geo;
        let (w, h) = (geo.w as i32, geo.h as i32);
        let gz = z.floor() as i32;
        if gz < 0 || gz >= h {
            return -1;
        }
        let gx = if geo.wrap_x { (x.floor() as i32).rem_euclid(w) } else { x.floor() as i32 };
        if gx < 0 || gx >= w {
            return -1;
        }
        match geo.province_of[(gz * w + gx) as usize] {
            NO_PROVINCE => -1,
            p => p as i32,
        }
    }

    #[func]
    fn province_count(&self) -> i32 {
        self.sim.as_ref().map_or(0, |s| s.world().provinces.len() as i32)
    }

    /// 州郡摘要（GDD 2.3、5.8 州郡卡）：`name`、`population`、`capacity`、`aura`、`karma_qi`、
    /// `merit`、`righteous`、`devastation`、`city_level`、`faith`（道、釋、儒）、`seat`（治所格心的世界座標）、
    /// `cells`（格數）、`seat_name`（治所名）、`terrain_mix`（各地形的格數，索引為地形代碼 0 海 .. 8 澤）。
    #[func]
    fn province_info(&self, id: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let w = s.world();
        if id < 0 {
            return d;
        }
        let Some(p) = w.provinces.get(id as usize) else {
            return d;
        };
        let gw = w.geo.w as u32;
        d.set("name", &GString::from(p.name.as_str()));
        d.set("population", p.population);
        d.set("capacity", p.capacity);
        d.set("aura", p.aura);
        d.set("karma_qi", p.karma_qi);
        d.set("merit", p.merit);
        d.set("righteous", p.righteous);
        d.set("devastation", p.devastation);
        d.set("city_level", p.city_level as i32);
        d.set("faith", Vector3::new(p.faith[0], p.faith[1], p.faith[2]));
        d.set("seat", Vector2::new((p.seat % gw) as f32 + 0.5, (p.seat / gw) as f32 + 0.5));
        d.set("cells", p.cells.len() as i32);
        d.set("seat_name", &GString::from(seat_name(self.seat_stems.get(p.id as usize).map_or("", |s| s.as_str()), p.city_level).as_str()));
        let mix: Vec<i32> = p.terrain_mix.iter().map(|&n| n as i32).collect();
        d.set("terrain_mix", &PackedInt32Array::from(mix.as_slice()));
        d
    }

    /// 地圖標籤（GDD 5.3「文字標籤」）：每筆有 `kind`（`province`、`river`、`range`、`node`、`continent`、`ocean`、`sea`、`lake`）、
    /// `text`、`pos`（世界座標 x、z）、`rank`（越大越重要）與 `path`（江河的幹流、山脈的主軸；其餘為空）。
    #[func]
    fn map_labels(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else {
            return out;
        };
        let w = s.world();
        let (gw, gwf) = (w.geo.w as u32, w.geo.w as f32);
        let centre = |cell: u32| Vector2::new((cell % gw) as f32 + 0.5, (cell / gw) as f32 + 0.5);
        let mut push = |kind: &str, text: &str, pos: Vector2, rank: f32, path: PackedVector2Array| {
            let mut d = VarDictionary::new();
            d.set("kind", &GString::from(kind));
            d.set("text", &GString::from(text));
            d.set("pos", pos);
            d.set("rank", rank);
            d.set("path", &path);
            out.push(&d);
        };
        let none = PackedVector2Array::new;
        for (i, c) in w.geo.continents.iter().enumerate() {
            push("continent", &c.name, centre(continent_label_cell(&w.geo, i)), 100.0 + c.area, none());
        }
        for p in &w.provinces {
            push(
                "province",
                &p.name,
                centre(mapbake::province_label_cell(&p.cells, w.geo.w as usize, w.geo.h as usize)),
                p.city_level as f32,
                none(),
            );
        }
        for water in &self.waters {
            use siming_sim::worldgen::WaterKind;
            let (kind, rank) = match water.kind {
                WaterKind::Ocean => ("ocean", 60.0 + water.area as f32 * 0.001),
                WaterKind::Sea => ("sea", 30.0 + water.area as f32 * 0.001),
                WaterKind::Lake => ("lake", 8.0 + water.area as f32 * 0.01),
            };
            push(kind, &water.name, centre(water.cell), rank, none());
        }
        for r in &w.geo.rivers {
            if let Some(&mid) = r.cells.get(r.cells.len() / 2) {
                // 幹流的格心，由源頭到出海口；跨東西接縫時接到最近的副本，折線才連續。
                let mut x_prev = centre(r.cells[0]).x;
                let path: PackedVector2Array = r
                    .cells
                    .iter()
                    .map(|&c| {
                        let p = centre(c);
                        x_prev = p.x + gwf * ((x_prev - p.x) / gwf).round();
                        Vector2::new(x_prev, p.y)
                    })
                    .collect();
                push("river", &r.name, centre(mid), r.level as f32 * 10.0 + r.cells.len() as f32 * 0.01, path);
            }
        }
        for r in &w.geo.ranges {
            // 山脈的走向：以最高峰為原點，取山脈格的主軸（共變異數的主方向）。
            let peak = centre(r.peak);
            let rel: Vec<(f32, f32)> = r
                .cells
                .iter()
                .map(|&c| {
                    let p = centre(c);
                    let dx = p.x - peak.x;
                    (dx - gwf * (dx / gwf).round(), p.y - peak.y)
                })
                .collect();
            let n = rel.len().max(1) as f32;
            let (mx, my) = rel.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
            let (mut sxx, mut sxy, mut syy) = (0.0f32, 0.0f32, 0.0f32);
            for p in &rel {
                let (dx, dy) = (p.0 - mx, p.1 - my);
                sxx += dx * dx;
                sxy += dx * dy;
                syy += dy * dy;
            }
            let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
            let (ax, ay) = (angle.cos(), angle.sin());
            let (mut lo, mut hi) = (0.0f32, 0.0f32);
            for p in &rel {
                let t = (p.0 - mx) * ax + (p.1 - my) * ay;
                lo = lo.min(t);
                hi = hi.max(t);
            }
            // 沿主軸分段，每段取該段山脈格的重心連成折線：山脈彎曲時，折線跟著山勢走，不會落到山外。
            let segments = ((hi - lo) / RANGE_LABEL_SEGMENT).round().clamp(3.0, 9.0) as usize;
            let mut sums = vec![(0.0f32, 0.0f32, 0u32); segments];
            for p in &rel {
                let t = (p.0 - mx) * ax + (p.1 - my) * ay;
                let i = (((t - lo) / (hi - lo).max(1e-3)) * segments as f32) as usize;
                let cell = &mut sums[i.min(segments - 1)];
                *cell = (cell.0 + p.0, cell.1 + p.1, cell.2 + 1);
            }
            let path: PackedVector2Array = if hi - lo >= 1.0 {
                sums.iter().filter(|c| c.2 > 0).map(|c| Vector2::new(peak.x + c.0 / c.2 as f32, peak.y + c.1 / c.2 as f32)).collect()
            } else {
                none()
            };
            push("range", &r.name, peak, 10.0 + r.cells.len() as f32 * 0.01, path);
        }
        for n in &w.geo.nodes {
            push("node", &n.name, centre(n.cell), n.strength * 20.0, none());
        }
        out
    }

    /// 靈脈（GDD 2.4、5.4）：每條有 `strength` 與 `path`（沿線格心的世界座標 x、z）。
    #[func]
    fn ley_lines(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else {
            return out;
        };
        let geo = &s.world().geo;
        let gw = geo.w as u32;
        for e in &geo.ley {
            let path: PackedVector2Array = e.path.iter().map(|&c| Vector2::new((c % gw) as f32 + 0.5, (c / gw) as f32 + 0.5)).collect();
            let mut d = VarDictionary::new();
            d.set("strength", e.strength);
            d.set("path", &path);
            out.push(&d);
        }
        out
    }

    /// 洞天福地（GDD 2.4）：每個有 `name`、`strength` 與 `pos`。
    #[func]
    fn ley_nodes(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else {
            return out;
        };
        let geo = &s.world().geo;
        let gw = geo.w as u32;
        for n in &geo.nodes {
            let mut d = VarDictionary::new();
            d.set("name", &GString::from(n.name.as_str()));
            d.set("strength", n.strength);
            d.set("pos", Vector2::new((n.cell % gw) as f32 + 0.5, (n.cell / gw) as f32 + 0.5));
            out.push(&d);
        }
        out
    }

    /// 宗門（GDD 3.5、5.5）：每個含 `id`、`name`、`pos`（山門所在格心的世界座標）、`province`（山門所在州郡）、
    /// `node`（山門所在洞天福地名，無則空字串）、`members`（成員數）、`prestige`（有效聲望）、`radius`（勢力範圍半徑，步）、
    /// `range`（勢力範圍內的州郡 ID，`PackedInt32Array`）、`range_radius`（範圍圈半徑，格：範圍內州郡格數之和的等面積圓半徑）、`evil`（邪道）、`alive`（是否還在）。已覆滅者也在清單內。
    #[func]
    fn sects(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else {
            return out;
        };
        let w = s.world();
        let gw = w.geo.w as u32;
        for p in &w.powers {
            let Some(sect) = siming_sim::dao::sect_of(p) else {
                continue;
            };
            let mut d = VarDictionary::new();
            d.set("id", p.id as i32);
            d.set("name", &GString::from(p.name.as_str()));
            d.set("pos", Vector2::new((p.seat_cell % gw) as f32 + 0.5, (p.seat_cell / gw) as f32 + 0.5));
            d.set("province", p.seat_province as i32);
            let node = sect.seat_node.and_then(|n| w.geo.nodes.get(n as usize)).map_or("", |n| n.name.as_str());
            d.set("node", &GString::from(node));
            d.set("members", p.members.len() as i32);
            d.set("prestige", siming_sim::dao::effective_prestige(p));
            d.set("radius", sect.radius as i32);
            let range_ids = siming_sim::dao::sect_range(w, p.id);
            let range: Vec<i32> = range_ids.iter().map(|&q| q as i32).collect();
            // 範圍圈半徑：範圍內各州郡格數之和的等面積圓半徑（格）。
            let cells: usize = range_ids.iter().filter_map(|&q| w.provinces.get(q as usize)).map(|q| q.cells.len()).sum();
            d.set("range_radius", (cells as f32 / std::f32::consts::PI).sqrt());
            d.set("range", &PackedInt32Array::from(range.as_slice()));
            d.set("evil", sect.evil);
            d.set("alive", p.fallen.is_none());
            out.push(&d);
        }
        out
    }

    // ── 人物卡（GDD 5.8）──────────────────────────────────────

    /// 人物卡的基本資料；沒有這個人時回傳空字典。欄位：
    /// - `name`、`title`（道號等，沒有為空字串）、`gender`（`Male`／`Female`）、`age`（身故者為享年）、`lifespan`（壽元上限）、`alive`；
    /// - `died_tick`、`death_cause`（`DeathCause` 的代號，對應 `death_*` 語系鍵；在世為 -1、空字串）；
    /// - `province`／`province_name`（所在，身故者為歿處）、`home`／`home_name`（故里）、`pos`（所在格心的地圖座標）；
    /// - `role`（`Role` 代號）、`power`（勢力編號，無為 -1）、`power_name`、`is_leader`（是否為該勢力首領）；
    /// - `realm`（修士境界，非修士為 -1）、`fruit`（僧的果位，非僧為 -1）；
    /// - `root`、`insight`、`talent`、`virtue`、`ferocity`（資質）；
    /// - `soul`（魂編號）、`life_index`（這是魂的第幾世，從 1 起算）、`lives_total`（魂已有的世數）、
    ///   `merit`、`sin`、`wisdom`（魂的功德、業障、宿慧）；
    /// - `last_event`：此人最近一則事件的編號（沒有為 -1；修剪後指向最近保留的祖先）。
    #[func]
    fn person(&self, id: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let w = s.world();
        let Some(p) = usize::try_from(id).ok().and_then(|i| w.persons.get(i)) else {
            return d;
        };
        let gw = w.geo.w as u32;
        let province_name = |id: siming_sim::ProvinceId| w.provinces.get(id as usize).map_or("", |p| p.name.as_str());
        d.set("id", id);
        d.set("name", &GString::from(p.name.as_str()));
        d.set("title", &GString::from(p.title.as_deref().unwrap_or("")));
        d.set("gender", &GString::from(format!("{:?}", p.gender).as_str()));
        d.set("age", p.age(w.tick) as i32);
        d.set("lifespan", p.lifespan as i32);
        d.set("alive", p.alive());
        d.set("died_tick", p.died.map_or(-1, |(t, _)| t as i64));
        d.set("death_cause", &GString::from(p.died.map_or(String::new(), |(_, c)| format!("{c:?}")).as_str()));
        d.set("province", p.province as i32);
        d.set("province_name", &GString::from(province_name(p.province)));
        d.set("home", p.home as i32);
        d.set("home_name", &GString::from(province_name(p.home)));
        d.set("pos", Vector2::new((p.cell % gw) as f32 + 0.5, (p.cell / gw) as f32 + 0.5));
        d.set("role", &GString::from(format!("{:?}", p.role).as_str()));
        let power = p.power.and_then(|pw| w.powers.get(pw as usize));
        d.set("power", power.map_or(-1, |pw| pw.id as i32));
        d.set("power_name", &GString::from(power.map_or("", |pw| pw.name.as_str())));
        d.set("is_leader", power.is_some_and(|pw| pw.leader == Some(p.id)));
        let (realm, fruit) = match p.path {
            siming_sim::PathState::Cultivator { realm, .. } => (realm as i32, -1),
            siming_sim::PathState::Monk { fruit, .. } => (-1, fruit as i32),
            _ => (-1, -1),
        };
        d.set("realm", realm);
        d.set("fruit", fruit);
        d.set("root", p.attrs.root);
        d.set("insight", p.attrs.insight);
        d.set("talent", p.attrs.talent);
        d.set("virtue", p.attrs.virtue);
        d.set("ferocity", p.attrs.ferocity);
        d.set("soul", p.soul as i32);
        d.set("last_event", p.last_event.map_or(-1, |e| e as i64));
        if let Some(soul) = w.souls.get(p.soul as usize) {
            d.set("life_index", soul.lives.iter().position(|&l| l == p.id).map_or(1, |i| i as i32 + 1));
            d.set("lives_total", soul.lives.len() as i32);
            d.set("merit", soul.merit);
            d.set("sin", soul.sin);
            d.set("wisdom", soul.wisdom);
        }
        d
    }

    /// 這個人牽涉的事件，由舊到新（依 tick，同 tick 依事件編號）；修剪過的墓碑不算。身故者只算到死為止。
    /// 每筆有 `id`、`tick`、`kind`、`importance`、`province`（無為 -1）、`pos`（無地點沒有此鍵）、
    /// `era`（當時的紀元名）、`year`（紀元內年份）、`month`，以及與 `drain_events` 相同的文字欄位
    /// `a`、`b`、`f`、`g`、`p`、`q`、`x`、`n`、`m`；另有 `a_person`、`b_person`（兩位人物的編號，無為 -1）
    /// 與 `q_province`（`q` 那個州郡的編號，無為 -1），供介面把人名與地名做成可點。
    #[func]
    fn person_history(&self, id: i32) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else { return out };
        let w = s.world();
        let Some(p) = usize::try_from(id).ok().and_then(|i| w.persons.get(i)) else {
            return out;
        };
        // 只記了魂的事件（判入六道）只算本世存在期間內的。
        let mut events: Vec<&siming_sim::Event> = w.ledger.events.iter().filter(|e| e.importance > 0 && belongs_to(e, p)).collect();
        events.sort_by_key(|e| (e.tick, e.id));
        let gw = w.geo.w as u32;
        // 故里日後可能變：遊歷是否為歸來，改看它是不是回到上一趟的出發地，或回到現在的故里。
        let mut away_from: Option<siming_sim::ProvinceId> = None;
        for e in events {
            let back = match e.args {
                siming_sim::EventArgs::Travel { to, .. } => Some(away_from == Some(to) || p.home == to),
                _ => None,
            };
            let mut d = self.event_fields(w, e, back);
            if let siming_sim::EventArgs::Travel { from, .. } = e.args {
                away_from = if back == Some(true) { None } else { Some(from) };
            }
            d.set("id", e.id as i64);
            d.set("tick", e.tick as i64);
            d.set("kind", &GString::from(format!("{:?}", e.kind).as_str()));
            d.set("importance", e.importance as i32);
            d.set("province", e.province.map_or(-1, |p| p as i32));
            let seat = e.province.and_then(|p| w.provinces.get(p as usize)).map(|p| p.seat);
            if let Some(c) = e.cell.or(seat) {
                d.set("pos", Vector2::new((c % gw) as f32 + 0.5, (c / gw) as f32 + 0.5));
            }
            let (era, year) = siming_sim::calendar::era_year(e.tick, w.evolution.history_years, &w.eras);
            d.set("era", &GString::from(era.name.as_str()));
            d.set("year", year as i32);
            d.set("month", siming_sim::calendar::date(e.tick).month as i32);
            set_day_and_cause(&mut d, e);
            out.push(&d);
        }
        out
    }

    /// 某州郡現在在世的有名之人，依境界由高到低、同境界依編號：`id`、`name`、`role`（`Role` 代號）、
    /// `realm`（修士境界，其他為 -1）、`fruit`（僧的果位，其他為 -1）、`power_name`、`is_leader`、`age`。
    #[func]
    fn people_in(&self, province: i32) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else { return out };
        let w = s.world();
        let mut rows: Vec<(i32, &siming_sim::Person)> = w
            .alive_ids
            .iter()
            .filter_map(|&i| w.persons.get(i as usize))
            .filter(|p| p.alive() && p.province as i32 == province)
            .map(|p| match p.path {
                siming_sim::PathState::Cultivator { realm, .. } => (realm as i32, p),
                _ => (-1, p),
            })
            .collect();
        rows.sort_by_key(|(realm, p)| (std::cmp::Reverse(*realm), p.id));
        for (realm, p) in rows {
            let mut d = VarDictionary::new();
            let power = p.power.and_then(|pw| w.powers.get(pw as usize));
            d.set("id", p.id as i32);
            d.set("name", &GString::from(p.name.as_str()));
            d.set("role", &GString::from(format!("{:?}", p.role).as_str()));
            d.set("realm", realm);
            let fruit = if let siming_sim::PathState::Monk { fruit, .. } = p.path { fruit as i32 } else { -1 };
            d.set("fruit", fruit);
            d.set("power_name", &GString::from(power.map_or("", |pw| pw.name.as_str())));
            d.set("is_leader", power.is_some_and(|pw| pw.leader == Some(p.id)));
            d.set("age", p.age(w.tick) as i32);
            out.push(&d);
        }
        out
    }

    // ── 因果模式（GDD 5.4）──────────────────────────────────────

    /// 因果模式要畫的東西，一次取回（取捨規則與位置的決定方式見 `karma.rs`）。可選的 `options`：
    /// `all`（畫全部的線，預設否）、`min_strength`（預設 30）、`max_lines`（預設 1500）、`max_souls`（預設 4000）。
    /// 魂光（每位在世有名之人一點）：`glow_person`（人物編號）、`glow_pos`（地圖座標）、`glow_height`（地表高度）、
    /// `glow_province`、`glow_karma`（淨業力 G − S）。因果線：`line_thread`（宿緣編號）、`line_kind`（0 殺業、1 恩義、2 情緣、3 師徒、4 怨）、
    /// `line_strength`、`line_from`／`line_to`（欠債方、債主一端的地圖座標；不在世的一端取最後所在地）、`line_from_height`／`line_to_height`、
    /// `line_debtor`／`line_creditor`（兩端在世的人物編號，不在世為 -1）、`line_dead`（0 兩端在世、1 欠債方不在世、2 債主不在世）、`line_woven`（司命所牽）。
    /// 另有 `qualified_lines`（取捨前符合條件的線數）、`total_glows`（在世有名之人數）。
    #[func]
    fn karma_view(&self, options: VarDictionary) -> VarDictionary {
        let Some(s) = self.sim.as_ref() else { return VarDictionary::new() };
        let mut opts = karma::Options::default();
        if let Some(v) = options.get("all") {
            opts.all = v.try_to::<bool>().unwrap_or(false);
        }
        if let Some(v) = options.get("min_strength") {
            opts.min_strength = v.try_to::<f32>().unwrap_or(opts.min_strength);
        }
        if let Some(v) = options.get("max_lines") {
            opts.max_lines = v.try_to::<i64>().map_or(opts.max_lines, |n| n.max(0) as usize);
        }
        if let Some(v) = options.get("max_souls") {
            opts.max_souls = v.try_to::<i64>().map_or(opts.max_souls, |n| n.max(0) as usize);
        }
        let view = karma::view(s.world(), opts);
        let mut d = VarDictionary::new();
        let glows = &view.glows;
        d.set("glow_person", &PackedInt32Array::from(glows.iter().map(|g| g.person as i32).collect::<Vec<_>>().as_slice()));
        d.set("glow_province", &PackedInt32Array::from(glows.iter().map(|g| g.province as i32).collect::<Vec<_>>().as_slice()));
        d.set("glow_pos", &vec2s(&glows.iter().map(|g| [g.pos.0, g.pos.1]).collect::<Vec<_>>()));
        d.set("glow_height", &PackedFloat32Array::from(glows.iter().map(|g| self.height_at(g.pos)).collect::<Vec<_>>().as_slice()));
        d.set("glow_karma", &PackedFloat32Array::from(glows.iter().map(|g| g.karma).collect::<Vec<_>>().as_slice()));
        self.pack_lines(&mut d, &view.lines);
        d.set("qualified_lines", view.qualified_lines as i32);
        d.set("total_glows", view.total_glows as i32);
        d
    }

    /// 指定的宿緣（編號）各畫成一條線，欄位同 `karma_view` 的 `line_*`；已了結、不存在或兩端都不在世的略過。
    #[func]
    fn karma_threads(&self, ids: PackedInt32Array) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let ids: Vec<u32> = ids.as_slice().iter().filter(|&&i| i >= 0).map(|&i| i as u32).collect();
        self.pack_lines(&mut d, &karma::threads(s.world(), &ids));
        d
    }

    /// 兩端的魂都屬於這組人的未了宿緣編號（事件卡溯因視圖用：這些人之間的線）。
    #[func]
    fn karma_between(&self, people: PackedInt32Array) -> PackedInt32Array {
        let Some(s) = self.sim.as_ref() else { return PackedInt32Array::new() };
        let ids: Vec<u32> = people.as_slice().iter().filter(|&&i| i >= 0).map(|&i| i as u32).collect();
        let out: Vec<i32> = karma::threads_between(s.world(), &ids).into_iter().map(|t| t as i32).collect();
        PackedInt32Array::from(out.as_slice())
    }

    /// 選取某人的魂光時要顯示的東西：`threads`（這個魂牽著的未了宿緣編號，強度高者在前）、
    /// `past`（前世所在地點的地圖座標，由近到遠）、`past_height`（各點的地表高度）。
    #[func]
    fn karma_focus(&self, person: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let Ok(id) = u32::try_from(person) else { return d };
        let w = s.world();
        let threads: Vec<i32> = karma::threads_of(w, id).into_iter().map(|t| t as i32).collect();
        d.set("threads", &PackedInt32Array::from(threads.as_slice()));
        let past = karma::past_places(w, id);
        d.set("past", &vec2s(&past.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>()));
        d.set("past_height", &PackedFloat32Array::from(past.iter().map(|&p| self.height_at(p)).collect::<Vec<_>>().as_slice()));
        d
    }

    // ── 統計 ────────────────────────────────────────────────────

    #[func]
    fn total_population(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.world().provinces.iter().map(|p| p.population).sum())
    }

    /// 在世有名之人數（GDD 3.4）。
    #[func]
    fn alive_person_count(&self) -> i32 {
        self.sim.as_ref().map_or(0, |s| s.world().alive_ids.len() as i32)
    }

    // ── 指令與事件（GDD 4.7、6.3）───────────────────────────────

    /// 關注或取消關注一個魂；在下一個 tick 開始前生效。
    #[func]
    fn follow(&mut self, soul: i64, on: bool) {
        if let Some(sim) = self.sim.as_mut() {
            sim.apply(Command::Follow { soul: soul as _, on });
        }
    }

    /// 取走上次呼叫以來的新事件。每筆都有：
    /// - `id`、`tick`、`kind`（事件代號）、`importance`、`province`（無則 -1）、`cell`（無則 -1）；
    /// - `pos`：事件地點的地圖座標（格中心，沒有格時取州郡治所；無地點則沒有這個鍵）；
    /// - `souls`：牽涉的魂（`PackedInt32Array`）；
    /// - `year`：事件當時的紀元內年份（與題簽同一套，從 1 起算）；
    /// - 7.2 模板要填的字：`a`、`b`（人名）、`f`、`g`（勢力名）、`p`（州郡名）、`q`（第二個州郡名）、
    ///   `x`（依事件而定的代號字串）、`n`、`m`（依事件而定的整數），沒有的給空字串或 0。
    /// 世界大事（`WorldChanged`）另有 `change`（大事代號）與 `era`（這件大事開啟的紀元名，沒改元為空字串）。
    #[func]
    fn drain_events(&mut self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(sim) = self.sim.as_mut() else {
            return out;
        };
        let ids = sim.drain_events();
        let identities = std::mem::take(&mut self.identities);
        let Some(sim) = self.sim.as_ref() else {
            return out;
        };
        let w = sim.world();
        for id in ids {
            let Some(e) = siming_sim::ledger::get(w, id) else {
                continue;
            };
            // 主角（{A}）的身分：事件發生當月月底的樣子；沒有記下（讀檔重播的事件）才取現況。
            let d = self.event_record(w, e, identities.get(&e.id).cloned());
            out.push(&d);
        }
        out
    }

    // ── 事件卡（溯因視圖，GDD 5.8）──────────────────────────────

    /// 一則事件，欄位與 `drain_events` 的每筆相同（身分欄 `a_power`、`a_role`、`a_realm` 取人物現況）；
    /// 沒有這則事件或已被修剪（墓碑）時回傳空字典。
    #[func]
    fn event(&self, id: i64) -> VarDictionary {
        let Some(sim) = self.sim.as_ref() else { return VarDictionary::new() };
        let w = sim.world();
        match event_by_id(w, id) {
            Some(e) => self.event_record(w, e, None),
            None => VarDictionary::new(),
        }
    }

    /// 事件 `id` 的「因」樹，沿 `causes` 往上最多 `depth` 層，每個節點最多取 `CAUSE_FANOUT` 個因（依重要度，同級取較晚的），
    /// 同一事件在樹中只出現一次；以先根順序攤平。每筆是 `event` 的欄位再加 `level`（1 為直接因）、
    /// `parent`（上一層節點在本陣列的位置，直接因為 -1）。事件不存在或已被修剪時為空陣列。
    #[func]
    fn event_causes(&self, id: i64, depth: i32) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(sim) = self.sim.as_ref() else { return out };
        let w = sim.world();
        let Some(root) = event_by_id(w, id) else { return out };
        for node in cause_tree(w, root.id, depth.max(0) as usize, CAUSE_FANOUT) {
            let Some(e) = siming_sim::ledger::get(w, node.id) else { continue };
            let mut d = self.event_record(w, e, None);
            d.set("level", node.level as i32);
            d.set("parent", node.parent);
            out.push(&d);
        }
        out
    }

    /// 由事件 `id` 引出的後續事件數（沿反向索引往下，最多 `RIPPLE_CAP` 筆，到頂即為 `RIPPLE_CAP`）。
    /// 事件不存在或已被修剪為 0。
    #[func]
    fn event_ripple(&self, id: i64) -> i32 {
        let Some(sim) = self.sim.as_ref() else { return 0 };
        let w = sim.world();
        event_by_id(w, id).map_or(0, |e| ripple_ids(w, e.id, RIPPLE_CAP).len() as i32)
    }

    /// 由事件 `id` 引出的後續事件，依時間由舊到新，最多 `RIPPLE_CAP` 筆；每筆欄位同 `event`。
    #[func]
    fn event_ripples(&self, id: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(sim) = self.sim.as_ref() else { return out };
        let w = sim.world();
        let Some(root) = event_by_id(w, id) else { return out };
        for rid in ripple_ids(w, root.id, RIPPLE_CAP) {
            if let Some(e) = siming_sim::ledger::get(w, rid) {
                out.push(&self.event_record(w, e, None));
            }
        }
        out
    }
}

/// 因樹每個節點最多取幾個因（GDD 5.8）。
const CAUSE_FANOUT: usize = 4;
/// 漣漪最多數到幾筆（GDD 5.8）。
const RIPPLE_CAP: usize = 300;

/// 因樹的一個節點：事件編號、層（1 為直接因）、父節點在攤平陣列的位置（直接因為 -1）。
#[derive(Debug, PartialEq, Eq)]
struct CauseNode {
    id: siming_sim::EventId,
    level: u32,
    parent: i32,
}

/// 事件字典的日、成因與關係：`day`（月內第幾日，1..=30）、`why`（成因代碼字串，無則空字串）、
/// `rel`（兩方關係旗標名稱的字串陣列）、`rel_lives`（未了宿緣線已隔的世數）。
fn set_day_and_cause(d: &mut VarDictionary, e: &siming_sim::Event) {
    d.set("day", siming_sim::calendar::date(e.tick).day as i32);
    d.set("why", &GString::from(e.why.code()));
    let names: Vec<GString> = e.rel.names().into_iter().map(GString::from).collect();
    d.set("rel", &PackedStringArray::from(names.as_slice()));
    d.set("rel_lives", e.rel.lives() as i32);
}

fn event_by_id(w: &siming_sim::World, id: i64) -> Option<&siming_sim::Event> {
    u32::try_from(id).ok().and_then(|i| siming_sim::ledger::get(w, i))
}

struct TreeWalk<'a> {
    w: &'a siming_sim::World,
    depth: usize,
    fanout: usize,
    seen: std::collections::HashSet<siming_sim::EventId>,
    out: Vec<CauseNode>,
}

impl TreeWalk<'_> {
    fn walk(&mut self, id: siming_sim::EventId, level: u32, parent: i32) {
        if level as usize > self.depth {
            return;
        }
        let Some(e) = siming_sim::ledger::get(self.w, id) else { return };
        let mut causes: Vec<&siming_sim::Event> = e.causes.iter().filter_map(|&c| siming_sim::ledger::get(self.w, c)).collect();
        causes.sort_by_key(|c| (std::cmp::Reverse(c.importance), std::cmp::Reverse(c.id)));
        for c in causes.into_iter().take(self.fanout) {
            if !self.seen.insert(c.id) {
                continue;
            }
            self.out.push(CauseNode { id: c.id, level, parent });
            let at = self.out.len() as i32 - 1;
            self.walk(c.id, level + 1, at);
        }
    }
}

/// 沿 `causes` 往上最多 `depth` 層的因樹，先根順序；每個節點最多取 `fanout` 個因，已出現過的事件不重複。
fn cause_tree(w: &siming_sim::World, root: siming_sim::EventId, depth: usize, fanout: usize) -> Vec<CauseNode> {
    let mut walk = TreeWalk { w, depth, fanout, seen: std::collections::HashSet::from([root]), out: Vec::new() };
    walk.walk(root, 1, -1);
    walk.out
}

/// 由事件 `root` 沿反向索引往下的後續事件，依事件編號（即時間）升冪，最多 `cap` 筆；不含自己。
fn ripple_ids(w: &siming_sim::World, root: siming_sim::EventId, cap: usize) -> Vec<siming_sim::EventId> {
    let mut seen = std::collections::HashSet::from([root]);
    let mut queue = std::collections::VecDeque::from([root]);
    let mut out = Vec::new();
    while let Some(id) = queue.pop_front() {
        let Some(list) = w.ledger.effects.get(id as usize) else { continue };
        for &x in list {
            if siming_sim::ledger::get(w, x).is_some() && seen.insert(x) {
                out.push(x);
                queue.push_back(x);
            }
        }
    }
    out.sort_unstable();
    out.truncate(cap);
    out
}

/// 事件是否算這個人的歷程：他是事件的人物；或事件只記了他的魂，且發生在他在世（或身故當下）之間。
impl SimWorld {
    fn height_at(&self, pos: (f32, f32)) -> f32 {
        self.land.as_ref().map_or(0.0, |l| l.relief.height_at(pos.0, pos.1))
    }

    /// 因果線寫成 `line_*` 欄位（見 `karma_view`）。
    fn pack_lines(&self, d: &mut VarDictionary, lines: &[karma::Line]) {
        let ints = |f: &dyn Fn(&karma::Line) -> i32| PackedInt32Array::from(lines.iter().map(f).collect::<Vec<_>>().as_slice());
        d.set("line_thread", &ints(&|l| l.thread as i32));
        d.set("line_kind", &ints(&|l| l.kind as i32));
        d.set("line_debtor", &ints(&|l| l.debtor));
        d.set("line_creditor", &ints(&|l| l.creditor));
        d.set("line_dead", &ints(&|l| l.dead as i32));
        d.set("line_woven", &ints(&|l| l.woven as i32));
        d.set("line_strength", &PackedFloat32Array::from(lines.iter().map(|l| l.strength).collect::<Vec<_>>().as_slice()));
        d.set("line_from", &vec2s(&lines.iter().map(|l| [l.from.0, l.from.1]).collect::<Vec<_>>()));
        d.set("line_to", &vec2s(&lines.iter().map(|l| [l.to.0, l.to.1]).collect::<Vec<_>>()));
        d.set("line_from_height", &PackedFloat32Array::from(lines.iter().map(|l| self.height_at(l.from)).collect::<Vec<_>>().as_slice()));
        d.set("line_to_height", &PackedFloat32Array::from(lines.iter().map(|l| self.height_at(l.to)).collect::<Vec<_>>().as_slice()));
    }
}

fn belongs_to(e: &siming_sim::Event, p: &siming_sim::Person) -> bool {
    if e.persons.contains(&p.id) {
        return true;
    }
    e.persons.is_empty() && e.souls.contains(&p.soul) && (e.tick as i64) >= p.born as i64 && p.died.is_none_or(|(t, _)| e.tick <= t)
}

impl SimWorld {
    /// 一則事件的完整記錄：7.2 模板欄位（`event_fields`）、`id`、`tick`、`kind`、`importance`、`province`、`cell`、`pos`、
    /// `souls`、`year`、`month`、`persons`、`powers` 與主角的身分 `a_power`、`a_role`、`a_realm`。
    /// `identity` 為事件當時主角的身分；沒有就取人物現況。
    fn event_record(&self, w: &siming_sim::World, e: &siming_sim::Event, identity: Option<Identity>) -> VarDictionary {
        let gw = w.geo.w as u32;
        let cell_pos = |c: u32| Vector2::new((c % gw) as f32 + 0.5, (c / gw) as f32 + 0.5);
        let mut d = self.event_fields(w, e, None);
        d.set("id", e.id as i64);
        d.set("tick", e.tick as i64);
        d.set("kind", &GString::from(format!("{:?}", e.kind).as_str()));
        d.set("importance", e.importance as i32);
        d.set("province", e.province.map_or(-1, |p| p as i32));
        d.set("cell", e.cell.map_or(-1, |c| c as i32));
        let province = e.province.and_then(|p| w.provinces.get(p as usize));
        if let Some(c) = e.cell {
            d.set("pos", cell_pos(c));
        } else if let Some(p) = province {
            d.set("pos", cell_pos(p.seat));
        }
        let souls: Vec<i32> = e.souls.iter().map(|&s| s as i32).collect();
        d.set("souls", &PackedInt32Array::from(souls.as_slice()));
        let (date_era, year) = siming_sim::calendar::era_year(e.tick, w.evolution.history_years, &w.eras);
        d.set("date_era", &GString::from(date_era.name.as_str()));
        d.set("year", year as i32);
        d.set("month", siming_sim::calendar::date(e.tick).month as i32);
        set_day_and_cause(&mut d, e);
        // 可追對象：牽涉的人物與勢力編號（州郡編號為 province）。
        let person_ids: Vec<i32> = e.persons.iter().map(|&i| i as i32).collect();
        let power_ids: Vec<i32> = e.powers.iter().map(|&i| i as i32).collect();
        d.set("persons", &PackedInt32Array::from(person_ids.as_slice()));
        d.set("powers", &PackedInt32Array::from(power_ids.as_slice()));
        let identity = identity.or_else(|| subjects(w, e).0.map(|p| identity_of(w, p)));
        let identity = identity.unwrap_or(Identity { power: String::new(), role: "", realm: -1 });
        d.set("a_power", &GString::from(identity.power.as_str()));
        d.set("a_role", &GString::from(identity.role));
        d.set("a_realm", identity.realm);
        d
    }

    /// 事件的 7.2 模板欄位（`a`、`b`、`f`、`g`、`p`、`q`、`x`、`n`、`m`，世界大事另有 `change`、`era`），
    /// 再加 `a_person`、`b_person`、`q_province`；`drain_events` 與 `person_history` 共用這一套規則。
    /// `back` 指定遊歷是否為歸來；沒給就看人物此刻的故里是不是目的地（`drain_events` 在事件剛發生時如此判斷）。
    fn event_fields(&self, w: &siming_sim::World, e: &siming_sim::Event, back: Option<bool>) -> VarDictionary {
        let mut d = VarDictionary::new();
        let province = e.province.and_then(|p| w.provinces.get(p as usize));
        let person_id = |i: usize| e.persons.get(i).copied().filter(|&id| w.persons.get(id as usize).is_some());
        let person = |i: usize| person_id(i).and_then(|id| w.persons.get(id as usize));
        let power = |i: usize| e.powers.get(i).and_then(|&id| w.powers.get(id as usize));
        let province_name = |id: siming_sim::ProvinceId| w.provinces.get(id as usize).map_or("", |p| p.name.as_str());
        let (person_a, person_b) = subjects(w, e);
        let a = person_a.map_or("", |p| p.name.as_str());
        let b = person_b.map_or("", |p| p.name.as_str());
        let (first, second_power) = if e.kind == siming_sim::EventKind::SectSplit { (1, 0) } else { (0, 1) };
        let mut f = power(first).map_or("", |p| p.name.as_str());
        let mut g = power(second_power).map_or("", |p| p.name.as_str());
        let mut m = 0i64;
        let mut second = "";
        let mut second_id: Option<siming_sim::ProvinceId> = None;
        let mut x = String::new();
        let mut n = 0i64;
        match &e.args {
            siming_sim::EventArgs::Death { cause, age } => {
                x = format!("{cause:?}");
                n = *age as i64;
            }
            siming_sim::EventArgs::Realm { realm } => n = *realm as i64,
            siming_sim::EventArgs::Tribulation { realm, outcome } => {
                n = *realm as i64;
                x = format!("{outcome:?}");
            }
            siming_sim::EventArgs::Thread { kind, lives, .. } => {
                x = format!("{kind:?}");
                n = *lives as i64;
            }
            siming_sim::EventArgs::Judged { path } => x = format!("{path:?}"),
            siming_sim::EventArgs::Travel { to, .. } => {
                let is_home = back.unwrap_or_else(|| person(0).is_some_and(|p| p.home == *to));
                x = if is_home { "Back" } else { "Out" }.to_string();
                second = province_name(*to);
                second_id = Some(*to);
            }
            siming_sim::EventArgs::Role { role } => {
                x = format!("{role:?}");
                if e.kind == siming_sim::EventKind::ComeOfAge && f.is_empty() {
                    f = person(0).and_then(|p| p.power).and_then(|id| w.powers.get(id as usize)).map_or("", |pw| pw.name.as_str());
                }
            }
            siming_sim::EventArgs::Book { entry } => x = format!("{entry:?}"),
            siming_sim::EventArgs::Sect { node } => {
                if e.kind == siming_sim::EventKind::SectWar {
                    x = node.and_then(|id| w.geo.nodes.get(id as usize)).map_or(String::new(), |nd| nd.name.clone());
                }
            }
            siming_sim::EventArgs::Famine { deaths } => n = *deaths as i64,
            siming_sim::EventArgs::World { .. } | siming_sim::EventArgs::None => {}
            siming_sim::EventArgs::Dynasty { dynasty, other, n: count } => {
                f = w.dynasty.get(*dynasty).map_or("", |d| d.name.as_str());
                g = other.and_then(|id| w.dynasty.get(id)).map_or("", |d| d.name.as_str());
                n = *count as i64;
                x = "Dynasty".to_string();
            }
            siming_sim::EventArgs::Node { node, from, to } => {
                x = w.geo.nodes.get(*node as usize).map_or(String::new(), |nd| nd.name.clone());
                n = *to as i64;
                m = *from as i64;
            }
            siming_sim::EventArgs::Seat { .. } => {
                if let (Some(stem), Some(p)) = (e.province.and_then(|p| self.seat_stems.get(p as usize)), province) {
                    x = seat_name(stem, p.city_level);
                }
            }
            siming_sim::EventArgs::Lost { refuge, perished } => {
                second = refuge.map_or("", province_name);
                second_id = *refuge;
                n = *perished as i64;
            }
            siming_sim::EventArgs::Link { other, joined } => {
                second = province_name(*other);
                second_id = Some(*other);
                x = if *joined { "Joined" } else { "Parted" }.to_string();
            }
        }
        d.set("a", &GString::from(a));
        d.set("b", &GString::from(b));
        d.set("f", &GString::from(f));
        d.set("g", &GString::from(g));
        d.set("p", &GString::from(province.map_or("", |p| p.name.as_str())));
        d.set("q", &GString::from(second));
        d.set("x", &GString::from(x.as_str()));
        d.set("n", n);
        d.set("m", m);
        d.set("a_person", person_id(0).map_or(-1, |id| id as i32));
        d.set("b_person", person_id(1).map_or(-1, |id| id as i32));
        d.set("q_province", second_id.map_or(-1, |id| id as i32));
        if let siming_sim::EventArgs::World { change, new_era } = &e.args {
            let year = w.evolution.history_years + siming_sim::evolution::year_of(e.tick);
            let era = w.eras.iter().find(|era| era.start_year == year + 1);
            let name = if *new_era { era.map_or("", |era| era.name.as_str()) } else { "" };
            d.set("change", &GString::from(format!("{change:?}").as_str()));
            d.set("era", &GString::from(name));
        }
        d
    }
}

/// 人物的身分：所屬勢力名、職分代號、修士境界。
#[derive(Clone)]
struct Identity {
    power: String,
    role: &'static str,
    realm: i32,
}

/// 事件的兩位主角（`{A}`、`{B}`）。判入六道只記了魂，人名取魂最近一世的肉身；
/// 兩魂牽涉的事件（宿緣）若有一魂當時沒有肉身，人物欄少於魂欄：改按魂的順序各取最近一世，免得把另一人當成主角。
fn subjects<'a>(w: &'a siming_sim::World, e: &siming_sim::Event) -> (Option<&'a siming_sim::Person>, Option<&'a siming_sim::Person>) {
    let person = |i: usize| e.persons.get(i).and_then(|&id| w.persons.get(id as usize));
    let soul_body = |i: usize| {
        e.souls.get(i).and_then(|&s| w.souls.get(s as usize)).and_then(|s| s.lives.last()).and_then(|&pid| w.persons.get(pid as usize))
    };
    if e.souls.len() >= 2 && e.persons.len() < e.souls.len() {
        (soul_body(0), soul_body(1))
    } else {
        (person(0).or(soul_body(0)), person(1))
    }
}

/// 一個月推進完之後，記下這個月新事件主角的身分；快轉一次推進多月時，取走事件時人物的現況可能已經不同。
/// 掌門身死時繼任緊接在同一個月寫入，月底掌門已換人：以「前任 Died」為因的繼任事件把死者補回掌門。
fn note_identities(w: &siming_sim::World, out: &mut std::collections::HashMap<siming_sim::EventId, Identity>) {
    let events = &w.ledger.events;
    let mut start = events.len();
    while start > 0 && events[start - 1].tick == w.tick {
        start -= 1;
    }
    for e in &events[start..] {
        if let (Some(p), _) = subjects(w, e) {
            out.insert(e.id, identity_of(w, p));
        }
    }
    for e in &events[start..] {
        if e.kind != siming_sim::EventKind::Succession || matches!(e.args, siming_sim::EventArgs::Dynasty { .. }) {
            continue;
        }
        let name = e.powers.first().and_then(|&id| w.powers.get(id as usize)).map_or("", |pw| pw.name.as_str());
        for cause in &e.causes {
            let died = matches!(events.get(*cause as usize).map(|c| &c.args), Some(siming_sim::EventArgs::Death { .. }));
            if let Some(id) = out.get_mut(cause).filter(|id| died && id.role != "Leader" && id.role != "Ruler") {
                id.role = "Leader";
                id.power = name.to_string();
            }
        }
    }
}

/// 人物目前的身分。
/// 職分代號：`Ruler` 君主、`Leader` 掌門、`Member` 宗門成員（長老或弟子由介面端依境界分）、`Rogue` 散修、
/// `Monk`、`Scholar`、`Demonic`、`Beast`、`Official`、`General`、`Royal`；其餘為空字串。境界非修士為 −1。
fn identity_of(w: &siming_sim::World, p: &siming_sim::Person) -> Identity {
    use siming_sim::{PathState, Role};
    let power = p.power.and_then(|id| w.powers.get(id as usize));
    let realm = match p.path {
        PathState::Cultivator { realm, .. } => realm as i32,
        _ => -1,
    };
    let role = match p.role {
        Role::Ruler => "Ruler",
        Role::Cultivator => match power {
            Some(pw) if pw.leader == Some(p.id) => "Leader",
            Some(_) => "Member",
            None => "Rogue",
        },
        Role::Monk => "Monk",
        Role::Scholar => "Scholar",
        Role::Demonic => "Demonic",
        Role::Beast => "Beast",
        Role::Official => "Official",
        Role::General | Role::GeneralCandidate => "General",
        Role::Royal => "Royal",
        Role::Child | Role::Commoner => "",
    };
    let power_name = if role == "Ruler" { "" } else { power.map_or("", |pw| pw.name.as_str()) };
    Identity { power: power_name.to_string(), role, realm }
}

#[cfg(test)]
mod tests {
    use super::*;
    use siming_sim::{Sim, WorldParams, WorldSize};

    fn aged_world() -> Sim {
        let mut p = WorldParams::new(20261011);
        p.size = WorldSize::Small;
        let mut sim = Sim::new(&p).unwrap();
        for _ in 0..12 * 30 {
            sim.step();
        }
        sim
    }

    /// 挑一則因樹至少兩層的事件。
    fn deep_event(w: &siming_sim::World) -> siming_sim::EventId {
        w.ledger
            .events
            .iter()
            .filter(|e| e.importance >= 2)
            .map(|e| e.id)
            .find(|&id| cause_tree(w, id, 6, CAUSE_FANOUT).iter().any(|n| n.level >= 2))
            .expect("三十年內應有因樹兩層以上的事件")
    }

    #[test]
    fn gdd_5_8_cause_tree_is_bounded_unique_and_rooted_in_direct_causes() {
        let sim = aged_world();
        let w = sim.world();
        let mut deepest = 0;
        for e in w.ledger.events.iter().filter(|e| e.importance > 0) {
            let tree = cause_tree(w, e.id, 6, CAUSE_FANOUT);
            let mut seen = std::collections::HashSet::new();
            for (i, n) in tree.iter().enumerate() {
                assert!(seen.insert(n.id), "同一事件在樹中只出現一次");
                assert!((1..=6).contains(&n.level));
                assert!(n.id < e.id, "因必在果之前");
                if n.level == 1 {
                    assert_eq!(n.parent, -1);
                    assert!(e.causes.contains(&n.id), "第一層是直接因");
                } else {
                    assert!((n.parent as usize) < i, "先根順序：父節點在前");
                    assert_eq!(tree[n.parent as usize].level + 1, n.level);
                }
            }
            let first_level = tree.iter().filter(|n| n.level == 1).count();
            assert!(first_level <= CAUSE_FANOUT);
            deepest = deepest.max(tree.iter().map(|n| n.level).max().unwrap_or(0));
        }
        assert!(deepest >= 2, "三十年內應有兩層以上的因樹");
        let root = deep_event(w);
        assert!(cause_tree(w, root, 1, CAUSE_FANOUT).iter().all(|n| n.level == 1), "深度 1 只到直接因");
        assert!(cause_tree(w, root, 0, CAUSE_FANOUT).is_empty());
    }

    #[test]
    fn gdd_5_8_cause_tree_prefers_important_causes() {
        let sim = aged_world();
        let w = sim.world();
        for e in w.ledger.events.iter().filter(|e| e.importance > 0 && e.causes.len() > 1) {
            let top: Vec<u8> = cause_tree(w, e.id, 1, 1).iter().map(|n| w.ledger.events[n.id as usize].importance).collect();
            let best = e.causes.iter().filter_map(|&c| siming_sim::ledger::get(w, c)).map(|c| c.importance).max();
            assert_eq!(top.first().copied(), best, "只取一個因時取重要度最高的");
        }
    }

    #[test]
    fn gdd_5_8_ripple_follows_effects_downward_and_is_capped() {
        let sim = aged_world();
        let w = sim.world();
        let mut any = false;
        for e in w.ledger.events.iter().filter(|e| e.importance > 0) {
            let ids = ripple_ids(w, e.id, RIPPLE_CAP);
            assert!(ids.len() <= RIPPLE_CAP);
            assert!(ids.iter().all(|&x| x > e.id), "後果必在事件之後");
            assert!(ids.windows(2).all(|p| p[0] < p[1]));
            // 直接後果一定在漣漪裡。
            for &x in w.ledger.effects[e.id as usize].iter().filter(|&&x| siming_sim::ledger::get(w, x).is_some()) {
                assert!(ids.contains(&x) || ids.len() == RIPPLE_CAP);
            }
            any |= !ids.is_empty();
        }
        assert!(any);
        let root = w.ledger.events.iter().find(|e| e.importance > 0 && ripple_ids(w, e.id, RIPPLE_CAP).len() >= 3).map(|e| e.id).unwrap();
        assert_eq!(ripple_ids(w, root, 2).len(), 2, "cap 截斷");
    }

    #[test]
    fn gdd_5_8_pruned_or_missing_event_is_not_found() {
        let sim = aged_world();
        let w = sim.world();
        assert!(event_by_id(w, -1).is_none());
        assert!(event_by_id(w, i64::from(u32::MAX) + 1).is_none());
        assert!(event_by_id(w, w.ledger.events.len() as i64).is_none());
        if let Some(tomb) = w.ledger.events.iter().find(|e| e.importance == 0) {
            assert!(event_by_id(w, tomb.id as i64).is_none(), "墓碑視為不存在");
            assert!(cause_tree(w, tomb.id, 6, CAUSE_FANOUT).is_empty());
            assert!(ripple_ids(w, tomb.id, RIPPLE_CAP).is_empty() || event_by_id(w, tomb.id as i64).is_none());
        }
        let root = deep_event(w);
        assert!(event_by_id(w, root as i64).is_some());
    }
}
