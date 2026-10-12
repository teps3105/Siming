extends "res://tests/harness/suite.gd"
## 流程：開機後以鍵鼠與搖桿操作地圖、時間與介面（GDD 5.2、5.7、5.9；路線圖 M1 完成判準）。
## 輸入一律經引擎的輸入分派送入，檢查的是鏡頭、模擬與介面的狀態。

var _game: Node
var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _hud: Hud


func suite_name() -> String:
	return "map_journey"


func setup() -> void:
	_game = await boot()
	_sim = _game.get_node("SimHost")
	_map = _game.get_node("TerrainMap")
	_camera = _game.get_node("MapCamera")
	_hud = _game.get_node("Hud")


## 把鏡頭立刻移到第一個州郡的治所上方，回傳該州郡 ID 所在的螢幕位置。
func _aim_at_province(id: int) -> Vector2:
	var seat: Vector2 = _sim.world.province_info(id)["seat"]
	_camera.look_at_map(seat, _map.map_size.x * 0.2, true)
	await frames(2)
	return _camera.unproject_position(Vector3(seat.x, _map.surface_height(seat.x, seat.y), seat.y))


func test_boot_shows_a_world_with_time_stopped_by_the_test() -> void:
	assert_true(_sim.world.is_ready(), "世界應已生成")
	assert_gt(_sim.world.province_count(), 0)
	assert_eq(_map.map_size, Vector2(_sim.world.grid_size()))
	assert_eq(_map.mode, TerrainMap.Mode.SHANHE, "預設為山河模式")
	assert_gt(_hud.get_child_count(), 3, "介面器物應已建立")


func test_time_runs_when_unpaused_and_stops_on_space() -> void:
	var start := _sim.world.tick()
	_sim.set_paused(false)
	_sim.set_speed(3)
	await wait(0.4)
	assert_gt(_sim.world.tick(), start, "放行後時間應前進")
	await key(KEY_SPACE)
	assert_true(_sim.paused, "空白鍵暫停")
	var stopped := _sim.world.tick()
	await wait(0.3)
	assert_eq(_sim.world.tick(), stopped, "暫停後時間不動")


func test_tab_cycles_speed_and_number_keys_do_nothing() -> void:
	await key(KEY_TAB)
	assert_eq(_sim.speed_index, 1, "Tab 切到下一檔")
	assert_false(_sim.paused, "切速度即放行")
	await key(KEY_TAB)
	await key(KEY_TAB)
	assert_eq(_sim.speed_index, 3)
	await key(KEY_TAB)
	assert_eq(_sim.speed_index, 0, "最快之後回到最慢")
	await key(KEY_3)
	assert_eq(_sim.speed_index, 0, "數字鍵不再調速")
	_sim.set_paused(true)
	await click(_hud._dial.position + _hud._dial._stroke_rect(2).get_center())
	assert_eq(_sim.speed_index, 2, "滑鼠點時速簽的第三筆設為第三檔")
	assert_false(_sim.paused)


func test_function_keys_switch_map_mode() -> void:
	await key(KEY_F2)
	assert_eq(_map.mode, TerrainMap.Mode.TIANXIA)
	await key(KEY_F4)
	assert_eq(_map.mode, TerrainMap.Mode.LINGMAI)
	await key(KEY_F1)
	assert_eq(_map.mode, TerrainMap.Mode.SHANHE)


func test_keyboard_pans_the_camera() -> void:
	var before := _camera.focus
	Input.action_press(InputActions.PAN_RIGHT)
	await wait(0.4)
	Input.action_release(InputActions.PAN_RIGHT)
	await wait(0.3)
	assert_gt(_camera.focus.x, before.x, "向右平移後焦點東移")


func test_wheel_zooms_and_drag_pans() -> void:
	var centre := tree.root.get_visible_rect().size * 0.5
	var z_before := _camera.z
	await wheel(centre, true)
	await wait(0.4)
	assert_lt(_camera.z, z_before, "滾輪往上拉近")
	var focus_before := _camera.focus
	await drag(centre, centre + Vector2(160.0, 0.0))
	await wait(0.4)
	assert_lt(_camera.focus.x, focus_before.x, "往右拖曳，地圖跟著手走，焦點西移")
	assert_eq(_hud._card.province, -1, "拖曳不算點選")


func test_click_selects_a_province_and_escape_closes_the_card() -> void:
	var screen := await _aim_at_province(0)
	await click(screen)
	assert_eq(_hud._card.province, 0, "點治所開啟該州郡的卡")
	assert_true(_hud._card.visible)
	await key(KEY_ESCAPE)
	assert_eq(_hud._card.province, -1, "Esc 收卷")


func test_clicking_instruments_does_not_leak_to_the_map() -> void:
	var screen := await _aim_at_province(0)
	await click(screen)
	assert_eq(_hud._card.province, 0)
	var was_paused := _sim.paused
	await click(_hud._dial.position + _hud._dial._disc_center())
	assert_ne(_sim.paused, was_paused, "點天時儀盤心切換行止")
	assert_eq(_hud._card.province, 0, "點器物不應改變地圖的選取")
	await click(_hud._seals.position + _hud._seals._seal_rect(1).get_center())
	assert_eq(_map.mode, TerrainMap.Mode.TIANXIA, "點「國」印切到天下")
	await click(_hud._seals.position + _hud._seals._seal_rect(2).get_center())
	assert_eq(_map.mode, TerrainMap.Mode.TIANXIA, "尚未開放的印不切換模式")
	await click(_hud._card.position + _hud._card._close_rect().get_center())
	assert_eq(_hud._card.province, -1, "點「閉」印收卷")


func test_gamepad_drives_camera_time_and_modes() -> void:
	await joy_button(JOY_BUTTON_RIGHT_SHOULDER)
	assert_eq(_map.mode, TerrainMap.Mode.TIANXIA, "RB 下一個模式")
	await joy_button(JOY_BUTTON_LEFT_SHOULDER)
	assert_eq(_map.mode, TerrainMap.Mode.SHANHE, "LB 上一個模式")
	assert_true(_camera.use_crosshair, "用過搖桿後改以準星選取")
	var focus_before := _camera.focus
	var z_before := _camera.z
	var yaw_before := _camera.yaw
	await joy_axis(JOY_AXIS_LEFT_X, 1.0, 0.4)
	await joy_hold(JOY_BUTTON_DPAD_UP, 0.4)
	await joy_hold(JOY_BUTTON_DPAD_RIGHT, 0.4)
	await wait(0.3)
	assert_gt(_camera.focus.x, focus_before.x, "左搖桿平移")
	assert_lt(_camera.z, z_before, "十字鍵上拉近")
	assert_ne(_camera.yaw, yaw_before, "十字鍵左右旋轉")
	await joy_button(JOY_BUTTON_Y)
	assert_eq(_sim.speed_index, 1, "Y 切到下一檔速度")
	assert_false(_sim.paused, "切速度即放行")
	for i in 3:
		await joy_button(JOY_BUTTON_Y)
	assert_eq(_sim.speed_index, 0, "最快之後回到最慢")
	var was_paused := _sim.paused
	await joy_button(JOY_BUTTON_BACK)
	assert_ne(_sim.paused, was_paused, "Select 切換暫停")
	was_paused = _sim.paused
	var speed := _sim.speed_index
	await joy_button(JOY_BUTTON_START)
	assert_eq(_sim.paused, was_paused, "Start 留給選單，不暫停")
	assert_eq(_sim.speed_index, speed, "Start 不調速")


func test_gamepad_crosshair_selects_and_b_closes() -> void:
	await joy_button(JOY_BUTTON_LEFT_SHOULDER)
	await joy_button(JOY_BUTTON_RIGHT_SHOULDER)
	var seat: Vector2 = _sim.world.province_info(0)["seat"]
	_camera.yaw = 0.0
	_camera.look_at_map(seat, _map.map_size.x * 0.2, true)
	await frames(2)
	await joy_button(JOY_BUTTON_A)
	assert_eq(_hud._card.province, 0, "A 鎖定離游標最近的治所並選取該州郡")
	await joy_button(JOY_BUTTON_B)
	assert_eq(_hud._card.province, -1, "B 返回")


func test_card_center_action_moves_the_camera_to_the_seat() -> void:
	var screen := await _aim_at_province(0)
	await click(screen)
	assert_eq(_hud._card.province, 0)
	var seat: Vector2 = _sim.world.province_info(0)["seat"]
	_camera.look_at_map(seat + Vector2(6.0, 4.0), -1.0, true)
	await frames(2)
	await key(KEY_C)
	await wait(0.6)
	assert_lt(_camera.focus.distance_to(seat), 0.5, "C 把鏡頭移到治所")
	_camera.look_at_map(seat + Vector2(-5.0, 3.0), -1.0, true)
	await frames(2)
	await joy_button(JOY_BUTTON_X)
	await wait(0.6)
	assert_lt(_camera.focus.distance_to(seat), 0.5, "搖桿 X 把鏡頭移到治所")


func test_switching_mode_shows_the_legend_briefly() -> void:
	assert_eq(_hud._seals._flash, 0.0, "一開始不顯示圖例")
	await key(KEY_F2)
	assert_gt(_hud._seals._flash, 0.0, "切換模式後圖例出現")
	await wait(SealBox.FLASH_SECONDS + 0.3)
	assert_eq(_hud._seals._flash, 0.0, "過一會兒自動收起")


func test_province_card_reports_terrain_mix() -> void:
	var mix: PackedInt32Array = _sim.world.province_info(0)["terrain_mix"]
	assert_eq(mix.size(), 9, "九種地形代碼")
	var total := 0
	for count in mix:
		total += count
	assert_eq(total, int(_sim.world.province_info(0)["cells"]), "各地形格數合計等於州郡格數")


func test_view_button_switches_between_globe_and_plane_at_any_zoom() -> void:
	_camera.look_at_map(_map.map_size * 0.5, _map.map_size.x * 1.05, true)
	await wait(0.9)
	assert_eq(_map.globe_morph, 0.0, "拉到最遠仍是平面地圖，不自動變成星球")
	await click(_hud._view.position + _hud._view.size * 0.5)
	await wait(0.9)
	assert_true(_camera.globe, "按視角鈕換到星球")
	assert_true(is_equal_approx(_map.globe_morph, 1.0), "過渡完成：%s" % _map.globe_morph)
	var centre := tree.root.get_visible_rect().size * 0.5
	var hit := _map.pick(_camera.project_ray_origin(centre), _camera.project_ray_normal(centre))
	assert_lt(hit.distance_to(_camera.focus), 1.0, "星球正中央就是鏡頭焦點")
	# 星球視角可以像地球儀一樣拉近：鏡頭停在球面外，正中央仍是焦點。
	_camera.look_at_map(_map.map_size * 0.5, _map.map_size.x * 0.1, true)
	await wait(0.9)
	assert_true(is_equal_approx(_map.globe_morph, 1.0), "拉近後仍是星球")
	var radius := MapView.globe_radius(_map.map_size.x)
	assert_gt(_camera.global_position.distance_to(_map.globe_xform.origin), radius, "鏡頭在球面外")
	hit = _map.pick(_camera.project_ray_origin(centre), _camera.project_ray_normal(centre))
	assert_lt(hit.distance_to(_camera.focus), 1.0, "拉近後正中央仍是鏡頭焦點")
	await key(KEY_G)
	await wait(0.9)
	assert_eq(_map.globe_morph, 0.0, "按 G 變回平面")
	assert_true(is_equal_approx(_camera.z, _map.map_size.x * 0.1), "切換視角不改鏡頭高度")


func test_game_cursor_changes_with_the_situation() -> void:
	# 無視窗的顯示驅動不能改滑鼠模式，只在有視窗時檢查。
	if DisplayServer.get_name() != "headless":
		assert_eq(Input.mouse_mode, Input.MOUSE_MODE_HIDDEN, "系統游標隱藏，由遊戲自己畫")
	var screen := await _aim_at_province(0)
	await _move_mouse(screen)
	assert_eq(_hud._cursor.kind(), &"arrow", "在地圖上是一般游標")
	await _move_mouse(_hud._view.position + _hud._view.size * 0.5)
	assert_eq(_hud._cursor.kind(), &"point", "在按鈕上是可按的游標")
	var centre := Vector2(tree.root.get_visible_rect().size) * 0.5
	await _move_mouse(centre)
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, true, centre))
	await frames(2)
	await _move_mouse(centre + Vector2(60.0, 0.0), MOUSE_BUTTON_MASK_LEFT, Vector2(60.0, 0.0))
	assert_eq(_hud._cursor.kind(), &"drag", "拖動地圖時是拖動的游標")
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, false, centre + Vector2(60.0, 0.0)))
	await frames(2)
	assert_eq(_hud._cursor.kind(), &"arrow", "放開後回到一般游標")
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_RIGHT, true, centre))
	await frames(2)
	assert_eq(_hud._cursor.kind(), &"rotate", "按住右鍵旋轉時是旋轉的游標")
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_RIGHT, false, centre))
	await frames(2)


func _move_mouse(pos: Vector2, mask := 0, relative := Vector2.ZERO) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = pos
	motion.global_position = pos
	motion.relative = relative
	motion.button_mask = mask
	Input.parse_input_event(motion)
	await frames(2)


func test_touch_tap_selects_and_one_finger_drag_pans() -> void:
	var screen := await _aim_at_province(0)
	await tap(screen)
	assert_eq(_hud._card.province, 0, "手指點治所開啟該州郡的卡")
	await key(KEY_ESCAPE)
	var centre := tree.root.get_visible_rect().size * 0.5
	var before := _camera.focus
	await touch(0, centre, true)
	await touch_move(0, centre, centre + Vector2(160.0, 0.0))
	await touch(0, centre + Vector2(160.0, 0.0), false)
	await wait(0.4)
	assert_lt(_camera.focus.x, before.x, "單指往右拖，地圖跟著手走")
	assert_eq(_hud._card.province, -1, "拖曳不算點選")


func test_two_finger_pinch_zooms_without_panning_or_selecting() -> void:
	await _aim_at_province(0)
	var centre := tree.root.get_visible_rect().size * 0.5
	var z_before := _camera.z
	var focus_before := _camera.focus
	var near := Vector2(60.0, 0.0)
	var far := Vector2(220.0, 0.0)
	await two_fingers(centre - near, centre - far, centre + near, centre + far)
	await wait(0.4)
	assert_lt(_camera.z, z_before * 0.6, "兩指張開拉近")
	assert_lt(_camera.focus.distance_to(focus_before), _map.map_size.x * 0.002, "以兩指中點為中心縮放，畫面不平移")
	assert_eq(_hud._card.province, -1, "兩指手勢不算點選")
	var z_near := _camera.z
	await two_fingers(centre - far, centre - near, centre + far, centre + near)
	await wait(0.4)
	assert_gt(_camera.z, z_near * 1.5, "兩指捏合拉遠")


func test_two_finger_twist_turns_the_map_with_the_fingers() -> void:
	await _aim_at_province(0)
	var centre := tree.root.get_visible_rect().size * 0.5
	var east := _camera.focus + Vector2(_camera.z * 0.2, 0.0)
	var east_3d := Vector3(east.x, 0.0, east.y)
	var before := (_camera.unproject_position(east_3d) - centre).angle()
	var arm := Vector2(180.0, 0.0)
	var turned := arm.rotated(deg_to_rad(25.0))
	await two_fingers(centre - arm, centre - turned, centre + arm, centre + turned)
	await wait(0.4)
	var after := (_camera.unproject_position(east_3d) - centre).angle()
	assert_gt(rad_to_deg(after - before), 15.0, "地圖跟著兩指轉同一個方向")
	assert_eq(_hud._card.province, -1, "兩指手勢不算點選")


func test_long_press_shows_the_hint_without_selecting() -> void:
	var screen := await _aim_at_province(0)
	await touch(0, screen, true)
	await wait(0.7)
	assert_eq(_camera._hover_province, 0, "長按顯示該州郡的提示")
	await touch(0, screen, false)
	await frames(2)
	assert_eq(_hud._card.province, -1, "長按放開不算點選")


func test_right_stick_moves_the_cursor_and_triggers_are_mouse_buttons() -> void:
	var screen := await _aim_at_province(0)
	await joy_button(JOY_BUTTON_LEFT_SHOULDER)
	await joy_button(JOY_BUTTON_RIGHT_SHOULDER)
	var centre := Vector2(tree.root.get_visible_rect().size) * 0.5
	assert_lt(_camera.cursor_position().distance_to(_camera._mouse), 1.0, "換到搖桿時，游標從滑鼠所在的位置接手")
	_camera.set_cursor_position(centre)
	await joy_axis(JOY_AXIS_RIGHT_X, 1.0, 0.3)
	assert_gt(_camera.cursor_position().x, centre.x + 100.0, "右搖桿往右推，游標往右移")
	var z_before := _camera.z
	assert_eq(_camera.z, z_before, "右搖桿不再縮放")
	_camera.set_cursor_position(screen)
	await frames(2)
	var speed := _sim.speed_index
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_eq(_hud._card.province, 0, "LT 是滑鼠左鍵：選取游標下的州郡")
	assert_eq(_sim.speed_index, speed, "扳機不再調速")
	assert_true(_camera.use_crosshair, "搖桿轉出的滑鼠事件不會把裝置切回鍵鼠")


func test_pad_cursor_can_click_interface_buttons() -> void:
	await joy_button(JOY_BUTTON_LEFT_STICK)
	_camera.set_cursor_position(_hud._seals.position + _hud._seals._seal_rect(1).get_center())
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_eq(_map.mode, TerrainMap.Mode.TIANXIA, "游標移到模式鈕上按 LT 就切換模式")
	assert_eq(_hud._card.province, -1, "點介面不會選到地圖")
	var was_paused := _sim.paused
	_camera.set_cursor_position(_hud._dial.position + _hud._dial._disc_center())
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_ne(_sim.paused, was_paused, "游標移到行止鈕上按 LT 就切換行止")
	was_paused = _sim.paused
	await joy_button(JOY_BUTTON_A)
	assert_eq(_sim.paused, was_paused, "A 不是滑鼠左鍵，不會按到游標下的介面按鈕")


func test_right_trigger_held_rotates_with_the_cursor() -> void:
	await joy_button(JOY_BUTTON_LEFT_STICK)
	var yaw_before := _camera.yaw
	var hold := InputEventJoypadMotion.new()
	hold.axis = JOY_AXIS_TRIGGER_RIGHT
	hold.axis_value = 1.0
	Input.parse_input_event(hold)
	await frames(2)
	await joy_axis(JOY_AXIS_RIGHT_X, 1.0, 0.3)
	hold = InputEventJoypadMotion.new()
	hold.axis = JOY_AXIS_TRIGGER_RIGHT
	hold.axis_value = 0.0
	Input.parse_input_event(hold)
	await wait(0.3)
	assert_ne(_camera.yaw, yaw_before, "按住 RT（右鍵）移動游標，地圖跟著旋轉")


func test_no_map_hint_while_the_pointer_is_on_an_instrument() -> void:
	await _aim_at_province(0)
	await joy_button(JOY_BUTTON_LEFT_STICK)
	await frames(2)
	assert_eq(_camera._hover_province, 0, "游標在地圖上：顯示游標下的州郡")
	_camera.set_cursor_position(_hud._seals.position + _hud._seals._seal_rect(0).get_center())
	await frames(3)
	assert_eq(_camera._hover_province, -1, "游標在模式鈕上：不顯示地圖的停留提示")


func test_a_locks_on_to_the_nearest_target_and_cycles() -> void:
	await _aim_at_province(0)
	await joy_button(JOY_BUTTON_LEFT_STICK)
	# 把游標放在離治所有一段距離的地方：不必對準，A 會自己吸過去。
	var seat: Vector2 = _sim.world.province_info(0)["seat"]
	var seat_screen := _camera.unproject_position(Vector3(seat.x, _map.surface_height(seat.x, seat.y), seat.y))
	_camera.set_cursor_position(seat_screen + Vector2(70.0, 40.0))
	await frames(2)
	await joy_button(JOY_BUTTON_A)
	assert_eq(_hud._card.province, 0, "A 鎖定離游標最近的治所所在的州郡")
	assert_lt(_camera.cursor_position().distance_to(seat_screen), 2.0, "游標吸到治所上")
	await joy_button(JOY_BUTTON_A)
	var second: int = _hud._card.province
	assert_ne(second, 0, "再按一次換下一個最近的目標")
	assert_gt(second, -1)
	await joy_axis(JOY_AXIS_RIGHT_X, 1.0, 0.05)
	_camera.set_cursor_position(seat_screen + Vector2(10.0, 10.0))
	await frames(2)
	await joy_button(JOY_BUTTON_A)
	assert_eq(_hud._card.province, 0, "動過右搖桿後重新從最近的算起")



## GDD 2.8「畫面」：一世結算改了水陸後，高度場在背景重算，算好後地圖換上新的貼圖與河流，地形網格不重建。
func test_map_swaps_textures_after_the_land_changes() -> void:
	var terrain: MeshInstance3D = _map.get_node("Terrain0")
	var river: MeshInstance3D = _map.get_node("River0")
	var material: ShaderMaterial = terrain.material_override
	var mesh := terrain.mesh
	var elevation: Texture2D = material.get_shader_parameter("elevation_tex")
	var borders: Texture2D = material.get_shader_parameter("pid_tex")
	var river_mesh := river.mesh
	var heard: Array[bool] = []
	var listen := func(changed: bool) -> void: heard.append(changed)
	_map.refreshed.connect(listen)
	var shi := 0
	while not _sim.world.terrain_pending() and shi < 40:
		_sim.advance(360)
		shi += 1
	assert_true(_sim.world.terrain_pending(), "40 世內水陸應有變動")
	var waited := 0
	while not heard.has(true) and waited < 600:
		await frames(1)
		waited += 1
	_map.refreshed.disconnect(listen)
	assert_true(heard.has(true), "高度場算好後地圖應通知換了地形")
	assert_false(_sim.world.terrain_pending())
	assert_eq(terrain.mesh, mesh, "地形網格不重建")
	assert_eq(terrain.material_override, material, "材質沿用")
	assert_ne(material.get_shader_parameter("elevation_tex"), elevation, "高度貼圖換新")
	assert_ne(material.get_shader_parameter("pid_tex"), borders, "州界貼圖換新")
	assert_ne(river.mesh, river_mesh, "河流網格換新")
	# 換上後約 3 秒內在前後兩張貼圖之間漸變（GDD 2.8「畫面」）：舊的高度貼圖留作起點，舊河流淡出。
	var old_river: MeshInstance3D = _map.get_node("RiverOld0")
	assert_eq(material.get_shader_parameter("elevation_prev_tex"), elevation, "舊的高度貼圖留作漸變的起點")
	assert_lt(material.get_shader_parameter("terrain_blend") as float, 0.5, "剛換上時還在起點附近")
	assert_eq(old_river.mesh, river_mesh, "舊河流留著淡出")
	assert_true(old_river.visible)
	_map._process(TerrainMap.TERRAIN_FADE_SECONDS)
	assert_eq(material.get_shader_parameter("terrain_blend") as float, 1.0, "約 3 秒後完全換成新的")
	assert_eq(material.get_shader_parameter("elevation_prev_tex"), material.get_shader_parameter("elevation_tex"), "漸變結束後不再留舊貼圖")
	assert_false(old_river.visible, "舊河流淡完就收起來")
	assert_eq(_map.mode, TerrainMap.Mode.SHANHE)
