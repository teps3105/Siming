//! GDD 2.6、7.3 紀元名：比照歷代從經典取字定年號的做法，但取的是講天地格局的字。
//!
//! 紀元名兩個字：首字講這段時間的氣象，依世界大事的性質（[`EraKind`]）從該類的字池取；
//! 次字講世界的格局，從共用的字池取。經典裡現成的雙字詞（鴻蒙、玄黃一類）優先。
//! 字可以跨紀元重用，同一張年表裡不重名。不消耗模擬的隨機流：由種子與紀元序號算出。

use serde::Deserialize;
use std::collections::BTreeMap;

/// 開啟紀元的世界大事的性質（GDD 2.8「世界大事與紀元」）。變體的順序進世界雜湊，只在末尾追加。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EraKind {
    /// 開天之初。
    Genesis,
    /// 大寒。
    Cold,
    /// 大暖（海進）。
    Warm,
    /// 末法。
    Decline,
    /// 靈氣復甦。
    Revival,
    /// 天崩地裂。
    Sundering,
    /// 魔劫蔽日。
    Cataclysm,
    /// 重光（魔劫平息）。
    Restoration,
    /// 浩劫之後。
    Aftermath,
    /// 承平。
    Peace,
}

impl EraKind {
    pub const ALL: [EraKind; 10] = [
        EraKind::Genesis,
        EraKind::Cold,
        EraKind::Warm,
        EraKind::Decline,
        EraKind::Revival,
        EraKind::Sundering,
        EraKind::Cataclysm,
        EraKind::Restoration,
        EraKind::Aftermath,
        EraKind::Peace,
    ];

    /// 字庫裡的鍵。
    pub fn key(self) -> &'static str {
        match self {
            EraKind::Genesis => "genesis",
            EraKind::Cold => "cold",
            EraKind::Warm => "warm",
            EraKind::Decline => "decline",
            EraKind::Revival => "revival",
            EraKind::Sundering => "sundering",
            EraKind::Cataclysm => "cataclysm",
            EraKind::Restoration => "restoration",
            EraKind::Aftermath => "aftermath",
            EraKind::Peace => "peace",
        }
    }
}

/// 一個字或一個現成的雙字詞，連同出處（顯示在天書的紀元年表）。
#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub text: String,
    /// 書名與篇名。
    pub source: String,
    /// 含該字的原句。
    pub quote: String,
}

/// 紀元名字庫（`data/eras.zh-Hant.json`）。
#[derive(Debug, Clone, Deserialize)]
pub struct Lexicon {
    /// 各類時間段的首字池，鍵為 [`EraKind::key`]。
    pub first: BTreeMap<String, Vec<Entry>>,
    /// 共用的次字池。
    pub second: Vec<Entry>,
    /// 各類時間段現成的雙字詞。
    pub ready: BTreeMap<String, Vec<Entry>>,
    /// 不用的組合：與現代常用詞相同、會讓人出戲的。
    pub banned: Vec<String>,
}

const DATA: &str = include_str!("../data/eras.zh-Hant.json");

/// 現成雙字詞被選中的機率（百分比）；其餘時候由首字與次字組合。
const READY_PERCENT: u64 = 50;

impl Lexicon {
    pub fn builtin() -> Self {
        serde_json::from_str(DATA).expect("eras.zh-Hant.json")
    }

    /// 第 `index` 個紀元（開局為 0）的名字：性質為 `kind`，不與 `used` 重名。
    pub fn name(&self, seed: u64, index: u32, kind: EraKind, used: &[String]) -> String {
        let key = kind.key();
        let firsts = self.first.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let ready = self.ready.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let free = |name: &str| !used.iter().any(|u| u == name) && !self.banned.iter().any(|b| b == name);
        for attempt in 0..64u64 {
            let h = mix(seed ^ ((index as u64) << 32) ^ (attempt << 8) ^ 0xE2A);
            let candidate = if !ready.is_empty() && h % 100 < READY_PERCENT {
                ready[((h >> 8) as usize) % ready.len()].text.clone()
            } else if !firsts.is_empty() && !self.second.is_empty() {
                let a = &firsts[((h >> 8) as usize) % firsts.len()].text;
                let b = &self.second[((h >> 24) as usize) % self.second.len()].text;
                if a == b {
                    continue;
                }
                format!("{a}{b}")
            } else {
                continue;
            };
            if free(&candidate) {
                return candidate;
            }
        }
        // 抽樣都撞名：依序列舉第一個沒用過的組合。
        for a in firsts {
            for b in &self.second {
                let candidate = format!("{}{}", a.text, b.text);
                if a.text != b.text && free(&candidate) {
                    return candidate;
                }
            }
        }
        // 這一類的組合全部用盡（年表極長時）：加上序號。
        format!("{}{}", firsts.first().map_or("", |e| e.text.as_str()), index)
    }

    /// 名字裡每個字（或整個現成詞）的出處，供年表顯示；查不到的字略過。
    pub fn sources(&self, name: &str) -> Vec<&Entry> {
        if let Some(entry) = self.ready.values().flatten().find(|e| e.text == name) {
            return vec![entry];
        }
        name.chars()
            .filter_map(|c| {
                let s = c.to_string();
                self.first
                    .values()
                    .flatten()
                    .chain(self.second.iter())
                    .find(|e| e.text == s)
            })
            .collect()
    }
}

/// SplitMix64 的混合步驟。
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_characters_and_every_entry_has_a_source() {
        let lex = Lexicon::builtin();
        for kind in EraKind::ALL {
            let pool = lex.first.get(kind.key()).unwrap_or_else(|| panic!("缺 {} 的首字池", kind.key()));
            assert!(pool.len() >= 3, "{} 的首字少於 3 個", kind.key());
        }
        assert!(lex.second.len() >= 10, "共用次字少於 10 個");
        let all = lex.first.values().flatten().chain(lex.second.iter()).chain(lex.ready.values().flatten());
        for entry in all {
            assert!(!entry.source.is_empty() && !entry.quote.is_empty(), "「{}」沒有出處或原句", entry.text);
            for c in entry.text.chars() {
                assert!(entry.quote.contains(c), "「{}」的原句裡沒有「{c}」字：{}", entry.text, entry.quote);
            }
        }
    }

    #[test]
    fn names_are_two_characters_deterministic_and_unique_in_a_chronicle() {
        let lex = Lexicon::builtin();
        let mut used: Vec<String> = Vec::new();
        // 一張很長的年表：每一類輪四次。
        for round in 0..4u32 {
            for (i, kind) in EraKind::ALL.into_iter().enumerate() {
                let index = round * 10 + i as u32;
                let name = lex.name(20261011, index, kind, &used);
                assert_eq!(name.chars().count(), 2, "紀元名應為兩個字：{name}");
                assert_eq!(name, lex.name(20261011, index, kind, &used), "同樣的輸入得到同樣的名字");
                assert!(!used.contains(&name), "年表裡重名：{name}");
                assert!(!lex.banned.contains(&name), "用了不該用的組合：{name}");
                assert!(!lex.sources(&name).is_empty(), "「{name}」查不到出處");
                used.push(name);
            }
        }
    }

    #[test]
    fn different_worlds_get_different_chronicles() {
        let lex = Lexicon::builtin();
        let names = |seed: u64| -> Vec<String> {
            let mut used = Vec::new();
            for (i, kind) in EraKind::ALL.into_iter().enumerate() {
                let name = lex.name(seed, i as u32, kind, &used);
                used.push(name);
            }
            used
        };
        assert_ne!(names(1), names(2), "不同種子的年表不該完全一樣");
    }
}
