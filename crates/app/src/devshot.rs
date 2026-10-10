//! 開發與驗收用截圖：設定 `SIMING_SCREENSHOT=路徑` 時，於第 N 幀截圖後離開（N 由 `SIMING_SCREENSHOT_FRAME` 指定，預設 180）。
//! 用於容器中以 Xvfb 驗證畫面（GDD 7.9 #20）。

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub struct DevShotPlugin;

#[derive(Resource)]
struct ShotConfig {
    path: String,
    frame: u32,
    count: u32,
}

impl Plugin for DevShotPlugin {
    fn build(&self, app: &mut App) {
        let Ok(path) = std::env::var("SIMING_SCREENSHOT") else { return };
        let frame = std::env::var("SIMING_SCREENSHOT_FRAME").ok().and_then(|s| s.parse().ok()).unwrap_or(180);
        app.insert_resource(ShotConfig { path, frame, count: 0 }).add_systems(Update, shoot);
    }
}

fn shoot(mut commands: Commands, mut cfg: ResMut<ShotConfig>, mut exit: EventWriter<AppExit>) {
    cfg.count += 1;
    if cfg.count == cfg.frame {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(cfg.path.clone()));
    }
    if cfg.count == cfg.frame + 30 {
        exit.write(AppExit::Success);
    }
}
