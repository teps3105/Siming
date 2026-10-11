//! GDD 6.5 確定性測試。

use siming_sim::{Sim, WorldParams};

#[test]
fn gdd_6_5_same_seed_same_hash_after_one_yun() {
    let params = WorldParams::new(12345);
    let mut a = Sim::new(&params).unwrap();
    let mut b = Sim::new(&params).unwrap();
    assert_eq!(a.hash(), b.hash());
    for _ in 0..4320 {
        a.step();
        b.step();
    }
    assert_eq!(a.hash(), b.hash());
}

/// GDD 6.5 規則 4：跨平台差異以固定的期望雜湊把關——在任何平台跑這個測試，值都要相同。
/// 模擬規則或世界生成有意改動時，這個值會變：確認改動是預期的，再更新常數。
#[test]
fn gdd_6_5_golden_hash_is_stable_across_platforms() {
    let mut sim = Sim::new(&WorldParams::new(20261011)).unwrap();
    assert_eq!(sim.world().tick, 120, "開天後預跑十年");
    assert_eq!(format!("{:016x}", sim.hash()), GOLDEN_AT_CREATION);
    for _ in 0..360 {
        sim.step();
    }
    assert_eq!(format!("{:016x}", sim.hash()), GOLDEN_AFTER_30_YEARS);
}

/// 種子 20261011、預設尺寸：開天並預跑後（tick 120）與再推進 360 個月後（tick 480）的世界雜湊。
const GOLDEN_AT_CREATION: &str = "9f4f0b1b311aae3f";
const GOLDEN_AFTER_30_YEARS: &str = "77c5f774e02c4d48";

