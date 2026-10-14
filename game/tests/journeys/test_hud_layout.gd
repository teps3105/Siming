extends "res://tests/harness/suite.gd"
## 介面器物的尺寸與內容（GDD 5.7 區域表、5.8 命簿形式、5.9 Steam Deck 版面）。

const DECK_VIEW := Vector2i(1280, 800)

var _game: Node
var _sim: SimHost
var _hud: Hud


func suite_name() -> String:
	return "hud_layout"


func _boot(view: Vector2i) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")


## 找一個州郡，其 key 欄位的值符合 want_zero（為零或不為零）。
func _province_where(key: String, want_zero: bool) -> int:
	for id in _sim.world.province_count():
		var value: Variant = _sim.world.province_info(id)[key]
		var amount: float = value.length() if value is Vector3 else float(value)
		if (amount < 0.005) == want_zero:
			return id
	return -1


func test_desktop_sizes_match_the_layout_table() -> void:
	await _boot(FLOW_VIEW)
	assert_eq(_hud._dial.position, Vector2(12.0, 12.0), "天時儀左上角內縮 12")
	assert_eq(TimeDial.DISC_RADIUS * 2.0, 56.0, "天時儀直徑 56")
	assert_eq(TimeDial.SEAL_RADIUS * 2.0, 30.0, "盤心印直徑 30")
	assert_eq(_hud._dial._title_rect().size.y, 30.0, "題簽高 30")
	assert_eq(_hud._seals._seal_rect(0).size.y, 38.0, "模式鈕高 38")
	assert_eq(_hud._seals._seal_rect(1).position.x - _hud._seals._seal_rect(0).end.x, 6.0, "印距 6")
	var bounds := Vector2(tree.root.get_visible_rect().size)
	assert_eq(_hud._seals.position + _hud._seals.size, bounds - Vector2(12.0, 12.0), "印匣右下角內縮 12")
	_hud._card.show_province(0)
	await wait(0.5)
	assert_eq(_hud._card.size.x, 380.0, "命簿寬 380")
	assert_eq(_hud._card._close_rect().size, Vector2(26.0, 26.0), "閉印直徑 26")


func test_deck_sizes_and_text_floor() -> void:
	await _boot(DECK_VIEW)
	assert_lt(absf(_hud._dial._disc_radius() * 2.0 - 64.0), 0.01, "Deck 天時儀直徑 64")
	assert_lt(absf(_hud._dial._title_rect().size.y - 34.0), 0.5, "Deck 題簽高 34")
	assert_lt(absf(_hud._seals._seal_rect(0).size.y - 43.5), 0.6, "Deck 模式鈕高約 43")
	for base: int in [Ink.SIZE_NOTE, Ink.SIZE_SECTION, Ink.SIZE_BODY, Ink.SIZE_TITLE]:
		assert_gt(Ink.font_size(base, _hud._unit), 15, "Deck 下所有字級至少 16")
	assert_eq(Ink.font_size(Ink.SIZE_NOTE, 1.0), Ink.SIZE_NOTE, "桌機字級不變")
	assert_gt(Ink.font_size(Ink.KEY_TEXT_SIZE, _hud._unit), 15, "Deck 下鍵帽裡的字也至少 16")
	_hud._card.show_province(0)
	await wait(0.5)
	assert_lt(absf(_hud._card.size.x - DECK_VIEW.x * 0.45), 1.0, "Deck 命簿是右側 45% 寬的抽屜")


func test_close_seal_is_round() -> void:
	await _boot(FLOW_VIEW)
	_hud._card.show_province(0)
	await wait(0.5)
	var rect: Rect2 = _hud._card._close_rect()
	await click(_hud._card.position + rect.position + Vector2(1.5, 1.5))
	assert_eq(_hud._card.province, 0, "點在圓印外接方框的角上不收卷")
	await click(_hud._card.position + rect.get_center())
	assert_eq(_hud._card.province, -1, "點圓印收卷")


func test_card_hides_empty_bars_and_empty_faith() -> void:
	await _boot(FLOW_VIEW)
	var quiet := _province_where("merit", true)
	assert_gt(quiet, -1, "測試世界應有功德為零的州郡")
	_hud._card.show_province(quiet)
	await wait(0.5)
	assert_false(_hud._card._bar_keys().has("card_merit"), "值為零的數值條收起")
	assert_true(_hud._card._bar_keys().has("card_aura"), "靈氣條一律顯示")
	var faithless := _province_where("faith", true)
	if faithless >= 0:
		_hud._card.show_province(faithless)
		await wait(0.3)
		assert_false(_hud._card._shows_faith_ring(), "沒有教化時不畫教化環")
	var card_bottom: float = _hud._card.position.y + _hud._card.size.y
	assert_lt(card_bottom, _hud._seals.top_edge(), "命簿不與印匣重疊")


func test_card_height_limit_follows_the_legend_when_it_shows() -> void:
	await _boot(FLOW_VIEW)
	_hud._card.show_province(0)
	await wait(0.5)
	var plain: float = _hud._card.max_height
	assert_gt(plain, _hud._seals.top_edge() - Hud.MARGIN * 2.0 - 1.0, "平時卡高上限到印匣上緣之上一個間距")
	_hud._seals._flash = 5.0
	await frames(3)
	assert_lt(_hud._card.max_height, plain, "例簽顯示時，卡高上限縮到例簽上緣之上")
	assert_lt(_hud._card.position.y + _hud._card.size.y, _hud._seals.top_edge(), "命簿不壓到例簽")
	_hud._seals._flash = 0.0
	await frames(3)
	assert_eq(_hud._card.max_height, plain, "例簽收起後恢復")


func test_legend_and_card_use_one_terrain_palette() -> void:
	await _boot(FLOW_VIEW)
	for key in SealBox.TERRAIN_LEGEND:
		assert_true(TerrainSwatches.COLORS.has(key), "例簽的「%s」取自共用色表" % key)
	for key in TerrainSwatches.KEYS:
		assert_true(TerrainSwatches.COLORS.has(key), "命簿的「%s」取自共用色表" % key)


func test_disabled_seals_stay_readable_and_do_not_switch() -> void:
	await _boot(FLOW_VIEW)
	assert_gt(Ink.DISABLED_GLYPH_ALPHA, 0.6, "尚未開放的模式鈕仍看得清字")
	assert_eq(_hud._seals._key_label(5), "", "尚未開放的模式鈕不掛鍵帽")
	await click(_hud._seals.position + _hud._seals._seal_rect(5).get_center())
	assert_eq(_hud._seals.mode, TerrainMap.Mode.SHANHE, "尚未開放的印不切換模式")


func test_speed_strokes_sit_on_their_own_slip() -> void:
	await _boot(FLOW_VIEW)
	var slip: Rect2 = _hud._dial._speed_rect()
	for i in TimeDial.STROKE_COUNT:
		assert_true(slip.encloses(_hud._dial._stroke_rect(i)), "疾徐四筆畫在疾徐簽上")
	assert_true(_sim.paused)
	await click(_hud._dial.position + _hud._dial._stroke_rect(2).get_center())
	assert_eq(_sim.speed_index, 2, "點第三筆設為第三速")
	assert_false(_sim.paused, "點疾徐即放行")


func test_input_hints_come_from_the_binding_table() -> void:
	assert_eq(InputHints.label(InputActions.PAUSE, false), "Space")
	assert_eq(InputHints.label(InputActions.PAUSE, true), "Select")
	assert_eq(InputHints.label(InputActions.MENU, true), "Start")
	assert_eq(InputHints.label(InputActions.CANCEL, false), "Esc")
	assert_eq(InputHints.label(InputActions.CANCEL, true), "B")
	assert_eq(InputHints.label(InputActions.MODE_LINGMAI, false), "F4")
	assert_eq(InputHints.label(InputActions.SPEED_CYCLE, true), "Y")
	assert_eq(InputHints.labels(InputActions.PAD_LEFT, true), ["LT"], "A 不兼作滑鼠左鍵")
	assert_eq(InputHints.label(InputActions.LOCK_ON, true), "A")
	assert_eq(InputHints.label(InputActions.PAD_RIGHT, true), "RT")
	assert_eq(InputHints.label(InputActions.CURSOR_LEFT, true), Ink.PAD_RIGHT_STICK, "右搖桿畫成圖標")
	assert_eq(InputHints.label(InputActions.ZOOM_IN, true), Ink.PAD_DPAD_VERTICAL, "縮放是十字鍵上下")
	assert_eq(InputHints.label(InputActions.ROTATE_RIGHT, true), Ink.PAD_DPAD_HORIZONTAL, "旋轉是十字鍵左右")
	assert_eq(Ink.key_glyph_width(Ink.PAD_RIGHT_STICK, 1.0), Ink.KEY_HEIGHT, "圖標是方的，不隨文字變寬")
	assert_eq(InputHints.label(InputActions.PAD_LEFT, false), "", "鍵盤沒有綁定的動作回傳空字串")


func test_every_control_has_text_and_a_key_image() -> void:
	await _boot(FLOW_VIEW)
	for i in SealBox.SEALS.size():
		assert_eq(_hud._seals._button_label(i).length(), 2, "模式鈕寫兩個字的模式名")
	assert_eq(_hud._seals._key_label(0), "F1")
	assert_eq(_hud._seals._key_label(3), "F4")
	assert_eq(_hud._dial._state_label(), Locale.t("dial_paused"), "時速簽寫出行止狀態")
	assert_eq(_hud._dial._pause_key(), "Space")
	assert_eq(_hud._dial._speed_label(), "1日/秒", "時速簽寫出每秒幾日")
	assert_eq(_hud._dial._speed_keys(), ["Tab"])
	await key(KEY_TAB)
	await key(KEY_TAB)
	assert_eq(_hud._dial._state_label(), Locale.t("dial_running"))
	assert_eq(_hud._dial._speed_label(), "5日/秒")
	await key(KEY_TAB)
	assert_eq(_hud._dial._speed_label(), "10日/秒", "最快每秒 10 日")
	assert_eq(_hud._dial._fast_key(), "R", "快進鈕帶快捷鍵的圖像")
	assert_eq(_hud._view._key(), "G", "視角鈕帶快捷鍵的圖像")
	assert_false(_hud._view._label().is_empty(), "視角鈕有文字")
	var badge_owner: Rect2 = _hud._seals._seal_rect(0)
	assert_lt(_hud._seals._badge_position(0).x, badge_owner.get_center().x, "模式鈕的鍵帽掛在左上角")
	assert_eq(_hud._card._close_key(), "Esc")
	assert_eq(_hud._card._center_key(), "C")


func test_key_images_follow_the_input_device() -> void:
	await _boot(FLOW_VIEW)
	await joy_button(JOY_BUTTON_PADDLE1)
	await frames(2)
	assert_eq(_hud._dial._pause_key(), "Select", "搖桿模式畫搖桿的鍵")
	assert_eq(_hud._dial._speed_keys(), ["Y"])
	assert_eq(_hud._seals._key_label(0), "", "搖桿模式不掛鍵帽")
	assert_eq(_hud._seals._cycle_keys(), ["LB", "RB"])
	assert_eq(_hud._card._close_key(), "B")
	assert_eq(_hud._card._center_key(), "X")
	await click(Vector2(tree.root.get_visible_rect().size) * 0.5)
	await frames(2)
	assert_eq(_hud._dial._pause_key(), "Space", "動了滑鼠就換回鍵盤的鍵")


func test_clicking_the_state_text_toggles_pause() -> void:
	await _boot(FLOW_VIEW)
	assert_true(_sim.paused)
	var slip: Rect2 = _hud._dial._speed_rect()
	await click(_hud._dial.position + slip.position + Vector2(12.0, slip.size.y * 0.5))
	assert_false(_sim.paused, "點時速簽的狀態字放行")


func test_path_labels_shrink_then_hide_when_the_path_is_short() -> void:
	var gap := MapOverlay.PATH_SPACING_MIN
	# 三個字、字級 15：兩個字距至少要 15 × gap × 2 像素。
	assert_eq(MapOverlay.path_label_size(400.0, 3, 15, 11), 15, "路徑夠長用原字級")
	assert_eq(MapOverlay.path_label_size(15.0 * gap * 2.0, 3, 15, 11), 15, "剛好排得下")
	assert_eq(MapOverlay.path_label_size(13.0 * gap * 2.0 + 0.5, 3, 15, 11), 13, "排不下就縮小字級，字距仍不小於下限")
	assert_eq(MapOverlay.path_label_size(11.0 * gap * 2.0, 3, 15, 11), 11, "縮到字級下限")
	assert_eq(MapOverlay.path_label_size(11.0 * gap * 2.0 - 1.0, 3, 15, 11), 0, "低於字級下限就不顯示")
	assert_eq(MapOverlay.path_label_size(400.0, 1, 15, 11), 0, "只有一個字不沿路徑排")


func test_each_kind_of_place_name_has_its_own_colour() -> void:
	var seen := {}
	for kind: String in MapOverlay.KIND_COLORS:
		var colour: Color = MapOverlay.KIND_COLORS[kind]
		assert_false(seen.has(colour.to_html()), "「%s」的顏色與別種地名相同" % kind)
		seen[colour.to_html()] = true
	for kind in SealBox.PLACE_LEGEND:
		assert_true(MapOverlay.KIND_COLORS.has(kind), "例簽列的「%s」在地圖上有對應的顏色" % kind)
	assert_eq(SealBox.PLACE_LEGEND.size(), MapOverlay.KIND_COLORS.size(), "例簽列出全部地名種類")


func test_buttons_show_a_pressed_state_while_held() -> void:
	await _boot(FLOW_VIEW)
	var at: Vector2 = _hud._seals.position + _hud._seals._seal_rect(1).get_center()
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, true, at))
	await frames(2)
	assert_eq(_hud._seals._pressed, 1, "按住模式鈕時顯示按下的樣子")
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, false, at))
	await frames(2)
	assert_eq(_hud._seals._pressed, -1, "放開後恢復")
	at = _hud._dial.position + _hud._dial._disc_center()
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, true, at))
	await frames(2)
	assert_eq(_hud._dial._pressed, TimeDial.PRESSED_PAUSE, "按住行止鈕時顯示按下的樣子")
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, false, at))
	await frames(2)
	assert_eq(_hud._dial._pressed, -1)


func test_a_path_half_a_map_away_is_not_split_across_the_screen() -> void:
	await _boot(FLOW_VIEW)
	var camera: MapCamera = _game.get_node("MapCamera")
	var map: TerrainMap = _game.get_node("TerrainMap")
	camera.look_at_map(map.map_size * 0.5, map.map_size.x * 0.16, true)
	await frames(2)
	# 一條短路徑，正好跨在離鏡頭半個地圖寬的地方：兩端各自最近的副本一東一西。
	var far_x := camera.focus.x + map.map_size.x * 0.5
	var path := PackedVector2Array([Vector2(far_x - 1.0, camera.focus.y), Vector2(far_x + 1.0, camera.focus.y)])
	var points: PackedVector2Array = _hud._overlay._path_on_screen(path)
	assert_eq(points.size(), 2)
	assert_lt(points[0].distance_to(points[1]), 400.0, "相鄰兩格在畫面上應該相鄰，不是分在畫面兩側")


func test_map_labels_keep_clear_of_the_instruments() -> void:
	await _boot(FLOW_VIEW)
	_hud._card.show_province(0)
	await wait(0.5)
	var reserved: Array[Rect2] = _hud._overlay.reserved
	assert_eq(reserved.size(), 6, "圓盤、兩張簽、傳聞簽、模式鈕、視角鈕與開著的命簿")
	var card := Rect2(_hud._card.position, _hud._card.size)
	assert_true(reserved[3].encloses(card), "命簿的範圍在避讓區內")
	_hud._card.close()
	await wait(0.5)
	assert_eq(_hud._overlay.reserved.size(), 5, "命簿收起後不再佔位")


func test_card_title_is_written_vertically_on_a_side_strip() -> void:
	await _boot(FLOW_VIEW)
	_hud._card.show_province(0)
	await wait(0.5)
	var strip: Rect2 = _hud._card._title_strip_rect()
	assert_gt(strip.size.y, strip.size.x * 3.0, "題簽是直的一條")
	assert_lt(strip.position.x, _hud._card.size.x * 0.1, "題簽在紙面左側")
	var name: String = _sim.world.province_info(0)["name"]
	var needed := Ink.text_size(name[0], Ink.SIZE_TITLE).y * name.length()
	assert_gt(strip.size.y, needed + 14.0, "題簽放得下直書的州郡名")
	assert_gt(_hud._card._center_rect().position.x, strip.end.x, "內容與動作鈕在題簽右邊")


func test_water_names_stay_on_the_water() -> void:
	await _boot(FLOW_VIEW)
	var camera: MapCamera = _game.get_node("MapCamera")
	var map: TerrainMap = _game.get_node("TerrainMap")
	var overlay: MapOverlay = _hud._overlay
	var lakes := 0
	var hidden_or_shrunk := 0
	for label: Dictionary in _sim.world.map_labels():
		if label["kind"] != "lake":
			continue
		lakes += 1
		var pos: Vector2 = label["pos"]
		camera.look_at_map(pos, map.map_size.x * 0.25, true)
		await frames(1)
		var world_pos := Vector3(pos.x, map.surface_height(pos.x, pos.y), pos.y)
		var point := camera.unproject_position(world_pos)
		var size := overlay._fit_on_water(point, world_pos.y, label["text"], 14)
		if size < 14:
			hidden_or_shrunk += 1
		if size > 0:
			var half := Ink.text_size(label["text"], size) * 0.5
			for corner: Vector2 in [Vector2(-1, -1), Vector2(1, -1), Vector2(-1, 1), Vector2(1, 1)]:
				assert_true(overlay._is_water_at(point + corner * half, world_pos.y), "「%s」的四角都在水面上" % label["text"])
	assert_gt(lakes, 0, "測試世界應有湖")
	assert_gt(hidden_or_shrunk, 0, "拉遠時，小湖放不下原字級的名字：要嘛縮小、要嘛不顯示")


func test_time_slips_keep_one_fixed_width() -> void:
	await _boot(FLOW_VIEW)
	var title_width: float = _hud._dial._title_rect().size.x
	var speed_width: float = _hud._dial._speed_rect().size.x
	assert_eq(title_width, speed_width, "題簽與時速簽同寬")
	# 推進到月份變兩位數、年份多一位數，寬度都不變。
	var seen_two_digit_month := false
	for i in 30:
		_sim.world.step(1)
		_hud._dial.queue_redraw()
		assert_eq(_hud._dial._title_rect().size.x, title_width, "日期變了，題簽寬度不變：%s" % EventText.now_label(_sim.world))
		if EventText.now_label(_sim.world).ends_with("十一月"):
			seen_two_digit_month = true
	assert_true(seen_two_digit_month, "測試涵蓋到兩位數的月份")
	# 切到每一檔速度，寬度都不變。
	for i in SimHost.DAYS_PER_SECOND.size():
		await key(KEY_TAB)
		assert_eq(_hud._dial._speed_rect().size.x, speed_width, "每秒日數 %s 時寬度不變" % _hud._dial._speed_label())
	# 換成搖桿（鍵圖像不同），寬度也不變。
	await joy_button(JOY_BUTTON_PADDLE1)
	await frames(2)
	assert_eq(_hud._dial._speed_rect().size.x, speed_width, "換輸入裝置時寬度不變")
	assert_eq(_hud._dial._title_rect().size.x, title_width)
	for i in TimeDial.STROKE_COUNT:
		assert_true(_hud._dial._speed_rect().encloses(_hud._dial._stroke_rect(i)), "四筆仍在時速簽內")


func test_date_is_centred_in_the_title_slip() -> void:
	await _boot(FLOW_VIEW)
	for i in 3:
		var title: Rect2 = _hud._dial._title_rect()
		var date: Rect2 = _hud._dial._date_rect()
		assert_lt(absf(date.get_center().x - title.get_center().x), 0.5, "整串日期在題簽裡置中：%s" % EventText.now_label(_sim.world))
		assert_true(title.encloses(date), "日期不超出題簽")
		_sim.world.step(700)


func test_gdd_2_8_dial_carries_two_arcs_with_dots_inside_the_outer_ring() -> void:
	for view: Vector2i in [FLOW_VIEW, DECK_VIEW]:
		await _boot(view)
		var dial := _hud._dial
		var center := dial._disc_center()
		assert_gt(center.x - dial.outer_radius(), -0.01, "外圈不超出器物的左上角")
		for kind: StringName in [&"warmth", &"tide"]:
			var dot := dial.arc_dot(kind)
			assert_gt(dot.distance_to(center), dial._disc_radius(), "朱點在圓盤外")
			assert_lt(dot.distance_to(center) + TimeDial.ARC_DOT * _hud._unit, dial.outer_radius(), "朱點在外圈內")
		assert_lt(dial.arc_dot(&"warmth").x, center.x, "寒暖在左弧")
		assert_gt(dial.arc_dot(&"tide").x, center.x, "靈潮在右弧")
		assert_gt(dial._title_rect().position.x, center.x + dial.outer_radius(), "題簽讓開外圈")
		assert_gt(_hud._view.position.y, dial.position.y + dial.outer_radius() * 2.0, "視角鈕在外圈之下")
		assert_gt(TimeDial.ARC_DOT * 2.0 * _hud._unit, 6.9, "朱點直徑至少 7 px")


func test_gdd_2_8_arc_dot_and_arrow_follow_the_world() -> void:
	await _boot(FLOW_VIEW)
	var dial := _hud._dial
	var seen := {}
	for shi in 40:
		var c: Dictionary = _sim.world.climate()
		var want := signi(int(signf(float(c["warmth_next"]) - float(c["warmth"])))) if absf(float(c["warmth_next"]) - float(c["warmth"])) >= 0.02 else 0
		assert_eq(dial.trend(&"warmth"), want)
		seen[dial.trend(&"warmth")] = true
		# 左弧下寒上暖：寒暖愈高朱點愈高。
		var level: float = inverse_lerp(-1.0, 1.0, c["warmth"])
		assert_lt(absf(dial.arc_dot(&"warmth").y - dial._arc_point(&"warmth", level).y), 0.01)
		_sim.advance(360)
	assert_true(seen.has(1) and seen.has(-1), "四十世裡寒暖應有升有降")
	assert_lt(dial._arc_point(&"warmth", 1.0).y, dial._arc_point(&"warmth", 0.0).y, "暖在上")
	assert_lt(dial._arc_point(&"tide", 1.0).y, dial._arc_point(&"tide", 0.0).y, "盛在上")


func test_gdd_2_8_hovering_an_arc_explains_it() -> void:
	await _boot(FLOW_VIEW)
	var dial := _hud._dial
	assert_eq(dial.hover_text(), "")
	await move_mouse(dial.position + dial.arc_dot(&"warmth"))
	assert_true(dial.hover_text().begins_with(Locale.t("dial_warmth")), "停在左弧說寒暖：%s" % dial.hover_text())
	assert_false(dial.is_hot(dial.arc_dot(&"warmth")), "弧不能按")
	await move_mouse(dial.position + dial.arc_dot(&"tide"))
	assert_true(dial.hover_text().begins_with(Locale.t("dial_tide")), "停在右弧說靈潮：%s" % dial.hover_text())
	await move_mouse(Vector2(900.0, 600.0))
	assert_eq(dial.hover_text(), "", "移開後說明簽收起")


func test_gdd_2_8_a_world_change_posts_a_notice_that_goes_away() -> void:
	await _boot(FLOW_VIEW)
	var notice := _hud._notice
	assert_false(notice.showing(), "開局沒有告示")
	var heard: Array = []
	_sim.world_changed.connect(func(change: String, era: String) -> void: heard.append([change, era]))
	for shi in 60:
		_sim.advance(360)
		if not heard.is_empty():
			break
	assert_gt(heard.size(), 0, "六十世裡應有世界大事")
	assert_true(notice.showing(), "世界大事發生時出現告示")
	assert_eq(notice.text(), Hud._world_notice(heard[0][0], heard[0][1]))
	var bounds := Vector2(tree.root.get_visible_rect().size)
	var rect := Rect2(notice.position, notice.size)
	assert_lt(absf(rect.get_center().x - bounds.x * 0.5), 1.0, "告示在上方中央")
	await wait(NoticeSlip.HOLD_SECONDS * heard.size() + NoticeSlip.FADE_SECONDS * 2.0 * heard.size() + 0.5)
	assert_false(notice.showing(), "告示停幾秒後收起")


func test_gdd_2_8_notice_text_names_the_new_era() -> void:
	assert_eq(Hud._world_notice("ColdBegan", "玄冥"), "天下大寒，改元玄冥")
	assert_eq(Hud._world_notice("ColdEnded", ""), "大寒已過")

