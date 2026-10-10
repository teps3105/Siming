//! GDD 3.2 六道判定與等待條件（純函式，不碰亂數）。

use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::Tick;

/// 判定值 `x = G − S + 超度加成 + 雜訊`（GDD 3.2）。
pub fn judge_value(soul: &Soul, salvation: f32, noise: f32) -> f32 {
    soul.merit - soul.sin + salvation + noise
}

/// GDD 3.2 六道判定表（依序，先符合者優先）。
///
/// M2–M3 暫行：畜生道不論 W 一律為獸（W ≥ 30 生為妖自 M4 起，3.2）。
/// 菩薩願與引渡令不在此處理（見 `mod.rs` 的 `choose_path`）。
pub fn judge_path(soul: &Soul, x: f32) -> SixPath {
    if x >= PATH_HEAVEN_X && soul.wisdom >= PATH_HEAVEN_W {
        SixPath::Heaven
    } else if soul.sin >= PATH_ASURA_S && soul.affinity[Teaching::Demon as usize] >= PATH_ASURA_DEMON {
        SixPath::Asura
    } else if x >= PATH_HUMAN_X {
        SixPath::Human
    } else if x >= PATH_BEAST_X {
        SixPath::Beast
    } else if x >= PATH_HUNGRY_X {
        SixPath::Hungry
    } else {
        SixPath::Hell
    }
}

/// 他道停留的月數區間（含兩端）；人道、阿修羅道不停留。
pub fn realm_months(path: SixPath) -> Option<(u32, u32)> {
    let (lo, hi) = match path {
        SixPath::Heaven => (PATH_HEAVEN_YEARS_MIN, PATH_HEAVEN_YEARS_MAX),
        SixPath::Beast => (PATH_BEAST_YEARS_MIN, PATH_BEAST_YEARS_MAX),
        SixPath::Hungry => (PATH_HUNGRY_YEARS_MIN, PATH_HUNGRY_YEARS_MAX),
        SixPath::Hell => (PATH_HELL_YEARS_MIN, PATH_HELL_YEARS_MAX),
        SixPath::Human | SixPath::Asura => return None,
    };
    Some((lo * 12, hi * 12))
}

/// GDD 3.2 等待：本魂判入人道時，找出值得等的宿緣對象。
///
/// 條件：未了、強度 ≥ `WAIT_MIN_STRENGTH`，且對方處於中陰、或他道將在 `WAIT_MAX` 內期滿。
/// 對方已超脫、期滿尚遠、在等待中或在世時不等。多條符合時取強度最高者，同強度取 ThreadId 小者。
pub fn wait_target(w: &World, soul: &Soul, tick: Tick) -> Option<SoulId> {
    let horizon = tick + WAIT_MAX_YEARS * 12;
    let mut best: Option<(f32, ThreadId, SoulId)> = None;
    for &tid in &soul.threads {
        let Some(t) = w.threads.get(tid as usize) else { continue };
        if t.resolved.is_some() || t.strength < WAIT_MIN_STRENGTH {
            continue;
        }
        let other = if t.debtor == soul.id {
            t.creditor
        } else if t.creditor == soul.id {
            t.debtor
        } else {
            continue;
        };
        if other == soul.id || other == HEAVEN_SOUL {
            continue;
        }
        let Some(os) = w.souls.get(other as usize) else { continue };
        let ok = match os.state {
            SoulState::Bardo { .. } => true,
            SoulState::Realm { until, .. } => until <= horizon,
            _ => false,
        };
        if !ok {
            continue;
        }
        let better = match best {
            None => true,
            Some((s, id, _)) => t.strength > s || (t.strength == s && tid < id),
        };
        if better {
            best = Some((t.strength, tid, other));
        }
    }
    best.map(|b| b.2)
}
