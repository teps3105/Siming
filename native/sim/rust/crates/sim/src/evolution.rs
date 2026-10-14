//! GDD 2.8 天地演變：慢變量、每世結算、世界大事與改元。
//!
//! 全球的三個慢變量——寒暖、海平面偏移、靈潮——每世（30 年）更新一次；寒暖與靈潮跨過門檻就是
//! 世界大事，並依 GDD 2.6 開啟新紀元。靈潮是格點靈氣的倍率，並使洞天福地升降級；
//! 寒暖與海平面改地形與海陸（[`crate::landform`]）；人口改草木（[`crate::flora`]）。
//!
//! 開局不是開天第一年：世界在開局前已有一段歷史（[`Evolution::history_years`]），那段歷史只推演
//! 這裡的慢變量、紀元年表與洞天福地的升降，不推演人間。週期的長度與相位、洞天福地每世的升降
//! 都由世界種子算出，不消耗隨機流；
//! 歷史的長度與靈潮在開局時的位置由開局時代決定。

use serde::{Deserialize, Serialize};

use crate::calendar::Era as CalendarEra;
use crate::era::{EraKind, Lexicon};
use crate::ledger::NewEvent;
use crate::model::{EventArgs, EventKind};
use crate::params::*;
use crate::sim::World;
use crate::types::{Era, LeyNode, NodeId, Tick};
use crate::worldgen::grid::{sin, Sphere, TAU};
use crate::worldgen::ley;

/// 寒暖所在的時期（GDD 2.8「世界大事與紀元」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Climate {
    /// 大寒。
    Cold,
    Mild,
    /// 大暖（海進）。
    Warm,
}

/// 靈潮所在的時期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuraAge {
    /// 末法。
    Decline,
    Plain,
    /// 靈氣復甦。
    Revival,
}

/// 一次世界大事：某個時期開始或結束。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldChange {
    ColdBegan,
    ColdEnded,
    WarmBegan,
    WarmEnded,
    DeclineBegan,
    DeclineEnded,
    RevivalBegan,
    RevivalEnded,
    CataclysmBegan,
    CataclysmEnded,
    /// 天地回到寒暖平常、靈潮平常：承平。紀元因過了最短年數才開啟時，天地的狀態早已不是剛跨門檻的那一刻，
    /// 用它記「進入承平」。變體的順序進世界雜湊與存檔，只在末尾追加。
    Calm,
}

impl WorldChange {
    /// 開啟 `kind` 類紀元的大事（紀元因最短年數延後開啟、當世沒有對應的跨門檻大事時，用它記這次改元）。
    fn starting(kind: EraKind) -> Self {
        match kind {
            EraKind::Cold => WorldChange::ColdBegan,
            EraKind::Warm => WorldChange::WarmBegan,
            EraKind::Decline => WorldChange::DeclineBegan,
            EraKind::Revival => WorldChange::RevivalBegan,
            EraKind::Cataclysm => WorldChange::CataclysmBegan,
            _ => WorldChange::Calm,
        }
    }

    /// 這件大事開啟哪一類紀元；時期結束的不指定。
    fn opens(self) -> Option<EraKind> {
        match self {
            WorldChange::ColdBegan => Some(EraKind::Cold),
            WorldChange::WarmBegan => Some(EraKind::Warm),
            WorldChange::DeclineBegan => Some(EraKind::Decline),
            WorldChange::RevivalBegan => Some(EraKind::Revival),
            WorldChange::CataclysmBegan => Some(EraKind::Cataclysm),
            _ => None,
        }
    }
}

/// 天地的慢變量、它們的週期與目前所在的時期（GDD 2.8）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evolution {
    /// 開局前已過的年數（30 的倍數）。開天以來的年數 = 這個數 + 開局後的年數。
    pub history_years: u32,
    /// 全球寒暖：−1（極寒）..1（極暖）。
    pub warmth: f32,
    /// 海平面的偏移（高度單位）。
    pub sea_shift: f32,
    /// 靈潮：天地靈氣的倍率。
    pub tide: f32,
    /// 開局那一年的寒暖與海平面偏移：生成出來的地形就是那一年的地形，之後以它們為零點算變化量。
    pub opening_warmth: f32,
    pub opening_sea_shift: f32,
    /// 寒暖兩個週期的長度（年）。
    pub warmth_periods: [u32; 2],
    /// 寒暖兩個週期在開天時的相位（0..1，一整圈為 1）。
    pub warmth_phases: [f32; 2],
    /// 靈潮週期的長度（年）。
    pub tide_period: u32,
    /// 靈潮週期在開天時的相位（0..1）。
    pub tide_phase: f32,
    pub climate: Climate,
    pub aura: AuraAge,
    /// 目前的天地狀態（[`Evolution::state_kind`]）從哪一年起沒有變過；改元要等狀態穩定 [`EVO_ERA_SETTLE_YEARS`] 年。
    #[serde(default)]
    pub kind_since: u32,
}

impl Evolution {
    /// 開天第 0 年的天地：週期由種子定；歷史的長度與靈潮在開局時的位置由開局時代定。
    fn at_genesis(seed: u64, era: Era, open_years: u32) -> Self {
        let pick = |tag: u64, lo: u32, hi: u32| lo + (mix(seed ^ (tag << 48)) % u64::from(hi - lo + 1)) as u32;
        let phase = |tag: u64| (mix(seed ^ (tag << 48)) >> 40) as f32 / (1u64 << 24) as f32;
        let opening_turn = match era {
            Era::Ancient => EVO_TIDE_TURN_ANCIENT,
            Era::Middle => EVO_TIDE_TURN_MIDDLE,
            Era::Decline => EVO_TIDE_TURN_DECLINE,
        };
        // 開天即第 0 年，沒有開局前的歷史；`open_years` 是預跑的總年數，相位據此定在接手那一年的靈潮位置。
        let history_years = 0;
        let tide_period = pick(5, EVO_TIDE_PERIOD_MIN, EVO_TIDE_PERIOD_MAX);
        // 相位取使開局那一年的靈潮正好落在開局時代指定的位置。
        let elapsed = (open_years % tide_period) as f32 / tide_period as f32;
        let tide_phase = opening_turn - elapsed;
        let mut out = Self {
            history_years,
            warmth: 0.0,
            sea_shift: 0.0,
            tide: 1.0,
            opening_warmth: 0.0,
            opening_sea_shift: 0.0,
            warmth_periods: [pick(1, EVO_WARMTH_P1_MIN, EVO_WARMTH_P1_MAX), pick(2, EVO_WARMTH_P2_MIN, EVO_WARMTH_P2_MAX)],
            warmth_phases: [phase(3), phase(4)],
            tide_period,
            tide_phase: tide_phase - tide_phase.floor(),
            climate: Climate::Mild,
            aura: AuraAge::Plain,
            kind_since: 0,
        };
        out.warmth = out.warmth_cycle(0);
        out.tide = out.tide_cycle(0);
        out
    }

    /// 寒暖的週期值：兩個長週期疊加，−1..1。
    pub fn warmth_cycle(&self, year: u32) -> f32 {
        EVO_WARMTH_W1 * wave(year, self.warmth_periods[0], self.warmth_phases[0])
            + EVO_WARMTH_W2 * wave(year, self.warmth_periods[1], self.warmth_phases[1])
    }

    /// 寒暖相對於開局那一年的變化量。
    pub fn warmth_delta(&self) -> f32 {
        self.warmth - self.opening_warmth
    }

    /// 海平面相對於開局那一年的變化量（高度單位）。
    pub fn sea_delta(&self) -> f32 {
        self.sea_shift - self.opening_sea_shift
    }

    /// 靈潮的週期值：以 1 為中心上下起伏。
    pub fn tide_cycle(&self, year: u32) -> f32 {
        1.0 + EVO_TIDE_AMPLITUDE * wave(year, self.tide_period, self.tide_phase)
    }

    /// 第 `year` 年結算之後的寒暖與靈潮（畫面的走向箭頭用）；不改動自己。
    pub fn forecast(&self, year: u32) -> (f32, f32) {
        let mut ahead = self.clone();
        ahead.settle(year);
        (ahead.warmth, ahead.tide)
    }

    /// 每世結算一次（`year` 為開天以來的年數）：寒暖與靈潮取週期值，海平面追寒暖；
    /// 回傳這一世跨過門檻的世界大事，先寒暖後靈潮。
    pub fn settle(&mut self, year: u32) -> Vec<WorldChange> {
        self.warmth = self.warmth_cycle(year).clamp(-1.0, 1.0);
        let sea_target = EVO_SEA_RANGE * self.warmth;
        self.sea_shift = (self.sea_shift + (sea_target - self.sea_shift) * EVO_SEA_APPROACH).clamp(-EVO_SEA_RANGE, EVO_SEA_RANGE);
        self.tide = self.tide_cycle(year).clamp(EVO_TIDE_MIN, EVO_TIDE_MAX);

        let before = self.state_kind();
        let mut changes = Vec::new();
        // 進入與離開用不同的門檻，免得在門檻附近反覆。
        match self.climate {
            Climate::Mild if self.warmth >= EVO_WARMTH_ENTER => {
                self.climate = Climate::Warm;
                changes.push(WorldChange::WarmBegan);
            }
            Climate::Mild if self.warmth <= -EVO_WARMTH_ENTER => {
                self.climate = Climate::Cold;
                changes.push(WorldChange::ColdBegan);
            }
            Climate::Warm if self.warmth <= EVO_WARMTH_LEAVE => {
                self.climate = Climate::Mild;
                changes.push(WorldChange::WarmEnded);
            }
            Climate::Cold if self.warmth >= -EVO_WARMTH_LEAVE => {
                self.climate = Climate::Mild;
                changes.push(WorldChange::ColdEnded);
            }
            _ => {}
        }
        match self.aura {
            AuraAge::Plain if self.tide <= EVO_DECLINE_ENTER => {
                self.aura = AuraAge::Decline;
                changes.push(WorldChange::DeclineBegan);
            }
            AuraAge::Plain if self.tide >= EVO_REVIVAL_ENTER => {
                self.aura = AuraAge::Revival;
                changes.push(WorldChange::RevivalBegan);
            }
            AuraAge::Decline if self.tide >= EVO_DECLINE_LEAVE => {
                self.aura = AuraAge::Plain;
                changes.push(WorldChange::DeclineEnded);
            }
            AuraAge::Revival if self.tide <= EVO_REVIVAL_LEAVE => {
                self.aura = AuraAge::Plain;
                changes.push(WorldChange::RevivalEnded);
            }
            _ => {}
        }
        if self.state_kind() != before {
            self.kind_since = year;
        }
        changes
    }

    /// 天地現在的狀態屬於哪一類紀元：寒暖不平常取大寒、大暖，否則靈潮不平常取末法、靈氣復甦，都平常則承平。
    pub fn state_kind(&self) -> EraKind {
        match (self.climate, self.aura) {
            (Climate::Cold, _) => EraKind::Cold,
            (Climate::Warm, _) => EraKind::Warm,
            (_, AuraAge::Decline) => EraKind::Decline,
            (_, AuraAge::Revival) => EraKind::Revival,
            _ => EraKind::Peace,
        }
    }

    /// 這一世是否改元（GDD 2.8）：距目前紀元開始已滿 [`EVO_ERA_MIN_YEARS`] 年，天地現在的狀態
    /// （[`Evolution::state_kind`]）已穩定 [`EVO_ERA_SETTLE_YEARS`] 年，且與目前的紀元不同類，就改成現在狀態的那一類。
    /// 不在大事跨門檻的當下改元，所以紀元名取決於改元時天地的樣子，而不是早已過去的那件大事。
    pub fn next_era(&self, year: u32, current: &CalendarEra) -> Option<EraKind> {
        let kind = self.state_kind();
        // 新紀元的元年是下一年。
        let long_enough = year + 1 >= current.start_year + EVO_ERA_MIN_YEARS;
        let settled = year >= self.kind_since + EVO_ERA_SETTLE_YEARS;
        (long_enough && settled && kind != current.kind).then_some(kind)
    }
}

/// 開局時的天地與紀元年表：自開天第 0 年起，每世結算一次，推演到開局那一年（GDD 2.8）。
pub fn genesis(seed: u64, era: Era, open_years: u32) -> (Evolution, Vec<CalendarEra>) {
    let lexicon = Lexicon::builtin();
    let mut evolution = Evolution::at_genesis(seed, era, open_years);
    let mut eras = vec![CalendarEra { name: lexicon.name(seed, 0, EraKind::Genesis, &[]), start_year: 1, kind: EraKind::Genesis }];
    let mut year = 30;
    while year <= evolution.history_years {
        evolution.settle(year);
        open_era(&evolution, year, seed, &lexicon, &mut eras);
        year += 30;
    }
    evolution.opening_warmth = evolution.warmth;
    evolution.opening_sea_shift = evolution.sea_shift;
    (evolution, eras)
}

/// 依天地現在的狀態決定是否改元；改元則追加一筆紀元並回傳它的類別。
pub fn open_era(evolution: &Evolution, year: u32, seed: u64, lexicon: &Lexicon, eras: &mut Vec<CalendarEra>) -> Option<EraKind> {
    let current = eras.last().expect("紀元年表至少有開天一筆");
    let kind = evolution.next_era(year, current)?;
    let used: Vec<String> = eras.iter().map(|e| e.name.clone()).collect();
    let name = lexicon.name(seed, eras.len() as u32, kind, &used);
    eras.push(CalendarEra { name, start_year: year + 1, kind });
    Some(kind)
}

/// 一個洞天福地在這一世升或降了一級（等級見 [`LeyNode::grade`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeShift {
    pub node: NodeId,
    pub from: u8,
    pub to: u8,
}

/// GDD 2.8「每世結算」第 6 步：靈潮低於 [`EVO_NODE_FADE_BELOW`] 時，每個洞天福地以機率
/// [`EVO_NODE_CHANCE`] 降一級（福地則沉寂）；高於 [`EVO_NODE_RISE_ABOVE`] 時，降過級的以同樣的機率
/// 回復一級，至多回到生成時的等級。擲骰只看種子、節點與年份，開局前後用的是同一套。
pub fn shift_nodes(seed: u64, year: u32, tide: f32, nodes: &mut [LeyNode]) -> Vec<NodeShift> {
    shift_nodes_by(tide, nodes, |i| (mix(mix(seed ^ (7 << 48)) ^ ((i as u64) << 32) ^ u64::from(year)) >> 40) as f32 / (1u64 << 24) as f32)
}

/// 同 [`shift_nodes`]，但擲骰取自世界的演化亂數流（人物落地之後的演化每次不同，隨存檔保存）。
pub fn shift_nodes_rng(rng: &mut rand_chacha::ChaCha8Rng, tide: f32, nodes: &mut [LeyNode]) -> Vec<NodeShift> {
    use rand::Rng;
    shift_nodes_by(tide, nodes, |_| rng.gen::<f32>())
}

fn shift_nodes_by(tide: f32, nodes: &mut [LeyNode], mut roll_of: impl FnMut(usize) -> f32) -> Vec<NodeShift> {
    let fading = tide < EVO_NODE_FADE_BELOW;
    let rising = tide > EVO_NODE_RISE_ABOVE;
    let mut shifts = Vec::new();
    if !fading && !rising {
        return shifts;
    }
    for (i, node) in nodes.iter_mut().enumerate() {
        let from = node.grade();
        let to = if fading && from > 0 {
            from - 1
        } else if rising && from < LeyNode::grade_of(node.origin) {
            from + 1
        } else {
            continue;
        };
        if roll_of(i) < EVO_NODE_CHANCE {
            node.strength = LeyNode::LADDER[to as usize];
            shifts.push(NodeShift { node: i as NodeId, from, to });
        }
    }
    shifts
}

/// 開局前的歷史裡洞天福地的升降：自開天第 0 年起每世擲一次，推演到開局那一年；
/// 回傳開局那一年的靈潮（世界生成用它算格點靈氣）。
pub fn history_nodes(seed: u64, era: Era, open_years: u32, _nodes: &mut [LeyNode]) -> f32 {
    // 分期預跑後開天即第 0 年，沒有開局前的歷史；洞天福地的升降在預跑中由每世結算自然發生。
    Evolution::at_genesis(seed, era, open_years).tide
}

/// 依目前的靈潮與洞天福地的等級，重算靈脈強度、格點靈氣與州郡靈氣（GDD 2.4）。
/// 沒有格點資料的手工世界不重算。
pub fn refresh_aura(w: &mut World) {
    if !prepare_aura(w) {
        return;
    }
    let geo = &mut w.geo;
    for edge in &mut geo.ley {
        edge.strength = ley::edge_strength(&geo.nodes, edge.a, edge.b);
    }
    let basis = w.aura_basis.as_ref().expect("prepare_aura 剛建立");
    geo.aura_cell = basis.cells(&geo.terrain, &geo.nodes, &geo.ley, w.evolution.tide);
    for p in &mut w.provinces {
        if !p.cells.is_empty() {
            p.aura = ley::province_aura(&p.cells, &geo.aura_cell);
        }
    }
}

/// 建立格點靈氣的距離衰減表（[`ley::AuraBasis`]）；已有且對得上地圖就不重建。開天時先建好，
/// 每世結算才不必在推演途中花這筆時間。回傳這個世界有沒有格點資料。
pub fn prepare_aura(w: &mut World) -> bool {
    let geo = &w.geo;
    let cells = geo.w as usize * geo.h as usize;
    if cells == 0 || geo.terrain.len() != cells || geo.aura_cell.len() != cells {
        return false;
    }
    if !w.aura_basis.as_ref().is_some_and(|b| b.fits(cells, &geo.nodes, &geo.ley)) {
        let sphere = Sphere::new(geo.w as usize, geo.h as usize);
        w.aura_basis = Some(ley::AuraBasis::build(&sphere, &geo.nodes, &geo.ley));
    }
    true
}

/// GDD 2.8「每世結算」：在每世的天機收入之前執行。草木隨墾殖度消長；地形與海陸隨寒暖與海平面推進一步、州郡重算；
/// 靈氣隨靈潮重算；世界大事、地形的變動與洞天福地的升降寫入因果帳；改元則追加紀元。
pub fn shi_settle(w: &mut World) {
    shi_settle_coarse(w, &mut Vec::new(), true);
}

/// 同 [`shi_settle`]；`land` 為假時略過地形與海陸的推進（最耗時的一步），草木旗標有變的格先記在 `pending`，
/// 下一次 `land` 為真時一併套用。人口期用它把地形每數世推進一次（GDD 4.6）。
pub fn shi_settle_coarse(w: &mut World, pending: &mut Vec<u32>, land: bool) {
    let year = w.evolution.history_years + year_of(w.tick);
    let changes = w.evolution.settle(year);
    let flora = crate::flora::settle(w);
    pending.extend_from_slice(&flora.touched);
    let land = if land {
        pending.sort_unstable();
        pending.dedup();
        let shift = crate::landform::settle_with(w, pending);
        pending.clear();
        shift
    } else {
        Default::default()
    };
    let shifts = shift_nodes_rng(&mut w.rng.evolution, w.evolution.tide, &mut w.geo.nodes);
    refresh_aura(w);
    let opened = open_era(&w.evolution, year, w.params.seed, &Lexicon::builtin(), &mut w.eras);
    // 開啟紀元的那一件掛在紀元上：這一世有跨門檻的對應大事就取它，否則（延後開啟）另記一件。
    let mut events: Vec<(WorldChange, bool)> = changes.iter().map(|&c| (c, false)).collect();
    if let Some(kind) = opened {
        let opener = events.iter().position(|(c, _)| c.opens() == Some(kind)).or_else(|| {
            let ended = kind == EraKind::Peace;
            ended.then(|| events.iter().position(|(c, _)| c.opens().is_none())).flatten()
        });
        match opener {
            Some(i) => events[i].1 = true,
            None => events.push((WorldChange::starting(kind), true)),
        }
    }
    for (change, new_era) in events {
        crate::ledger::record(w, NewEvent::new(EventKind::WorldChanged, EVO_EVENT_IMPORTANCE, EventArgs::World { change, new_era }));
    }
    record_land(w, &land);
    for (kind, provinces) in [(EventKind::ForestCleared, &flora.exhausted), (EventKind::Desertified, &flora.desertified)] {
        for &province in provinces {
            let mut event = NewEvent::new(kind, EVO_FLORA_IMPORTANCE, EventArgs::None);
            event.province = Some(province);
            crate::ledger::record(w, event);
        }
    }
    for shift in shifts {
        let cell = w.geo.nodes[shift.node as usize].cell;
        let args = EventArgs::Node { node: shift.node, from: shift.from, to: shift.to };
        let mut event = NewEvent::new(EventKind::NodeShifted, EVO_NODE_IMPORTANCE, args);
        event.cell = Some(cell);
        event.province = w.geo.province_of.get(cell as usize).copied().filter(|&p| (p as usize) < w.provinces.len());
        crate::ledger::record(w, event);
    }
}

/// 地形演變裡要記事的變動寫入因果帳（GDD 2.8、7.2）：州郡消失、治所遷移、陸橋浮現與沉沒。
fn record_land(w: &mut World, land: &crate::landform::LandShift) {
    for lost in &land.lost {
        let args = EventArgs::Lost { refuge: lost.refuge, perished: lost.perished };
        let mut event = NewEvent::new(EventKind::ProvinceLost, EVO_LOST_IMPORTANCE, args);
        event.province = Some(lost.province);
        crate::ledger::record(w, event);
    }
    for &(province, from, to) in &land.seats {
        let mut event = NewEvent::new(EventKind::SeatMoved, EVO_SEAT_IMPORTANCE, EventArgs::Seat { from, to });
        event.province = Some(province);
        event.cell = Some(to);
        crate::ledger::record(w, event);
    }
    for &(a, b, joined) in &land.links {
        let args = EventArgs::Link { other: b, joined };
        let mut event = NewEvent::new(EventKind::LandLinked, EVO_LINK_IMPORTANCE, args);
        event.province = Some(a);
        crate::ledger::record(w, event);
    }
}

/// 開局以來過了幾年（tick 360 是第 30 年的年底）。
pub fn year_of(tick: Tick) -> u32 {
    tick / crate::calendar::DAYS_PER_YEAR
}

/// 週期為 `period` 年、開天時相位為 `phase` 的正弦波在第 `year` 年的值。
/// 年數先以整數取餘再換成圈數，年代再久也不損精度。
fn wave(year: u32, period: u32, phase: f32) -> f32 {
    let turn = (year % period) as f32 / period as f32 + phase;
    sin(TAU * (turn - turn.floor()))
}

/// SplitMix64 的混合步驟。
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ERAS: [Era; 3] = [Era::Ancient, Era::Middle, Era::Decline];
    const THREE_YUN: u32 = 1080;
    /// 住劫的終點（世界第 113,400 年）：推演到這裡看整個住劫的紀元。
    const KALPA: u32 = LANDING_YEAR_MAX;

    /// 開局後再推演 `years` 年的紀元年表與每世的慢變量。
    fn play(seed: u64, era: Era, years: u32) -> (Evolution, Vec<CalendarEra>, Vec<Evolution>) {
        let lexicon = Lexicon::builtin();
        let (mut e, mut eras) = genesis(seed, era, 0);
        let mut curve = Vec::new();
        let mut year = e.history_years + 30;
        while year <= e.history_years + years {
            e.settle(year);
            open_era(&e, year, seed, &lexicon, &mut eras);
            curve.push(e.clone());
            year += 30;
        }
        (e, eras, curve)
    }

    #[test]
    fn gdd_2_8_periods_and_history_are_within_range() {
        for seed in 0..40u64 {
            for era in ERAS {
                let e = Evolution::at_genesis(seed * 7919 + 1, era, 0);
                assert!((EVO_WARMTH_P1_MIN..=EVO_WARMTH_P1_MAX).contains(&e.warmth_periods[0]));
                assert!((EVO_WARMTH_P2_MIN..=EVO_WARMTH_P2_MAX).contains(&e.warmth_periods[1]));
                assert!((EVO_TIDE_PERIOD_MIN..=EVO_TIDE_PERIOD_MAX).contains(&e.tide_period));
                assert_eq!(e.history_years % 30, 0, "歷史是整數世");
                for p in e.warmth_phases.into_iter().chain([e.tide_phase]) {
                    assert!((0.0..1.0).contains(&p), "相位 {p}");
                }
            }
        }
    }

    #[test]
    fn gdd_2_8_seeds_differ_in_period() {
        let periods: std::collections::BTreeSet<u32> =
            (1..=20u64).map(|s| Evolution::at_genesis(s, Era::Middle, 0).warmth_periods[0]).collect();
        assert!(periods.len() > 10, "20 個種子只有 {} 種週期", periods.len());
    }

    #[test]
    fn gdd_2_8_takeover_tide_follows_the_opening_era() {
        // 開天沒有開局前的歷史；預跑 `open_years` 年後（玩家接手那一年）的靈潮落在開局時代指定的位置。
        for seed in 1..=30u64 {
            for open in [0u32, 1_000, 11_000] {
                let (ancient, _) = genesis(seed, Era::Ancient, open);
                let (middle, _) = genesis(seed, Era::Middle, open);
                let (decline, _) = genesis(seed, Era::Decline, open);
                assert!((ancient.tide_cycle(open) - 1.3).abs() < 0.01, "上古接手靈潮 {}", ancient.tide_cycle(open));
                assert!((middle.tide_cycle(open) - 1.0).abs() < 0.01, "中古接手靈潮 {}", middle.tide_cycle(open));
                assert!((decline.tide_cycle(open) - 0.6).abs() < 0.01, "末法接手靈潮 {}", decline.tide_cycle(open));
                assert_eq!(ancient.history_years, 0, "開天沒有開局前的歷史");
            }
        }
    }

    #[test]
    fn gdd_2_8_values_stay_in_range() {
        for seed in 1..=30u64 {
            for era in ERAS {
                for e in play(seed, era, THREE_YUN).2 {
                    assert!((-1.0..=1.0).contains(&e.warmth), "寒暖 {}", e.warmth);
                    assert!(e.sea_shift.abs() <= EVO_SEA_RANGE, "海平面 {}", e.sea_shift);
                    assert!((EVO_TIDE_MIN..=EVO_TIDE_MAX).contains(&e.tide), "靈潮 {}", e.tide);
                }
            }
        }
    }

    #[test]
    fn gdd_2_8_sea_follows_warmth_one_step_behind() {
        let (mut e, _) = genesis(11, Era::Middle, 0);
        for shi in 1..=36 {
            let before = e.sea_shift;
            e.settle(e.history_years + shi * 30);
            let target = EVO_SEA_RANGE * e.warmth;
            assert!((e.sea_shift - (before + target) * 0.5).abs() < 1e-6, "每世向目標靠近一半");
        }
    }

    #[test]
    fn gdd_2_8_tide_reaches_both_decline_and_revival() {
        // 一個靈潮週期內末法與靈氣復甦各出現一次。
        let (mut e, _) = genesis(5, Era::Middle, 0);
        let mut seen = std::collections::BTreeSet::new();
        let mut year = e.history_years;
        while year <= e.history_years + e.tide_period + 60 {
            year += 30;
            for change in e.settle(year) {
                seen.insert(format!("{change:?}"));
            }
        }
        for name in ["DeclineBegan", "DeclineEnded", "RevivalBegan", "RevivalEnded"] {
            assert!(seen.contains(name), "一個週期內沒有 {name}：{seen:?}");
        }
    }

    #[test]
    fn gdd_2_8_hysteresis_no_flapping() {
        // 同一個時期不會在相鄰兩世開始又結束再開始。
        for seed in 1..=30u64 {
            let (mut e, _) = genesis(seed, Era::Middle, 0);
            let mut last_begin: std::collections::BTreeMap<String, u32> = Default::default();
            let mut year = e.history_years;
            while year <= e.history_years + THREE_YUN {
                year += 30;
                for change in e.settle(year) {
                    let name = format!("{change:?}");
                    if name.ends_with("Began") {
                        if let Some(prev) = last_begin.insert(name.clone(), year) {
                            assert!(year - prev >= 240, "種子 {seed}：{name} 相隔只有 {} 年", year - prev);
                        }
                    }
                }
            }
        }
    }

    /// 老闆裁定（訪談第 65 輪）：紀元最短 8,000 年、中位約 15,000 年，住劫（86,400 年）內約 5–10 個紀元。
    #[test]
    fn gdd_2_8_eras_last_at_least_8000_years_and_median_about_15000() {
        let mut lengths = Vec::new();
        let (mut before_landing, mut seeds) = (Vec::new(), 0u32);
        for seed in 1..=200u64 {
            for era in ERAS {
                let (_, eras, _) = play(seed, era, KALPA);
                for pair in eras.windows(2) {
                    let len = pair[1].start_year - pair[0].start_year;
                    assert!(len >= EVO_ERA_MIN_YEARS, "種子 {seed}：紀元 {} 只有 {len} 年", pair[0].name);
                    assert_ne!(pair[0].kind, pair[1].kind, "相鄰的紀元不同類");
                    lengths.push(len);
                }
                // 住劫內（第 27,000–113,400 年）的紀元數。
                let in_kalpa = eras.iter().filter(|x| x.start_year > LANDING_YEAR_MIN).count() as u32;
                before_landing.push(in_kalpa);
                seeds += 1;
            }
        }
        lengths.sort_unstable();
        let median = lengths[lengths.len() / 2];
        assert!((12_000..=18_000).contains(&median), "紀元長度中位 {median} 年");
        let ok = before_landing.iter().filter(|&&n| (3..=10).contains(&n)).count() as u32;
        assert!(ok * 100 >= seeds * 85, "住劫內改元 3–10 次的只有 {ok}/{seeds}");
    }

    /// 紀元的類別要與開啟時天地的狀態相符，且狀態已穩定 [`EVO_ERA_SETTLE_YEARS`] 年。
    #[test]
    fn gdd_2_8_era_kind_matches_the_world_when_it_opens() {
        for seed in 1..=60u64 {
            let lexicon = Lexicon::builtin();
            let (mut e, mut eras) = genesis(seed, Era::Middle, 0);
            let mut year = 30;
            while year <= KALPA {
                e.settle(year);
                if let Some(kind) = open_era(&e, year, seed, &lexicon, &mut eras) {
                    assert_eq!(kind, e.state_kind(), "種子 {seed} 第 {year} 年：紀元類別與天地狀態不符");
                    assert!(year >= e.kind_since + EVO_ERA_SETTLE_YEARS, "種子 {seed} 第 {year} 年：狀態還沒穩定就改元");
                }
                year += 30;
            }
        }
    }

    #[test]
    fn gdd_2_8_genesis_chronicle_starts_with_one_era() {
        for seed in 1..=30u64 {
            let (e, eras) = genesis(seed, Era::Middle, 0);
            assert_eq!(e.history_years, 0);
            assert_eq!(eras.len(), 1, "開天只有開天一個紀元");
            assert_eq!(eras[0].kind, EraKind::Genesis);
            assert_eq!(eras[0].start_year, 1);
        }
        // 往後推演，同一張年表不重名。
        let mut names = std::collections::BTreeSet::new();
        for seed in 1..=30u64 {
            let (_, eras, _) = play(seed, Era::Middle, KALPA);
            let unique: std::collections::BTreeSet<&str> = eras.iter().map(|x| x.name.as_str()).collect();
            assert_eq!(unique.len(), eras.len(), "同一張年表不重名");
            names.insert(eras.last().unwrap().name.clone());
        }
        assert!(names.len() > 10, "30 個種子的末代紀元只有 {} 種名字", names.len());
    }

    #[test]
    fn gdd_2_8_same_seed_same_chronicle() {
        let a = play(20261011, Era::Middle, KALPA);
        let b = play(20261011, Era::Middle, KALPA);
        assert_eq!(a.1, b.1);
        assert_eq!(a.2, b.2);
        assert_ne!(a.1, play(20261012, Era::Middle, KALPA).1);
    }

    fn node(origin: f32) -> LeyNode {
        LeyNode { cell: 0, strength: origin, origin, name: String::new() }
    }

    #[test]
    fn gdd_2_8_nodes_fade_in_decline_and_stop_at_dormant() {
        let mut nodes: Vec<LeyNode> = [1.0, 0.75, 0.5].into_iter().cycle().take(60).map(node).collect();
        let mut shifted = 0;
        for shi in 1..=10u32 {
            for s in shift_nodes(9, shi * 30, 0.6, &mut nodes) {
                assert_eq!(s.to + 1, s.from, "一世至多降一級");
                shifted += 1;
            }
        }
        // 60 個節點、10 世、每世 0.1：期望約 55 次（已沉寂的不再擲）。
        assert!((30..=80).contains(&shifted), "降級 {shifted} 次");
        assert!(nodes.iter().all(|n| LeyNode::LADDER.contains(&n.strength)));
        for shi in 11..=400u32 {
            shift_nodes(9, shi * 30, 0.6, &mut nodes);
        }
        assert!(nodes.iter().all(|n| n.strength == 0.0), "末法夠久，全部沉寂");
    }

    #[test]
    fn gdd_2_8_nodes_recover_up_to_their_origin() {
        let mut nodes: Vec<LeyNode> = [1.0, 0.75, 0.5].into_iter().cycle().take(60).map(node).collect();
        for n in &mut nodes {
            n.strength = 0.0;
        }
        for shi in 1..=400u32 {
            for s in shift_nodes(9, shi * 30, 1.2, &mut nodes) {
                assert_eq!(s.to, s.from + 1, "一世至多回復一級");
            }
            assert!(nodes.iter().all(|n| n.strength <= n.origin), "不超過生成時的等級");
        }
        assert!(nodes.iter().all(|n| n.strength == n.origin), "靈潮高得夠久，全部回復");
    }

    #[test]
    fn gdd_2_8_nodes_hold_between_the_two_gates() {
        let mut nodes: Vec<LeyNode> = (0..40).map(|_| node(0.75)).collect();
        nodes[0].strength = 0.5;
        let before = nodes.clone();
        for tide in [EVO_NODE_FADE_BELOW, 0.9, 1.0, EVO_NODE_RISE_ABOVE] {
            for shi in 1..=50u32 {
                assert!(shift_nodes(3, shi * 30, tide, &mut nodes).is_empty());
            }
        }
        assert_eq!(nodes, before);
    }

    #[test]
    fn wave_is_exact_at_quarter_turns() {
        assert!((wave(0, 400, 0.25) - 1.0).abs() < 1e-3);
        assert!(wave(100, 400, 0.25).abs() < 1e-3);
        assert!((wave(200, 400, 0.25) + 1.0).abs() < 1e-3);
        assert!((wave(400_000_100, 400, 0.0) - 1.0).abs() < 1e-3, "年代久遠不損精度");
    }
}
