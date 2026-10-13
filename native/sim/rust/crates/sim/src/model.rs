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
/// GDD 3.7 王朝 ID：`World::dynasty.list` 的索引。
pub type DynastyId = u16;

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
    Bardo {
        until: Tick,
    },
    /// 天道、畜生道（為獸）、餓鬼道、地獄道。
    Realm {
        path: SixPath,
        until: Tick,
    },
    Waiting {
        until: Tick,
        for_souls: SmallVec<[SoulId; 2]>,
    },
    Liberated {
        way: Liberation,
        tick: Tick,
    },
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
    /// 已了結（或消散）宿緣的精簡紀錄，供天書偵測等回看；每魂上限 [`SETTLED_CAP`]。
    #[serde(default)]
    pub settled: Vec<Settled>,
}

/// 每魂保留的已了結宿緣紀錄上限；滿了先丟最舊的非師徒紀錄，全是師徒才丟最舊的。
pub const SETTLED_CAP: usize = 16;

/// 一條已了結宿緣的精簡紀錄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settled {
    /// 對方的魂。
    pub other: SoulId,
    pub kind: ThreadKind,
    /// 本魂是否為欠債方（師徒線：欠債方是徒弟，債主是師父）。
    pub debtor: bool,
    /// 了結（或消散）的事件。
    pub event: EventId,
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
    Cultivator {
        realm: u8,
        exp: f32,
        last_attempt: Option<Tick>,
    },
    /// 僧（GDD 3.6）。
    Monk {
        fruit: u8,
        practice: f32,
    },
    /// 儒士（GDD 3.8）：立德、立功、立言。
    Scholar {
        virtue: f32,
        deed: f32,
        word: f32,
    },
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
    /// 婚配的下一次抽中日；隊列裡只有日序等於它的婚配事件有效（重排時舊事件自然失效）。
    pub next_marry: Option<Tick>,
    pub last_event: Option<EventId>,
}

impl Person {
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }

    /// GDD 3.4 年齡 = floor((tick − born) / 360)。
    pub fn age(&self, tick: Tick) -> u32 {
        let end = self.died.map(|d| d.0).unwrap_or(tick);
        ((end as i64 - self.born as i64).max(0) / crate::calendar::DAYS_PER_YEAR as i64) as u32
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
    /// GDD 2.8 世界大事：大寒、大暖、末法、靈氣復甦的開始或結束。
    WorldChanged,
    /// GDD 2.8 洞天福地升降級：末法時降級或沉寂，靈潮回漲後回復。
    NodeShifted,
    /// GDD 2.8 治所被海淹沒，遷到州內宜居度最高的格。
    SeatMoved,
    /// GDD 2.8 州郡的格全數沒入海中，州郡消失。
    ProvinceLost,
    /// GDD 2.8 陸橋浮現或沉沒：兩州成為陸鄰，或不再是陸鄰。
    LandLinked,
    /// GDD 2.8 一州的森林墾盡。
    ForestCleared,
    /// GDD 2.8 一州開始沙化。
    Desertified,
    /// GDD 3.7 立國（7.2 `DynastyFounded`）。
    DynastyFounded,
    /// GDD 3.7 開疆（7.2 `Annexed`）。
    Annexed,
    /// GDD 3.7 宣戰判定成立（7.2 `WarDeclared`；戰鬥屬 3.9）。
    WarDeclared,
    /// GDD 3.7 天命 < 40 的起義判定（7.2 `Rebellion`；叛軍與戰鬥屬 3.9）。
    Rebellion,
    /// GDD 3.7 割據：武將據州自立（7.2 `Secession`）。
    Secession,
    /// GDD 3.7 篡位（7.2 `Usurpation`）。
    Usurpation,
    /// GDD 3.7 內戰分裂（7.2 `CivilWar`）。
    CivilWar,
    /// GDD 3.7 天命耗盡，土崩瓦解（7.2 `DynastyCollapsed`）。
    DynastyCollapsed,
    /// GDD 3.7 封禪（7.2 `Fengshan`）。
    Fengshan,
    /// GDD 3.7 賑災（7.2 `Relief`）。
    Relief,
    /// GDD 3.7 禁道詔令（7.2 `BanTao`）。
    BanTao,
    /// GDD 5.12 人物生平：散修投靠宗門、隨師遷入新宗（7.2 `SectJoined`；只供生平，不上傳聞）。
    SectJoined,
    /// GDD 5.12 人物生平：離開宗門（7.2 `SectLeft`；只供生平，不上傳聞）。
    SectLeft,
    /// GDD 5.12 人物生平：師徒關係解除（7.2 `MasterEnded`；只供生平，不上傳聞）。
    MasterEnded,
}

impl EventKind {
    /// 只供人物卡生平使用的紀錄：重要度 1，不進傳聞與因果圖，橋接的事件查詢不回傳。
    pub fn is_chronicle_only(self) -> bool {
        matches!(self, EventKind::SectJoined | EventKind::SectLeft | EventKind::MasterEnded)
    }
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
    Death {
        cause: DeathCause,
        age: u16,
    },
    Realm {
        realm: u8,
    },
    Tribulation {
        realm: u8,
        outcome: TribulationOutcome,
    },
    Thread {
        thread: ThreadId,
        kind: ThreadKind,
        lives: u16,
    },
    Judged {
        path: SixPath,
    },
    Travel {
        from: ProvinceId,
        to: ProvinceId,
    },
    Role {
        role: Role,
    },
    Book {
        entry: BookEntryKind,
    },
    Sect {
        node: Option<NodeId>,
    },
    Famine {
        deaths: u32,
    },
    /// GDD 2.8：`new_era` 為真表示這件大事開啟了新紀元（年表的最後一筆）。
    World {
        change: crate::evolution::WorldChange,
        new_era: bool,
    },
    /// GDD 2.8：節點由 `from` 級變為 `to` 級（0 沉寂、1 福地、2 洞天、3 大洞天）。
    Node {
        node: NodeId,
        from: u8,
        to: u8,
    },
    /// GDD 2.8：治所由格 `from` 遷到格 `to`。
    Seat {
        from: u32,
        to: u32,
    },
    /// GDD 2.8：州郡消失；`refuge` 為收容最多人的鄰州，`perished` 為鄰州容不下而折損的人數。
    Lost {
        refuge: Option<ProvinceId>,
        perished: u32,
    },
    /// GDD 2.8：與 `other` 成為陸鄰（`joined`）或不再是陸鄰。
    Link {
        other: ProvinceId,
        joined: bool,
    },
    /// GDD 3.7 王朝事件：`dynasty` 為主角王朝，`other` 為對手或新生的王朝，`n` 為依事件而定的數值
    /// （賑災為救活人數；其餘 0）。
    Dynasty {
        dynasty: DynastyId,
        other: Option<DynastyId>,
        n: u32,
    },
}

/// 事件的成因分類（隨事件記錄；`None` 表示該種事件沒有分類或查不到）。
/// 每種事件只用自己那組值，編碼對照見 `docs/7-附錄/7.2-事件總表.md`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum Why {
    #[default]
    None = 0,
    // Killed
    /// 尋仇：兩魂間有未了的殺業或怨線。
    Vendetta = 1,
    /// 除魔：修士斬殺魔修。
    SlayDemon = 2,
    /// 魔修襲殺。
    DemonAssassin = 3,
    /// 爭脈戰死。
    SectWarDeath = 4,
    /// 其他。
    KillOther = 5,
    // FellToDemon
    /// 罪業積累的心魔。
    SinDemon = 11,
    /// 前世入魔（修羅）印記。
    PastLifeMark = 13,
    // PowerFounded
    /// 自立：散修自行創宗。
    SelfFounded = 20,
    // Married
    /// 前世宿緣牽引：兩魂有未了、已隔世的情緣線。
    FateTug = 30,
    /// 同門。
    SameSect = 31,
    /// 同鄉。
    SameHometown = 32,
    // ThreadResolved
    /// 還清（贖罪）。
    Repaid = 40,
    /// 殺償（復仇了結）。
    Avenged = 41,
    /// 成婚了結（再續前緣）。
    WedSettled = 42,
    /// 其他了結（再結師徒等）。
    ResolvedOther = 43,
    // Rebellion、Secession、Usurpation 的主因
    /// 天命低。
    LowMandate = 50,
    /// 國庫不足，武將叛離。
    Finances = 52,
    /// 無儲君，文武篡位。
    NoHeir = 53,
    // SectJoined、SectLeft
    /// 散修投靠宗門。
    RogueJoined = 60,
    /// 隨師父遷入（或遷出）：分宗時師父門下的徒弟隨行。
    FollowedMaster = 61,
    /// 入魔離宗。
    LeftForDemon = 62,
    /// 宗門覆滅，門人成為散修。
    SectDestroyed = 63,
    // MasterEnded
    /// 師父身故。
    MasterDied = 70,
    /// 師父離宗（入魔、分宗另立），門下不隨行的徒弟失師。
    MasterLeft = 71,
    /// 徒弟離宗，師徒解除。
    DiscipleLeft = 72,
}

impl Why {
    /// 給介面與文件用的代碼字串（對照見 `docs/7-附錄/7.2-事件總表.md`）。
    pub fn code(self) -> &'static str {
        match self {
            Why::None => "",
            Why::Vendetta => "vendetta",
            Why::SlayDemon => "slay_demon",
            Why::DemonAssassin => "demon_assassin",
            Why::SectWarDeath => "sect_war_death",
            Why::KillOther => "kill_other",
            Why::SinDemon => "sin_demon",
            Why::PastLifeMark => "past_life_mark",
            Why::SelfFounded => "self_founded",
            Why::FateTug => "fate_tug",
            Why::SameSect => "same_sect",
            Why::SameHometown => "same_hometown",
            Why::Repaid => "repaid",
            Why::Avenged => "avenged",
            Why::WedSettled => "wed_settled",
            Why::ResolvedOther => "resolved_other",
            Why::LowMandate => "low_mandate",
            Why::Finances => "finances",
            Why::NoHeir => "no_heir",
            Why::RogueJoined => "rogue_joined",
            Why::FollowedMaster => "followed_master",
            Why::LeftForDemon => "left_for_demon",
            Why::SectDestroyed => "sect_destroyed",
            Why::MasterDied => "master_died",
            Why::MasterLeft => "master_left",
            Why::DiscipleLeft => "disciple_left",
        }
    }
}

/// 事件兩方關係的位元旗標快照（事件有兩方時記錄；查不到的位元留空）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rel(pub u16);

impl Rel {
    pub const MASTER_DISCIPLE: u16 = 1 << 0;
    pub const SAME_SECT: u16 = 1 << 1;
    pub const SPOUSES: u16 = 1 << 2;
    pub const PARENT_CHILD: u16 = 1 << 3;
    pub const SAME_HOMETOWN: u16 = 1 << 4;
    /// 兩魂之間有未了的宿緣線，依種類：殺業、恩義、情緣、師徒、怨。
    pub const THREAD_FEUD: u16 = 1 << 5;
    pub const THREAD_GRACE: u16 = 1 << 6;
    pub const THREAD_LOVE: u16 = 1 << 7;
    pub const THREAD_MENTOR: u16 = 1 << 8;
    pub const THREAD_GRUDGE: u16 = 1 << 9;
    /// 未了宿緣線已隔的世數（取最多者，上限 7）：第 10–12 位。
    pub const LIVES_SHIFT: u16 = 10;

    /// 成立的關係旗標名稱（不含世數；介面用）。
    pub fn names(self) -> Vec<&'static str> {
        const TABLE: [(u16, &str); 10] = [
            (Rel::MASTER_DISCIPLE, "master_disciple"),
            (Rel::SAME_SECT, "same_sect"),
            (Rel::SPOUSES, "spouses"),
            (Rel::PARENT_CHILD, "parent_child"),
            (Rel::SAME_HOMETOWN, "same_hometown"),
            (Rel::THREAD_FEUD, "thread_feud"),
            (Rel::THREAD_GRACE, "thread_grace"),
            (Rel::THREAD_LOVE, "thread_love"),
            (Rel::THREAD_MENTOR, "thread_mentor"),
            (Rel::THREAD_GRUDGE, "thread_grudge"),
        ];
        TABLE.iter().filter(|(b, _)| self.has(*b)).map(|(_, n)| *n).collect()
    }

    pub fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// 宿緣線已隔的世數。
    pub fn lives(self) -> u16 {
        (self.0 >> Self::LIVES_SHIFT) & 7
    }
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
    /// 成因分類（見 [`Why`]）。
    #[serde(default)]
    pub why: Why,
    /// 兩方關係快照（見 [`Rel`]）。
    #[serde(default)]
    pub rel: Rel,
}

/// GDD 3.3 因果帳。`effects[i]` 為以事件 i 為直接原因的事件（漣漪用）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// 以 EventId 為索引。修剪後的事件留下墓碑（`importance == 0`，GDD 3.3），索引不變。
    pub events: Vec<Event>,
    pub effects: Vec<SmallVec<[EventId; 4]>>,
    /// 修剪游標（GDD 3.3，U7）：此 ID 之前的事件都已檢查過。
    pub prune_cursor: EventId,
    /// 因在世者或未了線而暫不修剪的舊事件（每十年複查）。
    #[serde(default)]
    pub prune_deferred: Vec<EventId>,
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
