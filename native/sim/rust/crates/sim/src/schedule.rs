//! GDD 6.3 判定月：個人與宗門的年度判定，各落在自己固定的月份。
//!
//! 每個實體（有名之人、魂、宗門）依「世界種子＋類別＋編號」雜湊出 0–11 的判定月（對應曆法 1–12 月）；
//! 同一實體每年同一月，一年內恰好輪到一次。所有「每年判定一次」的個人與宗門規則，都以 [`Round`] 決定本次要處理誰。
//! 全域的年度結算（州郡人口、年齡、補魂、王朝、天地演變）不分月，仍在年底月（第 12 月）。

use std::cell::RefCell;

use crate::types::Tick;

/// 判定月所屬的實體類別；不同類別各自獨立雜湊，避免同編號的人與宗門永遠同月。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// 有名之人（`PersonId`）：修煉、突破、遊歷、入道、婚配、散修創宗與投靠。
    Person = 1,
    /// 魂（`SoulId`）：宿緣的相遇、師徒超越與預兆（以債務方的魂為準）。
    Soul = 2,
    /// 宗門（`PowerId`）：補足、拜師、爭脈（以編號較小者為準）、繼任、聲望與範圍。
    Sect = 3,
    /// 州郡（`ProvinceId`）：人口成長、饑荒、遷徙與殘破恢復，以及無主州郡的立國判定。
    Province = 4,
    /// 王朝（`DynastyId`）：天命、繼承、財政、補缺、起義、割據、開疆、宣戰、禁道與封禪的年度結算。
    Dynasty = 5,
}

fn mix(mut x: u64) -> u64 {
    // splitmix64 的終結步驟。
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// 實體的判定月（0–11，0 是曆法一月、11 是十二月）。確定性，只與種子、類別、編號有關。
pub fn judge_month(seed: u64, domain: Domain, id: u32) -> u8 {
    let h = mix(mix(seed ^ 0x9e37_79b9_7f4a_7c15) ^ ((domain as u64) << 40 | id as u64));
    (h % 12) as u8
}

/// 某個 (種子, 類別) 下，編號依判定月分成 12 桶（桶內編號升冪）。判定月只由種子、類別、編號決定，
/// 因此這份表是純函式 [`judge_month`] 的備忘，與任何世界狀態無關，可跨世界、跨存讀檔共用。
#[derive(Default)]
struct Buckets {
    built: usize,
    by_month: [Vec<u32>; 12],
}

thread_local! {
    static BUCKETS: RefCell<Vec<((u64, u8), Buckets)>> = const { RefCell::new(Vec::new()) };
}

/// 編號 `0..n` 中判定月等於 `month` 者，升冪。等價於逐一呼叫 [`judge_month`] 過濾，但不用逐個重算。
pub fn month_ids(seed: u64, domain: Domain, n: usize, month: u8) -> Vec<u32> {
    BUCKETS.with(|c| {
        let mut c = c.borrow_mut();
        let key = (seed, domain as u8);
        let k = match c.iter().position(|e| e.0 == key) {
            Some(k) => k,
            None => {
                c.push((key, Buckets::default()));
                c.len() - 1
            }
        };
        let b = &mut c[k].1;
        while b.built < n {
            let id = b.built as u32;
            b.by_month[judge_month(seed, domain, id) as usize].push(id);
            b.built += 1;
        }
        let v = &b.by_month[month as usize % 12];
        let end = v.partition_point(|&x| (x as usize) < n);
        v[..end].to_vec()
    })
}

/// tick 對應的月（0–11）：tick 1 是一月，tick 12 是十二月（年底月）。與 [`crate::calendar::date`] 一致。
pub fn month_of(tick: Tick) -> u8 {
    (tick.saturating_sub(1) / crate::calendar::DAYS_PER_MONTH % 12) as u8
}

/// tick 所屬的年序（從 0 起算）：tick 1–12 為第 0 年。
pub fn year_of(tick: Tick) -> u32 {
    tick.saturating_sub(1) / crate::calendar::DAYS_PER_YEAR
}

/// 本次呼叫要處理哪些實體。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    /// 全部實體（一次判定整年；測試與工具用）。
    All,
    /// 只處理判定月等於此月（0–11）的實體。
    Month(u8),
}

impl Round {
    /// 目前這個 tick 的月份輪次（`Sim::step` 用）。
    pub fn at(tick: Tick) -> Round {
        Round::Month(month_of(tick))
    }

    /// 這個實體本輪是否輪到。
    pub fn takes(self, seed: u64, domain: Domain, id: u32) -> bool {
        match self {
            Round::All => true,
            Round::Month(m) => judge_month(seed, domain, id) == m,
        }
    }

    /// 編號 `0..n` 中本輪輪到者，升冪。
    pub fn ids(self, seed: u64, domain: Domain, n: usize) -> Vec<u32> {
        match self {
            Round::All => (0..n as u32).collect(),
            Round::Month(m) => month_ids(seed, domain, n, m),
        }
    }

    /// 是否為單月輪次（單月輪次才以年度戳記擋重複判定）。
    pub fn is_month(self) -> bool {
        matches!(self, Round::Month(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdd_6_3_judge_month_is_deterministic_and_in_range() {
        for id in 0..2000 {
            let m = judge_month(42, Domain::Person, id);
            assert!(m < 12);
            assert_eq!(m, judge_month(42, Domain::Person, id));
        }
    }

    #[test]
    fn gdd_6_3_judge_month_is_roughly_uniform() {
        for domain in [Domain::Person, Domain::Soul, Domain::Sect] {
            let mut count = [0u32; 12];
            for id in 0..12_000 {
                count[judge_month(7, domain, id) as usize] += 1;
            }
            for c in count {
                assert!((800..=1200).contains(&c), "{domain:?} 分布 {count:?}");
            }
        }
    }

    #[test]
    fn gdd_6_3_judge_month_differs_by_seed_and_domain() {
        let same_seed = (0..200).filter(|&i| judge_month(1, Domain::Person, i) == judge_month(2, Domain::Person, i)).count();
        let same_domain = (0..200).filter(|&i| judge_month(1, Domain::Person, i) == judge_month(1, Domain::Sect, i)).count();
        assert!(same_seed < 60 && same_domain < 60, "{same_seed} {same_domain}");
    }

    #[test]
    fn gdd_6_3_each_entity_is_judged_exactly_once_per_year() {
        for id in 0..300 {
            for start in [1u32, 5, 13, 100] {
                let n = (start..start + 12).map(|k| k * 30).filter(|&t| Round::at(t).takes(9, Domain::Person, id)).count();
                assert_eq!(n, 1, "id {id} 起 tick {start}");
            }
        }
    }

    #[test]
    fn gdd_6_3_year_end_tick_is_month_twelve() {
        assert_eq!(month_of(12 * 30), 11);
        assert_eq!(month_of(13 * 30), 0);
        assert_eq!(year_of(12 * 30), 0);
        assert_eq!(year_of(13 * 30), 1);
        assert_eq!(crate::calendar::date(12 * 30).month, 12);
    }
}
