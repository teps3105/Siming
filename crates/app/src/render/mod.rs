//! GDD 5.3 地圖渲染（T8）：地形網格、水墨材質、世界貼圖、靈脈圖層。
pub mod globe;
mod ley;
mod material;
mod mesh;
pub(crate) mod palette;
mod textures;

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::NoFrustumCulling;

use crate::bridge::SimRes;
use crate::input::MapCamera;
use crate::state::{camera_pitch, MapMode, MapView, Selection};
use material::{InkParams, LeyMaterial, TerrainInkMaterial};
pub use mesh::TerrainCpu;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin::<TerrainInkMaterial>::default(), MaterialPlugin::<LeyMaterial>::default()))
            .insert_resource(ClearColor(Color::LinearRgba(palette::lin_color(palette::PAPER))))
            .add_systems(Startup, setup_map)
            .add_systems(PostUpdate, (update_province_colors, update_materials, update_copies));
    }
}

#[derive(Resource)]
struct MapAssets {
    terrain_mat: Handle<TerrainInkMaterial>,
    line_mat: Handle<LeyMaterial>,
    disc_mat: Handle<LeyMaterial>,
    color_image: Handle<Image>,
    assignment: Vec<usize>,
}

#[derive(Component)]
struct LeyLayer;

/// 東西無縫（GDD 5.3）：每個圖層預先生成 `COPY_SLOTS` 份，依視野逐幀指派副本編號與位置。
#[derive(Component)]
struct MapCopy {
    slot: usize,
}

const COPY_SLOTS: usize = 4;

#[allow(clippy::too_many_arguments)]
fn setup_map(
    mut commands: Commands,
    sim: Res<SimRes>,
    mut mode: ResMut<MapMode>,
    mut selection: ResMut<Selection>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<TerrainInkMaterial>>,
    mut ley_mats: ResMut<Assets<LeyMaterial>>,
) {
    // 開發／驗收用環境變數
    if let Ok(m) = std::env::var("SIMING_MAP_MODE") {
        *mode = match m.to_ascii_lowercase().as_str() {
            "tianxia" => MapMode::Tianxia,
            "lingmai" => MapMode::Lingmai,
            _ => MapMode::Shanhe,
        };
    }
    for (var, hov) in [("SIMING_SELECT", false), ("SIMING_HOVER", true)] {
        if let Some(id) = std::env::var(var).ok().and_then(|s| s.parse::<u16>().ok()) {
            if hov {
                selection.hovered = Some(id);
            } else {
                selection.selected = Some(id);
            }
        }
    }

    let world = sim.0.world();
    let geo = &world.geo;
    let cpu = TerrainCpu::new(geo);
    let terrain_mesh = mesh::build_terrain_mesh(&cpu);

    let assignment = textures::color_provinces(&world.provinces);
    let table = textures::province_color_table(*mode, &world.provinces, &assignment, palette::lin(palette::PAPER));
    let wi = textures::build_world_images(geo, &world.provinces, &table);

    let paper = images.add(textures::gen_paper());
    let stroke = images.add(textures::gen_stroke());
    let mut color_img = wi.province_color;
    color_img.asset_usage = RenderAssetUsages::default(); // 之後模式切換要在主世界改寫
    let color_image = images.add(color_img);

    let mut params = InkParams::new(geo.w as f32, geo.h as f32, cpu.max_height, geo.wrap_x);
    params.view.z = *mode as u8 as f32;
    let terrain_mat = mats.add(TerrainInkMaterial {
        params,
        paper,
        stroke,
        cls_a: images.add(wi.cls_a),
        cls_b: images.add(wi.cls_b),
        province_id: images.add(wi.province_id),
        province_color: color_image.clone(),
        field: images.add(wi.field),
        terrain: images.add(wi.terrain),
    });

    let terrain_mesh = meshes.add(terrain_mesh);
    for slot in 0..COPY_SLOTS {
        commands.spawn((
            Mesh3d(terrain_mesh.clone()),
            MeshMaterial3d(terrain_mat.clone()),
            Transform::default(),
            NoFrustumCulling,
            MapCopy { slot },
        ));
    }

    // 靈脈圖層
    let (line_mesh, disc_mesh) = ley::build_ley_meshes(geo, &cpu);
    let line_mat = ley_mats.add(LeyMaterial::new(false));
    let disc_mat = ley_mats.add(LeyMaterial::new(true));
    let vis = if *mode == MapMode::Lingmai { Visibility::Inherited } else { Visibility::Hidden };
    let (line_mesh, disc_mesh) = (meshes.add(line_mesh), meshes.add(disc_mesh));
    for slot in 0..COPY_SLOTS {
        commands.spawn((
            Mesh3d(line_mesh.clone()),
            MeshMaterial3d(line_mat.clone()),
            Transform::default(),
            vis,
            NoFrustumCulling,
            LeyLayer,
            MapCopy { slot },
        ));
        commands.spawn((
            Mesh3d(disc_mesh.clone()),
            MeshMaterial3d(disc_mat.clone()),
            Transform::default(),
            vis,
            NoFrustumCulling,
            LeyLayer,
            MapCopy { slot },
        ));
    }

    commands.insert_resource(cpu);
    commands.insert_resource(MapAssets { terrain_mat, line_mat, disc_mat, color_image, assignment });
}

/// 地圖模式改變 → 重算 `province_color` 查表並上傳（GDD 6.4 可視同步）。
fn update_province_colors(
    mode: Res<MapMode>,
    sim: Res<SimRes>,
    assets: Option<Res<MapAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<Option<MapMode>>,
) {
    let Some(assets) = assets else { return };
    if *last == Some(*mode) {
        return;
    }
    let first = last.is_none();
    *last = Some(*mode);
    if first {
        return; // 啟動時已依當前模式建好
    }
    let world = sim.0.world();
    let table = textures::province_color_table(*mode, &world.provinces, &assets.assignment, palette::lin(palette::PAPER));
    if let Some(img) = images.get_mut(&assets.color_image) {
        let mut bytes = Vec::with_capacity(1024);
        for c in table {
            bytes.extend_from_slice(&c);
        }
        img.data = Some(bytes);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_materials(
    mode: Res<MapMode>,
    view: Res<MapView>,
    selection: Res<Selection>,
    time: Res<Time>,
    assets: Option<Res<MapAssets>>,
    sim: Res<SimRes>,
    cam: Query<&Transform, With<MapCamera>>,
    mut mats: ResMut<Assets<TerrainInkMaterial>>,
    mut ley_mats: ResMut<Assets<LeyMaterial>>,
) {
    let Some(assets) = assets else { return };
    let geo = &sim.0.world().geo;
    let map_w = geo.w as f32;
    let t = time.elapsed_secs();
    if let Some(m) = mats.get_mut(&assets.terrain_mat) {
        let p = &mut m.params;
        p.view = Vec4::new(view.height_scale, view.flatten, *mode as u8 as f32, t);
        let id = |s: Option<u16>| s.map_or(-1.0, |v| v as f32);
        p.sel.x = id(selection.hovered);
        p.sel.y = id(selection.selected);
        let pitch = camera_pitch(view.camera_z, map_w);
        let dist = view.camera_z / pitch.sin();
        p.fog.x = dist * 1.5;
        p.fog.y = dist * 3.5;
        // GDD 5.3 星球視角：過渡權重與球面框架（球心 = 注視點，焦點經緯正對鏡頭）
        p.globe.x = view.globe;
        if view.globe > 0.0 {
            let rot = cam.single().map_or(Quat::IDENTITY, |t| t.rotation);
            p.grot = globe::globe_frame(view.focus, Vec2::new(map_w, geo.h as f32), rot);
        }
    }
    let width = (view.camera_z * 0.004).max(0.35);
    for h in [&assets.line_mat, &assets.disc_mat] {
        if let Some(m) = ley_mats.get_mut(h) {
            m.params.view.x = view.height_scale;
            m.params.view.y = width;
            m.params.view.z = t;
        }
    }
}

/// 東西無縫：依目前視野把各圖層副本排到所需的 x 偏移（通常 2–3 份），其餘隱藏。
/// 星球視角（過渡完成）只需一份。
fn update_copies(
    mode: Res<MapMode>,
    view: Res<MapView>,
    sim: Res<SimRes>,
    mut q: Query<(&MapCopy, &mut Transform, &mut Visibility, Option<&LeyLayer>)>,
) {
    let geo = &sim.0.world().geo;
    let w = geo.w as f32;
    let fx = view.focus.x;
    // 視野的水平半徑（保守估計：遠側地面距離與橫向展開，假設最寬 2.4:1 的畫面）+ 靈脈線／標記的外溢邊距。
    let pitch = camera_pitch(view.camera_z, w);
    let edge = (pitch - 15f32.to_radians()).max(5f32.to_radians());
    let fwd = view.camera_z / edge.tan();
    let half = view.camera_z / edge.sin() * 15f32.to_radians().tan() * 2.4;
    let reach = fwd.hypot(half) + 4.0;
    let (k_lo, k_hi) = if !geo.wrap_x {
        (0, 0)
    } else if view.globe >= 0.999 {
        let k = (fx / w).floor() as i32;
        (k, k)
    } else {
        let lo = ((fx - reach) / w).floor() as i32;
        let hi = (((fx + reach) / w).floor() as i32).min(lo + COPY_SLOTS as i32 - 1);
        (lo, hi)
    };
    let ley_on = *mode == MapMode::Lingmai && view.globe < 0.01;
    for (copy, mut tf, mut vis, ley) in &mut q {
        let k = k_lo + copy.slot as i32;
        let active = k <= k_hi;
        let x = k as f32 * w;
        if tf.translation.x != x {
            tf.translation.x = x;
        }
        let want = if active && (ley.is_none() || ley_on) { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
    }
}
