class_name MapCamera
extends Camera3D
## GDD 5.2 鏡頭：透視 30°，以焦點、離地高度 z 與方位角描述；俯角隨高度內插。
## 讀 InputMap 動作、滑鼠（拖曳平移、滾輪縮放、右鍵旋轉）、觸控與搖桿（GDD 5.9），並每幀把鏡頭狀態交給地圖。
## 搖桿當滑鼠用：右搖桿移動游標，A、LT 是左鍵，RT 是右鍵；這裡把它們轉成滑鼠事件送回輸入分派，
## 地圖與介面因此都以同一條滑鼠路徑回應。
## 單指的點與拖由引擎轉成滑鼠事件（專案設定 emulate_mouse_from_touch），這裡只另外處理雙指與長按。

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
## 開場運鏡的秒數（GDD 5.2）。
const OPENING_SECONDS := 3.5
## 手指按住不動超過這個秒數算長按：提示照常出現，放開不選取（GDD 5.9）。
const LONG_PRESS_SECONDS := 0.5
## 搖桿游標推滿時每秒移動的距離（以 1080 高的畫面計，其他解析度等比）；實際速度再乘輸入量。
const CURSOR_SPEED := 1100.0
## 由搖桿轉出的滑鼠事件以這個裝置編號標記，收到時不把輸入裝置切回鍵鼠。
const PAD_DEVICE := 77

var focus := Vector2.ZERO
var z := 100.0
var yaw := 0.0
## 最後一次輸入來自搖桿：為真時以搖桿游標取代滑鼠位置。
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
## 搖桿游標在畫面上的位置；(-1, -1) 表示還沒用過，第一次用時放到畫面中央。
var _cursor := Vector2(-1.0, -1.0)
var _press_msec := 0
## 螢幕上的手指：index → 位置。
var _touches := {}
## 這次按下期間出現過第二指；到手指全部放開為止，放開都不算點選。
var _two_finger_gesture := false
var _flight_from_z := 0.0
var _flight_to_z := 0.0
var _flight_seconds := 0.0
var _flight_progress := 0.0


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


## 開場運鏡（GDD 5.2）：在 seconds 秒內把高度降到 target_z，焦點不動；期間按下任何鍵或螢幕就直接到位。
func descend_to(target_z: float, seconds: float) -> void:
	_flight_from_z = _target_z
	_flight_to_z = target_z
	_flight_seconds = seconds
	_flight_progress = 0.0


## 開場運鏡是否進行中。
func is_flying() -> bool:
	return _flight_seconds > 0.0


func _unhandled_input(event: InputEvent) -> void:
	if _map == null:
		return
	if is_flying() and _is_press(event):
		_flight_seconds = 0.0
		_target_z = _flight_to_z
	if event is InputEventScreenTouch:
		_on_touch(event)
	elif event is InputEventScreenDrag:
		_on_touch_drag(event)
	elif event is InputEventMouseButton:
		_on_mouse_button(event)
	elif event is InputEventMouseMotion:
		_on_mouse_motion(event)
	elif event is InputEventJoypadButton or event is InputEventJoypadMotion:
		use_crosshair = true


static func _is_press(event: InputEvent) -> bool:
	var is_button := (
		event is InputEventKey or event is InputEventMouseButton or event is InputEventJoypadButton or event is InputEventScreenTouch
	)
	return is_button and event.is_pressed() and not event.is_echo()


func _on_touch(event: InputEventScreenTouch) -> void:
	if event.pressed:
		_touches[event.index] = event.position
		if _touches.size() >= 2:
			_two_finger_gesture = true
			_dragging = false
	else:
		_touches.erase(event.index)
		if _touches.is_empty():
			_two_finger_gesture = false


## 雙指：兩指距離的變化是縮放（以兩指中點為中心），連線方向的變化是旋轉。
func _on_touch_drag(event: InputEventScreenDrag) -> void:
	if not _touches.has(event.index):
		return
	if _touches.size() != 2:
		_touches[event.index] = event.position
		return
	var ids := _touches.keys()
	var before: Vector2 = _touches[ids[1]] - _touches[ids[0]]
	_touches[event.index] = event.position
	var a: Vector2 = _touches[ids[0]]
	var b: Vector2 = _touches[ids[1]]
	var after := b - a
	if before.length() < 1.0 or after.length() < 1.0:
		return
	_zoom_about((a + b) * 0.5, before.length() / after.length())
	var limit := deg_to_rad(YAW_LIMIT_DEG)
	_target_yaw = clampf(_target_yaw + angle_difference(before.angle(), after.angle()), -limit, limit)


## 搖桿游標的位置（畫面座標）。
func cursor_position() -> Vector2:
	var bounds := get_viewport().get_visible_rect().size
	if _cursor.x < 0.0:
		_cursor = bounds * 0.5
	_cursor = _cursor.clamp(Vector2.ZERO, bounds)
	return _cursor


## 把搖桿游標直接放到畫面上的某一點。
func set_cursor_position(screen: Vector2) -> void:
	_cursor = screen
	_send_pad_motion(Vector2.ZERO)


## 搖桿當滑鼠：右搖桿移動游標，A、LT、RT 的按下與放開轉成滑鼠鍵事件。
func _update_pad_mouse(delta: float) -> void:
	var push := Input.get_vector(InputActions.CURSOR_LEFT, InputActions.CURSOR_RIGHT, InputActions.CURSOR_UP, InputActions.CURSOR_DOWN)
	if push != Vector2.ZERO:
		use_crosshair = true
		var before := cursor_position()
		var speed := CURSOR_SPEED * get_viewport().get_visible_rect().size.y / 1080.0
		# 輸入量取平方，輕推時移得慢、好對準。
		_cursor = before + push * push.length() * speed * delta
		_send_pad_motion(cursor_position() - before)
	for pair: Array in [[InputActions.PAD_LEFT, MOUSE_BUTTON_LEFT], [InputActions.PAD_RIGHT, MOUSE_BUTTON_RIGHT]]:
		if Input.is_action_just_pressed(pair[0]) or Input.is_action_just_released(pair[0]):
			use_crosshair = true
			var button := InputEventMouseButton.new()
			button.device = PAD_DEVICE
			button.button_index = pair[1]
			button.pressed = Input.is_action_pressed(pair[0])
			button.position = cursor_position()
			button.global_position = button.position
			# 直接送進視窗並註明是視窗內座標：畫面有縮放時，走全域輸入會把座標再換算一次。
			get_viewport().push_input(button, true)


func _send_pad_motion(relative: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.device = PAD_DEVICE
	motion.position = cursor_position()
	motion.global_position = motion.position
	motion.relative = relative
	if Input.is_action_pressed(InputActions.PAD_LEFT):
		motion.button_mask |= MOUSE_BUTTON_MASK_LEFT
	if Input.is_action_pressed(InputActions.PAD_RIGHT):
		motion.button_mask |= MOUSE_BUTTON_MASK_RIGHT
	get_viewport().push_input(motion, true)


func _on_mouse_button(event: InputEventMouseButton) -> void:
	if event.device != PAD_DEVICE:
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
				_press_msec = Time.get_ticks_msec()
			else:
				_dragging = false
				# 由觸控轉來的滑鼠事件：長按與雙指手勢放開時不選取。
				var from_touch := event.device == InputEvent.DEVICE_ID_EMULATION
				var long_press := from_touch and Time.get_ticks_msec() - _press_msec >= int(LONG_PRESS_SECONDS * 1000.0)
				var is_click := event.position.distance_to(_press_position) <= CLICK_SLOP
				if is_click and not long_press and not _two_finger_gesture:
					_select_at(event.position)
		MOUSE_BUTTON_RIGHT:
			_rotating = event.pressed


func _on_mouse_motion(event: InputEventMouseMotion) -> void:
	if event.device != PAD_DEVICE:
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
	_update_pad_mouse(delta)
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
	if is_flying():
		# 高度取對數內插，畫面縮放的速度才均勻；兩端緩起緩停。
		_flight_progress = minf(_flight_progress + delta / _flight_seconds, 1.0)
		_target_z = _flight_from_z * pow(_flight_to_z / _flight_from_z, smoothstep(0.0, 1.0, _flight_progress))
		if _flight_progress >= 1.0:
			_flight_seconds = 0.0
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
		return cursor_position()
	return get_viewport().get_mouse_position()


func _province_under(screen: Vector2) -> int:
	return _map.province_at(_map.pick(project_ray_origin(screen), project_ray_normal(screen)))


func _select_at(screen: Vector2) -> void:
	var province := _province_under(screen)
	_map.set_selected(province)
	province_selected.emit(province)


func _update_hover(delta: float) -> void:
	# 指標停在介面器物上時，不顯示地圖上的停留提示。
	if get_viewport().gui_get_hovered_control() != null:
		_set_hover(-1)
		return
	var pointer := _pointer()
	if use_crosshair:
		# 搖桿游標下的州郡即時跟隨。
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
