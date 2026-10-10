//! GDD 6.4 呈現層進入點。

mod bridge;
mod dev;
mod devshot;
mod input;
mod render;
mod state;
mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use siming_sim::{Sim, WorldParams};

/// 視窗大小；`SIMING_WINDOW=1280x800` 供 Deck 版面驗收（預設 1600×900）。
fn main() {
    // wasm32 沒有環境變數與命令列參數，也沒有 SystemTime：使用固定種子。
    #[cfg(target_arch = "wasm32")]
    let (seed, dev_world) = (42u64, false);
    #[cfg(not(target_arch = "wasm32"))]
    let (seed, dev_world) = (
        std::env::args()
            .skip_while(|a| a != "--seed")
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(42)
            }),
        std::env::var("SIMING_DEV_WORLD").is_ok(),
    );
    let sim = if dev_world {
        dev::synthetic_sim(seed)
    } else {
        match Sim::new(&WorldParams::new(seed)) {
            Ok(sim) => sim,
            // wasm 沒有主控台可看 panic 訊息；世界生成失敗時退回合成世界以便檢視呈現層。
            #[cfg(target_arch = "wasm32")]
            Err(_) => dev::synthetic_sim(seed),
            #[cfg(not(target_arch = "wasm32"))]
            Err(e) => panic!("世界生成失敗：{e:?}"),
        }
    };

    // 網頁版靜態伺服器沒有 .meta 檔：不檢查，避免對每個資產多發一次 404 請求。
    #[cfg(target_arch = "wasm32")]
    let asset_plugin = bevy::asset::AssetPlugin {
        meta_check: bevy::asset::AssetMetaCheck::Never,
        ..default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let asset_plugin = bevy::asset::AssetPlugin::default();

    App::new()
        .add_plugins(DefaultPlugins.set(asset_plugin).set(WindowPlugin {
            primary_window: Some(Window {
                title: "司業命緣 Siming: Karma & Fate".into(),
                resolution: window_size().into(),
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#bevy".into()),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                #[cfg(target_arch = "wasm32")]
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        .insert_resource(bridge::SimRes(sim))
        .init_resource::<state::MapMode>()
        .init_resource::<state::TimeControl>()
        .init_resource::<state::Selection>()
        .init_resource::<state::MapView>()
        .init_resource::<state::FlyTo>()
        .add_event::<state::SimAdvanced>()
        .add_plugins((bridge::SimPlugin, render::MapPlugin, input::CameraPlugin, input::InputPlugin, ui::UiPlugin))
        .add_plugins(devshot::DevShotPlugin)
        .run();
}

/// 視窗大小；開發／驗收可用 `SIMING_WINDOW=寬x高` 覆寫（例：1920x1080 截圖）。
fn window_size() -> (f32, f32) {
    std::env::var("SIMING_WINDOW")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        })
        .unwrap_or((1600.0, 900.0))
}
