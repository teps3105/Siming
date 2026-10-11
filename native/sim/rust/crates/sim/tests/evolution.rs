//! GDD 2.8 天地演變：每世結算接在推演流程上。

use siming_sim::{Sim, WorldParams};

#[test]
fn gdd_2_8_slow_variables_settle_once_per_shi() {
    let mut sim = Sim::new(&WorldParams::new(20261011)).unwrap();
    let at_creation = sim.world().evolution.clone();
    assert_eq!(at_creation.sea_shift, 0.0, "開局的海岸就是生成時的海岸");
    while sim.world().tick < 359 {
        sim.step();
    }
    assert_eq!(sim.world().evolution, at_creation, "一世之內不變");
    sim.step();
    let first = sim.world().evolution.clone();
    assert_ne!(first.tide, at_creation.tide, "第 360 個月結算一次");
    assert_eq!(
        first.warmth_periods, at_creation.warmth_periods,
        "週期不隨結算改變"
    );
    while sim.world().tick < 719 {
        sim.step();
    }
    assert_eq!(sim.world().evolution, first);
    sim.step();
    assert_ne!(sim.world().evolution, first, "第 720 個月再結算一次");
}
