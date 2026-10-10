//! GDD 3.4 戰力：所有身分共用的個人戰力。

use crate::model::*;
use crate::params::*;
use crate::sim::World;

/// GDD 3.5 小境界層數：`min(10, floor(修為 / E × 10) + 1)`，1..10。
pub fn layer(realm: u8, exp: f32) -> u32 {
    let e = REALM_EXP[(realm as usize).min(REALM_EXP.len() - 1)];
    (((exp / e) * LAYER_SCALE).floor().max(0.0) as u32 + 1).min(LAYER_MAX)
}

/// 個人戰力（GDD 3.4）。已死者、幼年者為 0（幼年者不參與衝突）。
///
/// | 身分 | 戰力 |
/// |---|---|
/// | 修士、魔修、妖（`Cultivator` 狀態） | `3^境界 × (1 + 層/10)` |
/// | 僧 | `2 × 3^果位` |
/// | 武將 | `3 + 才略` |
/// | 君主 | `2 + 才略 × 0.5` |
/// | 其他（含一般儒士） | `1 + 才略 × 0.2` |
///
/// 大儒（27，浩然正氣）依賴 3.8 的大儒判定，M4 實作書院時在此加入。
pub fn combat_power(p: &Person) -> f32 {
    if !p.alive() || p.role == Role::Child {
        return 0.0;
    }
    match (&p.path, p.role) {
        (PathState::Cultivator { realm, exp, .. }, _) => {
            ipow(POWER_CULTIVATOR_BASE, *realm as u32) * (1.0 + layer(*realm, *exp) as f32 / POWER_LAYER_DIV)
        }
        (PathState::Monk { fruit, .. }, _) => POWER_MONK_BASE * ipow(POWER_MONK_FRUIT_BASE, *fruit as u32),
        (_, Role::General) => POWER_GENERAL_BASE + p.attrs.talent * POWER_GENERAL_TALENT_COEF,
        (_, Role::Ruler) => POWER_MONARCH_BASE + p.attrs.talent * POWER_MONARCH_TALENT_COEF,
        _ => POWER_OTHER_BASE + p.attrs.talent * POWER_OTHER_TALENT_COEF,
    }
}

/// 作為被襲擊方的戰力（君主 × 3 護衛，GDD 3.4）。
pub fn combat_power_as_target(p: &Person) -> f32 {
    let base = combat_power(p);
    if p.role == Role::Ruler {
        base * POWER_MONARCH_ASSAULT_MULT
    } else {
        base
    }
}

/// 以 ID 查詢的便利版；不存在者為 0。
pub fn power_of(w: &World, pid: PersonId) -> f32 {
    w.persons.get(pid as usize).map_or(0.0, combat_power)
}

/// 整數次方以連乘計算（6.5：不用 `powi`，避免跨平台差異）。
fn ipow(base: f32, n: u32) -> f32 {
    (0..n).fold(1.0, |acc, _| acc * base)
}
