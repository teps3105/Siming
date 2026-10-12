extends "res://tests/harness/suite.gd"
## 流程：因果模式（GDD 5.3「因果模式後處理」、5.4「因果模式細節」）——F5 進出、轉墨、魂光與因果線、點魂光開人物卡、高亮接點。

var _game: Node
var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _hud: Hud


func suite_name() -> String:
	return "karma_mode"


func setup() -> void:
	_game = await boot()
	_sim = _game.get_node("SimHost")
	_map = _game.get_node("TerrainMap")
	_camera = _game.get_node("MapCamera")
	_hud = _game.get_node("Hud")
	# 推進三年，宿緣與在世的人才有看頭。
	_sim.advance(36)
	await frames(2)


func _karma() -> KarmaLayer:
	return _map.karma_layer()


## 進因果模式並等轉墨走完。
func _enter() -> void:
	await key(KEY_F5)
	await wait(0.5)
	await frames(2)


## 把鏡頭立刻移到某地圖座標上方、中景高度。
func _aim_at(pos: Vector2, z_frac := 0.12) -> void:
	_camera.look_at_map(pos, _map.map_size.x * z_frac, true)
	await frames(3)


func test_bridge_returns_consistent_arrays_within_the_caps() -> void:
	var view := _sim.world.karma_view({})
	var glows: int = (view["glow_person"] as PackedInt32Array).size()
	assert_gt(glows, 0, "有在世的有名之人")
	assert_eq(glows, _sim.world.alive_person_count(), "每位在世有名之人一點魂光")
	assert_eq((view["glow_pos"] as PackedVector2Array).size(), glows)
	assert_eq((view["glow_height"] as PackedFloat32Array).size(), glows)
	assert_eq((view["glow_karma"] as PackedFloat32Array).size(), glows)
	var lines: int = (view["line_thread"] as PackedInt32Array).size()
	for key_name in ["line_kind", "line_debtor", "line_creditor", "line_dead", "line_woven", "line_strength", "line_from", "line_to", "line_from_height", "line_to_height"]:
		assert_eq(view[key_name].size(), lines, key_name)
	assert_gt(lines, 0, "開局與三年間結下的宿緣")
	var capped := _sim.world.karma_view({"all": true, "max_lines": 2, "max_souls": 3})
	assert_lt((capped["line_thread"] as PackedInt32Array).size(), 3, "線數受上限約束")
	assert_gt(capped["qualified_lines"], 2 - 1)
	assert_lt((capped["glow_person"] as PackedInt32Array).size(), 2 * 2 + 4, "魂光受上限約束，有線的端點仍保留")
	var kinds: PackedInt32Array = view["line_kind"]
	for kind in kinds:
		assert_true(kind >= 0 and kind < 5, "種類是五種宿緣之一")


func test_bridge_glows_stay_in_their_own_province_and_do_not_stack() -> void:
	var view := _sim.world.karma_view({})
	var people: PackedInt32Array = view["glow_person"]
	var pos: PackedVector2Array = view["glow_pos"]
	var province: PackedInt32Array = view["glow_province"]
	var seen := {}
	for i in people.size():
		assert_eq(_sim.world.province_at(pos[i].x, pos[i].y), province[i], "魂光落在所在州郡的陸地上")
		var key := Vector3i(province[i], roundi(pos[i].x * 20.0), roundi(pos[i].y * 20.0))
		assert_false(seen.has(key), "同州的人不疊成一點")
		seen[key] = true
	var again := _sim.world.karma_view({})
	assert_eq(again["glow_pos"], view["glow_pos"], "同一世界兩次取得的位置不變")


func test_f5_enters_karma_mode_and_ink_wash_runs_both_ways() -> void:
	var seal: Dictionary = SealBox.SEALS[4]
	assert_eq(seal["mode"], TerrainMap.Mode.KARMA, "印匣的因果已開放")
	assert_eq(InputHints.label(InputActions.MODE_YINGUO, false), "F5")
	await _enter()
	assert_eq(_map.mode, TerrainMap.Mode.KARMA)
	assert_eq(_hud._seals.mode, TerrainMap.Mode.KARMA)
	assert_true(_karma().is_active())
	assert_gt(_map.ink_wash(), 0.99, "地形轉墨")
	var material: ShaderMaterial = _map._terrain_material
	assert_gt(material.get_shader_parameter("ink_wash"), 0.99)
	for node in _karma().get_children():
		assert_true((node as Node3D).visible, "三份副本都顯示")
	await key(KEY_F1)
	await wait(0.6)
	assert_eq(_map.mode, TerrainMap.Mode.SHANHE)
	assert_eq(_map.ink_wash(), 0.0, "離開後不再轉墨")
	assert_eq(material.get_shader_parameter("ink_wash"), 0.0)
	for node in _karma().get_children():
		assert_false((node as Node3D).visible, "離開後魂光與因果線全部隱藏")


func test_karma_layer_draws_the_glows_and_lines_it_was_given() -> void:
	await _enter()
	var layer := _karma()
	assert_eq(layer.glow_count(), _sim.world.alive_person_count())
	assert_gt(layer.line_count(), 0)
	assert_true(layer.line_count() <= KarmaLayer.MAX_LINES)
	assert_true(layer.glow_count() <= KarmaLayer.MAX_GLOWS)
	assert_eq(layer._line_multimesh.visible_instance_count, layer.line_count())
	assert_eq(layer._glow_multimesh.visible_instance_count, layer.glow_count(), "沒有選取時沒有標記")
	# 推進後重讀。
	_sim.advance(12)
	await frames(2)
	assert_eq(layer.glow_count(), _sim.world.alive_person_count(), "推進後魂光跟著人數")


func test_karma_mode_hides_ley_lines_and_keeps_rivers_in_ink() -> void:
	await _enter()
	for copy in _map._ley_copies:
		assert_false(copy.visible)
	var color: Vector3 = _map._river_material.get_shader_parameter("line_color")
	assert_lt(color.x + color.y + color.z, 0.6, "河流色轉墨")


func test_clicking_a_glow_opens_that_persons_card_and_keeps_only_its_lines() -> void:
	await _enter()
	var layer := _karma()
	# 挑一位有線的人。
	var person := -1
	for id: int in layer.line_ends()[0]:
		if id >= 0:
			person = id
			break
	assert_ne(person, -1, "有人身上有線")
	var spot := layer.glow_position_of(person)
	await _aim_at(spot)
	var at := _camera.unproject_position(_map.world_position(spot, _camera.focus.x, _sim.world.karma_view({})["glow_height"][layer.glow_people().find(person)]))
	await click(at)
	await frames(3)
	assert_true(_hud.person_card().is_open(), "點魂光開人物卡")
	assert_eq(_hud.person_card().person, person)
	assert_true(layer.is_highlighting())
	assert_true(layer.highlighted_people().has(person))
	assert_gt(layer.line_count(), 0, "只顯示被選魂的線")
	assert_true(layer.line_count() <= _sim.world.karma_focus(person)["threads"].size())
	assert_gt(layer.mark_count(), 0, "選取環")
	# 點空白處回到全部的線，且不留選取。
	await click(Vector2(960.0, 1040.0))
	await frames(2)
	_hud.person_card().close()
	await frames(2)
	assert_false(layer.is_highlighting(), "關卡後回到全部的線")


func test_nearest_glow_wins_in_a_crowd() -> void:
	await _enter()
	var layer := _karma()
	var people := layer.glow_people()
	var positions := layer.glow_positions()
	# 找同州最近的一對。
	var best := INF
	var a := -1
	var b := -1
	var view := _sim.world.karma_view({})
	var provinces: PackedInt32Array = view["glow_province"]
	for i in people.size():
		for j in range(i + 1, people.size()):
			if provinces[i] != provinces[j]:
				continue
			var d := positions[i].distance_to(positions[j])
			if d < best:
				best = d
				a = i
				b = j
	assert_ne(a, -1, "同州至少有兩個人")
	await _aim_at((positions[a] + positions[b]) * 0.5, 0.05)
	var heights: PackedFloat32Array = view["glow_height"]
	var screen_a := _camera.unproject_position(_map.world_position(positions[a], _camera.focus.x, heights[a]))
	var screen_b := _camera.unproject_position(_map.world_position(positions[b], _camera.focus.x, heights[b]))
	assert_eq(layer.soul_at(_camera, screen_a), people[a], "點在 A 上取 A")
	assert_eq(layer.soul_at(_camera, screen_b), people[b], "點在 B 上取 B")
	assert_eq(layer.soul_at(_camera, Vector2(-500.0, -500.0)), -1, "畫面外沒有")


func test_highlight_api_shows_only_the_given_lines_and_dims_the_rest() -> void:
	await _enter()
	var layer := _karma()
	var view := _sim.world.karma_view({"all": true})
	var threads: PackedInt32Array = view["line_thread"]
	assert_gt(threads.size(), 1)
	var debtor: PackedInt32Array = view["line_debtor"]
	var creditor: PackedInt32Array = view["line_creditor"]
	var pick := PackedInt32Array([threads[0]])
	var people := PackedInt32Array()
	for id in [debtor[0], creditor[0]]:
		if id >= 0:
			people.append(id)
	_map.highlight_karma(people, pick)
	await frames(2)
	assert_true(layer.is_highlighting())
	assert_eq(layer.line_count(), 1, "只畫指定的線")
	assert_eq(layer.line_threads()[0], threads[0])
	assert_eq(layer.glow_count(), _sim.world.alive_person_count(), "魂光仍全數在，只是淡去")
	_map.clear_karma_highlight()
	await frames(2)
	assert_false(layer.is_highlighting())
	assert_gt(layer.line_count(), 1)


func test_legend_lists_the_five_kinds_and_every_string_exists() -> void:
	var keys := _hud._seals.legend_keys(TerrainMap.Mode.KARMA)
	for kind in KarmaLayer.KIND_KEYS:
		assert_true(keys.has("kind_%s" % kind), "例簽列出 %s" % kind)
	assert_true(keys.has("legend_karma_soul"))
	for key_name in keys:
		assert_false(Locale.t(key_name).is_empty(), key_name)
		assert_ne(Locale.t(key_name), key_name, "語系檔有 %s" % key_name)


func test_shoulder_buttons_cycle_through_karma() -> void:
	await key(KEY_F4)
	await joy_button(JOY_BUTTON_RIGHT_SHOULDER)
	assert_eq(_map.mode, TerrainMap.Mode.KARMA, "靈脈的下一個是因果")
	await joy_button(JOY_BUTTON_LEFT_SHOULDER)
	assert_eq(_map.mode, TerrainMap.Mode.LINGMAI)


func test_seal_click_enters_karma_and_lock_on_targets_are_glows() -> void:
	var seal_rect := _hud._seals._seal_rect(4)
	await click(_hud._seals.position + seal_rect.get_center())
	assert_eq(_map.mode, TerrainMap.Mode.KARMA, "點印進因果模式")
	assert_eq(_map.lock_targets().size(), _karma().glow_count(), "搖桿鎖定的目標是魂光")


func test_five_kind_colors_are_the_palette_ones() -> void:
	assert_eq(KarmaLayer.KIND_COLORS[0], Color("B23A2E"), "殺業 seal-red")
	assert_eq(KarmaLayer.KIND_COLORS[1], Color("C9A246"), "恩義 gold")
	assert_eq(KarmaLayer.KIND_COLORS[2], Color("D48A9A"), "情緣 pink")
	assert_eq(KarmaLayer.KIND_COLORS[3], Color("4F8F9A"), "師徒 cyan")
	assert_eq(KarmaLayer.KIND_COLORS[4], Color("5B3A6B"), "怨 violet")
