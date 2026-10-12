//! GDD 3.5 道：修煉與天劫、宗門；3.10 魔修（M2 部分）（U6）。
//!
//! # 步驟對應（GDD 6.3）
//!
//! | 函式 | 步驟 | 子流 | 內容 |
//! |---|---|---|---|
//! | [`monthly_forced`] | 3 | — | 開局保證的強制突破（成功率與存活率皆 1，不抽樣）；之後處理首領缺位 |
//! | [`yearly`] | 11 | `people` | 依人物 ID 升冪：突破（含天劫）→ 修煉 → 心魔 → 斬妖除魔／魔修襲殺；末尾判定「劫雲將聚」預兆 |
//! | [`yearly_sects`] | 15–18 | `powers` | 繼任檢查 → 創立 → 招收、補足、拜師 → 爭脈之戰 → 繼任與分裂 → 覆滅檢查、聲望與範圍 |
//!
//! # 給其他系統的介面
//!
//! | 函式 | 誰呼叫 | 用途 |
//! |---|---|---|
//! | [`choose_seat`]、[`found_sect`] | genesis（開天宗門，與創立共用 `SECT_SITE`）、本模組 | 選山門、建立宗門 |
//! | [`sect_name`] | genesis、本模組 | 宗門命名（`orgs.zh-Hant.json`，用 `powers` 子流） |
//! | [`join_sect`] | genesis、本模組 | 人物加入宗門（設定 `power`、`home`、位置；不拜師） |
//! | [`fall_to_demon`] | 本模組、司命「降罰：心魔」（M5） | 入魔：身分轉魔修、離開宗門、魂加印記、寫入 `FellToDemon` |
//! | [`kill`] | 本模組、war（M3） | 一次殺人的完整處理：`Killed` 事件 → `people::die` → 計業 → 殺業線 |
//! | [`breakthrough_chance`]、[`tribulation_survival`]、[`ascension_survival`]、[`cultivation_coefficient`] | 測試、UI 預覽 | 純公式 |
//! | [`sect_range`]、[`effective_prestige`]、[`resolve_leaders`] | 本模組、UI | 勢力範圍、聲望、首領缺位處理 |
//!
//! # 約定
//!
//! - 殺業線與業力透過 `thread::kill_karma`／`thread::on_kill`（U7）；拜師建線透過 `thread::on_apprentice`。
//! - `Power::prestige` 存「公式值」，有效聲望 = `prestige + prestige_mod`（[`effective_prestige`]）。
//! - 首領缺位 = `leader` 為 `None`、已死或其 `power` 已不是該宗門（入魔離宗）；由 [`resolve_leaders`] 處理。
//! - 魔修（`Role::Demonic`）M2 無魔門，恆為無勢力的散修；不投靠宗門、不創立宗門。
//! - M2 沒有妖：斬妖除魔的對象只有魔修。

mod breakthrough;
mod cultivate;
mod demon;
mod kill;
mod sect;
#[cfg(test)]
mod tests;
mod war;

pub use breakthrough::{ascension_survival, breakthrough_chance, cooldown_ready, fail_death_chance, tribulation_survival, ASCENSION_REALM};
pub use cultivate::cultivation_coefficient;
pub use demon::{fall_to_demon, heart_demon_chance};
pub use kill::kill;
pub use sect::{
    choose_seat, effective_prestige, found_sect, join_sect, provinces_within, resolve_leaders, sect_name, sect_of, sect_range, Seat,
};

use crate::model::*;
use crate::schedule::Round;
use crate::sim::World;

/// GDD 6.3 步驟 3：開局保證的強制突破（4.6）。
///
/// `forced_breakthrough <= tick` 的人物就地執行完整的突破與天劫判定（成功率與存活率皆為 1，不消耗亂數），
/// 寫入 `Breakthrough`、`Tribulation` 事件（大乘則為飛升），然後清除。依人物 ID 升冪。
pub fn monthly_forced(w: &mut World) {
    let tick = w.tick;
    let mut any_death = false;
    for i in w.alive_snapshot().into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if p.forced_breakthrough.is_none_or(|t| t > tick) {
            continue;
        }
        let pid = p.id;
        w.persons[i].forced_breakthrough = None;
        if !w.persons[i].alive() || !matches!(w.persons[i].path, PathState::Cultivator { .. }) {
            continue;
        }
        if !breakthrough::attempt(w, pid, true) {
            any_death = true;
        }
    }
    if any_death {
        sect::resolve_leaders(w);
    }
}

/// GDD 6.3 步驟 11：修煉、突破、天劫、心魔、斬妖除魔、魔修襲殺、劫雲將聚預兆。
///
/// 逐人判定，輪到判定月（見 [`crate::schedule`]）的人才處理；`Sim::step` 每月呼叫。
pub fn monthly_personal(w: &mut World) {
    personal_round(w, Round::at(w.tick));
}

/// 同 [`monthly_personal`]，但一次判定全部人（整年份；測試與工具用）。
pub fn yearly(w: &mut World) {
    personal_round(w, Round::All);
}

fn personal_round(w: &mut World, round: Round) {
    cultivate::yearly_actions(w, round);
    breakthrough::yearly_omens(w, round);
}

/// GDD 6.3 步驟 15–18：宗門創立、招收、爭脈、繼任分裂、聲望與範圍；`Sim::step` 每月呼叫。
///
/// 散修創宗與投靠是逐人判定，依人物的判定月；補足、拜師、繼任、聲望與範圍是逐宗判定，依宗門編號的判定月；
/// 爭脈是兩宗之間的判定，由編號較小的一宗的判定月帶出。
pub fn monthly_sects(w: &mut World) {
    sects_round(w, Round::at(w.tick));
}

/// 同 [`monthly_sects`]，但一次判定全部（整年份；測試與工具用）。
pub fn yearly_sects(w: &mut World) {
    sects_round(w, Round::All);
}

fn sects_round(w: &mut World, round: Round) {
    // 勢力範圍在本輪內不變（半徑只在最後的聲望重算改），招收與爭脈共用一份快取。
    let mut ranges = sect::RangeCache::default();
    sect::resolve_leaders_round(w, round);
    sect::yearly_found(w, round);
    sect::yearly_recruit(w, round, &mut ranges);
    war::yearly_wars(w, round, &mut ranges);
    sect::resolve_leaders_round(w, round);
    sect::yearly_prestige(w, round);
}
