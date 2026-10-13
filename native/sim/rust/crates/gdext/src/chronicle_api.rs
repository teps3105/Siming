//! GDD 5.12 人物生平的橋接查詢：把 `siming_sim::chronicle` 的結果轉成字典給畫面。
//! 全部唯讀；放在 `SimWorld` 的第二個 `impl` 區塊，不動既有的類別定義。

use godot::prelude::*;
use siming_sim::chronicle::{self, Because, Entry, PastLife, Who};
use siming_sim::World;

use crate::SimWorld;

fn gs(text: &str) -> GString {
    GString::from(text)
}

/// 年月三件組：紀元名、紀元內年份、月。
fn set_date(w: &World, d: &mut VarDictionary, prefix: &str, tick: u32) {
    let (era, year) = siming_sim::calendar::era_year(tick, w.evolution.history_years, &w.eras);
    d.set(format!("{prefix}era").as_str(), &gs(&era.name));
    d.set(format!("{prefix}year").as_str(), year as i32);
    d.set(format!("{prefix}month").as_str(), siming_sim::calendar::date(tick).month as i32);
}

fn set_who(d: &mut VarDictionary, prefix: &str, who: Option<&Who>) {
    d.set(prefix, who.and_then(|x| x.person).map_or(-1, |p| p as i32));
    d.set(format!("{prefix}_name").as_str(), &gs(who.map_or("", |x| x.name.as_str())));
}

fn province_name(w: &World, id: Option<siming_sim::ProvinceId>) -> &str {
    id.and_then(|p| w.provinces.get(p as usize)).map_or("", |p| p.name.as_str())
}

fn power_name(w: &World, id: Option<u32>) -> &str {
    id.and_then(|p| w.powers.get(p as usize)).map_or("", |p| p.name.as_str())
}

fn because_dict(w: &World, b: &Because) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("event", b.event as i64);
    d.set("tick", b.tick as i64);
    set_date(w, &mut d, "", b.tick);
    set_who(&mut d, "me", Some(&b.me));
    set_who(&mut d, "other", Some(&b.other));
    d.set("thread", &gs(b.thread));
    d.set("lives", b.lives as i32);
    d
}

fn entry_dict(w: &World, e: &Entry) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("id", e.id as i64);
    d.set("tick", e.tick as i64);
    set_date(w, &mut d, "", e.tick);
    d.set("kind", &gs(e.kind));
    d.set("role", &gs(e.role));
    set_who(&mut d, "other", e.other.as_ref());
    set_who(&mut d, "other_bond", e.other_bond.as_ref());
    d.set("province", e.province.map_or(-1, |p| p as i32));
    d.set("province_name", &gs(province_name(w, e.province)));
    d.set("power", e.power.map_or(-1, |p| p as i32));
    d.set("power_name", &gs(power_name(w, e.power)));
    d.set("power_from", e.power_from.map_or(-1, |p| p as i32));
    d.set("power_from_name", &gs(power_name(w, e.power_from)));
    d.set("why", &gs(e.why));
    d.set("detail", &gs(&e.detail));
    d.set("n", e.n);
    d.set("thread", &gs(e.thread));
    d.set("target", e.target.map_or(-1, |p| p as i32));
    d.set("target_name", &gs(province_name(w, e.target)));
    d.set("end_reason", &gs(e.end_reason));
    d.set("group", e.group as i64);
    if let Some(b) = &e.because {
        d.set("because", &because_dict(w, b));
    }
    d
}

fn past_life_dict(w: &World, p: &PastLife) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("id", if p.id == u32::MAX { -1 } else { p.id as i64 });
    d.set("tick", p.died as i64);
    set_date(w, &mut d, "", p.died);
    d.set("kind", &gs("past_life"));
    d.set("role", &gs("self"));
    d.set("person", p.person as i32);
    d.set("name", &gs(&p.name));
    d.set("title", &gs(p.title.as_deref().unwrap_or("")));
    d.set("identity", &gs(&p.role));
    d.set("realm", p.realm.map_or(-1, |r| r as i32));
    d.set("fruit", p.fruit.map_or(-1, |r| r as i32));
    d.set("power", p.power.map_or(-1, |x| x as i32));
    d.set("power_name", &gs(power_name(w, p.power)));
    d.set("cause", &gs(&p.cause));
    d.set("age", p.age as i32);
    d.set("province", p.province as i32);
    d.set("province_name", &gs(province_name(w, Some(p.province))));
    set_who(&mut d, "killer", p.killer.as_ref());
    d.set("path", &gs(&p.path));
    d.set("reborn_tick", p.reborn as i64);
    set_date(w, &mut d, "reborn_", p.reborn);
    d.set("reborn_province", p.reborn_province as i32);
    d.set("reborn_province_name", &gs(province_name(w, Some(p.reborn_province))));
    d
}

#[godot_api(secondary)]
impl SimWorld {
    /// 人物的生平事實（GDD 5.12），由舊到新；文字由 Godot 端依句型組成。人物不存在回傳空陣列。
    ///
    /// 這個人若是某個魂的轉世，第一筆是上一世摘要（`kind` 為 `past_life`）：`person`、`name`、`title`、`identity`
    /// （`Role` 代號）、`realm`、`fruit`、`power`／`power_name`、`cause`（`DeathCause` 代號）、`age`、`province`／
    /// `province_name`（歿處）、`killer`／`killer_name`（殺他的人，當時的名字）、`path`（魂判入的六道，查不到為空）、
    /// `reborn_tick`、`reborn_era`／`reborn_year`／`reborn_month`、`reborn_province`／`reborn_province_name`。
    /// 其餘每筆都是本世的事：
    ///
    /// - `id`（事件編號，整份清單不重複）、`tick`、`era`／`year`／`month`（當時的紀元名、紀元內年份、月）；
    /// - `kind`（種類代碼）與 `role`（本人在這件事裡的角色，沒有方向為 `self`），代碼表見 5.12；
    /// - `other`／`other_name`（對方的人物編號與事件當時的名字，無為 -1、空字串）、
    ///   `other_bond`／`other_bond_name`（對方在結緣當時用過不同名字時的稱呼，否則 -1、空字串）；
    /// - `province`／`province_name`、`power`／`power_name`（宗門）、`power_from`／`power_from_name`（分宗時的原宗門）、
    ///   `target`／`target_name`（遊歷的目的地）；
    /// - `why`（成因代碼）、`detail`（依種類：身分、死因、六道、天劫結果、遊歷歸來 `back`／`out`）、`n`（境界、享年、世數）、
    ///   `thread`（宿緣種類代碼）；
    /// - `end_reason`（事情結束的原因：`spouse_died`、`master_died`、`master_left`、`disciple_left`；非結束為空）；
    /// - `group`（同一件事的組別：同組各筆應合成一句；獨立的事等於自己的 `id`）；
    /// - `because`（宿緣後果才有：`event`、`tick`、`era`／`year`／`month`、`me`／`me_name`、`other`／`other_name`
    ///   為結緣當時雙方的稱呼，`thread`、`lives` 為這筆後果發生時已隔幾世）。
    #[func]
    fn person_chronicle(&self, id: i32) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(s) = self.sim.as_ref() else { return out };
        let w = s.world();
        let Some(c) = u32::try_from(id).ok().and_then(|i| chronicle::chronicle(w, i)) else { return out };
        if let Some(p) = &c.past_life {
            out.push(&past_life_dict(w, p));
        }
        for e in &c.entries {
            out.push(&entry_dict(w, e));
        }
        out
    }
}
