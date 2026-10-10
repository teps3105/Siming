//! GDD 3.4 出生：屬性、壽元、姓名。

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use super::names::person_name;
use crate::model::*;
use crate::params::*;
use crate::sim::World;
use crate::types::ProvinceId;

/// 7.1 `MORTAL_LIFESPAN`：N(62, 12) 截斷到 30..100。
/// Marsaglia 極座標法：只用 `ln`、`sqrt`（6.5 允許的浮點函式），不用三角函數。
pub(super) fn mortal_lifespan(rng: &mut ChaCha8Rng) -> u16 {
    let z = loop {
        let x = 2.0 * rng.gen::<f64>() - 1.0;
        let y = 2.0 * rng.gen::<f64>() - 1.0;
        let r2 = x * x + y * y;
        if r2 > 0.0 && r2 < 1.0 {
            break x * (-2.0 * r2.ln() / r2).sqrt();
        }
    };
    let years = MORTAL_LIFESPAN_MEAN as f64 + MORTAL_LIFESPAN_STD as f64 * z;
    (years.round() as i64).clamp(MORTAL_LIFESPAN_MIN as i64, MORTAL_LIFESPAN_MAX as i64) as u16
}

fn uniform(rng: &mut ChaCha8Rng, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * rng.gen::<f32>()
}

/// GDD 3.4：屬性上限 10。
const ATTR_MAX: f32 = 10.0;

/// GDD 3.4 屬性：根骨、悟性 `U(0,6) + W/40`；才略 `U(0,8)`；德行 `U(2,8) + (G−S)/100`；
/// 戾氣 `U(0,6)` + 修羅印記 4。截斷 0..10。亂數消耗順序固定：根骨、悟性、才略、德行、戾氣。
pub fn roll_attrs(rng: &mut ChaCha8Rng, soul: &Soul) -> Attrs {
    let c = |x: f32| x.clamp(0.0, ATTR_MAX);
    let root = uniform(rng, ATTR_ROOT_MIN, ATTR_ROOT_MAX) + soul.wisdom / ATTR_ROOT_WISDOM_DIV;
    let insight = uniform(rng, ATTR_INSIGHT_MIN, ATTR_INSIGHT_MAX) + soul.wisdom / ATTR_INSIGHT_WISDOM_DIV;
    let talent = uniform(rng, ATTR_TALENT_MIN, ATTR_TALENT_MAX);
    let virtue = uniform(rng, ATTR_VIRTUE_MIN, ATTR_VIRTUE_MAX) + soul.net_karma() / ATTR_VIRTUE_NET_DIV;
    let mut ferocity = uniform(rng, ATTR_FEROCITY_MIN, ATTR_FEROCITY_MAX);
    if soul.marks.has(SoulMarks::ASURA) {
        ferocity += ATTR_FEROCITY_ASURA_BONUS;
    }
    Attrs { root: c(root), insight: c(insight), talent: c(talent), virtue: c(virtue), ferocity: c(ferocity) }
}

/// GDD 3.4 出生：建立年齡 0 的有名之人（`Role::Child`），抽屬性、性別、姓名、凡人壽元。
/// 呼叫端（`soul::rebirth`、`soul::spawn_new_life`）須已完成宿慧傳承與修羅印記設定（屬性吃 W、G−S、印記）；
/// 之後由 `soul::attach_person` 綁定到魂。不寫事件。
pub fn spawn(w: &mut World, soul: SoulId, province: ProvinceId, cell: u32) -> PersonId {
    let id = w.persons.len() as PersonId;
    w.alive_ids.push(id);
    let s = w.souls.get(soul as usize).cloned().expect("spawn: soul");
    let attrs = roll_attrs(&mut w.rng.people, &s);
    let gender = if w.rng.people.gen::<bool>() { Gender::Male } else { Gender::Female };
    let name = person_name(&mut w.rng.people, gender);
    let lifespan = mortal_lifespan(&mut w.rng.people);
    w.persons.push(Person {
        id,
        soul,
        name,
        title: None,
        gender,
        born: w.tick as i32,
        died: None,
        home: province,
        province,
        cell,
        role: Role::Child,
        power: None,
        attrs,
        lifespan,
        spouse: None,
        master: None,
        disciples: Vec::new(),
        children: Vec::new(),
        path: PathState::Child,
        blessed_until: None,
        travel_until: None,
        forced_breakthrough: None,
        last_event: None,
    });
    id
}
