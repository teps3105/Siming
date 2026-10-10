//! GDD 5.2／5.3 星球視角：球面幾何（著色器的頂點內插與滑鼠選取共用）。
//!
//! 球面以地圖 UV 為經緯：`u → 經度 = 2π·u`（向東增加）、`v → 緯度 = π/2 − π·v`。
//! 球心在鏡頭注視點，球的旋轉使「焦點所在的經緯」正對鏡頭、北方朝向畫面上方；
//! 因此平移（改變焦點）即旋轉星球，往內推回時平面地圖正好在同一焦點。
use bevy::prelude::*;

/// 球半徑：圓周 = 地圖寬（與等距圓柱的赤道一致）。
pub fn globe_radius(map_w: f32) -> f32 {
    map_w / std::f32::consts::TAU
}

/// 單位球面上 (經度, 緯度) 的方向（球面局部座標，+y 為北）。
pub fn sphere_dir(lon: f32, lat: f32) -> Vec3 {
    Vec3::new(lat.cos() * lon.cos(), lat.sin(), -lat.cos() * lon.sin())
}

/// 焦點（世界 xz，x 為欄座標、y 為列座標）對應的 (經度, 緯度)。
pub fn focus_lonlat(focus: Vec2, map: Vec2) -> (f32, f32) {
    (focus.x / map.x * std::f32::consts::TAU, std::f32::consts::FRAC_PI_2 - focus.y / map.y * std::f32::consts::PI)
}

/// 球面局部座標 → 世界座標的變換（旋轉 + 球心平移）。`cam` 為鏡頭的世界旋轉。
pub fn globe_frame(focus: Vec2, map: Vec2, cam: Quat) -> Mat4 {
    let (lon, lat) = focus_lonlat(focus, map);
    let e_r = sphere_dir(lon, lat);
    let e_e = Vec3::new(-lon.sin(), 0.0, -lon.cos());
    let e_n = Vec3::new(-lat.sin() * lon.cos(), lat.cos(), lat.sin() * lon.sin());
    let sphere_basis = Mat3::from_cols(e_e, e_n, e_r);
    let cam_basis = Mat3::from_cols(cam * Vec3::X, cam * Vec3::Y, cam * Vec3::Z);
    let rot = cam_basis * sphere_basis.transpose();
    Mat4::from_cols(rot.x_axis.extend(0.0), rot.y_axis.extend(0.0), rot.z_axis.extend(0.0), Vec4::new(focus.x, 0.0, focus.y, 1.0))
}

/// 射線與星球求交 → 地圖 UV（x 0..1 向東、y 0..1 向南）。
pub fn ray_globe_uv(frame: &Mat4, radius: f32, o: Vec3, d: Vec3) -> Option<Vec2> {
    let c = frame.w_axis.truncate();
    let oc = o - c;
    let b = oc.dot(d);
    let disc = b * b - (oc.dot(oc) - radius * radius);
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t < 0.0 {
        return None;
    }
    let p = o + d * t - c;
    let local = frame.inverse().transform_vector3(p).normalize();
    let lat = local.y.clamp(-1.0, 1.0).asin();
    let lon = (-local.z).atan2(local.x);
    Some(Vec2::new((lon / std::f32::consts::TAU).rem_euclid(1.0), (0.5 - lat / std::f32::consts::PI).clamp(0.0, 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_point_faces_camera_and_picks_back() {
        let map = Vec2::new(512.0, 256.0);
        let focus = Vec2::new(188.0, 90.0);
        // 俯視鏡頭：看向 -y，畫面上方 = -z（北）、右方 = +x（東）
        let cam = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        let frame = globe_frame(focus, map, cam);
        let r = globe_radius(map.x);
        let o = Vec3::new(focus.x, 300.0, focus.y);
        let uv = ray_globe_uv(&frame, r, o, Vec3::NEG_Y).unwrap();
        assert!((uv.x - focus.x / map.x).abs() < 1e-3, "{uv:?}");
        assert!((uv.y - focus.y / map.y).abs() < 1e-3, "{uv:?}");
        let uv2 = ray_globe_uv(&frame, r, o + Vec3::new(0.0, 0.0, -r * 0.5), Vec3::NEG_Y).unwrap();
        assert!(uv2.y < uv.y, "north up {uv2:?}");
        let uv3 = ray_globe_uv(&frame, r, o + Vec3::new(r * 0.3, 0.0, 0.0), Vec3::NEG_Y).unwrap();
        assert!(uv3.x > uv.x, "east right {uv3:?}");
    }
}
