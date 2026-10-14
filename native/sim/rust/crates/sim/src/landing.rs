//! GDD 2.6「成住壞空」、4.6 開局落點：玩家接手的世界年落在住劫內（第 27,000–113,400 年）。
//!
//! 落點年份由世界種子決定（或玩家在開局設定指定），不取作業系統亂數：天地（地形、州郡、靈脈）
//! 隨人口期長度一起推演，同種子同設定必須得到同一片天地。人物期長度不變，所以
//! `pop_years` = 落點年份 − `prerun_years`。

use crate::params::{LANDING_YEAR_MAX, LANDING_YEAR_MIN};
use crate::types::WorldParams;

/// 開局落點的設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    /// 住劫內由種子決定的一年。
    Random,
    /// 住劫前段（前三分之一）內由種子決定的一年。
    Early,
    /// 住劫中段內由種子決定的一年。
    Mid,
    /// 住劫後段內由種子決定的一年。
    Late,
    /// 指定的世界年；超出住劫的夾回住劫。
    Year(u32),
}

impl Landing {
    /// 由字串解析（`"random"`、`"early"`、`"mid"`、`"late"`）；不認得的視為 `Random`。
    pub fn parse(text: &str) -> Self {
        match text {
            "early" => Landing::Early,
            "mid" => Landing::Mid,
            "late" => Landing::Late,
            _ => Landing::Random,
        }
    }
}

/// SplitMix64 的混合步驟。
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 玩家接手時的世界年（人口期 + 人物期）。同種子同設定永遠回傳同一個值，落在 `LANDING_YEAR_MIN..=LANDING_YEAR_MAX`。
pub fn landing_year(seed: u64, landing: Landing) -> u32 {
    let span = LANDING_YEAR_MAX - LANDING_YEAR_MIN + 1;
    let third = span / 3;
    let (lo, len) = match landing {
        Landing::Year(y) => return y.clamp(LANDING_YEAR_MIN, LANDING_YEAR_MAX),
        Landing::Random => (LANDING_YEAR_MIN, span),
        Landing::Early => (LANDING_YEAR_MIN, third),
        Landing::Mid => (LANDING_YEAR_MIN + third, third),
        Landing::Late => (LANDING_YEAR_MIN + 2 * third, span - 2 * third),
    };
    lo + (mix(seed ^ 0x4C41_4E44_494E_47) % len as u64) as u32
}

impl WorldParams {
    /// 依種子與落點設定算出 `pop_years`，使人口期加人物期正好是落點年份。
    /// 先設好 `seed` 與 `prerun_years` 再呼叫；落點年份比人物期短時人口期為 0。
    pub fn set_landing(&mut self, landing: Landing) {
        self.pop_years = landing_year(self.seed, landing).saturating_sub(self.prerun_years);
    }

    /// 接手時的世界年（人口期 + 人物期）。
    pub fn landing_year(&self) -> u32 {
        self.pop_years + self.prerun_years
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seed_lands_in_the_stable_kalpa() {
        for seed in 0..2000u64 {
            for landing in [Landing::Random, Landing::Early, Landing::Mid, Landing::Late] {
                let y = landing_year(seed.wrapping_mul(0x9E37_79B9), landing);
                assert!((LANDING_YEAR_MIN..=LANDING_YEAR_MAX).contains(&y), "{landing:?} 種子 {seed}：{y}");
            }
        }
    }

    #[test]
    fn same_seed_same_landing_and_phases_are_ordered() {
        for seed in [1u64, 20261011, 777] {
            assert_eq!(landing_year(seed, Landing::Random), landing_year(seed, Landing::Random));
            let (e, m, l) = (landing_year(seed, Landing::Early), landing_year(seed, Landing::Mid), landing_year(seed, Landing::Late));
            assert!(e < m && m < l, "前中後段順序：{e} {m} {l}");
        }
    }

    #[test]
    fn different_seeds_spread_over_the_kalpa() {
        let years: Vec<u32> = (0..400u64).map(|s| landing_year(s, Landing::Random)).collect();
        let (lo, hi) = (years.iter().min().unwrap(), years.iter().max().unwrap());
        assert!(*lo < LANDING_YEAR_MIN + 8_000 && *hi > LANDING_YEAR_MAX - 8_000, "落點分布 {lo}..{hi}");
        let early = years.iter().filter(|&&y| y < LANDING_YEAR_MIN + 28_800).count();
        assert!((80..200).contains(&early), "前三分之一有 {early}/400");
    }

    #[test]
    fn explicit_year_is_kept_and_clamped() {
        assert_eq!(landing_year(5, Landing::Year(50_000)), 50_000);
        assert_eq!(landing_year(5, Landing::Year(100)), LANDING_YEAR_MIN);
        assert_eq!(landing_year(5, Landing::Year(900_000)), LANDING_YEAR_MAX);
    }

    #[test]
    fn set_landing_keeps_the_prerun_length() {
        let mut p = WorldParams::new(42);
        p.set_landing(Landing::Random);
        assert_eq!(p.prerun_years, crate::params::GENESIS_PRERUN_YEARS);
        assert_eq!(p.landing_year(), landing_year(42, Landing::Random));
    }
}
