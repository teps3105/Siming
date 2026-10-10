//! GDD 4.2、4.7 指令套用（U9）。
use crate::model::Command;
use crate::sim::World;

pub fn apply(w: &mut World, cmd: &Command) {
    match *cmd {
        Command::Follow { soul, on } => {
            if on {
                w.follows.insert(soul);
            } else {
                w.follows.remove(&soul);
            }
        }
    }
}
