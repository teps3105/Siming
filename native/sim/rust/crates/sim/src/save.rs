//! GDD 6.5 最小存檔（U9）：種子 + 開局設定 + 指令紀錄，無快照。
//!
//! 格式為 `serde_json`（人類可讀、無新增相依、跨平台位元組一致）；`bincode` 快照留待 M2 之後
//! （`snapshot` 欄位保留，M2 恆為 `None`）。載入 = 以 `params` 重新開天，再把指令紀錄依 tick 重播到
//! `tick`，最後比對 `hash`；不符回 [`SaveError::Desync`]（天機紊亂）。

use serde::{Deserialize, Serialize};

use crate::model::Command;
use crate::sim::Sim;
use crate::types::{Tick, WorldGenError, WorldParams};

pub const MAGIC: [u8; 4] = *b"SMGS";
/// 模擬規則版本：規則（含任何影響推演結果的參數）改變時遞增，舊存檔即拒絕載入。
pub const SIM_VERSION: u32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub magic: [u8; 4],
    pub sim_version: u32,
    pub params: WorldParams,
    /// 已套用的指令（依 tick 遞增），其後接尚未套用的排隊指令（tick = 存檔 tick + 1）。
    pub commands: Vec<(Tick, Command)>,
    pub tick: Tick,
    pub hash: u64,
    /// M2 恆為 `None`；載入時忽略。
    #[serde(default)]
    pub snapshot: Option<Vec<u8>>,
    #[serde(default)]
    pub snapshot_tick: Tick,
}

#[derive(Debug)]
pub enum SaveError {
    Parse(String),
    BadMagic,
    Version { file: u32, current: u32 },
    WorldGen(WorldGenError),
    /// 指令紀錄不是依 tick 遞增，或 tick 落在開天之前。
    BadCommandLog(Tick),
    /// 天機紊亂：重播後的雜湊與存檔不符。
    Desync { expected: u64, got: u64, tick: Tick },
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Parse(e) => write!(f, "存檔格式錯誤：{e}"),
            SaveError::BadMagic => write!(f, "不是《司業命緣》存檔"),
            SaveError::Version { file, current } => write!(f, "存檔規則版本 {file} 與目前 {current} 不符"),
            SaveError::WorldGen(e) => write!(f, "世界重建失敗：{e:?}"),
            SaveError::BadCommandLog(t) => write!(f, "指令紀錄在 tick {t} 處無效"),
            SaveError::Desync { expected, got, tick } => {
                write!(f, "天機紊亂：tick {tick} 雜湊應為 {expected:#018x}，重播得 {got:#018x}")
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl SaveFile {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("save serialize")
    }

    pub fn from_json(s: &str) -> Result<Self, SaveError> {
        let f: SaveFile = serde_json::from_str(s).map_err(|e| SaveError::Parse(e.to_string()))?;
        f.check_header()?;
        Ok(f)
    }

    fn check_header(&self) -> Result<(), SaveError> {
        if self.magic != MAGIC {
            return Err(SaveError::BadMagic);
        }
        if self.sim_version != SIM_VERSION {
            return Err(SaveError::Version { file: self.sim_version, current: SIM_VERSION });
        }
        Ok(())
    }
}

impl Sim {
    /// GDD 6.5：產生存檔（種子 + 指令紀錄；無快照）。
    pub fn save(&self) -> SaveFile {
        let w = self.world();
        let mut commands = self.command_log().to_vec();
        commands.extend(self.queued_commands().iter().map(|c| (w.tick + 1, c.clone())));
        SaveFile {
            magic: MAGIC,
            sim_version: SIM_VERSION,
            params: w.params.clone(),
            commands,
            tick: w.tick,
            hash: self.hash(),
            snapshot: None,
            snapshot_tick: 0,
        }
    }

    /// 載入：重新開天（[`Sim::new`]）再重播指令。
    pub fn load(file: &SaveFile) -> Result<Self, SaveError> {
        Self::load_with(file, Sim::new)
    }

    /// 載入，開局由 `build` 提供（測試與開發工具用，須與存檔時的建構方式相同）。
    pub fn load_with(
        file: &SaveFile,
        build: impl FnOnce(&WorldParams) -> Result<Sim, WorldGenError>,
    ) -> Result<Self, SaveError> {
        file.check_header()?;
        let mut sim = build(&file.params).map_err(SaveError::WorldGen)?;
        for (t, cmd) in &file.commands {
            if *t <= sim.world().tick {
                return Err(SaveError::BadCommandLog(*t));
            }
            if *t > file.tick {
                // 存檔時尚未套用的排隊指令：先推進到存檔 tick，只排隊，不再推進。
                while sim.world().tick < file.tick {
                    sim.step();
                }
                sim.apply(cmd.clone());
                continue;
            }
            while sim.world().tick + 1 < *t {
                sim.step();
            }
            sim.apply(cmd.clone());
        }
        while sim.world().tick < file.tick {
            sim.step();
        }
        let got = sim.hash();
        if got != file.hash {
            return Err(SaveError::Desync { expected: file.hash, got, tick: file.tick });
        }
        Ok(sim)
    }
}
