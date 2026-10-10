//! GDD 6.5 隨機數子流。
//!
//! 每個系統只使用自己的子流：以同一個種子建立 ChaCha8，再以 `set_stream` 切到固定編號。
//! 新增系統時新增編號，不影響既有子流的序列。

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// 子流編號。數值一旦使用即不可更動。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum Stream {
    Worldgen = 1,
    Genesis = 2,
    Soul = 3,
    People = 4,
    Powers = 5,
    War = 6,
    Field = 7,
    Cosmos = 8,
    Command = 9,
}

pub fn stream(seed: u64, s: Stream) -> ChaCha8Rng {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    rng.set_stream(s as u64);
    rng
}

/// 模擬期間使用的子流（世界生成的子流只在生成時使用，不保存）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RngStreams {
    pub genesis: ChaCha8Rng,
    pub soul: ChaCha8Rng,
    pub people: ChaCha8Rng,
    pub powers: ChaCha8Rng,
    pub field: ChaCha8Rng,
    pub command: ChaCha8Rng,
}

impl RngStreams {
    pub fn new(seed: u64) -> Self {
        Self {
            genesis: stream(seed, Stream::Genesis),
            soul: stream(seed, Stream::Soul),
            people: stream(seed, Stream::People),
            powers: stream(seed, Stream::Powers),
            field: stream(seed, Stream::Field),
            command: stream(seed, Stream::Command),
        }
    }
}
