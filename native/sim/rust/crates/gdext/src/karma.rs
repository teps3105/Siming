//! GDD 5.4 因果模式的資料：魂光（在世有名之人）與因果線（未了的宿緣）。
//! 只讀世界，不含 Godot 型別（`lib.rs` 的 `karma_view` 等入口負責轉成字典），所以能直接用 `cargo test` 測。
//!
//! 模擬端的有名之人只有「所在州」，沒有州內座標；魂光的位置由本模組以人物編號在所在州內決定一個固定的點
//! （[`Placer`]）：同一個人在同一州、同一份州界下永遠落在同一點；同州的人各佔一個小格位，不疊成一點；
//! 只落在州內「四鄰都屬同州」的格（沒有這種格的州才退回州內所有的格），避開海岸。

use std::collections::{HashMap, HashSet};

use siming_sim::thread::is_open;
use siming_sim::{PersonId, ProvinceId, ThreadKind, World};

/// GDD 6.8：因果模式最多繪製的線數與魂光數。
pub const MAX_LINES: usize = 1500;
pub const MAX_SOULS: usize = 4000;
/// GDD 5.4：線數過多時預設只畫強度不小於這個值的線。
pub const DEFAULT_MIN_STRENGTH: f32 = 30.0;
/// 一格切成 3×3 個格位，每個格位最多放一個魂光。
const SLOTS_PER_CELL: usize = 9;
/// 格位在格內的間距（格）；最外圈格位離格心 0.28，加上雜訊仍在格內。
const SLOT_PITCH: f32 = 0.28;
const SLOT_JITTER: f32 = 0.06;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// 為真時畫全部的線（圖層選項「全部」）。
    pub all: bool,
    pub min_strength: f32,
    pub max_lines: usize,
    pub max_souls: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { all: false, min_strength: DEFAULT_MIN_STRENGTH, max_lines: MAX_LINES, max_souls: MAX_SOULS }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Glow {
    pub person: PersonId,
    pub province: ProvinceId,
    pub pos: (f32, f32),
    /// 淨業力 G − S。
    pub karma: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub thread: u32,
    /// 0 殺業、1 恩義、2 情緣、3 師徒、4 怨（[`kind_index`]）。
    pub kind: u8,
    pub strength: f32,
    /// 欠債方一端、債主一端的位置；不在世的一端取最後所在地。
    pub from: (f32, f32),
    pub to: (f32, f32),
    /// 兩端目前在世的人物編號，不在世為 -1。
    pub debtor: i32,
    pub creditor: i32,
    /// 0 兩端都在世；1 欠債方不在世；2 債主不在世。
    pub dead: u8,
    /// 司命所牽。
    pub woven: bool,
}

#[derive(Debug, Default)]
pub struct View {
    pub glows: Vec<Glow>,
    pub lines: Vec<Line>,
    /// 取捨前符合條件的線數與在世有名之人數。
    pub qualified_lines: usize,
    pub total_glows: usize,
}

pub fn kind_index(kind: ThreadKind) -> u8 {
    match kind {
        ThreadKind::Feud => 0,
        ThreadKind::Grace => 1,
        ThreadKind::Love => 2,
        ThreadKind::Mentor => 3,
        ThreadKind::Grudge => 4,
    }
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// 魂光的定位器：每州的候選格（四鄰同州的格）只算一次，格位占用以 `place` 登記。
pub struct Placer<'a> {
    w: &'a World,
    interior: HashMap<ProvinceId, Vec<u32>>,
    taken: HashMap<ProvinceId, HashSet<usize>>,
}

impl<'a> Placer<'a> {
    pub fn new(w: &'a World) -> Self {
        Self { w, interior: HashMap::new(), taken: HashMap::new() }
    }

    fn cells(&mut self, province: ProvinceId) -> &Vec<u32> {
        let w = self.w;
        self.interior.entry(province).or_insert_with(|| {
            let Some(p) = w.provinces.get(province as usize) else { return Vec::new() };
            let (gw, gh) = (w.geo.w as u32, w.geo.h as u32);
            let own = |x: i64, y: i64| -> bool {
                if y < 0 || y >= gh as i64 {
                    return false;
                }
                let x = if w.geo.wrap_x { x.rem_euclid(gw as i64) } else { x };
                if x < 0 || x >= gw as i64 {
                    return false;
                }
                w.provinces[province as usize].cells.binary_search(&(y as u32 * gw + x as u32)).is_ok()
            };
            let inner: Vec<u32> = p
                .cells
                .iter()
                .copied()
                .filter(|&c| {
                    let (x, y) = ((c % gw) as i64, (c / gw) as i64);
                    own(x - 1, y) && own(x + 1, y) && own(x, y - 1) && own(x, y + 1)
                })
                .collect();
            if inner.is_empty() {
                p.cells.clone()
            } else {
                inner
            }
        })
    }

    /// 這個人在 `province` 的固定位置，不登記占用（不在世者的最後所在地用）。
    pub fn spot(&mut self, province: ProvinceId, person: PersonId) -> (f32, f32) {
        let slot = self.start_slot(province, person);
        self.slot_pos(province, person, slot)
    }

    /// 這個人在 `province` 的位置，格位已被別人占去就往後找下一個空的；呼叫端依人物編號升冪呼叫，結果因此確定。
    pub fn place(&mut self, province: ProvinceId, person: PersonId) -> (f32, f32) {
        let slots = self.cells(province).len() * SLOTS_PER_CELL;
        let mut slot = self.start_slot(province, person);
        if slots > 0 {
            let used = self.taken.entry(province).or_default();
            let mut tries = 0;
            while used.contains(&slot) && tries < slots {
                slot = (slot + 1) % slots;
                tries += 1;
            }
            used.insert(slot);
        }
        self.slot_pos(province, person, slot)
    }

    fn start_slot(&mut self, province: ProvinceId, person: PersonId) -> usize {
        let slots = self.cells(province).len() * SLOTS_PER_CELL;
        if slots == 0 {
            return 0;
        }
        (splitmix(person as u64 ^ 0x51ED_270B_1D5A_C0DE) % slots as u64) as usize
    }

    fn slot_pos(&mut self, province: ProvinceId, person: PersonId, slot: usize) -> (f32, f32) {
        let gw = self.w.geo.w as u32;
        let cells = self.cells(province);
        if cells.is_empty() {
            // 州已消失：退到最後所在的格心不可得，取地圖原點；呼叫端不會對消失的州取位。
            return (0.5, 0.5);
        }
        let cell = cells[(slot / SLOTS_PER_CELL) % cells.len()];
        let sub = slot % SLOTS_PER_CELL;
        let h = splitmix(person as u64 ^ 0xA5A5_1234_9E37_0001);
        let jx = ((h & 0xFFFF) as f32 / 65535.0 - 0.5) * 2.0 * SLOT_JITTER;
        let jy = (((h >> 16) & 0xFFFF) as f32 / 65535.0 - 0.5) * 2.0 * SLOT_JITTER;
        let x = (cell % gw) as f32 + 0.5 + ((sub % 3) as f32 - 1.0) * SLOT_PITCH + jx;
        let y = (cell / gw) as f32 + 0.5 + ((sub / 3) as f32 - 1.0) * SLOT_PITCH + jy;
        (x, y)
    }
}

/// 某個魂現在的肉身，在世才回傳。
fn living_body(w: &World, soul: u32) -> Option<PersonId> {
    let last = *w.souls.get(soul as usize)?.lives.last()?;
    w.persons.get(last as usize).filter(|p| p.alive()).map(|_| last)
}

/// 全部在世有名之人的位置（人物編號升冪）。
fn all_positions(w: &World, placer: &mut Placer) -> Vec<Glow> {
    let mut ids: Vec<PersonId> = w.alive_ids.iter().copied().filter(|&i| w.persons.get(i as usize).is_some_and(|p| p.alive())).collect();
    ids.sort_unstable();
    ids.into_iter()
        .map(|id| {
            let p = &w.persons[id as usize];
            let karma = w.souls.get(p.soul as usize).map_or(0.0, |s| s.net_karma());
            Glow { person: id, province: p.province, pos: placer.place(p.province, id), karma }
        })
        .collect()
}

/// 一條宿緣的畫法；兩端都不在世回傳 None。`positions` 是在世者的位置（人物編號 → 位置）。
fn line_of(w: &World, placer: &mut Placer, positions: &HashMap<PersonId, (f32, f32)>, tid: usize) -> Option<Line> {
    let t = &w.threads[tid];
    let debtor = living_body(w, t.debtor);
    let creditor = living_body(w, t.creditor);
    if debtor.is_none() && creditor.is_none() {
        return None;
    }
    let mut end = |soul: u32, body: Option<PersonId>| -> (f32, f32) {
        match body {
            Some(id) => positions.get(&id).copied().unwrap_or_else(|| placer.spot(w.persons[id as usize].province, id)),
            None => {
                let s = &w.souls[soul as usize];
                let last = s.lives.last().copied().unwrap_or(0);
                placer.spot(s.last_place.0, last)
            }
        }
    };
    let from = end(t.debtor, debtor);
    let to = end(t.creditor, creditor);
    Some(Line {
        thread: tid as u32,
        kind: kind_index(t.kind),
        strength: t.strength,
        from,
        to,
        debtor: debtor.map_or(-1, |i| i as i32),
        creditor: creditor.map_or(-1, |i| i as i32),
        dead: if debtor.is_none() {
            1
        } else if creditor.is_none() {
            2
        } else {
            0
        },
        woven: t.woven,
    })
}

/// 因果模式要畫的東西。線的取捨：符合條件（強度不小於門檻，或司命所牽，或牽到關注中的魂；`all` 時全部）的未了宿緣，
/// 司命所牽與關注對象的線排最前，其餘強度高者優先（同強度編號小者先），超過 `max_lines` 的捨去。
/// 魂光：有線的端點一定畫，其餘依淨業力的絕對值由大到小補到 `max_souls`。
pub fn view(w: &World, opts: Options) -> View {
    let mut placer = Placer::new(w);
    let all = all_positions(w, &mut placer);
    let positions: HashMap<PersonId, (f32, f32)> = all.iter().map(|g| (g.person, g.pos)).collect();

    let mut picks: Vec<(bool, usize)> = Vec::new();
    for (i, t) in w.threads.iter().enumerate() {
        if !is_open(t) {
            continue;
        }
        let followed = w.follows.contains(&t.debtor) || w.follows.contains(&t.creditor);
        let priority = t.woven || followed;
        if opts.all || priority || t.strength >= opts.min_strength {
            picks.push((priority, i));
        }
    }
    picks.sort_by(|a, b| b.0.cmp(&a.0).then(w.threads[b.1].strength.total_cmp(&w.threads[a.1].strength)).then(a.1.cmp(&b.1)));
    let mut lines = Vec::new();
    let mut qualified = 0;
    for &(_, i) in &picks {
        if let Some(line) = line_of(w, &mut placer, &positions, i) {
            qualified += 1;
            if lines.len() < opts.max_lines {
                lines.push(line);
            }
        }
    }

    let total_glows = all.len();
    let mut keep: HashSet<PersonId> = HashSet::new();
    for l in &lines {
        for id in [l.debtor, l.creditor] {
            if id >= 0 {
                keep.insert(id as PersonId);
            }
        }
    }
    let mut glows: Vec<Glow> = all.iter().filter(|g| keep.contains(&g.person)).cloned().collect();
    if glows.len() < opts.max_souls {
        let mut rest: Vec<&Glow> = all.iter().filter(|g| !keep.contains(&g.person)).collect();
        rest.sort_by(|a, b| b.karma.abs().total_cmp(&a.karma.abs()).then(a.person.cmp(&b.person)));
        rest.truncate(opts.max_souls - glows.len());
        glows.extend(rest.into_iter().cloned());
        glows.sort_by_key(|g| g.person);
    }
    View { glows, lines, qualified_lines: qualified, total_glows }
}

/// 指定的宿緣各畫成一條線（溯因視圖與選取魂光用）；不存在、已了結或兩端都不在世的略過。
pub fn threads(w: &World, ids: &[u32]) -> Vec<Line> {
    let mut placer = Placer::new(w);
    let positions: HashMap<PersonId, (f32, f32)> = all_positions(w, &mut placer).into_iter().map(|g| (g.person, g.pos)).collect();
    ids.iter()
        .filter(|&&i| w.threads.get(i as usize).is_some_and(is_open))
        .filter_map(|&i| line_of(w, &mut placer, &positions, i as usize))
        .collect()
}

/// 這個人的魂牽著的未了宿緣編號，強度高者在前。
pub fn threads_of(w: &World, person: PersonId) -> Vec<u32> {
    let Some(p) = w.persons.get(person as usize) else { return Vec::new() };
    let Some(s) = w.souls.get(p.soul as usize) else { return Vec::new() };
    let mut ids: Vec<u32> = s.threads.iter().copied().filter(|&t| w.threads.get(t as usize).is_some_and(is_open)).collect();
    ids.sort_by(|&a, &b| w.threads[b as usize].strength.total_cmp(&w.threads[a as usize].strength).then(a.cmp(&b)));
    ids
}

/// 這個人的魂前幾世的所在（前世所在地點，GDD 5.4 選取時以虛線標出），由近到遠；本世不算。
pub fn past_places(w: &World, person: PersonId) -> Vec<(f32, f32)> {
    let Some(p) = w.persons.get(person as usize) else { return Vec::new() };
    let Some(s) = w.souls.get(p.soul as usize) else { return Vec::new() };
    let mut placer = Placer::new(w);
    s.lives
        .iter()
        .rev()
        .filter(|&&l| l != person)
        .filter_map(|&l| w.persons.get(l as usize).filter(|q| q.id != person && q.born <= p.born).map(|q| (l, q.province)))
        .map(|(l, prov)| placer.spot(prov, l))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use siming_sim::{Sim, WorldParams, WorldSize};

    fn world(months: u32) -> Sim {
        let mut p = WorldParams::new(20261011);
        p.size = WorldSize::Small;
        let mut sim = Sim::new(&p).unwrap();
        for _ in 0..months {
            sim.step();
        }
        sim
    }

    #[test]
    fn gdd_5_4_every_living_named_person_has_one_glow() {
        let sim = world(12);
        let w = sim.world();
        let v = view(w, Options::default());
        let alive = w.alive_ids.iter().filter(|&&i| w.persons[i as usize].alive()).count();
        assert!(alive > 0);
        assert_eq!(v.total_glows, alive);
        assert_eq!(v.glows.len(), alive.min(MAX_SOULS));
        let mut ids: Vec<_> = v.glows.iter().map(|g| g.person).collect();
        ids.dedup();
        assert_eq!(ids.len(), v.glows.len(), "每人一點");
    }

    #[test]
    fn gdd_5_4_glow_stays_inside_its_province_on_land() {
        let sim = world(12);
        let w = sim.world();
        let gw = w.geo.w as u32;
        for g in view(w, Options::default()).glows {
            let cell = (g.pos.1.floor() as u32) * gw + g.pos.0.floor() as u32;
            assert!(w.provinces[g.province as usize].cells.contains(&cell), "人 {} 落在所在州之外：{:?}", g.person, g.pos);
            assert_eq!(g.province, w.persons[g.person as usize].province);
        }
    }

    #[test]
    fn gdd_5_4_glow_position_is_stable_and_does_not_stack() {
        let sim = world(24);
        let w = sim.world();
        let a = view(w, Options::default());
        let b = view(w, Options::default());
        assert_eq!(a.glows, b.glows, "同一世界兩次取得的位置一致");
        let mut seen = HashSet::new();
        let mut crowded = 0;
        for g in &a.glows {
            // 同州的人不疊成一點：位置取到 0.05 格的解析度仍互異。
            let key = (g.province, (g.pos.0 * 20.0).round() as i32, (g.pos.1 * 20.0).round() as i32);
            if !seen.insert(key) {
                crowded += 1;
            }
        }
        assert_eq!(crowded, 0, "有 {crowded} 個魂光與同州的人疊在同一點");
    }

    #[test]
    fn gdd_5_4_lines_default_to_strong_woven_or_followed_and_respect_the_cap() {
        let mut sim = world(36);
        let w = sim.world();
        let strong = view(w, Options::default());
        for l in &strong.lines {
            assert!(l.strength >= DEFAULT_MIN_STRENGTH || l.woven || w.follows.contains(&w.threads[l.thread as usize].debtor));
            assert!(l.debtor >= 0 || l.creditor >= 0);
            assert_eq!(l.dead == 1, l.debtor < 0);
            assert_eq!(l.dead == 2, l.creditor < 0);
        }
        let all = view(w, Options { all: true, ..Options::default() });
        assert!(all.lines.len() >= strong.lines.len());
        let capped = view(w, Options { all: true, max_lines: 3, max_souls: 5, ..Options::default() });
        assert!(capped.lines.len() <= 3);
        assert!(capped.glows.len() <= 5.max(capped.lines.len() * 2), "有線的端點一定畫，其餘補到上限");
        // 取捨：被留下的線強度不低於被捨去的（無司命所牽與關注對象時）。
        if all.lines.len() > 3 && all.lines.iter().all(|l| !l.woven) {
            let min_kept = capped.lines.iter().map(|l| l.strength).fold(f32::MAX, f32::min);
            let max_cut = all.lines.iter().skip(3).map(|l| l.strength).fold(0.0, f32::max);
            assert!(min_kept >= max_cut);
        }
        // 關注一個魂：牽到它的線排最前、不受強度門檻限制。
        let soul = w.threads.iter().find(|t| is_open(t) && t.strength < DEFAULT_MIN_STRENGTH).map(|t| t.debtor);
        if let Some(soul) = soul {
            sim.apply(siming_sim::Command::Follow { soul, on: true });
            sim.step();
            let w = sim.world();
            let v = view(w, Options::default());
            assert!(v.lines.iter().any(|l| {
                let t = &w.threads[l.thread as usize];
                t.debtor == soul || t.creditor == soul
            }));
        }
    }

    #[test]
    fn gdd_5_4_half_line_hangs_at_last_place() {
        let sim = world(120);
        let w = sim.world();
        let all = view(w, Options { all: true, ..Options::default() });
        for l in all.lines.iter().filter(|l| l.dead != 0) {
            let t = &w.threads[l.thread as usize];
            let soul = if l.dead == 1 { t.debtor } else { t.creditor };
            let s = &w.souls[soul as usize];
            let end = if l.dead == 1 { l.from } else { l.to };
            let gw = w.geo.w as u32;
            let cell = (end.1.floor() as u32) * gw + end.0.floor() as u32;
            assert!(w.provinces[s.last_place.0 as usize].cells.contains(&cell), "半條線懸在最後所在的州");
        }
    }

    #[test]
    fn gdd_5_4_focus_threads_and_past_places() {
        let sim = world(120);
        let w = sim.world();
        let all = view(w, Options { all: true, ..Options::default() });
        let Some(l) = all.lines.iter().find(|l| l.debtor >= 0) else { return };
        let ids = threads_of(w, l.debtor as u32);
        assert!(ids.contains(&l.thread));
        let lines = threads(w, &ids);
        assert!(lines.iter().any(|x| x.thread == l.thread));
        // 轉世過的魂有前世所在。
        let reborn = w.alive_ids.iter().find(|&&i| w.souls[w.persons[i as usize].soul as usize].lives.len() > 1);
        if let Some(&p) = reborn {
            assert!(!past_places(w, p).is_empty());
        }
        assert!(past_places(w, u32::MAX).is_empty());
    }

    /// 量中型世界推進 3 年後取一次因果資料要多久與各數量；平時不跑：
    /// `cargo test --release -p siming-gdext karma_view_cost -- --ignored --nocapture`。
    #[test]
    #[ignore]
    fn karma_view_cost() {
        let mut p = WorldParams::new(20261011);
        p.size = WorldSize::Medium;
        let mut sim = Sim::new(&p).unwrap();
        for _ in 0..36 {
            sim.step();
        }
        let w = sim.world();
        let open = w.threads.iter().filter(|t| is_open(t)).count();
        for (name, o) in [("預設", Options::default()), ("全部", Options { all: true, ..Options::default() })] {
            let t0 = std::time::Instant::now();
            let mut v = view(w, o);
            for _ in 0..19 {
                v = view(w, o);
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0 / 20.0;
            println!(
                "{name}：{ms:.2} ms／次；魂光 {}（在世 {}）、線 {}（符合 {}、未了宿緣 {}、全部宿緣 {}）",
                v.glows.len(),
                v.total_glows,
                v.lines.len(),
                v.qualified_lines,
                open,
                w.threads.len()
            );
        }
    }
}
