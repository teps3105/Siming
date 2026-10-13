class_name FigureActor
extends Node3D
## 地圖上的一個人物（8.11）：2D 棋子立在 3D 場景裡一塊後仰的面片上。
## 造型是封面那套 CultivatorFigure（look 取自 CultivatorFigure.traits()），姿勢是 FigureMotion 的動作資料
## （3D 骨架設計、逐格取樣成關節）。棋子畫進一個 SubViewport，貼到面片上；只在姿勢換格（24 格/秒）或畫面密度換階時重畫。
## play() 依奇觀的 age 擺姿勢，不讀引擎時間：暫停時停住，同一個 look 與 age 畫出相同的多邊形。
## 位置與姿態由 place() 依鏡頭擺：腳底在地表，往畫面上方後仰，從鏡頭看過去恆為 30° 俯角的立牌；
## 棋子永遠正對鏡頭、鏡頭轉動時不轉身；朝左（原朝向）或朝右（鏡像）由 face_right 決定。

const CF := preload("res://modules/ui_kit/code/cultivator_figure.gd")
const CARD_SHADER := preload("res://modules/figure/code/figure_card.gdshader")

## 從鏡頭看過去的固定俯角（度）。
const VIEW_PITCH_DEG := 30.0
## 畫面上的高度範圍（像素，乘介面單位）與沒夾住時的自然高度（幾格高）。
const HEIGHT_PX_MIN := 80.0
const HEIGHT_PX_MAX := 160.0
const NATURAL_CELLS := 6.0
## 保留給舊呼叫：棋子沒有朝向可轉，yaw 不再起作用。
const DEFAULT_YAW := 0.0
## 鏡頭高度超過地圖寬的這個比例就不畫（與山門圖樣同一條線，5.5）。
const MAX_CAMERA_FRACTION := 0.4
## 沿視線把棋子拉近鏡頭多少倍人高，後仰的面片才不被身後的地形蓋住（著色器只改深度，不改畫面位置）；量測用，可改。
static var depth_pull_factor := 2.0
## 畫框（身高為 1 的座標）：涵蓋雙臂過頭、甩開與風吹的大氅袖擺髮帶；實測 160 種樣子 × 三個動作的範圍是
## x −0.38 到 1.11、y −1.41 到 0.02（tests/harness/figure_extent.gd），人朝 −x、衣擺飄向身後（+x），所以畫框偏右。
const FRAME := Rect2(-0.55, -1.45, 1.8, 1.7)
## 面片貼圖的密度以幾個像素為一階（縮放時不逐像素重配貼圖）；貼圖比畫面略密，墨線約 1.6 畫面像素。
const DENSITY_STEP := 16.0
const DENSITY_MARGIN := 1.25
## 累計重畫的次數與微秒（量效能用）。
static var redraws := 0
static var redraw_usec := 0

## 朝右（左右鏡像）。預設朝左，是棋子的原朝向。
var face_right := false

var _look := {}
var _action := "stand"
var _age := 0.0
var _pose := {}
var _time := 0.0
var _height := 1.0
var _density := 0.0
var _key := ""
var _viewport: SubViewport
var _canvas: CardCanvas
var _card: MeshInstance3D
var _material: ShaderMaterial


class CardCanvas:
	extends Control

	var look := {}
	var pose := {}
	var time := 0.0
	var density := 100.0

	func _draw() -> void:
		if look.is_empty():
			return
		var start := Time.get_ticks_usec()
		CF.draw(self, Vector2(-FRAME.position.x, -FRAME.position.y) * density, density, -1.0, look, pose, time, Color(Ink.INK, 0.94))
		FigureActor.redraws += 1
		FigureActor.redraw_usec += Time.get_ticks_usec() - start


func _ready() -> void:
	_ensure_nodes()


func _ensure_nodes() -> void:
	if _viewport != null:
		return
	_viewport = SubViewport.new()
	_viewport.transparent_bg = true
	_viewport.disable_3d = true
	_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	_viewport.size = Vector2i(64, 64)
	add_child(_viewport)
	_canvas = CardCanvas.new()
	_viewport.add_child(_canvas)
	_material = ShaderMaterial.new()
	_material.shader = CARD_SHADER
	# 劫雲是半透明粒子、依距離排序，雲底會蓋住頭部；棋子排在雲之後畫，人物始終讀得清楚（地形與近處的雷仍照深度）。
	_material.render_priority = 8
	_material.set_shader_parameter("card_texture", _viewport.get_texture())
	_card = MeshInstance3D.new()
	_card.mesh = QuadMesh.new()
	_card.material_override = _material
	_card.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_card)


## 套上樣子（CultivatorFigure.traits() 的結果）。
func setup(look: Dictionary) -> void:
	_ensure_nodes()
	_look = look
	_key = ""
	play(_action, _age)


## 擺出 action 在奇觀時間 age 的姿勢（循環動作取餘，不循環的停在最後一格）。
func play(action: String, age: float) -> void:
	_action = action
	_age = age
	_time = floorf(age * FigureMotion.fps()) / FigureMotion.fps()
	_pose = FigureMotion.pose(action, _time)
	_redraw()


## 這個 look 在目前姿勢與時間的剪影多邊形（測試與診斷用）。
func polygons() -> Array[PackedVector2Array]:
	return CF.shapes(_look, _pose, _time)


## 目前姿勢的關節（CultivatorFigure 的姿勢字典，帶 anchors）。
func pose() -> Dictionary:
	return _pose


## 目前姿勢的定位點（棋子座標：身高為 1、腳下為原點、−y 向上、人朝 −x）：crown 頭頂、foot 腳底、palm_f / palm_b 兩掌心。
func anchor(anchor_name: String) -> Vector2:
	return (_pose.get("anchors", {}) as Dictionary).get(anchor_name, Vector2.ZERO)


## 動作的長度（秒）。
func action_length(action: String) -> float:
	return FigureMotion.length(action)


## 動作內的事件時間（秒），沒有是 −1。
func event_time(action: String, event: String) -> float:
	return FigureMotion.event_time(action, event)


## 整枚棋子的不透明度 0..1（淡入淡出用）。
func set_fade(alpha: float) -> void:
	_ensure_nodes()
	_material.set_shader_parameter("fade", clampf(alpha, 0.0, 1.0))


## 目前播的動作名。
func action() -> String:
	return _action


## 棋子一個身高單位目前的世界高度（place 之後有效）。
func body_height() -> float:
	return _height


## 面片目前在水平方向的縮放（負的是朝右的鏡像）。
func card_scale_x() -> float:
	return _card.scale.x if _card != null else 0.0


func _redraw() -> void:
	if _canvas == null or _look.is_empty() or _density <= 0.0:
		return
	var key := "%s|%.4f|%s" % [_action, _time, _density]
	if key == _key:
		return
	_key = key
	_canvas.look = _look
	_canvas.pose = _pose
	_canvas.time = _time
	_canvas.density = _density
	_canvas.queue_redraw()
	_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE


## 事件當事人（橋接 person(id) 的字典）換成 traits() 的 given：只給得出的欄位，其餘由種子決定。
## 橋接現在給得出性別、年紀與壽元、僧的果位；給不出貧富與門派（僧以外）。
static func given_from_person(info: Dictionary) -> Dictionary:
	var given := {}
	if info.has("gender"):
		given["sex"] = CF.Sex.FEMALE if str(info["gender"]) == "Female" else CF.Sex.MALE
	if info.has("age"):
		var years := int(info["age"])
		var lifespan := maxi(int(info.get("lifespan", 80)), 1)
		given["age"] = CF.Age.CHILD if years < 16 else (CF.Age.ELDER if years >= 60 or float(years) >= 0.7 * float(lifespan) else CF.Age.ADULT)
	if int(info.get("fruit", -1)) >= 0:
		given["school"] = CF.School.SHI
	return given


## 一個人的 look：已知的欄位（given）照用，其餘由種子決定。
static func look_of(figure_seed: int, given := {}) -> Dictionary:
	return CF.traits(figure_seed, given)


## 鏡頭高度超過地圖寬的 MAX_CAMERA_FRACTION 就不畫。
static func too_far(camera_z: float, map_width: float) -> bool:
	return camera_z > MAX_CAMERA_FRACTION * map_width


# ── 在地圖上 ──────────────────────────────────────────────────

## 棋子朝上的軸與朝向的基底（單位長，不含大小）：
## to_camera 是腳底指向鏡頭的單位向量（正交鏡頭是鏡頭後向軸；透視鏡頭是腳底到鏡頭的連線），up 是地表向上（星球視角是球面法線），
## screen_up 是畫面上方的方向。以鏡頭右軸為轉軸，把 up 在「視線與畫面上方」的平面內轉到與視線夾 (90° − 30°)，
## 從鏡頭看恆為 30° 俯角的立牌；up 離開這個平面的分量（星球曲率）原樣保留。yaw 是繞自身上軸的額外轉動，棋子不用（0）。
static func stand_basis(to_camera: Vector3, up: Vector3, screen_up: Vector3, yaw := DEFAULT_YAW) -> Basis:
	var v := to_camera.normalized()
	var su := (screen_up - v * screen_up.dot(v)).normalized()
	var right := su.cross(v).normalized()
	var ground := up.normalized()
	var now := atan2(ground.dot(su), ground.dot(v))
	var delta := deg_to_rad(90.0 - VIEW_PITCH_DEG) - now
	var axis := (Quaternion(-right, delta) * ground).normalized()
	var toward := v - axis * v.dot(axis)
	if toward.length() < 1e-3:
		toward = su - axis * su.dot(axis)
	toward = toward.normalized()
	var upright := Basis(axis.cross(toward).normalized(), axis, toward)
	return Basis(axis, yaw) * upright


## 棋子一個身高單位的世界高度：畫面上 80–160 px（乘介面單位），自然高度為 NATURAL_CELLS 格；30° 俯角下鉛直長度在畫面上乘 cos 30°。
static func world_height(px_per_unit: float, unit: float) -> float:
	var natural := px_per_unit * NATURAL_CELLS * cos(deg_to_rad(VIEW_PITCH_DEG))
	var px := clampf(natural, HEIGHT_PX_MIN * unit, HEIGHT_PX_MAX * unit)
	return px / (px_per_unit * cos(deg_to_rad(VIEW_PITCH_DEG)))


## 貼圖的密度（每個身高單位幾個貼圖像素）：畫面上一個身高單位是 screen_px 像素，貼圖取略密的一階，墨線落在約 1.6 畫面像素。
static func texture_scale(screen_px: float) -> float:
	return ceilf(screen_px * DENSITY_MARGIN / DENSITY_STEP) * DENSITY_STEP


## 鏡頭下每個世界單位幾個畫面像素（垂直於視線的方向）。
static func pixels_per_unit(camera: Camera3D, at: Vector3) -> float:
	var height := camera.get_viewport().get_visible_rect().size.y
	if camera.projection == Camera3D.PROJECTION_ORTHOGONAL:
		return height / camera.size
	var distance := camera.global_position.distance_to(at)
	return height / (2.0 * distance * tan(deg_to_rad(camera.fov) * 0.5))


## 依鏡頭把棋子擺在 origin（地表上的腳底）。hidden 為真（鏡頭拉得太遠）就不畫。
func place(origin: Vector3, up: Vector3, camera: Camera3D, unit: float, hidden := false, _yaw := DEFAULT_YAW) -> void:
	_ensure_nodes()
	visible = not hidden
	if hidden or camera == null:
		return
	var to_camera := camera.global_basis.z
	if camera.projection != Camera3D.PROJECTION_ORTHOGONAL:
		to_camera = camera.global_position - origin
	var ppu := pixels_per_unit(camera, origin)
	_height = world_height(ppu, unit)
	var basis := stand_basis(to_camera, up, camera.global_basis.y, 0.0)
	global_transform = Transform3D(basis, origin)
	# 面片：高度方向被後仰壓扁 cos 30°，所以高度用 _height（畫面上剛好是 80–160 px）、寬度用 _height × cos 30°，棋子在畫面上不變形。
	var width := _height * cos(deg_to_rad(VIEW_PITCH_DEG))
	var mirror := -1.0 if face_right else 1.0
	_card.scale = Vector3(FRAME.size.x * width * mirror, FRAME.size.y * _height, 1.0)
	_card.position = Vector3(FRAME.get_center().x * width * mirror, -FRAME.get_center().y * _height, 0.0)
	_material.set_shader_parameter("depth_pull", _height * depth_pull_factor)
	var screen_px := _height * ppu * cos(deg_to_rad(VIEW_PITCH_DEG))
	var density := texture_scale(screen_px)
	if not is_equal_approx(density, _density):
		_density = density
		_viewport.size = Vector2i(ceili(FRAME.size.x * density), ceili(FRAME.size.y * density))
		_canvas.size = Vector2(_viewport.size)
		_key = ""
	_redraw()
