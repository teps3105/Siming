//! 事件隊列：世界持有的待發生事件，依日序排列的二元堆。
//!
//! 事件驅動引擎的骨幹。規則算出「某件事會在第幾日發生」就排進來，推進時取出到期者執行，
//! 不再每月掃描全體。取消用惰性法：[`EventQueue::cancel`] 只把句柄從「有效集合」移除，堆裡的殘條
//! 在浮到堆頂時才丟棄，所以取消與重排都是 O(log n)。
//!
//! 排序鍵是 `(日, 流水號)`：同一日的事件依排入順序執行，結果與堆的內部排列無關。

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeSet, BinaryHeap};

use serde::{Deserialize, Serialize};

use crate::calendar::Day;

/// 排進隊列的事件做什麼。規則陸續從月步進移植過來時在此新增變體。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// 標記事件：執行時什麼都不做，只計入 [`EventQueue::fired`]。測試與工具用。
    Marker(u32),
    /// 某人的個人規則（見 [`PersonAct`]）。
    Person(u32, PersonAct),
    /// 某個魂的狀態到期（中陰、他道、等待）。
    Soul(u32),
}

/// 個人規則的事件種類。算得出時刻者（壽終、入道、遊歷返回）直接排定；
/// 隨機觸發者（遊歷出發、婚配）每人各抽一個指數等待時間；年度結算（修煉、突破、心魔、斬妖）在每人固定的日子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersonAct {
    /// 壽元已盡（出生日 + 壽元）。壽元之後延長則在觸發時重排。
    Age,
    /// 滿 12 歲後入道。
    ComeOfAge,
    /// 遊歷期滿返回。
    Return,
    /// 遊歷出發的抽中。
    Travel,
    /// 婚配的抽中。
    Marry,
    /// 修士與魔修的年度結算：突破、修煉、心魔、斬妖或襲殺、劫雲預兆。
    Year,
}

/// 排程的句柄，用來取消或重排。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Handle(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    day: Day,
    seq: u64,
    action: Action,
}

impl PartialEq for Entry {
    fn eq(&self, o: &Self) -> bool {
        (self.day, self.seq) == (o.day, o.seq)
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Entry {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.day, self.seq).cmp(&(o.day, o.seq))
    }
}

/// 待發生事件隊列。屬於世界狀態，隨存檔保存。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventQueue {
    heap: BinaryHeap<Reverse<Entry>>,
    /// 仍有效（未取消、未執行）的句柄。
    live: BTreeSet<u64>,
    next_seq: u64,
    /// 已執行的標記事件數（測試與工具用）。
    pub fired: u64,
}

impl EventQueue {
    /// 排入一件事，回傳句柄。
    pub fn schedule(&mut self, day: Day, action: Action) -> Handle {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.live.insert(seq);
        self.heap.push(Reverse(Entry { day, seq, action }));
        Handle(seq)
    }

    /// 取消。句柄已執行或已取消時回傳 `false`。
    pub fn cancel(&mut self, h: Handle) -> bool {
        self.live.remove(&h.0)
    }

    /// 取消舊的並排入新的（狀態改變而重排時用）。
    pub fn reschedule(&mut self, h: Handle, day: Day, action: Action) -> Handle {
        self.cancel(h);
        self.schedule(day, action)
    }

    /// 句柄是否仍有效。
    pub fn is_live(&self, h: Handle) -> bool {
        self.live.contains(&h.0)
    }

    /// 有效事件數。
    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    /// 丟掉堆頂已取消的殘條。
    fn trim(&mut self) {
        while let Some(Reverse(e)) = self.heap.peek() {
            if self.live.contains(&e.seq) {
                break;
            }
            self.heap.pop();
        }
    }

    /// 最早一件有效事件的日序。
    pub fn peek_day(&mut self) -> Option<Day> {
        self.trim();
        self.heap.peek().map(|Reverse(e)| e.day)
    }

    /// 取出最早一件有效事件；它的日序不晚於 `until` 才取。
    pub fn pop_due(&mut self, until: Day) -> Option<(Day, Action)> {
        self.trim();
        if self.heap.peek()?.0.day > until {
            return None;
        }
        let Reverse(e) = self.heap.pop()?;
        self.live.remove(&e.seq);
        Some((e.day, e.action))
    }

    /// 堆內條目數（含尚未丟棄的殘條；監看殘條累積用）。
    pub fn heap_len(&self) -> usize {
        self.heap.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_day_then_insertion_order() {
        let mut q = EventQueue::default();
        q.schedule(10, Action::Marker(1));
        q.schedule(5, Action::Marker(2));
        q.schedule(10, Action::Marker(3));
        q.schedule(7, Action::Marker(4));
        let mut got = Vec::new();
        while let Some((d, Action::Marker(m))) = q.pop_due(u32::MAX) {
            got.push((d, m));
        }
        assert_eq!(got, vec![(5, 2), (7, 4), (10, 1), (10, 3)]);
        assert!(q.is_empty());
    }

    #[test]
    fn pop_due_respects_the_limit() {
        let mut q = EventQueue::default();
        q.schedule(10, Action::Marker(1));
        assert_eq!(q.pop_due(9), None);
        assert_eq!(q.peek_day(), Some(10));
        assert_eq!(q.pop_due(10), Some((10, Action::Marker(1))));
    }

    #[test]
    fn cancelled_events_never_fire() {
        let mut q = EventQueue::default();
        let a = q.schedule(3, Action::Marker(1));
        let b = q.schedule(4, Action::Marker(2));
        assert!(q.cancel(a));
        assert!(!q.cancel(a), "第二次取消回傳 false");
        assert_eq!(q.len(), 1);
        assert_eq!(q.peek_day(), Some(4), "堆頂殘條被丟棄");
        assert!(q.is_live(b));
        assert_eq!(q.pop_due(100), Some((4, Action::Marker(2))));
        assert!(!q.cancel(b), "已執行的句柄取消回傳 false");
        assert_eq!(q.pop_due(100), None);
    }

    #[test]
    fn reschedule_moves_the_event() {
        let mut q = EventQueue::default();
        let a = q.schedule(3, Action::Marker(1));
        let a2 = q.reschedule(a, 9, Action::Marker(1));
        assert!(!q.is_live(a));
        assert!(q.is_live(a2));
        assert_eq!(q.pop_due(8), None);
        assert_eq!(q.pop_due(9), Some((9, Action::Marker(1))));
    }

    #[test]
    fn serde_round_trip_keeps_order_and_cancellations() {
        let mut q = EventQueue::default();
        let a = q.schedule(3, Action::Marker(1));
        q.schedule(2, Action::Marker(2));
        q.schedule(8, Action::Marker(3));
        q.cancel(a);
        let s = serde_json::to_string(&q).unwrap();
        let mut r: EventQueue = serde_json::from_str(&s).unwrap();
        let mut got = Vec::new();
        while let Some((d, Action::Marker(m))) = r.pop_due(u32::MAX) {
            got.push((d, m));
        }
        assert_eq!(got, vec![(2, 2), (8, 3)]);
    }
}
