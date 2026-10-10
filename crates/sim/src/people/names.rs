//! GDD 8.3 人名：俗名 = 姓 + 名 1–2 字。只用 People 子流。

use std::sync::OnceLock;

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::model::Gender;

const DATA: &str = include_str!("../../data/names.zh-Hant.json");

/// 8.3：複姓機率 10%。
const COMPOUND_SURNAME_CHANCE: f32 = 0.10;
/// 8.3：中性字機率 20%。
const NEUTRAL_CHAR_CHANCE: f32 = 0.20;
/// 8.3：名最多 2 字（1–2 字等機率）。
const GIVEN_LEN_MAX: usize = 2;

struct Pool {
    common: Vec<String>,
    compound: Vec<String>,
    male_yang: Vec<String>,
    male_yin: Vec<String>,
    female_yang: Vec<String>,
    female_yin: Vec<String>,
    neutral: Vec<String>,
}

fn texts(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.get("text").and_then(|t| t.as_str()).or_else(|| x.as_str()))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(DATA).expect("names.zh-Hant.json");
        let g = &v["given_chars_by_attribute"];
        Pool {
            common: texts(&v["surnames"]["common"]),
            compound: texts(&v["surnames"]["compound"]),
            male_yang: texts(&g["male_yang"]),
            male_yin: texts(&g["male_yin"]),
            female_yang: texts(&g["female_yang"]),
            female_yin: texts(&g["female_yin"]),
            neutral: texts(&g["neutral"]),
        }
    })
}

fn pick<'a>(rng: &mut ChaCha8Rng, v: &'a [String]) -> &'a str {
    &v[rng.gen_range(0..v.len())]
}

/// 依性別產生俗名。亂數消耗量固定（姓 3 次、名字數 1 次、每字 4 次）。
pub fn person_name(rng: &mut ChaCha8Rng, gender: Gender) -> String {
    let p = pool();
    let compound = rng.gen::<f32>() < COMPOUND_SURNAME_CHANCE;
    let common = pick(rng, &p.common);
    let comp = pick(rng, &p.compound);
    let mut s = String::from(if compound { comp } else { common });
    let len = 1 + rng.gen_range(0..GIVEN_LEN_MAX);
    for _ in 0..len {
        let neutral = rng.gen::<f32>() < NEUTRAL_CHAR_CHANCE;
        let yang = rng.gen::<bool>();
        let a = pick(rng, &p.neutral);
        let b = pick(
            rng,
            match (gender, yang) {
                (Gender::Male, true) => &p.male_yang,
                (Gender::Male, false) => &p.male_yin,
                (Gender::Female, true) => &p.female_yang,
                (Gender::Female, false) => &p.female_yin,
            },
        );
        s.push_str(if neutral { a } else { b });
    }
    s
}

// ── 道號（GDD 8.3，金丹起）────────────────────────────────

const TITLES: &str = include_str!("../../data/titles.zh-Hant.json");

/// `suffix_by_realm` 的鍵，依境界索引（GDD 3.5 境界表）。
const REALM_KEYS: [&str; 9] = ["煉氣期", "築基期", "金丹期", "元嬰期", "化神期", "煉虛期", "合體期", "渡劫期", "大乘期"];

type CorePool = (Vec<String>, Vec<String>);

struct TitlePool {
    /// [正道, 魔門] × [陽, 陰] 的 (前字, 後字)。
    cores: [[CorePool; 2]; 2],
    suffixes: Vec<Vec<String>>,
}

fn strs(v: &serde_json::Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

fn title_pool() -> &'static TitlePool {
    static POOL: OnceLock<TitlePool> = OnceLock::new();
    POOL.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(TITLES).expect("titles.zh-Hant.json");
        let core = |k: &str| (strs(&v["prefix_chars"][k]["first"]), strs(&v["prefix_chars"][k]["second"]));
        TitlePool {
            cores: [[core("正道_陽"), core("正道_陰")], [core("魔門_陽"), core("魔門_陰")]],
            suffixes: REALM_KEYS.iter().map(|k| strs(&v["suffix_by_realm"][*k])).collect(),
        }
    })
}

/// 道號的兩字核心。正邪與陰陽見 8.3。亂數消耗固定 2 次。
pub fn title_core(rng: &mut ChaCha8Rng, gender: Gender, evil: bool) -> String {
    let p = title_pool();
    let (first, second) = &p.cores[evil as usize][(gender == Gender::Female) as usize];
    let a = pick(rng, first);
    let b = pick(rng, second);
    format!("{a}{b}")
}

/// 依境界的尊稱（`suffix_by_realm`，每境 10 條）。亂數消耗固定 1 次。
pub fn title_suffix(rng: &mut ChaCha8Rng, realm: u8) -> String {
    let p = title_pool();
    pick(rng, &p.suffixes[(realm as usize).min(REALM_KEYS.len() - 1)]).to_owned()
}
