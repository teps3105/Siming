//! GDD 6.4 Godot 橋接：`SimWorld` 持有一個 [`Sim`]，Godot 端只經由本類別讀取世界、推進時間。
//! 本 crate 不含遊戲規則；規則一律在 siming-sim（GDD 6.1 依賴原則）。

mod mapbake;
mod relief;

use std::sync::mpsc::{self, Receiver, TryRecvError};

use godot::prelude::*;
use relief::Relief;
use siming_sim::save::{Replay, SaveFile};
use siming_sim::{calendar, Command, Era, FineGeography, Geography, Sim, WorldParams, WorldSize, NO_PROVINCE};

/// 山脈標籤的路徑：沿主軸每幾格取一段（GDD 5.3「文字標籤」）。
const RANGE_LABEL_SEGMENT: f32 = 4.0;

/// 治所的全名（GDD 7.3）：兩字名加上城池等級的字（0 村 .. 4 都）。
fn seat_name(stem: &str, city_level: u8) -> String {
    const SUFFIX: [&str; 5] = ["村", "鎮", "縣", "城", "都"];
    format!(
        "{stem}{}",
        SUFFIX[(city_level as usize).min(SUFFIX.len() - 1)]
    )
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
    /// 有名字的海區與湖泊（世界生成後算一次）。
    waters: Vec<siming_sim::worldgen::NamedWater>,
    /// 進行中的讀檔重播（`load_begin` 之後、`load_step` 回報完成之前）。
    replay: Option<Replay>,
    /// `land` 是依哪一份水陸算出來的（`land_stamp`）。
    baked: u64,
    /// 背景執行緒上重算中的一批（GDD 2.8「畫面」）。
    rebake: Option<Rebake>,
    /// 高度場換過幾次；地圖顯示的資料（含州界、靈氣）換過幾次。
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
fn water_names(
    seed: u64,
    geo: &siming_sim::Geography,
    origin: Option<&[siming_sim::Terrain]>,
) -> Vec<siming_sim::worldgen::NamedWater> {
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
        self.seat_stems =
            siming_sim::worldgen::seat_stems(sim.world().params.seed, &sim.world().provinces, &sim.world().geo);
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
        match Sim::new(&params) {
            Ok(sim) => {
                self.adopt(sim);
                GString::new()
            }
            Err(e) => GString::from(e.to_string().as_str()),
        }
    }
}

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
        self.sim.as_ref().map_or(Vector2i::ZERO, |s| {
            Vector2i::new(s.world().geo.w as i32, s.world().geo.h as i32)
        })
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
        self.sim.as_ref().map_or_else(PackedByteArray::new, |s| {
            bytes(&mapbake::province_color_table(&s.world().provinces))
        })
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
        let gx = if geo.wrap_x {
            (x.floor() as i32).rem_euclid(w)
        } else {
            x.floor() as i32
        };
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
        self.sim
            .as_ref()
            .map_or(0, |s| s.world().provinces.len() as i32)
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
        d.set(
            "seat",
            Vector2::new((p.seat % gw) as f32 + 0.5, (p.seat / gw) as f32 + 0.5),
        );
        d.set("cells", p.cells.len() as i32);
        d.set(
            "seat_name",
            &GString::from(
                seat_name(
                    self.seat_stems
                        .get(p.id as usize)
                        .map_or("", |s| s.as_str()),
                    p.city_level,
                )
                .as_str(),
            ),
        );
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
        let mut push =
            |kind: &str, text: &str, pos: Vector2, rank: f32, path: PackedVector2Array| {
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
            push(
                "continent",
                &c.name,
                centre(continent_label_cell(&w.geo, i)),
                100.0 + c.area,
                none(),
            );
        }
        for p in &w.provinces {
            push(
                "province",
                &p.name,
                centre(mapbake::province_label_cell(
                    &p.cells,
                    w.geo.w as usize,
                    w.geo.h as usize,
                )),
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
                push(
                    "river",
                    &r.name,
                    centre(mid),
                    r.level as f32 * 10.0 + r.cells.len() as f32 * 0.01,
                    path,
                );
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
            let (mx, my) = rel
                .iter()
                .fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
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
                sums.iter()
                    .filter(|c| c.2 > 0)
                    .map(|c| Vector2::new(peak.x + c.0 / c.2 as f32, peak.y + c.1 / c.2 as f32))
                    .collect()
            } else {
                none()
            };
            push(
                "range",
                &r.name,
                peak,
                10.0 + r.cells.len() as f32 * 0.01,
                path,
            );
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
            let path: PackedVector2Array = e
                .path
                .iter()
                .map(|&c| Vector2::new((c % gw) as f32 + 0.5, (c / gw) as f32 + 0.5))
                .collect();
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
            d.set(
                "pos",
                Vector2::new((n.cell % gw) as f32 + 0.5, (n.cell / gw) as f32 + 0.5),
            );
            out.push(&d);
        }
        out
    }

    // ── 統計 ────────────────────────────────────────────────────

    #[func]
    fn total_population(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| {
            s.world().provinces.iter().map(|p| p.population).sum()
        })
    }

    /// 在世有名之人數（GDD 3.4）。
    #[func]
    fn alive_person_count(&self) -> i32 {
        self.sim
            .as_ref()
            .map_or(0, |s| s.world().alive_ids.len() as i32)
    }

    // ── 指令與事件（GDD 4.7、6.3）───────────────────────────────

    /// 關注或取消關注一個魂；在下一個 tick 開始前生效。
    #[func]
    fn follow(&mut self, soul: i64, on: bool) {
        if let Some(sim) = self.sim.as_mut() {
            sim.apply(Command::Follow {
                soul: soul as _,
                on,
            });
        }
    }

    /// 取走上次呼叫以來的新事件：每筆有 `id`、`tick`、`kind`（事件代號）、`importance`、`province`（無則 -1）。
    #[func]
    fn drain_events(&mut self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(sim) = self.sim.as_mut() else {
            return out;
        };
        let ids = sim.drain_events();
        for id in ids {
            let Some(e) = siming_sim::ledger::get(sim.world(), id) else {
                continue;
            };
            let mut d = VarDictionary::new();
            d.set("id", e.id as i64);
            d.set("tick", e.tick as i64);
            d.set("kind", &GString::from(format!("{:?}", e.kind).as_str()));
            d.set("importance", e.importance as i32);
            d.set("province", e.province.map_or(-1, |p| p as i32));
            out.push(&d);
        }
        out
    }
}
