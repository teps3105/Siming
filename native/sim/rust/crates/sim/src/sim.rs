//! GDD 6.2 `Sim` 與 `World`；GDD 6.3 推演流程。

use std::collections::{BTreeMap, BTreeSet};

use crate::calendar::{Day, DAYS_PER_MONTH};
use crate::model::*;
use crate::rng::RngStreams;
use crate::types::{FineGeography, Geography, Province, ProvinceId, Tick, WorldGenError, WorldParams};

/// 世界狀態（GDD 6.2）。只能經由 [`Sim`] 改變。
#[derive(Debug, Clone)]
pub struct World {
    pub params: WorldParams,
    pub tick: Tick,
    pub geo: Geography,
    /// GDD 2.2 地理解析度的細格資料（渲染用；決策 #35）。
    pub fine: FineGeography,
    pub provinces: Vec<Province>,
    /// GDD 2.6 紀元年表：依元年遞增；第一筆是開天的紀元，開局前的歷史已有數筆，之後由世界大事（2.8）追加。
    pub eras: Vec<crate::calendar::Era>,
    /// GDD 2.8 天地的慢變量（寒暖、海平面偏移、靈潮）、所在的時期與開局前的歷史年數；每世結算一次。
    pub evolution: crate::evolution::Evolution,
    /// GDD 2.8 草木：各州的墾殖度與遊戲格的墾殖、沙化旗標；每世結算一次。
    pub flora: crate::flora::Flora,
    pub souls: Vec<Soul>,
    pub persons: Vec<Person>,
    pub powers: Vec<Power>,
    pub threads: Vec<Thread>,
    pub ledger: Ledger,
    pub book: WorldBook,
    /// GDD 3.3、4.5：同一對魂（較小 ID 在前）之間累計建立的殺業線數。
    pub kill_pairs: BTreeMap<(SoulId, SoulId), u8>,
    /// GDD 4.7：關注中的魂。
    pub follows: BTreeSet<SoulId>,
    /// GDD 4.6：預跑期間為真；事件不進傳聞、不觸發奇觀、不給天機。
    pub prerun: bool,
    /// 在世有名之人的 ID，升冪（效能快取，GDD 6.8；不納入雜湊與存檔）。`people::spawn` 追加、`people::die` 移除；
    /// 逐人掃描一律先取 [`World::alive_snapshot`]，迴圈內仍檢查 `Person::alive`，因此與全表掃描等價但不隨歷史變長。
    pub alive_ids: Vec<PersonId>,
    /// 格點靈氣的距離衰減表（效能快取，GDD 6.8；不納入雜湊與存檔）。開天時建立；以 `from_parts` 建立的世界在第一次每世結算時建立。
    pub aura_basis: Option<crate::worldgen::ley::AuraBasis>,
    /// 生成時的地形，寒暖與海平面改地形時的基準（GDD 2.8；不納入雜湊與存檔，讀檔重播時在開天當下重建）。
    pub land_basis: Option<crate::landform::LandBasis>,
    /// GDD 3.7 王朝與州郡歸屬。新狀態，世界雜湊附加在最後。
    pub dynasty: crate::dynasty::DynastyState,
    /// GDD 3.3 宿緣相遇的年度戳記（魂）：值為「年序 + 1」，等於本年表示該魂本年已相遇過。各魂在自己的判定月處理相遇，
    /// 每魂每年的次數上限因此要跨月記帳。世界雜湊附加在最後。
    pub meet_stamp: Vec<u32>,
    /// 同上（宿緣線）：本年已判定過的線，一條線的兩端同一年只判定一次。
    pub thread_stamp: Vec<u32>,
    /// 宗門勢力範圍快取（效能快取，GDD 6.8；不納入雜湊與存檔）。
    pub range_cache: crate::dao::RangeCache,
    /// 在世者依判定月分的 12 桶（效能快取，GDD 6.8；不納入雜湊與存檔）。`people::spawn`、`people::die` 同步維護；
    /// 與 `alive_ids` 對不上時（測試直接改了 `alive_ids`）下次使用時重建。
    pub alive_months: AliveMonths,
    /// 宿緣線依債務方判定月分的 12 桶（效能快取，GDD 6.8；不納入雜湊與存檔）。宿緣只增不減、債務方建線後不變，
    /// 所以只需追加新線；線數比已建的少時（測試換了線表）重建。
    pub thread_months: ThreadMonths,
    /// GDD 4.6 第一批聚落的落點（州郡、格）。開天時依種子灑落，之後不再改變。世界雜湊附加在最後。
    pub settlements: Vec<(ProvinceId, u32)>,
    /// 天地負荷與大劫的狀態（GDD 3.11）。
    pub calamity: crate::cataclysm::Calamity,
    pub rng: RngStreams,
    /// 待發生事件隊列（事件驅動引擎）。
    pub queue: crate::queue::EventQueue,
    /// 全部子流上次重設用的種子；`None` 表示仍是世界種子決定的初始狀態（人物落地之前）。
    /// 讀檔重播的過渡期用它重建同一個世界；快照存檔上線後不再需要。
    pub rng_origin: Option<u64>,
}

/// 在世者依判定月分桶（桶內升冪）。`total` 為桶內人數總和，`usize::MAX` 表示失效。
#[derive(Debug, Clone)]
pub struct AliveMonths {
    seed: u64,
    total: usize,
    by_month: [Vec<PersonId>; 12],
}

impl Default for AliveMonths {
    fn default() -> Self {
        Self { seed: 0, total: usize::MAX, by_month: Default::default() }
    }
}

/// 宿緣線依債務方判定月分桶（桶內 ThreadId 升冪）。
#[derive(Debug, Clone, Default)]
pub struct ThreadMonths {
    seed: u64,
    built: usize,
    by_month: [Vec<ThreadId>; 12],
}

impl World {
    /// 只有世界生成結果、尚未開天的世界（測試與 `Sim::from_parts` 用）。
    pub fn empty(params: WorldParams, geo: Geography, provinces: Vec<Province>) -> Self {
        let rng = RngStreams::new(params.seed);
        let (evolution, eras) = crate::evolution::genesis(params.seed, params.era, params.pop_years + params.prerun_years);
        let dynasty = crate::dynasty::DynastyState::new(provinces.len());
        Self {
            params,
            tick: 0,
            geo,
            fine: FineGeography::default(),
            provinces,
            eras,
            evolution,
            flora: Default::default(),
            souls: Vec::new(),
            persons: Vec::new(),
            powers: Vec::new(),
            threads: Vec::new(),
            ledger: Ledger::default(),
            book: WorldBook::default(),
            kill_pairs: BTreeMap::new(),
            follows: BTreeSet::new(),
            prerun: false,
            alive_ids: Vec::new(),
            aura_basis: None,
            land_basis: None,
            dynasty,
            meet_stamp: Vec::new(),
            thread_stamp: Vec::new(),
            range_cache: Default::default(),
            alive_months: Default::default(),
            thread_months: Default::default(),
            settlements: Vec::new(),
            calamity: Default::default(),
            rng,
            queue: Default::default(),
            rng_origin: None,
        }
    }

    /// 在世有名之人 ID 的快照（升冪）。等價於 `0..persons.len()` 中 `alive()` 者；迴圈中途新生者不在其中，
    /// 與舊的「迴圈開始時固定範圍」寫法相同。
    pub fn alive_snapshot(&self) -> Vec<PersonId> {
        self.alive_ids.clone()
    }

    /// `alive_ids` 剛追加 `id` 之後呼叫，維護判定月分桶。
    pub(crate) fn alive_added(&mut self, id: PersonId) {
        let seed = self.params.seed;
        let before = self.alive_ids.len() - 1;
        let am = &mut self.alive_months;
        if am.total == before && am.seed == seed {
            am.by_month[crate::schedule::judge_month(seed, crate::schedule::Domain::Person, id) as usize].push(id);
            am.total += 1;
        } else {
            am.total = usize::MAX;
        }
    }

    /// `alive_ids` 剛移除 `id` 之後呼叫，維護判定月分桶。
    pub(crate) fn alive_removed(&mut self, id: PersonId) {
        let seed = self.params.seed;
        let before = self.alive_ids.len() + 1;
        let am = &mut self.alive_months;
        if am.total == before && am.seed == seed {
            let b = &mut am.by_month[crate::schedule::judge_month(seed, crate::schedule::Domain::Person, id) as usize];
            match b.binary_search(&id) {
                Ok(k) => {
                    b.remove(k);
                    am.total -= 1;
                }
                Err(_) => am.total = usize::MAX,
            }
        } else {
            am.total = usize::MAX;
        }
    }

    fn rebuild_alive_months(&mut self) {
        let seed = self.params.seed;
        let mut by_month: [Vec<PersonId>; 12] = Default::default();
        for &id in &self.alive_ids {
            by_month[crate::schedule::judge_month(seed, crate::schedule::Domain::Person, id) as usize].push(id);
        }
        self.alive_months = AliveMonths { seed, total: self.alive_ids.len(), by_month };
    }

    /// 債務方的魂本輪輪到判定月的宿緣線（ThreadId 升冪）。等價於逐條線以 `round.takes(seed, Domain::Soul, debtor)` 過濾。
    pub fn round_threads(&mut self, round: crate::schedule::Round) -> Vec<ThreadId> {
        match round {
            crate::schedule::Round::All => (0..self.threads.len() as ThreadId).collect(),
            crate::schedule::Round::Month(m) => {
                let seed = self.params.seed;
                let tm = &mut self.thread_months;
                if tm.seed != seed || tm.built > self.threads.len() {
                    *tm = ThreadMonths { seed, ..Default::default() };
                }
                while tm.built < self.threads.len() {
                    let t = &self.threads[tm.built];
                    tm.by_month[crate::schedule::judge_month(seed, crate::schedule::Domain::Soul, t.debtor) as usize]
                        .push(tm.built as ThreadId);
                    tm.built += 1;
                }
                tm.by_month[m as usize % 12].clone()
            }
        }
    }

    /// 在世者中本輪輪到判定月者（升冪）。等價於 [`World::alive_snapshot`] 再以
    /// `round.takes(seed, Domain::Person, id)` 過濾，但直接取判定月分桶，不逐一重算，也不碰人物資料。
    pub fn round_alive(&mut self, round: crate::schedule::Round) -> Vec<PersonId> {
        match round {
            crate::schedule::Round::All => self.alive_ids.clone(),
            crate::schedule::Round::Month(m) => {
                if self.alive_months.total != self.alive_ids.len() || self.alive_months.seed != self.params.seed {
                    self.rebuild_alive_months();
                }
                self.alive_months.by_month[m as usize % 12].clone()
            }
        }
    }
}

/// 觸發條件（GDD 6.3）。
pub fn is_yearly(tick: Tick) -> bool {
    tick.is_multiple_of(crate::calendar::DAYS_PER_YEAR)
}
pub fn is_quarterly(tick: Tick) -> bool {
    tick.is_multiple_of(3 * DAYS_PER_MONTH)
}
pub fn is_shi(tick: Tick) -> bool {
    tick.is_multiple_of(30 * crate::calendar::DAYS_PER_YEAR)
}

/// `tick` 之後的第一個月底（月步進結算日）。
pub fn next_month_end(tick: Tick) -> Tick {
    (tick / DAYS_PER_MONTH + 1) * DAYS_PER_MONTH
}

pub struct Sim {
    world: World,
    /// 本 tick 前送達、尚未套用的指令（GDD 4.2 規則：在下一個 tick 開始前生效）。
    queued: Vec<Command>,
    /// 已套用的指令紀錄（GDD 6.5 存檔）。
    log: Vec<(Tick, Command)>,
    /// `drain_events` 尚未取走的事件（GDD 6.3 事件輸出）。
    pending: Vec<EventId>,
    /// 測試與除錯用：指定人物落地時的亂數種子。`None`（預設）從作業系統取。
    fixed_rng: Option<u64>,
}

/// [`Sim::advance_until`] 的停止條件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    /// 下一件重要度不小於 k 的事件。
    Importance(u8),
    /// 下一件牽涉指定魂的事件。
    Soul(SoulId),
    /// 再過這麼多年。
    Years(u32),
}

impl Sim {
    /// GDD 4.6 開天：世界生成 → 開天 → 預跑。
    pub fn new(params: &WorldParams) -> Result<Self, WorldGenError> {
        Self::new_with_diag(params, &mut |_, _| {})
    }

    /// 同 [`Sim::new`]，但人物落地起的亂數種子由呼叫端指定（`None` 同 [`Sim::new`]，從作業系統取）。
    /// 測試與除錯用：同種子、同亂數種子得到同一個世界。
    pub fn new_seeded(params: &WorldParams, rng_seed: Option<u64>) -> Result<Self, WorldGenError> {
        Self::new_with_progress_seeded(params, rng_seed, &mut |_, _| {}, &mut |_, _| {})
    }

    /// 同 [`Sim::new`]，但世界生成每次品質檢查失敗時回呼 `diag(種子, 原因)`（工具用）。
    pub fn new_with_diag(params: &WorldParams, diag: &mut dyn FnMut(u64, &str)) -> Result<Self, WorldGenError> {
        Self::new_with_progress(params, diag, &mut |_, _| {})
    }

    /// 同 [`Sim::new_with_diag`]，並在開天的各階段回報進度：`progress(階段, 0..1)`。
    pub fn new_with_progress(
        params: &WorldParams,
        diag: &mut dyn FnMut(u64, &str),
        progress: &mut dyn FnMut(crate::genesis::Stage, f32),
    ) -> Result<Self, WorldGenError> {
        Self::new_with_progress_seeded(params, None, diag, progress)
    }

    /// 同 [`Sim::new_with_progress`]，人物落地起的亂數種子由呼叫端指定（見 [`Sim::new_seeded`]）。
    pub fn new_with_progress_seeded(
        params: &WorldParams,
        rng_seed: Option<u64>,
        diag: &mut dyn FnMut(u64, &str),
        progress: &mut dyn FnMut(crate::genesis::Stage, f32),
    ) -> Result<Self, WorldGenError> {
        let g = crate::worldgen::generate_with(params, diag)?;
        let mut sim = Self::from_parts(g.params, g.geo, g.provinces);
        sim.fixed_rng = rng_seed;
        sim.world.fine = g.fine;
        crate::evolution::prepare_aura(&mut sim.world);
        crate::landform::prepare(&mut sim.world);
        crate::genesis::run_with_progress(&mut sim, progress);
        crate::flora::prepare(&mut sim.world);
        Ok(sim)
    }

    /// 以既有的世界資料建立（測試、開發工具用；不執行開天）。
    pub fn from_parts(params: WorldParams, geo: Geography, provinces: Vec<Province>) -> Self {
        Self { world: World::empty(params, geo, provinces), queued: Vec::new(), log: Vec::new(), pending: Vec::new(), fixed_rng: None }
    }

    /// 人物落地時重設全部亂數子流（開天用）：有指定固定種子用它，否則從作業系統取。
    pub fn reseed_rng(&mut self) {
        let k = self.fixed_rng.unwrap_or_else(crate::rng::os_seed);
        self.world.rng.reseed(k);
        self.world.rng_origin = Some(k);
    }

    /// 測試與除錯用：目前指定的固定亂數種子。
    pub fn fixed_rng(&self) -> Option<u64> {
        self.fixed_rng
    }

    /// 推進一個月（30 日）：等於 `advance_to` 下個月的最後一日。
    pub fn step(&mut self) {
        self.advance_to(next_month_end(self.world.tick));
    }

    /// 推進到第 `target` 日（含）：依時刻先後執行月步進與隊列裡到期的事件，同日先月步進再隊列。
    /// 月步進是尚未移植的舊規則（GDD 6.3），每月最後一日結算一次；規則移植到隊列後，它會逐步縮小。
    pub fn advance_to(&mut self, target: Day) {
        loop {
            let month_day = next_month_end(self.world.tick);
            let queued = self.world.queue.peek_day();
            match queued {
                Some(qd) if qd < month_day && qd <= target => self.run_queued(qd),
                _ if month_day <= target => self.month_step(),
                _ => break,
            }
        }
        if self.world.tick < target {
            self.world.tick = target;
        }
        self.pending.extend(crate::ledger::take_new(&mut self.world));
    }

    /// 取出並執行一件到期的隊列事件。
    fn run_queued(&mut self, qd: Day) {
        let Some((day, action)) = self.world.queue.pop_due(qd) else { return };
        self.world.tick = self.world.tick.max(day);
        match action {
            crate::queue::Action::Marker(_) => self.world.queue.fired += 1,
            crate::queue::Action::Person(pid, act) => crate::people::events::fire(&mut self.world, pid, act),
        }
    }

    /// 推進到停止條件成立或超過 `max_days` 日；回傳使條件成立的事件（因果帳 ID），沒有則 `None`。
    /// 逐個時間單位（一次月步進或一件隊列事件）推進並檢查新寫入的事件。
    pub fn advance_until(&mut self, cond: Until, max_days: Day) -> Option<EventId> {
        let start_day = self.world.tick;
        let deadline = match cond {
            Until::Years(n) => start_day + n * crate::calendar::DAYS_PER_YEAR,
            _ => start_day + max_days,
        };
        let mut seen = self.world.ledger.events.len();
        while self.world.tick < deadline {
            let month_day = next_month_end(self.world.tick);
            let next = self.world.queue.peek_day().map_or(month_day, |q| q.min(month_day)).min(deadline);
            self.advance_to(next);
            let w = &self.world;
            for id in seen..w.ledger.events.len() {
                let e = &w.ledger.events[id];
                let hit = match cond {
                    Until::Importance(k) => e.importance >= k,
                    Until::Soul(s) => e.souls.contains(&s),
                    Until::Years(_) => false,
                };
                if hit {
                    return Some(id as EventId);
                }
            }
            seen = w.ledger.events.len();
        }
        None
    }

    /// 月步進（GDD 6.3）：尚未移植到隊列的舊規則。
    fn month_step(&mut self) {
        let w = &mut self.world;
        // 1
        w.tick = next_month_end(w.tick);
        let tick = w.tick;
        // 2 指令
        for cmd in std::mem::take(&mut self.queued) {
            crate::command::apply(w, &cmd);
            self.log.push((tick, cmd));
        }
        // 3 個人規則（壽終、入道、遊歷、婚配、修煉突破）已改走事件隊列（`people::events`）。
        // 4 軍隊（M3）
        // 5 中陰、他道、等待、出生
        crate::soul::monthly(w);
        // 6 每季：氣場擴散（M4）
        // 個人、宗門、州郡、王朝的年度判定各在自己的判定月（`schedule`），每月都跑；
        // 只有補魂與天地演變、因果帳修剪留在年底月（第 12 月）。同一個月內各步的相對順序維持不變。
        let yearly = is_yearly(tick);
        // 7 州郡人口與饑荒（州郡在自己的判定月；盛世倍率、賑災與天命的饑荒項當月處理）
        let mult = crate::dynasty::growth_mult(w);
        let famines = crate::population::monthly_with(&mut w.provinces, &mult, w.params.seed, crate::schedule::Round::at(tick));
        crate::ledger::record_famines(w, &famines);
        crate::dynasty::on_famines(w, &famines);
        // 8、9 氣場與勢力的年度效果（M3、M4）
        // 10–13 壽終、修煉突破、遊歷、入道、婚配：事件隊列。
        if yearly {
            //    補魂（GDD 3.4 來源第 3 項）
            crate::soul::yearly_refill(w);
        }
        // 14 宿緣
        crate::thread::monthly(w);
        // 15–18 勢力
        crate::dao::monthly_sects(w);
        // 19 王朝與天命（GDD 3.7）：每個王朝在自己的判定月結算；20 魔勢（M4）
        crate::dynasty::monthly(w);
        if yearly {
            // 每世：天地演變（GDD 2.8），在天機收入之前
            if is_shi(tick) {
                crate::evolution::shi_settle(w);
            }
            // 21 每世：天機（M5）
            // 22–24 天地負荷與大劫（GDD 3.11）
            crate::cataclysm::yearly(w);
            // 25 因果帳修剪
            crate::ledger::prune(w);
        }
        self.pending.extend(crate::ledger::take_new(&mut self.world));
    }

    /// GDD 6.2 送出指令，於下一個 tick 開始前生效。
    pub fn apply(&mut self, cmd: Command) {
        self.queued.push(cmd);
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// 開天、測試與存檔重播專用。
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// 已套用的指令紀錄（GDD 6.5）。
    pub fn command_log(&self) -> &[(Tick, Command)] {
        &self.log
    }

    /// 已送出、尚未套用的指令（GDD 6.5 存檔用）。
    pub fn queued_commands(&self) -> &[Command] {
        &self.queued
    }

    /// 取走自上次呼叫以來新增的事件（GDD 6.3）。
    pub fn drain_events(&mut self) -> Vec<EventId> {
        std::mem::take(&mut self.pending)
    }

    /// GDD 6.5 世界雜湊。
    pub fn hash(&self) -> u64 {
        crate::hash::world_hash(&self.world)
    }
}
