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
    pub rng: RngStreams,
}

impl World {
    /// 只有世界生成結果、尚未開天的世界（測試與 `Sim::from_parts` 用）。
    pub fn empty(params: WorldParams, geo: Geography, provinces: Vec<Province>) -> Self {
        let rng = RngStreams::new(params.seed);
        let (evolution, eras) = crate::evolution::genesis(params.seed, params.era);
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
    pub fn new_with_diag(params: &WorldParams, diag: &mut dyn FnMut(u64, &str)) -> Result<Self, WorldGenError> {
        let g = crate::worldgen::generate_with(params, diag)?;
        let mut sim = Self::from_parts(g.params, g.geo, g.provinces);
        sim.world.fine = g.fine;
        crate::evolution::prepare_aura(&mut sim.world);
        crate::landform::prepare(&mut sim.world);
        crate::genesis::run(&mut sim);
        crate::flora::prepare(&mut sim.world);
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
        // 個人、宗門、州郡、王朝的年度判定各在自己的判定月（`schedule`），每月都跑；
        // 只有補魂與天地演變、因果帳修剪留在年底月（第 12 月）。同一個月內各步的相對順序維持不變。
        let yearly = is_yearly(tick);
        // 7 州郡人口與饑荒（州郡在自己的判定月；盛世倍率、賑災與天命的饑荒項當月處理）
        let mult = crate::dynasty::growth_mult(w);
        let famines = crate::population::monthly_with(&mut w.provinces, &mult, w.params.seed, crate::schedule::Round::at(tick));
        crate::ledger::record_famines(w, &famines);
        crate::dynasty::on_famines(w, &famines);
        // 8、9 氣場與勢力的年度效果（M3、M4）
        // 10 年齡與壽元：年齡由 tick 與出生月即時算出（滿一歲落在出生月），壽元已盡者在滿壽元的那個月死亡。
        crate::people::monthly_aging(w);
        // 11 依身分行動
        crate::dao::monthly_personal(w);
        crate::people::monthly_travel(w);
        // 12 入道
        crate::people::monthly_coming_of_age(w);
        // 13 婚配
        crate::people::monthly_marriage(w);
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
