//! GDD 6.5 存檔、重播與確定性（U9）。
//!
//! 開天（U8）未接上前，`Sim::new` 的世界沒有人；這裡另以 `populated` 建立一個有 400 名魂與肉身的世界，
//! 讓出生、入道、婚配、修煉、突破、轉世、宿緣、因果帳都真的跑起來。

use siming_sim::model::{Command, SoulOrigin};
use siming_sim::save::{SaveError, SaveFile};
use siming_sim::types::{WorldGenError, WorldParams};
use siming_sim::{soul, Sim};

const YUN: u32 = 4320;

fn populated(p: &WorldParams) -> Result<Sim, WorldGenError> {
    let g = siming_sim::worldgen::generate(p)?;
    let mut sim = Sim::from_parts(g.params, g.geo, g.provinces);
    let w = sim.world_mut();
    w.tick = 1200;
    soul::ensure_heaven(w);
    let n = w.provinces.len() as u16;
    for i in 0..400u32 {
        let prov = (i * 7 % n as u32) as u16;
        let (_, pid) = soul::spawn_new_life(w, SoulOrigin::Primordial, prov);
        w.persons[pid as usize].born = w.tick as i32 - ((16 + i % 30) * 12) as i32;
    }
    Ok(sim)
}

fn params(seed: u64) -> WorldParams {
    WorldParams::new(seed)
}

#[test]
fn two_runs_same_seed_same_hash_at_every_yun_boundary() {
    let mut a = populated(&params(777)).unwrap();
    let mut b = populated(&params(777)).unwrap();
    assert_eq!(a.hash(), b.hash());
    for _ in 0..3 {
        for _ in 0..YUN {
            a.step();
            b.step();
        }
        assert_eq!(a.hash(), b.hash());
        assert_eq!(a.world().tick % YUN, 1200 % YUN);
    }
    // 世界確實在演化（不是兩個空世界）。
    assert!(a.world().ledger.events.len() > 400);
}

#[test]
fn different_seed_differs() {
    let mut a = populated(&params(777)).unwrap();
    let mut b = populated(&params(4242)).unwrap();
    for _ in 0..120 {
        a.step();
        b.step();
    }
    assert_ne!(a.hash(), b.hash());
}

#[test]
fn save_load_replay_matches_and_continues_identically() {
    let mut a = populated(&params(4242)).unwrap();
    for _ in 0..600 {
        a.step();
    }
    a.apply(Command::Follow { soul: 5, on: true });
    for _ in 0..300 {
        a.step();
    }
    a.apply(Command::Follow { soul: 9, on: true });
    a.apply(Command::Follow { soul: 5, on: false });
    for _ in 0..7 {
        a.step();
    }
    // 存檔時還有一筆排隊中的指令。
    a.apply(Command::Follow { soul: 11, on: true });
    let file = a.save();
    assert_eq!(file.commands.len(), 4);

    let json = file.to_json();
    let parsed = SaveFile::from_json(&json).unwrap();
    assert_eq!(parsed, file);
    let mut b = Sim::load_with(&parsed, populated).unwrap();
    assert_eq!(b.hash(), a.hash());
    assert_eq!(b.world().tick, a.world().tick);
    assert_eq!(b.queued_commands(), a.queued_commands());

    for i in 0..1500 {
        a.step();
        b.step();
        if i % 100 == 0 {
            assert_eq!(a.hash(), b.hash(), "tick {}", a.world().tick);
        }
    }
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.command_log(), b.command_log());
    assert!(a.world().follows.contains(&9) && a.world().follows.contains(&11) && !a.world().follows.contains(&5));
}

#[test]
fn tampered_save_is_rejected() {
    let mut a = populated(&params(9)).unwrap();
    for _ in 0..50 {
        a.step();
    }
    let mut file = a.save();
    file.hash ^= 1;
    assert!(matches!(Sim::load_with(&file, populated), Err(SaveError::Desync { .. })));
    let mut file = a.save();
    file.sim_version += 1;
    assert!(matches!(Sim::load_with(&file, populated), Err(SaveError::Version { .. })));
    let mut file = a.save();
    file.magic = *b"XXXX";
    assert!(matches!(Sim::load_with(&file, populated), Err(SaveError::BadMagic)));
    assert!(matches!(SaveFile::from_json("{"), Err(SaveError::Parse(_))));
}

#[test]
fn follow_changes_only_follows_not_the_simulation() {
    let mut a = populated(&params(31337)).unwrap();
    let mut b = populated(&params(31337)).unwrap();
    for _ in 0..24 {
        a.step();
        b.step();
    }
    a.apply(Command::Follow { soul: 3, on: true });
    a.apply(Command::Follow { soul: 4, on: true });
    for _ in 0..YUN / 2 {
        a.step();
        b.step();
    }
    assert_ne!(a.hash(), b.hash(), "follows 納入雜湊");
    // 除 follows 外，兩個世界完全相同：把 follows 補齊後雜湊必相等。
    b.world_mut().follows = a.world().follows.clone();
    assert_eq!(a.hash(), b.hash());
    // 取消關注後亦回到原樣。
    a.apply(Command::Follow { soul: 3, on: false });
    a.apply(Command::Follow { soul: 4, on: false });
    a.step();
    b.world_mut().follows.clear();
    b.step();
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn hash_covers_new_fields() {
    let make = || {
        let mut s = populated(&params(5)).unwrap();
        for _ in 0..240 {
            s.step();
        }
        s
    };
    let h0 = make().hash();
    assert_eq!(h0, make().hash());
    let mutate = |f: &dyn Fn(&mut siming_sim::World)| {
        let mut s = make();
        f(s.world_mut());
        s.hash()
    };
    assert_ne!(h0, mutate(&|w| w.ledger.prune_cursor += 1));
    assert_ne!(h0, mutate(&|w| w.ledger.emit_cursor += 1));
    assert_ne!(h0, mutate(&|w| w.souls[1].merit += 1.0));
    assert_ne!(h0, mutate(&|w| w.persons[1].lifespan += 1));
    assert_ne!(h0, mutate(&|w| w.kill_pairs.insert((1, 2), 1).map(|_| ()).unwrap_or(())));
    assert_ne!(h0, mutate(&|w| w.prerun = !w.prerun));
    assert_ne!(h0, mutate(&|w| {
        use rand::RngCore;
        w.rng.command.next_u32();
    }));
    assert_ne!(h0, mutate(&|w| {
        use rand::RngCore;
        w.rng.people.next_u32();
    }));
}
