//! GDD 3.7 儒：王朝與天命。
//!
//! 王朝不放進 `World::powers`（那裡只有宗門，且勢力編號會與宗門交錯），而是自成一個狀態
//! [`DynastyState`]：王朝列表（`DynastyId` = 索引，滅亡的留在原位）與各州的歸屬。
//! 歸屬經 [`crate::field::get`] 的 `FieldId::Owner` 讀出（編碼為王朝 ID + 1，0 = 無主）。
//!
//! # 每年的處理順序（`Sim::step` 的 6.3 步驟 19，宗門年度處理之後）
//!
//! 王朝依 ID 升冪逐一處理，本年新生的王朝從下一年才處理；每個王朝內部依序：
//!
//! 1. [`court::prune`]：失去（沉沒）的州郡、已死或已換身分的人員從名冊移除；
//! 2. [`court::succession`]：君主死亡時，儲君繼位 → 篡位 → 內戰（無人可繼則崩潰）；
//! 3. 財政（[`finance`]、[`state_finance`]）：收入、常備軍、軍費、官俸、宮廷、賑災、國庫上限；
//! 4. 國策與詔令的年度效果（[`court::policy_effects`]、[`court::decree`]）；
//! 5. 擴張：開疆、宣戰（[`realm::expansion`]）；
//! 6. 天命 ΔM（[`mandate_delta`]）與截斷；
//! 7. 天命的效果：崩潰、割據、起義（[`realm::unrest`]）；
//! 8. 封禪（[`court::fengshan`]）；
//! 9. 文臣、武將補缺與駐地、王族生育（[`court::staff`]、[`court::royal_birth`]）；
//! 10. 關係值與聲望。
//!
//! 全部處理完後才做「立國」判定（[`realm::yearly_found`]）。亂數只用 `World::rng.dynasty`。
//!
//! # 取 0 或只做到事件的項目（規格依賴尚未實作的系統）
//!
//! - 天命的天象項（4.2）：0。
//! - 稅收的寺院香火折扣（3.6）：0；動員軍的軍費與流失（3.9）：0。
//! - 起義（3.9）：只判定並發出 `Rebellion` 事件，不出叛軍。宣戰（3.9）：只判定並發出 `WarDeclared`、記入 `wars`。
//! - 禁道的宗門之戰（3.9）、尊道的國師（3.9）、尊佛與尊儒對寺院書院的效果（3.6、3.8）、封禪的君主立功（3.8）：不做。

pub mod court;
pub mod realm;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use court::{crown, policy_of};
pub use realm::{civil_war, collapse, found, found_initial, secede};

use crate::field::{self, FieldId};
use crate::ledger::{self, NewEvent};
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::{ProvinceId, Tick};

/// 無主（`DynastyState::owner` 的空值）。
pub const NO_OWNER: DynastyId = DynastyId::MAX;

/// GDD 3.7 國策：即位時依君主道緣決定，在位不變。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Policy {
    None,
    Tao,
    Buddha,
    Ru,
}

/// GDD 3.7 詔令：頒布後鎖定到君主駕崩。滅佛屬 3.6，尚未實作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decree {
    None,
    BanTao,
}

/// GDD 3.7 王朝。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dynasty {
    pub id: DynastyId,
    /// 國號（一字，7.3）。
    pub name: String,
    pub founder: Option<PersonId>,
    /// 在位君主；已死者留在欄位上，由下一次年度處理的繼承接手。
    pub ruler: Option<PersonId>,
    /// 儲君（王族子女），不含君主。
    pub heirs: Vec<PersonId>,
    pub capital: ProvinceId,
    /// 疆域，州郡 ID 升冪。
    pub provinces: Vec<ProvinceId>,
    /// 天命 M，0..100。
    pub mandate: f32,
    /// 國庫（錢糧）。
    pub treasury: f64,
    pub policy: Policy,
    pub decree: Decree,
    pub officials: Vec<PersonId>,
    /// 武將與駐地。
    pub generals: Vec<(PersonId, ProvinceId)>,
    /// 常備軍兵力。
    pub standing_army: f64,
    /// 進行中的戰爭對手（3.9 接手；目前只記錄宣戰）。
    pub wars: Vec<DynastyId>,
    /// 歷代滅佛次數（3.6 尚未實作，恆 0）。
    pub purges: u16,
    /// 公式值：州數 × 10 + M。
    pub prestige: f32,
    /// 修正值，每年 × 0.95。
    pub prestige_mod: f32,
    pub founded: Tick,
    pub fallen: Option<Tick>,
    pub relations: BTreeMap<DynastyId, i8>,
    /// 禁道詔令下與疆域內宗門的關係（宗門 ID → 關係值）。
    pub sect_relations: BTreeMap<PowerId, i8>,
    pub ruler_since: Tick,
    /// 天命連續 ≥ 90 的年數（封禪條件）。
    pub high_years: u16,
    /// 已封禪的君主（一位君主一生一次）。
    pub fengshan_by: Option<PersonId>,
    /// 本年新得與失去的州數（天命項；每年算過天命後歸零）。
    pub gained: u16,
    pub lost: u16,
}

/// GDD 3.7 王朝與州郡歸屬。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DynastyState {
    pub list: Vec<Dynasty>,
    /// 以州郡 ID 為索引的歸屬；[`NO_OWNER`] 為無主。
    pub owner: Vec<DynastyId>,
}

impl DynastyState {
    pub fn new(provinces: usize) -> Self {
        Self { list: Vec::new(), owner: vec![NO_OWNER; provinces] }
    }

    /// 州郡的所屬王朝。
    pub fn owner_of(&self, p: ProvinceId) -> Option<DynastyId> {
        self.owner.get(p as usize).copied().filter(|&o| o != NO_OWNER)
    }

    pub fn get(&self, id: DynastyId) -> Option<&Dynasty> {
        self.list.get(id as usize)
    }

    /// 在世（未滅亡）的王朝 ID，升冪。
    pub fn alive_ids(&self) -> Vec<DynastyId> {
        self.list.iter().filter(|d| d.fallen.is_none()).map(|d| d.id).collect()
    }

    fn ensure(&mut self, provinces: usize) {
        if self.owner.len() < provinces {
            self.owner.resize(provinces, NO_OWNER);
        }
    }
}

/// 寫入王朝事件。`persons` 的魂一併記入事件。
#[allow(clippy::too_many_arguments)]
pub(crate) fn rec(
    w: &mut World,
    kind: EventKind,
    importance: u8,
    province: Option<ProvinceId>,
    persons: &[PersonId],
    dynasty: DynastyId,
    other: Option<DynastyId>,
    n: u32,
    causes: &[EventId],
) -> EventId {
    let mut e = NewEvent::new(kind, importance, EventArgs::Dynasty { dynasty, other, n });
    e.province = province;
    for &p in persons {
        e.persons.push(p);
        if let Some(pp) = w.persons.get(p as usize) {
            e.souls.push(pp.soul);
        }
    }
    e.causes.extend_from_slice(causes);
    ledger::record(w, e)
}

/// 疆域各州（未沉沒）的州域場平均值。
fn mean_field(w: &World, d: &Dynasty, id: FieldId) -> f32 {
    if d.provinces.is_empty() {
        return 0.0;
    }
    d.provinces.iter().map(|&p| field::get(w, id, p)).sum::<f32>() / d.provinces.len() as f32
}

/// GDD 3.7 財政的本年數字（單位：錢糧）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Finance {
    pub income: f64,
    pub standing: f64,
    pub military: f64,
    pub officers: f64,
    pub court: f64,
    /// 年支出 = 軍費 + 官俸 + 宮廷（不含賑災）。
    pub expenses: f64,
}

/// GDD 2.3、3.7 財政的唯讀計算：收入為疆域稅收總和（`P × 0.001 × (1 − 殘破)`，寺院香火折扣 3.6 尚無，取 0）。
pub fn finance(w: &World, di: usize) -> Finance {
    let d = &w.dynasty.list[di];
    let (mut income, mut pop) = (0.0f64, 0.0f64);
    for &p in &d.provinces {
        let pr = &w.provinces[p as usize];
        income += pr.population * TAX_RATE * (1.0 - pr.devastation as f64);
        pop += pr.population;
    }
    let standing = pop * STANDING_ARMY_RATE;
    // 動員軍（3.9）尚無，軍費只算常備軍。
    let military = standing / 1000.0 * UPKEEP_TROOP_PER_1000;
    let officers = (d.officials.len() + d.generals.len()) as f64 * UPKEEP_OFFICER;
    let court = income * UPKEEP_COURT;
    Finance { income, standing, military, officers, court, expenses: military + officers + court }
}

/// GDD 3.7、3.9 軍力 = 常備軍 + 可動員兵力（`P × 0.02 × min(1, M/100 + 0.5)` 的總和）。
pub fn military_power(w: &World, di: usize) -> f64 {
    let d = &w.dynasty.list[di];
    let k = (d.mandate as f64 / 100.0 + MOBILIZE_MANDATE_OFFSET).min(1.0);
    let pop: f64 = d.provinces.iter().map(|&p| w.provinces[p as usize].population).sum();
    pop * STANDING_ARMY_RATE + pop * MOBILIZE_RATE * k
}

/// GDD 3.7 天命每年的 ΔM（未截斷）。`famine_n` 為本年疆域內的饑荒州數，`broke` 為國庫不足。
/// 天象（4.2）立即 ±15 的項尚無來源，取 0；篡位 −20 在繼位時另行扣除。
pub fn mandate_delta(w: &World, di: usize, famine_n: u32, broke: bool) -> f32 {
    let d = &w.dynasty.list[di];
    let mut m = 0.0;
    if let Some(r) = d.ruler.and_then(|r| w.persons.get(r as usize)) {
        m += MANDATE_VIRTUE_COEF * (r.attrs.virtue - MANDATE_VIRTUE_MID);
    }
    let virtues: Vec<f32> = d
        .officials
        .iter()
        .filter_map(|&o| w.persons.get(o as usize))
        .filter(|p| p.alive())
        .map(|p| p.attrs.virtue)
        .collect();
    if !virtues.is_empty() {
        m += MANDATE_OFFICIAL_COEF * (virtues.iter().sum::<f32>() / virtues.len() as f32 - MANDATE_VIRTUE_MID);
    }
    m += MANDATE_RIGHT_COEF * mean_field(w, d, FieldId::Righteous);
    m += MANDATE_KARMA_COEF * mean_field(w, d, FieldId::KarmaQi);
    m += MANDATE_AURA_COEF * (field::get(w, FieldId::Aura, d.capital) - MANDATE_AURA_MID);
    m += MANDATE_FAMINE_COEF * famine_n as f32;
    m += MANDATE_GAIN_COEF * d.gained as f32 + MANDATE_LOSS_COEF * d.lost as f32;
    let years = w.tick.saturating_sub(d.founded) as f32 / 12.0;
    m -= (years / MANDATE_CYCLE_YEARS).min(MANDATE_CYCLE_MAX);
    if broke {
        m += MANDATE_BROKE;
    }
    m
}

/// 各州人口成長率倍率（盛世疆域 × `POP_GROWTH_PROSPEROUS_MULT`，GDD 3.7）；索引 = 州郡 ID。
pub fn growth_mult(w: &World) -> Vec<f64> {
    let mut v = vec![1.0; w.provinces.len()];
    for d in w.dynasty.list.iter().filter(|d| d.fallen.is_none() && d.mandate >= GOLDEN_AGE_MANDATE) {
        for &p in &d.provinces {
            if let Some(x) = v.get_mut(p as usize) {
                *x = POP_GROWTH_PROSPEROUS_MULT;
            }
        }
    }
    v
}

/// 財政的年度結算（會改變國庫、人口與功德）：賑災 → 收支 → 國庫上限。回傳本年數字與是否國庫不足。
fn state_finance(w: &mut World, di: usize, famines: &[(ProvinceId, u32)]) -> (Finance, bool) {
    let fin = finance(w, di);
    let id = di as DynastyId;
    let (mandate, treasury) = {
        let d = &w.dynasty.list[di];
        (d.mandate, d.treasury)
    };
    let mut spent = 0.0;
    let mut stock = treasury;
    // 賑災：天命 ≥ 40 且國庫 > 一年支出。
    if mandate >= RELIEF_MANDATE && treasury > fin.expenses {
        // 主事文臣：德行 + 才略最高者（同值取 ID 小者）。
        let chief = w.dynasty.list[di]
            .officials
            .iter()
            .copied()
            .filter(|&o| w.persons[o as usize].alive())
            .max_by(|&a, &b| {
                let (pa, pb) = (&w.persons[a as usize], &w.persons[b as usize]);
                (pa.attrs.virtue + pa.attrs.talent).total_cmp(&(pb.attrs.virtue + pb.attrs.talent)).then(b.cmp(&a))
            });
        for &(p, deaths) in famines {
            let cost = deaths as f64 * RELIEF_SPEND_PER_DEATH;
            if cost > stock {
                break;
            }
            stock -= cost;
            spent += cost;
            let saved = deaths as f64 * RELIEF_SAVED_RATIO;
            field::add(w, FieldId::Population, p, saved as f32);
            if let Some(c) = chief {
                let soul = w.persons[c as usize].soul;
                w.souls[soul as usize].merit += (saved / MERIT_RELIEF_PER_LIVES as f64) as f32;
            }
            rec(w, EventKind::Relief, 2, Some(p), &[], id, None, saved as u32, &[]);
        }
    }
    let cap = fin.income * TREASURY_CAP_YEARS;
    let d = &mut w.dynasty.list[di];
    d.standing_army = fin.standing;
    d.treasury = (treasury + fin.income - fin.expenses - spent).min(cap);
    let broke = d.treasury < fin.expenses;
    (fin, broke)
}

/// GDD 6.3 步驟 19：全部在世王朝的年度處理，然後是立國判定。
pub fn yearly(w: &mut World, famines: &[(ProvinceId, u32)]) {
    let n_prov = w.provinces.len();
    w.dynasty.ensure(n_prov);
    let n = w.dynasty.list.len();
    for di in 0..n {
        if w.dynasty.list[di].fallen.is_none() {
            year_of(w, di, famines);
        }
    }
    realm::yearly_found(w);
}

fn year_of(w: &mut World, di: usize, famines: &[(ProvinceId, u32)]) {
    court::prune(w, di);
    if w.dynasty.list[di].provinces.is_empty() {
        collapse(w, di);
        return;
    }
    court::succession(w, di);
    if w.dynasty.list[di].fallen.is_some() {
        return;
    }
    let id = di as DynastyId;
    let fam: Vec<(ProvinceId, u32)> =
        famines.iter().copied().filter(|&(p, _)| w.dynasty.owner_of(p) == Some(id)).collect();
    let (fin, broke) = state_finance(w, di, &fam);
    court::policy_effects(w, di);
    court::decree(w, di);
    realm::expansion(w, di, &fin);

    let delta = mandate_delta(w, di, fam.len() as u32, broke);
    let d = &mut w.dynasty.list[di];
    d.mandate = (d.mandate + delta).clamp(0.0, MANDATE_MAX);
    d.gained = 0;
    d.lost = 0;
    d.high_years = if d.mandate >= FENGSHAN_MANDATE { d.high_years + 1 } else { 0 };
    if d.mandate <= 0.0 {
        collapse(w, di);
        return;
    }
    realm::unrest(w, di, broke);
    if w.dynasty.list[di].fallen.is_some() {
        return;
    }
    court::fengshan(w, di);
    court::staff(w, di);
    court::royal_birth(w, di);
    realm::update_relations(w, di);
    let d = &mut w.dynasty.list[di];
    d.prestige = d.provinces.len() as f32 * DYN_PRESTIGE_PER_PROVINCE + d.mandate;
    d.prestige_mod *= PRESTIGE_MOD_DECAY;
}
