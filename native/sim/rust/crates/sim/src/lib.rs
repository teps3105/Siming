//! GDD 6.1 siming-sim：模擬核心。純 Rust 函式庫，無引擎、無 IO、確定性（GDD 1.4 規則 5、6）。

pub mod book;
pub mod calendar;
pub mod command;
pub mod dao;
pub mod era;
pub mod evolution;
pub mod genesis;
pub mod hash;
pub mod landform;
mod hash_ser;
pub mod ledger;
pub mod model;
pub mod params;
pub mod people;
pub mod population;
pub mod rng;
pub mod save;
pub mod sim;
pub mod soul;
pub mod thread;
pub mod types;
pub mod worldgen;

pub use sim::{Sim, World};
pub use model::*;
pub use types::*;
