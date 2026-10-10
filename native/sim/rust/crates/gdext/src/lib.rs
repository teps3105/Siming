//! GDD 6.4 Godot 橋接：`SimWorld` 持有一個 [`Sim`]，Godot 端只經由本類別讀取世界、推進時間。
//! 本 crate 不含遊戲規則；規則一律在 siming-sim（GDD 6.1 依賴原則）。

use godot::prelude::*;
use siming_sim::{calendar, Sim, WorldParams, WorldSize};

struct SimingExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SimingExtension {}

/// 一個模擬世界。未呼叫 `create` 前所有查詢回傳空值。
#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct SimWorld {
    sim: Option<Sim>,
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

    /// GDD 6.5 世界雜湊，十六進位 16 字元。
    #[func]
    fn hash_hex(&self) -> GString {
        match self.sim.as_ref() {
            Some(s) => GString::from(format!("{:016x}", s.hash()).as_str()),
            None => GString::new(),
        }
    }

    /// 細格尺寸（GDD 6.4 地理細格）；沒有細格資料時為遊戲格尺寸。
    #[func]
    fn map_size(&self) -> Vector2i {
        let Some(s) = self.sim.as_ref() else { return Vector2i::ZERO };
        let w = s.world();
        if w.fine.is_empty() {
            Vector2i::new(w.geo.w as i32, w.geo.h as i32)
        } else {
            Vector2i::new(w.fine.w as i32, w.fine.h as i32)
        }
    }

    #[func]
    fn sea_level(&self) -> f32 {
        let Some(s) = self.sim.as_ref() else { return 0.0 };
        let w = s.world();
        if w.fine.is_empty() {
            w.geo.sea_level
        } else {
            w.fine.sea_level
        }
    }

    /// 高度 0..1，列優先，長度為 `map_size` 的寬 × 高。
    #[func]
    fn height_map(&self) -> PackedFloat32Array {
        let Some(s) = self.sim.as_ref() else { return PackedFloat32Array::new() };
        let w = s.world();
        let v: Vec<f32> = if w.fine.is_empty() {
            w.geo.height.clone()
        } else {
            w.fine.height.iter().map(|&h| h as f32 / 65535.0).collect()
        };
        PackedFloat32Array::from(v.as_slice())
    }

    /// 地形代碼（GDD 2.2；0 海 .. 8 澤），與 `height_map` 同尺寸。
    #[func]
    fn terrain_map(&self) -> PackedByteArray {
        let Some(s) = self.sim.as_ref() else { return PackedByteArray::new() };
        let w = s.world();
        let src = if w.fine.is_empty() { &w.geo.terrain } else { &w.fine.terrain };
        let v: Vec<u8> = src.iter().map(|&t| t as u8).collect();
        PackedByteArray::from(v.as_slice())
    }

    /// 河流等級（0 無 .. 3 江），與 `height_map` 同尺寸。
    #[func]
    fn river_map(&self) -> PackedByteArray {
        let Some(s) = self.sim.as_ref() else { return PackedByteArray::new() };
        let w = s.world();
        let src = if w.fine.is_empty() { &w.geo.river } else { &w.fine.river };
        PackedByteArray::from(src.as_slice())
    }

    #[func]
    fn province_count(&self) -> i32 {
        self.sim.as_ref().map_or(0, |s| s.world().provinces.len() as i32)
    }

    /// 州郡摘要（GDD 2.3）：`name`、`population`、`aura`、`city_level`。
    #[func]
    fn province_info(&self, id: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let Some(p) = s.world().provinces.get(id.max(0) as usize) else { return d };
        d.set("name", &GString::from(p.name.as_str()));
        d.set("population", p.population);
        d.set("aura", p.aura);
        d.set("city_level", p.city_level as i32);
        d
    }

    #[func]
    fn total_population(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.world().provinces.iter().map(|p| p.population).sum())
    }

    /// 在世有名之人數（GDD 3.4）。
    #[func]
    fn alive_person_count(&self) -> i32 {
        self.sim.as_ref().map_or(0, |s| s.world().alive_ids.len() as i32)
    }
}
