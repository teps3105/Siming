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
