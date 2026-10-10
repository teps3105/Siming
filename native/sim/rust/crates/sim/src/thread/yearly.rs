//! GDD 6.3 步驟 14：宿緣的年度處理——師徒超越、相遇與結果、預兆。

use smallvec::smallvec;

use super::*;

/// GDD 6.3 步驟 14：
/// 1. 師徒線：徒弟今生成就超過 `master_best`（且 `lives ≥ 1`）→ 了結。
/// 2. 相遇（3.3）：每年依魂 ID 升冪；每個魂最多 `MEET_PER_SOUL_PER_YEAR` 次相遇（計「實際發生」的相遇）；
///    每個魂的線依強度由高到低、同強度取 ThreadId 小者；一條線的兩端同一年只判定一次。
/// 3. 預兆。
pub fn yearly(w: &mut World) {
    if w.souls.is_empty() || w.threads.is_empty() {
        return;
    }
    mentor_surpass(w);
    meetings(w);
    omens(w);
}

fn mentor_surpass(w: &mut World) {
    for i in 0..w.threads.len() {
        let t = &w.threads[i];
        if t.kind != ThreadKind::Mentor || !is_open(t) {
            continue;
        }
        let Some(best) = t.master_best else { continue };
        let Some(d) = body(w, t.debtor) else { continue };
        if !exceeds(body_achievements(d), best) {
            continue;
        }
        let t = w.threads[i].clone();
        if lives(w, &t) >= 1 {
            resolve(w, i as ThreadId, Resolution::Other, None);
        }
    }
}

fn meetings(w: &mut World) {
    let n = w.souls.len();
    let mut met = vec![0u32; n];
    // 效能（6.8）：魂的 `threads` 含歷來所有線（含已了結者），逐魂過濾會隨歷史變慢。
    // 年初先把「仍未了」的線依魂分桶（CSR）；線一旦了結或消散就不會復活，而年中新建的線
    // 一定接在魂 `threads` 的尾端（ThreadId ≥ `start_len`），所以「桶 + 尾端」再過濾一次 `is_open`，
    // 與逐魂過濾全部 `threads` 得到相同的集合；排序鍵是全序，結果與舊寫法逐位元相同。
    let start_len = w.threads.len();
    let mut off = vec![0u32; n + 1];
    // 兩端都須有非幼年的肉身、且相距 ≤ `MEET_RANGE_STEPS` 才可能相遇（天道不相遇）。年中只會有人死亡，
    // 不會有人出生、長大或換州（相遇的結果不含遷徙），所以年初不符合的線整年都會在原寫法的
    // `body`／`distance` 檢查被略過（無副作用），可以直接不放進桶。
    let prov: Vec<Option<ProvinceId>> =
        (0..n).map(|s| if s == HEAVEN_SOUL as usize { None } else { body(w, s as SoulId).map(|p| p.province) }).collect();
    let ends = |t: &Thread| {
        let (d, c) = (t.debtor as usize, t.creditor as usize);
        let (Some(Some(pd)), Some(Some(pc))) = (prov.get(d), prov.get(c)) else { return None };
        distance(w, *pd, *pc, MEET_RANGE_STEPS).map(|_| (d, c))
    };
    for t in w.threads.iter().filter(|t| is_open(t)) {
        if let Some((d, c)) = ends(t) {
            off[d + 1] += 1;
            off[c + 1] += 1;
        }
    }
    for i in 0..n {
        off[i + 1] += off[i];
    }
    let mut fill = off.clone();
    let mut flat: Vec<ThreadId> = vec![0; off[n] as usize];
    for (i, t) in w.threads.iter().enumerate().filter(|(_, t)| is_open(t)) {
        if let Some((d, c)) = ends(t) {
            flat[fill[d] as usize] = i as ThreadId;
            fill[d] += 1;
            flat[fill[c] as usize] = i as ThreadId;
            fill[c] += 1;
        }
    }
    let mut done: Vec<bool> = vec![false; start_len];
    for sid in 1..n {
        if met[sid] >= MEET_PER_SOUL_PER_YEAR || body(w, sid as SoulId).is_none() {
            continue;
        }
        let bucket = &flat[off[sid] as usize..off[sid + 1] as usize];
        let all = &w.souls[sid].threads;
        let tail = &all[all.partition_point(|&t| (t as usize) < start_len)..];
        let mut cands: Vec<ThreadId> = bucket
            .iter()
            .chain(tail.iter())
            .copied()
            .filter(|&t| w.threads.get(t as usize).is_some_and(is_open))
            .collect();
        cands.sort_by(|&a, &b| {
            let (sa, sb) = (w.threads[a as usize].strength, w.threads[b as usize].strength);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b))
        });
        for tid in cands {
            if met[sid] >= MEET_PER_SOUL_PER_YEAR {
                break;
            }
            if done.get(tid as usize).copied().unwrap_or(false) {
                continue;
            }
            let t = w.threads[tid as usize].clone();
            if !is_open(&t) || t.debtor == HEAVEN_SOUL || t.creditor == HEAVEN_SOUL {
                continue;
            }
            let other = if t.debtor as usize == sid { t.creditor } else { t.debtor };
            if met[other as usize] >= MEET_PER_SOUL_PER_YEAR {
                continue;
            }
            let (Some(pa), Some(pb)) = (body(w, t.debtor), body(w, t.creditor)) else { continue };
            if t.kind == ThreadKind::Love && pa.spouse == Some(pb.id) {
                continue;
            }
            let Some(dist) = distance(w, pa.province, pb.province, MEET_RANGE_STEPS) else { continue };
            if done.len() <= tid as usize {
                done.resize(tid as usize + 1, false);
            }
            done[tid as usize] = true;
            let mut chance = MEET_CHANCE_COEF * t.strength / MEET_CHANCE_STRENGTH_DIV;
            if dist == 0 {
                chance *= MEET_SAME_PROVINCE_MULT;
            }
            if !roll(w, chance) {
                continue;
            }
            met[t.debtor as usize] += 1;
            met[t.creditor as usize] += 1;
            meet(w, tid);
        }
    }
}

fn meet(w: &mut World, tid: ThreadId) {
    match w.threads[tid as usize].kind {
        ThreadKind::Feud | ThreadKind::Grudge => clash(w, tid),
        ThreadKind::Grace => repay(w, tid),
        ThreadKind::Love => love(w, tid),
        ThreadKind::Mentor => mentor(w, tid),
    }
}

/// 殺業、怨：50% 爆發衝突；勝率 `r/(1+r)`，r = 債主戰力 / 欠債方戰力（分母下限 1）。
/// 債主勝：殺死欠債方（復仇，由 [`on_kill`] 了結）；欠債方勝：殺死債主（冤冤相報）。
fn clash(w: &mut World, tid: ThreadId) {
    if !roll(w, FEUD_CLASH_CHANCE) {
        return;
    }
    let t = w.threads[tid as usize].clone();
    let (Some(d), Some(c)) = (body(w, t.debtor), body(w, t.creditor)) else { return };
    let (did, cid) = (d.id, c.id);
    let r = crate::people::combat_power(c) / crate::people::combat_power(d).max(1.0);
    let creditor_wins = roll(w, r / (1.0 + r));
    if creditor_wins {
        kill_person(w, cid, did, Some(t.created.1));
    } else {
        kill_person(w, did, cid, Some(t.created.1));
    }
}

/// 恩義：50% 報恩。受恩者（欠債方）報恩：債主為修士時修為 +10%；了結。
fn repay(w: &mut World, tid: ThreadId) {
    if !roll(w, GRACE_REPAY_CHANCE) {
        return;
    }
    let creditor = w.threads[tid as usize].creditor;
    if let Some(SoulState::Incarnate(pid)) = w.souls.get(creditor as usize).map(|s| s.state.clone()) {
        if let PathState::Cultivator { exp, .. } = &mut w.persons[pid as usize].path {
            *exp *= 1.0 + GRACE_CULT_BONUS;
        }
    }
    resolve(w, tid, Resolution::Other, None);
}

/// 情緣：年齡差 ≤ 10 且皆未婚 60%，否則 10%。成功且 `lives ≥ 1`、雙方成年未婚 → 結為夫妻（再續前緣，了結）。
/// 其餘（含同世）為有緣無分：強度 −10，降到 0 線消散。
fn love(w: &mut World, tid: ThreadId) {
    let t = w.threads[tid as usize].clone();
    let (Some(a), Some(b)) = (body(w, t.debtor), body(w, t.creditor)) else { return };
    let (aid, bid) = (a.id, b.id);
    let single = a.spouse.is_none() && b.spouse.is_none();
    let gap = a.age(w.tick).abs_diff(b.age(w.tick));
    let adult = a.age(w.tick) >= MARRY_AGE && b.age(w.tick) >= MARRY_AGE;
    let chance = if gap <= LOVE_NEAR_AGE_GAP && single { LOVE_UNITE_CHANCE_NEAR_AGE } else { LOVE_UNITE_CHANCE_FAR_AGE };
    let unite = roll(w, chance);
    if unite && lives(w, &t) >= 1 && single && adult {
        crate::people::marry(w, aid, bid);
        return;
    }
    let s = &mut w.threads[tid as usize];
    s.strength += LOVE_FAIL_STRENGTH_PENALTY;
    if s.strength <= 0.0 {
        fade(w, tid);
    }
}

fn teaching_of(p: &Person) -> Option<Teaching> {
    match p.role {
        Role::Cultivator => Some(Teaching::Tao),
        Role::Monk => Some(Teaching::Buddha),
        Role::Scholar => Some(Teaching::Ru),
        Role::Demonic => Some(Teaching::Demon),
        _ => None,
    }
}

/// 師徒：同教時 70% 拜入對方門下；戰力較高者為師（相同取債主）。
/// 身分允許：師父非幼年、徒弟尚無師父、師父徒弟 < 5，修士師父境界 ≥ 金丹，雙方尚非師徒。
fn mentor(w: &mut World, tid: ThreadId) {
    let t = w.threads[tid as usize].clone();
    let (Some(d), Some(c)) = (body(w, t.debtor), body(w, t.creditor)) else { return };
    let (Some(td), Some(tc)) = (teaching_of(d), teaching_of(c)) else { return };
    if td != tc {
        return;
    }
    let (master, disciple) = if crate::people::combat_power(d) > crate::people::combat_power(c) { (d, c) } else { (c, d) };
    let (mid, did) = (master.id, disciple.id);
    let elder_ok = match master.path {
        PathState::Cultivator { realm, .. } => realm >= SECT_ELDER_MIN_REALM,
        _ => true,
    };
    if disciple.master.is_some()
        || master.master == Some(did)
        || master.disciples.len() as u32 >= MAX_DISCIPLES
        || !elder_ok
    {
        return;
    }
    if !roll(w, MASTER_JOIN_CHANCE) {
        return;
    }
    let (ms, ds) = (w.persons[mid as usize].soul, w.persons[did as usize].soul);
    w.persons[did as usize].master = Some(mid);
    w.persons[mid as usize].disciples.push(did);
    w.souls[ms as usize].merit += MASTER_TEACH_MERIT;
    on_apprentice(w, ds, ms, None);
}

/// 預兆（3.3）：線兩端在世、距離 ≤ 2、強度 ≥ 50，且 `omen_at ≠ lives` → `ThreadOmen`，記下 `omen_at = lives`。
pub(super) fn omens(w: &mut World) {
    for i in 0..w.threads.len() {
        let t = &w.threads[i];
        if !is_open(t) || t.strength < OMEN_MIN_STRENGTH || t.debtor == HEAVEN_SOUL || t.creditor == HEAVEN_SOUL {
            continue;
        }
        let t = t.clone();
        let n = lives(w, &t);
        if t.omen_at == Some(n) {
            continue;
        }
        let (Some(a), Some(b)) = (body(w, t.debtor), body(w, t.creditor)) else { continue };
        let (pa, pb, ia, ib) = (a.province, b.province, a.id, b.id);
        if distance(w, pa, pb, OMEN_RANGE_STEPS).is_none() {
            continue;
        }
        w.threads[i].omen_at = Some(n);
        let mut e = NewEvent::new(EventKind::ThreadOmen, 3, EventArgs::Thread { thread: t.id, kind: t.kind, lives: n });
        e.province = Some(pa);
        e.cell = Some(w.persons[ia as usize].cell);
        e.souls = smallvec![t.debtor, t.creditor];
        e.persons = smallvec![ia, ib];
        e.causes = smallvec![t.created.1];
        ledger::record(w, e);
    }
}
