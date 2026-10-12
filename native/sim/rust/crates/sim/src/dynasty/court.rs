//! GDD 3.7 朝廷：繼承、文臣與武將、王族、國策與詔令、封禪。

use rand::Rng;

use super::realm::{make_ruler, move_to};
use super::*;
use crate::soul;

/// GDD 3.7 國策：君主道緣中最高者（同值依道、釋、儒、魔的次序）；魔最高為「無」。
pub fn policy_of(w: &World, ruler: PersonId) -> Policy {
    let aff = w.souls[w.persons[ruler as usize].soul as usize].affinity;
    let mut best = 0;
    for k in 1..4 {
        if aff[k] > aff[best] {
            best = k;
        }
    }
    match best {
        0 => Policy::Tao,
        1 => Policy::Buddha,
        2 => Policy::Ru,
        _ => Policy::None,
    }
}

/// 即位：`pid` 成為君主，國策依其道緣重新決定，詔令解除（駕崩即解鎖），移駐都城。
pub fn crown(w: &mut World, di: usize, pid: PersonId) {
    let cap = w.dynasty.list[di].capital;
    let policy = policy_of(w, pid);
    let tick = w.tick;
    let d = &mut w.dynasty.list[di];
    d.officials.retain(|&o| o != pid);
    d.generals.retain(|g| g.0 != pid);
    d.heirs.retain(|&h| h != pid);
    d.ruler = Some(pid);
    d.policy = policy;
    d.decree = Decree::None;
    d.sect_relations.clear();
    d.ruler_since = tick;
    make_ruler(w, pid, cap);
}

/// 名冊整理：沉沒的州郡、已死或已換身分的人員移出。都城沉沒時遷到人口最多的剩餘州。
pub(super) fn prune(w: &mut World, di: usize) {
    let lost: Vec<ProvinceId> = w.dynasty.list[di].provinces.iter().copied().filter(|&p| w.provinces[p as usize].lost).collect();
    for p in lost {
        w.dynasty.owner[p as usize] = NO_OWNER;
        w.dynasty.list[di].provinces.retain(|&x| x != p);
    }
    let cap = w.dynasty.list[di].capital;
    if !w.dynasty.list[di].provinces.contains(&cap) {
        let new_cap = w.dynasty.list[di]
            .provinces
            .iter()
            .copied()
            .max_by(|&a, &b| w.provinces[a as usize].population.total_cmp(&w.provinces[b as usize].population).then(b.cmp(&a)));
        if let Some(c) = new_cap {
            w.dynasty.list[di].capital = c;
        }
    }
    let live: Vec<bool> = w.dynasty.list.iter().map(|x| x.fallen.is_none()).collect();
    let persons = &w.persons;
    let ok = |p: PersonId, role: Role| persons.get(p as usize).is_some_and(|x| x.alive() && x.role == role);
    let d = &mut w.dynasty.list[di];
    d.heirs.retain(|&h| ok(h, Role::Royal));
    d.officials.retain(|&o| ok(o, Role::Official));
    d.generals.retain(|g| ok(g.0, Role::General));
    d.wars.retain(|&o| live.get(o as usize).copied().unwrap_or(false));
}

/// GDD 3.7 繼承：君主死亡 → 最年長的在世儲君繼位 → 無儲君則才略最高的武將或文臣篡位（天命 −20）
/// → 前兩名才略差 ≤ 1 時 35% 機率內戰。無儲君也無文武時王朝崩潰。
pub(super) fn succession(w: &mut World, di: usize) {
    let id = di as DynastyId;
    let ruler = w.dynasty.list[di].ruler;
    if ruler.is_some_and(|r| w.persons[r as usize].alive()) {
        return;
    }
    let cause: Vec<EventId> = ruler.and_then(|r| w.persons[r as usize].last_event).into_iter().collect();
    let heir = w.dynasty.list[di].heirs.iter().copied().min_by_key(|&h| (w.persons[h as usize].born, h));
    let cap = w.dynasty.list[di].capital;
    if let Some(h) = heir {
        crown(w, di, h);
        rec(w, EventKind::Succession, 3, Some(cap), &[h], id, None, 0, &cause);
        return;
    }
    // 篡位候選：文臣（駐都城）與武將（駐地）。才略高者在前，同值取 ID 小者。
    let mut cands: Vec<(PersonId, ProvinceId)> = w.dynasty.list[di]
        .officials
        .iter()
        .map(|&o| (o, cap))
        .chain(w.dynasty.list[di].generals.iter().copied())
        .filter(|&(p, _)| w.persons[p as usize].alive())
        .collect();
    cands.sort_by(|a, b| w.persons[b.0 as usize].attrs.talent.total_cmp(&w.persons[a.0 as usize].attrs.talent).then(a.0.cmp(&b.0)));
    let Some(&(a, core_a)) = cands.first() else {
        super::collapse(w, di);
        return;
    };
    crown(w, di, a);
    {
        let d = &mut w.dynasty.list[di];
        d.mandate = (d.mandate + MANDATE_USURP).clamp(0.0, MANDATE_MAX);
    }
    rec(w, EventKind::Usurpation, 4, Some(cap), &[a], id, None, 0, &cause);
    if let Some(&(b, core_b)) = cands.get(1) {
        let gap = w.persons[a as usize].attrs.talent - w.persons[b as usize].attrs.talent;
        if gap <= CIVIL_WAR_GAP && w.rng.dynasty.gen::<f32>() < CIVIL_WAR_CHANCE {
            super::civil_war(w, di, core_a, b, core_b);
        }
    }
}

/// GDD 3.7 國策與詔令的年度效果：尊道使疆域宗門聲望修正 +2；尊儒使疆域正氣 +0.01；禁道期間宗門聲望修正 −5。
/// 尊佛（寺院壓制，3.6）與尊儒的書院效果（3.8）尚無對象。
pub(super) fn policy_effects(w: &mut World, di: usize) {
    let id = di as DynastyId;
    let (policy, decree, provs) = {
        let d = &w.dynasty.list[di];
        (d.policy, d.decree, d.provinces.clone())
    };
    if policy == Policy::Ru {
        for &p in &provs {
            field::add(w, FieldId::Righteous, p, REVERE_RU_RIGHT);
        }
    }
    let owner = &w.dynasty;
    for pw in w.powers.iter_mut().filter(|pw| pw.fallen.is_none() && owner.owner_of(pw.seat_province) == Some(id)) {
        if policy == Policy::Tao {
            pw.prestige_mod += REVERE_TAO_SECT_MOD;
        }
        if decree == Decree::BanTao {
            pw.prestige_mod += BAN_TAO_SECT_MOD;
        }
    }
}

/// GDD 3.7 詔令（禁道）：無詔令時，君主道緣中道 < 0.15 且疆域內有聲望 ≥ 100 的宗門，每年 5% 頒布；
/// 宗門與王朝關係 −100。宗門之戰（3.9）、滅佛（3.6）不做。
pub(super) fn decree(w: &mut World, di: usize) {
    let id = di as DynastyId;
    let Some(ruler) = w.dynasty.list[di].ruler else { return };
    if w.dynasty.list[di].decree != Decree::None {
        return;
    }
    let tao = w.souls[w.persons[ruler as usize].soul as usize].affinity[Teaching::Tao as usize];
    if tao >= BAN_TAO_AFFINITY {
        return;
    }
    let owner = &w.dynasty;
    let sects: Vec<PowerId> = w
        .powers
        .iter()
        .filter(|pw| pw.fallen.is_none() && owner.owner_of(pw.seat_province) == Some(id))
        .filter(|pw| crate::dao::effective_prestige(pw) >= BAN_TAO_PRESTIGE)
        .map(|pw| pw.id)
        .collect();
    if sects.is_empty() || w.rng.dynasty.gen::<f32>() >= BAN_TAO_CHANCE {
        return;
    }
    let all: Vec<PowerId> =
        w.powers.iter().filter(|pw| pw.fallen.is_none() && w.dynasty.owner_of(pw.seat_province) == Some(id)).map(|pw| pw.id).collect();
    let d = &mut w.dynasty.list[di];
    d.decree = Decree::BanTao;
    for s in all {
        d.sect_relations.insert(s, BAN_TAO_RELATION);
    }
    let cap = d.capital;
    rec(w, EventKind::BanTao, 4, Some(cap), &[ruler], id, None, 0, &[]);
}

/// GDD 3.7 封禪：天命 ≥ 90 持續 30 年、君主德行 ≥ 8、一位君主一生一次。君主的魂 G +300，天命 +10，都城正氣 +0.2。
/// （君主立功 +50 屬 3.8，尚未實作。）
pub(super) fn fengshan(w: &mut World, di: usize) {
    let id = di as DynastyId;
    let (ruler, high, done, cap) = {
        let d = &w.dynasty.list[di];
        (d.ruler, d.high_years, d.fengshan_by, d.capital)
    };
    let Some(r) = ruler else { return };
    if high < FENGSHAN_YEARS || done == Some(r) || w.persons[r as usize].attrs.virtue < FENGSHAN_VIRTUE {
        return;
    }
    let soul = w.persons[r as usize].soul;
    w.souls[soul as usize].merit += FENGSHAN_MERIT;
    field::add(w, FieldId::Righteous, cap, FENGSHAN_RIGHT);
    let d = &mut w.dynasty.list[di];
    d.mandate = (d.mandate + FENGSHAN_MANDATE_BONUS).min(MANDATE_MAX);
    d.fengshan_by = Some(r);
    rec(w, EventKind::Fengshan, 5, Some(cap), &[r], id, None, 0, &[]);
}

/// 疆域內年滿 `OFFICIAL_MIN_AGE`、無勢力、身分為 `role` 的人，依 `score` 由高到低（同值取 ID 小者）。
fn pool(w: &World, di: usize, role: Role, score: impl Fn(&Person) -> f32) -> Vec<PersonId> {
    let id = di as DynastyId;
    let tick = w.tick;
    let mut v: Vec<(f32, PersonId)> = w
        .alive_ids
        .iter()
        .map(|&i| &w.persons[i as usize])
        .filter(|p| p.role == role && p.power.is_none() && p.age(tick) >= OFFICIAL_MIN_AGE)
        .filter(|p| w.dynasty.owner_of(p.province) == Some(id))
        .map(|p| (score(p), p.id))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|x| x.1).collect()
}

/// GDD 3.7 文臣與武將：員額 `2 + 州數/5`（≤ 10）、`1 + 州數/4`（≤ 12），每年補足缺額，無人可補維持缺額。
/// 文臣來源（書院 3.8 尚無）取疆域內的散儒，依「德行 + 才略」；武將來源取疆域內的武將候補，依才略。
/// 文臣駐都城；武將依序駐守與敵國接壤的州 → 與無主州接壤的州 → 都城。
pub(super) fn staff(w: &mut World, di: usize) {
    let n = w.dynasty.list[di].provinces.len() as u32;
    let want_off = (OFFICIALS_BASE + n / OFFICIALS_DIV).min(OFFICIALS_MAX) as usize;
    let want_gen = (GENERALS_BASE + n / GENERALS_DIV).min(GENERALS_MAX) as usize;
    let cap = w.dynasty.list[di].capital;
    let have = w.dynasty.list[di].officials.len();
    if have < want_off {
        for pid in pool(w, di, Role::Scholar, |p| p.attrs.virtue + p.attrs.talent).into_iter().take(want_off - have) {
            w.persons[pid as usize].role = Role::Official;
            move_to(w, pid, cap);
            w.dynasty.list[di].officials.push(pid);
        }
    }
    let have = w.dynasty.list[di].generals.len();
    if have < want_gen {
        for pid in pool(w, di, Role::GeneralCandidate, |p| p.attrs.talent).into_iter().take(want_gen - have) {
            w.persons[pid as usize].role = Role::General;
            w.dynasty.list[di].generals.push((pid, cap));
        }
    }
    post_generals(w, di);
}

/// 武將駐地：才略由高到低，依序取「敵國接壤州 → 無主接壤州 → 都城」的駐地名單。
fn post_generals(w: &mut World, di: usize) {
    let id = di as DynastyId;
    let d = &w.dynasty.list[di];
    let (mut enemy, mut free) = (Vec::new(), Vec::new());
    for &p in &d.provinces {
        let nb = &w.provinces[p as usize].neighbors;
        if nb.iter().any(|&n| w.dynasty.owner_of(n).is_some_and(|o| o != id)) {
            enemy.push(p);
        } else if nb.iter().any(|&n| w.dynasty.owner_of(n).is_none() && !w.provinces[n as usize].lost) {
            free.push(p);
        }
    }
    let mut posts = enemy;
    posts.extend(free);
    let cap = d.capital;
    let mut gens: Vec<PersonId> = d.generals.iter().map(|g| g.0).collect();
    gens.sort_by(|&a, &b| w.persons[b as usize].attrs.talent.total_cmp(&w.persons[a as usize].attrs.talent).then(a.cmp(&b)));
    let assigned: Vec<(PersonId, ProvinceId)> = gens.iter().enumerate().map(|(k, &g)| (g, posts.get(k).copied().unwrap_or(cap))).collect();
    for &(g, p) in &assigned {
        move_to(w, g, p);
    }
    w.dynasty.list[di].generals = assigned;
}

/// GDD 3.4 王族生育：君主有在世的配偶，每年 15%、儲君至多 5 名。子女的魂取自等待中、宿緣對象在都城者，
/// 沒有時建立新魂（受有名之人上限限制）。子女身分為王族，不經 12 歲入道。
pub(super) fn royal_birth(w: &mut World, di: usize) {
    let (ruler, heirs, cap) = {
        let d = &w.dynasty.list[di];
        (d.ruler, d.heirs.len(), d.capital)
    };
    let Some(r) = ruler else { return };
    let Some(spouse) = w.persons[r as usize].spouse.filter(|&s| w.persons[s as usize].alive()) else { return };
    if heirs >= ROYAL_MAX || w.rng.dynasty.gen::<f32>() >= ROYAL_BIRTH {
        return;
    }
    let pid = match soul::take_for_birth(w, cap) {
        Some(sid) => soul::rebirth(w, sid, cap),
        None => {
            if soul::living_notables(w) >= soul::notable_cap(w) {
                return;
            }
            soul::spawn_new_life(w, SoulOrigin::New, cap).1
        }
    };
    let p = &mut w.persons[pid as usize];
    p.role = Role::Royal;
    p.path = PathState::Mortal;
    w.persons[r as usize].children.push(pid);
    w.persons[spouse as usize].children.push(pid);
    w.dynasty.list[di].heirs.push(pid);
}
