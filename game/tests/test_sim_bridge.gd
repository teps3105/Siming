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


func test_gdd_6_5_same_seed_gives_same_hash() -> void:
	var other := SimWorld.new()
	other.create(SEED, "small")
	var world := _small_world()
	other.step(maxi(int(world.tick() - other.tick()), 0))
	world.step(maxi(int(other.tick() - world.tick()), 0))
	assert_eq(other.hash_hex(), world.hash_hex())
	world.step(12)
	other.step(12)
	assert_eq(other.hash_hex(), world.hash_hex(), "各推進 12 個月後仍相同")


func test_date_uses_the_era_form_with_arabic_numerals() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(20261011, "small"), "", "世界應能生成")
	world.step(22)
	# 世界生成後已預跑一段時間；月份由 tick 換算（tick 1 是 1月）。
	var months := world.tick() - 1
	var date := world.date_short()
	var parts := date.split(" ")
	assert_eq(parts.size(), 3, "日期是「某某紀元 N年 M月」：%s" % date)
	assert_true(parts[0].ends_with("紀元") and parts[0].length() == 4, "紀元名是兩個字加「紀元」：%s" % parts[0])
	assert_true(parts[1].ends_with("年") and parts[1].trim_suffix("年").is_valid_int(), "年是阿拉伯數字：%s" % parts[1])
	assert_eq(parts[2], "%d月" % (months % 12 + 1))
	# 開局不是開天第一年：開局前的世界已有一段歷史（GDD 2.8）。
	var history := world.history_years()
	assert_gt(history, 0, "開局前已有歷史")
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
	assert_eq(world.load_begin(text.replace('"hash":', '"hash":1')), "", "雜湊到重播完才比對")
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
	world.step(359 - world.tick() % 360)
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
