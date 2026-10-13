extends SceneTree
## 試驗類（portrait_trial）真部件的隨機組合圖（GDD 8.12 驗收「疊回」「分辨」）：
## 用目錄裡現有的款，依固定種子隨機組 12 個頭（臉×髮×眼×眉×鬚×領，膚色髮色衣色各異）。
## random_x3.png：4×3，放大 3 倍（最近鄰）；random_actual.png：4×3，每個 160×200，走 PartStack 著色器。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1280x800 --script res://tests/harness/parts_trial_random.gd -- --out=<資料夾> [--seed=7]

const CLASS_ID := "portrait_trial"
const SKINS := ["dfb182", "eed0b8", "b07e58", "c98f66", "f0d9c0", "9a6a48"]
const HAIRS := ["605450", "e1e1dc", "7a4c32", "2f2a28", "a8864e", "8a8a90"]
const ROBES := ["5c7080", "967652", "56806c", "7a5c68", "4a4f6a", "8a4a44"]
const TRIMS := ["b23a2e", "3c342e", "c9a246", "e8dcc0", "2f5a7a"]
const IRISES := ["466e96", "6e543a", "3a7860", "3c3c3c"]
var _out := "user://parts_trial_random"
var _seed := 7


func _initialize() -> void:
	_main.call_deferred()


func _pick(rng: RandomNumberGenerator, list: Array) -> Variant:
	return list[rng.randi() % list.size()]


## 髮式成對：前髮與後髮取同一個款名（目錄代號末段）。
func _recipe(catalog: PartCatalog, rng: RandomNumberGenerator) -> Dictionary:
	var tints := {"skin": Color(_pick(rng, SKINS)), "hair": Color(_pick(rng, HAIRS)), "robe": Color(_pick(rng, ROBES)),
		"trim": Color(_pick(rng, TRIMS)), "iris": Color(_pick(rng, IRISES))}
	var chosen := {}
	for slot in catalog.slots():
		if slot in ["back_hair", "front_hair", "hat"]:
			continue
		var codes := catalog.parts_in(slot)
		if codes.is_empty():
			continue
		if slot == "beard" and rng.randi() % 3 == 0:
			continue   # 三分之一無鬚
		chosen[slot] = codes[rng.randi() % codes.size()]
	# 先定髮式（前後髮成對），再只從相容的冠帽裡挑（含不戴）
	var look := PortraitRecipe.pair_hair_hat(catalog, rng.randi())
	chosen["back_hair"] = "%s/back_hair.%s" % [CLASS_ID, look["hair"]]
	chosen["front_hair"] = "%s/front_hair.%s" % [CLASS_ID, look["hair"]]
	if not String(look["hat"]).is_empty():
		chosen["hat"] = look["hat"]
	var items: Array[Dictionary] = []
	for slot in catalog.slots():
		if chosen.has(slot):
			items.append({"part": chosen[slot], "tints": tints})
	return {"items": items, "halo": 0, "_names": chosen}


func _over(bg: Color, image: Image) -> Image:
	var out := Image.create(image.get_width(), image.get_height(), false, Image.FORMAT_RGBA8)
	out.fill(bg)
	out.blend_rect(image, Rect2i(Vector2i.ZERO, image.get_size()), Vector2i.ZERO)
	return out


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--seed="):
			_seed = int(arg.trim_prefix("--seed="))
	DirAccess.make_dir_recursive_absolute(_out)
	var catalog := PartCatalog.load_class(CLASS_ID)
	if catalog == null:
		push_error("讀不到 %s 目錄" % CLASS_ID)
		quit(1)
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = _seed
	var recipes: Array[Dictionary] = []
	for i in 12:
		recipes.append(_recipe(catalog, rng))
		print("頭 %d：%s" % [i + 1, recipes[i]["_names"]])
	var sheet := Image.create(320 * 4, 400 * 3, false, Image.FORMAT_RGBA8)
	for i in 12:
		var img := _over(Color("ede4cf"), PartCompose.render(recipes[i], catalog, 1))
		sheet.blit_rect(img, Rect2i(Vector2i.ZERO, img.get_size()), Vector2i(320 * (i % 4), 400 * (i / 4)))
	var big := sheet.duplicate() as Image
	big.resize(sheet.get_width() * 3, sheet.get_height() * 3, Image.INTERPOLATE_NEAREST)
	big.save_png(_out.path_join("random_x3.png"))
	var actual := Image.create(160 * 4, 200 * 3, false, Image.FORMAT_RGBA8)
	actual.fill(Color("ede4cf"))
	for i in 12:
		var viewport := SubViewport.new()
		viewport.size = Vector2i(160, 200)
		viewport.transparent_bg = true
		viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
		var stack := PartStack.new()
		stack.size = Vector2(320.0, 400.0)
		stack.scale = Vector2(0.5, 0.5)
		viewport.add_child(stack)
		root.add_child(viewport)
		stack.set_recipe(recipes[i], catalog)
		for f in 4:
			await process_frame
		var shot := viewport.get_texture().get_image()
		shot.convert(Image.FORMAT_RGBA8)
		actual.blend_rect(shot, Rect2i(Vector2i.ZERO, shot.get_size()), Vector2i(160 * (i % 4), 200 * (i / 4)))
		root.remove_child(viewport)
		viewport.queue_free()
	actual.save_png(_out.path_join("random_actual.png"))
	print("完成：", _out)
	quit(0)
