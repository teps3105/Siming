//! GDD 3.4「凡人、門人與出頭」、6.10 #93：門人這一層。
//!
//! 世間分三層：凡人（州郡人口）、門人（入了道而沒有出頭的人，每州五個數字）、有名之人（逐一模擬）。
//! 本模組只維護中間那一層，**不用任何亂數**，也**不寫回州郡**，因此不回頭影響有名之人的任何判定；
//! 魔道的消耗（每個魔道門人每年吃掉 [`ADEPT_DEMON_EAT`] 個凡人）只記在門人層自己的帳上。
//!
//! # 每年結算（6.3 步驟 7a，州郡的判定月、接在人口更新之後）
//!
//! 1. 人口比上次結算少：五門同比例減少（戰亂、饑荒、疫病、大劫都經由人口反映）。
//! 2. 入道率隨靈潮對數內插（[`enter_rate`]）；目標總數 = 人口 × 入道率。
//! 3. 五門各向「目標 × 該門權重」靠近 1/[`ADEPT_YEARS`]；權重 = 0.2 × 世局係數，五門正規化。
//! 4. 魔道消耗本州凡人（記帳，不扣人口）。
//! 5. 算出各門在本州的養分（凡人 ÷ 門人，相對 [`ADEPT_NOURISH_BASE`]，上限 1），存著供查詢。
//!
//! 世局係數取不到資料的項（寺院香火、書院、毀佛、正道宗門清剿）一律取 1；魔道消耗他門門人本步不做。

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::dao;
use crate::evolution::year_of;
use crate::model::*;
use crate::params::*;
use crate::people::sect_capacity;
use crate::schedule::{Domain, Round};
use crate::sim::World;
use crate::types::ProvinceId;

/// 門的數目。
pub const DOORS: usize = 5;
/// 門的名稱，依索引：道、佛、儒、魔、散修。
pub const DOOR_NAMES: [&str; DOORS] = ["道", "佛", "儒", "魔", "散修"];
pub const TAO: usize = 0;
pub const BUDDHA: usize = 1;
pub const RU: usize = 2;
pub const DEMON: usize = 3;
pub const FREE: usize = 4;

/// 一州的門人。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProvAdepts {
    /// 五門的門人數（道、佛、儒、魔、散修）。
    pub n: [f64; DOORS],
    /// 上次結算後的州郡人口：人口比它少時，門人同比例減少。
    pub prev_pop: f64,
    /// 被魔道消耗、還沒長回來的凡人數（只在門人層記帳，不從州郡人口扣；見 [`settle`]）。
    pub eaten: f64,
    /// 各門在本州的養分（0..1；無門人時為 1）。只存著供查詢，目前不接到任何判定。
    pub nourish: [f64; DOORS],
}

impl ProvAdepts {
    pub fn total(&self) -> f64 {
        self.n.iter().sum()
    }
}

/// 門人的世界狀態（進世界雜湊，附在最後）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AdeptState {
    /// 以州郡 ID 為索引。
    pub provinces: Vec<ProvAdepts>,
    /// 各門「出了超脫者或在世最高境界者」的加成到哪一年為止（開天以來的年數，不含）。
    pub boost_until: [u32; DOORS],
    /// 因果帳已掃到的事件（此 ID 之前都看過）。
    pub ledger_cursor: u32,
    /// 上一年在世最高境界者所屬的門（[`NO_DOOR`] 表示無人）。
    pub top_door: u8,
}

/// `AdeptState::top_door` 的空值。
pub const NO_DOOR: u8 = u8::MAX;

impl AdeptState {
    pub fn new(provinces: usize) -> Self {
        Self { provinces: vec![ProvAdepts { nourish: [1.0; DOORS], ..Default::default() }; provinces], top_door: NO_DOOR, ..Default::default() }
    }
}

/// 入道率：靈潮 1.0 為基準 0.3%，靈潮最高值（`EVO_TIDE_MAX`）1%，最低值（`EVO_TIDE_MIN`）0.03%，兩側各自對數內插。
pub fn enter_rate(tide: f32) -> f64 {
    let t = f64::from(tide.clamp(EVO_TIDE_MIN, EVO_TIDE_MAX));
    let (hi, lo) = (f64::from(EVO_TIDE_MAX), f64::from(EVO_TIDE_MIN));
    if t >= 1.0 {
        ADEPT_RATE_BASE * (ADEPT_RATE_MAX / ADEPT_RATE_BASE).powf((t - 1.0) / (hi - 1.0))
    } else {
        ADEPT_RATE_BASE * (ADEPT_RATE_MIN / ADEPT_RATE_BASE).powf((1.0 - t) / (1.0 - lo))
    }
}

/// 單項世局係數夾在 0.5–2。
fn coef(x: f64) -> f64 {
    x.clamp(ADEPT_COEF_MIN, ADEPT_COEF_MAX)
}

/// 人物屬於哪一門。
pub fn door_of(w: &World, p: &Person) -> Option<usize> {
    match p.role {
        Role::Monk => Some(BUDDHA),
        Role::Scholar => Some(RU),
        Role::Demonic => Some(DEMON),
        Role::Cultivator => match p.power.and_then(|id| w.powers.get(id as usize)).and_then(dao::sect_of) {
            Some(s) if s.evil => Some(DEMON),
            Some(_) => Some(TAO),
            None => Some(FREE),
        },
        _ => None,
    }
}

/// 人物的成就，跨門可比（3.1 宿慧的前世成就）：修士與魔修 境界 × 8、僧 果位 × 15、儒士 三不朽總和 / 10。
fn achievement(p: &Person) -> f64 {
    match p.path {
        PathState::Cultivator { realm, .. } => f64::from(realm) * 8.0,
        PathState::Monk { fruit, .. } => f64::from(fruit) * 15.0,
        PathState::Scholar { virtue, deed, word } => f64::from(virtue + deed + word) / 10.0,
        _ => 0.0,
    }
}

/// 每年一次、全世界一份：掃因果帳裡新出的超脫者與在世最高成就者，更新各門的加成期限。
/// 一門的加成在「出了超脫者」或「成為在世最高者」那一年起算 [`ADEPT_TOP_YEARS`] 年；一門持續坐在最高位時，加成不中斷。
pub fn refresh_world(w: &mut World) {
    let year = w.evolution.history_years + year_of(w.tick);
    let n = w.provinces.len();
    if w.adepts.provinces.len() != n {
        w.adepts.provinces.resize(n, ProvAdepts { nourish: [1.0; DOORS], ..Default::default() });
    }
    // 超脫者：Died／Ascended 事件的死因為飛升、涅槃、成聖。
    let end = w.ledger.events.len() as u32;
    let mut boosted = [false; DOORS];
    for i in w.adepts.ledger_cursor..end {
        let e = &w.ledger.events[i as usize];
        if let EventArgs::Death { cause: DeathCause::Ascended | DeathCause::Nirvana | DeathCause::Sagehood, .. } = e.args {
            if let Some(d) = e.persons.first().and_then(|&pid| w.persons.get(pid as usize)).and_then(|p| door_of(w, p)) {
                boosted[d] = true;
            }
        }
    }
    w.adepts.ledger_cursor = end;
    // 在世最高成就者（同值取人物 ID 小者）。
    let mut best: Option<(f64, PersonId, usize)> = None;
    for &pid in &w.alive_ids {
        let p = &w.persons[pid as usize];
        let Some(d) = door_of(w, p) else { continue };
        let a = achievement(p);
        if a > 0.0 && best.is_none_or(|(b, _, _)| a > b) {
            best = Some((a, pid, d));
        }
    }
    let top = best.map_or(NO_DOOR, |(_, _, d)| d as u8);
    if top != NO_DOOR {
        if top != w.adepts.top_door {
            boosted[top as usize] = true;
        } else {
            w.adepts.boost_until[top as usize] = w.adepts.boost_until[top as usize].max(year + 1);
        }
    }
    w.adepts.top_door = top;
    for (d, hit) in boosted.iter().enumerate() {
        if *hit {
            w.adepts.boost_until[d] = w.adepts.boost_until[d].max(year + ADEPT_TOP_YEARS);
        }
    }
}

/// 各州的鄰近資訊（一次掃描，供本輪各州共用）。
struct Context {
    year: u32,
    tide: f32,
    /// 以州 ID 為索引：本州在世有名之人的平均戾氣（無人為 None）。
    ferocity: Vec<Option<f64>>,
    /// 以州 ID 為索引：本州山門的正道宗門聲望合計、成員數、名額。
    sect_prestige: Vec<f64>,
    sect_members: Vec<f64>,
    sect_capacity: Vec<f64>,
    post_cataclysm: bool,
    decline: bool,
}

impl Context {
    fn build(w: &World) -> Self {
        let n = w.provinces.len();
        let year = year_of(w.tick);
        let mut sum = vec![0.0f64; n];
        let mut cnt = vec![0u32; n];
        for &pid in &w.alive_ids {
            let p = &w.persons[pid as usize];
            if let Some(k) = sum.get_mut(p.province as usize) {
                *k += f64::from(p.attrs.ferocity);
                cnt[p.province as usize] += 1;
            }
        }
        let ferocity = sum.iter().zip(&cnt).map(|(&s, &c)| (c > 0).then(|| s / f64::from(c))).collect();
        let (mut prestige, mut members, mut capacity) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        for pw in &w.powers {
            let Some(s) = dao::sect_of(pw) else { continue };
            let Some(i) = Some(pw.seat_province as usize).filter(|&i| i < n) else { continue };
            if pw.fallen.is_some() || s.evil {
                continue;
            }
            let pr = dao::effective_prestige(pw);
            prestige[i] += f64::from(pr);
            members[i] += pw.members.len() as f64;
            capacity[i] += sect_capacity(pr) as f64;
        }
        let post_cataclysm = w.calamity.history.last().is_some_and(|&(y, _)| year < y + CATA_DURATION_YEARS + ADEPT_POST_CATA_YEARS);
        Self {
            year: w.evolution.history_years + year,
            tide: w.evolution.tide,
            ferocity,
            sect_prestige: prestige,
            sect_members: members,
            sect_capacity: capacity,
            post_cataclysm,
            decline: w.evolution.aura == crate::evolution::AuraAge::Decline,
        }
    }
}

/// 一州五門的世局係數（各項相乘，每項 0.5–2；再乘上世界層級的加成）。
/// 取不到資料的項（寺院香火、書院、毀佛、正道宗門清剿）取 1。
pub fn door_coefs(w: &World, province: ProvinceId) -> [f64; DOORS] {
    coefs_in(w, &Context::build(w), province)
}

fn coefs_in(w: &World, cx: &Context, province: ProvinceId) -> [f64; DOORS] {
    let p = &w.provinces[province as usize];
    let pi = province as usize;
    // 戰亂度 0..1：殘破度，或所屬王朝正在交戰時至少 `ADEPT_TURMOIL_WAR`。
    let owner = w.dynasty.owner_of(province).and_then(|d| w.dynasty.get(d));
    let at_war = owner.is_some_and(|d| !d.wars.is_empty());
    let turmoil = f64::from(p.devastation).max(if at_war { ADEPT_TURMOIL_WAR } else { 0.0 }).clamp(0.0, 1.0);

    // 道：本州與陸鄰正道宗門的聲望、靈潮、名額。
    let near = std::iter::once(pi).chain(p.neighbors.iter().map(|&n| n as usize));
    let (mut pr, mut mem, mut cap) = (0.0, 0.0, 0.0);
    for i in near {
        pr += cx.sect_prestige.get(i).copied().unwrap_or(0.0);
        mem += cx.sect_members.get(i).copied().unwrap_or(0.0);
        cap += cx.sect_capacity.get(i).copied().unwrap_or(0.0);
    }
    let fill = if cap > 0.0 { (mem / cap).min(1.0) } else { 0.0 };
    let tao = coef(1.0 + pr / ADEPT_PRESTIGE_REF) * coef(f64::from(cx.tide)) * coef(1.0 - ADEPT_FULL_PULL * fill);

    // 佛：末法、戰亂、量劫剛過（寺院香火、毀佛取 1）。
    let buddha = coef(if cx.decline { ADEPT_DECLINE_BUDDHA } else { 1.0 })
        * coef(1.0 + 0.5 * turmoil)
        * coef(if cx.post_cataclysm { ADEPT_POST_CATA_BUDDHA } else { 1.0 });

    // 儒：王朝承平（有主且不在交戰，隨天命）、無主之地、戰亂（書院取 1）。
    let peace = match owner {
        Some(d) if !at_war => 1.0 + ADEPT_PEACE_RU * f64::from(d.mandate) / 100.0,
        Some(_) => 1.0,
        None => ADEPT_UNOWNED_RU,
    };
    let ru = coef(peace) * coef(1.0 - 0.5 * turmoil);

    // 魔：本州濁氣、戾氣、亂世（正道宗門清剿取 1；清氣盛時濁氣項低於 1）。
    let turbid = f64::from(crate::field::derive::turbid_of(p));
    let fer = cx.ferocity.get(pi).copied().flatten().map_or(1.0, |f| 1.0 + (f - ADEPT_FEROCITY_NEUTRAL) / ADEPT_FEROCITY_SPAN);
    let demon = coef(1.0 + ADEPT_TURBID_SLOPE * (turbid - ADEPT_TURBID_NEUTRAL)) * coef(fer) * coef(1.0 + turmoil);

    let mut c = [tao, buddha, ru, demon, 1.0];
    for (d, v) in c.iter_mut().enumerate() {
        if cx.year < w.adepts.boost_until[d] {
            *v *= ADEPT_TOP_BOOST;
        }
    }
    c
}

/// 五門的權重（0.2 × 世局係數，正規化後合計 1）。
fn weights(c: &[f64; DOORS]) -> [f64; DOORS] {
    let raw = c.map(|x| 0.2 * x);
    let s: f64 = raw.iter().sum();
    raw.map(|x| x / s)
}

fn nourish_of(p: &ProvAdepts, pop: f64) -> [f64; DOORS] {
    let civilians = (pop - p.total() - p.eaten).max(0.0);
    p.n.map(|n| if n <= 0.0 { 1.0 } else { (civilians / n / ADEPT_NOURISH_BASE).min(1.0) })
}

/// 開天：各州門人數直接設為目標值（人口 × 入道率）的五等分。人口為 0 時全為 0。
pub fn init(w: &mut World) {
    let rate = enter_rate(w.evolution.tide);
    w.adepts = AdeptState::new(w.provinces.len());
    for (i, p) in w.provinces.iter().enumerate() {
        let a = &mut w.adepts.provinces[i];
        a.n = [p.population.max(0.0) * rate / DOORS as f64; DOORS];
        a.prev_pop = p.population.max(0.0);
        a.nourish = nourish_of(a, p.population);
    }
}

/// 一整年對全部州郡結算（人口期、測試與工具用；人物期走 [`settle`] 分月）。
pub fn yearly(w: &mut World) {
    refresh_world(w);
    settle(w, Round::All);
}

/// 本輪輪到判定月的州郡結算一次（6.3 步驟 7a）。須在 [`refresh_world`] 之後（每年一次）。
pub fn settle(w: &mut World, round: Round) {
    let n = w.provinces.len();
    if w.adepts.provinces.len() != n {
        w.adepts.provinces.resize(n, ProvAdepts { nourish: [1.0; DOORS], ..Default::default() });
    }
    let seed = w.params.seed;
    let ids: Vec<usize> = (0..n).filter(|&i| round.takes(seed, Domain::Province, w.provinces[i].id as u32)).collect();
    if ids.is_empty() {
        return;
    }
    let cx = Context::build(w);
    let rate = enter_rate(cx.tide);
    for i in ids {
        if w.provinces[i].lost {
            continue;
        }
        let c = coefs_in(w, &cx, i as ProvinceId);
        let wt = weights(&c);
        let pop = w.provinces[i].population.max(0.0);
        let a = &mut w.adepts.provinces[i];
        // 人口驟減：門人同比例減少。
        if a.prev_pop > 0.0 && pop < a.prev_pop {
            let r = pop / a.prev_pop;
            a.n = a.n.map(|x| x * r);
            a.eaten *= r;
        }
        let target = pop * rate;
        for (x, share) in a.n.iter_mut().zip(wt) {
            *x = (*x + (target * share - *x) / ADEPT_YEARS).max(0.0);
        }
        // 門人總數不超過人口。
        let tot = a.total();
        if tot > pop {
            let r = if tot > 0.0 { pop / tot } else { 0.0 };
            a.n = a.n.map(|x| x * r);
        }
        // 魔道每年消耗本州凡人：記在 `eaten`，隨人口以 `POP_GROWTH` 的速率長回；人口驟減時同比例縮。
        // 不從 `Province::population` 扣：那會經饑荒、業氣、濁氣反過來改動有名之人的入道與統計（6.10 #93 的消耗之後再議）。
        a.eaten = a.eaten * (1.0 - POP_GROWTH) + a.n[DEMON] * ADEPT_DEMON_EAT;
        a.prev_pop = pop;
        a.nourish = nourish_of(a, pop);
    }
}

/// 全世界的門人統計。
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub population: f64,
    pub total: f64,
    pub by_door: [f64; DOORS],
    /// 各門的平均養分（以該門門人數加權）。
    pub nourish: [f64; DOORS],
}

impl Summary {
    /// 門人總數 ÷ 人口。
    pub fn rate(&self) -> f64 {
        if self.population > 0.0 {
            self.total / self.population
        } else {
            0.0
        }
    }

    /// 各門占門人總數的比。
    pub fn shares(&self) -> [f64; DOORS] {
        self.by_door.map(|x| if self.total > 0.0 { x / self.total } else { 0.0 })
    }

    /// 養分最低的門與其值（同值取索引小者；無門人的門不算）。
    pub fn lowest_nourish(&self) -> Option<(usize, f64)> {
        (0..DOORS).filter(|&d| self.by_door[d] > 0.0).map(|d| (d, self.nourish[d])).min_by(|a, b| a.1.total_cmp(&b.1))
    }
}

pub fn summary(w: &World) -> Summary {
    let mut by_door = [0.0; DOORS];
    let mut weighted = [0.0; DOORS];
    let mut population = 0.0;
    for (p, a) in w.provinces.iter().zip(&w.adepts.provinces) {
        population += p.population.max(0.0);
        for d in 0..DOORS {
            by_door[d] += a.n[d];
            weighted[d] += a.n[d] * a.nourish[d];
        }
    }
    let nourish = std::array::from_fn(|d| if by_door[d] > 0.0 { weighted[d] / by_door[d] } else { 1.0 });
    Summary { population, total: by_door.iter().sum(), by_door, nourish }
}
