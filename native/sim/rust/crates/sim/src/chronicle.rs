//! GDD 5.12 人物生平：把因果帳裡屬於某個人的事實排成一份清單，供橋接的 `person_chronicle` 使用。
//!
//! 全部唯讀、不耗亂數；同一個世界狀態下同一個人的結果逐項相同。清單由舊到新，同 tick 依事件編號：
//!
//! - 只含本世的事；前世只以 [`PastLife`] 摘要交代。
//! - 同一件事只一筆：殺人事件在被殺者一側併入死亡，成年入道後才入宗門的第二次入道改標為入宗。
//!   成婚同時結下的情緣、殺人當下結下的殺業、入門當月的拜師這類附帶的事仍各自列出，但 [`Entry::group`] 相同，
//!   介面應合成一件。
//! - 有方向的事給出 [`Entry::role`]；事情的結束（喪偶、師徒解除）各有一筆，帶 [`Entry::end_reason`]。
//! - 宿緣的後果（將遇、了結、世仇、緣散）帶 [`Because`]：結緣的事件、年月與雙方當時的名字、隔了幾世。

use crate::calendar;
use crate::model::*;
use crate::sim::World;
use crate::types::{ProvinceId, Tick};

/// 人物在某個事件當時的稱呼：人物編號（當時沒有肉身為 `None`）與當時的名字。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Who {
    pub person: Option<PersonId>,
    pub name: String,
}

/// 一筆宿緣後果的前因：這條緣是哪一世、因什麼事結下。
#[derive(Debug, Clone, PartialEq)]
pub struct Because {
    /// 結緣事件的編號（開局宿緣為開天事件）。
    pub event: EventId,
    pub tick: Tick,
    /// 本人在結緣當時的稱呼。
    pub me: Who,
    /// 對方在結緣當時的稱呼。
    pub other: Who,
    /// 緣的種類代碼：`feud`、`grace`、`love`、`mentor`、`grudge`。
    pub thread: &'static str,
    /// 這筆後果發生時，這條緣已隔幾世。
    pub lives: u16,
}

/// 清單中的一筆。
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// 對應的事件編號；推得的事實（喪偶）取它所本的事件。
    pub id: EventId,
    pub tick: Tick,
    /// 種類代碼，見 `docs/5-表現/5.12-史書文本.md`。
    pub kind: &'static str,
    /// 本人在這件事裡的角色代碼；沒有方向的事為 `self`。
    pub role: &'static str,
    /// 對方（事件當時的名字）。
    pub other: Option<Who>,
    /// 對方若在結緣當時用過不同的名字（轉世）：結緣當時的稱呼。
    pub other_bond: Option<Who>,
    pub province: Option<ProvinceId>,
    pub power: Option<PowerId>,
    /// 第二個勢力（分宗時為原宗門）。
    pub power_from: Option<PowerId>,
    /// 成因代碼（`Why::code`，沒有為空）。
    pub why: &'static str,
    /// 依種類而定的代碼：身分、死因、六道、天劫結果、歸來與否（`back`／`out`）。
    pub detail: String,
    /// 依種類而定的整數：境界、享年、宿緣已隔世數。
    pub n: i64,
    /// 宿緣的種類代碼；非宿緣為空。
    pub thread: &'static str,
    /// 遊歷的目的地。
    pub target: Option<ProvinceId>,
    pub because: Option<Because>,
    /// 事情結束的原因：`spouse_died`、`master_died`、`master_left`、`disciple_left`；非結束為空。
    pub end_reason: &'static str,
    /// 同一件事的組別：同組各筆應合成一句。獨立的事等於自己的 `id`。
    pub group: EventId,
}

/// 上一世摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct PastLife {
    /// 上一世的事件編號（死亡事件，查不到為 `EventId::MAX`）。
    pub id: EventId,
    pub person: PersonId,
    pub name: String,
    pub title: Option<String>,
    pub role: String,
    pub realm: Option<u8>,
    pub fruit: Option<u8>,
    pub power: Option<PowerId>,
    /// 死因代碼（`DeathCause` 的 `Debug` 名）。
    pub cause: String,
    pub age: u32,
    pub died: Tick,
    pub province: ProvinceId,
    /// 殺他的人（當時的名字）。
    pub killer: Option<Who>,
    /// 魂判入的六道代碼（`SixPath` 的 `Debug` 名），查不到為空。
    pub path: String,
    /// 本世降生的時刻與州郡（轉生年與地）。
    pub reborn: Tick,
    pub reborn_province: ProvinceId,
}

/// 一個人的生平事實。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chronicle {
    pub past_life: Option<PastLife>,
    pub entries: Vec<Entry>,
}

/// 魂在事件當時的那一世：出生不晚於事件的最近一世；判入六道算事件之前已去世的最近一世。
pub fn life_at<'a>(w: &'a World, sid: SoulId, e: &Event) -> Option<&'a Person> {
    let mut lives = w.souls.get(sid as usize)?.lives.iter().rev().filter_map(|&pid| w.persons.get(pid as usize));
    if e.kind == EventKind::Judged {
        lives.find(|p| p.died.is_some_and(|(t, _)| t <= e.tick))
    } else {
        lives.find(|p| (p.born as i64) <= e.tick as i64)
    }
}

/// 事件是否算這個人的：他是事件的人物；或事件只記了魂，而他是這個魂在事件當時的那一世
/// （判入六道算上一世的死後，其餘的事身故者只算到死為止）。
pub fn belongs_to(w: &World, e: &Event, p: &Person) -> bool {
    if e.persons.contains(&p.id) {
        return true;
    }
    if !e.persons.is_empty() || !e.souls.contains(&p.soul) || life_at(w, p.soul, e).is_none_or(|l| l.id != p.id) {
        return false;
    }
    e.kind == EventKind::Judged || p.died.is_none_or(|(t, _)| e.tick <= t)
}

fn who_of(w: &World, soul: SoulId, e: &Event) -> Who {
    match life_at(w, soul, e) {
        Some(p) => Who { person: Some(p.id), name: p.name.clone() },
        None => Who { person: None, name: String::new() },
    }
}

fn thread_code(kind: ThreadKind) -> &'static str {
    match kind {
        ThreadKind::Feud => "feud",
        ThreadKind::Grace => "grace",
        ThreadKind::Love => "love",
        ThreadKind::Mentor => "mentor",
        ThreadKind::Grudge => "grudge",
    }
}

/// 本人在宿緣裡的角色：`debtor` 為欠債方（殺人者、受恩者、徒弟、結怨的一方）。
fn thread_role(kind: ThreadKind, debtor: bool) -> &'static str {
    match (kind, debtor) {
        (ThreadKind::Feud, true) => "killer",
        (ThreadKind::Feud, false) => "victim",
        (ThreadKind::Grace, true) => "beneficiary",
        (ThreadKind::Grace, false) => "benefactor",
        (ThreadKind::Love, _) => "lover",
        (ThreadKind::Mentor, true) => "disciple",
        (ThreadKind::Mentor, false) => "master",
        (ThreadKind::Grudge, true) => "offender",
        (ThreadKind::Grudge, false) => "offended",
    }
}

fn blank(e: &Event, kind: &'static str) -> Entry {
    Entry {
        id: e.id,
        tick: e.tick,
        kind,
        role: "self",
        other: None,
        other_bond: None,
        province: e.province,
        power: None,
        power_from: None,
        why: e.why.code(),
        detail: String::new(),
        n: 0,
        thread: "",
        target: None,
        because: None,
        end_reason: "",
        group: e.id,
    }
}

/// 事件裡本人以外的另一方（事件當時的名字）。
fn other_of(w: &World, e: &Event, me: &Person) -> Option<Who> {
    if let Some(&o) = e.persons.iter().find(|&&x| x != me.id) {
        return w.persons.get(o as usize).map(|p| Who { person: Some(p.id), name: p.name.clone() });
    }
    let sid = e.souls.iter().copied().find(|&s| s != me.soul)?;
    Some(who_of(w, sid, e)).filter(|x| !x.name.is_empty())
}

/// 宿緣事件的條目：方向、對方、前因。
fn thread_entry(w: &World, e: &Event, me: &Person, kind: &'static str) -> Option<Entry> {
    let EventArgs::Thread { thread, kind: tk, lives } = e.args else { return None };
    let t = w.threads.get(thread as usize)?;
    let mine_is_debtor = t.debtor == me.soul;
    let mut out = blank(e, kind);
    out.role = thread_role(tk, mine_is_debtor);
    out.thread = thread_code(tk);
    out.n = lives as i64;
    out.other = other_of(w, e, me);
    if kind != "thread_created" && kind != "apprenticed" {
        let (ct, ce) = t.created;
        // 墓碑仍留有 tick 與 kind，足以決定「當時的那一世」。
        let created = w.ledger.events.get(ce as usize);
        let probe = created.cloned().unwrap_or_else(|| Event { tick: ct, ..e.clone() });
        let (my_soul, other_soul) = if mine_is_debtor { (t.debtor, t.creditor) } else { (t.creditor, t.debtor) };
        let (m, o) = (who_of(w, my_soul, &probe), who_of(w, other_soul, &probe));
        if out.other.as_ref().is_some_and(|now| now.name != o.name) {
            out.other_bond = Some(o.clone());
        }
        out.because = Some(Because { event: ce, tick: ct, me: m, other: o, thread: thread_code(tk), lives });
    }
    Some(out)
}

/// 由本人的事件與推得的事實組成清單。
pub fn chronicle(w: &World, pid: PersonId) -> Option<Chronicle> {
    let me = w.persons.get(pid as usize)?;
    let from = w.ledger.events.partition_point(|e| (e.tick as i64) < me.born as i64);
    let mut mine: Vec<&Event> = w.ledger.events[from..].iter().filter(|e| e.importance > 0 && belongs_to(w, e, me)).collect();
    mine.sort_by_key(|e| (e.tick, e.id));

    let mut entries: Vec<Entry> = Vec::new();
    let mut came_of_age = false;
    let mut away_from: Option<ProvinceId> = None;
    let mut spouses: Vec<(Tick, PersonId)> = Vec::new();
    for &e in &mine {
        let entry = match e.kind {
            EventKind::ComeOfAge => {
                let role = match e.args {
                    EventArgs::Role { role } => format!("{role:?}"),
                    _ => String::new(),
                };
                let power = e.powers.first().copied();
                // 未得入道的人後來被宗門收錄：第二次入道是入宗。
                let late = came_of_age && power.is_some();
                came_of_age = true;
                let mut x = blank(e, if late { "sect_joined" } else { "come_of_age" });
                x.detail = role;
                x.power = power;
                if late {
                    x.why = "recruited";
                }
                Some(x)
            }
            EventKind::SectJoined | EventKind::SectLeft => {
                let mut x = blank(e, if e.kind == EventKind::SectJoined { "sect_joined" } else { "sect_left" });
                x.power = e.powers.first().copied();
                Some(x)
            }
            EventKind::MasterEnded => {
                let disciple = e.persons.first() == Some(&pid);
                if !disciple && e.why == Why::MasterDied {
                    None
                } else {
                    let mut x = blank(e, "mentor_ended");
                    x.role = if disciple { "disciple" } else { "master" };
                    x.other = other_of(w, e, me);
                    x.end_reason = e.why.code();
                    Some(x)
                }
            }
            EventKind::ThreadCreated => {
                let mentor = matches!(e.args, EventArgs::Thread { kind: ThreadKind::Mentor, .. });
                thread_entry(w, e, me, if mentor { "apprenticed" } else { "thread_created" })
            }
            EventKind::ThreadOmen => thread_entry(w, e, me, "thread_omen"),
            EventKind::ThreadResolved => thread_entry(w, e, me, "thread_resolved"),
            EventKind::FeudEscalated => thread_entry(w, e, me, "feud_escalated").map(|mut x| {
                x.role = if e.persons.first() == Some(&pid) { "killer" } else { "victim" };
                x
            }),
            EventKind::ThreadFaded => thread_entry(w, e, me, "thread_faded"),
            EventKind::Married => {
                let mut x = blank(e, "married");
                x.role = "spouse";
                x.other = other_of(w, e, me);
                if let Some(Who { person: Some(o), .. }) = x.other {
                    spouses.push((e.tick, o));
                }
                Some(x)
            }
            EventKind::Killed => {
                if e.persons.get(1) == Some(&pid) {
                    None
                } else {
                    let mut x = blank(e, "killed");
                    x.role = "killer";
                    x.other = other_of(w, e, me);
                    Some(x)
                }
            }
            EventKind::SoulDissolved => {
                // 魂散：本人最後一世的結尾，不是他自己經歷的事；`n` 為魂齡（年）。
                let mut x = blank(e, "soul_dissolved");
                if let EventArgs::Dissolved { age_years, .. } = e.args {
                    x.n = age_years as i64;
                }
                Some(x)
            }
            EventKind::Died | EventKind::Ascended => {
                let mut x = blank(e, if e.kind == EventKind::Died { "died" } else { "ascended" });
                if let EventArgs::Death { cause, age } = e.args {
                    x.detail = format!("{cause:?}");
                    x.n = age as i64;
                }
                x.power = e.powers.first().copied();
                if let Some(k) = mine.iter().find(|k| k.kind == EventKind::Killed && k.tick == e.tick && k.persons.get(1) == Some(&pid)) {
                    x.other = k
                        .persons
                        .first()
                        .and_then(|&o| w.persons.get(o as usize))
                        .map(|p| Who { person: Some(p.id), name: p.name.clone() });
                    x.why = k.why.code();
                }
                Some(x)
            }
            EventKind::Judged => {
                let mut x = blank(e, "judged");
                if let EventArgs::Judged { path } = e.args {
                    x.detail = format!("{path:?}");
                }
                Some(x)
            }
            EventKind::Traveled => {
                let mut x = blank(e, "traveled");
                if let EventArgs::Travel { from, to } = e.args {
                    let back = away_from == Some(to) || me.home == to;
                    away_from = if back { None } else { Some(from) };
                    x.detail = if back { "back" } else { "out" }.to_string();
                    x.target = Some(to);
                }
                Some(x)
            }
            EventKind::Breakthrough => {
                let mut x = blank(e, "breakthrough");
                if let EventArgs::Realm { realm } = e.args {
                    x.n = realm as i64;
                }
                Some(x)
            }
            EventKind::Tribulation => {
                let mut x = blank(e, "tribulation");
                if let EventArgs::Tribulation { realm, outcome } = e.args {
                    x.n = realm as i64;
                    x.detail = format!("{outcome:?}");
                }
                Some(x)
            }
            EventKind::TribulationOmen => Some(blank(e, "tribulation_omen")),
            EventKind::FellToDemon => Some(blank(e, "fell_to_demon")),
            EventKind::PowerFounded => {
                let mut x = blank(e, "sect_founded");
                x.power = e.powers.first().copied();
                Some(x)
            }
            EventKind::SectSplit => {
                let mut x = blank(e, "sect_split");
                x.power = e.powers.first().copied();
                x.power_from = e.powers.get(1).copied();
                Some(x)
            }
            EventKind::Succession => {
                let dynasty = matches!(e.args, EventArgs::Dynasty { .. });
                let mut x = blank(e, if dynasty { "throne_succeeded" } else { "succeeded" });
                x.power = e.powers.first().copied();
                Some(x)
            }
            EventKind::DynastyFounded => Some(blank(e, "dynasty_founded")),
            EventKind::Rebellion => Some(blank(e, "rebellion")),
            EventKind::Secession => Some(blank(e, "secession")),
            EventKind::Usurpation => Some(blank(e, "usurpation")),
            EventKind::Fengshan => Some(blank(e, "fengshan")),
            _ => None,
        };
        entries.extend(entry);
    }

    // 喪偶：成婚對象在本人在世時去世。婚姻只因此結束。
    for (married, spouse) in spouses {
        let Some(sp) = w.persons.get(spouse as usize) else { continue };
        let Some((died, _)) = sp.died else { continue };
        if died < married || me.died.is_some_and(|(mt, _)| died > mt) {
            continue;
        }
        let Some(ev) = death_event(w, spouse, died) else { continue };
        let mut x = blank(ev, "widowed");
        x.other = Some(Who { person: Some(sp.id), name: sp.name.clone() });
        x.end_reason = "spouse_died";
        x.why = "";
        x.province = ev.province;
        entries.push(x);
    }
    entries.sort_by_key(|x| (x.tick, x.id));
    assign_groups(&mut entries);

    Some(Chronicle { past_life: past_life(w, me), entries })
}

/// 某人的死亡事件（`Died` 或 `Ascended`）。事件依 tick 排列，二分到死亡那一日。
fn death_event(w: &World, pid: PersonId, died: Tick) -> Option<&Event> {
    let events = &w.ledger.events;
    let start = events.partition_point(|e| e.tick < died);
    events[start..]
        .iter()
        .take_while(|e| e.tick == died)
        .find(|e| matches!(e.kind, EventKind::Died | EventKind::Ascended) && e.persons.contains(&pid))
}

/// 同一件事的附帶紀錄併入主紀錄的組別（主紀錄永遠是組內最早的那筆）。
fn assign_groups(entries: &mut [Entry]) {
    let key = |x: &Entry| (crate::evolution::year_of(x.tick), calendar::date(x.tick).month);
    for i in 0..entries.len() {
        let (kind, thread, tick, role) = (entries[i].kind, entries[i].thread, entries[i].tick, entries[i].role);
        let other = entries[i].other.as_ref().and_then(|o| o.person);
        let anchor = (0..entries.len()).find(|&j| {
            j != i && {
                let a = &entries[j];
                let same_other = other.is_some() && a.other.as_ref().and_then(|o| o.person) == other;
                match (kind, thread) {
                    // 成婚同時結下情緣、了結前世的情緣。
                    ("thread_created", "love") | ("thread_resolved", "love") => a.kind == "married" && a.tick == tick && same_other,
                    // 殺人當下結下殺業。
                    ("thread_created", "feud" | "grudge") => matches!(a.kind, "killed" | "died") && a.tick == tick && same_other,
                    // 再結師徒：了結舊線與新線同時。
                    ("thread_resolved", "mentor") => a.kind == "apprenticed" && a.tick == tick && same_other,
                    // 同年月的入門與拜師。
                    ("apprenticed", _) if role == "disciple" => {
                        matches!(a.kind, "come_of_age" | "sect_joined") && a.power.is_some() && key(a) == key(&entries[i])
                    }
                    _ => false,
                }
            }
        });
        if let Some(j) = anchor {
            entries[i].group = entries[j].group;
        }
    }
}

fn past_life(w: &World, me: &Person) -> Option<PastLife> {
    let soul = w.souls.get(me.soul as usize)?;
    let idx = soul.lives.iter().position(|&l| l == me.id)?;
    let prev = w.persons.get(*soul.lives.get(idx.checked_sub(1)?)? as usize)?;
    let (died, cause) = prev.died?;
    let events = &w.ledger.events;
    let start = events.partition_point(|e| e.tick < died);
    let mut death: Option<&Event> = None;
    let mut killer: Option<Who> = None;
    for e in events[start..].iter().take_while(|e| e.tick == died) {
        match e.kind {
            EventKind::Died | EventKind::Ascended if e.persons.contains(&prev.id) => death = Some(e),
            EventKind::Killed if e.persons.get(1) == Some(&prev.id) => {
                killer =
                    e.persons.first().and_then(|&k| w.persons.get(k as usize)).map(|k| Who { person: Some(k.id), name: k.name.clone() });
            }
            _ => {}
        }
    }
    // 判入的六道：死後到轉生之間。
    let reborn = me.born.max(0) as Tick;
    let path = events[start..]
        .iter()
        .take_while(|e| e.tick <= reborn)
        .find(|e| e.kind == EventKind::Judged && e.souls.contains(&me.soul))
        .and_then(|e| match e.args {
            EventArgs::Judged { path } => Some(format!("{path:?}")),
            _ => None,
        })
        .unwrap_or_default();
    Some(PastLife {
        id: death.map_or(EventId::MAX, |e| e.id),
        person: prev.id,
        name: prev.name.clone(),
        title: prev.title.clone(),
        role: format!("{:?}", prev.role),
        realm: match prev.path {
            PathState::Cultivator { realm, .. } => Some(realm),
            _ => None,
        },
        fruit: match prev.path {
            PathState::Monk { fruit, .. } => Some(fruit),
            _ => None,
        },
        power: prev.power,
        cause: format!("{cause:?}"),
        age: prev.age(died),
        died,
        province: prev.province,
        killer,
        path,
        reborn,
        reborn_province: me.home,
    })
}
