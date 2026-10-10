//! GDD 3.4 有名之人、3.5 遊歷（U5）。
//!
//! 亂數只用 `World::rng.people`；同一步內依人物 ID 升冪處理（GDD 6.3、6.5）。
//!
//! # 給其他系統的介面
//!
//! | 函式 | 誰呼叫 | 用途 |
//! |---|---|---|
//! | [`spawn`] | `soul::rebirth`（輪迴、新魂） | 建立年齡 0 的有名之人（屬性、性別、姓名、壽元） |
//! | [`enter_path`] | genesis（4.6 年齡 ≥ 12 者）、本模組入道 | 依 3.4 入道規則直接定身分；不寫事件、不拜師 |
//! | [`die`] | dao（突破失敗、天劫、飛升、斬妖）、war、thread（衝突） | 死亡處理（6.3「死亡」）；超脫以 `liberation` 指定 |
//! | [`combat_power`]、[`combat_power_as_target`] | thread（衝突）、dao（斬妖、魔修、爭脈） | 個人戰力 |
//! | [`marry`] | genesis（開局夫妻）、本模組婚配 | 結為夫妻並結算情緣線 |
//! | [`assign_title`] | genesis（開天時 ≥ 金丹者）、dao（突破到新境界 ≥ 金丹） | 8.3 道號：核心 + 依境界的尊稱 |
//! | [`monthly_due`]、[`yearly_aging`]、[`yearly_travel`]、[`yearly_coming_of_age`]、[`yearly_marriage`] | `Sim::step` | 6.3 步驟 3、10、11、12、13 |
//!
//! # 約定
//!
//! - 強制突破（`forced_breakthrough`，6.3 步驟 3）由 `dao::monthly_forced` 執行，不在本模組。
//! - 宗門首領死亡時，[`die`] 只移除成員，不改 `leader`（繼承由 `dao` 處理）。
//! - 師徒線與拜師（3.5「拜師」）由 `dao` 的招收步驟處理；入道時只設定 `power` 與 `members`。
//! - 殺業計算與殺業線由衝突的呼叫端負責（`die` 不計業）。

mod combat;
mod death;
mod marriage;
mod names;
mod path;
mod spawn;
#[cfg(test)]
mod tests;
mod travel;

pub use combat::{combat_power, combat_power_as_target, layer, power_of};
pub use death::{die, yearly_aging};
pub use marriage::{marry, yearly_marriage};
pub use names::{person_name, title_core, title_suffix};
pub use path::{enter_path, sect_capacity, yearly_coming_of_age};
pub use spawn::{roll_attrs, spawn};
pub use travel::{travel_candidates, yearly_travel};

use crate::model::*;
use crate::params::SECT_FOUND_MIN_REALM;
use crate::sim::World;

/// GDD 6.3 步驟 3：賜福到期、遊歷返回。依人物 ID 升冪。
pub fn monthly_due(w: &mut World) {
    let tick = w.tick;
    for i in w.alive_snapshot().into_iter().map(|x| x as usize) {
        let p = &mut w.persons[i];
        if !p.alive() {
            continue;
        }
        if p.blessed_until.is_some_and(|t| t <= tick) {
            p.blessed_until = None;
        }
        if p.travel_until.is_some_and(|t| t <= tick) {
            travel::return_home(w, i);
        }
    }
}

/// GDD 8.3 道號（金丹起）：境界 ≥ 金丹的修士取得「兩字核心 + 依境界的尊稱」。
/// 已有道號者核心不變，只依新境界重抽尊稱。非修士或境界不足者不做事。
///
/// 不消耗任何子流：亂數由（世界種子、人物 ID、用途、境界）雜湊成的臨時 ChaCha8 取得，
/// 因此強制突破（4.6、3.5：不抽樣）取得道號時不影響 `people` 子流，結果也不隨呼叫順序改變。
///
/// 正邪：魔修、邪道宗門成員、無宗門而淨業力 < 0 者取魔門字庫，其餘正道；男陽女陰。
pub fn assign_title(w: &mut World, pid: PersonId) {
    let p = &w.persons[pid as usize];
    let PathState::Cultivator { realm, .. } = p.path else { return };
    if realm < SECT_FOUND_MIN_REALM {
        return;
    }
    let gender = p.gender;
    let evil = match (p.role, p.power) {
        (Role::Demonic, _) => true,
        (_, Some(id)) => matches!(w.powers.get(id as usize).map(|pw| &pw.kind), Some(PowerKind::Sect(s)) if s.evil),
        (_, None) => w.souls[p.soul as usize].net_karma() < 0.0,
    };
    let seed = w.params.seed;
    let core: String = match &p.title {
        Some(t) => t.chars().take(2).collect(),
        None => names::title_core(&mut title_rng(seed, pid, 0), gender, evil),
    };
    let suffix = names::title_suffix(&mut title_rng(seed, pid, 1 + realm as u64), realm);
    w.persons[pid as usize].title = Some(format!("{core}{suffix}"));
}

/// 道號用的臨時亂數：以（種子、人物 ID、用途）做 splitmix64 雜湊。`salt` 0 為核心，1 + 境界為尊稱。
fn title_rng(seed: u64, pid: PersonId, salt: u64) -> rand_chacha::ChaCha8Rng {
    use rand::SeedableRng;
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(pid as u64 + 1) ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    rand_chacha::ChaCha8Rng::seed_from_u64(x)
}
