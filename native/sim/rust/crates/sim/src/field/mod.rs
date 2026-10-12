//! GDD 6.9 場域框架：維的識別、規格表與統一讀寫入口。
//!
//! 場的儲存不另起一套結構，仍住在 `Geography` 與 `Province`；本模組只提供統一入口
//! （[`get`]、[`get_cell`]、[`add`]、[`gather`]、[`scatter`]）、傳輸運算子（[`kernel`]）與衍生場（[`derive`]）。
//!
//! `Owner` 的儲存在 `World::dynasty`（3.7），只能經 [`get`] 讀（編碼：王朝 ID + 1，0 為無主）；
//! 目前州郡尚無儲存欄位的維：`CapMod`、`Incense`、`Bond`、`KalpaLocal`、`BeastQi`，
//! 以及順流所需的 `downstream`。這些維在 [`FieldId`] 與 [`FIELD_SPECS`] 照樣列出，
//! 讀取回 0，[`add`]、[`scatter`] 不動作；待各自的里程碑補上儲存欄位後再接入。
//! `Bond` 依規格是衍生場，但其輸入（宿緣）不在 `Province`，同樣暫回 0。

pub mod derive;
pub mod kernel;

#[cfg(test)]
mod tests;

use crate::params::*;
use crate::sim::World;
use crate::types::{Province, ProvinceId};

/// GDD 2.7 場域識別。順序即存檔與工具列舉的順序；只追加、不重排。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FieldId {
    // 地：基底場（格域）
    Height = 0,
    Terrain,
    River,
    Habitability,
    Continent,
    // 冥：格域
    AuraCell,
    // 人：州域
    Population,
    CapMod,
    Devastation,
    FaithTao,
    FaithBuddha,
    FaithRu,
    Incense,
    Owner,
    // 冥：州域
    Aura,
    KarmaQi,
    Merit,
    Righteous,
    // 衍生（不保存）
    Turbid,
    Bond,
    KalpaLocal,
    BeastQi,
}

impl FieldId {
    /// 維的總數；與 [`FIELD_SPECS`] 的列數相等。
    pub const COUNT: usize = FieldId::BeastQi as usize + 1;

    /// 依 `FieldId` 順序的全部維。
    pub const ALL: [FieldId; FieldId::COUNT] = [
        FieldId::Height,
        FieldId::Terrain,
        FieldId::River,
        FieldId::Habitability,
        FieldId::Continent,
        FieldId::AuraCell,
        FieldId::Population,
        FieldId::CapMod,
        FieldId::Devastation,
        FieldId::FaithTao,
        FieldId::FaithBuddha,
        FieldId::FaithRu,
        FieldId::Incense,
        FieldId::Owner,
        FieldId::Aura,
        FieldId::KarmaQi,
        FieldId::Merit,
        FieldId::Righteous,
        FieldId::Turbid,
        FieldId::Bond,
        FieldId::KalpaLocal,
        FieldId::BeastQi,
    ];

    /// 此維在規格表中的一列。
    pub fn spec(self) -> &'static FieldSpec {
        &FIELD_SPECS[self as usize]
    }

    /// 州郡是否已有此維的儲存欄位（或可由已有欄位計算）；沒有者讀取回 0、寫入不動作。
    pub fn has_province_storage(self) -> bool {
        matches!(
            self,
            FieldId::Population
                | FieldId::Devastation
                | FieldId::FaithTao
                | FieldId::FaithBuddha
                | FieldId::FaithRu
                | FieldId::Aura
                | FieldId::KarmaQi
                | FieldId::Merit
                | FieldId::Righteous
                | FieldId::Turbid
        )
    }
}

/// 場所在的域（GDD 2.7 三種域）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDomain {
    Cell,
    Province,
    Ley,
}

/// 場的種類：連續標量、類別、衍生（不保存）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    Category,
    Derived,
}

/// 規格表的一列（GDD 2.7 表格在程式中的唯一對應；數值引用 params.rs）。
///
/// 地圖色階 `ramp` 留給 Godot 端場域烘焙（6.9「渲染取樣」），此處尚無。
#[derive(Debug, Clone, Copy)]
pub struct FieldSpec {
    pub id: FieldId,
    pub domain: FieldDomain,
    pub kind: FieldKind,
    /// 截斷範圍 `(下限, 上限)`。
    pub range: (f32, f32),
    /// 每年衰減比例；0 表示不衰減。
    pub decay_per_year: f32,
    /// 每季擴散比例；0 表示不擴散。
    pub diffuse_per_season: f32,
    /// 每季順流比例；0 表示不順流。
    pub drift_per_season: f32,
    /// 每年向目標靠攏的比例；0 表示不靠攏。
    pub relax_per_year: f32,
}

const fn spec(id: FieldId, domain: FieldDomain, kind: FieldKind, range: (f32, f32), transport: (f32, f32, f32, f32)) -> FieldSpec {
    FieldSpec {
        id,
        domain,
        kind,
        range,
        decay_per_year: transport.0,
        diffuse_per_season: transport.1,
        drift_per_season: transport.2,
        relax_per_year: transport.3,
    }
}

use FieldDomain::{Cell, Province as Prov};
use FieldKind::{Category, Derived, Scalar};

const NONE: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);
const UNIT: (f32, f32) = (0.0, 1.0);
/// 教化三維各自靠攏；「和 ≤ 1」與等比縮放由 2.3 的規則處理（GDD 6.9 識別）。
const FAITH_TRANSPORT: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, FAITH_DRIFT);

/// GDD 2.7 規格表，依 [`FieldId`] 順序。
///
/// 值域中未見於 2.7 者：`Continent` 為類別索引（0..u16 上限）；`Population` 無上限；`KalpaLocal` 暫取 0..1，
/// 待 M6 劫煞接上時再定。
pub const FIELD_SPECS: [FieldSpec; FieldId::COUNT] = [
    spec(FieldId::Height, Cell, Scalar, UNIT, NONE),
    spec(FieldId::Terrain, Cell, Category, (0.0, 8.0), NONE),
    spec(FieldId::River, Cell, Scalar, (0.0, 3.0), NONE),
    spec(FieldId::Habitability, Cell, Derived, (0.0, 2.1), NONE),
    spec(FieldId::Continent, Cell, Category, (0.0, u16::MAX as f32), NONE),
    spec(FieldId::AuraCell, Cell, Scalar, (0.0, AURA_CELL_MAX), NONE),
    spec(FieldId::Population, Prov, Scalar, (0.0, f32::MAX), NONE),
    spec(FieldId::CapMod, Prov, Category, (0.5, 1.5), NONE),
    spec(FieldId::Devastation, Prov, Scalar, UNIT, NONE),
    spec(FieldId::FaithTao, Prov, Scalar, UNIT, FAITH_TRANSPORT),
    spec(FieldId::FaithBuddha, Prov, Scalar, UNIT, FAITH_TRANSPORT),
    spec(FieldId::FaithRu, Prov, Scalar, UNIT, FAITH_TRANSPORT),
    spec(FieldId::Incense, Prov, Derived, UNIT, NONE),
    spec(FieldId::Owner, Prov, Category, (0.0, u16::MAX as f32), NONE),
    spec(FieldId::Aura, Prov, Scalar, UNIT, NONE),
    spec(FieldId::KarmaQi, Prov, Scalar, UNIT, (KARMA_DECAY, KARMA_DIFFUSE, KARMA_DRIFT, 0.0)),
    spec(FieldId::Merit, Prov, Scalar, UNIT, (MERIT_DECAY, MERIT_DIFFUSE, 0.0, 0.0)),
    spec(FieldId::Righteous, Prov, Scalar, UNIT, (RIGHT_DECAY, RIGHT_DIFFUSE, 0.0, 0.0)),
    spec(FieldId::Turbid, Prov, Derived, UNIT, NONE),
    spec(FieldId::Bond, Prov, Derived, UNIT, NONE),
    spec(FieldId::KalpaLocal, Prov, Derived, UNIT, NONE),
    spec(FieldId::BeastQi, Prov, Derived, UNIT, NONE),
];

/// 讀州郡 `p` 的某一維（州域與衍生場）；格域場與尚無儲存的維回 0。
pub fn get_of(p: &Province, id: FieldId) -> f32 {
    match id {
        FieldId::Population => p.population as f32,
        FieldId::Devastation => p.devastation,
        FieldId::FaithTao => p.faith[0],
        FieldId::FaithBuddha => p.faith[1],
        FieldId::FaithRu => p.faith[2],
        FieldId::Aura => p.aura,
        FieldId::KarmaQi => p.karma_qi,
        FieldId::Merit => p.merit,
        FieldId::Righteous => p.righteous,
        FieldId::Turbid => derive::turbid_of(p),
        _ => 0.0,
    }
}

/// 直接寫州郡 `p` 的某一維，不截斷（內部用，呼叫者負責已截斷）；衍生與尚無儲存的維不動作。
fn put(p: &mut Province, id: FieldId, value: f32) {
    match id {
        FieldId::Population => p.population = value as f64,
        FieldId::Devastation => p.devastation = value,
        FieldId::FaithTao => p.faith[0] = value,
        FieldId::FaithBuddha => p.faith[1] = value,
        FieldId::FaithRu => p.faith[2] = value,
        FieldId::Aura => p.aura = value,
        FieldId::KarmaQi => p.karma_qi = value,
        FieldId::Merit => p.merit = value,
        FieldId::Righteous => p.righteous = value,
        _ => {}
    }
}

/// 源與匯：對州郡 `p` 的某一維加 `delta`，結果依規格截斷。
///
/// 供只拿得到州郡切片、拿不到整個 `World` 的呼叫端使用（如 `population::yearly`）；[`add`] 是它的 `World` 版。
/// 人口以 f64 儲存，加總在 f64 進行，下限為 0。衍生與尚無儲存的維不動作。
pub fn add_to(p: &mut Province, id: FieldId, delta: f32) {
    if id == FieldId::Population {
        p.population = (p.population + delta as f64).max(0.0);
        return;
    }
    if !id.has_province_storage() || id.spec().kind == FieldKind::Derived {
        return;
    }
    let (lo, hi) = id.spec().range;
    put(p, id, (get_of(p, id) + delta).clamp(lo, hi));
}

/// 讀州郡的某一維（州域與衍生場）；格域場與尚無儲存的維回 0。
pub fn get(w: &World, id: FieldId, province: ProvinceId) -> f32 {
    // 歸屬（GDD 3.7）住在 `World::dynasty`：編碼為王朝 ID + 1，0 為無主。
    if id == FieldId::Owner {
        return w.dynasty.owner_of(province).map_or(0.0, |o| o as f32 + 1.0);
    }
    w.provinces.get(province as usize).map_or(0.0, |p| get_of(p, id))
}

/// 讀格 `cell` 的某一維。格域場回該格的值；州域場回所屬州的值（海格回 0）。
///
/// 格域的 `Habitability`（衍生，不保存）與 `Continent`（類別索引，無值代碼 `NO_CONTINENT`）尚未提供，回 0。
pub fn get_cell(w: &World, id: FieldId, cell: u32) -> f32 {
    let g = &w.geo;
    let i = cell as usize;
    match id {
        FieldId::Height => g.height.get(i).copied().unwrap_or(0.0),
        FieldId::Terrain => g.terrain.get(i).map_or(0.0, |t| *t as u8 as f32),
        FieldId::River => g.river.get(i).map_or(0.0, |r| *r as f32),
        FieldId::AuraCell => g.aura_cell.get(i).copied().unwrap_or(0.0),
        FieldId::Habitability | FieldId::Continent => 0.0,
        _ => match g.province_of.get(i) {
            Some(&p) if p != crate::types::NO_PROVINCE => get(w, id, p),
            _ => 0.0,
        },
    }
}

/// 源與匯：對州郡的某一維加 `delta`，結果依規格截斷。
pub fn add(w: &mut World, id: FieldId, province: ProvinceId, delta: f32) {
    if let Some(p) = w.provinces.get_mut(province as usize) {
        add_to(p, id, delta);
    }
}

/// 州域快照，索引 = `ProvinceId`（假設 `provinces[i].id == i`）。
pub fn gather(w: &World, id: FieldId) -> Vec<f32> {
    w.provinces.iter().map(|p| get_of(p, id)).collect()
}

/// 截斷後寫回州域快照；長度不符的多餘部分忽略。衍生與尚無儲存的維不動作。
///
/// 人口以 f64 儲存，經 f32 往返會損失精度；人口搬移走 `migrate`，不經 `gather`／`scatter`。
pub fn scatter(w: &mut World, id: FieldId, values: &[f32]) {
    if !id.has_province_storage() || id.spec().kind == FieldKind::Derived {
        return;
    }
    let (lo, hi) = id.spec().range;
    for (p, v) in w.provinces.iter_mut().zip(values) {
        put(p, id, v.clamp(lo, hi));
    }
}
