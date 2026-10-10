//! GDD 8.2 事件文字：依 `assets/text/events.zh-Hant.json` 的模板把因果帳事件填成一句話，
//! 以及魂、人、境界、六道、宿緣種類、天書條目的顯示名。文字只存在介面，不在模擬中（3.3）。
use std::collections::HashMap;

use siming_sim::{
    BookEntryKind, DeathCause, Event, EventArgs, EventKind, PersonId, Role, SixPath, SoulId, SoulState, ThreadKind, TribulationOutcome,
    World,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct EventEntry {
    seal: String,
    template: String,
}

#[derive(Deserialize)]
struct Raw {
    events: HashMap<String, EventEntry>,
}

/// 事件模板與印字（8.2）。
#[derive(bevy::prelude::Resource)]
pub struct EventText {
    events: HashMap<String, EventEntry>,
}

/// GDD 3.5 境界名（從 0 起算）。
pub const REALM_NAMES: [&str; 9] = ["煉氣", "築基", "金丹", "元嬰", "化神", "煉虛", "合體", "渡劫", "大乘"];
/// GDD 3.5 進入各境界的天劫名。
const TRIBULATION_NAMES: [&str; 9] = ["", "", "三九雷劫", "四九雷劫", "五九雷劫", "六九雷劫", "七九雷劫", "八九雷劫", "九九雷劫"];

pub fn realm_name(realm: u8) -> &'static str {
    REALM_NAMES[(realm as usize).min(REALM_NAMES.len() - 1)]
}

pub fn path_name(p: SixPath) -> &'static str {
    match p {
        SixPath::Heaven => "天道",
        SixPath::Human => "人道",
        SixPath::Asura => "阿修羅道",
        SixPath::Beast => "畜生道",
        SixPath::Hungry => "餓鬼道",
        SixPath::Hell => "地獄道",
    }
}

pub fn thread_name(k: ThreadKind) -> &'static str {
    match k {
        ThreadKind::Feud => "殺業",
        ThreadKind::Grace => "恩義",
        ThreadKind::Love => "情緣",
        ThreadKind::Mentor => "師徒",
        ThreadKind::Grudge => "怨",
    }
}

pub fn role_name(r: Role) -> &'static str {
    match r {
        Role::Child => "幼童",
        Role::Cultivator => "修士",
        Role::Monk => "僧",
        Role::Scholar => "儒士",
        Role::Ruler => "君主",
        Role::Royal => "宗室",
        Role::Official => "官員",
        Role::General => "武將",
        Role::GeneralCandidate => "武人",
        Role::Demonic => "魔修",
        Role::Beast => "妖",
        Role::Commoner => "散人",
    }
}

pub fn death_name(c: DeathCause) -> &'static str {
    match c {
        DeathCause::Age => "壽終",
        DeathCause::BreakthroughFailed => "突破失敗，走火身死",
        DeathCause::Tribulation => "渡劫身死",
        DeathCause::ShedBody => "兵解",
        DeathCause::Killed => "被殺",
        DeathCause::War => "戰死",
        DeathCause::Smite => "遭天罰",
        DeathCause::Plague => "死於疫病",
        DeathCause::Famine => "死於饑荒",
        DeathCause::Kalpa => "死於量劫",
        DeathCause::Ascended => "飛升",
        DeathCause::Nirvana => "涅槃",
        DeathCause::Sagehood => "成聖",
    }
}

pub fn book_name(k: BookEntryKind) -> &'static str {
    match k {
        BookEntryKind::ThreeLivesFeud => "三世血仇",
        BookEntryKind::EndlessVengeance => "冤冤相報",
        BookEntryKind::PastAndPresent => "前世今生",
        BookEntryKind::ReunitedLove => "再續前緣",
        BookEntryKind::ReincarnatedMaster => "轉世大能",
        BookEntryKind::DemonToBuddha => "魔佛一念",
        BookEntryKind::ThreeSagesOneSect => "一門三聖",
        BookEntryKind::ThreeTeachingsOne => "三教歸一",
        BookEntryKind::AscendByKilling => "以殺證道",
        BookEntryKind::BodhisattvaVow => "菩薩不入",
        BookEntryKind::ThreePurges => "三武滅佛",
        BookEntryKind::DeathByRemonstrance => "文死諫",
        BookEntryKind::Fengshan => "封禪泰山",
        BookEntryKind::DynastyChange => "改朝換代",
        BookEntryKind::Millennium => "國祚千年",
        BookEntryKind::Unification => "天下歸一",
        BookEntryKind::Cataclysm => "魔劫",
        BookEntryKind::SurvivedKalpa => "劫後餘生",
        BookEntryKind::GreatPeace => "大治之運",
        BookEntryKind::HeavenErred => "天道自誤",
        BookEntryKind::OneThought => "一念之差",
        BookEntryKind::RuleByInaction => "無為而治",
        BookEntryKind::DefyHeaven => "逆天改命",
    }
}

/// 天書條目總數（4.5）。
pub const BOOK_TOTAL: usize = 23;

/// 人物顯示名：有道號／法號者以其為主。
pub fn person_name(w: &World, pid: PersonId) -> String {
    w.persons.get(pid as usize).map(|p| p.name.clone()).unwrap_or_else(|| "無名".into())
}

/// 魂的顯示名：在世取今生姓名；不在世取最後一世姓名加「之魂」。
pub fn soul_name(w: &World, sid: SoulId) -> String {
    let Some(s) = w.souls.get(sid as usize) else { return "無名之魂".into() };
    match s.state {
        SoulState::Incarnate(pid) => person_name(w, pid),
        _ => match s.lives.last() {
            Some(&pid) => format!("{}之魂", person_name(w, pid)),
            None => "無名之魂".into(),
        },
    }
}

/// 魂的一行狀態（命牌用，5.7）：在世·境界·州郡／中陰·待轉世／他道。
pub fn soul_status(w: &World, sid: SoulId) -> String {
    let Some(s) = w.souls.get(sid as usize) else { return String::new() };
    let prov = |p: siming_sim::ProvinceId| w.provinces.get(p as usize).map(|p| p.name.as_str()).unwrap_or("");
    match &s.state {
        SoulState::Incarnate(pid) => {
            let Some(p) = w.persons.get(*pid as usize) else { return String::new() };
            let nth = s.lives.len();
            let life = if nth >= 2 { format!("第{}世 · ", siming_sim::calendar::zh_num(nth as u32)) } else { String::new() };
            let what = match (&p.path, p.role) {
                (siming_sim::PathState::Cultivator { realm, .. }, _) => realm_name(*realm).to_string(),
                (_, r) => role_name(r).to_string(),
            };
            format!("{life}在世 · {what} · {}", prov(p.province))
        }
        SoulState::Bardo { .. } => format!("中陰 · 待轉世 · 前世 {}", s.lives.last().map(|&pid| life_desc(w, pid)).unwrap_or_default()),
        SoulState::Realm { path, .. } => format!("{} · 前世 {}", path_name(*path), s.lives.last().map(|&pid| life_desc(w, pid)).unwrap_or_default()),
        SoulState::Waiting { .. } => "中陰 · 待宿緣".into(),
        SoulState::Liberated { way, .. } => match way {
            siming_sim::Liberation::Ascension => "已飛升",
            siming_sim::Liberation::Nirvana => "已涅槃",
            siming_sim::Liberation::Sagehood => "已成聖",
        }
        .into(),
    }
}

/// 一世的兩字描述：身分·結局（前世今生用，5.8）。
pub fn life_desc(w: &World, pid: PersonId) -> String {
    let Some(p) = w.persons.get(pid as usize) else { return String::new() };
    let what = match &p.path {
        siming_sim::PathState::Cultivator { realm, .. } if *realm >= 2 => realm_name(*realm).to_string(),
        _ => role_name(p.role).to_string(),
    };
    match p.died {
        Some((_, c)) => format!("{what}·{}", death_name(c).chars().take(2).collect::<String>()),
        None => format!("{what}·在世"),
    }
}

impl EventText {
    pub fn load() -> Self {
        let raw: Raw = serde_json::from_str(include_str!("../../assets/text/events.zh-Hant.json")).expect("events.zh-Hant.json 格式錯誤");
        Self { events: raw.events }
    }

    /// 事件印字（8.2「印」欄）。
    pub fn seal(&self, kind: EventKind) -> &str {
        self.events.get(&format!("{kind:?}")).map(|e| e.seal.as_str()).unwrap_or("事")
    }

    /// 以模板填出一句話。模板以「／」分隔的變體擇第一個占位符都能解析者。
    pub fn line(&self, w: &World, e: &Event) -> String {
        let Some(entry) = self.events.get(&format!("{:?}", e.kind)) else { return format!("{:?}", e.kind) };
        let a = e.persons.first().map(|&p| person_name(w, p)).or_else(|| e.souls.first().map(|&s| soul_name(w, s)));
        let b = e.persons.get(1).map(|&p| person_name(w, p)).or_else(|| e.souls.get(1).map(|&s| soul_name(w, s)));
        let prov = |p: siming_sim::ProvinceId| w.provinces.get(p as usize).map(|p| p.name.clone());
        let mut place = e.province.and_then(prov);
        let f = e.powers.first().and_then(|&p| w.powers.get(p as usize)).map(|p| p.name.clone());
        let g = e.powers.get(1).and_then(|&p| w.powers.get(p as usize)).map(|p| p.name.clone());
        let mut x: Option<String> = None;
        let mut n: Option<String> = None;
        let mut outcome: Option<String> = None;
        match &e.args {
            EventArgs::Death { cause, age } => {
                x = Some(death_name(*cause).into());
                n = Some(siming_sim::calendar::zh_num(*age as u32));
            }
            EventArgs::Realm { realm } => x = Some(realm_name(*realm).into()),
            EventArgs::Tribulation { realm, outcome: o } => {
                x = Some(TRIBULATION_NAMES[(*realm as usize).min(8)].into());
                outcome = Some(
                    match o {
                        TribulationOutcome::Passed => "渡過",
                        TribulationOutcome::ShedBody => "兵解",
                        TribulationOutcome::Perished => "身死",
                    }
                    .into(),
                );
            }
            EventArgs::Thread { kind, lives, .. } => {
                x = Some(thread_name(*kind).into());
                n = Some(siming_sim::calendar::zh_num(*lives as u32));
            }
            EventArgs::Judged { path } => x = Some(path_name(*path).into()),
            EventArgs::Travel { to, .. } => place = prov(*to),
            EventArgs::Role { role } => x = Some(role_name(*role).into()),
            EventArgs::Book { entry } => x = Some(book_name(*entry).into()),
            EventArgs::Sect { node } => x = node.and_then(|n| w.geo.nodes.get(n as usize)).map(|n| n.name.clone()),
            EventArgs::Famine { deaths } => n = Some(format!("{deaths}")),
            EventArgs::None => {}
        }
        if x.is_none() {
            x = match e.kind {
                EventKind::Married => Some("夫妻".into()),
                EventKind::Succession => Some("掌門".into()),
                EventKind::Rebirth => Some("人".into()),
                _ => None,
            };
        }
        let slots: [(&str, Option<&String>); 8] = [
            ("{A}", a.as_ref()),
            ("{B}", b.as_ref()),
            ("{P}", place.as_ref()),
            ("{F}", f.as_ref()),
            ("{G}", g.as_ref()),
            ("{X}", x.as_ref()),
            ("{N}", n.as_ref()),
            ("{結果}", outcome.as_ref()),
        ];
        let variants: Vec<&str> = entry.template.split('／').collect();
        let pick = variants.iter().find(|v| slots.iter().all(|(k, val)| !v.contains(k) || val.is_some())).or(variants.last()).copied().unwrap_or("");
        let mut out = pick.to_string();
        for (k, val) in slots {
            if out.contains(k) {
                out = out.replace(k, val.map(String::as_str).unwrap_or(""));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_m2_event_kind_has_a_template() {
        let t = EventText::load();
        for k in [
            EventKind::Genesis,
            EventKind::Born,
            EventKind::Died,
            EventKind::Breakthrough,
            EventKind::Tribulation,
            EventKind::Ascended,
            EventKind::ThreadCreated,
            EventKind::ThreadOmen,
            EventKind::ThreadResolved,
            EventKind::BookEntry,
            EventKind::Famine,
        ] {
            assert!(t.events.contains_key(&format!("{k:?}")), "{k:?}");
            assert_eq!(t.seal(k).chars().count(), 1, "{k:?} 印字須為一字");
        }
    }

    #[test]
    fn names_cover_all_variants() {
        assert_eq!(REALM_NAMES.len(), 9);
        assert_eq!(realm_name(20), "大乘");
        assert_eq!(book_name(BookEntryKind::ThreeLivesFeud), "三世血仇");
        assert_eq!(thread_name(ThreadKind::Feud), "殺業");
    }
}
