//! GDD 6.4 Godot 橋接：`SimWorld` 持有一個 [`Sim`]，Godot 端只經由本類別讀取世界、推進時間。
//! 本 crate 不含遊戲規則；規則一律在 siming-sim（GDD 6.1 依賴原則）。

mod mapbake;

use godot::prelude::*;
use mapbake::TerrainCpu;
use siming_sim::{calendar, Command, Sim, WorldParams, WorldSize, NO_PROVINCE};

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
    /// 地形高度場的副本（網格、河流、選取共用）。
    cpu: Option<TerrainCpu>,
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
                self.cpu = Some(TerrainCpu::new(&sim.world().geo));
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
            Some(s) => GString::from(calendar::full(s.world().tick).as_str()),
            None => GString::new(),
        }
    }

    /// GDD 2.6 曆法簡稱（題簽用）。
    #[func]
    fn date_short(&self) -> GString {
        match self.sim.as_ref() {
            Some(s) => GString::from(calendar::short(s.world().tick).as_str()),
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
        self.cpu.as_ref().map_or(0.0, |c| c.max_height)
    }

    /// 世界座標 (x, z) 的地形高度（高度縮放 = 1）。
    #[func]
    fn terrain_height_at(&self, x: f32, z: f32) -> f32 {
        self.cpu.as_ref().map_or(0.0, |c| c.height_at(x, z))
    }

    /// 地形網格陣列：`positions`、`normals`、`uvs`、`curvature`、`indices`。
    #[func]
    fn terrain_mesh(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(cpu) = self.cpu.as_ref() else {
            return d;
        };
        let m = mapbake::build_terrain_mesh(cpu);
        d.set("positions", &vec3s(&m.positions));
        d.set("normals", &vec3s(&m.normals));
        d.set("uvs", &vec2s(&m.uvs));
        d.set("curvature", &vec2s(&m.curvature));
        d.set("indices", &PackedInt32Array::from(m.indices.as_slice()));
        d
    }

    /// 河流帶狀網格陣列：`positions`、`tangents`、`side_level`、`taper`、`indices`（GDD 5.3「河流」）。
    #[func]
    fn river_mesh(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(sim), Some(cpu)) = (self.sim.as_ref(), self.cpu.as_ref()) else {
            return d;
        };
        let r = mapbake::build_river_mesh(&sim.world().geo, cpu);
        d.set("positions", &vec3s(&r.positions));
        d.set("tangents", &vec3s(&r.tangents));
        d.set("side_level", &vec2s(&r.side_level));
        d.set("taper", &vec2s(&r.taper));
        d.set("indices", &PackedInt32Array::from(r.indices.as_slice()));
        d
    }

    /// 世界貼圖的位元組：`terrain`（RGBA8，格 × `scale`）、`aura`（R8，格 × `scale`）、
    /// `cls_a`、`cls_b`（RGBA8，格）、`province_id`（RG8，格）；另有 `scale`。
    #[func]
    fn map_textures(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(sim) = self.sim.as_ref() else {
            return d;
        };
        let t = mapbake::build_world_textures(&sim.world().geo);
        d.set("scale", mapbake::SS as i32);
        d.set("terrain", &bytes(&t.terrain));
        d.set("aura", &bytes(&t.aura));
        d.set("cls_a", &bytes(&t.cls_a));
        d.set("cls_b", &bytes(&t.cls_b));
        d.set("province_id", &bytes(&t.province_id));
        d
    }

    /// 紙紋：512×512 RGBA8（與世界種子無關，GDD 8.3）。
    #[func]
    fn paper_texture() -> PackedByteArray {
        bytes(&mapbake::gen_paper())
    }

    /// 皴法筆觸：256×256 R8（GDD 8.3）。
    #[func]
    fn stroke_texture() -> PackedByteArray {
        bytes(&mapbake::gen_stroke())
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
    /// `cells`（格數）。
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
        d
    }

    /// 地圖標籤（GDD 5.3「文字標籤」）：每筆有 `kind`（`province`、`river`、`range`、`node`、`continent`）、
    /// `text`、`pos`（世界座標 x、z）與 `rank`（越大越重要）。
    #[func]
    fn map_labels(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else {
            return out;
        };
        let w = s.world();
        let gw = w.geo.w as u32;
        let centre = |cell: u32| Vector2::new((cell % gw) as f32 + 0.5, (cell / gw) as f32 + 0.5);
        let mut push = |kind: &str, text: &str, pos: Vector2, rank: f32| {
            let mut d = VarDictionary::new();
            d.set("kind", &GString::from(kind));
            d.set("text", &GString::from(text));
            d.set("pos", pos);
            d.set("rank", rank);
            out.push(&d);
        };
        for c in &w.geo.continents {
            push("continent", &c.name, centre(c.centroid), 100.0 + c.area);
        }
        for p in &w.provinces {
            push("province", &p.name, centre(p.seat), p.city_level as f32);
        }
        for r in &w.geo.rivers {
            if let Some(&mid) = r.cells.get(r.cells.len() / 2) {
                push(
                    "river",
                    &r.name,
                    centre(mid),
                    r.level as f32 * 10.0 + r.cells.len() as f32 * 0.01,
                );
            }
        }
        for r in &w.geo.ranges {
            push(
                "range",
                &r.name,
                centre(r.peak),
                10.0 + r.cells.len() as f32 * 0.01,
            );
        }
        for n in &w.geo.nodes {
            push("node", &n.name, centre(n.cell), n.strength * 20.0);
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
