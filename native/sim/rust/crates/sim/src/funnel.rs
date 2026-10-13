//! 成道漏斗統計：每一世在哪一關被淘汰。只讀世界狀態與事件帳，不改任何模擬狀態，也不進世界雜湊。
//!
//! 資料來源：
//! - 人物與魂的欄位（出生、死因、最終境界、屬性、壽元、歷世）永久保留，期末直接讀。
//! - 帳會修剪重要度 1 的舊事件（`ComeOfAge`、`Judged` 滿百年且當事人皆歿後清空參數），
//!   所以 [`Observer`] 在跑模擬時每月掃一次新事件，把十二歲的去向與六道判定記進自己的結構。
//! - 魂的宿慧、道緣在轉世時改寫，[`Observer`] 在人物落地後第一次掃描時拍下快照。
//!
//! 母體：出生不早於 [`Observer::start_tick`]、且已歿的每一世。期末仍在世者另列（右截斷）。
//! 輸出是 [`Report`]：一組 [`Table`]，可轉成中文文字表或 JSON。

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{json, Value};

use crate::calendar::DAYS_PER_YEAR;
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::{ProvinceId, Tick};

/// 境界名（索引 0..=8）。
pub const REALM_NAMES: [&str; 9] = ["煉氣", "築基", "金丹", "元嬰", "化神", "煉虛", "合體", "渡劫", "大乘"];

// ── 觀察者 ───────────────────────────────────────────────────

/// 十二歲入道時的去向（取第一次 `ComeOfAge`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// 道門入宗。
    Sect,
    /// 走道門但本州與陸鄰沒有宗門。
    RogueNoSect,
    /// 走道門但聲望最高的宗門已滿員。
    RogueFull,
    /// 其他身分（散修以外）。
    Other,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    role: Role,
    gate: Gate,
}

/// 人物落地時拍下的魂快照（轉世時才會改寫，事後讀不到這一世的值）。
#[derive(Debug, Clone, Copy)]
struct Snap {
    wisdom: f32,
}

/// 逐月掃新事件與新人物的觀察者。呼叫端每次 `Sim::step` 後呼叫 [`Observer::observe`]。
pub struct Observer {
    /// 母體起點：出生 tick 不早於它的人才算。
    pub start_tick: Tick,
    /// 觀察者建立時的 tick（開天與預跑完成處）。
    pub t0: Tick,
    events_seen: usize,
    persons_seen: usize,
    entries: HashMap<PersonId, Entry>,
    late_recruited: HashSet<PersonId>,
    kill_why: HashMap<PersonId, Why>,
    /// 每一世死後的第一次六道判定。
    judged: HashMap<PersonId, SixPath>,
    snaps: HashMap<PersonId, Snap>,
}

impl Observer {
    /// 自目前的世界建立；`warmup_years` 年之內出生者不計入母體。
    pub fn new(w: &World, warmup_years: u32) -> Self {
        Self {
            start_tick: w.tick + warmup_years * DAYS_PER_YEAR,
            t0: w.tick,
            events_seen: w.ledger.events.len(),
            persons_seen: w.persons.len(),
            entries: HashMap::new(),
            late_recruited: HashSet::new(),
            kill_why: HashMap::new(),
            judged: HashMap::new(),
            snaps: HashMap::new(),
        }
    }

    /// 掃描自上次以來新增的人物與事件。
    pub fn observe(&mut self, w: &World) {
        for pid in self.persons_seen..w.persons.len() {
            let p = &w.persons[pid];
            if let Some(s) = w.souls.get(p.soul as usize) {
                self.snaps.insert(pid as PersonId, Snap { wisdom: s.wisdom });
            }
        }
        self.persons_seen = w.persons.len();
        for id in self.events_seen..w.ledger.events.len() {
            self.on_event(w, &w.ledger.events[id]);
        }
        self.events_seen = w.ledger.events.len();
    }

    fn on_event(&mut self, w: &World, e: &Event) {
        match (e.kind, &e.args) {
            (EventKind::ComeOfAge, EventArgs::Role { role }) => {
                let Some(&pid) = e.persons.first() else { return };
                if self.entries.contains_key(&pid) {
                    self.late_recruited.insert(pid);
                    return;
                }
                let gate = self.gate_of(w, pid, *role);
                self.entries.insert(pid, Entry { role: *role, gate });
            }
            (EventKind::Killed, _) => {
                if let Some(&victim) = e.persons.get(1) {
                    self.kill_why.entry(victim).or_insert(e.why);
                }
            }
            (EventKind::Judged, EventArgs::Judged { path }) => {
                let Some(&sid) = e.souls.first() else { return };
                let Some(soul) = w.souls.get(sid as usize) else { return };
                // 這次判定屬於魂「在判定時已歿」的最近一世。
                let life = soul
                    .lives
                    .iter()
                    .rev()
                    .copied()
                    .find(|&pid| w.persons.get(pid as usize).and_then(|p| p.died).is_some_and(|d| d.0 <= e.tick));
                if let Some(pid) = life {
                    self.judged.entry(pid).or_insert(*path);
                }
            }
            _ => {}
        }
    }

    /// 道門入道者沒進宗門的原因。複製 `people::path` 的選宗規則（本州或陸鄰、聲望最高、未滿員）。
    fn gate_of(&self, w: &World, pid: PersonId, role: Role) -> Gate {
        let p = &w.persons[pid as usize];
        if role != Role::Cultivator {
            return Gate::Other;
        }
        if p.power.is_some() {
            return Gate::Sect;
        }
        match best_sect(w, p.home) {
            None => Gate::RogueNoSect,
            Some(_) => Gate::RogueFull,
        }
    }
}

/// 本州或陸鄰有山門的宗門中，聲望最高者（同值取 ID 小者）。
fn best_sect(w: &World, home: ProvinceId) -> Option<PowerId> {
    let nb: &[ProvinceId] = w.provinces.get(home as usize).map_or(&[], |p| p.neighbors.as_slice());
    let mut best: Option<(f32, PowerId)> = None;
    for p in &w.powers {
        if !matches!(p.kind, PowerKind::Sect(_)) || p.fallen.is_some() {
            continue;
        }
        if p.seat_province != home && !nb.contains(&p.seat_province) {
            continue;
        }
        if best.is_none_or(|(bp, _)| p.prestige > bp) {
            best = Some((p.prestige, p.id));
        }
    }
    best.map(|(_, id)| id)
}

// ── 表格 ─────────────────────────────────────────────────────

/// 表格的一格。
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    N(u64),
    /// 人數與分母；文字顯示百分比。
    P(u64, u64),
    /// 人數與分母；文字顯示「人數 (百分比)」。
    NP(u64, u64),
    F(f64),
}

fn ratio(n: u64, den: u64) -> f64 {
    if den == 0 {
        0.0
    } else {
        n as f64 / den as f64
    }
}

impl Cell {
    fn text(&self) -> String {
        match self {
            Cell::Text(s) => s.clone(),
            Cell::N(n) => n.to_string(),
            Cell::P(n, d) => {
                if *d == 0 {
                    "-".into()
                } else {
                    format!("{:.1}%", 100.0 * ratio(*n, *d))
                }
            }
            Cell::NP(n, d) => {
                if *d == 0 {
                    format!("{n}")
                } else {
                    format!("{n} ({:.1}%)", 100.0 * ratio(*n, *d))
                }
            }
            Cell::F(f) => format!("{f:.1}"),
        }
    }

    fn json(&self) -> Value {
        match self {
            Cell::Text(s) => json!(s),
            Cell::N(n) => json!(n),
            Cell::P(n, d) | Cell::NP(n, d) => json!({"n": n, "den": d, "pct": round4(ratio(*n, *d))}),
            Cell::F(f) => json!(round4(*f)),
        }
    }

    /// 該格的人數（`N`、`P`、`NP`）。
    pub fn count(&self) -> Option<u64> {
        match self {
            Cell::N(n) | Cell::P(n, _) | Cell::NP(n, _) => Some(*n),
            _ => None,
        }
    }
}

fn round4(f: f64) -> f64 {
    (f * 10000.0).round() / 10000.0
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub title: String,
    pub note: String,
    pub cols: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

impl Table {
    fn new(title: &str, note: &str, cols: &[&str]) -> Self {
        Self { title: title.into(), note: note.into(), cols: cols.iter().map(|s| s.to_string()).collect(), rows: Vec::new() }
    }

    fn push(&mut self, label: &str, cells: Vec<Cell>) {
        let mut row = vec![Cell::Text(label.into())];
        row.extend(cells);
        self.rows.push(row);
    }

    /// 某欄各列的人數和（欄名須存在）。
    pub fn col_sum(&self, col: &str) -> u64 {
        let k = self.cols.iter().position(|c| c == col).expect("欄名");
        self.rows.iter().filter_map(|r| r[k].count()).sum()
    }

    /// 以列名取得該列。
    pub fn row(&self, label: &str) -> Option<&Vec<Cell>> {
        self.rows.iter().find(|r| matches!(&r[0], Cell::Text(s) if s == label))
    }

    fn json(&self) -> Value {
        let rows: Vec<Value> = self
            .rows
            .iter()
            .map(|r| {
                let mut m = serde_json::Map::new();
                for (c, v) in self.cols.iter().zip(r) {
                    m.insert(c.clone(), v.json());
                }
                Value::Object(m)
            })
            .collect();
        json!({"title": self.title, "note": self.note, "cols": self.cols, "rows": rows})
    }

    fn render(&self) -> String {
        let mut grid: Vec<Vec<String>> = vec![self.cols.clone()];
        for r in &self.rows {
            grid.push(r.iter().map(Cell::text).collect());
        }
        let n = self.cols.len();
        let widths: Vec<usize> = (0..n).map(|k| grid.iter().map(|r| disp_width(&r[k])).max().unwrap_or(0)).collect();
        let mut out = format!("■ {}\n", self.title);
        if !self.note.is_empty() {
            out.push_str(&format!("  （{}）\n", self.note));
        }
        for (i, r) in grid.iter().enumerate() {
            let mut line = String::from("  ");
            for (k, cell) in r.iter().enumerate() {
                let pad = widths[k] - disp_width(cell);
                if k == 0 {
                    line.push_str(cell);
                    line.push_str(&" ".repeat(pad));
                } else {
                    line.push_str(&" ".repeat(pad));
                    line.push_str(cell);
                }
                line.push_str("  ");
            }
            out.push_str(line.trim_end());
            out.push('\n');
            if i == 0 {
                out.push_str(&format!("  {}\n", "─".repeat(widths.iter().map(|w| w + 2).sum::<usize>().min(160))));
            }
        }
        out
    }
}

/// 終端機顯示寬度：全形字算 2。
fn disp_width(s: &str) -> usize {
    s.chars().map(|c| if (c as u32) >= 0x2E80 || c == '■' || c == '─' { 2 } else { 1 }).sum()
}

/// 整份統計。
#[derive(Debug, Clone)]
pub struct Report {
    pub meta: Value,
    /// 母體：觀察期內結束的世數。
    pub ended: u64,
    /// 期末仍在世、出生於母體期間的人數。
    pub alive: u64,
    pub tables: Vec<Table>,
}

impl Report {
    pub fn table(&self, title: &str) -> &Table {
        self.tables.iter().find(|t| t.title == title).unwrap_or_else(|| panic!("沒有表：{title}"))
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        if let Value::Object(m) = &self.meta {
            for (k, v) in m {
                let s = match v {
                    Value::String(s) => s.clone(),
                    o => o.to_string(),
                };
                out.push_str(&format!("{k}：{s}\n"));
            }
        }
        out.push_str(&format!("母體（觀察期內結束的世）：{}　期末仍在世（出生於母體期間）：{}\n\n", self.ended, self.alive));
        for t in &self.tables {
            out.push_str(&t.render());
            out.push('\n');
        }
        out
    }

    pub fn to_json(&self) -> Value {
        json!({"meta": self.meta, "ended": self.ended, "alive": self.alive, "tables": self.tables.iter().map(Table::json).collect::<Vec<_>>()})
    }

    /// 各表總數的自我核對；回傳不一致的說明（空表示全部對得起來）。
    pub fn check(&self) -> Vec<String> {
        let mut bad = Vec::new();
        fn eq(bad: &mut Vec<String>, name: &str, got: u64, want: u64) {
            if got != want {
                bad.push(format!("{name}：{got} ≠ {want}"));
            }
        }
        eq(&mut bad, "路線人數和", self.table("入道路線").col_sum("人數"), self.ended);
        let cause_tbl = self.table("死因（依最高境界分層）");
        eq(
            &mut bad,
            "死因表人數和",
            cause_tbl.rows.iter().filter(|r| r[0].text() != "全部").filter_map(|r| r[1].count()).sum(),
            self.ended,
        );
        eq(&mut bad, "享年表（依路線）人數和", self.table("享年（依路線）").col_sum("世數"), self.ended);
        eq(&mut bad, "享年表（依境界）人數和", self.table("享年（依最高境界）").col_sum("世數"), self.ended);
        let cause = self.table("死因（依最高境界分層）");
        let n_col = cause.cols.iter().position(|c| c == "人數").unwrap_or(1);
        for r in &cause.rows {
            let sum: u64 = r.iter().enumerate().filter(|(k, _)| *k > n_col).filter_map(|(_, c)| c.count()).sum();
            let n = r[n_col].count().unwrap_or(0);
            if sum != n {
                bad.push(format!("死因列 {}：死因和 {sum} ≠ 人數 {n}", r[0].text()));
            }
        }
        eq(&mut bad, "轉世次數表人數和", self.table("轉世次數（這一世是魂的第幾世）").col_sum("世數"), self.ended);
        eq(
            &mut bad,
            "前世對照表人數和（互斥列）",
            self.table("前世境界對照")
                .rows
                .iter()
                .take(self.table("前世境界對照").rows.len().saturating_sub(2))
                .filter_map(|r| r[1].count())
                .sum(),
            self.ended,
        );
        eq(&mut bad, "六道去向人數和", self.table("六道去向").col_sum("世數"), self.ended);
        bad
    }
}

// ── 逐世整理 ─────────────────────────────────────────────────

const ROUTES: [&str; 10] = ["夭折（未滿十二歲）", "道門入宗", "散修", "僧", "儒", "魔", "散人", "武將候補", "其他職分", "十二歲後未及入道"];
const R_CHILD: usize = 0;
const R_SECT: usize = 1;
const R_ROGUE: usize = 2;
const R_MONK: usize = 3;
const R_SCHOLAR: usize = 4;
const R_DEMON: usize = 5;
const R_COMMONER: usize = 6;
const R_GENERAL: usize = 7;
const R_OTHER: usize = 8;
const R_LATE: usize = 9;

/// 最高境界分層：夭折、凡人、僧、儒、九境界、飛升。
const STRATA: [&str; 14] = ["夭折", "凡人", "僧", "儒", "煉氣", "築基", "金丹", "元嬰", "化神", "煉虛", "合體", "渡劫", "大乘", "飛升"];
const S_CHILD: usize = 0;
const S_MORTAL: usize = 1;
const S_MONK: usize = 2;
const S_SCHOLAR: usize = 3;
const S_REALM0: usize = 4;
const S_ASCEND: usize = 13;

const CAUSES: [&str; 15] = [
    "壽終",
    "突破走火",
    "天劫",
    "兵解",
    "宿緣仇殺",
    "斬妖除魔",
    "魔修襲殺",
    "宗門戰殺",
    "被殺其他",
    "戰爭",
    "量劫",
    "其他災禍",
    "飛升",
    "涅槃",
    "成聖",
];

fn cause_bucket(c: DeathCause, why: Option<Why>) -> usize {
    match c {
        DeathCause::Age => 0,
        DeathCause::BreakthroughFailed => 1,
        DeathCause::Tribulation => 2,
        DeathCause::ShedBody => 3,
        DeathCause::Killed => match why {
            Some(Why::Vendetta) => 4,
            Some(Why::SlayDemon) => 5,
            Some(Why::DemonAssassin) => 6,
            Some(Why::SectWarDeath) => 7,
            _ => 8,
        },
        DeathCause::War => 9,
        DeathCause::Kalpa => 10,
        DeathCause::Smite | DeathCause::Plague | DeathCause::Famine => 11,
        DeathCause::Ascended => 12,
        DeathCause::Nirvana => 13,
        DeathCause::Sagehood => 14,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prev {
    None,
    NonCultivator,
    Realm(u8),
}

struct Life {
    pid: PersonId,
    soul: SoulId,
    nth: usize,
    route: usize,
    stratum: usize,
    cause: usize,
    age: u32,
    lifespan: u32,
    entry: Option<Entry>,
    late: bool,
    realm: Option<u8>,
    root: f32,
    insight: f32,
    wisdom: f32,
    prev: Prev,
    prev_any_ge2: bool,
    judged: Option<SixPath>,
    gap_years: Option<f64>,
    way: Option<Liberation>,
}

impl Life {
    fn came(&self) -> bool {
        self.entry.is_some()
    }
    /// 十二歲走道門或魔（修士路線）。
    fn tao(&self) -> bool {
        self.entry.is_some_and(|e| matches!(e.role, Role::Cultivator | Role::Demonic))
    }
    /// 十二歲入了道、釋、儒、魔任一門。
    fn any(&self) -> bool {
        self.entry.is_some_and(|e| matches!(e.role, Role::Cultivator | Role::Demonic | Role::Monk | Role::Scholar))
    }
    fn reached(&self, r: u8) -> bool {
        self.realm.is_some_and(|x| x >= r)
    }
}

fn stratum_of(p: &Person, age: u32, cause: DeathCause) -> usize {
    if cause == DeathCause::Ascended {
        return S_ASCEND;
    }
    if age < COMING_OF_AGE_YEARS {
        return S_CHILD;
    }
    match p.path {
        PathState::Cultivator { realm, .. } => S_REALM0 + realm.min(8) as usize,
        PathState::Monk { .. } => S_MONK,
        PathState::Scholar { .. } => S_SCHOLAR,
        _ => S_MORTAL,
    }
}

fn live_stratum(p: &Person) -> usize {
    match p.path {
        PathState::Cultivator { realm, .. } => S_REALM0 + realm.min(8) as usize,
        PathState::Monk { .. } => S_MONK,
        PathState::Scholar { .. } => S_SCHOLAR,
        PathState::Child => S_CHILD,
        _ => S_MORTAL,
    }
}

fn person_realm(p: &Person) -> Option<u8> {
    match p.path {
        PathState::Cultivator { realm, .. } => Some(realm),
        _ => None,
    }
}

fn life_label(p: &Person) -> String {
    match p.path {
        PathState::Cultivator { realm, .. } => REALM_NAMES[realm.min(8) as usize].to_string(),
        PathState::Monk { .. } => "僧".into(),
        PathState::Scholar { .. } => "儒".into(),
        PathState::Child => "夭".into(),
        _ => "凡".into(),
    }
}

impl Observer {
    fn lives(&self, w: &World) -> Vec<Life> {
        let mut out = Vec::new();
        for p in &w.persons {
            let Some((died_tick, cause)) = p.died else { continue };
            if (p.born as i64) < self.start_tick as i64 {
                continue;
            }
            let soul = &w.souls[p.soul as usize];
            let nth = soul.lives.iter().position(|&x| x == p.id).unwrap_or(0);
            let age = p.age(w.tick);
            let entry = self.entries.get(&p.id).copied();
            let route = match entry {
                _ if age < COMING_OF_AGE_YEARS => R_CHILD,
                Some(e) => match e.role {
                    Role::Cultivator if e.gate == Gate::Sect => R_SECT,
                    Role::Cultivator => R_ROGUE,
                    Role::Monk => R_MONK,
                    Role::Scholar => R_SCHOLAR,
                    Role::Demonic => R_DEMON,
                    Role::Commoner => R_COMMONER,
                    Role::GeneralCandidate => R_GENERAL,
                    _ => R_OTHER,
                },
                None if p.role == Role::Child => R_LATE,
                None => R_OTHER,
            };
            let (prev, prev_any_ge2) = if nth == 0 {
                (Prev::None, false)
            } else {
                let prevs: Vec<&Person> = soul.lives[..nth].iter().filter_map(|&x| w.persons.get(x as usize)).collect();
                let last = prevs.last().map_or(Prev::None, |q| person_realm(q).map_or(Prev::NonCultivator, Prev::Realm));
                (last, prevs.iter().any(|q| person_realm(q).is_some_and(|r| r >= 2)))
            };
            let gap_years = soul
                .lives
                .get(nth + 1)
                .and_then(|&n| w.persons.get(n as usize))
                .map(|n| (n.born as i64 - died_tick as i64) as f64 / DAYS_PER_YEAR as f64);
            let way = match cause {
                DeathCause::Ascended => Some(Liberation::Ascension),
                DeathCause::Nirvana => Some(Liberation::Nirvana),
                DeathCause::Sagehood => Some(Liberation::Sagehood),
                _ => None,
            };
            out.push(Life {
                pid: p.id,
                soul: p.soul,
                nth,
                route,
                stratum: stratum_of(p, age, cause),
                cause: cause_bucket(cause, self.kill_why.get(&p.id).copied()),
                age,
                lifespan: p.lifespan as u32,
                entry,
                late: self.late_recruited.contains(&p.id),
                realm: person_realm(p),
                root: p.attrs.root,
                insight: p.attrs.insight,
                wisdom: self.snaps.get(&p.id).map_or(0.0, |s| s.wisdom),
                prev,
                prev_any_ge2,
                judged: self.judged.get(&p.id).copied(),
                gap_years,
                way,
            });
        }
        out
    }
}

// ── 統計小工具 ───────────────────────────────────────────────

#[derive(Default)]
struct Rates {
    ended: u64,
    came: u64,
    tao: u64,
    any: u64,
    r1: u64,
    r2: u64,
}

impl Rates {
    fn add(&mut self, l: &Life) {
        self.ended += 1;
        if l.came() {
            self.came += 1;
            self.tao += l.tao() as u64;
            self.any += l.any() as u64;
            self.r1 += l.reached(1) as u64;
            self.r2 += l.reached(2) as u64;
        }
    }

    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::N(self.ended),
            Cell::N(self.came),
            Cell::P(self.tao, self.came),
            Cell::P(self.any, self.came),
            Cell::P(self.r1, self.came),
            Cell::P(self.r2, self.came),
        ]
    }
}

const RATE_COLS: [&str; 7] = ["分組", "世數", "滿十二歲入道判定", "走修士路線率", "入任一門率", "走到築基率", "走到金丹率"];

fn rate_table(title: &str, note: &str, groups: Vec<(String, Rates)>) -> Table {
    let mut t = Table::new(title, note, &RATE_COLS);
    for (k, r) in groups {
        t.push(&k, r.cells());
    }
    t
}

/// 取排序後陣列的分位（最近排名）。
fn quantile(v: &[u32], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v[((v.len() - 1) as f64 * q).round() as usize] as f64
}

fn mean_u32(v: &[u32]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64
    }
}

fn mean_f(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn median_f(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn bin5(x: f32) -> usize {
    ((x / 2.0).floor() as i64).clamp(0, 4) as usize
}

const BIN5: [&str; 5] = ["0～2", "2～4", "4～6", "6～8", "8～10"];

fn path_name(p: SixPath) -> &'static str {
    match p {
        SixPath::Heaven => "天道",
        SixPath::Human => "人道",
        SixPath::Asura => "阿修羅道",
        SixPath::Beast => "畜生道",
        SixPath::Hungry => "餓鬼道",
        SixPath::Hell => "地獄道",
    }
}

const PATHS: [SixPath; 6] = [SixPath::Heaven, SixPath::Human, SixPath::Asura, SixPath::Beast, SixPath::Hungry, SixPath::Hell];

fn way_name(w: Liberation) -> &'static str {
    match w {
        Liberation::Ascension => "飛升",
        Liberation::Nirvana => "涅槃",
        Liberation::Sagehood => "成聖",
    }
}

// ── 報表 ─────────────────────────────────────────────────────

impl Observer {
    /// 產生整份報表。`meta` 為呼叫端附的說明（種子、尺寸、年數、耗時等）。
    pub fn report(&self, w: &World, meta: Value) -> Report {
        let lives = self.lives(w);
        let ended = lives.len() as u64;
        let alive_persons: Vec<&Person> = w.persons.iter().filter(|p| p.alive() && p.born as i64 >= self.start_tick as i64).collect();
        let mut tables = vec![
            self.t_route(&lives),
            self.t_gate(&lives),
            self.t_realm(&lives, &alive_persons),
            self.t_cause(&lives),
            self.t_cause_by_path(&lives),
            self.t_age_by(&lives, "享年（依路線）", &ROUTES, |l| l.route),
            self.t_age_by(&lives, "享年（依最高境界）", &STRATA, |l| l.stratum),
        ];
        tables.extend(self.t_cross(&lives, w));
        tables.extend(self.t_liberation(&lives, w));
        tables.extend(self.t_start(&lives));
        tables.extend(self.t_six_paths(&lives));
        Report { meta, ended, alive: alive_persons.len() as u64, tables }
    }

    fn t_route(&self, lives: &[Life]) -> Table {
        let mut t = Table::new(
            "入道路線",
            "十二歲入道判定的去向；夭折為未滿十二歲而歿；「後來被宗門收錄」指入道紀錄之外，宗門補足時又寫了一次入道（散人或幼年被收進門）",
            &["路線", "人數", "後來被宗門收錄"],
        );
        let total = lives.len() as u64;
        for (k, name) in ROUTES.iter().enumerate() {
            let n = lives.iter().filter(|l| l.route == k).count() as u64;
            let late = lives.iter().filter(|l| l.route == k && l.late).count() as u64;
            t.push(name, vec![Cell::NP(n, total), Cell::N(late)]);
        }
        t
    }

    fn t_gate(&self, lives: &[Life]) -> Table {
        let mut t = Table::new(
            "入道門檻與去向",
            "只計滿十二歲後有入道紀錄者；根骨 3 為道門門檻；原因由十二歲當下的屬性與世界狀態重建（非事件帳原有），宗門有無以掃描時為準",
            &["條件", "人數"],
        );
        let came: Vec<&Life> = lives.iter().filter(|l| l.came()).collect();
        let den = came.len() as u64;
        let cnt = |f: &dyn Fn(&Life, Entry) -> bool| came.iter().filter(|l| f(l, l.entry.unwrap())).count() as u64;
        let open = |l: &Life| l.root >= ENTRY_REQ_TAO_ROOT;
        t.push("根骨≥3：道門入宗", vec![Cell::NP(cnt(&|l, e| open(l) && e.role == Role::Cultivator && e.gate == Gate::Sect), den)]);
        t.push(
            "根骨≥3：成散修（本州與陸鄰沒有宗門）",
            vec![Cell::NP(cnt(&|l, e| open(l) && e.role == Role::Cultivator && e.gate == Gate::RogueNoSect), den)],
        );
        t.push(
            "根骨≥3：成散修（最高聲望宗門已滿員）",
            vec![Cell::NP(cnt(&|l, e| open(l) && e.role == Role::Cultivator && e.gate == Gate::RogueFull), den)],
        );
        t.push("根骨≥3：沒抽到道門，入了僧", vec![Cell::NP(cnt(&|l, e| open(l) && e.role == Role::Monk), den)]);
        t.push("根骨≥3：沒抽到道門，入了儒", vec![Cell::NP(cnt(&|l, e| open(l) && e.role == Role::Scholar), den)]);
        t.push("根骨≥3：沒抽到道門，入了魔", vec![Cell::NP(cnt(&|l, e| open(l) && e.role == Role::Demonic), den)]);
        t.push(
            "根骨≥3：仍是散人或武將候補（無門可入）",
            vec![Cell::NP(cnt(&|l, e| open(l) && matches!(e.role, Role::Commoner | Role::GeneralCandidate)), den)],
        );
        t.push("根骨<3（根骨不足）：入僧", vec![Cell::NP(cnt(&|l, e| !open(l) && e.role == Role::Monk), den)]);
        t.push("根骨<3（根骨不足）：入儒", vec![Cell::NP(cnt(&|l, e| !open(l) && e.role == Role::Scholar), den)]);
        t.push("根骨<3（根骨不足）：入魔", vec![Cell::NP(cnt(&|l, e| !open(l) && e.role == Role::Demonic), den)]);
        t.push(
            "根骨<3（根骨不足）：散人或武將候補",
            vec![Cell::NP(cnt(&|l, e| !open(l) && matches!(e.role, Role::Commoner | Role::GeneralCandidate)), den)],
        );
        t.push("根骨<3（根骨不足）：被宗門補足收錄為修士", vec![Cell::NP(cnt(&|l, e| !open(l) && e.role == Role::Cultivator), den)]);
        let listed: u64 = t.rows.iter().filter_map(|r| r[1].count()).sum();
        t.push("其他（不在上列）", vec![Cell::NP(den - listed, den)]);
        t.push("合計", vec![Cell::NP(den, den)]);
        t
    }

    fn t_realm(&self, lives: &[Life], alive: &[&Person]) -> Table {
        let mut t = Table::new(
            "境界漏斗",
            "母體為走修士路線（路徑為煉氣系）的已歿世；到過=最高境界不低於；晉級率=到過下一境者／到過本境者；大乘的下一境是飛升；期末在世為右截斷，不在母體內",
            &["境界", "到過", "占修士", "死在這一境", "晉級到下一境", "晉級率", "期末仍在世（未計）"],
        );
        let cult: Vec<&Life> = lives.iter().filter(|l| l.realm.is_some()).collect();
        let total = cult.len() as u64;
        let reached = |r: usize| cult.iter().filter(|l| l.realm.unwrap() as usize >= r).count() as u64;
        let ascended = cult.iter().filter(|l| l.stratum == S_ASCEND).count() as u64;
        for (r, name) in REALM_NAMES.iter().enumerate() {
            let here = reached(r);
            let next = if r == 8 { ascended } else { reached(r + 1) };
            let died_here = cult.iter().filter(|l| l.realm.unwrap() as usize == r && l.stratum != S_ASCEND).count() as u64;
            let still = alive.iter().filter(|p| person_realm(p) == Some(r as u8)).count() as u64;
            t.push(name, vec![Cell::N(here), Cell::P(here, total), Cell::N(died_here), Cell::N(next), Cell::P(next, here), Cell::N(still)]);
        }
        t.push("飛升", vec![Cell::N(ascended), Cell::P(ascended, total), Cell::N(0), Cell::N(0), Cell::P(0, 0), Cell::N(0)]);
        t
    }

    fn cause_row(lives: &[&Life]) -> Vec<Cell> {
        let n = lives.len() as u64;
        let mut v = vec![Cell::N(n)];
        for c in 0..CAUSES.len() {
            v.push(Cell::P(lives.iter().filter(|l| l.cause == c).count() as u64, n));
        }
        v
    }

    fn t_cause(&self, lives: &[Life]) -> Table {
        let mut cols = vec!["最高境界", "人數"];
        cols.extend(CAUSES);
        let mut t = Table::new(
            "死因（依最高境界分層）",
            "斬妖除魔、魔修襲殺、仇殺、宗門戰殺為「被殺」依成因細分；戰爭指戰場陣亡；僧、儒、凡人各列一層；夭折為未滿十二歲",
            &cols,
        );
        for (k, name) in STRATA.iter().enumerate() {
            let sel: Vec<&Life> = lives.iter().filter(|l| l.stratum == k).collect();
            t.push(name, Self::cause_row(&sel));
        }
        let all: Vec<&Life> = lives.iter().collect();
        t.push("全部", Self::cause_row(&all));
        t
    }

    fn t_cause_by_path(&self, lives: &[Life]) -> Table {
        let mut cols = vec!["入道路線", "人數"];
        cols.extend(CAUSES);
        let mut t = Table::new("死因（依入道路線）", "路線同「入道路線」表", &cols);
        for (k, name) in ROUTES.iter().enumerate() {
            let sel: Vec<&Life> = lives.iter().filter(|l| l.route == k).collect();
            t.push(name, Self::cause_row(&sel));
        }
        t
    }

    fn t_age_by(&self, lives: &[Life], title: &str, names: &[&str], key: impl Fn(&Life) -> usize) -> Table {
        let mut t = Table::new(
            title,
            "享年以整歲計；壽元上限取歿時的值；差=壽元上限−享年；未活滿壽元=享年小於壽元上限者占比",
            &[
                "分組",
                "世數",
                "平均",
                "中位",
                "P10",
                "P20",
                "P30",
                "P40",
                "P60",
                "P70",
                "P80",
                "P90",
                "平均壽元上限",
                "平均差",
                "未活滿壽元",
            ],
        );
        for (k, name) in names.iter().enumerate() {
            let sel: Vec<&Life> = lives.iter().filter(|l| key(l) == k).collect();
            let mut ages: Vec<u32> = sel.iter().map(|l| l.age).collect();
            ages.sort_unstable();
            let n = sel.len() as u64;
            let spans: Vec<u32> = sel.iter().map(|l| l.lifespan).collect();
            let gaps: Vec<f64> = sel.iter().map(|l| l.lifespan as f64 - l.age as f64).collect();
            let short = sel.iter().filter(|l| l.age < l.lifespan).count() as u64;
            let mut cells = vec![Cell::N(n), Cell::F(mean_u32(&ages)), Cell::F(quantile(&ages, 0.5))];
            for q in [0.1, 0.2, 0.3, 0.4, 0.6, 0.7, 0.8, 0.9] {
                cells.push(Cell::F(quantile(&ages, q)));
            }
            cells.push(Cell::F(mean_u32(&spans)));
            cells.push(Cell::F(mean_f(&gaps)));
            cells.push(Cell::P(short, n));
            t.push(name, cells);
        }
        t.cols[1] = "世數".into();
        t
    }

    fn t_cross(&self, lives: &[Life], w: &World) -> Vec<Table> {
        let mut out = Vec::new();
        // 轉世次數（這一世是第幾世）。
        let mut t = Table::new("轉世次數（這一世是魂的第幾世）", "第 1 世=新魂首世", &["第幾世", "世數"]);
        let total = lives.len() as u64;
        let nth_bins: [(&str, usize, usize); 9] = [
            ("第 1 世", 0, 0),
            ("第 2 世", 1, 1),
            ("第 3 世", 2, 2),
            ("第 4 世", 3, 3),
            ("第 5 世", 4, 4),
            ("第 6～10 世", 5, 9),
            ("第 11～20 世", 10, 19),
            ("第 21～50 世", 20, 49),
            ("第 51 世以上", 50, usize::MAX),
        ];
        for (name, lo, hi) in nth_bins {
            let n = lives.iter().filter(|l| (lo..=hi).contains(&l.nth)).count() as u64;
            t.push(name, vec![Cell::NP(n, total)]);
        }
        out.push(t);
        // 魂的轉世次數（期末）。
        let mut souls: Vec<SoulId> = lives.iter().map(|l| l.soul).collect();
        souls.sort_unstable();
        souls.dedup();
        let mut t = Table::new("魂的轉世次數（期末）", "母體內出現過的魂，期末已轉世幾次（含母體外的世）", &["轉世次數", "魂數"]);
        let sn = souls.len() as u64;
        let bins: [(&str, u16, u16); 9] = [
            ("0 次", 0, 0),
            ("1 次", 1, 1),
            ("2 次", 2, 2),
            ("3 次", 3, 3),
            ("4 次", 4, 4),
            ("5～9 次", 5, 9),
            ("10～19 次", 10, 19),
            ("20～49 次", 20, 49),
            ("50 次以上", 50, u16::MAX),
        ];
        for (name, lo, hi) in bins {
            let n = souls.iter().filter(|&&s| (lo..=hi).contains(&w.souls[s as usize].rebirths())).count() as u64;
            t.push(name, vec![Cell::NP(n, sn)]);
        }
        out.push(t);
        // 前世境界對照。
        let mut groups: Vec<(String, Rates)> = Vec::new();
        let mut mk = |name: &str, f: &dyn Fn(&Life) -> bool| {
            let mut r = Rates::default();
            for l in lives.iter().filter(|l| f(l)) {
                r.add(l);
            }
            groups.push((name.to_string(), r));
        };
        mk("初世（沒有前世）", &|l| l.prev == Prev::None);
        mk("前一世非修士（凡、僧、儒、夭折）", &|l| l.prev == Prev::NonCultivator);
        for r in 0..9u8 {
            mk(&format!("前一世最高 {}", REALM_NAMES[r as usize]), &move |l| l.prev == Prev::Realm(r));
        }
        mk("【對照】歷世中到過金丹以上", &|l| l.prev_any_ge2);
        mk("【對照】歷世都沒到過金丹（含初世）", &|l| !l.prev_any_ge2);
        out.push(rate_table(
            "前世境界對照",
            "前一世=魂的上一個肉身；入道率等分母為「滿十二歲入道判定」者；最後兩列為對照，與上面各列重疊",
            groups,
        ));
        // 宿慧。
        let bins: [(&str, f32, f32); 6] = [
            ("0（無）", 0.0, 0.0001),
            ("0～5", 0.0001, 5.0),
            ("5～10", 5.0, 10.0),
            ("10～20", 10.0, 20.0),
            ("20～40", 20.0, 40.0),
            ("40 以上", 40.0, f32::MAX),
        ];
        let mut groups = Vec::new();
        for (name, lo, hi) in bins {
            let mut r = Rates::default();
            for l in lives.iter().filter(|l| l.wisdom >= lo && l.wisdom < hi) {
                r.add(l);
            }
            groups.push((name.to_string(), r));
        }
        out.push(rate_table("宿慧分布", "宿慧 W 為這一世出生時的值（落地後第一次掃描的快照）", groups));
        out
    }

    fn t_liberation(&self, lives: &[Life], w: &World) -> Vec<Table> {
        let mut out = Vec::new();
        let in_window = |p: &Person| p.died.is_some_and(|d| d.0 >= self.start_tick);
        let way_of = |p: &Person| match p.died {
            Some((_, DeathCause::Ascended)) => Some(Liberation::Ascension),
            Some((_, DeathCause::Nirvana)) => Some(Liberation::Nirvana),
            Some((_, DeathCause::Sagehood)) => Some(Liberation::Sagehood),
            _ => None,
        };
        let mut t =
            Table::new("超脫次數", "觀察期內（母體起點之後）歿於超脫者；出生於母體內者另列", &["方式", "觀察期內", "其中出生於母體內"]);
        for way in [Liberation::Ascension, Liberation::Nirvana, Liberation::Sagehood] {
            let all = w.persons.iter().filter(|p| in_window(p) && way_of(p) == Some(way)).count() as u64;
            let pop = lives.iter().filter(|l| l.way == Some(way)).count() as u64;
            t.push(way_name(way), vec![Cell::N(all), Cell::N(pop)]);
        }
        out.push(t);
        let mut t = Table::new(
            "超脫者的歷世",
            "第幾世與前世最高境界序列（凡=凡人、夭=夭折、僧、儒）；含觀察期內全部超脫者",
            &["魂", "方式", "第幾世", "享年", "此世路線", "歷世（舊→新）"],
        );
        let mut list: Vec<&Person> = w.persons.iter().filter(|p| in_window(p) && way_of(p).is_some()).collect();
        list.sort_by_key(|p| p.died.map(|d| d.0));
        for p in list {
            let soul = &w.souls[p.soul as usize];
            let nth = soul.lives.iter().position(|&x| x == p.id).unwrap_or(0);
            let seq: Vec<String> = soul.lives[..=nth].iter().filter_map(|&x| w.persons.get(x as usize)).map(life_label).collect();
            let route = lives.iter().find(|l| l.pid == p.id).map_or("（母體外）", |l| ROUTES[l.route]);
            t.push(
                &format!("魂{}", p.soul),
                vec![
                    Cell::Text(way_name(way_of(p).unwrap()).into()),
                    Cell::N(nth as u64 + 1),
                    Cell::N(p.age(w.tick) as u64),
                    Cell::Text(route.into()),
                    Cell::Text(seq.join("→")),
                ],
            );
        }
        out.push(t);
        out
    }

    fn t_start(&self, lives: &[Life]) -> Vec<Table> {
        let mut out = Vec::new();
        for (title, f) in [("起點：根骨", (|l: &Life| l.root) as fn(&Life) -> f32), ("起點：悟性", |l: &Life| l.insight)] {
            let mut groups = Vec::new();
            for (k, name) in BIN5.iter().enumerate() {
                let mut r = Rates::default();
                for l in lives.iter().filter(|l| bin5(f(l)) == k) {
                    r.add(l);
                }
                groups.push((name.to_string(), r));
            }
            out.push(rate_table(title, "五檔等寬；走到築基、金丹以最終境界計，分母為滿十二歲入道判定者", groups));
        }
        out
    }

    fn t_six_paths(&self, lives: &[Life]) -> Vec<Table> {
        let mut out = Vec::new();
        let mortal: Vec<&Life> = lives.iter().filter(|l| l.way.is_none()).collect();
        let mut t = Table::new(
            "六道去向",
            "每一世歿後的第一次六道判定（超脫者不經六道）；間隔=下一世出生−本世歿，含中陰與等待；下一世未出生者不計間隔；世數欄含超脫者",
            &["去向", "世數", "占比", "已轉世者數", "平均間隔（年）", "中位間隔（年）"],
        );
        let ended = lives.len() as u64;
        let lib = lives.len() as u64 - mortal.len() as u64;
        for p in PATHS {
            let sel: Vec<&&Life> = mortal.iter().filter(|l| l.judged == Some(p)).collect();
            let mut gaps: Vec<f64> = sel.iter().filter_map(|l| l.gap_years).collect();
            t.push(
                path_name(p),
                vec![
                    Cell::N(sel.len() as u64),
                    Cell::P(sel.len() as u64, ended),
                    Cell::N(gaps.len() as u64),
                    Cell::F(mean_f(&gaps)),
                    Cell::F(median_f(&mut gaps)),
                ],
            );
        }
        let unj = mortal.iter().filter(|l| l.judged.is_none()).count() as u64;
        t.push("尚未判定（歿後不久，或未見判定事件）", vec![Cell::N(unj), Cell::P(unj, ended), Cell::N(0), Cell::F(0.0), Cell::F(0.0)]);
        t.push("超脫（不入六道）", vec![Cell::N(lib), Cell::P(lib, ended), Cell::N(0), Cell::F(0.0), Cell::F(0.0)]);
        out.push(t);
        // 最高境界 × 去向。
        let mut cols = vec!["最高境界", "世數"];
        let names: Vec<&str> = PATHS.iter().map(|&p| path_name(p)).collect();
        cols.extend(names);
        let mut t = Table::new("六道去向（依最高境界分層）", "超脫者與尚未判定者不列入各道欄，列總數含之", &cols);
        for (k, name) in STRATA.iter().enumerate() {
            let sel: Vec<&Life> = lives.iter().filter(|l| l.stratum == k).collect();
            let n = sel.len() as u64;
            let mut cells = vec![Cell::N(n)];
            for p in PATHS {
                cells.push(Cell::P(sel.iter().filter(|l| l.judged == Some(p) && l.way.is_none()).count() as u64, n));
            }
            t.push(name, cells);
        }
        out.push(t);
        out
    }
}

/// 期末仍在世者（出生於母體期間）按目前層別的人數，供測試核對。
pub fn alive_by_stratum(obs: &Observer, w: &World) -> BTreeMap<&'static str, u64> {
    let mut m = BTreeMap::new();
    for p in w.persons.iter().filter(|p| p.alive() && p.born as i64 >= obs.start_tick as i64) {
        *m.entry(STRATA[live_stratum(p)]).or_insert(0) += 1;
    }
    m
}
