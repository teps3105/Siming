//! GDD 2.8 天地演變：慢變量與每世結算。
//!
//! 目前只有全球的三個慢變量——寒暖、海平面偏移、靈潮——每世（30 年）更新一次；
//! 它們還沒有接到地形、海陸與靈氣上，也還不觸發世界大事。
//! 週期的長度與相位由世界種子算出，不消耗隨機流；靈潮的起點由開局時代決定。

use serde::{Deserialize, Serialize};

use crate::params::*;
use crate::sim::World;
use crate::types::{Era, Tick};
use crate::worldgen::grid::{sin, TAU};

/// 天地的慢變量與它們的週期（GDD 2.8「慢變量」）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evolution {
    /// 全球寒暖：−1（極寒）..1（極暖）。
    pub warmth: f32,
    /// 海平面的偏移（高度單位）。開局為 0：生成時的海岸就是開局的海岸。
    pub sea_shift: f32,
    /// 靈潮：天地靈氣的倍率。
    pub tide: f32,
    /// 寒暖兩個週期的長度（年）。
    pub warmth_periods: [u32; 2],
    /// 寒暖兩個週期在開天時的相位（0..1，一整圈為 1）。
    pub warmth_phases: [f32; 2],
    /// 靈潮週期的長度（年）。
    pub tide_period: u32,
    /// 靈潮週期在開天時的相位（0..1）。
    pub tide_phase: f32,
}

impl Evolution {
    /// 開天時的天地：週期由種子定，靈潮的初值與相位由開局時代定。
    pub fn new(seed: u64, era: Era) -> Self {
        let pick = |tag: u64, lo: u32, hi: u32| {
            lo + (mix(seed ^ (tag << 48)) % u64::from(hi - lo + 1)) as u32
        };
        let phase = |tag: u64| (mix(seed ^ (tag << 48)) >> 40) as f32 / (1u64 << 24) as f32;
        let (tide, tide_phase) = match era {
            Era::Ancient => (AURA_ERA_ANCIENT, EVO_TIDE_PHASE_PEAK),
            Era::Middle => (AURA_ERA_MIDDLE, EVO_TIDE_PHASE_FALLING),
            Era::Decline => (AURA_ERA_DECLINE, EVO_TIDE_PHASE_TROUGH),
        };
        let mut out = Self {
            warmth: 0.0,
            sea_shift: 0.0,
            tide,
            warmth_periods: [
                pick(1, EVO_WARMTH_P1_MIN, EVO_WARMTH_P1_MAX),
                pick(2, EVO_WARMTH_P2_MIN, EVO_WARMTH_P2_MAX),
            ],
            warmth_phases: [phase(3), phase(4)],
            tide_period: pick(5, EVO_TIDE_PERIOD_MIN, EVO_TIDE_PERIOD_MAX),
            tide_phase,
        };
        out.warmth = out.warmth_cycle(0);
        out
    }

    /// 寒暖的週期值：兩個長週期疊加，−1..1。
    pub fn warmth_cycle(&self, year: u32) -> f32 {
        EVO_WARMTH_W1 * wave(year, self.warmth_periods[0], self.warmth_phases[0])
            + EVO_WARMTH_W2 * wave(year, self.warmth_periods[1], self.warmth_phases[1])
    }

    /// 靈潮的週期值：以 1 為中心上下起伏。
    pub fn tide_cycle(&self, year: u32) -> f32 {
        1.0 + EVO_TIDE_AMPLITUDE * wave(year, self.tide_period, self.tide_phase)
    }

    /// 每世結算一次：寒暖取週期值，海平面追寒暖，靈潮追自己的週期值。
    pub fn settle(&mut self, year: u32) {
        self.warmth = self.warmth_cycle(year).clamp(-1.0, 1.0);
        let sea_target = EVO_SEA_RANGE * self.warmth;
        self.sea_shift = (self.sea_shift + (sea_target - self.sea_shift) * EVO_SEA_APPROACH)
            .clamp(-EVO_SEA_RANGE, EVO_SEA_RANGE);
        self.tide = (self.tide + (self.tide_cycle(year) - self.tide) * EVO_TIDE_APPROACH)
            .clamp(EVO_TIDE_MIN, EVO_TIDE_MAX);
    }
}

/// GDD 2.8「每世結算」：在每世的天機收入之前執行。
pub fn shi_settle(w: &mut World) {
    let year = year_of(w.tick);
    w.evolution.settle(year);
}

/// 開天以來過了幾年（tick 360 是第 30 年的年底）。
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

    const SHI_PER_THREE_YUN: u32 = 36;

    fn curve(seed: u64, era: Era) -> Vec<Evolution> {
        let mut e = Evolution::new(seed, era);
        (1..=SHI_PER_THREE_YUN)
            .map(|shi| {
                e.settle(shi * 30);
                e.clone()
            })
            .collect()
    }

    #[test]
    fn gdd_2_8_periods_and_phases_are_within_range() {
        for seed in 0..40u64 {
            let e = Evolution::new(seed * 7919 + 1, Era::Middle);
            assert!((EVO_WARMTH_P1_MIN..=EVO_WARMTH_P1_MAX).contains(&e.warmth_periods[0]));
            assert!((EVO_WARMTH_P2_MIN..=EVO_WARMTH_P2_MAX).contains(&e.warmth_periods[1]));
            assert!((EVO_TIDE_PERIOD_MIN..=EVO_TIDE_PERIOD_MAX).contains(&e.tide_period));
            for p in e.warmth_phases {
                assert!((0.0..1.0).contains(&p), "相位 {p}");
            }
        }
    }

    #[test]
    fn gdd_2_8_seeds_differ_in_period() {
        let periods: std::collections::BTreeSet<u32> = (1..=20u64)
            .map(|s| Evolution::new(s, Era::Middle).warmth_periods[0])
            .collect();
        assert!(periods.len() > 10, "20 個種子只有 {} 種週期", periods.len());
    }

    #[test]
    fn gdd_2_8_tide_starts_by_opening_era() {
        let ancient = Evolution::new(3, Era::Ancient);
        let middle = Evolution::new(3, Era::Middle);
        let decline = Evolution::new(3, Era::Decline);
        assert_eq!((ancient.tide, middle.tide, decline.tide), (1.3, 1.0, 0.6));
        // 上古在高點、末法在低點、中古正在退。
        assert!((ancient.tide_cycle(0) - 1.3).abs() < 1e-3);
        assert!((decline.tide_cycle(0) - 0.7).abs() < 1e-3);
        assert!((middle.tide_cycle(0) - 1.0).abs() < 1e-3);
        assert!(middle.tide_cycle(30) < middle.tide_cycle(0));
    }

    #[test]
    fn gdd_2_8_values_stay_in_range_for_three_yun() {
        for seed in 1..=30u64 {
            for era in [Era::Ancient, Era::Middle, Era::Decline] {
                for e in curve(seed, era) {
                    assert!((-1.0..=1.0).contains(&e.warmth), "寒暖 {}", e.warmth);
                    assert!(e.sea_shift.abs() <= EVO_SEA_RANGE, "海平面 {}", e.sea_shift);
                    assert!(
                        (EVO_TIDE_MIN..=EVO_TIDE_MAX).contains(&e.tide),
                        "靈潮 {}",
                        e.tide
                    );
                }
            }
        }
    }

    #[test]
    fn gdd_2_8_sea_follows_warmth_one_step_behind() {
        let mut e = Evolution::new(11, Era::Middle);
        for shi in 1..=SHI_PER_THREE_YUN {
            let before = e.sea_shift;
            e.settle(shi * 30);
            let target = EVO_SEA_RANGE * e.warmth;
            assert!(
                (e.sea_shift - (before + target) * 0.5).abs() < 1e-6,
                "每世向目標靠近一半"
            );
        }
    }

    #[test]
    fn gdd_2_8_decline_tide_climbs_from_its_low_start() {
        let c = curve(5, Era::Decline);
        assert!(c[0].tide > 0.6, "末法開局低於週期的低點，第一世就回升");
        assert!(
            c.iter().map(|e| e.tide).fold(0.0, f32::max) > 1.0,
            "三運內漲過 1"
        );
    }

    #[test]
    fn gdd_2_8_warmth_swings_both_ways_within_three_yun() {
        for seed in 1..=30u64 {
            let c = curve(seed, Era::Middle);
            let hi = c.iter().map(|e| e.warmth).fold(-1.0, f32::max);
            let lo = c.iter().map(|e| e.warmth).fold(1.0, f32::min);
            assert!(hi > 0.3 && lo < -0.3, "種子 {seed} 的寒暖只在 {lo}..{hi}");
        }
    }

    #[test]
    fn gdd_2_8_same_seed_same_curve() {
        assert_eq!(curve(20261011, Era::Middle), curve(20261011, Era::Middle));
        assert_ne!(curve(20261011, Era::Middle), curve(20261012, Era::Middle));
    }

    #[test]
    fn wave_is_exact_at_quarter_turns() {
        assert!((wave(0, 400, 0.25) - 1.0).abs() < 1e-3);
        assert!(wave(100, 400, 0.25).abs() < 1e-3);
        assert!((wave(200, 400, 0.25) + 1.0).abs() < 1e-3);
        assert!(
            (wave(400_000_100, 400, 0.0) - 1.0).abs() < 1e-3,
            "年代久遠不損精度"
        );
    }
}
