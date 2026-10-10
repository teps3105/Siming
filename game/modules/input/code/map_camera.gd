class_name MapCamera
extends Camera3D
## GDD 5.2 鏡頭：透視 30°，以焦點、離地高度 z 與方位角描述；俯角隨高度內插。
## 讀 InputMap 動作與滑鼠（拖曳平移、滾輪縮放、右鍵旋轉），並每幀把鏡頭狀態交給地圖。

signal province_hovered(province: int)
signal province_selected(province: int)

## 臨界阻尼平滑的時間常數（秒）。
const SMOOTH_SECONDS := 0.12
## 每次滾輪或每秒按鍵的縮放倍率。
const WHEEL_ZOOM := 0.88
const KEY_ZOOM_PER_SECOND := 0.35
## 每秒平移的距離（畫面可見地面寬度的比例）。
const PAN_PER_SECOND := 0.7
const YAW_LIMIT_DEG := 45.0
const YAW_PER_SECOND_DEG := 60.0
## 滑鼠停留多久後回報停留的州郡（秒，GDD 5.7）。
const HOVER_DELAY := 0.3
## 拖曳超過這個像素數就不算點擊。
const CLICK_SLOP := 6.0

var focus := Vector2.ZERO
var z := 100.0
var yaw := 0.0
## 搖桿準星選取：為真時以畫面中心取代滑鼠位置。
var use_crosshair := false

var _map: TerrainMap
var _target_focus := Vector2.ZERO
var _target_z := 100.0
var _target_yaw := 0.0
var _dragging := false
var _rotating := false
var _press_position := Vector2.ZERO
var _hover_timer := 0.0
var _hover_province := -1
var _last_pointer := Vector2.ZERO


## 綁定地圖並把鏡頭放到指定焦點與高度。
func attach(map: TerrainMap, start_focus: Vector2, start_z: float) -> void:
	_map = map
	fov = MapView.FOV_DEG
	focus = start_focus
	_target_focus = start_focus
	z = start_z
	_target_z = start_z
	near = 0.5
	far = map.map_size.x * 8.0
	_apply(0.0)


## 立即或平滑地移到指定焦點與高度（高度不指定則維持）。
func look_at_map(target_focus: Vector2, target_z := -1.0, instant := false) -> void:
	_target_focus = target_focus
	if target_z > 0.0:
		_target_z = target_z
	if instant:
		focus = _target_focus
		z = _target_z
		yaw = _target_yaw
		_apply(0.0)


func _unhandled_input(event: InputEvent) -> void:
	if _map == null:
		return
	if event is InputEventMouseButton:
		_on_mouse_button(event)
	elif event is InputEventMouseMotion:
		_on_mouse_motion(event)
	elif event is InputEventJoypadButton or event is InputEventJoypadMotion:
		use_crosshair = true
	if event.is_action_pressed(InputActions.SELECT):
		_select_at(_pointer())


func _on_mouse_button(event: InputEventMouseButton) -> void:
	use_crosshair = false
	match event.button_index:
		MOUSE_BUTTON_WHEEL_UP:
			_zoom_about(event.position, WHEEL_ZOOM)
		MOUSE_BUTTON_WHEEL_DOWN:
			_zoom_about(event.position, 1.0 / WHEEL_ZOOM)
		MOUSE_BUTTON_LEFT:
			if event.pressed:
				_dragging = true
				_press_position = event.position
			else:
				_dragging = false
				if event.position.distance_to(_press_position) <= CLICK_SLOP:
					_select_at(event.position)
		MOUSE_BUTTON_RIGHT:
			_rotating = event.pressed


func _on_mouse_motion(event: InputEventMouseMotion) -> void:
	use_crosshair = false
	_hover_timer = 0.0
	if _dragging and event.position.distance_to(_press_position) > CLICK_SLOP:
		var per_pixel := _ground_width() / get_viewport().get_visible_rect().size.x
		_target_focus -= Vector2(event.relative.x, event.relative.y / sin(MapView.pitch(z, _map.map_size.x))).rotated(-yaw) * per_pixel
	if _rotating:
		_target_yaw = clampf(_target_yaw - event.relative.x * 0.005, -deg_to_rad(YAW_LIMIT_DEG), deg_to_rad(YAW_LIMIT_DEG))


func _process(delta: float) -> void:
	if _map == null:
		return
	var pan := Input.get_vector(InputActions.PAN_LEFT, InputActions.PAN_RIGHT, InputActions.PAN_UP, InputActions.PAN_DOWN)
	if pan != Vector2.ZERO:
		_target_focus += pan.rotated(-yaw) * _ground_width() * PAN_PER_SECOND * delta
	var zoom := Input.get_axis(InputActions.ZOOM_IN, InputActions.ZOOM_OUT)
	if zoom != 0.0:
		_target_z *= pow(1.0 + KEY_ZOOM_PER_SECOND, zoom * delta * 4.0)
	var turn := Input.get_axis(InputActions.ROTATE_LEFT, InputActions.ROTATE_RIGHT)
	if turn != 0.0:
		_target_yaw = clampf(
			_target_yaw + deg_to_rad(YAW_PER_SECOND_DEG) * turn * delta, -deg_to_rad(YAW_LIMIT_DEG), deg_to_rad(YAW_LIMIT_DEG)
		)
	_apply(delta)
	_update_hover(delta)


func _apply(delta: float) -> void:
	var map_size := _map.map_size
	var limits := MapView.z_range(map_size.x)
	_target_z = clampf(_target_z, limits.x, limits.y)
	var rows := MapView.focus_row_limits(map_size, _target_z)
	_target_focus.y = clampf(_target_focus.y, rows.x, rows.y)

	var blend := 1.0 if delta <= 0.0 else 1.0 - exp(-delta / SMOOTH_SECONDS)
	focus = focus.lerp(_target_focus, blend)
	z = lerpf(z, _target_z, blend)
	yaw = lerp_angle(yaw, _target_yaw, blend)

	var tilt := MapView.pitch(z, map_size.x)
	var back := Vector3(0.0, 0.0, 1.0).rotated(Vector3.UP, yaw) * (z / tan(tilt))
	var target := Vector3(focus.x, 0.0, focus.y)
	global_position = target + back + Vector3(0.0, z, 0.0)
	look_at(target, Vector3.UP)
	_map.update_view(z, focus, global_basis, delta)


## 以指定螢幕位置為中心縮放：縮放後該位置下的地面點不動。
func _zoom_about(screen: Vector2, factor: float) -> void:
	var before := _map.pick(project_ray_origin(screen), project_ray_normal(screen))
	var limits := MapView.z_range(_map.map_size.x)
	var new_z := clampf(_target_z * factor, limits.x, limits.y)
	if before != Vector2.INF and _map.globe_morph < 0.5:
		var ratio := new_z / _target_z
		_target_focus = before + (_target_focus - before) * ratio
	_target_z = new_z


## 畫面中心橫向可見的地面寬度。
func _ground_width() -> float:
	var view := get_viewport().get_visible_rect().size
	var distance := z / sin(MapView.pitch(z, _map.map_size.x))
	return 2.0 * distance * tan(deg_to_rad(fov) * 0.5) * view.x / view.y


func _pointer() -> Vector2:
	if use_crosshair:
		return get_viewport().get_visible_rect().size * 0.5
	return get_viewport().get_mouse_position()


func _province_under(screen: Vector2) -> int:
	return _map.province_at(_map.pick(project_ray_origin(screen), project_ray_normal(screen)))


func _select_at(screen: Vector2) -> void:
	var province := _province_under(screen)
	_map.set_selected(province)
	province_selected.emit(province)


func _update_hover(delta: float) -> void:
	var pointer := _pointer()
	if use_crosshair:
		# 準星即時跟隨。
		_set_hover(_province_under(pointer))
		return
	if pointer != _last_pointer:
		_last_pointer = pointer
		_hover_timer = 0.0
		return
	_hover_timer += delta
	if _hover_timer >= HOVER_DELAY:
		_set_hover(_province_under(pointer))


func _set_hover(province: int) -> void:
	if province == _hover_province:
		return
	_hover_province = province
	_map.set_hover(province)
	province_hovered.emit(province)
