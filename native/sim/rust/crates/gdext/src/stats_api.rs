//! GDD 5.10 天下冊的橋接查詢：把 `siming_sim::stats` 的結果轉成字典給畫面。
//! 全部唯讀；放在 `SimWorld` 的第二個 `impl` 區塊，不動既有的類別定義。

use godot::prelude::*;
use siming_sim::stats;

use crate::SimWorld;

fn gs(text: &str) -> GString {
    GString::from(text)
}

fn person_ref(p: &Option<(u32, String)>) -> (i32, GString) {
    p.as_ref().map_or((-1, GString::new()), |(id, name)| (*id as i32, gs(name)))
}

fn province_dict(r: &stats::ProvinceRow) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("id", r.id as i32);
    d.set("name", &gs(&r.name));
    d.set("population", r.population);
    d.set("aura", r.aura);
    d.set("karma_qi", r.karma_qi);
    d.set("merit", r.merit);
    d.set("righteous", r.righteous);
    d.set("value", r.value);
    d
}

fn person_dict(r: &stats::PersonRow) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("id", r.id as i32);
    d.set("name", &gs(&r.name));
    d.set("realm", r.realm);
    d.set("fruit", r.fruit);
    d.set("role", &gs(&r.role));
    d.set("power_name", &gs(&r.power_name));
    d.set("is_leader", r.is_leader);
    d.set("age", r.age as i32);
    d.set("province", r.province as i32);
    d.set("province_name", &gs(&r.province_name));
    d
}

fn thread_dict(r: &stats::ThreadRow) -> VarDictionary {
    let mut d = VarDictionary::new();
    let (debtor, debtor_name) = person_ref(&r.debtor);
    let (creditor, creditor_name) = person_ref(&r.creditor);
    d.set("thread", r.thread as i32);
    d.set("kind", r.kind as i32);
    d.set("strength", r.strength);
    d.set("lives", r.lives as i32);
    d.set("debtor", debtor);
    d.set("debtor_name", &debtor_name);
    d.set("creditor", creditor);
    d.set("creditor_name", &creditor_name);
    d
}

fn rows<T>(total: usize, list: &[T], convert: impl Fn(&T) -> VarDictionary) -> VarDictionary {
    let mut out = VarDictionary::new();
    let mut array = Array::<VarDictionary>::new();
    for r in list {
        array.push(&convert(r));
    }
    out.set("total", total as i32);
    out.set("rows", &array);
    out
}

#[godot_api(secondary)]
impl SimWorld {
    /// 天下頁的總覽：`population`、`capacity`、`alive`（在世有名之人）、`provinces`（州郡數）、`years`（開天至今）、
    /// `continents`（每筆 `name`、`population`、`provinces`；沒有歸屬的海島為最後一筆、名字空字串）、
    /// `terrain`（陸地各地形的格數，索引為地形代碼 0 海 .. 8 澤，海與湖為 0）、`faith`（道、釋、儒、未受教化的人口占比）、
    /// `load`、`limit`（天地負荷與承受上限）、`climate`（`Cold`／`Mild`／`Warm`）、
    /// `years_since_cataclysm`（距上次大劫的年數，從沒有為 -1）、`in_cataclysm`。
    #[func]
    fn stats_world(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let st = stats::world_stats(s.world());
        d.set("population", st.total_population);
        d.set("capacity", st.total_capacity);
        d.set("alive", st.alive as i32);
        d.set("provinces", st.provinces as i32);
        d.set("years", st.years as i32);
        let mut continents = Array::<VarDictionary>::new();
        for c in &st.continents {
            let mut row = VarDictionary::new();
            row.set("name", &gs(&c.name));
            row.set("population", c.population);
            row.set("provinces", c.provinces as i32);
            continents.push(&row);
        }
        d.set("continents", &continents);
        let terrain: Vec<i64> = st.terrain.iter().map(|&n| n as i64).collect();
        d.set("terrain", &PackedInt64Array::from(terrain.as_slice()));
        let faith: Vec<f64> = st.faith.to_vec();
        d.set("faith", &PackedFloat64Array::from(faith.as_slice()));
        d.set("load", st.load);
        d.set("limit", st.limit);
        d.set("climate", &gs(&st.climate));
        d.set("years_since_cataclysm", st.years_since_cataclysm.map_or(-1, |y| y as i32));
        d.set("in_cataclysm", st.in_cataclysm);
        d
    }

    /// 州郡排行：`sort` 為 `population`、`aura`、`karma_qi`、`merit`、`righteous`；`descending` 為假由小到大。
    /// 回傳 `total`（州郡總數）與 `rows`（前 `limit` 筆，每筆 `id`、`name`、`population`、`aura`、`karma_qi`、`merit`、`righteous`、`value`）。
    #[func]
    fn stats_provinces(&self, sort: GString, limit: i32, descending: bool) -> VarDictionary {
        let Some(s) = self.sim.as_ref() else { return VarDictionary::new() };
        let (total, list) = stats::provinces_ranked(s.world(), &sort.to_string(), limit.max(0) as usize, descending);
        rows(total, &list, province_dict)
    }

    /// 一群州郡的名單：`kind` 為 `continent`（`value` 為大洲序）、`terrain`（地形代碼）、`faith`（0 道、1 釋、2 儒、3 未受教化）。
    /// 欄位同 `stats_provinces`，`value` 是排序依據（人口或占比）。
    #[func]
    fn province_roster(&self, kind: GString, value: i32, limit: i32) -> VarDictionary {
        let Some(s) = self.sim.as_ref() else { return VarDictionary::new() };
        let (total, list) = stats::province_roster(s.world(), &kind.to_string(), value, limit.max(0) as usize);
        rows(total, &list, province_dict)
    }

    /// 修士頁的總覽：`alive`、`realms`（煉氣至大乘各境界人數，9 筆）、`non_cultivators`（非修士，與 `realms` 相加為 `alive`）、
    /// `origins`（每筆 `origin`〔門派編號；-1 散修、-2 無門派的非修士〕、`name`、`count`，總和為 `alive`）、`male`、`female`、
    /// `ages`（少、壯、老）、`age_mean`、`eldest_id`、`eldest_age`（沒有為 -1）、`ways`（正道、魔道、妖）、`fallen_to_demon`（入魔者）。
    #[func]
    fn stats_people(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let w = s.world();
        let st = stats::people_stats(w);
        let ints = |v: &[usize]| PackedInt32Array::from(v.iter().map(|&n| n as i32).collect::<Vec<_>>().as_slice());
        d.set("alive", st.alive as i32);
        d.set("realms", &ints(&st.realms));
        d.set("non_cultivators", st.non_cultivators as i32);
        let mut origins = Array::<VarDictionary>::new();
        for o in &st.origins {
            let mut row = VarDictionary::new();
            row.set("origin", o.origin);
            row.set("name", &gs(&o.name));
            row.set("count", o.count as i32);
            origins.push(&row);
        }
        d.set("origins", &origins);
        d.set("male", st.male as i32);
        d.set("female", st.female as i32);
        d.set("ages", &ints(&st.ages));
        d.set("age_mean", st.age_mean);
        d.set("eldest_id", st.eldest.map_or(-1, |(id, _)| id as i32));
        d.set("eldest_age", st.eldest.map_or(-1, |(_, age)| age as i32));
        d.set("ways", &ints(&st.ways));
        d.set("fallen_to_demon", st.fallen_to_demon as i32);
        d
    }

    /// 符合條件的在世有名之人，依修為高低。`kind`：`realm`（`value` 為境界 0..8，-1 為非修士）、`origin`（同 `stats_people` 的 `origin`）、
    /// `gender`（0 男、1 女）、`age`（0 少、1 壯、2 老）、`way`（0 正道、1 魔道、2 妖、3 入魔者）、`all`、`top`（境界最高的人，即「高人」）。
    /// 回傳 `total` 與 `rows`（每筆 `id`、`name`、`realm`、`fruit`、`role`、`power_name`、`is_leader`、`age`、`province`、`province_name`）。
    #[func]
    fn people_where(&self, kind: GString, value: i32, limit: i32) -> VarDictionary {
        let Some(s) = self.sim.as_ref() else { return VarDictionary::new() };
        let kind = kind.to_string();
        let kind = if kind == "top" { "all" } else { kind.as_str() };
        let (total, list) = stats::people_where(s.world(), kind, value, limit.max(0) as usize);
        rows(total, &list, person_dict)
    }

    /// 近 `years` 年各項事件的次數：以項目代號為鍵（`tribulation_passed`、`tribulation_failed`、`ascended`、`fell_to_demon`、
    /// `fallen_high`、`sect_founded`、`sect_split`、`sect_war`、`sect_fallen`）。
    #[func]
    fn recent_counts(&self, years: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        for (key, n) in stats::recent_counts(s.world(), years.max(0) as u32) {
            d.set(key, n as i32);
        }
        d
    }

    /// 近 `years` 年某一項事件的名單：`total` 與 `ids`（事件編號，新到舊，至多 `limit` 筆），事件內容用 `event(id)` 取。
    #[func]
    fn recent_events(&self, key: GString, years: i32, limit: i32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let (total, ids) = stats::recent_events(s.world(), &key.to_string(), years.max(0) as u32, limit.max(0) as usize);
        let ids: Vec<i64> = ids.into_iter().map(i64::from).collect();
        d.set("total", total as i32);
        d.set("ids", &PackedInt64Array::from(ids.as_slice()));
        d
    }

    /// 宗門頁：`alive`（現存宗門，依門人數；每筆 `id`、`name`、`members`、`top_realm`、`leader`、`leader_name`、`age_years`、`evil`、
    /// `province`、`province_name`、`pos`〔山門格心的地圖座標〕、`continent`）、`by_continent`（每筆 `index`、`name`、`count`）、
    /// `fallen`（已覆滅的宗門，近的在前；每筆 `id`、`name`、`founded_year`、`fallen_year`、`lasted_years`、`evil`、`province`、
    /// `province_name`、`event`〔覆滅那則事件，沒有為 -1〕、`cause_kind`〔第一個起因事件的種類，沒有為空字串〕）。
    /// 覆滅時的門人數與首領沒有留下。
    #[func]
    fn stats_sects(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let w = s.world();
        let gw = w.geo.w as u32;
        let st = stats::sect_stats(w);
        let mut alive = Array::<VarDictionary>::new();
        for r in &st.alive {
            let mut row = VarDictionary::new();
            let (leader, leader_name) = person_ref(&r.leader);
            row.set("id", r.id as i32);
            row.set("name", &gs(&r.name));
            row.set("members", r.members as i32);
            row.set("top_realm", r.top_realm);
            row.set("leader", leader);
            row.set("leader_name", &leader_name);
            row.set("age_years", r.age_years as i32);
            row.set("evil", r.evil);
            row.set("province", r.province as i32);
            row.set("province_name", &gs(&r.province_name));
            row.set("pos", Vector2::new((r.seat_cell % gw) as f32 + 0.5, (r.seat_cell / gw) as f32 + 0.5));
            row.set("continent", r.continent as i32);
            alive.push(&row);
        }
        d.set("alive", &alive);
        let mut by_continent = Array::<VarDictionary>::new();
        for (index, name, count) in &st.by_continent {
            let mut row = VarDictionary::new();
            row.set("index", *index as i32);
            row.set("name", &gs(name));
            row.set("count", *count as i32);
            by_continent.push(&row);
        }
        d.set("by_continent", &by_continent);
        let mut fallen = Array::<VarDictionary>::new();
        for r in &st.fallen {
            let mut row = VarDictionary::new();
            row.set("id", r.id as i32);
            row.set("name", &gs(&r.name));
            row.set("founded_year", r.founded_year as i32);
            row.set("fallen_year", r.fallen_year as i32);
            row.set("founded_era", &gs(&r.founded_era));
            row.set("founded_era_year", r.founded_era_year as i32);
            row.set("fallen_era", &gs(&r.fallen_era));
            row.set("fallen_era_year", r.fallen_era_year as i32);
            row.set("lasted_years", r.lasted_years as i32);
            row.set("evil", r.evil);
            row.set("province", r.province as i32);
            row.set("province_name", &gs(&r.province_name));
            row.set("event", r.event.map_or(-1, |e| e as i64));
            row.set("cause_kind", &gs(&r.cause_kind));
            fallen.push(&row);
        }
        d.set("fallen", &fallen);
        d
    }

    /// 因果頁：`counts`（殺業、恩義、情緣、師徒、怨五類未了宿緣的條數）、`tangled`（身上未了宿緣最多的 20 個魂；
    /// 每筆 `soul`、`person`〔最近一世，沒有為 -1〕、`name`、`open`、`strength`）。
    #[func]
    fn stats_karma(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(s) = self.sim.as_ref() else { return d };
        let st = stats::karma_stats(s.world());
        let counts: Vec<i32> = st.counts.iter().map(|&n| n as i32).collect();
        d.set("counts", &PackedInt32Array::from(counts.as_slice()));
        let mut tangled = Array::<VarDictionary>::new();
        for r in &st.tangled {
            let mut row = VarDictionary::new();
            let (person, name) = person_ref(&r.person);
            row.set("soul", r.soul as i32);
            row.set("person", person);
            row.set("name", &name);
            row.set("open", r.open as i32);
            row.set("strength", r.strength);
            tangled.push(&row);
        }
        d.set("tangled", &tangled);
        d
    }

    /// 某一類（0 殺業 .. 4 怨）最深的 20 條未了宿緣：`total`（該類條數）與 `rows`
    /// （每筆 `thread`、`kind`、`strength`、`lives`、`debtor`、`debtor_name`、`creditor`、`creditor_name`；兩端為魂最近一世的人物）。
    #[func]
    fn karma_roster(&self, kind: i32) -> VarDictionary {
        let Some(s) = self.sim.as_ref() else { return VarDictionary::new() };
        let (total, list) = stats::threads_of_kind(s.world(), kind.max(0) as usize);
        rows(total, &list, thread_dict)
    }
}
