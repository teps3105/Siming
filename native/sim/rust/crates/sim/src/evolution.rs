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
}

impl WorldChange {
    /// 這件大事開啟哪一類紀元；時期結束的不指定（由 [`next_era`] 判斷是否承平）。
    fn opens(self) -> Option<EraKind> {
        match self {
            WorldChange::ColdBegan => Some(EraKind::Cold),
            WorldChange::WarmBegan => Some(EraKind::Warm),
            WorldChange::DeclineBegan => Some(EraKind::Decline),
            WorldChange::RevivalBegan => Some(EraKind::Revival),
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
}

impl Evolution {
    /// 開天第 0 年的天地：週期由種子定；歷史的長度與靈潮在開局時的位置由開局時代定。
    fn at_genesis(seed: u64, era: Era) -> Self {
        let pick = |tag: u64, lo: u32, hi: u32| lo + (mix(seed ^ (tag << 48)) % u64::from(hi - lo + 1)) as u32;
        let phase = |tag: u64| (mix(seed ^ (tag << 48)) >> 40) as f32 / (1u64 << 24) as f32;
        let (history, opening_turn) = match era {
            Era::Ancient => (EVO_HISTORY_ANCIENT, EVO_TIDE_TURN_ANCIENT),
            Era::Middle => (EVO_HISTORY_MIDDLE, EVO_TIDE_TURN_MIDDLE),
            Era::Decline => (EVO_HISTORY_DECLINE, EVO_TIDE_TURN_DECLINE),
        };
        let history_years = pick(6, history.0 / 30, history.1 / 30) * 30;
        let tide_period = pick(5, EVO_TIDE_PERIOD_MIN, EVO_TIDE_PERIOD_MAX);
        // 相位取使開局那一年的靈潮正好落在開局時代指定的位置。
        let elapsed = (history_years % tide_period) as f32 / tide_period as f32;
        let tide_phase = opening_turn - elapsed;
        let mut out = Self {
            history_years,
            warmth: 0.0,
            sea_shift: 0.0,
            tide: 1.0,
            opening_warmth: 0.0,
            opening_sea_shift: 0.0,
            warmth_periods: [
                pick(1, EVO_WARMTH_P1_MIN, EVO_WARMTH_P1_MAX),
                pick(2, EVO_WARMTH_P2_MIN, EVO_WARMTH_P2_MAX),
            ],
            warmth_phases: [phase(3), phase(4)],
            tide_period,
            tide_phase: tide_phase - tide_phase.floor(),
            climate: Climate::Mild,
            aura: AuraAge::Plain,
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

    /// 每世結算一次（`year` 為開天以來的年數）：寒暖與靈潮取週期值，海平面追寒暖；
    /// 回傳這一世跨過門檻的世界大事，先寒暖後靈潮。
    pub fn settle(&mut self, year: u32) -> Vec<WorldChange> {
        self.warmth = self.warmth_cycle(year).clamp(-1.0, 1.0);
        let sea_target = EVO_SEA_RANGE * self.warmth;
        self.sea_shift = (self.sea_shift + (sea_target - self.sea_shift) * EVO_SEA_APPROACH)
            .clamp(-EVO_SEA_RANGE, EVO_SEA_RANGE);
        self.tide = self.tide_cycle(year).clamp(EVO_TIDE_MIN, EVO_TIDE_MAX);

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
        changes
    }

    /// 這一世的世界大事開啟哪一類紀元（GDD 2.8）；不改元時回傳 `None`。
    ///
    /// 有時期開始，取第一件；只有時期結束，而寒暖與靈潮都回到平常，才是承平。
    /// 距上一次改元不滿 [`EVO_ERA_MIN_YEARS`] 年，或與目前的紀元同類，不改元。
    pub fn next_era(&self, changes: &[WorldChange], year: u32, current: &CalendarEra) -> Option<EraKind> {
        let kind = match changes.iter().find_map(|c| c.opens()) {
            Some(kind) => kind,
            None if !changes.is_empty() && self.climate == Climate::Mild && self.aura == AuraAge::Plain => {
                EraKind::Peace
            }
            None => return None,
        };
        // 新紀元的元年是下一年。
        let long_enough = year + 1 >= current.start_year + EVO_ERA_MIN_YEARS;
        (long_enough && kind != current.kind).then_some(kind)
    }
}

/// 開局時的天地與紀元年表：自開天第 0 年起，每世結算一次，推演到開局那一年（GDD 2.8）。
pub fn genesis(seed: u64, era: Era) -> (Evolution, Vec<CalendarEra>) {
    let lexicon = Lexicon::builtin();
    let mut evolution = Evolution::at_genesis(seed, era);
    let mut eras = vec![CalendarEra {
        name: lexicon.name(seed, 0, EraKind::Genesis, &[]),
        start_year: 1,
        kind: EraKind::Genesis,
    }];
    let mut year = 30;
    while year <= evolution.history_years {
        let changes = evolution.settle(year);
        open_era(&evolution, &changes, year, seed, &lexicon, &mut eras);
        year += 30;
    }
    evolution.opening_warmth = evolution.warmth;
    evolution.opening_sea_shift = evolution.sea_shift;
    (evolution, eras)
}

/// 依這一世的世界大事決定是否改元；改元則追加一筆紀元並回傳真。
pub fn open_era(
    evolution: &Evolution,
    changes: &[WorldChange],
    year: u32,
    seed: u64,
    lexicon: &Lexicon,
    eras: &mut Vec<CalendarEra>,
) -> bool {
    let current = eras.last().expect("紀元年表至少有開天一筆");
    let Some(kind) = evolution.next_era(changes, year, current) else {
        return false;
    };
    let used: Vec<String> = eras.iter().map(|e| e.name.clone()).collect();
    let name = lexicon.name(seed, eras.len() as u32, kind, &used);
    eras.push(CalendarEra { name, start_year: year + 1, kind });
    true
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
        let roll = (mix(mix(seed ^ (7 << 48)) ^ ((i as u64) << 32) ^ u64::from(year)) >> 40) as f32 / (1u64 << 24) as f32;
        if roll < EVO_NODE_CHANCE {
            node.strength = LeyNode::LADDER[to as usize];
            shifts.push(NodeShift { node: i as NodeId, from, to });
        }
    }
    shifts
}

/// 開局前的歷史裡洞天福地的升降：自開天第 0 年起每世擲一次，推演到開局那一年；
/// 回傳開局那一年的靈潮（世界生成用它算格點靈氣）。
pub fn history_nodes(seed: u64, era: Era, nodes: &mut [LeyNode]) -> f32 {
    let mut evolution = Evolution::at_genesis(seed, era);
    let mut year = 30;
    while year <= evolution.history_years {
        evolution.settle(year);
        shift_nodes(seed, year, evolution.tide, nodes);
        year += 30;
    }
    evolution.tide
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
    let year = w.evolution.history_years + year_of(w.tick);
    let changes = w.evolution.settle(year);
    let flora = crate::flora::settle(w);
    let land = crate::landform::settle_with(w, &flora.touched);
    let shifts = shift_nodes(w.params.seed, year, w.evolution.tide, &mut w.geo.nodes);
    refresh_aura(w);
    if !changes.is_empty() {
        let new_era = open_era(&w.evolution, &changes, year, w.params.seed, &Lexicon::builtin(), &mut w.eras);
        // 開啟紀元的那一件掛在紀元上：有時期開始取第一件開始的，否則（承平）取最後一件。
        let opener = changes.iter().position(|c| c.opens().is_some()).unwrap_or(changes.len() - 1);
        for (i, &change) in changes.iter().enumerate() {
            let args = EventArgs::World { change, new_era: new_era && i == opener };
            crate::ledger::record(w, NewEvent::new(EventKind::WorldChanged, EVO_EVENT_IMPORTANCE, args));
        }
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
    tick / 12
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

    /// 開局後再推演 `years` 年的紀元年表與每世的慢變量。
    fn play(seed: u64, era: Era, years: u32) -> (Evolution, Vec<CalendarEra>, Vec<Evolution>) {
        let lexicon = Lexicon::builtin();
        let (mut e, mut eras) = genesis(seed, era);
        let mut curve = Vec::new();
        let mut year = e.history_years + 30;
        while year <= e.history_years + years {
            let changes = e.settle(year);
            open_era(&e, &changes, year, seed, &lexicon, &mut eras);
            curve.push(e.clone());
            year += 30;
        }
        (e, eras, curve)
    }

    #[test]
    fn gdd_2_8_periods_and_history_are_within_range() {
        for seed in 0..40u64 {
            for (era, range) in ERAS.into_iter().zip([EVO_HISTORY_ANCIENT, EVO_HISTORY_MIDDLE, EVO_HISTORY_DECLINE]) {
                let e = Evolution::at_genesis(seed * 7919 + 1, era);
                assert!((EVO_WARMTH_P1_MIN..=EVO_WARMTH_P1_MAX).contains(&e.warmth_periods[0]));
                assert!((EVO_WARMTH_P2_MIN..=EVO_WARMTH_P2_MAX).contains(&e.warmth_periods[1]));
                assert!((EVO_TIDE_PERIOD_MIN..=EVO_TIDE_PERIOD_MAX).contains(&e.tide_period));
                assert!((range.0..=range.1).contains(&e.history_years), "歷史 {} 年", e.history_years);
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
            (1..=20u64).map(|s| Evolution::at_genesis(s, Era::Middle).warmth_periods[0]).collect();
        assert!(periods.len() > 10, "20 個種子只有 {} 種週期", periods.len());
    }

    #[test]
    fn gdd_2_8_opening_tide_follows_the_opening_era() {
        for seed in 1..=30u64 {
            let (ancient, _) = genesis(seed, Era::Ancient);
            let (middle, _) = genesis(seed, Era::Middle);
            let (decline, _) = genesis(seed, Era::Decline);
            assert!((ancient.tide - 1.3).abs() < 0.01, "上古開局靈潮 {}", ancient.tide);
            assert!((middle.tide - 1.0).abs() < 0.01, "中古開局靈潮 {}", middle.tide);
            assert!((decline.tide - 0.6).abs() < 0.01, "末法開局靈潮 {}", decline.tide);
            assert_eq!(ancient.aura, AuraAge::Revival, "上古開局正值靈氣復甦");
            assert_eq!(middle.aura, AuraAge::Plain);
            assert_eq!(decline.aura, AuraAge::Decline, "末法開局正值末法");
            // 上古與中古正在退，末法在谷底、之後回升。
            assert!(ancient.tide_cycle(ancient.history_years + 30) < ancient.tide);
            assert!(middle.tide_cycle(middle.history_years + 30) < middle.tide);
            assert!(decline.tide_cycle(decline.history_years + 30) >= decline.tide);
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
        let (mut e, _) = genesis(11, Era::Middle);
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
        let (mut e, _) = genesis(5, Era::Middle);
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
            let (mut e, _) = genesis(seed, Era::Middle);
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

    #[test]
    fn gdd_2_8_era_changes_two_to_six_per_three_yun_and_never_short() {
        let (mut total, mut within) = (0u32, 0u32);
        for seed in 1..=200u64 {
            for era in ERAS {
                let (e, eras, _) = play(seed, era, THREE_YUN);
                for pair in eras.windows(2) {
                    assert!(
                        pair[1].start_year - pair[0].start_year >= EVO_ERA_MIN_YEARS,
                        "種子 {seed}：紀元 {} 只有 {} 年",
                        pair[0].name,
                        pair[1].start_year - pair[0].start_year
                    );
                    assert_ne!(pair[0].kind, pair[1].kind, "相鄰的紀元不同類");
                }
                let after = eras.iter().filter(|x| x.start_year > e.history_years + 1).count();
                total += 1;
                within += u32::from((2..=6).contains(&after));
            }
        }
        assert!(within * 100 >= total * 90, "三運內改元 2–6 次的只有 {within}/{total}");
    }

    #[test]
    fn gdd_2_8_history_gives_a_chronicle_before_the_opening() {
        let mut names = std::collections::BTreeSet::new();
        for seed in 1..=30u64 {
            let (e, eras) = genesis(seed, Era::Middle);
            assert_eq!(eras[0].kind, EraKind::Genesis);
            assert_eq!(eras[0].start_year, 1);
            assert!(eras.len() >= 3, "種子 {seed}：開局前只有 {} 個紀元", eras.len());
            assert!(eras.iter().all(|x| x.start_year <= e.history_years + 1));
            let unique: std::collections::BTreeSet<&str> = eras.iter().map(|x| x.name.as_str()).collect();
            assert_eq!(unique.len(), eras.len(), "同一張年表不重名");
            names.insert(eras.last().unwrap().name.clone());
        }
        assert!(names.len() > 15, "30 個種子的開局紀元只有 {} 種名字", names.len());
    }

    #[test]
    fn gdd_2_8_same_seed_same_chronicle() {
        let a = play(20261011, Era::Middle, THREE_YUN);
        let b = play(20261011, Era::Middle, THREE_YUN);
        assert_eq!(a.1, b.1);
        assert_eq!(a.2, b.2);
        assert_ne!(a.1, play(20261012, Era::Middle, THREE_YUN).1);
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
    fn gdd_2_8_opening_nodes_carry_the_history() {
        let fresh: Vec<LeyNode> = [1.0, 0.75, 0.5].into_iter().cycle().take(22).map(node).collect();
        let (mut ancient_down, mut decline_down) = (0usize, 0usize);
        for seed in 1..=40u64 {
            let mut a = fresh.clone();
            let mut d = fresh.clone();
            assert!((history_nodes(seed, Era::Ancient, &mut a) - 1.3).abs() < 0.01);
            assert!((history_nodes(seed, Era::Decline, &mut d) - 0.6).abs() < 0.01);
            ancient_down += a.iter().filter(|n| n.strength < n.origin).count();
            decline_down += d.iter().filter(|n| n.strength < n.origin).count();
            let mut again = fresh.clone();
            history_nodes(seed, Era::Decline, &mut again);
            assert_eq!(again, d, "同種子同結果");
        }
        let total = 40 * fresh.len();
        // 實測 40 個種子：上古 19%、末法 53%（上古的歷史不滿一個靈潮週期，早年降的還沒全部回復）。
        assert!(ancient_down * 4 < total, "上古開局降級中的節點 {ancient_down}/{total}");
        assert!(decline_down * 10 > total * 4, "末法開局降級中的節點 {decline_down}/{total}");
        assert!(decline_down * 10 < total * 7, "末法開局降級中的節點 {decline_down}/{total}");
    }

    #[test]
    fn wave_is_exact_at_quarter_turns() {
        assert!((wave(0, 400, 0.25) - 1.0).abs() < 1e-3);
        assert!(wave(100, 400, 0.25).abs() < 1e-3);
        assert!((wave(200, 400, 0.25) + 1.0).abs() < 1e-3);
        assert!((wave(400_000_100, 400, 0.0) - 1.0).abs() < 1e-3, "年代久遠不損精度");
    }
}
