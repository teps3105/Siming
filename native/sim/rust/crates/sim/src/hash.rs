//! GDD 6.5 世界雜湊：依固定順序把所有欄位餵入 FxHasher（64 位元）。

use std::hash::Hasher;

use rustc_hash::FxHasher;

use crate::hash_ser::feed;
use crate::sim::World;
use crate::types::{Era, FineGeography, Geography, Province, WorldParams, WorldSize};

// 世界雜湊只在 64 位元目標上有定義：長度以 usize 餵入，FxHasher 的內部狀態也是 usize（GDD 6.5）。
const _: () = assert!(usize::BITS == 64, "世界雜湊只支援 64 位元目標");

fn f32s(h: &mut FxHasher, v: &[f32]) {
    h.write_usize(v.len());
    for x in v {
        h.write_u32(x.to_bits());
    }
}

fn u32s(h: &mut FxHasher, v: &[u32]) {
    h.write_usize(v.len());
    for x in v {
        h.write_u32(*x);
    }
}

fn u16s(h: &mut FxHasher, v: &[u16]) {
    h.write_usize(v.len());
    for x in v {
        h.write_u16(*x);
    }
}

fn str(h: &mut FxHasher, s: &str) {
    h.write_usize(s.len());
    h.write(s.as_bytes());
}

fn params(h: &mut FxHasher, p: &WorldParams) {
    str(h, &p.name);
    h.write_u64(p.seed);
    h.write_u8(match p.size {
        WorldSize::Small => 0,
        WorldSize::Medium => 1,
        WorldSize::Large => 2,
    });
    h.write_u32(p.land_ratio.to_bits());
    h.write_u32(p.relief.to_bits());
    h.write_u32(p.humidity.to_bits());
    h.write_u8(match p.era {
        Era::Ancient => 0,
        Era::Middle => 1,
        Era::Decline => 2,
    });
}

fn geo(h: &mut FxHasher, g: &Geography) {
    h.write_u16(g.w);
    h.write_u16(g.h);
    h.write_u8(g.wrap_x as u8);
    h.write_u32(g.sea_level.to_bits());
    f32s(h, &g.height);
    h.write_usize(g.terrain.len());
    for t in &g.terrain {
        h.write_u8(*t as u8);
    }
    h.write_usize(g.river.len());
    h.write(&g.river);
    u16s(h, &g.province_of);
    f32s(h, &g.aura_cell);
    h.write_usize(g.nodes.len());
    for n in &g.nodes {
        h.write_u32(n.cell);
        h.write_u32(n.strength.to_bits());
        h.write_u32(n.origin.to_bits());
        str(h, &n.name);
    }
    h.write_usize(g.ley.len());
    for e in &g.ley {
        h.write_u16(e.a);
        h.write_u16(e.b);
        h.write_u32(e.strength.to_bits());
        u32s(h, &e.path);
    }
    h.write_usize(g.rivers.len());
    for r in &g.rivers {
        str(h, &r.name);
        h.write_u8(r.level);
        u32s(h, &r.cells);
    }
    h.write_usize(g.ranges.len());
    for r in &g.ranges {
        str(h, &r.name);
        h.write_u32(r.peak);
        u32s(h, &r.cells);
    }
    h.write_usize(g.continents.len());
    for c in &g.continents {
        str(h, &c.name);
        h.write_u32(c.cells);
        h.write_u32(c.area.to_bits());
        h.write_u32(c.centroid);
    }
    u16s(h, &g.continent_of);
}

/// GDD 2.2 地理解析度的細格資料（決策 #35）。
fn fine(h: &mut FxHasher, f: &FineGeography) {
    h.write_u8(f.scale);
    h.write_u16(f.w);
    h.write_u16(f.h);
    h.write_u32(f.sea_level.to_bits());
    u16s(h, &f.height);
    h.write_usize(f.terrain.len());
    for t in &f.terrain {
        h.write_u8(*t as u8);
    }
    for v in [&f.river, &f.flow, &f.temperature, &f.moisture] {
        h.write_usize(v.len());
        h.write(v);
    }
}

fn province(h: &mut FxHasher, p: &Province) {
    h.write_u16(p.id);
    str(h, &p.name);
    u32s(h, &p.cells);
    h.write_u32(p.seat);
    u16s(h, &p.neighbors);
    u16s(h, &p.sea_neighbors);
    h.write_u8(p.lost as u8);
    for c in p.terrain_mix {
        h.write_u32(c);
    }
    h.write_u64(p.habitability.to_bits());
    h.write_u64(p.capacity.to_bits());
    h.write_u64(p.population.to_bits());
    h.write_u32(p.aura.to_bits());
    h.write_u32(p.karma_qi.to_bits());
    h.write_u32(p.merit.to_bits());
    h.write_u32(p.righteous.to_bits());
    h.write_u32(p.devastation.to_bits());
    for f in p.faith {
        h.write_u32(f.to_bits());
    }
    h.write_u8(p.city_level);
}

/// GDD 6.5：依固定順序把 World 的所有欄位餵入 FxHasher（64 位元）。
pub fn world_hash(world: &World) -> u64 {
    let mut h = FxHasher::default();
    params(&mut h, &world.params);
    h.write_u32(world.tick);
    geo(&mut h, &world.geo);
    fine(&mut h, &world.fine);
    h.write_usize(world.provinces.len());
    for p in &world.provinces {
        province(&mut h, p);
    }
    h.write_usize(world.eras.len());
    for era in &world.eras {
        str(&mut h, &era.name);
        h.write_u32(era.start_year);
        h.write_u8(era.kind as u8);
    }
    feed(&mut h, &world.evolution);
    feed(&mut h, &world.flora);
    // M2 新欄位：魂、有名之人、勢力、宿緣、因果帳（含修剪／輸出游標）、天書、殺業對、關注、預跑旗標。
    feed(&mut h, &world.souls);
    feed(&mut h, &world.persons);
    feed(&mut h, &world.powers);
    feed(&mut h, &world.threads);
    feed(&mut h, &world.ledger);
    feed(&mut h, &world.book);
    feed(&mut h, &world.kill_pairs);
    feed(&mut h, &world.follows);
    h.write_u8(world.prerun as u8);
    let g = &world.rng;
    for r in [&g.genesis, &g.soul, &g.people, &g.powers, &g.field, &g.command, &g.evolution] {
        h.write_u64(r.get_stream());
        h.write(&r.get_seed());
        h.write_u128(r.get_word_pos());
    }
    // GDD 3.7 王朝：新狀態一律附加在最後（不得調動既有餵入順序）。
    feed(&mut h, &world.dynasty);
    let r = &world.rng.dynasty;
    h.write_u64(r.get_stream());
    h.write(&r.get_seed());
    h.write_u128(r.get_word_pos());
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Terrain;
    use rand::RngCore;

    fn prov(id: u16, pop: f64) -> Province {
        Province {
            id,
            name: format!("p{id}"),
            cells: vec![id as u32],
            seat: id as u32,
            neighbors: vec![],
            sea_neighbors: vec![],
            terrain_mix: [0; Terrain::COUNT],
            habitability: 10.0,
            capacity: 0.0,
            population: pop,
            aura: 0.5,
            karma_qi: 0.0,
            merit: 0.0,
            righteous: 0.0,
            devastation: 0.0,
            faith: [0.1, 0.0, 0.0],
            city_level: 0,
            lost: false,
        }
    }

    fn world() -> World {
        let params = WorldParams::new(7);
        let geo = Geography {
            w: 2,
            h: 1,
            wrap_x: true,
            sea_level: 0.3,
            height: vec![0.5, 0.6],
            terrain: vec![Terrain::Plains, Terrain::Sea],
            river: vec![0, 1],
            province_of: vec![0, u16::MAX],
            aura_cell: vec![0.4, 0.0],
            nodes: vec![],
            ley: vec![],
            rivers: vec![],
            ranges: vec![],
            continents: vec![],
            continent_of: vec![crate::NO_CONTINENT; 2],
        };
        let rng = crate::rng::RngStreams::new(7);
        { let _ = rng; World::empty(params, geo, vec![prov(0, 100.0)]) }
    }

    #[test]
    fn identical_worlds_same_hash() {
        assert_eq!(world_hash(&world()), world_hash(&world()));
    }

    #[test]
    fn hash_changes_with_fields() {
        let base = world_hash(&world());
        let mut w = world();
        w.provinces[0].population += 1.0;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.tick = 1;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.geo.height[1] = 0.61;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.params.name = "x".into();
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.params.era = Era::Ancient;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.provinces[0].city_level = 1;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.rng.field.next_u32();
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.rng.evolution.next_u32();
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.evolution.tide += 0.01;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.evolution.sea_shift = 0.01;
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.flora.cultivation.push(0.5);
        assert_ne!(base, world_hash(&w));
        let mut w = world();
        w.flora.cleared.push(true);
        assert_ne!(base, world_hash(&w));
    }
}
