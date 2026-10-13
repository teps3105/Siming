//! GDD 5.10 天下冊的統計查詢：只讀世界，不改任何狀態，也不進世界雜湊。
//!
//! 畫面不讀遊戲規則，也不自己從事件重算：天下、修士、宗門、因果四頁要的數字都在這裡算好。
//! 年份的差一律走 [`calendar`] 的換算（開天以來的年序），不直接用 tick 的單位。
//! 結果一律是排序固定的 `Vec`（不洩漏 `HashMap` 的走訪順序），同一個世界查兩次結果相同。

use std::collections::BTreeMap;

use crate::calendar;
use crate::cataclysm;
use crate::evolution::year_of;
use crate::model::*;
use crate::sim::World;
use crate::thread::{is_open, lives};
use crate::types::{ProvinceId, Terrain, NO_CONTINENT};

/// 年齡分三段時的界線：年歲不足壽元的這個比例算少，達到另一個比例算老。
pub const YOUNG_UNTIL: f32 = 0.25;
pub const OLD_FROM: f32 = 0.7;
/// 「近百年」的長度（年）。
pub const RECENT_YEARS: u32 = 100;
/// 修士境界的數目（煉氣至大乘）。
pub const REALMS: usize = 9;

// ── 天下 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ContinentRow {
    pub name: String,
    pub population: f64,
    pub provinces: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldStats {
    pub total_population: f64,
    pub total_capacity: f64,
    pub alive: usize,
    pub provinces: usize,
    /// 開天至今的年數（開天以來第幾年）。
    pub years: u32,
    /// 各大洲；沒有歸屬任何大洲的州郡（海島）合為最後一列。
    pub continents: Vec<ContinentRow>,
    /// 陸地各地形的格數，索引為 [`Terrain`] 代碼；海與湖恆為 0。
    pub terrain: [u64; Terrain::COUNT],
    /// 道、釋、儒、未受教化的人口占比（和為 1；沒有人口時全 0）。
    pub faith: [f64; 4],
    pub load: f64,
    pub limit: f64,
    pub climate: String,
    /// 距上次大劫的年數；從沒降過大劫為 `None`。
    pub years_since_cataclysm: Option<u32>,
    /// 此刻是否在大劫之中。
    pub in_cataclysm: bool,
}

/// 州郡屬於哪個大洲（0 起算；沒有歸屬為大洲數，即「海島」那一列）。
pub fn continent_of_province(w: &World, p: &crate::types::Province) -> usize {
    match w.geo.continent_of.get(p.seat as usize) {
        Some(&c) if c != NO_CONTINENT => c as usize,
        _ => w.geo.continents.len(),
    }
}

fn live_provinces(w: &World) -> impl Iterator<Item = &crate::types::Province> {
    w.provinces.iter().filter(|p| !p.lost)
}

pub fn world_stats(w: &World) -> WorldStats {
    let mut continents: Vec<ContinentRow> =
        w.geo.continents.iter().map(|c| ContinentRow { name: c.name.clone(), population: 0.0, provinces: 0 }).collect();
    let mut isles = ContinentRow { name: String::new(), population: 0.0, provinces: 0 };
    let mut terrain = [0u64; Terrain::COUNT];
    let mut faith = [0.0f64; 4];
    let mut total_population = 0.0;
    let mut total_capacity = 0.0;
    let mut provinces = 0;
    for p in live_provinces(w) {
        provinces += 1;
        let pop = p.population.max(0.0);
        total_population += pop;
        total_capacity += p.capacity.max(0.0);
        let row = match continent_of_province(w, p) {
            c if c < continents.len() => &mut continents[c],
            _ => &mut isles,
        };
        row.population += pop;
        row.provinces += 1;
        for t in Terrain::ALL {
            if !t.is_water() {
                terrain[t as usize] += u64::from(p.terrain_mix[t as usize]);
            }
        }
        let sum: f64 = p.faith.iter().map(|&f| f64::from(f)).sum::<f64>().min(1.0);
        for (i, &f) in p.faith.iter().enumerate() {
            faith[i] += pop * f64::from(f).clamp(0.0, 1.0);
        }
        faith[3] += pop * (1.0 - sum).max(0.0);
    }
    if isles.provinces > 0 {
        continents.push(isles);
    }
    let face: f64 = faith.iter().sum();
    if face > 0.0 {
        for f in &mut faith {
            *f /= face;
        }
    }
    let now = year_of(w.tick);
    let years_since_cataclysm = w.calamity.history.last().map(|&(year, _)| now.saturating_sub(year));
    WorldStats {
        total_population,
        total_capacity,
        alive: w.alive_ids.len(),
        provinces,
        years: calendar::total_year(w.tick, w.evolution.history_years),
        continents,
        terrain,
        faith,
        load: cataclysm::load(w),
        limit: cataclysm::limit(w),
        climate: format!("{:?}", w.evolution.climate),
        years_since_cataclysm,
        in_cataclysm: w.calamity.ends.is_some_and(|e| now < e),
    }
}

// ── 州郡排行 ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ProvinceRow {
    pub id: ProvinceId,
    pub name: String,
    pub population: f64,
    pub aura: f32,
    pub karma_qi: f32,
    pub merit: f32,
    pub righteous: f32,
    /// 排行或名單依據的那個數（人口、占比等）；排行表用它排序。
    pub value: f64,
}

fn province_row(p: &crate::types::Province, value: f64) -> ProvinceRow {
    ProvinceRow {
        id: p.id,
        name: p.name.clone(),
        population: p.population,
        aura: p.aura,
        karma_qi: p.karma_qi,
        merit: p.merit,
        righteous: p.righteous,
        value,
    }
}

fn sort_key(p: &crate::types::Province, sort: &str) -> f64 {
    match sort {
        "aura" => f64::from(p.aura),
        "karma_qi" => f64::from(p.karma_qi),
        "merit" => f64::from(p.merit),
        "righteous" => f64::from(p.righteous),
        _ => p.population,
    }
}

/// 州郡依 `sort`（`population`、`aura`、`karma_qi`、`merit`、`righteous`）排行：`descending` 為真由大到小，否則由小到大；
/// 同值依州郡編號。回傳至多 `limit` 筆與州郡總數。
pub fn provinces_ranked(w: &World, sort: &str, limit: usize, descending: bool) -> (usize, Vec<ProvinceRow>) {
    let mut rows: Vec<ProvinceRow> = live_provinces(w).map(|p| province_row(p, sort_key(p, sort))).collect();
    rows.sort_by(|a, b| {
        let order = a.value.total_cmp(&b.value);
        (if descending { order.reverse() } else { order }).then(a.id.cmp(&b.id))
    });
    let total = rows.len();
    rows.truncate(limit);
    (total, rows)
}

/// 一群州郡的名單：`kind` 為 `continent`（`value` 是大洲序，等於大洲數為海島；依人口）、
/// `terrain`（`value` 是地形代碼；依該地形占州郡陸地格的比例）、`faith`（`value` 是 0 道、1 釋、2 儒、3 未受教化；依比例）。
/// 回傳符合條件的州郡總數與前 `limit` 筆；`value` 欄是排序依據。
pub fn province_roster(w: &World, kind: &str, value: i32, limit: usize) -> (usize, Vec<ProvinceRow>) {
    let mut rows: Vec<ProvinceRow> = Vec::new();
    for p in live_provinces(w) {
        match kind {
            "continent" => {
                if continent_of_province(w, p) as i32 == value {
                    rows.push(province_row(p, p.population));
                }
            }
            "terrain" => {
                let land: u32 = Terrain::ALL.iter().filter(|t| !t.is_water()).map(|&t| p.terrain_mix[t as usize]).sum();
                let cells = usize::try_from(value).ok().and_then(|v| p.terrain_mix.get(v)).copied().unwrap_or(0);
                if cells > 0 && land > 0 {
                    rows.push(province_row(p, f64::from(cells) / f64::from(land)));
                }
            }
            "faith" => {
                let share = match value {
                    0..=2 => f64::from(p.faith[value as usize]),
                    3 => (1.0 - f64::from(p.faith.iter().sum::<f32>())).max(0.0),
                    _ => 0.0,
                };
                if share > 0.0 {
                    rows.push(province_row(p, share));
                }
            }
            _ => {}
        }
    }
    rows.sort_by(|a, b| b.value.total_cmp(&a.value).then(a.id.cmp(&b.id)));
    let total = rows.len();
    rows.truncate(limit);
    (total, rows)
}

// ── 修士 ─────────────────────────────────────────────────────

/// 道途：只對修士（境界路徑）有意義。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// 正道：修士中不屬魔門、未入魔、非妖者。
    Righteous,
    /// 魔道：入魔者，或魔門（邪道宗門）的門人。
    Demonic,
    /// 妖。
    Beast,
}

/// 人物的道途；不是修士回傳 `None`。
pub fn way_of(w: &World, p: &Person) -> Option<Way> {
    if !matches!(p.path, PathState::Cultivator { .. }) {
        return None;
    }
    if p.role == Role::Beast {
        return Some(Way::Beast);
    }
    let evil_sect = p.power.and_then(|id| w.powers.get(id as usize)).and_then(crate::dao::sect_of).is_some_and(|s| s.evil);
    if p.role == Role::Demonic || evil_sect {
        Some(Way::Demonic)
    } else {
        Some(Way::Righteous)
    }
}

/// 出身的編號：門派編號（≥ 0）；`ROGUE` 散修（修士而無門派）；`PLAIN` 無門派的非修士。
pub const ORIGIN_ROGUE: i32 = -1;
pub const ORIGIN_PLAIN: i32 = -2;

pub fn origin_of(p: &Person) -> i32 {
    match p.power {
        Some(id) => id as i32,
        None if matches!(p.path, PathState::Cultivator { .. }) => ORIGIN_ROGUE,
        None => ORIGIN_PLAIN,
    }
}

pub fn realm_of(p: &Person) -> i32 {
    match p.path {
        PathState::Cultivator { realm, .. } => i32::from(realm),
        _ => -1,
    }
}

fn exp_of(p: &Person) -> f32 {
    match p.path {
        PathState::Cultivator { exp, .. } => exp,
        _ => 0.0,
    }
}

/// 年紀分段：0 少、1 壯、2 老，依年歲占壽元的比例。
pub fn age_band(w: &World, p: &Person) -> usize {
    let ratio = p.age(w.tick) as f32 / f32::from(p.lifespan.max(1));
    if ratio < YOUNG_UNTIL {
        0
    } else if ratio < OLD_FROM {
        1
    } else {
        2
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OriginRow {
    /// 門派編號，或 [`ORIGIN_ROGUE`]、[`ORIGIN_PLAIN`]。
    pub origin: i32,
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeopleStats {
    pub alive: usize,
    /// 各境界人數（煉氣至大乘）。
    pub realms: [usize; REALMS],
    /// 不是修士的在世有名之人（凡人、僧、儒等）；與 `realms` 相加等於 `alive`。
    pub non_cultivators: usize,
    /// 出身：各宗門依人數由多到少，其後散修、無門派的非修士；人數為 0 的不列。總和為 `alive`。
    pub origins: Vec<OriginRow>,
    pub male: usize,
    pub female: usize,
    /// 少、壯、老。
    pub ages: [usize; 3],
    pub age_mean: f32,
    pub eldest: Option<(PersonId, u32)>,
    /// 修士的道途：正道、魔道、妖；以及其中的入魔者。
    pub ways: [usize; 3],
    pub fallen_to_demon: usize,
}

fn alive_people(w: &World) -> impl Iterator<Item = &Person> {
    w.alive_ids.iter().filter_map(|&i| w.persons.get(i as usize)).filter(|p| p.alive())
}

pub fn people_stats(w: &World) -> PeopleStats {
    let mut s = PeopleStats {
        alive: 0,
        realms: [0; REALMS],
        non_cultivators: 0,
        origins: Vec::new(),
        male: 0,
        female: 0,
        ages: [0; 3],
        age_mean: 0.0,
        eldest: None,
        ways: [0; 3],
        fallen_to_demon: 0,
    };
    let mut by_origin: BTreeMap<i32, usize> = BTreeMap::new();
    let mut age_sum = 0u64;
    for p in alive_people(w) {
        s.alive += 1;
        match realm_of(p) {
            r if r >= 0 => s.realms[(r as usize).min(REALMS - 1)] += 1,
            _ => s.non_cultivators += 1,
        }
        *by_origin.entry(origin_of(p)).or_default() += 1;
        if p.gender == Gender::Male {
            s.male += 1;
        } else {
            s.female += 1;
        }
        s.ages[age_band(w, p)] += 1;
        let age = p.age(w.tick);
        age_sum += u64::from(age);
        if s.eldest.is_none_or(|(_, a)| age > a) {
            s.eldest = Some((p.id, age));
        }
        match way_of(w, p) {
            Some(Way::Righteous) => s.ways[0] += 1,
            Some(Way::Demonic) => s.ways[1] += 1,
            Some(Way::Beast) => s.ways[2] += 1,
            None => {}
        }
        if p.role == Role::Demonic {
            s.fallen_to_demon += 1;
        }
    }
    if s.alive > 0 {
        s.age_mean = age_sum as f32 / s.alive as f32;
    }
    let mut sects: Vec<OriginRow> = Vec::new();
    for (&origin, &count) in &by_origin {
        if origin >= 0 {
            let name = w.powers.get(origin as usize).map_or(String::new(), |pw| pw.name.clone());
            sects.push(OriginRow { origin, name, count });
        }
    }
    sects.sort_by(|a, b| b.count.cmp(&a.count).then(a.origin.cmp(&b.origin)));
    s.origins = sects;
    for (origin, count) in [(ORIGIN_ROGUE, by_origin.get(&ORIGIN_ROGUE)), (ORIGIN_PLAIN, by_origin.get(&ORIGIN_PLAIN))] {
        if let Some(&count) = count {
            s.origins.push(OriginRow { origin, name: String::new(), count });
        }
    }
    s
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersonRow {
    pub id: PersonId,
    pub name: String,
    /// 修士境界，非修士為 -1。
    pub realm: i32,
    /// 僧的果位，非僧為 -1。
    pub fruit: i32,
    pub role: String,
    pub power_name: String,
    pub is_leader: bool,
    pub age: u32,
    pub province: ProvinceId,
    pub province_name: String,
}

fn person_row(w: &World, p: &Person) -> PersonRow {
    let power = p.power.and_then(|id| w.powers.get(id as usize));
    PersonRow {
        id: p.id,
        name: p.name.clone(),
        realm: realm_of(p),
        fruit: if let PathState::Monk { fruit, .. } = p.path { i32::from(fruit) } else { -1 },
        role: format!("{:?}", p.role),
        power_name: power.map_or(String::new(), |pw| pw.name.clone()),
        is_leader: power.is_some_and(|pw| pw.leader == Some(p.id)),
        age: p.age(w.tick),
        province: p.province,
        province_name: w.provinces.get(p.province as usize).map_or(String::new(), |q| q.name.clone()),
    }
}

/// 修為高低的排序：境界高者在前，同境界修為多者在前，同值依編號。
fn by_cultivation(a: &Person, b: &Person) -> std::cmp::Ordering {
    realm_of(b).cmp(&realm_of(a)).then(exp_of(b).total_cmp(&exp_of(a))).then(a.id.cmp(&b.id))
}

/// 符合條件的在世有名之人，依修為高低；回傳符合者總數與前 `limit` 筆。`kind`：
/// `realm`（`value` 為境界 0..8，-1 為非修士）、`origin`（`value` 為 [`origin_of`]）、`gender`（0 男、1 女）、
/// `age`（0 少、1 壯、2 老）、`way`（0 正道、1 魔道、2 妖、3 入魔者）、`all`（全部）。
pub fn people_where(w: &World, kind: &str, value: i32, limit: usize) -> (usize, Vec<PersonRow>) {
    let mut found: Vec<&Person> = alive_people(w)
        .filter(|p| match kind {
            "realm" => realm_of(p) == value,
            "origin" => origin_of(p) == value,
            "gender" => (p.gender == Gender::Female) as i32 == value,
            "age" => age_band(w, p) as i32 == value,
            "way" => match value {
                0 => way_of(w, p) == Some(Way::Righteous),
                1 => way_of(w, p) == Some(Way::Demonic),
                2 => way_of(w, p) == Some(Way::Beast),
                3 => p.role == Role::Demonic,
                _ => false,
            },
            "all" => true,
            _ => false,
        })
        .collect();
    found.sort_by(|a, b| by_cultivation(a, b));
    let total = found.len();
    (total, found.into_iter().take(limit).map(|p| person_row(w, p)).collect())
}

// ── 近百年 ───────────────────────────────────────────────────

/// 近百年統計的項目代號。
pub const RECENT_KEYS: [&str; 9] = [
    "tribulation_passed",
    "tribulation_failed",
    "ascended",
    "fell_to_demon",
    "fallen_high",
    "sect_founded",
    "sect_split",
    "sect_war",
    "sect_fallen",
];

fn recent_key(e: &Event) -> Option<&'static str> {
    Some(match (e.kind, &e.args) {
        (EventKind::Tribulation, EventArgs::Tribulation { outcome, .. }) => match outcome {
            TribulationOutcome::Passed => "tribulation_passed",
            _ => "tribulation_failed",
        },
        (EventKind::Ascended, _) => "ascended",
        (EventKind::FellToDemon, _) => "fell_to_demon",
        // 元嬰以上的修士與君主身故才有 3 以上的重要度（`people::death`）。
        (EventKind::Died, _) if e.importance >= 3 => "fallen_high",
        (EventKind::PowerFounded, _) => "sect_founded",
        (EventKind::SectSplit, _) => "sect_split",
        (EventKind::SectWar, _) => "sect_war",
        (EventKind::PowerFallen, _) => "sect_fallen",
        _ => return None,
    })
}

/// 事件是否落在近 `years` 年之內（含今年）。
fn within(w: &World, e: &Event, years: u32) -> bool {
    let now = calendar::total_year(w.tick, w.evolution.history_years);
    let at = calendar::total_year(e.tick, w.evolution.history_years);
    at + years > now
}

/// 近 `years` 年各項事件的次數，順序同 [`RECENT_KEYS`]。因果帳修剪過的事件（重要度 0）不算。
pub fn recent_counts(w: &World, years: u32) -> Vec<(&'static str, usize)> {
    let mut counts = vec![0usize; RECENT_KEYS.len()];
    for e in w.ledger.events.iter().rev() {
        if e.importance == 0 {
            continue;
        }
        if !within(w, e, years) {
            break;
        }
        if let Some(key) = recent_key(e) {
            if let Some(i) = RECENT_KEYS.iter().position(|&k| k == key) {
                counts[i] += 1;
            }
        }
    }
    RECENT_KEYS.iter().copied().zip(counts).collect()
}

/// 近 `years` 年某一項的事件編號，新到舊；回傳總數與前 `limit` 筆。
pub fn recent_events(w: &World, key: &str, years: u32, limit: usize) -> (usize, Vec<EventId>) {
    let mut ids = Vec::new();
    let mut total = 0;
    for e in w.ledger.events.iter().rev() {
        if e.importance == 0 {
            continue;
        }
        if !within(w, e, years) {
            break;
        }
        if recent_key(e) == Some(key) {
            total += 1;
            if ids.len() < limit {
                ids.push(e.id);
            }
        }
    }
    (total, ids)
}

// ── 宗門 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SectRow {
    pub id: PowerId,
    pub name: String,
    pub members: usize,
    /// 在世門人中最高的境界；沒有修士門人為 -1。
    pub top_realm: i32,
    pub leader: Option<(PersonId, String)>,
    /// 立派至今的年數。
    pub age_years: u32,
    pub evil: bool,
    pub province: ProvinceId,
    pub province_name: String,
    pub seat_cell: u32,
    /// 山門所在的大洲序（海島為大洲數）。
    pub continent: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FallenRow {
    pub id: PowerId,
    pub name: String,
    pub founded_year: u32,
    pub fallen_year: u32,
    /// 存續的年數。
    pub lasted_years: u32,
    pub evil: bool,
    pub province: ProvinceId,
    pub province_name: String,
    /// 覆滅那則事件；因果帳沒有留下為 `None`。
    pub event: Option<EventId>,
    /// 覆滅的第一個起因事件的種類（`SectWar` 等）；沒有為空字串。
    pub cause_kind: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectStats {
    pub alive: Vec<SectRow>,
    /// 各大洲現存的宗門數（大洲序、洲名、數目），含海島列；數目為 0 的洲也列。
    pub by_continent: Vec<(usize, String, usize)>,
    pub fallen: Vec<FallenRow>,
}

pub fn sect_stats(w: &World) -> SectStats {
    let hist = w.evolution.history_years;
    let now = calendar::total_year(w.tick, hist);
    let mut count_by_power: BTreeMap<PowerId, (usize, i32)> = BTreeMap::new();
    for p in alive_people(w) {
        if let Some(id) = p.power {
            let entry = count_by_power.entry(id).or_insert((0, -1));
            entry.0 += 1;
            entry.1 = entry.1.max(realm_of(p));
        }
    }
    let mut alive = Vec::new();
    let mut fallen_ids = Vec::new();
    for pw in &w.powers {
        let Some(sect) = crate::dao::sect_of(pw) else { continue };
        if pw.fallen.is_some() {
            fallen_ids.push(pw.id);
            continue;
        }
        let (members, top_realm) = count_by_power.get(&pw.id).copied().unwrap_or((0, -1));
        let province = w.provinces.get(pw.seat_province as usize);
        alive.push(SectRow {
            id: pw.id,
            name: pw.name.clone(),
            members,
            top_realm,
            leader: pw.leader.and_then(|l| w.persons.get(l as usize)).map(|l| (l.id, l.name.clone())),
            age_years: now.saturating_sub(calendar::total_year(pw.founded, hist)),
            evil: sect.evil,
            province: pw.seat_province,
            province_name: province.map_or(String::new(), |q| q.name.clone()),
            seat_cell: pw.seat_cell,
            continent: province.map_or(w.geo.continents.len(), |q| continent_of_province(w, q)),
        });
    }
    let mut counts = vec![0usize; w.geo.continents.len() + 1];
    for s in &alive {
        counts[s.continent.min(w.geo.continents.len())] += 1;
    }
    let by_continent = counts
        .iter()
        .enumerate()
        .filter(|&(i, &n)| i < w.geo.continents.len() || n > 0)
        .map(|(i, &n)| (i, w.geo.continents.get(i).map_or(String::new(), |c| c.name.clone()), n))
        .collect();
    alive.sort_by(|a, b| b.members.cmp(&a.members).then(a.id.cmp(&b.id)));

    // 覆滅的宗門：`Power` 留在表裡（成員與首領已清空），覆滅那則事件在因果帳（重要度 4，不修剪）。
    let mut fall_event: BTreeMap<PowerId, &Event> = BTreeMap::new();
    for e in &w.ledger.events {
        if e.kind == EventKind::PowerFallen && e.importance > 0 {
            if let Some(&id) = e.powers.first() {
                fall_event.insert(id, e);
            }
        }
    }
    let mut fallen: Vec<FallenRow> = fallen_ids
        .into_iter()
        .filter_map(|id| {
            let pw = &w.powers[id as usize];
            let fell_tick = pw.fallen?;
            let event = fall_event.get(&id).copied();
            let cause = event.and_then(|e| e.causes.first()).and_then(|&c| crate::ledger::get(w, c));
            let founded_year = calendar::total_year(pw.founded, hist);
            let fallen_year = calendar::total_year(fell_tick, hist);
            Some(FallenRow {
                id,
                name: pw.name.clone(),
                founded_year,
                fallen_year,
                lasted_years: fallen_year.saturating_sub(founded_year),
                evil: crate::dao::sect_of(pw).is_some_and(|s| s.evil),
                province: pw.seat_province,
                province_name: w.provinces.get(pw.seat_province as usize).map_or(String::new(), |q| q.name.clone()),
                event: event.map(|e| e.id),
                cause_kind: cause.map_or(String::new(), |c| format!("{:?}", c.kind)),
            })
        })
        .collect();
    // 近的覆滅在前。
    fallen.sort_by(|a, b| b.fallen_year.cmp(&a.fallen_year).then(a.id.cmp(&b.id)));
    SectStats { alive, by_continent, fallen }
}

// ── 因果 ─────────────────────────────────────────────────────

/// 宿緣種類的順序（同因果線）：殺業、恩義、情緣、師徒、怨。
pub const THREAD_KINDS: usize = 5;
/// 每類列出最深的幾條、牽連最深列出幾個魂。
pub const KARMA_TOP: usize = 20;

pub fn thread_kind_index(kind: ThreadKind) -> usize {
    match kind {
        ThreadKind::Feud => 0,
        ThreadKind::Grace => 1,
        ThreadKind::Love => 2,
        ThreadKind::Mentor => 3,
        ThreadKind::Grudge => 4,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreadRow {
    pub thread: ThreadId,
    pub kind: usize,
    pub strength: f32,
    /// 經歷的世數（`thread::lives`）。
    pub lives: u16,
    /// 欠債方、債主兩端魂最近一世的人物與名字。
    pub debtor: Option<(PersonId, String)>,
    pub creditor: Option<(PersonId, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoulRow {
    pub soul: SoulId,
    /// 這個魂最近一世的人物。
    pub person: Option<(PersonId, String)>,
    /// 身上未了宿緣的條數。
    pub open: usize,
    pub strength: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KarmaStats {
    /// 五類未了宿緣的條數。
    pub counts: [usize; THREAD_KINDS],
    /// 每類最深（強度最大）的 [`KARMA_TOP`] 條。
    pub deepest: Vec<Vec<ThreadRow>>,
    /// 身上未了宿緣最多的 [`KARMA_TOP`] 個魂。
    pub tangled: Vec<SoulRow>,
}

fn latest_life(w: &World, soul: SoulId) -> Option<(PersonId, String)> {
    let p = w.souls.get(soul as usize).and_then(|s| s.lives.last()).and_then(|&id| w.persons.get(id as usize))?;
    Some((p.id, p.name.clone()))
}

pub fn karma_stats(w: &World) -> KarmaStats {
    let mut counts = [0usize; THREAD_KINDS];
    let mut per_kind: Vec<Vec<&Thread>> = vec![Vec::new(); THREAD_KINDS];
    let mut open_by_soul: BTreeMap<SoulId, (usize, f32)> = BTreeMap::new();
    for t in w.threads.iter().filter(|t| is_open(t)) {
        let k = thread_kind_index(t.kind);
        counts[k] += 1;
        per_kind[k].push(t);
        for soul in [t.debtor, t.creditor] {
            let e = open_by_soul.entry(soul).or_default();
            e.0 += 1;
            e.1 += t.strength;
        }
    }
    let deepest = per_kind
        .into_iter()
        .map(|mut list| {
            list.sort_by(|a, b| b.strength.total_cmp(&a.strength).then(a.id.cmp(&b.id)));
            list.into_iter()
                .take(KARMA_TOP)
                .map(|t| ThreadRow {
                    thread: t.id,
                    kind: thread_kind_index(t.kind),
                    strength: t.strength,
                    lives: lives(w, t),
                    debtor: latest_life(w, t.debtor),
                    creditor: latest_life(w, t.creditor),
                })
                .collect()
        })
        .collect();
    let mut souls: Vec<(SoulId, usize, f32)> = open_by_soul.into_iter().map(|(s, (n, sum))| (s, n, sum)).collect();
    souls.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.total_cmp(&a.2)).then(a.0.cmp(&b.0)));
    let tangled = souls
        .into_iter()
        .take(KARMA_TOP)
        .map(|(soul, open, strength)| SoulRow { soul, person: latest_life(w, soul), open, strength })
        .collect();
    KarmaStats { counts, deepest, tangled }
}

/// 某一類未了宿緣的全部條數與最深的 [`KARMA_TOP`] 條（名單用）。
pub fn threads_of_kind(w: &World, kind: usize) -> (usize, Vec<ThreadRow>) {
    let s = karma_stats(w);
    (s.counts.get(kind).copied().unwrap_or(0), s.deepest.into_iter().nth(kind).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Sim, WorldParams, WorldSize};

    fn aged(years: u32) -> Sim {
        let mut p = WorldParams::new(20261011);
        p.size = WorldSize::Small;
        let mut sim = Sim::new(&p).unwrap();
        for _ in 0..years * 12 {
            sim.step();
        }
        sim
    }

    #[test]
    fn realms_and_non_cultivators_add_up_to_the_living() {
        let sim = aged(30);
        let w = sim.world();
        let s = people_stats(w);
        assert!(s.alive > 0, "世界裡要有在世有名之人");
        assert_eq!(s.alive, w.alive_ids.len());
        assert_eq!(s.realms.iter().sum::<usize>() + s.non_cultivators, s.alive, "各境界加非修士等於在世有名之人");
        assert_eq!(world_stats(w).alive, s.alive);
        assert_eq!(s.male + s.female, s.alive);
        assert_eq!(s.ages.iter().sum::<usize>(), s.alive);
    }

    #[test]
    fn origins_add_up_and_match_the_sect_table() {
        let sim = aged(30);
        let w = sim.world();
        let s = people_stats(w);
        assert_eq!(s.origins.iter().map(|o| o.count).sum::<usize>(), s.alive, "出身各項總和等於在世有名之人");
        let sects = sect_stats(w);
        let from_origins: usize = s.origins.iter().filter(|o| o.origin >= 0).map(|o| o.count).sum();
        let from_table: usize = sects.alive.iter().map(|r| r.members).sum();
        assert_eq!(from_origins, from_table, "宗門榜門人數總和等於出身裡各宗門的總和");
        for o in s.origins.iter().filter(|o| o.origin >= 0) {
            assert_eq!(people_where(w, "origin", o.origin, usize::MAX).0, o.count, "名單人數等於圖上那一項");
        }
    }

    #[test]
    fn rosters_have_the_same_count_as_their_chart_items() {
        let sim = aged(30);
        let w = sim.world();
        let s = people_stats(w);
        for (r, &n) in s.realms.iter().enumerate() {
            assert_eq!(people_where(w, "realm", r as i32, 5).0, n);
        }
        assert_eq!(people_where(w, "realm", -1, 5).0, s.non_cultivators);
        assert_eq!(people_where(w, "gender", 0, 5).0, s.male);
        for (b, &n) in s.ages.iter().enumerate() {
            assert_eq!(people_where(w, "age", b as i32, 5).0, n);
        }
        for (k, &n) in s.ways.iter().enumerate() {
            assert_eq!(people_where(w, "way", k as i32, 5).0, n);
        }
        assert_eq!(people_where(w, "way", 3, 5).0, s.fallen_to_demon);
        let (total, rows) = people_where(w, "all", 0, 7);
        assert_eq!(total, s.alive);
        assert!(rows.len() <= 7);
        assert!(rows.windows(2).all(|p| p[0].realm >= p[1].realm), "名單依境界由高到低");
    }

    #[test]
    fn province_ranking_agrees_with_the_province_table() {
        let sim = aged(30);
        let w = sim.world();
        let (total, top) = provinces_ranked(w, "population", 10, true);
        assert_eq!(total, w.provinces.iter().filter(|p| !p.lost).count());
        assert!(top.windows(2).all(|p| p[0].population >= p[1].population));
        for r in &top {
            let p = &w.provinces[r.id as usize];
            assert_eq!(r.population, p.population);
            assert_eq!(r.name, p.name);
            assert_eq!(r.karma_qi, p.karma_qi);
        }
        let (_, bottom) = provinces_ranked(w, "population", 10, false);
        assert!(bottom.windows(2).all(|p| p[0].population <= p[1].population));
        let max = w.provinces.iter().filter(|p| !p.lost).map(|p| p.population).fold(f64::MIN, f64::max);
        assert_eq!(top[0].population, max);
        let (_, karma) = provinces_ranked(w, "karma_qi", 10, true);
        assert!(karma.windows(2).all(|p| p[0].karma_qi >= p[1].karma_qi));
    }

    #[test]
    fn world_totals_agree_with_the_provinces() {
        let sim = aged(10);
        let w = sim.world();
        let s = world_stats(w);
        let pop: f64 = w.provinces.iter().filter(|p| !p.lost).map(|p| p.population).sum();
        assert_eq!(s.total_population, pop);
        assert_eq!(s.continents.iter().map(|c| c.provinces).sum::<usize>(), s.provinces);
        assert!((s.continents.iter().map(|c| c.population).sum::<f64>() - pop).abs() < 1e-3 * pop.max(1.0));
        assert!((s.faith.iter().sum::<f64>() - 1.0).abs() < 1e-9 || pop == 0.0);
        assert_eq!(s.terrain[Terrain::Sea as usize], 0);
        assert_eq!(s.terrain[Terrain::Lake as usize], 0);
        assert!(s.years >= 1);
    }

    #[test]
    fn continent_roster_matches_the_continent_row() {
        let sim = aged(10);
        let w = sim.world();
        let s = world_stats(w);
        for (i, c) in s.continents.iter().enumerate() {
            let (total, rows) = province_roster(w, "continent", i as i32, usize::MAX);
            assert_eq!(total, c.provinces);
            assert!((rows.iter().map(|r| r.population).sum::<f64>() - c.population).abs() < 1e-6 * c.population.max(1.0));
        }
    }

    #[test]
    fn karma_counts_match_the_thread_table_and_rows_are_sorted() {
        let sim = aged(30);
        let w = sim.world();
        let k = karma_stats(w);
        assert_eq!(k.counts.iter().sum::<usize>(), w.threads.iter().filter(|t| is_open(t)).count());
        for (kind, list) in k.deepest.iter().enumerate() {
            assert!(list.len() <= KARMA_TOP && list.len() <= k.counts[kind]);
            assert!(list.windows(2).all(|p| p[0].strength >= p[1].strength));
            assert!(list.iter().all(|t| t.kind == kind));
        }
        assert!(k.tangled.windows(2).all(|p| p[0].open >= p[1].open));
        assert_eq!(threads_of_kind(w, 0).0, k.counts[0]);
    }

    #[test]
    fn recent_counts_equal_the_listed_events() {
        let sim = aged(60);
        let w = sim.world();
        for (key, n) in recent_counts(w, RECENT_YEARS) {
            assert_eq!(recent_events(w, key, RECENT_YEARS, 3).0, n, "{key}");
        }
        let shorter: usize = recent_counts(w, 10).iter().map(|c| c.1).sum();
        let longer: usize = recent_counts(w, RECENT_YEARS).iter().map(|c| c.1).sum();
        assert!(shorter <= longer);
    }

    #[test]
    fn fallen_sects_keep_their_record_in_the_power_table() {
        let sim = aged(200);
        let w = sim.world();
        let s = sect_stats(w);
        let fallen_in_table = w.powers.iter().filter(|p| p.fallen.is_some() && crate::dao::sect_of(p).is_some()).count();
        assert!(fallen_in_table > 0, "兩百年內要有覆滅的宗門，才驗得到覆滅的記錄");
        assert_eq!(s.fallen.len(), fallen_in_table);
        for f in &s.fallen {
            assert!(f.fallen_year >= f.founded_year);
            assert_eq!(f.lasted_years, f.fallen_year - f.founded_year);
            assert!(f.event.is_some(), "覆滅那則事件留在因果帳（重要度 4 不修剪）：{}", f.name);
        }
        assert_eq!(s.by_continent.iter().map(|c| c.2).sum::<usize>(), s.alive.len());
    }

    #[test]
    fn queries_do_not_change_the_world() {
        let sim = aged(20);
        let before = sim.hash();
        let w = sim.world();
        let _ = (world_stats(w), people_stats(w), sect_stats(w), karma_stats(w), recent_counts(w, RECENT_YEARS));
        assert_eq!(sim.hash(), before);
    }
}
