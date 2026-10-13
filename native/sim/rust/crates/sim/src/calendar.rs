//! GDD 2.6 曆法。
//!
//! 顯示用紀元：「開天紀元 12年 11月」，紀元由世界裡的大事開啟，紀元內的年份重新起算。
//! 規則用的週期沿用元會運世的長度：1 月 = 1 tick；12 月 = 1 年；30 年 = 1 世；
//! 12 世 = 1 運（360 年）；30 運 = 1 會（10,800 年）。這些單位不出現在畫面上。
//!
//! 計算約定（GDD 6.3 觸發條件：tick 從 1 起算，開天後第一個月為 1）：
//! - tick 0 是開天時刻（尚無月經過），本實作將 tick 0 與 tick 1 同樣對應到
//!   第一會第一運第一世第一年一月。即 `elapsed = tick.saturating_sub(1)`。
//! - 所有欄位皆為 1 起算。

use crate::types::Tick;

const MONTHS_PER_YEAR: u32 = 12;
const YEARS_PER_SHI: u32 = 30;
const SHI_PER_YUN: u32 = 12;
const YUN_PER_HUI: u32 = 30;

/// 元會運世日期（GDD 2.6）。所有欄位皆為 1 起算。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    /// 會（1..）。
    pub hui: u32,
    /// 運，於會之內（1..=30）。
    pub yun: u32,
    /// 世，於運之內（1..=12）。
    pub shi: u32,
    /// 年，於世之內（1..=30）。
    pub year: u32,
    /// 月，於年之內（1..=12）。
    pub month: u32,
    /// 自開天起的第幾年（第 N 年，1 起算）。
    pub total_year: u32,
}

/// 將 tick 換算為元會運世日期（GDD 2.6）。
///
/// tick 0 與 tick 1 皆對應第一年一月（見模組說明）。
pub fn date(tick: Tick) -> Date {
    let elapsed = tick.saturating_sub(1);
    let month = elapsed % MONTHS_PER_YEAR + 1;
    let year_idx = elapsed / MONTHS_PER_YEAR; // 自開天起已過的整年數
    let total_year = year_idx + 1;
    let shi_idx = year_idx / YEARS_PER_SHI;
    let year = year_idx % YEARS_PER_SHI + 1;
    let yun_idx = shi_idx / SHI_PER_YUN;
    let shi = shi_idx % SHI_PER_YUN + 1;
    let hui_idx = yun_idx / YUN_PER_HUI;
    let yun = yun_idx % YUN_PER_HUI + 1;
    let hui = hui_idx + 1;
    Date { hui, yun, shi, year, month, total_year }
}

/// 日序：自開天起的日數（開天當下為 0，第一個月的第一天為 1）。事件以日為時刻（事件驅動引擎）。
pub type Day = u32;
/// 每月 30 日；一年 12 月、360 日。
pub const DAYS_PER_MONTH: u32 = 30;
pub const DAYS_PER_YEAR: u32 = DAYS_PER_MONTH * MONTHS_PER_YEAR;

/// 月序 `tick` 的最後一日。月步進（舊規則）在這一日結算。
pub fn day_of_tick(tick: Tick) -> Day {
    tick * DAYS_PER_MONTH
}

/// 日所屬的月序（`tick`）：第 1–30 日屬 tick 1，開天當下（日 0）屬 tick 0。
pub fn tick_of_day(day: Day) -> Tick {
    day.div_ceil(DAYS_PER_MONTH)
}

/// 含日的日期：元會運世日期加上月內第幾日（1..=30）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayDate {
    pub date: Date,
    /// 日，於月之內（1..=30）。
    pub day: u32,
}

/// 將日序換算為含日的日期。日 0 與日 1 同為第一年一月一日。
pub fn day_date(day: Day) -> DayDate {
    DayDate { date: date(tick_of_day(day)), day: day.saturating_sub(1) % DAYS_PER_MONTH + 1 }
}

/// 紀元（GDD 2.6）：世界裡的大事會開啟新的紀元，日期以「某某紀元 N年 M月」表示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Era {
    /// 紀元名，不含「紀元」二字。
    pub name: String,
    /// 這個紀元的元年是自開天起的第幾年（1 起算）。
    pub start_year: u32,
    /// 開啟這個紀元的世界大事的性質。
    pub kind: crate::era::EraKind,
}

/// 開局那件世界大事的名稱；自它起算的年份寫作「開天以來第 N 年」。
pub const GENESIS: &str = "開天";

/// 自開天起的第幾年（1 起算）：`history` 是開局前已過的年數（GDD 2.8）。
pub fn total_year(tick: Tick, history: u32) -> u32 {
    history + date(tick).total_year
}

/// `tick` 所在的紀元與紀元內的年份（1 起算）：取起始年不晚於當年的最後一個紀元。
/// `eras` 須依起始年遞增排列，且第一個紀元始於第 1 年。
pub fn era_year(tick: Tick, history: u32, eras: &[Era]) -> (&Era, u32) {
    let total = total_year(tick, history);
    let era = eras.iter().rev().find(|e| e.start_year <= total).unwrap_or(&eras[0]);
    (era, total.saturating_sub(era.start_year) + 1)
}

/// 日期（GDD 2.6）：「鴻蒙紀元 12年 11月」，供題簽使用。年、月用阿拉伯數字。
pub fn era_date(tick: Tick, history: u32, eras: &[Era]) -> String {
    let (era, year) = era_year(tick, history, eras);
    format!("{}紀元 {}年 {}月", era.name, year, date(tick).month)
}

/// 完整日期：紀元日期加自開天起的年份，「鴻蒙紀元 12年 11月（開天以來第 12 年）」。
pub fn full(tick: Tick, history: u32, eras: &[Era]) -> String {
    format!("{}（{}以來第 {} 年）", era_date(tick, history, eras), GENESIS, total_year(tick, history))
}

/// 天書與卡片中的年份標記：「第 N 年」（自開天起，Arabic 數字；GDD 2.6）。
pub fn year_label(tick: Tick, history: u32) -> String {
    format!("第 {} 年", total_year(tick, history))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_0_and_1_are_first_month() {
        let expected = Date { hui: 1, yun: 1, shi: 1, year: 1, month: 1, total_year: 1 };
        assert_eq!(date(0), expected);
        assert_eq!(date(1), expected);
    }

    #[test]
    fn year_one_month_twelve() {
        let d = date(12);
        assert_eq!((d.year, d.month, d.total_year), (1, 12, 1));
    }

    #[test]
    fn tick_13_is_year_two_month_one() {
        let d = date(13);
        assert_eq!((d.year, d.month, d.total_year), (2, 1, 2));
    }

    #[test]
    fn tick_360_is_end_of_shi_one() {
        let d = date(360);
        assert_eq!((d.shi, d.year, d.month), (1, 30, 12));
        assert_eq!(d.total_year, 30);
    }

    #[test]
    fn tick_361_is_shi_two() {
        let d = date(361);
        assert_eq!((d.shi, d.year, d.month), (2, 1, 1));
        assert_eq!(d.total_year, 31);
    }

    #[test]
    fn tick_4320_is_last_month_of_yun_one() {
        let d = date(4320);
        assert_eq!((d.yun, d.shi, d.year, d.month), (1, 12, 30, 12));
        assert_eq!(d.hui, 1);
        assert_eq!(d.total_year, 360);
    }

    #[test]
    fn tick_4321_is_yun_two() {
        let d = date(4321);
        assert_eq!((d.yun, d.shi, d.year, d.month), (2, 1, 1, 1));
        assert_eq!(d.hui, 1);
        assert_eq!(d.total_year, 361);
    }

    #[test]
    fn tick_129600_is_hui_two() {
        // 1 會 = 30 運 = 129,600 月。
        let d = date(129_600);
        assert_eq!((d.hui, d.yun, d.shi, d.year, d.month), (1, 30, 12, 30, 12));
        let d = date(129_601);
        assert_eq!((d.hui, d.yun, d.shi, d.year, d.month), (2, 1, 1, 1, 1));
    }

    fn chronicle() -> Vec<Era> {
        vec![Era { name: "鴻蒙".to_string(), start_year: 1, kind: crate::era::EraKind::Genesis }]
    }

    #[test]
    fn date_is_era_with_arabic_year_and_month() {
        let eras = chronicle();
        assert_eq!(era_date(0, 0, &eras), "鴻蒙紀元 1年 1月");
        assert_eq!(era_date(12 * 11 + 11, 0, &eras), "鴻蒙紀元 12年 11月");
        // 年份不在 30 年或 360 年處歸零。
        assert_eq!(era_date(12 * 360 + 1, 0, &eras), "鴻蒙紀元 361年 1月");
    }

    #[test]
    fn full_form_adds_the_year_since_genesis() {
        assert_eq!(full(12 * 11 + 11, 0, &chronicle()), "鴻蒙紀元 12年 11月（開天以來第 12 年）");
    }

    #[test]
    fn a_new_era_restarts_the_year_count() {
        let mut eras = chronicle();
        eras.push(Era { name: "玄冥".to_string(), start_year: 361, kind: crate::era::EraKind::Cold });
        // 第 360 年還在舊紀元；第 361 年是新紀元元年。
        assert_eq!(era_date(12 * 359 + 12, 0, &eras), "鴻蒙紀元 360年 12月");
        assert_eq!(era_date(12 * 360 + 1, 0, &eras), "玄冥紀元 1年 1月");
        assert_eq!(era_date(12 * 371 + 3, 0, &eras), "玄冥紀元 12年 3月");
        assert_eq!(era_year(12 * 371 + 3, 0, &eras).0.name, "玄冥");
    }

    #[test]
    fn history_before_the_opening_counts_into_the_year() {
        // 開局前已過 1,800 年，目前的紀元始於第 1,741 年：開局第 1 年是該紀元的第 61 年。
        let mut eras = chronicle();
        eras.push(Era { name: "玄冥".to_string(), start_year: 1741, kind: crate::era::EraKind::Cold });
        assert_eq!(era_date(1, 1800, &eras), "玄冥紀元 61年 1月");
        assert_eq!(full(13, 1800, &eras), "玄冥紀元 62年 1月（開天以來第 1802 年）");
    }

    #[test]
    fn year_label_is_total_year() {
        assert_eq!(year_label(0, 0), "第 1 年");
        assert_eq!(year_label(13, 0), "第 2 年");
        assert_eq!(year_label(361, 0), "第 31 年");
        assert_eq!(year_label(361, 1800), "第 1831 年");
    }

    #[test]
    fn day_and_tick_round_trip() {
        assert_eq!(day_of_tick(0), 0);
        assert_eq!(day_of_tick(1), 30);
        assert_eq!(tick_of_day(0), 0);
        assert_eq!(tick_of_day(1), 1);
        assert_eq!(tick_of_day(30), 1);
        assert_eq!(tick_of_day(31), 2);
        for t in 0..500 {
            assert_eq!(tick_of_day(day_of_tick(t)), t);
        }
        assert_eq!(DAYS_PER_YEAR, 360);
    }

    #[test]
    fn day_date_has_day_of_month() {
        let d = day_date(1);
        assert_eq!((d.date.year, d.date.month, d.day), (1, 1, 1));
        let d = day_date(30);
        assert_eq!((d.date.month, d.day), (1, 30));
        let d = day_date(31);
        assert_eq!((d.date.month, d.day), (2, 1));
        let d = day_date(360);
        assert_eq!((d.date.year, d.date.month, d.day), (1, 12, 30));
        let d = day_date(361);
        assert_eq!((d.date.year, d.date.month, d.day, d.date.total_year), (2, 1, 1, 2));
        assert_eq!(day_date(0), day_date(1));
    }
}
