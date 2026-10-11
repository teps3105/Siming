//! GDD 6.2 `Sim` 與 `World`；GDD 6.3 推演流程。

use std::collections::{BTreeMap, BTreeSet};

use crate::model::*;
use crate::rng::RngStreams;
use crate::types::{FineGeography, Geography, Province, Tick, WorldGenError, WorldParams};

/// 世界狀態（GDD 6.2）。只能經由 [`Sim`] 改變。
#[derive(Debug, Clone)]
pub struct World {
    pub params: WorldParams,
    pub tick: Tick,
    pub geo: Geography,
    /// GDD 2.2 地理解析度的細格資料（渲染用；決策 #35）。
    pub fine: FineGeography,
    pub provinces: Vec<Province>,
    /// GDD 2.6 紀元年表：依元年遞增；第一筆是開局的紀元。世界大事（2.8）接上後由它們追加。
    pub eras: Vec<crate::calendar::Era>,
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
    pub rng: RngStreams,
}

impl World {
    /// 只有世界生成結果、尚未開天的世界（測試與 `Sim::from_parts` 用）。
    pub fn empty(params: WorldParams, geo: Geography, provinces: Vec<Province>) -> Self {
        let rng = RngStreams::new(params.seed);
        let first_era = crate::calendar::Era {
            name: crate::era::Lexicon::builtin().name(params.seed, 0, crate::era::EraKind::Genesis, &[]),
            start_year: 1,
        };
        Self {
            params,
            tick: 0,
            geo,
            fine: FineGeography::default(),
            provinces,
            eras: vec![first_era],
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
            rng,
        }
    }

    /// 在世有名之人 ID 的快照（升冪）。等價於 `0..persons.len()` 中 `alive()` 者；迴圈中途新生者不在其中，
    /// 與舊的「迴圈開始時固定範圍」寫法相同。
    pub fn alive_snapshot(&self) -> Vec<PersonId> {
        self.alive_ids.clone()
    }
}

/// 觸發條件（GDD 6.3）。
pub fn is_yearly(tick: Tick) -> bool {
    tick.is_multiple_of(12)
}
pub fn is_quarterly(tick: Tick) -> bool {
    tick.is_multiple_of(3)
}
pub fn is_shi(tick: Tick) -> bool {
    tick.is_multiple_of(360)
}

pub struct Sim {
    world: World,
    /// 本 tick 前送達、尚未套用的指令（GDD 4.2 規則：在下一個 tick 開始前生效）。
    queued: Vec<Command>,
    /// 已套用的指令紀錄（GDD 6.5 存檔）。
    log: Vec<(Tick, Command)>,
    /// `drain_events` 尚未取走的事件（GDD 6.3 事件輸出）。
    pending: Vec<EventId>,
}

impl Sim {
    /// GDD 4.6 開天：世界生成 → 開天 → 預跑。
    pub fn new(params: &WorldParams) -> Result<Self, WorldGenError> {
        Self::new_with_diag(params, &mut |_, _| {})
    }

    /// 同 [`Sim::new`]，但世界生成每次品質檢查失敗時回呼 `diag(種子, 原因)`（工具用）。
    pub fn new_with_diag(
        params: &WorldParams,
        diag: &mut dyn FnMut(u64, &str),
    ) -> Result<Self, WorldGenError> {
        let g = crate::worldgen::generate_with(params, diag)?;
        let mut sim = Self::from_parts(g.params, g.geo, g.provinces);
        sim.world.fine = g.fine;
        crate::genesis::run(&mut sim);
        Ok(sim)
    }

    /// 以既有的世界資料建立（測試、開發工具用；不執行開天）。
    pub fn from_parts(params: WorldParams, geo: Geography, provinces: Vec<Province>) -> Self {
        Self { world: World::empty(params, geo, provinces), queued: Vec::new(), log: Vec::new(), pending: Vec::new() }
    }

    /// 推進一個月（GDD 6.3）。M2 範圍以外的步驟為空操作。
    pub fn step(&mut self) {
        let w = &mut self.world;
        // 1
        w.tick += 1;
        let tick = w.tick;
        // 2 指令
        for cmd in std::mem::take(&mut self.queued) {
            crate::command::apply(w, &cmd);
            self.log.push((tick, cmd));
        }
        // 3 到期處理（含開局保證的強制突破）
        crate::people::monthly_due(w);
        crate::dao::monthly_forced(w);
        // 4 軍隊（M3）
        // 5 中陰、他道、等待、出生
        crate::soul::monthly(w);
        // 6 每季：氣場擴散（M4）
        if is_yearly(tick) {
            // 7 州郡人口
            let famines = crate::population::yearly(&mut w.provinces);
            crate::ledger::record_famines(w, &famines);
            // 8、9 氣場與勢力的年度效果（M3、M4）
            // 10 年齡與壽元
            crate::people::yearly_aging(w);
            // 11 依身分行動
            crate::dao::yearly(w);
            crate::people::yearly_travel(w);
            // 12 入道
            crate::people::yearly_coming_of_age(w);
            // 13 婚配
            crate::people::yearly_marriage(w);
            //    補魂（GDD 3.4 來源第 3 項）
            crate::soul::yearly_refill(w);
            // 14 宿緣
            crate::thread::yearly(w);
            // 15–18 勢力
            crate::dao::yearly_sects(w);
            // 19、20 王朝、魔勢（M3、M4）
            // 21 每世：天機（M5）
            // 22–24 量劫（M6）
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
