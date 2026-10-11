class_name MapView
extends RefCounted
## GDD 5.2、5.3 鏡頭高度決定的畫面參數與星球幾何。純函式，地圖、鏡頭與選取共用。

const FOV_DEG := 30.0
const Z_MIN_FRAC := 0.04
const Z_MAX_FRAC := 1.1
const PITCH_MIN_DEG := 68.0
const PITCH_MAX_DEG := 88.0
const FLATTEN_START_FRAC := 0.6
## 星球視角下，江在這個鏡頭高度（地圖寬的比例）以上淡出。
const RIVER_GLOBE_Z_FRAC := 0.5
const FOCUS_MAX_LAT_DEG := 75.0
## 星球視角的地形高度倍率：湊近時的值、拉遠後的值，與兩者之間的鏡頭高度（地圖寬的比例）。
const GLOBE_RELIEF_NEAR := 0.3
const GLOBE_RELIEF_FAR := 1.0
const GLOBE_RELIEF_NEAR_FRAC := 0.08
const GLOBE_RELIEF_FAR_FRAC := 0.6
## 星球視角的高度壓縮係數：低地照原樣，越高壓得越多（著色器同值）。
const GLOBE_COMPRESS := 2.0
const FRONTIER_FRAC := 0.05
## GDD 5.3「河流」：大河、小河出現的鏡頭高度（地圖寬的比例）與淡入區間。
const RIVER_BIG_Z_FRAC := 0.5
const RIVER_SMALL_Z_FRAC := 0.2
const RIVER_FADE := 0.1


static func z_range(map_w: float) -> Vector2:
	return Vector2(map_w * Z_MIN_FRAC, map_w * Z_MAX_FRAC)


## 俯角（弧度）：z_min 時 68°，z_max 時 88°。
static func pitch(z: float, map_w: float) -> float:
	var r := z_range(map_w)
	var t := clampf((z - r.x) / (r.y - r.x), 0.0, 1.0)
	return deg_to_rad(lerpf(PITCH_MIN_DEG, PITCH_MAX_DEG, t))


## 古地圖化權重：z > 地圖寬 × 0.6 起由 0 漸變到 1。
static func flatten_weight(z: float, map_w: float) -> float:
	return smoothstep(map_w * FLATTEN_START_FRAC, map_w * Z_MAX_FRAC, z)


## 地形高度縮放：由 1 漸變到 0.05。
static func height_scale(z: float, map_w: float) -> float:
	return lerpf(1.0, 0.05, flatten_weight(z, map_w))


## 星球視角的地形高度倍率：比例尺越小（拉遠）誇張越多，湊近（z ≤ 地圖寬 × 0.08）時最少。
static func globe_relief(z: float, map_w: float) -> float:
	return lerpf(GLOBE_RELIEF_NEAR, GLOBE_RELIEF_FAR, smoothstep(map_w * GLOBE_RELIEF_NEAR_FRAC, map_w * GLOBE_RELIEF_FAR_FRAC, z))


## 星球視角的高度壓縮：h 為地表高度、top 為全圖最高的地表高度。
## 坡度在海平面為 1、隨高度遞減，最高峰壓到 top × (1 − e^−K) / K。
static func globe_compress(h: float, top: float) -> float:
	if top <= 0.0:
		return h
	return top * (1.0 - exp(-GLOBE_COMPRESS * h / top)) / GLOBE_COMPRESS


## 河流各等級的不透明度（x 小河、y 大河、z 江）：門檻上下各 10% 線性淡入淡出；江在星球視角拉遠後淡出。
static func river_alpha(z: float, map_w: float, globe_morph: float) -> Vector3:
	return Vector3(
		_fade_below(z, map_w * RIVER_SMALL_Z_FRAC),
		_fade_below(z, map_w * RIVER_BIG_Z_FRAC),
		lerpf(1.0, _fade_below(z, map_w * RIVER_GLOBE_Z_FRAC), globe_morph),
	)


static func _fade_below(z: float, threshold: float) -> float:
	var lo := threshold * (1.0 - RIVER_FADE)
	var hi := threshold * (1.0 + RIVER_FADE)
	return 1.0 - clampf((z - lo) / (hi - lo), 0.0, 1.0)


## 鏡頭焦點的列座標範圍：南北緯 75° 之內；平面視角另要求畫面可見的地面不超出南北緣太多。
static func focus_row_limits(map_size: Vector2, z: float, globe := false) -> Vector2:
	var t := (90.0 - FOCUS_MAX_LAT_DEG) / 180.0
	var lo := map_size.y * t
	var hi := map_size.y * (1.0 - t)
	if globe:
		return Vector2(lo, hi)
	var p := pitch(z, map_size.x)
	var half := deg_to_rad(FOV_DEG) * 0.5
	var north := z * (1.0 / tan(p - half) - 1.0 / tan(p))
	var south := z * (1.0 / tan(p) - 1.0 / tan(p + half))
	var allow := map_size.x * FRONTIER_FRAC
	var lo2 := maxf(lo, north - allow)
	var hi2 := minf(hi, map_size.y - (south - allow))
	if lo2 > hi2:
		return Vector2(map_size.y * 0.5, map_size.y * 0.5)
	return Vector2(lo2, hi2)


## 把 x 搬到離 near_x 最近的東西副本。
static func nearest_copy_x(x: float, near_x: float, map_w: float) -> float:
	return x + map_w * roundf((near_x - x) / map_w)


static func globe_radius(map_w: float) -> float:
	return map_w / TAU


## 鏡頭到焦點的距離：平面視角是到地面的距離；星球視角焦點在球心，再加一個半徑，
## 因此同一個鏡頭高度在兩種視角下看到的地面大小相同。morph 為 0 平面 .. 1 球面。
static func camera_distance(z: float, map_w: float, morph: float) -> float:
	return z / sin(pitch(z, map_w)) + globe_radius(map_w) * morph


## 單位球面上（經度、緯度）的方向，+y 為北。
static func sphere_dir(lon: float, lat: float) -> Vector3:
	return Vector3(cos(lat) * cos(lon), sin(lat), -cos(lat) * sin(lon))


## 球面局部座標 → 世界座標：球心在焦點，焦點所在的經緯正對鏡頭、北方朝畫面上方。
static func globe_frame(focus: Vector2, map_size: Vector2, camera_basis: Basis) -> Transform3D:
	var lon := focus.x / map_size.x * TAU
	var lat := PI * 0.5 - focus.y / map_size.y * PI
	var e_r := sphere_dir(lon, lat)
	var e_e := Vector3(-sin(lon), 0.0, -cos(lon))
	var e_n := Vector3(-sin(lat) * cos(lon), cos(lat), sin(lat) * sin(lon))
	var sphere_basis := Basis(e_e, e_n, e_r)
	return Transform3D(camera_basis * sphere_basis.transposed(), Vector3(focus.x, 0.0, focus.y))


## 射線與星球求交 → 地圖座標（x、z，以格為單位）；未命中回傳 Vector2.INF。
static func ray_globe(frame: Transform3D, radius: float, map_size: Vector2, origin: Vector3, dir: Vector3) -> Vector2:
	var oc := origin - frame.origin
	var b := oc.dot(dir)
	var disc := b * b - (oc.dot(oc) - radius * radius)
	if disc < 0.0:
		return Vector2.INF
	var t := -b - sqrt(disc)
	if t < 0.0:
		return Vector2.INF
	var local := (frame.basis.inverse() * (origin + dir * t - frame.origin)).normalized()
	var lat := asin(clampf(local.y, -1.0, 1.0))
	var lon := atan2(-local.z, local.x)
	return Vector2(fposmod(lon / TAU, 1.0) * map_size.x, clampf(0.5 - lat / PI, 0.0, 1.0) * map_size.y)
