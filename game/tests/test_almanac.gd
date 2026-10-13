extends "res://tests/harness/suite.gd"
## 天下冊（GDD 5.10）：B 鍵與題簽的冊印展開、暫停與還原、四頁、圖表項點出名冊且筆數相符、
## 名冊的州名／人名／宗門名可點、與傳聞全螢幕和命簿的互斥、兩欄各自捲動。

const DECK_VIEW := Vector2i(1280, 800)

var _game: Node
var _sim: SimHost
var _hud: Hud
var _camera: MapCamera
var _almanac: Almanac
var _rumor: RumorSlip


func suite_name() -> String:
	return "almanac"


func setup() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func teardown() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func _boot(view: Vector2i = FLOW_VIEW) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_camera = _game.get_node("MapCamera")
	_almanac = _hud.almanac()
	_rumor = _hud.rumor()


## 展開並等兩幀，讓兩欄量好。
func _open(index := 0) -> void:
	_almanac.page = index
	_almanac.expanded = true
	await frames(2)


func _picks() -> Array[String]:
	var out: Array[String] = []
	for target in _almanac.left_targets():
		if target.begins_with("pick:") and _almanac.chart_count(target) >= 0:
			out.append(target)
	return out


func test_b_key_expands_and_collapses_with_the_pause_restored() -> void:
	await _boot()
	_sim.set_paused(false)
	_sim.set_speed(2)
	await key(KEY_B)
	assert_true(_almanac.expanded, "B 展開")
	assert_true(_sim.paused, "展開時模擬暫停")
	assert_true(_camera.modal, "展開時鏡頭讓出畫面")
	var tick := _sim.world.tick()
	await frames(10)
	assert_eq(_sim.world.tick(), tick, "展開期間 tick 不動")
	await key(KEY_B)
	assert_false(_almanac.expanded, "再按 B 收合")
	assert_false(_sim.paused, "收合後回到展開前的運行")
	assert_eq(_sim.speed_index, 2, "速度也回到展開前")
	assert_false(_camera.modal, "收合後鏡頭交還")


func test_stays_paused_when_it_was_paused_before() -> void:
	await _boot()
	_sim.set_paused(true)
	await key(KEY_B)
	await key(KEY_B)
	assert_true(_sim.paused, "展開前就暫停，收合後仍暫停")


func test_esc_joy_b_and_dim_click_collapse() -> void:
	await _boot()
	await _open()
	await key(KEY_ESCAPE)
	assert_false(_almanac.expanded, "Esc 收合")
	assert_false(_hud.menu_open(), "Esc 只收合，不開選單")
	await _open()
	await joy_button(JOY_BUTTON_B)
	assert_false(_almanac.expanded, "搖桿 B 收合")
	await _open()
	await click(Vector2(4.0, 4.0))
	assert_false(_almanac.expanded, "點紙外的暗處收合")
	await _open()
	await click(_almanac.paper_rect().get_center())
	assert_true(_almanac.expanded, "點紙面內不收合")


func test_the_seal_on_the_date_plate_opens_it() -> void:
	await _boot()
	var dial: TimeDial = _hud._dial
	await click(dial.position + dial._almanac_rect().get_center())
	await frames(2)
	assert_true(_almanac.expanded, "點題簽右端的冊印展開")
	assert_true(_sim.paused, "展開即暫停")


func test_tabs_switch_and_the_last_page_is_remembered() -> void:
	await _boot()
	await _open()
	assert_eq(_almanac.page_key(), "world", "初次打開是天下頁")
	await key(KEY_RIGHT)
	assert_eq(_almanac.page_key(), "people", "右鍵換到下一頁")
	await key(KEY_RIGHT)
	await key(KEY_RIGHT)
	await key(KEY_RIGHT)
	assert_eq(_almanac.page_key(), "karma", "到最後一頁不繞回")
	await key(KEY_LEFT)
	assert_eq(_almanac.page_key(), "sect", "左鍵回上一頁")
	await click(_almanac.tab_rect(1).get_center())
	assert_eq(_almanac.page_key(), "people", "點頁籤換頁")
	await key(KEY_B)
	await key(KEY_B)
	assert_eq(_almanac.page_key(), "people", "收合後再開，停在上次的一頁")
	_hud._build()
	_almanac = _hud.almanac()
	assert_eq(_almanac.page_key(), "people", "器物重建後也記得")


func test_world_charts_fill_the_roster_with_matching_counts() -> void:
	await _boot()
	await _open(0)
	var picks := _picks()
	assert_gt(picks.size(), 0, "天下頁有可點的圖表項")
	for target in picks:
		_almanac.activate_target(target)
		var rows := _almanac.roster_rows()
		assert_eq(_almanac.roster_total(), _almanac.chart_count(target), "%s：名冊總數等於圖上筆數" % target)
		assert_eq(rows.size(), mini(_almanac.roster_total(), Almanac.ROSTER_LIMIT), "%s：名冊列數" % target)
	var world: Dictionary = _sim.world.stats_world()
	var provinces := 0
	for c: Dictionary in world["continents"]:
		provinces += int(c["provinces"])
	assert_eq(provinces, int(world["provinces"]), "各大洲州數相加等於州郡總數")
	_almanac.activate_target("pick:terrain:2")
	assert_eq(_almanac.roster_rows().size(), _almanac.roster_total(), "地形名冊列出全部有這種地形的州郡")
	assert_gt(_almanac.roster_total(), 0, "平原至少有一州")
	_almanac.activate_target("pick:faith:3")
	assert_eq(_almanac.roster().get("type"), "province", "教化環點出州郡名冊")


func test_people_charts_fill_the_roster_with_matching_counts() -> void:
	await _boot()
	await _open(1)
	var people: Dictionary = _sim.world.stats_people()
	var sum := int(people["non_cultivators"])
	for n in people["realms"]:
		sum += int(n)
	assert_eq(sum, int(people["alive"]), "各境界加凡俗等於在世有名之人")
	var picks := _picks()
	assert_gt(picks.size(), 10, "修士頁有境界、出身、男女、年紀、道途、近百年等可點項")
	for target in picks:
		_almanac.activate_target(target)
		assert_eq(_almanac.roster_total(), _almanac.chart_count(target), "%s：名冊總數等於圖上筆數" % target)
		assert_eq(_almanac.roster_rows().size(), mini(_almanac.roster_total(), Almanac.ROSTER_LIMIT), "%s：名冊列數" % target)
	var origin_sum := 0
	for target in picks:
		if target.begins_with("pick:origin:"):
			origin_sum += _almanac.chart_count(target)
	assert_eq(origin_sum, int(people["alive"]), "出身各項相加等於在世有名之人")
	var gender_sum := _almanac.chart_count("pick:gender:0") + _almanac.chart_count("pick:gender:1")
	assert_eq(gender_sum, int(people["alive"]), "男女相加等於在世有名之人")


func test_sect_and_karma_charts_fill_the_roster_with_matching_counts() -> void:
	await _boot()
	await _open(2)
	assert_gt((_sim.world.stats_sects()["alive"] as Array).size(), 0, "試驗世界裡有宗門")
	var picks := _picks()
	assert_gt(picks.size(), 0, "宗門頁有可點項")
	for target in picks:
		_almanac.activate_target(target)
		assert_eq(_almanac.roster_total(), _almanac.chart_count(target), "%s：名冊總數等於圖上筆數" % target)
	_almanac.page = 3
	await frames(2)
	var karma_picks := _picks()
	assert_eq(karma_picks.size(), 5, "因果頁五類宿緣")
	for target in karma_picks:
		_almanac.activate_target(target)
		assert_eq(_almanac.roster_total(), _almanac.chart_count(target), "%s：名冊總數等於圖上筆數" % target)
		assert_true(_almanac.roster_rows().size() <= _almanac.roster_total(), "%s：列數不超過總數" % target)


func test_chronicle_is_a_single_closed_line() -> void:
	await _boot()
	await _open(3)
	assert_true(Locale.t("almanac_chronicle_closed") == "歲時紀錄尚未開放", "歲時一行字")
	for target in _almanac.left_targets():
		assert_false(target.contains("chronicle"), "歲時區沒有可點的曲線或年表")


func test_clicking_a_province_name_opens_its_card_and_collapses() -> void:
	await _boot()
	await _open(0)
	_almanac.activate_target("pick:continent:0")
	await frames(2)
	var rows := _almanac.roster_rows()
	assert_gt(rows.size(), 0, "大洲名冊有州郡")
	var id := int((rows[0] as Dictionary)["id"])
	var point := _almanac.target_point("go:province:%d" % id, true)
	assert_true(point.x >= 0.0, "名冊裡找得到州名")
	await click(point)
	await frames(2)
	assert_false(_almanac.expanded, "點州名後收合")
	assert_eq(_hud.province_card().province, id, "開出那一州的州郡卡")
	assert_false(_camera.modal, "鏡頭交還")


func test_clicking_a_person_name_opens_the_person_card() -> void:
	await _boot()
	await _open(1)
	_almanac.activate_target("pick:realm:0")
	await frames(2)
	var rows := _almanac.roster_rows()
	assert_gt(rows.size(), 0, "煉氣名冊有人")
	var id := int((rows[0] as Dictionary)["id"])
	var point := _almanac.target_point("go:person:%d" % id, true)
	assert_true(point.x >= 0.0, "名冊裡找得到人名")
	await click(point)
	await frames(2)
	assert_false(_almanac.expanded, "點人名後收合")
	assert_eq(_hud.person_card().person, id, "開出那位的人物卡")


func test_clicking_a_sect_name_flies_to_the_gate() -> void:
	await _boot()
	await _open(2)
	var sects: Array = _sim.world.stats_sects()["alive"]
	assert_gt(sects.size(), 0, "試驗世界裡有宗門")
	var id := int((sects[0] as Dictionary)["id"])
	var flown := []
	_almanac.sect_requested.connect(func(pos: Vector2) -> void: flown.append(pos))
	var point := _almanac.target_point("go:sect:%d" % id)
	assert_true(point.x >= 0.0, "宗門榜裡找得到宗門名")
	await click(point)
	await frames(2)
	assert_eq(flown.size(), 1, "發出飛往山門的要求")
	assert_eq(flown[0], (sects[0] as Dictionary)["pos"], "座標是山門所在")
	assert_false(_almanac.expanded, "飛往後收合")


func test_headers_resort_the_province_ranking() -> void:
	await _boot()
	await _open(0)
	assert_true(_almanac.left_targets().has("sort:aura"), "表頭可點")
	var first_before: int = ((_sim.world.stats_provinces("population", 1, true)["rows"] as Array)[0] as Dictionary)["id"]
	assert_true(_almanac.left_targets().has("go:province:%d" % first_before), "預設依人口排")
	_almanac.activate_target("sort:aura")
	var first_after: int = ((_sim.world.stats_provinces("aura", 1, true)["rows"] as Array)[0] as Dictionary)["id"]
	assert_true(_almanac.left_targets().has("go:province:%d" % first_after), "改依靈氣排後榜首換人")


func test_exclusive_with_rumor_cards_and_menu() -> void:
	await _boot()
	_sim.set_paused(false)
	_sim.set_speed(1)
	_rumor.expanded = true
	await frames(2)
	assert_true(_sim.paused, "傳聞全螢幕時暫停")
	await key(KEY_B)
	assert_true(_almanac.expanded, "傳聞開著按 B 換成天下冊")
	assert_false(_rumor.expanded, "傳聞收合")
	assert_true(_sim.paused, "換手期間仍暫停")
	await key(KEY_T)
	assert_true(_rumor.expanded, "天下冊開著按 T 換成傳聞")
	assert_false(_almanac.expanded, "天下冊收合")
	_rumor.expanded = false
	await frames(2)
	assert_false(_sim.paused, "全部收合後回到最初的運行")
	assert_eq(_sim.speed_index, 1, "速度回到最初")
	await _open()
	var people: Array = _sim.world.people_where("all", 0, 1)["rows"]
	_hud.open_person(int((people[0] as Dictionary)["id"]))
	await frames(2)
	assert_false(_almanac.expanded, "開命簿卡時天下冊收合")
	await _open()
	_hud.open_menu()
	await frames(2)
	assert_false(_almanac.expanded, "開選單時天下冊收合")
	_hud.close_menu()


func test_wheel_scrolls_only_the_column_under_the_cursor() -> void:
	await _boot(DECK_VIEW)
	await _open(1)
	var left := _almanac.left_list()
	assert_gt(left.max_scroll(), 0.0, "修士頁的左欄比畫面長")
	var point := left.global_position + left.size * 0.5
	await wheel(point, false)
	await wheel(point, false)
	assert_gt(left.scroll, 0.0, "滾輪捲動游標所在的欄")
	assert_eq(_almanac.right_list().scroll, 0.0, "另一欄不動")
	await wheel(point, true)
	await wheel(point, true)
	await wheel(point, true)
	assert_eq(left.scroll, 0.0, "往回捲到頂")


func _check_fit(view: Vector2i, scale: float) -> void:
	GameSettings.set_value("ui_scale", scale)
	await _boot(view)
	for index in 4:
		await _open(index)
		var left := _almanac.left_list()
		assert_true(left.content_height > 0.0, "%s 倍率 %s 第 %d 頁左欄有內容" % [view, scale, index])
		assert_true(_almanac.paper_rect().encloses(Rect2(left.position, left.size)), "%s 倍率 %s 兩欄落在紙面內" % [view, scale])
		assert_true(_almanac.paper_rect().encloses(Rect2(_almanac.right_list().position, _almanac.right_list().size)), "%s 倍率 %s 右欄落在紙面內" % [view, scale])


func test_fits_at_full_hd() -> void:
	await _check_fit(FLOW_VIEW, 1.0)


func test_fits_on_the_deck_at_larger_scale() -> void:
	await _check_fit(DECK_VIEW, 1.5)
