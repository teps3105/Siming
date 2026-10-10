//! GDD 3.5 修煉（每年）與步驟 11 的逐人行動順序。

use rand::Rng;

use super::{breakthrough, demon};
use crate::model::*;
use crate::params::*;
use crate::sim::World;

/// GDD 3.5 修煉的確定部分（不含 U(0.8, 1.2) 隨機係數）：
/// `根骨 × 靈氣 × 宗門 × 宿慧 × 賜福`。
///
/// - 魔修（3.10）：靈氣係數改為業氣係數 `0.5 + 2 × 所在州郡業氣`；無宗門加成。
/// - 宗門弟子 1.2，山門在洞天福地上（`seat_node`）再 × (1 + 強度 × 0.3)；散修 1.0。
/// - 遊歷期間以所在州（遊歷目的地）計。
pub fn cultivation_coefficient(w: &World, p: &Person) -> f32 {
    let root = CULT_ROOT_BASE + CULT_ROOT_COEF * p.attrs.root;
    let prov = w.provinces.get(p.province as usize);
    let env = if p.role == Role::Demonic {
        DEMON_CULT_KARMA_BASE + DEMON_CULT_KARMA_COEF * prov.map_or(0.0, |x| x.karma_qi)
    } else {
        CULT_AURA_BASE + CULT_AURA_COEF * prov.map_or(0.0, |x| x.aura)
    };
    let sect = match p.power.and_then(|id| w.powers.get(id as usize)) {
        Some(Power { kind: PowerKind::Sect(s), .. }) if p.role == Role::Cultivator => {
            let node = s
                .seat_node
                .and_then(|n| w.geo.nodes.get(n as usize))
                .map_or(1.0, |n| 1.0 + n.strength * CULT_NODE_COEF);
            CULT_SECT * node
        }
        _ => CULT_ROGUE,
    };
    let wisdom = CULT_WISDOM_BASE + w.souls.get(p.soul as usize).map_or(0.0, |s| s.wisdom) / CULT_WISDOM_DIV;
    let bless = if p.blessed_until.is_some_and(|t| t > w.tick) { BLESS_CULT_MULT } else { 1.0 };
    root * env * sect * wisdom * bless
}

/// 每年一次修煉：修為 += 係數 × U(0.8, 1.2)，上限 E。
fn cultivate(w: &mut World, pid: PersonId) {
    let noise = CULT_NOISE_MIN + (CULT_NOISE_MAX - CULT_NOISE_MIN) * w.rng.people.gen::<f32>();
    let p = &w.persons[pid as usize];
    let gain = cultivation_coefficient(w, p) * noise;
    if let PathState::Cultivator { realm, exp, last_attempt } = p.path {
        let e = REALM_EXP[realm as usize];
        w.persons[pid as usize].path = PathState::Cultivator { realm, exp: (exp + gain).min(e), last_attempt };
    }
}

/// 魔修斬殺所得修為：`+= 對方戰力 × 2`，上限 E。
pub(super) fn add_exp(w: &mut World, pid: PersonId, amount: f32) {
    if let PathState::Cultivator { realm, exp, last_attempt } = w.persons[pid as usize].path {
        let e = REALM_EXP[realm as usize];
        w.persons[pid as usize].path = PathState::Cultivator { realm, exp: (exp + amount).min(e), last_attempt };
    }
}

/// GDD 6.3 步驟 11（修士與魔修部分），依人物 ID 升冪：
/// 突破（含天劫）→ 修煉 → 心魔 → 斬妖除魔（修士）／襲殺（魔修）。
pub(super) fn yearly_actions(w: &mut World) {
    let tick = w.tick;
    let index = demon::PersonIndex::build(w);
    for i in w.alive_snapshot().into_iter().map(|x| x as usize) {
        let p = &w.persons[i];
        if !p.alive() || !matches!(p.role, Role::Cultivator | Role::Demonic) {
            continue;
        }
        if !matches!(p.path, PathState::Cultivator { .. }) {
            continue;
        }
        let pid = p.id;
        if breakthrough::eligible(p, tick) && !breakthrough::attempt(w, pid, false) {
            continue;
        }
        cultivate(w, pid);
        match w.persons[i].role {
            Role::Cultivator => {
                if demon::heart_demon_roll(w, pid) {
                    continue;
                }
                demon::hunt(w, &index, pid);
            }
            _ => demon::assassinate(w, &index, pid),
        }
    }
}
