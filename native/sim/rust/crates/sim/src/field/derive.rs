//! GDD 6.9 衍生場：不保存、不雜湊，讀取時由儲存欄位計算的純函數。

use crate::params::{TURBID_MIX_MERIT, TURBID_MIX_RIGHT};
use crate::types::Province;

/// 濁氣 `clamp(業氣 − 0.5 功德 − 0.5 正氣, 0, 1)`（GDD 2.7）。
pub fn turbid(karma_qi: f32, merit: f32, righteous: f32) -> f32 {
    (karma_qi - TURBID_MIX_MERIT * merit - TURBID_MIX_RIGHT * righteous).clamp(0.0, 1.0)
}

/// 州郡的濁氣。
pub fn turbid_of(p: &Province) -> f32 {
    turbid(p.karma_qi, p.merit, p.righteous)
}
