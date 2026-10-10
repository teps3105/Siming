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
	assert_lt(card_bottom, _hud._seals.position.y + _hud._seals._legend_top(), "命簿不與例簽重疊")


func test_legend_and_card_use_one_terrain_palette() -> void:
	await _boot(FLOW_VIEW)
	for key in SealBox.TERRAIN_LEGEND:
		assert_true(TerrainSwatches.COLORS.has(key), "例簽的「%s」取自共用色表" % key)
	for key in TerrainSwatches.KEYS:
		assert_true(TerrainSwatches.COLORS.has(key), "命簿的「%s」取自共用色表" % key)


func test_disabled_seals_stay_readable_and_do_not_switch() -> void:
	await _boot(FLOW_VIEW)
	assert_gt(Ink.DISABLED_GLYPH_ALPHA, 0.6, "尚未開放的模式鈕仍看得清字")
	assert_eq(_hud._seals._key_label(2), "", "尚未開放的模式鈕不掛鍵帽")
	await click(_hud._seals.position + _hud._seals._seal_rect(2).get_center())
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
	assert_eq(InputHints.label(InputActions.PAUSE, true), "Start")
	assert_eq(InputHints.label(InputActions.CANCEL, false), "Esc")
	assert_eq(InputHints.label(InputActions.CANCEL, true), "B")
	assert_eq(InputHints.label(InputActions.MODE_LINGMAI, false), "F4")
	assert_eq(InputHints.label(InputActions.SPEED_UP, true), "RT")
	assert_eq(InputHints.label(InputActions.SELECT, true), "A")
	assert_eq(InputHints.label(InputActions.SELECT, false), "", "鍵盤沒有綁定的動作回傳空字串")


func test_every_control_has_text_and_a_key_image() -> void:
	await _boot(FLOW_VIEW)
	for i in SealBox.SEALS.size():
		assert_eq(_hud._seals._button_label(i).length(), 2, "模式鈕寫兩個字的模式名")
	assert_eq(_hud._seals._key_label(0), "F1")
	assert_eq(_hud._seals._key_label(3), "F4")
	assert_eq(_hud._dial._state_label(), Locale.t("dial_paused"), "時速簽寫出行止狀態")
	assert_eq(_hud._dial._pause_key(), "Space")
	assert_eq(_hud._dial._speed_label(), "1 %s" % Locale.t("dial_speed_unit"), "時速簽寫出倍速")
	assert_eq(_hud._dial._speed_keys(), ["1", "4"])
	await key(KEY_3)
	assert_eq(_hud._dial._state_label(), Locale.t("dial_running"))
	assert_eq(_hud._dial._speed_label(), "20 %s" % Locale.t("dial_speed_unit"))
	assert_eq(_hud._card._close_key(), "Esc")
	assert_eq(_hud._card._center_key(), "C")


func test_key_images_follow_the_input_device() -> void:
	await _boot(FLOW_VIEW)
	await joy_button(JOY_BUTTON_Y)
	await frames(2)
	assert_eq(_hud._dial._pause_key(), "Start", "搖桿模式畫搖桿的鍵")
	assert_eq(_hud._dial._speed_keys(), ["LT", "RT"])
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

