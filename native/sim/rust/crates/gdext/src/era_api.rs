//! GDD 5.10 天下冊「世界簡史」的橋接查詢：紀元表。全部唯讀；放在 `SimWorld` 的第二個 `impl` 區塊。

use godot::prelude::*;

use crate::SimWorld;

#[godot_api(secondary)]
impl SimWorld {
    /// 世界簡史的紀元表，由開天依序排到現在：每筆 `name`（紀元名，不含「紀元」二字）、`kind`（起因性質的鍵，
    /// `genesis`／`cold`／`warm`…）、`start_year`、`end_year`（自開天起的第幾年，含；現在的紀元算到今天）、
    /// `event`（開啟這個紀元的世界大事 id；開天、開局前已過的紀元或事件已被帳修剪時為 -1）。
    #[func]
    fn era_table(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(sim) = self.sim.as_ref() else { return out };
        for row in siming_sim::era::table(sim.world()) {
            let mut d = VarDictionary::new();
            d.set("name", &GString::from(row.name.as_str()));
            d.set("kind", &GString::from(row.kind.key()));
            d.set("start_year", row.start_year as i32);
            d.set("end_year", row.end_year as i32);
            d.set("event", row.event.map_or(-1, |id| id as i64));
            out.push(&d);
        }
        out
    }
}
