extends "res://tests/harness/suite.gd"
## 部件庫的疊圖（GDD 8.12）：目錄、染色公式、PartStack 節點，與修士頭像的組合規則（PortraitRecipe）。
## 像素比對走 PartCompose（CPU 合成，與著色器同一條公式），headless 就能跑；GPU 與 CPU 的一致由截圖工具另外核對。

const TOTAL_PARTS := 70
const SLOT_COUNTS := {
	"back_hair": 8, "collar": 12, "face": 6, "wrinkle": 2, "mouth": 6, "eyes": 8,
	"brows": 6, "beard": 5, "mark": 3, "front_hair": 8, "hat": 6,
}
const STEP := 4


func suite_name() -> String:
	return "part_stack"


## 一位假想的人（橋接 person() 的字典形狀）。
func _info(id: int, overrides := {}) -> Dictionary:
	var info := {
		"id": id, "soul": id * 3 + 1, "gender": "Male" if id % 2 == 0 else "Female", "role": "Cultivator", "power_name": "",
		"is_leader": false, "age": 40, "lifespan": 100, "alive": true, "realm": 1, "fruit": -1,
	}
	info.merge(overrides, true)
	return info


## 代號依格索引。
func _by_slot(recipe: Dictionary) -> Dictionary:
	var out := {}
	for item: Dictionary in recipe["items"]:
		out[PartCatalog.slot_of(item["part"])] = item
	return out


func _luma(c: Color) -> float:
	return c.r * 0.299 + c.g * 0.587 + c.b * 0.114


## 圖上有顏色的像素的平均：[亮度, 飽和度, 藍減紅]。
func _mean(image: Image) -> Array[float]:
	var sum_l := 0.0
	var sum_s := 0.0
	var sum_b := 0.0
	var n := 0
	for y in image.get_height():
		for x in image.get_width():
			var c := image.get_pixel(x, y)
			if c.a > 0.5:
				sum_l += _luma(c)
				sum_s += c.s
				sum_b += c.b - c.r
				n += 1
	return [sum_l / n, sum_s / n, sum_b / n]


func test_gdd_8_12_catalog_lists_every_slot_and_part() -> void:
	var catalog := PartCatalog.load_class("portrait")
	assert_true(catalog != null, "目錄讀得到")
	assert_eq(catalog.canvas, Vector2i(320, 400), "入庫尺寸 320×400")
	assert_eq(catalog.part_count(), TOTAL_PARTS, "合計 70 款")
	assert_eq(catalog.slots().size(), SLOT_COUNTS.size())
	var order := -1
	for slot in catalog.slots():
		assert_gt(catalog.slot_order(slot), order, "格由後往前")
		order = catalog.slot_order(slot)
		assert_eq(catalog.parts_in(slot).size(), SLOT_COUNTS[slot], "%s 的款數" % slot)
	var atlas := catalog.atlas_image()
	for code in catalog.parts_in("eyes") + catalog.parts_in("collar") + catalog.parts_in("hat"):
		assert_true(catalog.anchor_of(code).x > 0.0, "有定位點")
		for layer in catalog.layers_of(code):
			var rect: Rect2i = layer["rect"]
			assert_true(Rect2i(Vector2i.ZERO, atlas.get_size()).encloses(rect), "子層在圖集內：" + code)
			assert_true(layer["mode"] in ["keyed", "fixed"])
			assert_true(layer["mode"] == "fixed" or catalog.channels.has(layer["tint"]), "染色欄位有宣告")


func test_gdd_8_12_ink_stays_ink_and_the_other_steps_take_the_tint() -> void:
	for base in [Color("a8463f"), Color("f6e2cf"), Color("7a8a9a")]:
		var stops := PartTint.stops_of(base)
		assert_eq(PartTint.key_color(0.0, stops), PartTint.INK, "墨線維持 ink #2B2620")
		assert_eq(PartTint.key_color(2.0 / 3.0, stops).to_html(), base.to_html(), "底色是傳入的顏色")
		assert_lt(_luma(PartTint.key_color(1.0 / 3.0, stops)), _luma(base), "陰影比底色暗")
		assert_gt(_luma(PartTint.key_color(1.0, stops)), _luma(base), "高光比底色亮")
	var explicit: Array[Color] = [Color.RED, Color.GREEN, Color.BLUE]
	assert_eq(PartTint.key_color(1.0, PartTint.stops_of(explicit)), Color.BLUE, "也可以直接傳三色")
	var shader := load("res://modules/parts/code/part_tint.gdshader") as Shader
	assert_true(shader != null and shader.code.contains("ink_color") and shader.code.contains("is_fixed"), "著色器在")


func test_gdd_8_12_the_same_stack_renders_pixel_for_pixel_the_same() -> void:
	var catalog := PartCatalog.load_class("portrait")
	var recipe := PortraitRecipe.from_person(_info(12, {"realm": 3}))
	var first := PartCompose.render(recipe, catalog, STEP)
	var second := PartCompose.render(PortraitRecipe.from_person(_info(12, {"realm": 3})), catalog, STEP)
	assert_eq(first.get_data(), second.get_data(), "兩次疊出的圖逐位元組相同")
	assert_eq(first.get_size(), Vector2i(80, 100))
	var plain := PartCompose.render(PortraitRecipe.from_person(_info(12, {"realm": -1})), catalog, STEP)
	assert_ne(first.get_data(), plain.get_data(), "有境界光暈與沒有不同")


func test_gdd_8_12_the_same_person_always_gets_the_same_stack() -> void:
	for id in 20:
		assert_eq(PortraitRecipe.key(PortraitRecipe.from_person(_info(id))), PortraitRecipe.key(PortraitRecipe.from_person(_info(id))))
	var catalog := PartCatalog.load_class("portrait")
	for id in 40:
		for item: Dictionary in PortraitRecipe.from_person(_info(id, {"age": 20 + id * 2}))["items"]:
			assert_true(catalog.has_part(item["part"]), "代號都在目錄裡：" + item["part"])


func test_gdd_8_12_thirty_people_have_thirty_different_stacks() -> void:
	var roles := ["Cultivator", "Monk", "Scholar", "Official", "Commoner", "Ruler"]
	var seen := {}
	for id in 30:
		var info := _info(100 + id, {"role": roles[id % roles.size()], "age": 20 + id * 2, "realm": id % 9})
		seen[PortraitRecipe.key(PortraitRecipe.from_person(info))] = true
	assert_eq(seen.size(), 30, "30 人沒有兩串完全相同")


func test_gdd_8_12_youth_prime_and_age_change_only_what_should_change() -> void:
	var beards_in_age := 0
	for id in range(2, 62, 2):
		var young := PortraitRecipe.from_person(_info(id, {"age": 12}))
		var prime := PortraitRecipe.from_person(_info(id, {"age": 45}))
		var elder := PortraitRecipe.from_person(_info(id, {"age": 90}))
		assert_eq(PortraitRecipe.stage(_info(id, {"age": 12})), PortraitRecipe.Stage.YOUNG)
		assert_eq(PortraitRecipe.stage(_info(id, {"age": 45})), PortraitRecipe.Stage.PRIME)
		assert_eq(PortraitRecipe.stage(_info(id, {"age": 90})), PortraitRecipe.Stage.ELDER)
		var y := _by_slot(young)
		var p := _by_slot(prime)
		var e := _by_slot(elder)
		for slot in ["back_hair", "collar", "face", "mouth", "eyes", "brows", "front_hair"]:
			assert_eq(y[slot]["part"], p[slot]["part"], "少壯同一款：" + slot)
			assert_eq(p[slot]["part"], e[slot]["part"], "壯老同一款：" + slot)
		assert_eq(p.has("hat"), e.has("hat"))
		assert_false(y.has("beard"), "少年無鬚")
		assert_false(y.has("wrinkle") or p.has("wrinkle"), "老年才疊皺紋")
		assert_true(e.has("wrinkle"), "老年疊皺紋")
		if p.has("beard"):
			assert_eq(p["beard"]["part"], e["beard"]["part"], "壯年留的鬚，老了還是那一款")
		if e.has("beard"):
			beards_in_age += 1
		assert_lt(float(y["face"]["scale"]), 1.0, "少年臉略縮")
		assert_false(p["face"].has("scale"))
		assert_gt(_luma(e["brows"]["tints"]["hair"]), _luma(p["brows"]["tints"]["hair"]), "老年髮色轉白")
		assert_eq(p["brows"]["tints"]["hair"], y["brows"]["tints"]["hair"], "少壯髮色相同")
		assert_eq(p["face"]["tints"]["skin"], e["face"]["tints"]["skin"], "膚色不隨年紀變")
	assert_gt(beards_in_age, 10, "老年男子多有鬚")


func test_gdd_8_12_age_is_the_share_of_lifespan_used_not_the_years() -> void:
	var short_lived := _info(8, {"age": 45, "lifespan": 60})
	var long_lived := _info(8, {"age": 45, "lifespan": 500})
	assert_eq(PortraitRecipe.stage(short_lived), PortraitRecipe.Stage.ELDER, "壽元六十者四十五歲已老")
	assert_eq(PortraitRecipe.stage(long_lived), PortraitRecipe.Stage.YOUNG, "壽元五百者四十五歲仍少")
	assert_eq(PortraitRecipe.stage(_info(8, {"role": "Child", "age": 50})), PortraitRecipe.Stage.YOUNG, "孩童恆為少")


func test_gdd_8_12_eyes_follow_the_soul_and_the_rest_of_the_face_follows_the_person() -> void:
	# 同一個魂轉世（魂編號相同、性別相同、人物編號不同）：眼不變。
	var eyes := {}
	for id in range(0, 40, 2):
		var a := _by_slot(PortraitRecipe.from_person(_info(id, {"soul": 77})))
		var b := _by_slot(PortraitRecipe.from_person(_info(id + 100, {"soul": 77})))
		assert_eq(a["eyes"]["part"], b["eyes"]["part"], "同一個魂，眼同一款")
		assert_eq(a["eyes"]["tints"]["iris"], b["eyes"]["tints"]["iris"], "瞳色也同")
	for soul in 30:
		eyes[_by_slot(PortraitRecipe.from_person(_info(0, {"soul": soul})))["eyes"]["part"]] = true
	assert_gt(eyes.size(), 2, "不同的魂有不同的眼")


func test_gdd_8_12_a_demon_gets_marks_violet_eyes_and_a_violet_cast_and_nothing_else_changes() -> void:
	var catalog := PartCatalog.load_class("portrait")
	for id in [4, 9, 14]:
		var human := PortraitRecipe.from_person(_info(id))
		var demon := PortraitRecipe.from_person(_info(id, {"role": "Demonic"}))
		var h := _by_slot(human)
		var d := _by_slot(demon)
		assert_true(d.has("mark") and not h.has("mark"), "入魔疊面紋")
		assert_eq(d["eyes"]["part"], h["eyes"]["part"], "眼的款不變")
		assert_ne(d["eyes"]["tints"]["iris"], h["eyes"]["tints"]["iris"], "瞳色改變")
		assert_gt(float(demon["grade"]["wash_amount"]), 0.0, "整張偏紫")
		assert_eq(float(human["grade"]["wash_amount"]), 0.0)
		for slot in ["back_hair", "collar", "face", "mouth", "brows", "front_hair"]:
			assert_eq(d[slot]["part"], h[slot]["part"], "款不變：" + slot)
		var a := _mean(PartCompose.render(human, catalog, STEP))
		var b := _mean(PartCompose.render(demon, catalog, STEP))
		assert_gt(b[2], a[2], "入魔者整體偏藍紫")


func test_gdd_8_12_the_dead_fade_to_pale_ink_without_changing_parts() -> void:
	var catalog := PartCatalog.load_class("portrait")
	for id in [3, 6, 11]:
		var living := PortraitRecipe.from_person(_info(id, {"realm": 4}))
		var gone := PortraitRecipe.from_person(_info(id, {"realm": 4, "alive": false}))
		assert_eq(PortraitRecipe.codes(gone), PortraitRecipe.codes(living), "身故不換部件")
		assert_gt(float(gone["grade"]["fade"]), 0.0)
		assert_gt(float(gone["grade"]["desat"]), 0.0)
		var a := _mean(PartCompose.render(living, catalog, STEP))
		var b := _mean(PartCompose.render(gone, catalog, STEP))
		assert_gt(b[0], a[0], "褪成淡墨：更亮")
		assert_lt(b[1], a[1] * 0.6, "褪成淡墨：去飽和")


func test_gdd_8_12_halo_rings_follow_the_realm_and_the_fruit() -> void:
	var last := 0
	for realm in 9:
		var rings := PortraitRecipe.halo_rings(_info(1, {"realm": realm}))
		assert_gt(rings, last, "境界越高圈數越多")
		last = rings
	assert_eq(PortraitRecipe.halo_rings(_info(1, {"realm": -1, "fruit": 4})), 5)
	assert_eq(PortraitRecipe.halo_rings(_info(1, {"realm": -1, "fruit": -1})), 0, "沒有境界不畫光暈")
	assert_eq(PortraitRecipe.from_person(_info(1, {"realm": 5}))["halo"], 6)


func test_gdd_8_12_identity_picks_the_collar_and_the_hat() -> void:
	var monk := _by_slot(PortraitRecipe.from_person(_info(10, {"role": "Monk", "realm": -1, "fruit": 2})))
	assert_true(monk["collar"]["part"].contains("shi_"), "僧人穿釋門領")
	assert_eq(monk["front_hair"]["part"], "portrait/front_hair.shaven", "僧人落髮")
	var ruler := _by_slot(PortraitRecipe.from_person(_info(10, {"role": "Ruler", "realm": -1})))
	assert_true(ruler["collar"]["part"].ends_with("_rich"), "君王衣飾華貴")
	var scholar := _by_slot(PortraitRecipe.from_person(_info(10, {"role": "Scholar", "realm": -1})))
	assert_true(scholar["collar"]["part"].contains("ru_"), "儒生穿儒領")


func test_gdd_8_12_part_stack_node_builds_one_view_per_layer_and_keeps_an_unchanged_stack() -> void:
	var catalog := PartCatalog.load_class("portrait")
	var recipe := PortraitRecipe.from_person(_info(5, {"role": "Demonic", "age": 90}))
	var stack := PartStack.new()
	track(stack)
	stack.size = Vector2(160.0, 200.0)
	stack.set_recipe(recipe, catalog)
	var layers := 0
	for item: Dictionary in recipe["items"]:
		layers += catalog.layers_of(item["part"]).size()
	assert_eq(stack.get_child_count(), layers, "每個子層一個節點、各一個著色器呼叫")
	var first := stack.get_child(0)
	stack.set_recipe(PortraitRecipe.from_person(_info(5, {"role": "Demonic", "age": 90})), catalog)
	assert_eq(stack.get_child(0), first, "同一串不重建")
	var layer: Dictionary = catalog.layers_of(recipe["items"][0]["part"])[0]
	assert_eq((first as Control).size, Vector2(layer["rect"].size) * 0.5, "畫布 320×400 縮成 160×200")
	stack.set_recipe(PortraitRecipe.from_person(_info(6)), catalog)
	assert_ne(stack.get_child(0), first, "換一串就重建")


## 試驗類：畫家畫的真部件（portrait_trial），格式與佔位類相同，款數隨已入庫的部件增加。
const TRIAL_SLOTS := ["back_hair", "face", "collar", "eyes", "brows", "beard", "front_hair"]
const TRIAL_TINTS := {"skin": Color("dfb182"), "hair": Color("605450"), "robe": Color("5c7080"), "trim": Color("b23a2e"), "iris": Color("466e96")}


func test_gdd_8_12_trial_catalog_reads_and_every_rect_is_inside_the_atlas() -> void:
	var catalog := PartCatalog.load_class("portrait_trial")
	assert_true(catalog != null, "試驗類目錄讀得到")
	assert_false(catalog.placeholder, "真部件的目錄不標佔位")
	assert_eq(catalog.canvas, Vector2i(320, 400), "入庫尺寸寫在目錄")
	assert_eq(catalog.slots().size(), TRIAL_SLOTS.size(), "格數")
	for i in TRIAL_SLOTS.size():
		assert_eq(catalog.slots()[i], TRIAL_SLOTS[i], "格由後往前：臉在領之後，領蓋住頸根")
	var atlas := catalog.atlas_image()
	var bounds := Rect2i(Vector2i.ZERO, atlas.get_size())
	for slot: String in TRIAL_SLOTS:
		assert_eq(catalog.parts_in(slot).size(), catalog.slot_count(slot), "%s 的款數與目錄一致" % slot)
		assert_gt(catalog.parts_in(slot).size(), 0, "%s 至少一款" % slot)
		for code in catalog.parts_in(slot):
			assert_true(code.begins_with("portrait_trial/"), "代號前綴是類名：" + code)
			assert_gt(catalog.anchor_of(code).x, 0.0, "有定位點：" + code)
			for layer in catalog.layers_of(code):
				var rect: Rect2i = layer["rect"]
				assert_true(bounds.encloses(rect), "子層在圖集內：" + code)
				assert_true(layer["mode"] in ["keyed", "fixed"])
				assert_true(layer["mode"] == "fixed" or catalog.channels.has(layer["tint"]), "染色欄位有宣告：" + code)
				var mid := atlas.get_pixel(rect.position.x + rect.size.x / 2, rect.position.y + rect.size.y / 2)
				assert_lt(maxf(mid.r, maxf(mid.g, mid.b)), mid.a + 0.01, "預乘 alpha：RGB 不超過 alpha")


func test_gdd_8_12_trial_stack_renders_a_non_empty_portrait_and_dye_changes_it() -> void:
	var catalog := PartCatalog.load_class("portrait_trial")
	var items: Array[Dictionary] = []
	for slot in catalog.slots():
		items.append({"part": catalog.parts_in(slot)[0], "tints": TRIAL_TINTS.duplicate()})
	var recipe := {"items": items, "halo": 0}
	var image := PartCompose.render(recipe, catalog, 2)
	var solid := 0
	for y in image.get_height():
		for x in image.get_width():
			if image.get_pixel(x, y).a > 0.5:
				solid += 1
	assert_gt(solid, image.get_width() * image.get_height() / 4, "疊出的圖不是空的")
	var white := recipe.duplicate(true)
	for item: Dictionary in white["items"]:
		item["tints"]["hair"] = Color("e1e1dc")
	assert_ne(PartCompose.render(white, catalog, 2).get_data(), image.get_data(), "髮色一換圖就不同")
	assert_eq(PartCompose.render(recipe, catalog, 2).get_data(), image.get_data(), "同一串兩次逐位元組相同")
	for item: Dictionary in items:
		var single := PartCompose.render({"items": [item], "halo": 0}, catalog, 2)
		var count := 0
		for y in single.get_height():
			for x in single.get_width():
				if single.get_pixel(x, y).a > 0.0:
					count += 1
		assert_gt(count, 50, "單層非空：" + String(item["part"]))
