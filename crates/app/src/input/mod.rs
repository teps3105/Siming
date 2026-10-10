//! GDD 5.2 鏡頭與操作、州郡選取（T8）。
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::window::PrimaryWindow;
use siming_sim::NO_PROVINCE;

use crate::bridge::SimRes;
use crate::render::globe::{globe_frame, globe_radius, ray_globe_uv};
use crate::render::TerrainCpu;
use crate::state::{
    camera_pitch, camera_z_range, flatten_weight, focus_row_limits, nearest_copy_x, FlyTo, InputActions, MapView, Selection, Target,
    GLOBE_Z_FRAC,
};

pub mod actions;
pub use actions::InputPlugin;

use crate::state::FOV_DEG;
/// GDD 5.2：臨界阻尼彈簧時間常數（秒）。
const TAU: f32 = 0.12;
/// 右鍵旋轉的方位角範圍（±45°）。
const YAW_LIMIT: f32 = std::f32::consts::FRAC_PI_4;
/// GDD 5.3：平面 ↔ 球面過渡時間（秒）。
const GLOBE_MORPH_SECS: f32 = 0.6;
/// 開場運鏡（GDD 4.1）：先停在星球視角的秒數、整段秒數、終點鏡頭高度（地圖寬的比例）。
const INTRO_HOLD: f32 = 0.9;
const INTRO_SECS: f32 = 5.5;
const INTRO_END_Z: f32 = 0.30;
/// GDD 5.2：鍵盤平移速度 = 高度 × 0.9／秒。
const KEY_PAN_PER_HEIGHT: f32 = 0.9;
/// GDD 5.7 飛往：鏡頭高於地圖寬 × 此值時先拉近到此高度。
const FLY_Z_FRAC: f32 = 0.18;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (camera_input, camera_smooth, pick_province).chain());
    }
}

#[derive(Component)]
pub struct MapCamera;

/// 開場運鏡：星球視角 → 推近到最顯著的陸塊（任何輸入即中止）。
struct Intro {
    t: f32,
    from_focus: Vec2,
    to_focus: Vec2,
    from_z: f32,
    to_z: f32,
}

#[derive(Resource)]
struct CamRig {
    map: Vec2,
    wrap_x: bool,
    /// 星球過渡的線性進度 0..1（對外以 smoothstep 緩動後寫入 `MapView.globe`）。
    globe_raw: f32,
    intro: Option<Intro>,
    // 目標值（輸入直接寫入）
    focus_t: Vec2,
    z_t: f32,
    yaw_t: f32,
    // 平滑後的值
    focus: Vec2,
    z: f32,
    yaw: f32,
    vel_focus: Vec2,
    vel_z: f32,
    vel_yaw: f32,
    /// Ctrl+1–9 書籤（焦點、高度、方位角）。
    bookmarks: [Option<(Vec2, f32, f32)>; 9],
}

fn damp(cur: f32, target: f32, vel: &mut f32, dt: f32) -> f32 {
    let w = 1.0 / TAU;
    let e = (-w * dt).exp();
    let d = cur - target;
    let tmp = (*vel + w * d) * dt;
    *vel = (*vel - w * tmp) * e;
    target + (d + tmp) * e
}

fn rig_transform(focus: Vec2, z: f32, yaw: f32, map_w: f32) -> Transform {
    let pitch = camera_pitch(z, map_w);
    let off = Quat::from_rotation_y(yaw) * Vec3::new(0.0, z, z / pitch.tan());
    let f = Vec3::new(focus.x, 0.0, focus.y);
    Transform::from_translation(f + off).looking_at(f, Vec3::Y)
}

/// 螢幕游標（邏輯像素、左上為原點）→ 世界射線 (起點, 方向)。
fn cursor_ray(cursor: Vec2, win: Vec2, tf: &Transform) -> (Vec3, Vec3) {
    let ndc = Vec2::new(2.0 * cursor.x / win.x - 1.0, 1.0 - 2.0 * cursor.y / win.y);
    let th = (FOV_DEG.to_radians() * 0.5).tan();
    let dir_cam = Vec3::new(ndc.x * th * win.x / win.y, ndc.y * th, -1.0).normalize();
    (tf.translation, tf.rotation * dir_cam)
}

fn ground_hit(o: Vec3, d: Vec3) -> Option<Vec2> {
    if d.y >= -1e-5 {
        return None;
    }
    let t = -o.y / d.y;
    let p = o + d * t;
    Some(Vec2::new(p.x, p.z))
}

fn spawn_camera(mut commands: Commands, sim: Res<SimRes>) {
    let geo = &sim.0.world().geo;
    let map = Vec2::new(geo.w as f32, geo.h as f32);
    let (zmin, zmax) = camera_z_range(map.x);
    // 初始：整片大陸入鏡（z_max）；可由環境變數覆寫（驗收截圖用）
    let env_z = std::env::var("SIMING_CAMERA_Z").ok().and_then(|s| s.parse::<f32>().ok());
    let z = env_z.unwrap_or(zmax).clamp(zmin, zmax);
    let (row_lo, row_hi) = focus_row_limits(map.x, map.y, z);
    let mut focus = map * 0.5;
    let mut env_focus = false;
    if let Ok(s) = std::env::var("SIMING_CAMERA_FOCUS") {
        let v: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if v.len() == 2 {
            // x 不受限（東西無縫，GDD 5.2）；y 限制在南北緯 75° 內
            focus = Vec2::new(if geo.wrap_x { v[0] } else { v[0].clamp(0.0, map.x) }, v[1].clamp(row_lo, row_hi));
            env_focus = true;
        }
    }
    // GDD 4.1 開場：星球視角 → 推近最顯著的陸塊。以環境變數指定鏡頭（驗收截圖）時略過。
    let intro = if env_z.is_none() && !env_focus {
        let to_z = (map.x * INTRO_END_Z).clamp(zmin, zmax);
        let (end_lo, end_hi) = focus_row_limits(map.x, map.y, to_z);
        let to_focus = prominent_landmass(geo).clamp(Vec2::new(f32::MIN, end_lo), Vec2::new(f32::MAX, end_hi));
        let from_focus = Vec2::new(to_focus.x - map.x * 0.3, map.y * 0.5);
        focus = from_focus;
        Some(Intro { t: 0.0, from_focus, to_focus, from_z: zmax, to_z })
    } else {
        None
    };
    let yaw = std::env::var("SIMING_CAMERA_YAW")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .map_or(0.0, |d| d.to_radians().clamp(-YAW_LIMIT, YAW_LIMIT));
    commands.insert_resource(CamRig {
        map,
        wrap_x: geo.wrap_x,
        globe_raw: if z > map.x * GLOBE_Z_FRAC { 1.0 } else { 0.0 },
        intro,
        focus_t: focus,
        z_t: z,
        yaw_t: yaw,
        focus,
        z,
        yaw,
        vel_focus: Vec2::ZERO,
        vel_z: 0.0,
        vel_yaw: 0.0,
        bookmarks: [None; 9],
    });
    commands.spawn((
        MapCamera,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEG.to_radians(),
            near: (zmin * 0.2).max(0.1),
            far: map.x * 8.0,
            ..default()
        }),
        Tonemapping::None,
        Msaa::Off,
        Exposure::default(),
        rig_transform(focus, z, yaw, map.x),
    ));
}

/// 最顯著的陸塊中心（世界 xz）：最大的大洲的形心；沒有大洲資料時取陸地格的（環狀）平均位置。
fn prominent_landmass(geo: &siming_sim::Geography) -> Vec2 {
    // 依面積降冪的大洲中，取第一個形心在南北緯 ±60° 內者（極區的大陸不當開場主角）。
    let h = geo.h as f32;
    let in_band = |c: &&siming_sim::Continent| {
        let y = geo.xy(c.centroid).1 as f32 + 0.5;
        y > h * 30.0 / 180.0 && y < h * 150.0 / 180.0
    };
    if let Some(c) = geo.continents.iter().find(in_band).or(geo.continents.first()) {
        let (x, y) = geo.xy(c.centroid);
        return Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
    }
    let w = geo.w as f32;
    let (mut sx, mut sy, mut sz, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for c in 0..geo.len() as u32 {
        if !geo.terrain[c as usize].is_water() {
            let (x, y) = geo.xy(c);
            let a = (x as f32 + 0.5) / w * std::f32::consts::TAU;
            sx += a.cos();
            sy += a.sin();
            sz += y as f32 + 0.5;
            n += 1.0;
        }
    }
    if n == 0.0 {
        return Vec2::new(w * 0.5, geo.h as f32 * 0.5);
    }
    let x = sy.atan2(sx).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * w;
    Vec2::new(x, sz / n)
}

/// 畫面中央的地面可見寬度（世界單位）；搖桿平移以「畫面寬／秒」計（5.9）。
fn ground_width(z: f32, map_w: f32, aspect: f32) -> f32 {
    let dist = z / camera_pitch(z, map_w).sin();
    2.0 * dist * (FOV_DEG.to_radians() * 0.5).tan() * aspect
}

/// 鏡頭只讀 `InputActions`（GDD 5.9）：平移、縮放、旋轉皆由動作層寫入。
fn camera_input(
    mut rig: ResMut<CamRig>,
    time: Res<Time>,
    act: Res<InputActions>,
    sel: Res<Selection>,
    sim: Res<SimRes>,
    mut fly: ResMut<FlyTo>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(win) = windows.single() else { return };
    // GDD 5.7「點了就到」：介面要求飛往某格（彈簧平滑即為飛行）；拉近到地圖寬 × 0.18 以內
    if let Some(cell) = fly.0.take() {
        rig.intro = None;
        let geo = &sim.0.world().geo;
        let (cx, cy) = geo.xy(cell);
        let target = Vec2::new(cx as f32 + 0.5, cy as f32 + 0.5);
        let x = if geo.wrap_x { nearest_copy_x(target.x, rig.focus_t.x, rig.map.x) } else { target.x };
        rig.focus_t = Vec2::new(x, target.y);
        let (zmin, zmax) = camera_z_range(rig.map.x);
        if rig.z_t > rig.map.x * FLY_Z_FRAC {
            rig.z_t = (rig.map.x * FLY_Z_FRAC).clamp(zmin, zmax);
        }
    }
    let win_size = Vec2::new(win.width(), win.height());
    let dt = time.delta_secs().min(0.1);
    let map_w = rig.map.x;
    let (zmin, zmax) = camera_z_range(map_w);

    // 開場運鏡：任何輸入即中止，否則依時間腳本推進目標值
    let user_active = act.pan_keys != Vec2::ZERO
        || act.pan != Vec2::ZERO
        || act.zoom != 0.0
        || act.pan_pixels != Vec2::ZERO
        || act.yaw != 0.0
        || act.bookmark_goto.is_some()
        || act.recenter_pressed;
    if user_active {
        rig.intro = None;
    }
    if let Some(intro) = rig.intro.as_mut() {
        intro.t += dt;
        let u = ((intro.t - INTRO_HOLD) / (INTRO_SECS - INTRO_HOLD)).clamp(0.0, 1.0);
        let e = u * u * u * (u * (u * 6.0 - 15.0) + 10.0); // smootherstep
        let (f, z) = (intro.from_focus.lerp(intro.to_focus, e), intro.from_z + (intro.to_z - intro.from_z) * e);
        let done = intro.t >= INTRO_SECS;
        rig.focus_t = f;
        rig.z_t = z;
        if done {
            rig.intro = None;
        }
    }

        // 以「目標」鏡頭求螢幕點下的地面點，使拖曳／縮放時該點留在原處
    let ground_under = |rig: &CamRig, c: Vec2, z: f32| {
        let tf = rig_transform(rig.focus_t, z, rig.yaw_t, map_w);
        let (o, d) = cursor_ray(c, win_size, &tf);
        ground_hit(o, d)
    };

    // 鍵盤平移（WASD／方向鍵）：高度 × 0.9／秒
    if act.pan_keys != Vec2::ZERO {
        let r = Quat::from_rotation_y(rig.yaw_t) * Vec3::new(act.pan_keys.x, 0.0, act.pan_keys.y);
        let step = rig.z_t * KEY_PAN_PER_HEIGHT * dt;
        rig.focus_t += Vec2::new(r.x, r.z).normalize_or_zero() * step;
    }
    // 搖桿平移：畫面寬比例／秒（已含死區與曲線）
    if act.pan != Vec2::ZERO {
        let r = Quat::from_rotation_y(rig.yaw_t) * Vec3::new(act.pan.x, 0.0, act.pan.y);
        let w = ground_width(rig.z_t, map_w, win_size.x / win_size.y);
        rig.focus_t += Vec2::new(r.x, r.z) * w * dt;
    }

    // 縮放（滾輪、搖桿、捏合；刻度數）：以錨點為中心
    if act.zoom != 0.0 {
        let z_new = (rig.z_t * 0.88f32.powf(act.zoom)).clamp(zmin, zmax);
        if let Some(c) = act.zoom_anchor {
            if let (Some(p0), Some(p1)) = (ground_under(&rig, c, rig.z_t), ground_under(&rig, c, z_new)) {
                rig.focus_t += p0 - p1;
            }
        }
        rig.z_t = z_new;
    }

    // 拖曳平移：以 pointer 為終點、回推 pan_pixels 為起點，兩點的地面投影相減
    if act.pan_pixels != Vec2::ZERO {
        if let Some(c) = act.pointer {
            if let (Some(p0), Some(p1)) = (ground_under(&rig, c - act.pan_pixels, rig.z_t), ground_under(&rig, c, rig.z_t)) {
                rig.focus_t += p0 - p1;
            }
        }
    }

    // 書籤（Ctrl+1–9 記下、Alt+1–9 回去）
    if let Some(i) = act.bookmark_set {
        rig.bookmarks[i as usize % 9] = Some((rig.focus_t, rig.z_t, rig.yaw_t));
    }
    if let Some(Some((f, z, y))) = act.bookmark_goto.map(|i| rig.bookmarks[i as usize % 9]) {
        rig.focus_t = f;
        rig.z_t = z.clamp(zmin, zmax);
        rig.yaw_t = y;
    }
    // R3：鏡頭回到選取的主體（人物所在格或州郡治所；不改高度）
    if act.recenter_pressed {
        let world = sim.0.world();
        let cell = match sel.target {
            Some(Target::Person(pid)) => world.persons.get(pid as usize).map(|p| p.cell),
            Some(Target::Event(eid)) => siming_sim::ledger::get(world, eid).and_then(|e| e.cell.or(e.province.and_then(|p| world.provinces.get(p as usize).map(|p| p.seat)))),
            _ => sel.selected.and_then(|id| world.provinces.get(id as usize)).map(|p| p.seat),
        };
        if let Some(cell) = cell {
            let (cx, cy) = world.geo.xy(cell);
            let x = if world.geo.wrap_x { nearest_copy_x(cx as f32 + 0.5, rig.focus_t.x, rig.map.x) } else { cx as f32 + 0.5 };
            rig.focus_t = Vec2::new(x, cy as f32 + 0.5);
        }
    }

    // 旋轉方位角（±45°，放開後不回彈）
    if act.yaw != 0.0 {
        rig.yaw_t = (rig.yaw_t + act.yaw).clamp(-YAW_LIMIT, YAW_LIMIT);
    }

    // GDD 5.2：x 東西無縫、不設界（環繞世界）；y 限制在南北緯 ±75°
    // 平面視角再依鏡頭高度／俯角限制，使畫面不超出地圖南北緣太多（極地邊疆）。
    let (row_lo, row_hi) = focus_row_limits(rig.map.x, rig.map.y, rig.z_t);
    rig.focus_t.y = rig.focus_t.y.clamp(row_lo, row_hi);
    if !rig.wrap_x {
        rig.focus_t.x = rig.focus_t.x.clamp(0.0, rig.map.x);
    }
}

fn camera_smooth(
    mut rig: ResMut<CamRig>,
    time: Res<Time>,
    mut view: ResMut<MapView>,
    mut cam: Query<&mut Transform, With<MapCamera>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (ft, zt, yt) = (rig.focus_t, rig.z_t, rig.yaw_t);
    let (mut vf, mut vz, mut vy) = (rig.vel_focus, rig.vel_z, rig.vel_yaw);
    let fx = damp(rig.focus.x, ft.x, &mut vf.x, dt);
    let fy = damp(rig.focus.y, ft.y, &mut vf.y, dt);
    let z = damp(rig.z, zt, &mut vz, dt);
    let yaw = damp(rig.yaw, yt, &mut vy, dt);
    rig.focus = Vec2::new(fx, fy);
    rig.z = z;
    rig.yaw = yaw;
    rig.vel_focus = vf;
    rig.vel_z = vz;
    rig.vel_yaw = vy;

    let map_w = rig.map.x;
    // x 以地圖寬為週期正規化（目標與現值一併平移，阻尼不受影響），避免座標無限增長。
    if rig.wrap_x && rig.intro.is_none() && !(0.0..map_w).contains(&rig.focus.x) {
        let k = (rig.focus.x / map_w).floor() * map_w;
        rig.focus.x -= k;
        rig.focus_t.x -= k;
    }
    let (zmin, zmax) = camera_z_range(map_w);
    let z = z.clamp(zmin, zmax);
    // GDD 5.2／5.3 星球視角：z 越過 0.9 倍地圖寬 → 0.6 秒內變形為球面（具遲滯，避免在門檻上閃爍）
    let target = if z > map_w * GLOBE_Z_FRAC {
        1.0
    } else if z < map_w * (GLOBE_Z_FRAC - 0.02) {
        0.0
    } else {
        rig.globe_raw.round()
    };
    let step = dt / GLOBE_MORPH_SECS;
    rig.globe_raw += (target - rig.globe_raw).clamp(-step, step);
    let g = rig.globe_raw * rig.globe_raw * (3.0 - 2.0 * rig.globe_raw);
    if (view.globe - g).abs() > 1e-5 {
        view.globe = g;
    }
    if view.focus != rig.focus {
        view.focus = rig.focus;
    }
    // GDD 5.2 古地圖化：高度縮放 1 → 0.05，著色改平塗
    let flatten = flatten_weight(z, map_w);
    let hs = 1.0 + (0.05 - 1.0) * flatten;
    if (view.height_scale - hs).abs() > 1e-5 || (view.flatten - flatten).abs() > 1e-5 || (view.camera_z - z).abs() > 1e-4 {
        view.height_scale = hs;
        view.flatten = flatten;
        view.camera_z = z;
    }
    if let Ok(mut tf) = cam.single_mut() {
        *tf = rig_transform(rig.focus, z, rig.yaw, map_w);
    }
}

/// 射線與高度場求交（步進 + 二分）。回傳世界座標。
fn ray_terrain(cpu: &TerrainCpu, hs: f32, o: Vec3, d: Vec3) -> Option<Vec3> {
    if d.y >= -1e-5 {
        return None;
    }
    let top = cpu.max_height * hs;
    let t_end = -o.y / d.y;
    let mut t = if o.y > top { (top - o.y) / d.y } else { 0.0 };
    let (mw, mh) = (cpu.w as f32, cpu.h as f32);
    // 東西環繞：x 不設界（height_at 以週期取樣）
    let inside = |p: Vec3| (cpu.wrap_x || (p.x >= 0.0 && p.x <= mw)) && p.z >= 0.0 && p.z <= mh;
    let below = |p: Vec3| p.y <= cpu.height_at(p.x, p.z) * hs;
    let mut prev = t;
    while t <= t_end {
        let p = o + d * t;
        if inside(p) && below(p) {
            let (mut lo, mut hi) = (prev, t);
            for _ in 0..12 {
                let mid = 0.5 * (lo + hi);
                if below(o + d * mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            return Some(o + d * hi);
        }
        prev = t;
        t += (0.25 + t * 0.003).min(2.0);
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn pick_province(
    view: Res<MapView>,
    cpu: Option<Res<TerrainCpu>>,
    sim: Res<SimRes>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cam: Query<&Transform, With<MapCamera>>,
    act: Res<InputActions>,
    mut sel: ResMut<Selection>,
) {
    let (Some(cpu), Ok(win), Ok(tf)) = (cpu, windows.single(), cam.single()) else { return };
    let mut hovered = None;
    if let (Some(c), false) = (act.pointer, act.pointer_over_ui) {
        let (o, d) = cursor_ray(c, Vec2::new(win.width(), win.height()), tf);
        // 星球視角：射線與球面求交；平面：與高度場求交（x 以地圖寬取模）
        let hit = if view.globe > 0.5 {
            let map = Vec2::new(cpu.w as f32, cpu.h as f32);
            let frame = globe_frame(view.focus, map, tf.rotation);
            ray_globe_uv(&frame, globe_radius(map.x), o, d).map(|uv| Vec2::new(uv.x * map.x, uv.y * map.y))
        } else {
            ray_terrain(&cpu, view.height_scale, o, d).map(|p| {
                let x = if cpu.wrap_x { p.x.rem_euclid(cpu.w as f32) } else { p.x };
                Vec2::new(x, p.z)
            })
        };
        if let Some(p) = hit {
            let (x, z) = (p.x.floor() as i64, p.y.floor() as i64);
            if x >= 0 && z >= 0 && (x as usize) < cpu.w && (z as usize) < cpu.h {
                let pid = sim.0.world().geo.province_of[z as usize * cpu.w + x as usize];
                if pid != NO_PROVINCE {
                    hovered = Some(pid);
                }
            }
        }
    }
    // 開發用：SIMING_HOVER 固定停留州郡（無指標時不覆寫）
    if (act.pointer.is_some() || std::env::var("SIMING_HOVER").is_err()) && sel.hovered != hovered {
        sel.hovered = hovered;
    }
    if act.select_pressed && !sel.ring_open {
        match hovered {
            Some(p) => {
                if sel.target != Some(Target::Province(p)) {
                    sel.open(Target::Province(p), Some(p));
                }
            }
            None => sel.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::focus_row_range;

    fn flat_cpu(w: usize, h: usize, height: f32) -> TerrainCpu {
        TerrainCpu { w, h, heights: vec![height; w * h], max_height: height.max(0.001), province_of: vec![0; w * h], wrap_x: false }
    }

    #[test]
    fn screen_centre_hits_focus() {
        let tf = rig_transform(Vec2::new(100.0, 60.0), 120.0, 0.3, 256.0);
        let (o, d) = cursor_ray(Vec2::new(800.0, 450.0), Vec2::new(1600.0, 900.0), &tf);
        let g = ground_hit(o, d).unwrap();
        assert!((g - Vec2::new(100.0, 60.0)).length() < 1e-2, "{g:?}");
    }

    #[test]
    fn ray_hits_raised_terrain_above_plane() {
        let cpu = flat_cpu(256, 160, 4.0);
        let tf = rig_transform(Vec2::new(128.0, 80.0), 100.0, 0.0, 256.0);
        let (o, d) = cursor_ray(Vec2::new(800.0, 450.0), Vec2::new(1600.0, 900.0), &tf);
        let p = ray_terrain(&cpu, 1.0, o, d).unwrap();
        assert!((p.y - 4.0).abs() < 0.05, "{p:?}");
        let p2 = ray_terrain(&cpu, 0.05, o, d).unwrap();
        assert!((p2.y - 0.2).abs() < 0.05, "{p2:?}");
    }

    #[test]
    fn wrapped_ray_picks_province_across_seam() {
        // 東西環繞：焦點在 x = 3、鏡頭俯視；游標左側 10 格的地面點 x = -7 < 0，須取模為 57
        let mut cpu = flat_cpu(64, 32, 0.0);
        cpu.wrap_x = true;
        cpu.heights = vec![1.0; 64 * 32];
        cpu.max_height = 1.0;
        let tf = rig_transform(Vec2::new(3.0, 16.0), 40.0, 0.0, 64.0);
        let (o, d) = cursor_ray(Vec2::new(800.0, 450.0), Vec2::new(1600.0, 900.0), &tf);
        let p = ray_terrain(&cpu, 1.0, o + Vec3::new(-10.0, 0.0, 0.0), d).unwrap();
        assert!(p.x < 0.0, "{p:?}");
        assert!((p.x.rem_euclid(64.0) - 57.0).abs() < 0.5, "{p:?}");
    }

    #[test]
    fn flat_view_never_shows_far_past_the_polar_edge() {
        use crate::state::{camera_pitch, focus_row_limits, FOV_DEG, FRONTIER_FRAC};
        let (w, h) = (384.0f32, 192.0f32);
        for z in [20.0, 60.0, 115.0, 200.0, 300.0] {
            let (lo, hi) = focus_row_limits(w, h, z);
            assert!(lo <= hi, "z={z}");
            let pitch = camera_pitch(z, w);
            let half = FOV_DEG.to_radians() * 0.5;
            let top_edge = lo - z * (1.0 / (pitch - half).tan() - 1.0 / pitch.tan());
            // 允許量 + 取整誤差；兩極相夾時（lo == hi）以赤道為準，不再斷言
            if (lo - hi).abs() > 1e-3 {
                assert!(top_edge >= -w * FRONTIER_FRAC - 1e-2, "z={z} top_edge={top_edge}");
            }
        }
        // 星球視角不受限
        assert_eq!(focus_row_limits(w, h, w), focus_row_range(h));
    }

    #[test]
    fn focus_rows_limited_to_75_degrees() {
        let (lo, hi) = focus_row_range(128.0);
        assert!((lo - 128.0 * 15.0 / 180.0).abs() < 1e-4 && (hi - (128.0 - lo)).abs() < 1e-4);
    }

    #[test]
    fn nearest_copy_wraps_to_closest() {
        use crate::state::nearest_copy_x;
        assert!((nearest_copy_x(250.0, 3.0, 256.0) - (-6.0)).abs() < 1e-4);
        assert!((nearest_copy_x(5.0, 250.0, 256.0) - 261.0).abs() < 1e-4);
    }

    #[test]
    fn damping_converges_without_overshoot() {
        let (mut x, mut v) = (0.0f32, 0.0f32);
        for _ in 0..600 {
            x = damp(x, 1.0, &mut v, 1.0 / 60.0);
            assert!(x <= 1.0001);
        }
        assert!((x - 1.0).abs() < 1e-3);
    }
}
