extends "res://tests/harness/suite.gd"
## GDD 6.4 橋接：SimWorld 各方法回傳的型別與尺寸，以及同種子的確定性。

const SEED := 20261011

var _world: SimWorld


func suite_name() -> String:
	return "sim_bridge"


func _small_world() -> SimWorld:
	if _world == null:
		_world = SimWorld.new()
		var error := _world.create(SEED, "small")
		assert_eq(error, "", "小尺寸世界應能生成")
	return _world


func test_gdd_6_4_unready_world_returns_empty_values() -> void:
	var world := SimWorld.new()
	assert_false(world.is_ready())
	assert_eq(world.grid_size(), Vector2i.ZERO)
	assert_eq(world.province_at(1.0, 1.0), -1)
	assert_true(world.terrain_mesh().is_empty())


func test_gdd_5_3_mesh_arrays_are_consistent() -> void:
	var mesh := _small_world().terrain_mesh()
	var count: int = (mesh["positions"] as PackedVector3Array).size()
	assert_gt(count, 0)
	assert_eq((mesh["uvs"] as PackedVector2Array).size(), count)
	assert_eq((mesh["indices"] as PackedInt32Array).size() % 3, 0)


func test_gdd_5_3_texture_sizes_follow_the_grid() -> void:
	var world := _small_world()
	var grid := world.grid_size()
	var textures := world.map_textures()
	var elevation_size: Vector2i = textures["elevation_size"]
	assert_eq((textures["elevation"] as PackedFloat32Array).size(), elevation_size.x * elevation_size.y)
	assert_eq((textures["lake"] as PackedByteArray).size(), elevation_size.x * elevation_size.y)
	var climate_size: Vector2i = textures["climate_size"]
	assert_eq((textures["climate"] as PackedByteArray).size(), climate_size.x * climate_size.y * 3)
	assert_eq((textures["cover"] as PackedByteArray).size(), climate_size.x * climate_size.y * 2)
	var aura_scale: int = textures["aura_scale"]
	assert_eq((textures["aura"] as PackedByteArray).size(), grid.x * aura_scale * grid.y * aura_scale)
	assert_eq((textures["province_id"] as PackedByteArray).size(), grid.x * grid.y * 2)
	assert_eq(world.province_colors().size(), 256 * 4)
	assert_eq(SimWorld.paper_texture().size(), 512 * 512 * 4)


func test_gdd_5_3_coast_follows_the_elevation_sign() -> void:
	var world := _small_world()
	var textures := world.map_textures()
	var elevation: PackedFloat32Array = textures["elevation"]
	var below := 0
	for value in elevation:
		if value < 0.0:
			below += 1
	var ratio := float(below) / elevation.size()
	assert_true(ratio > 0.3 and ratio < 0.95, "海面比例應合理：%s" % ratio)
	for id in world.province_count():
		var seat: Vector2 = world.province_info(id)["seat"]
		assert_true(world.terrain_height_at(seat.x, seat.y) >= 0.0)


func test_gdd_5_3_river_mesh_carries_side_and_level() -> void:
	var rivers := _small_world().river_mesh()
	var side_level: PackedVector2Array = rivers["side_level"]
	assert_gt(side_level.size(), 0, "世界應有河流")
	for entry in side_level:
		assert_true(absf(entry.x) == 1.0 and entry.y >= 1.0 and entry.y <= 3.0)


func test_gdd_2_3_province_seat_lies_in_its_own_province() -> void:
	var world := _small_world()
	assert_gt(world.province_count(), 0)
	for id in world.province_count():
		var seat: Vector2 = world.province_info(id)["seat"]
		assert_eq(world.province_at(seat.x, seat.y), id)


func test_gdd_5_3_labels_cover_every_province() -> void:
	var world := _small_world()
	var provinces := 0
	for label: Dictionary in world.map_labels():
		assert_false((label["text"] as String).is_empty())
		if label["kind"] == "province":
			provinces += 1
	assert_eq(provinces, world.province_count())


## 世界種子只決定天地；人物落地起的演變由作業系統亂數決定。測試用 `rng_seed` 固定亂數種子，才得到同一個世界。
func test_gdd_6_5_same_seed_and_rng_seed_give_same_hash() -> void:
	var other := SimWorld.new()
	other.create_with({"seed": SEED, "size": "small", "rng_seed": 7})
	var world := SimWorld.new()
	world.create_with({"seed": SEED, "size": "small", "rng_seed": 7})
	assert_eq(other.hash_hex(), world.hash_hex())
	world.step(12)
	other.step(12)
	assert_eq(other.hash_hex(), world.hash_hex(), "各推進 12 個月後仍相同")


func test_date_uses_the_era_form_with_arabic_numerals() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(20261011, "small"), "", "世界應能生成")
	world.step(22)
	# 世界生成後已預跑一段時間；月份由 tick 換算（tick 1 是 1月）。
	var months := (world.tick() - 1) / 30
	var date := world.date_short()
	var parts := date.split(" ")
	assert_eq(parts.size(), 3, "日期是「某某紀元 N年 M月」：%s" % date)
	assert_true(parts[0].ends_with("紀元") and parts[0].length() == 4, "紀元名是兩個字加「紀元」：%s" % parts[0])
	assert_true(parts[1].ends_with("年") and parts[1].trim_suffix("年").is_valid_int(), "年是阿拉伯數字：%s" % parts[1])
	assert_eq(parts[2], "%d月" % (months % 12 + 1))
	# 開天就是第 0 年，沒有開局前的歷史；題簽的「開天以來第 N 年」等於預跑年數加上之後的年數。
	var history := world.history_years()
	assert_eq(history, 0, "開天沒有開局前的歷史")
	assert_eq(world.date_text(), "%s（開天以來第 %d 年）" % [date, history + months / 12 + 1])


func test_gdd_5_3_range_label_paths_stay_on_land() -> void:
	var world := _small_world()
	var ranges := 0
	var points := 0
	var on_land := 0
	for label: Dictionary in world.map_labels():
		if label["kind"] != "range":
			continue
		ranges += 1
		for point: Vector2 in label["path"]:
			points += 1
			if world.terrain_height_at(point.x, point.y) > 0.0:
				on_land += 1
	assert_gt(ranges, 0, "測試世界應有山脈")
	assert_gt(points, 0)
	assert_eq(on_land, points, "山脈名沿山勢排，路徑上的每一點都在陸地上")


func test_gdd_5_3_province_names_sit_inside_their_province() -> void:
	var world := _small_world()
	var checked := 0
	var away_from_seat := 0
	var id := 0
	for label: Dictionary in world.map_labels():
		if label["kind"] != "province":
			continue
		var pos: Vector2 = label["pos"]
		assert_eq(world.province_at(pos.x, pos.y), id, "「%s」的名字應落在自己的州郡內" % label["text"])
		if pos.distance_to(world.province_info(id)["seat"]) > 1.0:
			away_from_seat += 1
		checked += 1
		id += 1
	assert_eq(checked, world.province_count())
	assert_gt(away_from_seat, checked / 2, "名字放在州郡中心，多數不會剛好在治所")


func test_gdd_7_3_seat_names_are_their_own_and_may_borrow_one_character() -> void:
	var world := _small_world()
	var suffixes := ["村", "鎮", "縣", "城", "都"]
	var seen := {}
	var borrowed := 0
	var count := world.province_count()
	for id in count:
		var info := world.province_info(id)
		var seat_name: String = info["seat_name"]
		var province_stem: String = (info["name"] as String).left(2)
		assert_eq(seat_name.length(), 3, "治所名是兩個字加城池等級的字：%s" % seat_name)
		assert_eq(seat_name.right(1), suffixes[int(info["city_level"])])
		var stem := seat_name.left(2)
		assert_ne(stem, province_stem, "治所名不與州郡名相同")
		assert_false(seen.has(stem), "治所不重名：%s" % stem)
		seen[stem] = true
		if province_stem.contains(stem[0]) or province_stem.contains(stem[1]):
			borrowed += 1
	assert_gt(borrowed, count / 6, "有一部分治所借了州郡名的一個字（%d／%d）" % [borrowed, count])
	assert_lt(borrowed, count * 3 / 4, "不是每個治所都借州郡名的字（%d／%d）" % [borrowed, count])
	var again := SimWorld.new()
	assert_eq(again.create(SEED, "small"), "")
	assert_eq(again.province_info(0)["seat_name"], world.province_info(0)["seat_name"], "同種子得到同樣的治所名")


func test_gdd_7_3_seas_and_lakes_are_named() -> void:
	var world := _small_world()
	var names := {}
	var seas := 0
	var oceans := 0
	for label: Dictionary in world.map_labels():
		var kind: String = label["kind"]
		if not kind in ["ocean", "sea", "lake"]:
			continue
		var text: String = label["text"]
		var pos: Vector2 = label["pos"]
		assert_false(names.has(text), "水域不重名：%s" % text)
		names[text] = true
		assert_eq(world.province_at(pos.x, pos.y), -1, "「%s」標在水上，不在任何州郡內" % text)
		match kind:
			"ocean":
				oceans += 1
				assert_true(text.ends_with("洋"), text)
			"sea":
				seas += 1
				assert_true(text.ends_with("海"), text)
			_:
				assert_true(text.right(1) in ["澤", "湖", "池"], text)
	assert_gt(oceans + seas, 1, "海分成不只一個海區")
	assert_gt(oceans, 0, "至少有一個洋")
	var again := SimWorld.new()
	assert_eq(again.create(SEED, "small"), "")
	var first := ""
	for label: Dictionary in again.map_labels():
		if label["kind"] in ["ocean", "sea", "lake"]:
			first = label["text"]
			break
	assert_true(names.has(first), "同種子得到同樣的水域名")


func test_gdd_4_6_create_with_applies_and_clamps_options() -> void:
	var world := SimWorld.new()
	var error := world.create_with({
		"seed": SEED, "name": "  試煉界  ", "size": "small", "era": "decline",
		"land_ratio": 0.9, "relief": 0.25, "humidity": 0.1,
	})
	assert_eq(error, "")
	var options := world.world_options()
	assert_eq(options["name"], "試煉界")
	assert_eq(options["size"], "small")
	assert_eq(options["era"], "decline")
	assert_true(absf(options["land_ratio"] - 0.55) < 0.001, "陸地比例夾在上限")
	assert_true(absf(options["relief"] - 0.25) < 0.001)
	assert_true(absf(options["humidity"] - 0.6) < 0.001, "濕潤度夾在下限")


func test_gdd_4_6_blank_name_takes_the_seed_name() -> void:
	var options := _small_world().world_options()
	assert_ne(options["name"], "")
	assert_eq(options["name"], SimWorld.random_world_name(options["seed"]))
	assert_eq(SimWorld.random_world_name(7), SimWorld.random_world_name(7))


func test_gdd_4_8_save_text_loads_back_in_steps_to_the_same_date() -> void:
	var source := SimWorld.new()
	assert_eq(source.create(SEED, "small"), "")
	source.step(30)
	var text := source.save_text()
	assert_true(text.length() > 0)

	var loaded := SimWorld.new()
	assert_eq(loaded.load_begin(text), "")
	assert_false(loaded.is_ready(), "讀完之前還沒有世界")
	var rounds := 0
	var last := 0.0
	while true:
		var result := loaded.load_step(8)
		assert_eq(result["error"], "")
		assert_true(result["progress"] >= last)
		last = result["progress"]
		rounds += 1
		if result["done"] or rounds > 100:
			break
	assert_gt(rounds, 2, "30 個月每次 8 個，分好幾段")
	assert_true(loaded.is_ready())
	assert_eq(loaded.tick(), source.tick())
	assert_eq(loaded.date_text(), source.date_text())
	assert_eq(loaded.save_text(), text, "讀回來再存，內容相同")


func test_gdd_4_8_bad_save_text_reports_an_error() -> void:
	var world := SimWorld.new()
	assert_ne(world.load_begin("{"), "")
	assert_ne(world.load_step(8)["error"], "")
	var text := _small_world().save_text()
	var hash_field := RegEx.create_from_string('"hash":[0-9]+')
	assert_eq(world.load_begin(hash_field.sub(text, '"hash":1')), "", "雜湊到重播完才比對")
	var result := world.load_step(100000)
	assert_true(result["done"])
	assert_ne(result["error"], "", "雜湊被改過的存檔讀不進來")
	assert_false(world.is_ready())



## 推進到水陸變了、背景開始重算為止；回傳過了幾世（40 世內沒變就回傳 -1）。
func _step_until_rebake(world: SimWorld) -> int:
	for shi in range(1, 41):
		world.step(360)
		if world.terrain_pending():
			return shi
	return -1


func test_gdd_2_8_terrain_is_rebaked_off_the_main_thread_after_the_land_changes() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	var before := world.map_textures()
	var terrain := world.terrain_revision()
	var shown := world.map_revision()
	assert_false(world.terrain_pending(), "剛開天不必重算")
	assert_gt(_step_until_rebake(world), 0, "40 世內水陸應有變動")
	assert_eq(world.terrain_revision(), terrain, "算好之前用的還是舊的高度場")
	assert_eq(world.map_textures()["climate"], before["climate"], "算好之前貼圖不變")
	world.flush_terrain()
	assert_false(world.terrain_pending())
	assert_eq(world.terrain_revision(), terrain + 1)
	assert_gt(world.map_revision(), shown)
	var after := world.map_textures()
	assert_eq(after["elevation_size"], before["elevation_size"])
	var changed := false
	for key: String in ["elevation", "lake", "climate", "cover"]:
		changed = changed or after[key] != before[key]
	assert_true(changed, "水陸變了，貼圖應跟著變")


func test_gdd_2_8_rebaked_terrain_equals_the_one_built_on_load() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	assert_gt(_step_until_rebake(world), 0)
	world.flush_terrain()
	var loaded := SimWorld.new()
	assert_eq(loaded.load_begin(world.save_text()), "")
	while not loaded.load_step(3600)["done"]:
		pass
	assert_true(loaded.is_ready())
	var here := world.map_textures()
	var there := loaded.map_textures()
	for key: String in ["elevation", "lake", "climate", "cover", "province_id"]:
		assert_true(here[key] == there[key], "%s 應與讀檔後重建的相同" % key)
	assert_true(world.river_mesh()["positions"] == loaded.river_mesh()["positions"], "河流網格應與讀檔後重建的相同")
	# 水域名以生成時的水域為準沿用（GDD 7.3），一路玩下來與讀檔重播得到同樣的名字與位置。
	var names_here := _label_lines(world)
	assert_gt(names_here.size(), 0)
	assert_eq(names_here, _label_lines(loaded), "地圖標籤應與讀檔後的相同")


func test_gdd_2_8_climate_reports_where_the_world_is_and_where_it_is_heading() -> void:
	var world := SimWorld.new()
	world.create(20261011, "small")
	for shi in 12:
		var before: Dictionary = world.climate()
		assert_true(absf(float(before["warmth"])) <= 1.0)
		assert_true(float(before["tide"]) >= float(before["tide_min"]) and float(before["tide"]) <= float(before["tide_max"]))
		assert_true(before["climate"] in ["Cold", "Mild", "Warm"])
		assert_true(before["aura"] in ["Decline", "Plain", "Revival"])
		world.step(360 - world.tick() / 30 % 360)
		var after: Dictionary = world.climate()
		assert_eq(float(before["warmth_next"]), float(after["warmth"]), "預告的寒暖應等於下一世結算後的寒暖")
		assert_eq(float(before["tide_next"]), float(after["tide"]), "預告的靈潮應等於下一世結算後的靈潮")
		world.step(7)


func test_gdd_2_8_world_changes_come_out_of_the_event_queue_with_their_era() -> void:
	var world := SimWorld.new()
	world.create(20261011, "small")
	world.drain_events()
	var changes: Array[Dictionary] = []
	for shi in 60:
		world.step(360)
		for event: Dictionary in world.drain_events():
			if event["kind"] == "WorldChanged":
				changes.append(event)
	assert_gt(changes.size(), 0, "六十世裡應有世界大事")
	var opened := 0
	for event in changes:
		assert_true(event["change"] in ["ColdBegan", "ColdEnded", "WarmBegan", "WarmEnded", "DeclineBegan", "DeclineEnded", "RevivalBegan", "RevivalEnded", "CataclysmBegan", "CataclysmEnded"])
		if event["era"] != "":
			opened += 1
	assert_gt(opened, 0, "六十世裡應有改元")
	assert_true(world.date_short().begins_with(changes.filter(func(e: Dictionary) -> bool: return e["era"] != "")[-1]["era"]), "題簽的紀元應是最後一次改元的名字")


func _label_lines(world: SimWorld) -> Array[String]:
	var lines: Array[String] = []
	for label: Dictionary in world.map_labels():
		lines.append("%s %s %s" % [label["kind"], label["text"], label["pos"]])
	return lines


func test_gdd_2_8_every_settlement_gives_the_map_a_new_batch() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	var shown := world.map_revision()
	# 開天時預跑過一段，離下一次結算不一定是整整一世。
	world.step(359 - world.tick() / 30 % 360)
	assert_false(world.map_pending(), "一世結算之前不重算")
	world.step(1)
	assert_true(world.map_pending(), "結算後在背景重算靈氣與州界")
	assert_eq(world.map_revision(), shown, "算好之前不換")
	assert_false(world.poll_terrain() and world.map_pending())
	world.flush_terrain()
	assert_false(world.map_pending())
	assert_eq(world.map_revision(), shown + 1)


func test_gdd_5_3_elevation_comes_as_half_floats_with_mipmaps() -> void:
	var world := _small_world()
	var textures := world.relief_textures()
	var size: Vector2i = textures["elevation_size"]
	var image := Image.create_from_data(size.x, size.y, true, Image.FORMAT_RH, textures["elevation_half"])
	assert_true(image != null and image.has_mipmaps(), "位元組數應符合整串多級縮圖")
	var raw: PackedFloat32Array = world.map_textures()["elevation"]
	var theirs := Image.create_from_data(size.x, size.y, false, Image.FORMAT_RF, raw.to_byte_array())
	theirs.convert(Image.FORMAT_RH)
	theirs.generate_mipmaps()
	assert_eq(image.get_data().size(), theirs.get_data().size(), "級數與引擎自己產生的相同")
	var worst := 0.0
	for i in 400:
		var x := (i * 7919) % size.x
		var y := (i * 104729) % size.y
		worst = maxf(worst, absf(image.get_pixel(x, y).r - raw[y * size.x + x]))
	assert_lt(worst, 0.001, "半精度的誤差")


func test_gdd_6_4_events_carry_what_the_templates_need() -> void:
	var world := SimWorld.new()
	world.create(SEED, "small")
	world.drain_events()
	var kinds := {}
	var located := 0
	var named := 0
	var seen_tribulation := 0
	for step in 40:
		world.step(120)
		for event: Dictionary in world.drain_events():
			for key in ["id", "tick", "kind", "importance", "province", "cell", "souls", "a", "b", "f", "g", "p", "q", "x", "n", "m", "year", "month", "persons", "powers", "a_power", "a_role", "a_realm"]:
				assert_true(event.has(key), "事件帶有「%s」" % key)
			assert_true(event["souls"] is PackedInt32Array)
			assert_true(event["persons"] is PackedInt32Array and event["powers"] is PackedInt32Array, "人物與勢力編號")
			assert_eq(event["month"], (maxi(event["tick"], 1) - 1) / 30 % 12 + 1, "月份與曆法（GDD 2.6）一致")
			assert_eq(event["day"], (maxi(event["tick"], 1) - 1) % 30 + 1, "日與曆法一致")
			assert_true(event["why"] is String and event["rel"] is PackedStringArray, "事件帶成因與關係")
			assert_true(event["a_realm"] >= -1 and event["a_realm"] <= 8, "境界在表內，非修士為 −1")
			assert_true(event["a_role"] in ["", "Ruler", "Leader", "Member", "Rogue", "Monk", "Scholar", "Demonic", "Beast", "Official", "General", "Royal"], "職分代號：" + str(event["a_role"]))
			if event["a_role"] == "Leader" or event["a_role"] == "Member":
				assert_ne(event["a_power"], "", "宗門成員帶門派名")
			if event["kind"] == "Tribulation":
				seen_tribulation += 1
				assert_true(event["a"] != "" and event["a_realm"] >= 0, "渡劫的是修士，帶境界")
			assert_gt(event["year"], 0, "年份從 1 起算")
			if event.has("pos"):
				located += 1
				var pos: Vector2 = event["pos"]
				assert_true(pos.x >= 0.0 and pos.y >= 0.0, "地點在地圖內")
				assert_true(event["province"] >= 0 or event["cell"] >= 0, "有地點的事件有州郡或格")
			if event["cell"] >= 0:
				var grid := world.grid_size()
				assert_eq(event["pos"], Vector2(event["cell"] % grid.x + 0.5, event["cell"] / grid.x + 0.5), "有格的事件地點是格中心")
			if event["a"] != "":
				named += 1
			kinds[event["kind"]] = true
			var line := EventText.line(event)
			assert_false(line.contains("{") or line.begins_with("event_") or line == Locale.t("event_unknown"), "「%s」有對應的模板：%s" % [event["kind"], line])
			assert_eq(EventText.seal(event["kind"]).length(), 2, "印是二字詞")
			assert_false(String(event["date_era"]).is_empty(), "事件帶當時的紀元名")
	assert_gt(kinds.size(), 8, "四百年裡出現多種事件")
	assert_gt(located, 0, "有地點的事件帶 pos")
	assert_gt(named, 0, "人物事件帶人名")
	assert_gt(seen_tribulation, 0, "四百年裡有渡劫")


## 快轉一次推進多個月，事件的主角身分（門派、職分、境界）仍是事件當月月底的樣子，與逐月推進、逐月取走的結果一致。
func test_gdd_6_4_batched_steps_keep_the_identity_at_event_time() -> void:
	var single := SimWorld.new()
	single.create_with({"seed": SEED, "size": "small", "rng_seed": 7})
	single.drain_events()
	var batched := SimWorld.new()
	batched.create_with({"seed": SEED, "size": "small", "rng_seed": 7})
	batched.drain_events()
	var expected := {}
	for month in 360:
		single.step(1)
		for event: Dictionary in single.drain_events():
			expected[event["id"]] = [event["a_power"], event["a_role"], event["a_realm"]]
	var checked := 0
	var leaders := 0
	for batch in 6:
		batched.step(60)
		for event: Dictionary in batched.drain_events():
			assert_true(expected.has(event["id"]), "兩邊的事件編號相同")
			var want: Array = expected[event["id"]]
			assert_eq([event["a_power"], event["a_role"], event["a_realm"]], want, "事件 %d（%s）的身分不因快轉而變" % [event["id"], event["kind"]])
			checked += 1
			if event["a_role"] == "Leader":
				leaders += 1
	assert_gt(checked, 50, "三十年裡有足夠的事件可比")
	assert_gt(leaders, 0, "其中有掌門")


func test_gdd_7_2_event_text_fills_each_template() -> void:
	var base := {"id": 1, "tick": 10, "importance": 3, "province": 0, "cell": -1, "souls": PackedInt32Array(), "a": "沈青霜", "b": "韓烈", "f": "玄清宗", "g": "赤霄門", "p": "青州", "q": "雲州", "x": "", "n": 0, "m": 0, "year": 1}
	var cases := [
		["Died", {"x": "Age", "n": 83}, "沈青霜壽終，享年83"],
		["Breakthrough", {"n": 2}, "沈青霜（築基）突破至金丹"],
		["Tribulation", {"n": 3, "x": "Passed"}, "沈青霜（金丹）渡四九雷劫，成就元嬰"],
		["Tribulation", {"n": 3, "x": "Perished"}, "沈青霜（金丹）渡四九雷劫未成元嬰，形神俱滅"],
		["ThreadCreated", {"x": "Feud"}, "沈青霜與韓烈結下殺業"],
		["ThreadResolved", {"x": "Love", "n": 3}, "歷經3次轉世，沈青霜與韓烈的情緣了結"],
		["ThreadCreated", {"x": "Mentor"}, "沈青霜拜韓烈為師"],
		["ThreadOmen", {"x": "Feud", "n": 0}, "沈青霜與韓烈的殺業將有分曉"],
		["ThreadOmen", {"x": "Feud", "n": 1}, "沈青霜與韓烈前世的殺業將有分曉"],
		["FeudEscalated", {"x": "Feud", "n": 0}, "沈青霜再殺韓烈，冤冤相報"],
		["FeudEscalated", {"x": "Feud", "n": 2}, "沈青霜再殺韓烈，延續前世的冤仇，冤冤相報"],
		["ThreadFaded", {"x": "Grace", "n": 0}, "沈青霜與韓烈有緣無分"],
		["ThreadFaded", {"x": "Grace", "n": 1}, "沈青霜與韓烈前世的恩義消散，有緣無分"],
		["ThreadResolved", {"x": "Love", "n": 0}, "沈青霜與韓烈的情緣了結"],
		["ThreadResolved", {"x": "Love", "n": 1}, "歷經1次轉世，沈青霜與韓烈的情緣了結"],
		["ComeOfAge", {}, "沈青霜入玄清宗"],
		["ComeOfAge", {"f": "", "x": "Cultivator"}, "沈青霜自行入道，為散修"],
		["ComeOfAge", {"f": "", "x": "Commoner"}, "沈青霜年滿十二，未得入道"],
		["ComeOfAge", {"f": "", "x": "Monk"}, "沈青霜自行出家"],
		["Judged", {"x": "Human"}, "沈青霜之魂判入人道"],
		["Traveled", {"x": "Out"}, "沈青霜出外遊歷雲州"],
		["Traveled", {"x": "Back"}, "沈青霜遊歷歸來"],
		["SectSplit", {}, "玄清宗內亂，沈青霜率眾另立赤霄門"],
		["SectWar", {"x": "太虛洞天"}, "玄清宗與赤霄門爭奪太虛洞天，玄清宗勝"],
		["NodeShifted", {"x": "太虛洞天", "n": 0, "m": 2}, "太虛洞天靈氣枯竭，就此沉寂"],
		["NodeShifted", {"x": "太虛洞天", "n": 2, "m": 0}, "太虛洞天靈氣回復，升為洞天"],
		["SeatMoved", {"x": "臨海鎮"}, "青州治所為海所淹，遷治"],
		["ProvinceLost", {}, "青州沒入海中，遺民徙往雲州"],
		["ProvinceLost", {"q": ""}, "青州沒入海中"],
		["ProvinceLost", {"n": 12}, "青州沒入海中，遺民徙往雲州，溺斃12人"],
		["LandLinked", {"x": "Joined"}, "海退，青州與雲州陸路相通"],
		["LandLinked", {"x": "Parted"}, "海進，青州與雲州陸路斷絕"],
		["BookEntry", {"x": "ThreeLivesFeud"}, "天書添一頁：三世血仇"],
	]
	for c in cases:
		var event := base.duplicate()
		event["kind"] = c[0]
		event.merge(c[1], true)
		assert_eq(EventText.line(event), c[2], "「%s」的一句話" % c[0])


func test_history_lines_read_from_the_owners_side() -> void:
	var mentor := {"kind": "ThreadCreated", "x": "Mentor", "a": "徒甲", "b": "師乙", "a_person": 5, "b_person": 9, "n": 0, "importance": 1}
	assert_eq(EventText.history_line(mentor, 5), "拜師乙為師", "本人是徒")
	assert_eq(EventText.history_line(mentor, 9), "收徒甲為徒", "本人是師")
	assert_eq(EventText.line(mentor), "徒甲拜師乙為師", "傳聞與事件卡")
	var judged := {"kind": "Judged", "x": "Human", "a": "前世甲", "a_person": 5}
	assert_eq(EventText.history_line(judged, 5), "死後魂入人道")
	for role in ["Child", "Cultivator", "Monk", "Scholar", "Official", "General", "GeneralCandidate", "Ruler", "Royal", "Demonic", "Beast", "Commoner"]:
		assert_true(Locale.has("role_" + role), "職分「%s」有詞" % role)


func test_gdd_5_5_sects_report_seat_province_and_range() -> void:
	var world := SimWorld.new()
	assert_eq(world.sects().size(), 0, "沒有世界時宗門清單為空")
	assert_eq(world.faith_colors().size(), 0)
	world.create(SEED, "small")
	var sects := world.sects()
	assert_gt(sects.size(), 0, "開天後世界應有宗門")
	var staffed := 0
	for sect: Dictionary in sects:
		for key in ["id", "name", "pos", "province", "node", "members", "prestige", "radius", "range", "range_radius", "evil", "alive"]:
			assert_true(sect.has(key), "宗門欄位 %s" % key)
		var pos: Vector2 = sect["pos"]
		assert_eq(world.province_at(pos.x, pos.y), sect["province"], "山門的位置落在山門所在州郡")
		assert_true((sect["range"] as PackedInt32Array).has(sect["province"]), "勢力範圍含山門所在州郡")
		assert_gt(sect["range_radius"], 0.0)
		# 補人只從當地既有的人裡挑，當地沒人的宗門可以暫時沒有成員。
		if sect["alive"] and sect["members"] > 0:
			staffed += 1
	assert_gt(staffed, 0, "還在的宗門裡有成員的至少一個")
	assert_eq(world.sects().size(), sects.size(), "只讀：連呼兩次結果相同")


func test_gdd_5_4_faith_colors_tint_only_provinces_led_by_tao() -> void:
	var world := _small_world()
	var table := world.faith_colors()
	assert_eq(table.size(), 256 * 4)
	var tinted := 0
	for id in world.province_count():
		var faith: Vector3 = world.province_info(id)["faith"]
		var alpha := table[id * 4 + 3]
		var led_by_tao := faith.x > 0.0 and faith.x >= faith.y and faith.x >= faith.z
		assert_eq(alpha > 0, led_by_tao, "州郡 %d：道為主教化才上色" % id)
		tinted += 1 if alpha > 0 else 0
	assert_true(tinted >= 0)


func test_gdd_3_5_sect_list_follows_the_world_over_time() -> void:
	var world := SimWorld.new()
	world.create(SEED, "small")
	var before := world.sects()
	world.step(12 * 60)
	var after := world.sects()
	assert_true(after.size() >= before.size(), "清單只增不減（覆滅者以 alive 標記）")


## 走遍所有人物，回傳 [在世歷程最長者, 已歿歷程最長者] 的編號。
func _richest_people(world: SimWorld, died_after := -1) -> Array[int]:
	var best_alive := -1
	var best_dead := -1
	var most_alive := -1
	var most_dead := -1
	var id := 0
	while not world.person(id).is_empty():
		var count := world.person_history(id).size()
		if world.person(id)["alive"]:
			if count > most_alive:
				most_alive = count
				best_alive = id
		elif count > most_dead and int(world.person(id)["died_tick"]) > died_after:
			most_dead = count
			best_dead = id
		id += 1
	return [best_alive, best_dead]


func test_gdd_5_8_person_card_fields() -> void:
	var world := SimWorld.new()
	world.create(SEED, "small")
	world.step(12 * 40)
	assert_true(world.person(-1).is_empty(), "沒有的編號給空字典")
	assert_true(world.person(99999999).is_empty(), "超出範圍給空字典")
	assert_eq(world.person_history(99999999).size(), 0)
	var seen := 0
	for province in world.province_count():
		for row: Dictionary in world.people_in(province):
			for key in ["id", "name", "role", "realm", "fruit", "power_name", "is_leader", "age"]:
				assert_true(row.has(key), "在此的有名之人帶有「%s」" % key)
			var card := world.person(row["id"])
			assert_eq(card["name"], row["name"])
			assert_true(card["alive"], "人名單裡都在世")
			assert_eq(card["province"], province, "所在州郡與人名單相符")
			seen += 1
			if seen == 1:
				for key in ["name", "title", "gender", "age", "lifespan", "alive", "died_tick", "death_cause", "province", "province_name", "home", "home_name", "pos", "role", "power", "power_name", "is_leader", "realm", "fruit", "root", "insight", "talent", "virtue", "ferocity", "soul", "life_index", "lives_total", "merit", "sin", "wisdom"]:
					assert_true(card.has(key), "人物卡帶有「%s」" % key)
				assert_true(card["gender"] in ["Male", "Female"])
				assert_gt(card["life_index"], 0, "第幾世從 1 起算")
				assert_true(card["life_index"] <= card["lives_total"])
				assert_ne(card["province_name"], "", "有所在州郡名")
	assert_gt(seen, 20, "四十年後各州郡有不少在世的有名之人")
	var sorted_ok := true
	for province in world.province_count():
		var last := 99
		for row: Dictionary in world.people_in(province):
			if row["realm"] > last:
				sorted_ok = false
			last = row["realm"]
	assert_true(sorted_ok, "人名單依境界由高到低")


func test_gdd_5_8_person_history_is_ordered_and_matches_the_ledger() -> void:
	var world := SimWorld.new()
	world.create_with({"seed": SEED, "size": "small", "rng_seed": 7})
	world.drain_events()
	var since := world.tick()
	var stream := {}
	for step in 40:
		world.step(12)
		for event: Dictionary in world.drain_events():
			stream[event["id"]] = event
	# 已歿者只挑事件流開始之後才身故的，預跑期間身故的人沒有身故事件可對。
	var picks := _richest_people(world, since)
	assert_gt(picks[0], -1, "有在世者")
	assert_gt(picks[1], -1, "四十年後有已歿者")
	for id in picks:
		var history := world.person_history(id)
		assert_gt(history.size(), 4, "歷程最長的人至少有 5 筆")
		var last_tick := -1
		var last_id := -1
		for entry: Dictionary in history:
			for key in ["id", "tick", "kind", "importance", "province", "era", "year", "month", "a", "b", "f", "g", "p", "q", "x", "n", "m", "a_person", "b_person", "q_province"]:
				assert_true(entry.has(key), "歷程帶有「%s」" % key)
			assert_true(entry["tick"] >= last_tick, "由舊到新")
			if entry["tick"] == last_tick:
				assert_gt(entry["id"], last_id, "同一刻依事件編號")
			last_tick = entry["tick"]
			last_id = entry["id"]
			assert_true(entry["month"] >= 1 and entry["month"] <= 12)
			assert_gt(entry["year"], 0)
			assert_gt(entry["importance"], 0, "墓碑不入歷程")
			assert_ne(EventText.line(entry), "event_unknown", "每一筆都有模板：" + entry["kind"])
			# 預跑期間的事件不在事件流裡；其餘必須與事件流同一筆。
			if stream.has(entry["id"]):
				for key in ["tick", "kind", "a", "b", "f", "g", "p", "q", "x", "n", "m", "year"]:
					# 遊歷的「出門或歸來」事件流在當下以故里判斷；歷程事後以行程順序判斷，兩者可以不同，不比。
					if key == "x" and entry["kind"] == "Traveled":
						continue
					var streamed: Dictionary = stream[entry["id"]]
					assert_eq(entry[key], streamed[key], "與事件流同一筆的「%s」" % key)
	var dead_id: int = picks[1]
	var dead := world.person(dead_id)
	assert_false(dead["alive"])
	assert_ne(dead["death_cause"], "", "已歿者有死因")
	for entry: Dictionary in world.person_history(dead_id):
		# 魂判入六道是死後的事，仍屬這一世的最後一條。
		assert_true(entry["tick"] <= dead["died_tick"] or entry["kind"] == "Judged", "歷程到死為止（死後只有魂的歸處）")
	var in_history := {}
	for entry: Dictionary in world.person_history(dead_id):
		in_history[entry["id"]] = true
	var found := 0
	for event_id: int in stream:
		var event: Dictionary = stream[event_id]
		if event["kind"] == "Died" and event["a"] == dead["name"] and event["tick"] == dead["died_tick"]:
			assert_true(in_history.has(event_id), "身故事件在歷程裡")
			assert_eq(event["n"], dead["age"], "享年與身故事件相符")
			found += 1
	assert_gt(found, 0, "事件流裡找到這位已歿者的身故事件")



## 魂的各世（依出生順序，兩世以上者）：魂編號 → 人物編號陣列。
func _lives_by_soul(world: SimWorld) -> Dictionary:
	var by_soul := {}
	var id := 0
	while not world.person(id).is_empty():
		var soul: int = world.person(id)["soul"]
		if not by_soul.has(soul):
			by_soul[soul] = []
		by_soul[soul].append(id)
		id += 1
	for soul: int in by_soul.keys():
		if (by_soul[soul] as Array).size() < 2:
			by_soul.erase(soul)
	return by_soul


func test_person_history_judged_closes_the_previous_life_and_the_next_starts_at_rebirth() -> void:
	var world := SimWorld.new()
	world.create(SEED, "small")
	world.step(12 * 80)
	var by_soul := _lives_by_soul(world)
	assert_gt(by_soul.size(), 0, "八十年後有轉世過的魂")
	var closed := 0
	for soul: int in by_soul:
		var lives: Array = by_soul[soul]
		for i in lives.size():
			var history := world.person_history(lives[i])
			var judged := history.filter(func(h: Dictionary) -> bool: return h["kind"] == "Judged")
			if i > 0:
				assert_ne(history[0]["kind"] if not history.is_empty() else "", "Judged", "新一世的歷程不以上一世的判入起頭")
			if i < lives.size() - 1 and not judged.is_empty():
				assert_eq(history[-1]["kind"], "Judged", "判入六道是上一世歷程的最後一條")
				assert_eq(judged[0]["a_person"], lives[i], "判入那筆的人名是上一世")
				closed += 1
			if i == lives.size() - 1:
				for h: Dictionary in judged:
					assert_true(h["tick"] >= world.person(lives[i])["died_tick"], "這一世的判入發生在這一世之後")
	assert_gt(closed, 0, "有上一世以判入六道收尾")


func test_event_names_follow_the_life_at_the_time_of_the_event() -> void:
	var stream := _stream_after(100)
	var world: SimWorld = stream["world"]
	stream.erase("world")
	var past := 0
	for id: int in stream:
		var event: Dictionary = stream[id]
		if not (event["kind"] in ["ThreadCreated", "ThreadOmen", "ThreadResolved", "ThreadFaded", "FeudEscalated", "Judged"]):
			continue
		for pair in [["a", "a_person", 0], ["b", "b_person", 1]]:
			var person_id: int = event[pair[1]]
			if person_id < 0:
				continue
			var card := world.person(person_id)
			assert_eq(event[pair[0]], card["name"], "事件%d的「%s」是 a_person／b_person 那一世的名字" % [id, pair[0]])
			if (event["souls"] as Array).size() > pair[2]:
				assert_eq(card["soul"], event["souls"][pair[2]], "那一世屬於事件裡的那個魂")
			if card["life_index"] < card["lives_total"]:
				past += 1
	assert_gt(past, 0, "有事件寫的是已轉世者當時的那一世")


## 把世界推進 years 年，回傳這段期間的事件流（依編號），另以 "world" 鍵帶回世界。
func _stream_after(years: int) -> Dictionary:
	var world := SimWorld.new()
	world.create(SEED, "small")
	world.drain_events()
	var stream := {}
	for month in years * 12:
		world.step(1)
		for event: Dictionary in world.drain_events():
			stream[event["id"]] = event
	stream["world"] = world
	return stream


func test_gdd_5_8_event_matches_the_stream_and_missing_is_empty() -> void:
	var stream := _stream_after(20)
	var world: SimWorld = stream["world"]
	stream.erase("world")
	assert_true(world.event(-1).is_empty(), "負的編號給空字典")
	assert_true(world.event(99999999).is_empty(), "超出範圍給空字典")
	assert_true(world.event_causes(99999999, 6).is_empty())
	assert_eq(world.event_ripple(99999999), 0)
	assert_true(world.event_ripples(99999999).is_empty())
	var checked := 0
	for id: int in stream:
		var streamed: Dictionary = stream[id]
		var got := world.event(id)
		assert_false(got.is_empty(), "事件 %d 取得到" % id)
		for key in ["id", "tick", "kind", "importance", "province", "a", "b", "f", "g", "p", "q", "x", "n", "m", "year", "month", "a_person", "b_person", "q_province", "persons", "powers", "souls"]:
			# 遊歷的「出門或歸來」事件流在當下以故里判斷，事後取得時故里可能已變，不比。
			if key == "x" and streamed["kind"] == "Traveled":
				continue
			assert_eq(got[key], streamed[key], "event(%d) 的「%s」與事件流相同" % [id, key])
		for key in ["a_power", "a_role", "a_realm"]:
			assert_true(got.has(key), "帶有身分欄「%s」" % key)
		assert_eq(got.has("pos"), streamed.has("pos"))
		checked += 1
		if checked >= 400:
			break
	assert_gt(checked, 50)


func test_gdd_5_8_event_causes_tree_and_ripple_agree() -> void:
	var stream := _stream_after(30)
	var world: SimWorld = stream["world"]
	stream.erase("world")
	var deepest := 0
	var with_ripple := 0
	var capped_ok := true
	for id: int in stream:
		if stream[id]["importance"] < 2:
			continue
		var tree := world.event_causes(id, 6)
		for i in tree.size():
			var node: Dictionary = tree[i]
			assert_true(node["id"] < id, "因在果之前")
			assert_true(node["level"] >= 1 and node["level"] <= 6)
			if node["level"] == 1:
				assert_eq(node["parent"], -1)
			else:
				assert_true(node["parent"] >= 0 and node["parent"] < i, "父節點在前")
				assert_eq(tree[node["parent"]]["level"], node["level"] - 1)
			assert_false(node["kind"].is_empty())
			deepest = maxi(deepest, node["level"])
		assert_true(world.event_causes(id, 1).size() <= 4, "一層至多 4 筆")
		for node: Dictionary in world.event_causes(id, 1):
			assert_eq(node["level"], 1)
		assert_eq(world.event_causes(id, 0).size(), 0, "深度 0 沒有因")
		var count := world.event_ripple(id)
		var list := world.event_ripples(id)
		assert_eq(list.size(), count, "漣漪數與清單同數")
		capped_ok = capped_ok and count <= 300
		if count > 0:
			with_ripple += 1
			assert_true(list[0]["id"] > id, "後果在事件之後")
			assert_true(list[-1]["tick"] >= list[0]["tick"], "由舊到新")
	assert_true(capped_ok, "漣漪最多 300")
	assert_gt(deepest, 1, "30 年內有兩層以上的因樹")
	assert_gt(with_ripple, 0, "有些事件有後果")


func test_gdd_5_8_person_last_event_points_at_a_live_event() -> void:
	var world := SimWorld.new()
	world.create(SEED, "small")
	world.step(12 * 20)
	var found := 0
	var id := 0
	while not world.person(id).is_empty():
		var last: int = world.person(id)["last_event"]
		if last >= 0:
			assert_false(world.event(last).is_empty(), "最近事件取得到")
			found += 1
		id += 1
	assert_gt(found, 0, "有人有最近事件")


## 走一段時間的世界：有在世宗門，也盡量有已覆滅的。
func _sect_world() -> SimWorld:
	var world := SimWorld.new()
	world.create(SEED, "small")
	world.step(12 * 120)
	return world


func test_gdd_5_8_sect_card_fields_agree_with_the_sect_list() -> void:
	var world := SimWorld.new()
	assert_true(world.sect(0).is_empty(), "沒有世界時宗門卡資料為空")
	world = _sect_world()
	assert_true(world.sect(-1).is_empty() and world.sect(1000000).is_empty(), "無此宗門回傳空字典")
	var listed := world.sects()
	assert_gt(listed.size(), 0)
	var alive_seen := 0
	for row: Dictionary in listed:
		var sect := world.sect(row["id"])
		for key in ["id", "name", "evil", "alive", "province", "province_name", "pos", "node", "radius", "range", "prestige", "leader", "leader_name",
				"leader_realm", "member_count", "members", "realm_counts", "other_members", "friends", "foes", "ascended", "ascended_people",
				"founded", "founded_era", "founded_year", "now_era", "fallen", "fallen_era", "fallen_year", "fallen_month", "fall_event", "fall_cause"]:
			assert_true(sect.has(key), "宗門卡欄位 %s" % key)
		assert_eq(sect["name"], row["name"])
		assert_eq(sect["alive"], row["alive"])
		assert_eq(sect["province"], row["province"])
		var members: Array = sect["members"]
		assert_eq(members.size(), sect["member_count"], "門人數等於名單條數")
		var counts: PackedInt32Array = sect["realm_counts"]
		var total := int(sect["other_members"])
		for n in counts:
			total += n
		assert_eq(total, int(sect["member_count"]), "各境界人數總和等於門人數")
		var last_realm := 100
		for member: Dictionary in members:
			assert_true(int(member["realm"]) <= last_realm, "門人依境界由高到低")
			last_realm = member["realm"]
			assert_eq(world.person(member["id"])["power"], row["id"], "名單裡的人確屬此宗")
		if row["alive"]:
			alive_seen += 1
			assert_eq(members.size(), row["members"], "名單條數等於宗門清單的成員數")
			if sect["leader"] >= 0:
				var found := false
				for member: Dictionary in members:
					found = found or member["id"] == sect["leader"]
				assert_true(found, "掌門在門人名單裡")
		else:
			assert_eq(members.size(), 0, "已覆滅的宗門沒有門人")
			assert_gt(int(sect["fallen_year"]), 0, "已覆滅的宗門有覆滅年")
	assert_gt(alive_seen, 0)


func test_gdd_5_8_sect_relations_are_ranked_and_capped() -> void:
	var world := _sect_world()
	for row: Dictionary in world.sects():
		var sect := world.sect(row["id"])
		var friends: Array = sect["friends"]
		var foes: Array = sect["foes"]
		assert_true(friends.size() <= 5 and foes.size() <= 5, "友好、敵對各至多前五名")
		var last := 1000
		for entry: Dictionary in friends:
			assert_gt(int(entry["value"]), 0)
			assert_true(int(entry["value"]) <= last, "友好由高到低")
			last = entry["value"]
			assert_true(entry.has("id") and entry.has("name") and entry.has("is_sect"))
		last = -1000
		for entry: Dictionary in foes:
			assert_lt(int(entry["value"]), 0)
			assert_true(int(entry["value"]) >= last, "敵對由深到淺")
			last = entry["value"]


func test_gdd_5_8_sect_history_is_ordered_and_names_the_sect() -> void:
	var world := _sect_world()
	var fallen_seen := 0
	var kinds := {}
	for row: Dictionary in world.sects():
		var id: int = row["id"]
		var history := world.sect_history(id)
		assert_gt(history.size(), 0, "宗門 %d 至少有創立或分宗那一則" % id)
		var last_tick := -1
		for entry: Dictionary in history:
			assert_true((entry["powers"] as PackedInt32Array).has(id), "門史每則的勢力裡有這個宗門")
			assert_true(int(entry["tick"]) >= last_tick, "門史由舊到新")
			last_tick = entry["tick"]
			kinds[entry["kind"]] = true
			assert_false(world.event(entry["id"]).is_empty(), "門史的事件編號取得到事件")
		assert_true(["PowerFounded", "SectSplit"].has(history[0]["kind"]), "門史第一則是立派或分出")
		if not row["alive"]:
			fallen_seen += 1
			assert_eq(history[-1]["kind"], "PowerFallen", "已覆滅宗門的門史以覆滅作結")
			assert_eq(world.sect(id)["fall_event"], history[-1]["id"], "覆滅事件編號對得上")
	assert_true(kinds.has("PowerFounded") or kinds.has("SectSplit"))
	print("[sect] 門史見到的事件種類：%s；已覆滅宗門 %d 個" % [str(kinds.keys()), fallen_seen])


func test_gdd_5_8_sect_ascended_list_matches_the_counter_or_is_pruned() -> void:
	var world := _sect_world()
	var mismatch := 0
	for row: Dictionary in world.sects():
		var sect := world.sect(row["id"])
		var list: Array = sect["ascended_people"]
		assert_true(list.size() <= int(sect["ascended"]), "飛升名錄不多於飛升人數")
		mismatch += 1 if list.size() != int(sect["ascended"]) else 0
		for entry: Dictionary in list:
			assert_true(entry.has("id") and entry.has("name") and entry.has("event"))
	print("[sect] 飛升名錄與計數不一致的宗門：%d" % mismatch)


func test_gdd_5_8_events_and_people_carry_sect_ids() -> void:
	var world := _sect_world()
	var checked := 0
	for row: Dictionary in world.sects():
		if row["members"] <= 0:
			continue
		for member: Dictionary in world.sect(row["id"])["members"]:
			var info := world.person(member["id"])
			assert_eq(info["power"], row["id"])
			for entry: Dictionary in world.person_history(member["id"]):
				assert_true(entry.has("f_power") and entry.has("g_power"), "歷程事件帶宗門編號")
				if entry["f_power"] >= 0:
					assert_eq(world.power_name(entry["f_power"]), entry["f"], "f_power 對得上勢力名")
					checked += 1
				if entry["g_power"] >= 0:
					assert_eq(world.power_name(entry["g_power"]), entry["g"], "g_power 對得上勢力名")
		if checked > 0:
			break
	assert_gt(checked, 0, "至少驗到一筆帶宗門名的歷程事件")
	assert_eq(world.power_name(-1), "")
	for id in world.province_count():
		for person: Dictionary in world.people_in(id):
			assert_true(person.has("power"), "州郡的有名之人帶宗門編號")
