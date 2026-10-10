//! GDD 2.6 元會運世曆法（T4 實作）。
//!
//! 採邵雍《皇極經世》的元會運世：1 月 = 1 tick；12 月 = 1 年；30 年 = 1 世；
//! 12 世 = 1 運（360 年）；30 運 = 1 會（10,800 年）；12 會 = 1 元。
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

const DIGITS: [&str; 10] = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];

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
    Date {
        hui,
        yun,
        shi,
        year,
        month,
        total_year,
    }
}

/// 將 1..=9999 的整數轉為中文數字（GDD 2.6 日期顯示）。
///
/// 規則：10 → 十、11 → 十一、20 → 二十、101 → 一百零一、110 → 一百一十、
/// 1000 → 一千、1203 → 一千二百零三。
/// 超過 9999 之值為本實作的延伸（以「萬」表示），GDD 未規定。
pub fn zh_num(n: u32) -> String {
    if n == 0 {
        return DIGITS[0].to_string();
    }
    if n >= 10_000 {
        let high = n / 10_000;
        let rest = n % 10_000;
        let mut out = zh_num(high);
        out.push('萬');
        if rest != 0 {
            if rest < 1000 {
                out.push('零');
            }
            out.push_str(&zh_num(rest));
        }
        return out;
    }

    const UNITS: [&str; 4] = ["", "十", "百", "千"];
    let digits = [n / 1000 % 10, n / 100 % 10, n / 10 % 10, n % 10];
    let mut out = String::new();
    let mut pending_zero = false;
    for (i, &d) in digits.iter().enumerate() {
        let pos = 3 - i; // 3 = 千、2 = 百、1 = 十、0 = 個
        let d = d as usize;
        if d == 0 {
            if !out.is_empty() {
                pending_zero = true;
            }
            continue;
        }
        if pending_zero {
            out.push_str(DIGITS[0]);
            pending_zero = false;
        }
        // 「十」「十X」開頭時省略前導「一」。
        if !(d == 1 && pos == 1 && out.is_empty()) {
            out.push_str(DIGITS[d]);
        }
        out.push_str(UNITS[pos]);
    }
    out
}

/// 完整日期（GDD 2.6）：「第一會 第三運 第七世 第二十一年 三月」。
pub fn full(tick: Tick) -> String {
    let d = date(tick);
    format!(
        "第{}會 第{}運 第{}世 第{}年 {}月",
        zh_num(d.hui),
        zh_num(d.yun),
        zh_num(d.shi),
        zh_num(d.year),
        zh_num(d.month),
    )
}

/// 簡短日期，供頂欄使用（GDD 2.6）：「三運七世 廿一年 三月」。
///
/// 年份數字於 20–29 用「廿」，30 用「卅」；其餘數字（運、世、月）一律用一般寫法。
pub fn short(tick: Tick) -> String {
    let d = date(tick);
    format!(
        "{}運{}世 {}年 {}月",
        zh_num(d.yun),
        zh_num(d.shi),
        short_year(d.year),
        zh_num(d.month),
    )
}

fn short_year(n: u32) -> String {
    match n {
        20..=29 => {
            let mut s = String::from("廿");
            if !n.is_multiple_of(10) {
                s.push_str(DIGITS[(n % 10) as usize]);
            }
            s
        }
        30 => "卅".to_string(),
        _ => zh_num(n),
    }
}

/// 天書與卡片中的年份標記：「第 N 年」（自開天起，Arabic 數字；GDD 2.6）。
pub fn year_label(tick: Tick) -> String {
    format!("第 {} 年", date(tick).total_year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_0_and_1_are_first_month() {
        let expected = Date {
            hui: 1,
            yun: 1,
            shi: 1,
            year: 1,
            month: 1,
            total_year: 1,
        };
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

    #[test]
    fn zh_num_cases() {
        let cases = [
            (1, "一"),
            (9, "九"),
            (10, "十"),
            (11, "十一"),
            (20, "二十"),
            (21, "二十一"),
            (99, "九十九"),
            (100, "一百"),
            (101, "一百零一"),
            (110, "一百一十"),
            (1000, "一千"),
            (1010, "一千零一十"),
            (1203, "一千二百零三"),
            (9999, "九千九百九十九"),
        ];
        for (n, s) in cases {
            assert_eq!(zh_num(n), s, "zh_num({n})");
        }
    }

    #[test]
    fn short_form_uses_nian_and_sa_for_year() {
        // 年 21 → 廿一，年 30 → 卅。
        let t21 = 12 * 20 + 3; // 第 21 年、三月（年 idx 20）
        assert_eq!(short(t21), "一運一世 廿一年 三月");
        let t30 = 12 * 29 + 1; // 年 30、一月
        assert_eq!(short(t30), "一運一世 卅年 一月");
        let t25 = 12 * 24 + 1;
        assert_eq!(short_year(25), "廿五");
        assert_eq!(date(t25).year, 25);
        assert_eq!(short_year(20), "廿");
    }

    #[test]
    fn short_form_example_three_yun_seven_shi() {
        // 三運七世 廿一年 三月：yun 3 (idx 2), shi 7 (idx 6), year 21, month 3。
        let elapsed = 2 * 4320 + 6 * 360 + 20 * 12 + 2;
        let tick = elapsed + 1;
        assert_eq!(short(tick), "三運七世 廿一年 三月");
    }

    #[test]
    fn full_form_example() {
        let elapsed = 2 * 4320 + 6 * 360 + 20 * 12 + 2;
        let tick = elapsed + 1;
        assert_eq!(full(tick), "第一會 第三運 第七世 第二十一年 三月");
    }

    #[test]
    fn year_label_is_total_year() {
        assert_eq!(year_label(0), "第 1 年");
        assert_eq!(year_label(13), "第 2 年");
        assert_eq!(year_label(361), "第 31 年");
    }
}
