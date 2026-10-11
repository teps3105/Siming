//! GDD 6.4 Godot 橋接：`SimWorld` 持有一個 [`Sim`]，Godot 端只經由本類別讀取世界、推進時間。
//! 本 crate 不含遊戲規則；規則一律在 siming-sim（GDD 6.1 依賴原則）。

mod mapbake;
mod relief;

use godot::prelude::*;
use relief::Relief;
use siming_sim::{calendar, Command, Sim, WorldParams, WorldSize, NO_PROVINCE};

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
    /// 細化後的高度場（貼圖、河流、選取共用）。
    relief: Option<Relief>,
    /// 各州郡治所的兩字名（世界生成後算一次）。
    seat_stems: Vec<String>,
    /// 有名字的海區與湖泊（世界生成後算一次）。
    waters: Vec<siming_sim::worldgen::NamedWater>,
    base: Base<RefCounted>,
}

#[godot_api]
impl SimWorld {
    /// GDD 4.6 開天：世界生成 → 開天 → 預跑。`size` 為 `"small"`、`"medium"`、`"large"`。
    /// 失敗時回傳原因；成功回傳空字串。
    #[func]
    fn create(&mut self, seed: i64, size: GString) -> GString {
        let mut params = WorldParams::new(seed as u64);
        params.size = match size.to_string().as_str() {
            "small" => WorldSize::Small,
            "large" => WorldSize::Large,
            _ => WorldSize::Medium,
        };
        match Sim::new(&params) {
            Ok(sim) => {
                self.relief = Some(relief::build_relief(&sim.world().geo, &sim.world().fine));
                self.waters =
                    siming_sim::worldgen::name_waters(sim.world().params.seed, &sim.world().geo);
                self.seat_stems = siming_sim::worldgen::seat_stems(
                    sim.world().params.seed,
                    &sim.world().provinces,
                    &sim.world().geo,
                );
                self.sim = Some(sim);
                GString::new()
            }
            Err(e) => GString::from(e.to_string().as_str()),
        }
    }

    #[func]
    fn is_ready(&self) -> bool {
        self.sim.is_some()
    }

    /// GDD 6.3 推進 `months` 個月。
    #[func]
    fn step(&mut self, months: i32) {
        if let Some(sim) = self.sim.as_mut() {
            for _ in 0..months.max(0) {
                sim.step();
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
            Some(s) => GString::from(calendar::full(s.world().tick, &s.world().eras).as_str()),
            None => GString::new(),
        }
    }

    /// GDD 2.6 曆法簡稱（題簽用）。
    #[func]
    fn date_short(&self) -> GString {
        match self.sim.as_ref() {
            Some(s) => GString::from(calendar::era_date(s.world().tick, &s.world().eras).as_str()),
            None => GString::new(),
        }
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
        self.relief.as_ref().map_or(0.0, |r| r.max_height())
    }

    /// 世界座標 (x, z) 的地形高度（高度縮放 = 1）。
    #[func]
    fn terrain_height_at(&self, x: f32, z: f32) -> f32 {
        self.relief.as_ref().map_or(0.0, |r| r.height_at(x, z))
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
        let (Some(sim), Some(relief)) = (self.sim.as_ref(), self.relief.as_ref()) else {
            return d;
        };
        let r = mapbake::build_river_mesh(&sim.world().geo, relief);
        d.set("positions", &vec3s(&r.positions));
        d.set("tangents", &vec3s(&r.tangents));
        d.set("side_level", &vec2s(&r.side_level));
        d.set("taper", &vec2s(&r.taper));
        d.set("indices", &PackedInt32Array::from(r.indices.as_slice()));
        d
    }

    /// 世界貼圖（GDD 5.3「貼圖」）：
    /// `elevation`（相對海平面的高度，`PackedFloat32Array`）與 `elevation_size`、`lake`（R8，同尺寸）、
    /// `climate`（RGB8）與 `climate_size`、`sea_level`、`aura`（R8，格 × `aura_scale`）、`province_id`（RG8，格）。
    #[func]
    fn map_textures(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(sim), Some(relief)) = (self.sim.as_ref(), self.relief.as_ref()) else {
            return d;
        };
        let t = mapbake::build_world_textures(&sim.world().geo);
        d.set(
            "elevation",
            &PackedFloat32Array::from(relief.elevation.as_slice()),
        );
        d.set(
            "elevation_size",
            Vector2i::new(relief.w as i32, relief.h as i32),
        );
        d.set("lake", &bytes(&relief.lake));
        d.set("climate", &bytes(&relief.climate));
        d.set(
            "climate_size",
            Vector2i::new(relief.climate_w as i32, relief.climate_h as i32),
        );
        d.set("sea_level", relief.sea_level);
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
        for c in &w.geo.continents {
            push(
                "continent",
                &c.name,
                centre(c.centroid),
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
