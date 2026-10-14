//! GDD 5.12 人物生平事實清單的驗收：跑一個世界，逐人檢查清單的規則；三種新紀錄在真實推演中都會出現。

use std::collections::BTreeSet;

use siming_sim::chronicle::{chronicle, Entry};
use siming_sim::{EventKind, Sim, WorldParams};

fn world() -> Sim {
    let mut p = WorldParams::new(20261013);
    p.pop_years = 3000;
    p.prerun_years = 700;
    Sim::new_seeded(&p, Some(7)).unwrap()
}

fn between(list: &[Entry], from: usize, to: usize, kind: &str) -> bool {
    list[from + 1..to].iter().any(|e| e.kind == kind)
}

#[test]
fn the_three_new_records_happen_in_a_real_run() {
    let sim = world();
    let w = sim.world();
    let count = |k: EventKind| w.ledger.events.iter().filter(|e| e.kind == k).count();
    assert!(count(EventKind::SectJoined) > 0, "散修投靠");
    assert!(count(EventKind::MasterEnded) > 0, "師徒解除");
    let master_died = w.ledger.events.iter().filter(|e| e.kind == EventKind::MasterEnded && e.why.code() == "master_died").count();
    assert!(master_died > 0, "師亡");
}

#[test]
fn chronicle_rules_hold_for_everyone() {
    let sim = world();
    let w = sim.world();
    let mut checked = 0;
    let mut with_past = 0;
    let mut same_name = 0;
    for p in &w.persons {
        let (a, b) = (chronicle(w, p.id).unwrap(), chronicle(w, p.id).unwrap());
        assert_eq!(a, b, "同一人再叫一次結果相同");
        let list = &a.entries;
        let ids: BTreeSet<_> = list.iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), list.len(), "人物 {} 的事件編號重複", p.id);
        assert!(list.windows(2).all(|x| (x[0].tick, x[0].id) <= (x[1].tick, x[1].id)), "由舊到新");
        // 再婚之前要有上一段婚姻的結束。
        let marriages: Vec<usize> = (0..list.len()).filter(|&i| list[i].kind == "married").collect();
        for pair in marriages.windows(2) {
            assert!(between(list, pair[0], pair[1], "widowed"), "人物 {} 再婚前沒有喪偶", p.id);
        }
        // 再次拜師之前要有結束的原因（死去超過百年者重要度 1 的紀錄已修剪，不查）。
        let recent = p.died.is_none_or(|(t, _)| t + 100 * 360 > w.tick);
        let masters: Vec<usize> = (0..list.len()).filter(|&i| list[i].kind == "apprenticed" && list[i].role == "disciple").collect();
        // 在世者的紀錄不修剪；已歿者重要度 1 的紀錄超過百年就修剪（死後十年內複查），所以只查前一段拜師仍在百年內的。
        for pair in masters.windows(2) {
            if p.alive() || list[pair[0]].tick + 100 * 360 > w.tick {
                let ended = list[pair[0] + 1..pair[1]].iter().any(|e| e.kind == "mentor_ended" && e.role == "disciple");
                assert!(ended, "人物 {} 換師前沒有結束原因", p.id);
            }
        }
        for e in list {
            // 宿緣的後果要帶得出前因與雙方當時的名字。
            if matches!(e.kind, "thread_omen" | "thread_resolved" | "feud_escalated") {
                let b = e.because.as_ref().unwrap_or_else(|| panic!("人物 {} 的 {} 沒有前因", p.id, e.kind));
                assert!(b.tick <= e.tick);
                if recent {
                    assert!(!b.me.name.is_empty() && !b.other.name.is_empty(), "人物 {} 的 {} 前因缺名字", p.id, e.kind);
                }
            }
            if e.kind == "mentor_ended" || e.kind == "widowed" {
                assert!(!e.end_reason.is_empty());
            }
            // 前世的事不混進今生。
            assert!(e.tick as i64 >= p.born as i64, "人物 {} 的清單含出生之前的事", p.id);
        }
        if let Some(pl) = &a.past_life {
            with_past += 1;
            same_name += usize::from(pl.name == p.name); // 名字隨機抽取，偶有撞名
        }
        checked += 1;
    }
    assert!(same_name * 50 <= with_past, "前世與今生同名太多：{same_name}/{with_past}");
    assert!(checked > 100 && with_past > 5, "樣本太少：{checked} 人、{with_past} 人有前世");
}
