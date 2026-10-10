//! GDD 6.2 資料模型：M2 起的魂、有名之人、宿緣、勢力、事件、天書、指令。
//!
//! 這些型別是 M2 各系統（soul、people、dao、thread、ledger、book、genesis、command、save）之間的契約。
//! 實作者可以「新增」變體或欄位，但不得改名或刪除；新增時在文件註解中標明 GDD 節號。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::types::{NodeId, ProvinceId, Tick};

pub type SoulId = u32;
pub type PersonId = u32;
pub type PowerId = u32;
pub type ThreadId = u32;
pub type EventId = u32;

/// GDD 3.1：0 號魂保留給「天道」（4.4）。
pub const HEAVEN_SOUL: SoulId = 0;

// ── 魂（GDD 3.1、3.2）─────────────────────────────────────

/// 道緣與教化的四個分量：道、釋、儒、魔（GDD 3.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Teaching {
    Tao = 0,
    Buddha = 1,
    Ru = 2,
    Demon = 3,
}

/// GDD 3.2 六道。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SixPath {
    Heaven,
    Human,
    Asura,
    Beast,
    Hungry,
    Hell,
}

/// GDD 3.1 超脫的三條路（魔修飛升也記為 Ascension）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Liberation {
    Ascension,
    Nirvana,
    Sagehood,
}

/// GDD 3.1 成就快照：境界與果位從 0 起算（煉氣 = 0、沙彌 = 0）。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Achievements {
    pub realm: Option<u8>,
    pub fruit: Option<u8>,
    /// 三不朽總和。
    pub immortality: f32,
}

/// GDD 3.1 魂的狀態。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SoulState {
    Incarnate(PersonId),
    Bardo { until: Tick },
    /// 天道、畜生道（為獸）、餓鬼道、地獄道。
    Realm { path: SixPath, until: Tick },
    Waiting { until: Tick, for_souls: SmallVec<[SoulId; 2]> },
    Liberated { way: Liberation, tick: Tick },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoulOrigin {
    /// 開天時建立。
    Primordial,
    /// 補魂、勢力缺額或滋生時新建。
    New,
    /// 怨靈。
    Wraith,
}

/// GDD 3.1 印記（位元旗標）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SoulMarks(pub u8);

impl SoulMarks {
    pub const ASURA: u8 = 1 << 0;
    pub const WRAITH: u8 = 1 << 1;
    pub const BODHISATTVA_VOW: u8 = 1 << 2;
    pub const FELL_TO_DEMON: u8 = 1 << 3;
    pub const GUIDED_FROM_HELL: u8 = 1 << 4;
    /// 暫存旗標（GDD 3.1 道緣）：前世入魔。死亡時由 `soul::on_death` 設定，下次轉世處理後清除。
    pub const LAST_LIFE_DEMON: u8 = 1 << 5;
    /// 暫存旗標（GDD 3.1 宿慧）：前世兵解。死亡時設定，下次轉世處理後清除。
    pub const LAST_LIFE_SHED: u8 = 1 << 6;
    /// 暫存旗標（GDD 3.1 道緣）：前世被度化。由度化的系統（M4）設定，下次轉世處理後清除。
    pub const LAST_LIFE_CONVERTED: u8 = 1 << 7;

    pub fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }

    pub fn clear(&mut self, flag: u8) {
        self.0 &= !flag;
    }
}

/// GDD 4.2 引渡令（M5）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GuideOrder {
    pub path: SixPath,
    pub province: Option<ProvinceId>,
}

/// GDD 3.1 魂。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Soul {
    pub id: SoulId,
    /// 功德 G。
    pub merit: f32,
    /// 業障 S。
    pub sin: f32,
    /// 宿慧 W，0..200。
    pub wisdom: f32,
    /// 道緣，和為 1，以 [`Teaching`] 為索引。
    pub affinity: [f32; 4],
    pub threads: Vec<ThreadId>,
    /// 歷次轉世的肉身，依序。
    pub lives: Vec<PersonId>,
    pub best_prev: Achievements,
    pub best_now: Achievements,
    pub state: SoulState,
    pub origin: SoulOrigin,
    pub marks: SoulMarks,
    /// 最後所在的州郡與格（不在世時，因果線懸在此處，GDD 5.4）。
    pub last_place: (ProvinceId, u32),
    pub guide: Option<GuideOrder>,
}

impl Soul {
    /// 淨業力 G − S（GDD 3.1）。
    pub fn net_karma(&self) -> f32 {
        self.merit - self.sin
    }

    /// 轉世次數（GDD 3.2）。
    pub fn rebirths(&self) -> u16 {
        self.lives.len().saturating_sub(1) as u16
    }
}

// ── 有名之人（GDD 3.4）───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

/// GDD 3.4 身分。是否屬於勢力看 `Person::power`（例如修士 vs 散修）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Child,
    Cultivator,
    Monk,
    Scholar,
    Ruler,
    Royal,
    Official,
    General,
    GeneralCandidate,
    Demonic,
    Beast,
    Commoner,
}

/// GDD 3.4 屬性，各 0..10。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Attrs {
    /// 根骨
    pub root: f32,
    /// 悟性
    pub insight: f32,
    /// 才略
    pub talent: f32,
    /// 德行
    pub virtue: f32,
    /// 戾氣
    pub ferocity: f32,
}

/// GDD 3.4 身分專屬狀態。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathState {
    Child,
    /// 修士、散修、魔修、妖（GDD 3.5、3.10）。境界從 0 起算。
    Cultivator { realm: u8, exp: f32, last_attempt: Option<Tick> },
    /// 僧（GDD 3.6）。
    Monk { fruit: u8, practice: f32 },
    /// 儒士（GDD 3.8）：立德、立功、立言。
    Scholar { virtue: f32, deed: f32, word: f32 },
    /// 散人、武將候補、君主等凡人身分。
    Mortal,
}

/// GDD 7.2 死因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathCause {
    Age,
    BreakthroughFailed,
    Tribulation,
    ShedBody,
    Killed,
    War,
    Smite,
    Plague,
    Famine,
    Kalpa,
    Ascended,
    Nirvana,
    Sagehood,
}

/// GDD 3.4 有名之人。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub soul: SoulId,
    pub name: String,
    /// 道號（金丹起）、法號、儒號、妖名等（GDD 7.3）。
    pub title: Option<String>,
    pub gender: Gender,
    /// 出生 tick；有號：開天時已成年的人物出生在 tick 0 之前（GDD 3.4、4.6）。
    pub born: i32,
    pub died: Option<(Tick, DeathCause)>,
    pub home: ProvinceId,
    pub province: ProvinceId,
    pub cell: u32,
    pub role: Role,
    pub power: Option<PowerId>,
    pub attrs: Attrs,
    /// 壽元上限（歲）。
    pub lifespan: u16,
    pub spouse: Option<PersonId>,
    pub master: Option<PersonId>,
    pub disciples: Vec<PersonId>,
    pub children: Vec<PersonId>,
    pub path: PathState,
    pub blessed_until: Option<Tick>,
    pub travel_until: Option<Tick>,
    /// GDD 4.6 開局保證：排定的強制突破 tick。
    pub forced_breakthrough: Option<Tick>,
    pub last_event: Option<EventId>,
}

impl Person {
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }

    /// GDD 3.4 年齡 = floor((tick − born) / 12)。
    pub fn age(&self, tick: Tick) -> u32 {
        let end = self.died.map(|d| d.0).unwrap_or(tick);
        ((end as i64 - self.born as i64).max(0) / 12) as u32
    }
}

// ── 宿緣（GDD 3.3）───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThreadKind {
    /// 殺業（赤）
    Feud,
    /// 恩義（金）
    Grace,
    /// 情緣（粉）
    Love,
    /// 師徒（青）；creditor 為師父
    Mentor,
    /// 怨（暗紫）
    Grudge,
}

/// GDD 3.3 宿緣。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub kind: ThreadKind,
    pub debtor: SoulId,
    pub creditor: SoulId,
    pub strength: f32,
    pub created: (Tick, EventId),
    /// 建線時雙方（debtor, creditor）的轉世次數快照；`lives` 由此推得。
    pub lives_base: (u16, u16),
    pub woven: bool,
    pub resolved: Option<(Tick, EventId)>,
    pub omen_at: Option<u16>,
    /// 師徒線：建線時師父的成就。
    pub master_best: Option<Achievements>,
}

// ── 勢力（GDD 2.5、3.5）──────────────────────────────────

/// GDD 3.5 宗門專屬欄位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sect {
    pub evil: bool,
    pub seat_node: Option<NodeId>,
    /// 勢力範圍半徑（步）。
    pub radius: u8,
    pub ascended: u16,
}

/// GDD 2.5 勢力種類。M3 起加入王朝、書院；M4 起加入寺院、魔門、妖域。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PowerKind {
    Sect(Sect),
}

/// GDD 2.5 勢力共通欄位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Power {
    pub id: PowerId,
    pub kind: PowerKind,
    pub name: String,
    pub seat_province: ProvinceId,
    pub seat_cell: u32,
    pub leader: Option<PersonId>,
    pub members: Vec<PersonId>,
    pub founded: Tick,
    pub fallen: Option<Tick>,
    /// 公式值（每年重算）。
    pub prestige: f32,
    /// 修正值（每年 × 0.95）。
    pub prestige_mod: f32,
    pub relations: BTreeMap<PowerId, i8>,
}

// ── 事件與因果帳（GDD 3.3、7.2）──────────────────────────

/// GDD 7.2 事件種類（M2 起）。之後的里程碑依 7.2 新增變體。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventKind {
    Genesis,
    Born,
    ComeOfAge,
    Married,
    Died,
    Breakthrough,
    Tribulation,
    TribulationOmen,
    Ascended,
    FellToDemon,
    Killed,
    Rebirth,
    Judged,
    Traveled,
    ThreadCreated,
    ThreadOmen,
    ThreadResolved,
    FeudEscalated,
    ThreadFaded,
    PowerFounded,
    Succession,
    SectSplit,
    SectWar,
    PowerFallen,
    Famine,
    BookEntry,
}

/// GDD 3.5 天劫結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TribulationOutcome {
    Passed,
    ShedBody,
    Perished,
}

/// GDD 7.2 事件參數：每種事件的模板所需資料。實作者可新增變體。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventArgs {
    None,
    Death { cause: DeathCause, age: u16 },
    Realm { realm: u8 },
    Tribulation { realm: u8, outcome: TribulationOutcome },
    Thread { thread: ThreadId, kind: ThreadKind, lives: u16 },
    Judged { path: SixPath },
    Travel { from: ProvinceId, to: ProvinceId },
    Role { role: Role },
    Book { entry: BookEntryKind },
    Sect { node: Option<NodeId> },
    Famine { deaths: u32 },
}

/// GDD 3.3 因果帳中的一筆事件。文字不存在模擬中（由介面依 7.2 模板產生）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub tick: Tick,
    pub kind: EventKind,
    pub province: Option<ProvinceId>,
    pub cell: Option<u32>,
    pub souls: SmallVec<[SoulId; 2]>,
    pub persons: SmallVec<[PersonId; 2]>,
    pub powers: SmallVec<[PowerId; 2]>,
    pub causes: SmallVec<[EventId; 3]>,
    pub importance: u8,
    pub args: EventArgs,
}

/// GDD 3.3 因果帳。`effects[i]` 為以事件 i 為直接原因的事件（漣漪用）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// 以 EventId 為索引。修剪後的事件留下墓碑（`importance == 0`，GDD 3.3），索引不變。
    pub events: Vec<Event>,
    pub effects: Vec<SmallVec<[EventId; 4]>>,
    /// 修剪游標（GDD 3.3，U7）：此 ID 之前的事件都已檢查過。
    pub prune_cursor: EventId,
    /// 輸出游標（GDD 6.3 事件輸出，U7）：此 ID 之前的事件都已由 `take_new` 取走。
    pub emit_cursor: EventId,
}

// ── 天書（GDD 4.5）────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BookEntryKind {
    ThreeLivesFeud,
    EndlessVengeance,
    PastAndPresent,
    ReunitedLove,
    ReincarnatedMaster,
    DemonToBuddha,
    ThreeSagesOneSect,
    ThreeTeachingsOne,
    AscendByKilling,
    BodhisattvaVow,
    ThreePurges,
    DeathByRemonstrance,
    Fengshan,
    DynastyChange,
    Millennium,
    Unification,
    Cataclysm,
    SurvivedKalpa,
    GreatPeace,
    HeavenErred,
    OneThought,
    RuleByInaction,
    DefyHeaven,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookRecord {
    pub count: u32,
    pub first_tick: Tick,
    pub first_event: EventId,
}

/// GDD 4.5 世界天書（存檔的一部分）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorldBook {
    pub entries: BTreeMap<BookEntryKind, BookRecord>,
}

// ── 指令（GDD 4.2、4.7、6.2）──────────────────────────────

/// GDD 6.2 指令。M2 只有 Follow；其餘權能於 M5 加入。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// GDD 4.7 關注／取消關注一個魂。
    Follow { soul: SoulId, on: bool },
}
