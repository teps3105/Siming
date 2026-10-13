//! GDD 6.1 siming-sim：模擬核心。純 Rust 函式庫，無引擎、無 IO、確定性（GDD 1.4 規則 5、6）。

pub mod book;
pub mod calendar;
pub mod cataclysm;
pub mod command;
pub mod dao;
pub mod dynasty;
pub mod era;
pub mod evolution;
pub mod field;
pub mod flora;
pub mod genesis;
pub mod hash;
mod hash_ser;
pub mod landform;
pub mod ledger;
pub mod model;
pub mod params;
pub mod people;
pub mod population;
pub mod rng;
pub mod save;
pub mod schedule;
pub mod sim;
pub mod soul;
pub mod stats;
pub mod thread;
pub mod types;
pub mod worldgen;

pub use model::*;
pub use sim::{Sim, World};
pub use types::*;
