class_name FigureActor
extends Node3D
## 地圖上的一個 3D 小人（8.11）：一副骨架、一套依樣子顯示或隱藏的部件、一組動作。
## 樣子取自 CultivatorFigure.traits() 的結果（look），動作取樣自 figure.glb 的動畫資料，由 play() 依奇觀的 age 擺出，
## 不用 AnimationPlayer 自己走時間、不讀引擎時間：暫停時停住，同一個 look 與 age 擺出的骨頭變換相同。
## 位置與姿態由 place() 依鏡頭擺：腳底在地表，往畫面上方後仰，從鏡頭看過去恆為 30° 俯角的立姿。

const FIGURE := preload("res://modules/figure/assets/figure.glb")
const INK_SHADER := preload("res://modules/figure/code/figure_ink.gdshader")
const CF := preload("res://modules/ui_kit/code/cultivator_figure.gd")

const INK := Color("2B2620")
## 墨線寬（螢幕像素，乘介面單位）。
const OUTLINE_PX := 1.5
## 從鏡頭看過去的固定俯角（度）。
const VIEW_PITCH_DEG := 30.0
## 畫面上的高度範圍（像素，乘介面單位）與沒夾住時的自然高度（幾格高）。
const HEIGHT_PX_MIN := 80.0
const HEIGHT_PX_MAX := 160.0
const NATURAL_CELLS := 6.0
## 預設朝向：繞自身上軸，轉向畫面左前。
const DEFAULT_YAW := -0.7
## 鏡頭高度超過地圖寬的這個比例就不畫（與山門圖樣同一條線，5.5）。
const MAX_CAMERA_FRACTION := 0.4
## 沿視線把人物拉近鏡頭多少倍人高，後仰的人物才不被身後的地形蓋住（著色器只改深度，不改畫面位置）；量測用，可改。
static var depth_pull_factor := 2.0
## 基準身形：骨架建模時的幾頭身與半肩寬、半腰寬。
const BASE_HEADS := 7.0
const BASE_SHOULDER := 0.105
const BASE_GIRTH := 0.09
## 動作是否循環（8.11 的動作表）。
const LOOPS := {"stand": true, "walk": true, "sit": true, "brace": true, "rise": true}
const HAIR_NAMES := ["topknot", "crown", "loose", "hat", "twin_buns", "high_bun", "shaven", "scholar_cap"]
const WHISKER_NAMES := ["", "moustache", "goatee", "three", "five", "full"]
const PROP_NAMES := ["", "sword_back", "whisk", "gourd", "staff", "fan", "flute", "lotus", "basket", "tablet", "beads", "scroll"]
## 檔案裡還沒有的部件以這幾個代用（鬚、持物、大氅沒有代用，不顯示）。
const SUBSTITUTES := {"robe": "robe.male_regular", "hair": "hair.topknot", "face": "face.adult"}

## 這個人現在的 look 想顯示、但檔案裡還沒有的部件。
var missing: Array[String] = []
## 實際顯示的部件（檔案裡的名稱）。
var shown: Array[String] = []

var _look := {}
var _model: Node
var _skeleton: Skeleton3D
var _animations: AnimationPlayer
var _parts := {}
var _inks: Array[ShaderMaterial] = []
var _fills: Array[ShaderMaterial] = []
var _action := "stand"
var _age := 0.0
var _height := 1.0


func _ready() -> void:
	_load_model()


## 套上樣子：顯示該顯示的部件、設各部件的顏色。身形（骨頭縮放）在每次 play() 疊上。
func setup(look: Dictionary) -> void:
	_look = look
	_load_model()
	missing.clear()
	shown.clear()
	var wanted := parts_for(look)
	for node: MeshInstance3D in _parts.values():
		node.visible = false
	for name in wanted:
		var key := _key(name)
		if not _parts.has(key):
			missing.append(name)
			var substitute: String = SUBSTITUTES.get(name.get_slice(".", 0), "")
			if substitute == "":
				continue
			key = _key(substitute)
			name = substitute
			if not _parts.has(key) or shown.has(name):
				continue
		(_parts[key] as MeshInstance3D).visible = true
		shown.append(name)
	_paint(look)
	play(_action, _age)


## look 想顯示的部件名稱（規格的完整部件表，不論檔案裡有沒有）。
static func parts_for(look: Dictionary) -> Array[String]:
	var out: Array[String] = ["body.base"]
	var female: bool = int(look.get("sex", CF.Sex.MALE)) == CF.Sex.FEMALE and int(look.get("school", 0)) != CF.School.SHI
	var sleeve := float(look.get("sleeve", 0.12))
	var width := "narrow" if sleeve < 0.07 else ("regular" if sleeve < 0.17 else "wide")
	out.append("robe.%s_%s" % ["female" if female else "male", width])
	if bool(look.get("cape", false)):
		out.append("cape.cloak")
	var hair := int(look.get("hair", 0))
	if hair != CF.Hair.SHAVEN:
		out.append("hair." + HAIR_NAMES[clampi(hair, 0, HAIR_NAMES.size() - 1)])
	var whiskers := int(look.get("whiskers", 0))
	if whiskers > 0:
		out.append("beard." + WHISKER_NAMES[clampi(whiskers, 0, WHISKER_NAMES.size() - 1)])
	var prop := int(look.get("prop", 0))
	if prop > 0:
		out.append("prop." + PROP_NAMES[clampi(prop, 0, PROP_NAMES.size() - 1)])
	var age := int(look.get("age", CF.Age.ADULT))
	out.append("face." + ("young" if age == CF.Age.CHILD else ("old" if age == CF.Age.ELDER else "adult")))
	return out


## 鏡頭高度超過地圖寬的 MAX_CAMERA_FRACTION 就不畫。
static func too_far(camera_z: float, map_width: float) -> bool:
	return camera_z > MAX_CAMERA_FRACTION * map_width


## 一個人的 look：已知的欄位（given）照用，其餘由種子決定。
static func look_of(figure_seed: int, given := {}) -> Dictionary:
	return CF.traits(figure_seed, given)


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


## 從 look 的鍵名取得穩定的部件鍵（匯入時節點名稱的「.」會換成「_」）。
static func _key(part: String) -> String:
	return part.replace(".", "_")


func _load_model() -> void:
	if _model != null:
		return
	_model = FIGURE.instantiate()
	add_child(_model)
	_skeleton = _model.find_children("*", "Skeleton3D", true, false)[0]
	var players := _model.find_children("*", "AnimationPlayer", true, false)
	_animations = players[0] if not players.is_empty() else null
	for node in _skeleton.find_children("*", "MeshInstance3D", true, false):
		_parts[str(node.name)] = node
		node.visible = false
		node.set_meta("part", str(node.name))


## 檔案裡的部件鍵（測試與診斷用）。
func part_names() -> PackedStringArray:
	return PackedStringArray(_parts.keys())


func skeleton() -> Skeleton3D:
	return _skeleton


func animation_names() -> PackedStringArray:
	return _animations.get_animation_list() if _animations != null else PackedStringArray()


## 動作的長度（秒）。
func action_length(action: String) -> float:
	var anim := _animations.get_animation(action) if _animations != null and _animations.has_animation(action) else null
	return anim.length if anim != null else 0.0


func _paint(look: Dictionary) -> void:
	_inks.clear()
	_fills.clear()
	var palette: Dictionary = look.get("colors", {})
	for key: String in _parts:
		var node := _parts[key] as MeshInstance3D
		var material := ShaderMaterial.new()
		material.shader = INK_SHADER
		var kind := key.get_slice("_", 0)
		material.set_shader_parameter("ink_color", INK)
		match kind:
			"body":
				material.set_shader_parameter("base_color", palette.get(CF.SKIN, Color.WHITE))
				material.set_shader_parameter("accent_color", palette.get(CF.SHOE, INK))
				material.set_shader_parameter("shade_threshold", -0.3)
			"robe":
				material.set_shader_parameter("base_color", palette.get(CF.ROBE, Color.WHITE))
				material.set_shader_parameter("accent_color", palette.get(CF.TRIM, Color.WHITE))
				material.set_shader_parameter("sash_color", palette.get(CF.SASH, Color.WHITE))
				material.set_shader_parameter("shade_threshold", 0.05)
			"hair", "beard":
				material.set_shader_parameter("base_color", palette.get(CF.HAIR if kind == "hair" else CF.BEARD, INK))
				material.set_shader_parameter("accent_color", palette.get(CF.HAIR, INK))
				material.set_shader_parameter("shade_threshold", 0.0)
			"face":
				material.set_shader_parameter("flat_ink", true)
		var shell: ShaderMaterial = null
		if kind != "face":
			shell = ShaderMaterial.new()
			shell.shader = INK_SHADER
			shell.set_shader_parameter("ink_color", INK)
			shell.set_shader_parameter("outline_px", OUTLINE_PX)
			material.next_pass = shell
			_inks.append(shell)
		_fills.append(material)
		node.material_override = material


## 擺出 action 在奇觀時間 age 的姿勢（循環動作取餘，不循環的停在最後一格），再疊上 look 的身形。
func play(action: String, age: float) -> void:
	_action = action
	_age = age
	if _skeleton == null:
		return
	var anim: Animation = null
	if _animations != null and _animations.has_animation(action):
		anim = _animations.get_animation(action)
	for i in _skeleton.get_bone_count():
		_skeleton.set_bone_pose(i, _skeleton.get_bone_rest(i))
	if anim != null:
		var t := fposmod(age, anim.length) if LOOPS.get(action, false) else clampf(age, 0.0, anim.length)
		for track in anim.get_track_count():
			var path := anim.track_get_path(track)
			if path.get_subname_count() == 0:
				continue
			var bone := _skeleton.find_bone(path.get_subname(0))
			if bone < 0:
				continue
			match anim.track_get_type(track):
				Animation.TYPE_POSITION_3D:
					_skeleton.set_bone_pose_position(bone, anim.position_track_interpolate(track, t))
				Animation.TYPE_ROTATION_3D:
					_skeleton.set_bone_pose_rotation(bone, anim.rotation_track_interpolate(track, t))
				Animation.TYPE_SCALE_3D:
					_skeleton.set_bone_pose_scale(bone, anim.scale_track_interpolate(track, t))
	_apply_shape()
	for material in _fills:
		material.set_shader_parameter("age", age)
	for material in _inks:
		material.set_shader_parameter("age", age)


## 身形：heads 縮放頭骨、stature 縮放根、shoulder 移動肩、girth 縮放腰的橫向與前後、stoop 轉胸與頸。
func _apply_shape() -> void:
	if _look.is_empty():
		return
	var heads := float(_look.get("heads", BASE_HEADS))
	var girth := float(_look.get("girth", BASE_GIRTH)) / BASE_GIRTH
	var shoulder := float(_look.get("shoulder", BASE_SHOULDER))
	var stoop := float(_look.get("stoop", 0.0))
	_scale_bone("root", Vector3.ONE * float(_look.get("stature", 1.0)))
	_scale_bone("head", Vector3.ONE * (BASE_HEADS / heads))
	_scale_bone("waist", Vector3(girth, 1.0, girth))
	for tag in ["_l", "_r"]:
		var bone := _skeleton.find_bone("shoulder" + tag)
		if bone >= 0:
			var side := 1.0 if tag == "_l" else -1.0
			var pose := _skeleton.get_bone_pose_position(bone)
			pose.x += side * (shoulder / girth - BASE_SHOULDER)
			_skeleton.set_bone_pose_position(bone, pose)
	_turn_bone("chest", Vector3.RIGHT, deg_to_rad(14.0) * stoop)
	_turn_bone("neck", Vector3.RIGHT, deg_to_rad(10.0) * stoop)


func _scale_bone(bone_name: String, scale: Vector3) -> void:
	var bone := _skeleton.find_bone(bone_name)
	if bone >= 0:
		_skeleton.set_bone_pose_scale(bone, _skeleton.get_bone_pose_scale(bone) * scale)


func _turn_bone(bone_name: String, axis: Vector3, angle: float) -> void:
	var bone := _skeleton.find_bone(bone_name)
	if bone >= 0:
		_skeleton.set_bone_pose_rotation(bone, Quaternion(axis, angle) * _skeleton.get_bone_pose_rotation(bone))


## 全部骨頭的姿勢（相對父骨頭）串成的字串，測試比對「同一個 look 與 age 兩次擺出相同」用。
func pose_signature() -> String:
	var parts := PackedStringArray()
	for i in _skeleton.get_bone_count():
		var p := _skeleton.get_bone_pose(i)
		parts.append("%s:%s:%s" % [p.origin, p.basis.get_rotation_quaternion(), p.basis.get_scale()])
	return "|".join(parts)


# ── 在地圖上 ──────────────────────────────────────────────────

## 人物朝上的軸與朝向的基底（單位長，不含大小）：
## to_camera 是腳底指向鏡頭的單位向量（正交鏡頭是鏡頭後向軸；透視鏡頭是腳底到鏡頭的連線），up 是地表向上（星球視角是球面法線），
## screen_up 是畫面上方的方向。以鏡頭右軸為轉軸，把 up 在「視線與畫面上方」的平面內轉到與視線夾 (90° − 30°)，
## 從鏡頭看恆為 30° 俯角的立姿；up 離開這個平面的分量（星球曲率）原樣保留。朝向（繞自身上軸）轉完後再套上，鏡頭正俯視時仍有定義。
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


## 人物的高度（世界單位）：畫面上 80–160 px（乘介面單位），自然高度為 NATURAL_CELLS 格；30° 俯角下鉛直長度在畫面上乘 cos 30°。
static func world_height(px_per_unit: float, unit: float) -> float:
	var natural := px_per_unit * NATURAL_CELLS * cos(deg_to_rad(VIEW_PITCH_DEG))
	var px := clampf(natural, HEIGHT_PX_MIN * unit, HEIGHT_PX_MAX * unit)
	return px / (px_per_unit * cos(deg_to_rad(VIEW_PITCH_DEG)))


## 鏡頭下每個世界單位幾個畫面像素（垂直於視線的方向）。
static func pixels_per_unit(camera: Camera3D, at: Vector3) -> float:
	var height := camera.get_viewport().get_visible_rect().size.y
	if camera.projection == Camera3D.PROJECTION_ORTHOGONAL:
		return height / camera.size
	var distance := camera.global_position.distance_to(at)
	return height / (2.0 * distance * tan(deg_to_rad(camera.fov) * 0.5))


## 依鏡頭把人物擺在 origin（地表上的腳底）。hidden 為真（鏡頭拉得太遠）就不畫。
func place(origin: Vector3, up: Vector3, camera: Camera3D, unit: float, hidden := false, yaw := DEFAULT_YAW) -> void:
	visible = not hidden
	if hidden or camera == null:
		return
	var to_camera := camera.global_basis.z
	if camera.projection != Camera3D.PROJECTION_ORTHOGONAL:
		to_camera = camera.global_position - origin
	var ppu := pixels_per_unit(camera, origin)
	_height = world_height(ppu, unit)
	var basis := stand_basis(to_camera, up, camera.global_basis.y, yaw)
	global_transform = Transform3D(basis.scaled(Vector3.ONE * _height), origin)
	for material in _fills:
		material.set_shader_parameter("depth_pull", _height * depth_pull_factor)
	for material in _inks:
		material.set_shader_parameter("depth_pull", _height * depth_pull_factor)
		material.set_shader_parameter("outline_px", OUTLINE_PX * unit)
