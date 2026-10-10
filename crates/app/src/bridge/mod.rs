//! GDD 6.4 模擬驅動（T9）。持有 `SimRes`，依 `TimeControl` 推進。
//!
//! 每幀預算：依速度算出應推進月數，呼叫 `step()` 直到完成或用掉 8 ms；
//! 未完成者累積到下一幀，累積上限 60 tick，超過即自動降速並提示。
use bevy::platform::time::Instant;
use bevy::prelude::*;
use siming_sim::Sim;

use crate::state::{NoticeLevel, SimAdvanced, SimNotice, TimeControl};
use crate::ui::stream::{self, EventFeed};
use crate::ui::text::EventText;

/// 模擬本體。只有 bridge 會呼叫 `step()`；其他系統唯讀。
#[derive(Resource)]
pub struct SimRes(pub Sim);

/// 每幀 `step()` 的時間預算（秒）。
const FRAME_BUDGET_SECS: f32 = 0.008;
/// 累積 tick 上限。
const BACKLOG_CAP: f32 = 60.0;
/// 提示顯示秒數。
const NOTICE_SECS: f32 = 4.0;

#[derive(Resource, Default)]
struct Accumulator(f32);

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Accumulator>().init_resource::<SimNotice>().add_systems(Update, drive_sim);
    }
}

#[allow(clippy::too_many_arguments)]
fn drive_sim(
    time: Res<Time>,
    mut sim: ResMut<SimRes>,
    mut tc: ResMut<TimeControl>,
    mut acc: ResMut<Accumulator>,
    mut notice: ResMut<SimNotice>,
    mut advanced: EventWriter<SimAdvanced>,
    mut feed: ResMut<EventFeed>,
    etext: Res<EventText>,
) {
    let dt = time.delta_secs().min(0.25);
    // 止級原因簽在續行前不倒數（stream::resume 會把它改為有限值）
    if notice.remaining > 0.0 && notice.remaining.is_finite() {
        notice.remaining -= dt;
    }
    if tc.paused {
        acc.0 = 0.0;
        return;
    }
    if notice.level == NoticeLevel::Stop && notice.remaining.is_infinite() {
        stream::resume(&mut notice);
    }
    acc.0 += tc.speed.months_per_second() * dt;
    let wanted = acc.0.floor() as u32;
    let start = Instant::now();
    let mut done = 0u32;
    while done < wanted {
        sim.0.step();
        done += 1;
        if start.elapsed().as_secs_f32() >= FRAME_BUDGET_SECS {
            break;
        }
    }
    acc.0 -= done as f32;
    if acc.0 > BACKLOG_CAP {
        // 跟不上：降一檔並提示；已是最慢則只截斷累積。
        if let Some(slower) = tc.speed.slower() {
            tc.speed = slower;
            *notice = SimNotice { key: "notice_slowdown", remaining: NOTICE_SECS, ..Default::default() };
        }
        acc.0 = acc.0.fract();
    }
    if done > 0 {
        advanced.write(SimAdvanced { months: done });
        // GDD 5.7 通知分級：新事件入傳聞簽，必要時自動止／緩
        let ids = sim.0.drain_events();
        if !ids.is_empty() {
            let now = time.elapsed_secs_f64();
            stream::ingest(&mut feed, sim.0.world(), &ids, &mut tc, &mut notice, &etext, now);
        }
    } else {
        sim.0.drain_events();
    }
}
