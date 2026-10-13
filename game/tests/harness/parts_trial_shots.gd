extends SceneTree
## 試驗類（portrait_trial）真部件的畫面證據（GDD 8.12 驗收）：三組膚色／髮色／衣色的並排圖（放大 3 倍）、
## 160×200 實際尺寸一張（走 PartStack 著色器，與遊戲同一條路）、每款單層在紙色底與墨色底的圖。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1280x800 --script res://tests/harness/parts_trial_shots.gd -- --out=<資料夾>

const CLASS_ID := "portrait_trial"
const PALETTES := [
	{"skin": "dfb182", "hair": "605450", "robe": "5c7080", "trim": "b23a2e", "iris": "466e96"},
	{"skin": "eed0b8", "hair": "e1e1dc", "robe": "967652", "trim": "3c342e", "iris": "6e543a"},
	{"skin": "b07e58", "hair": "7a4c32", "robe": "56806c", "trim": "c9a246", "iris": "3a7860"},
]
var _out := "user://parts_trial_shots"


func _initialize() -> void:
	_main.call_deferred()


func _tints(pal: Dictionary) -> Dictionary:
	var out := {}
	for key: String in pal:
		out[key] = Color(pal[key])
	return out


## 目錄裡每格的第一款，由後往前。
func _recipe(catalog: PartCatalog, pal: Dictionary, only := "") -> Dictionary:
	var items: Array[Dictionary] = []
	for slot in catalog.slots():
		for code in catalog.parts_in(slot):
			if only == "" or only == code:
				items.append({"part": code, "tints": _tints(pal)})
			break
	return {"items": items, "halo": 0}


func _over(bg: Color, image: Image) -> Image:
	var out := Image.create(image.get_width(), image.get_height(), false, Image.FORMAT_RGBA8)
	out.fill(bg)
	out.blend_rect(image, Rect2i(Vector2i.ZERO, image.get_size()), Vector2i.ZERO)
	return out


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	DirAccess.make_dir_recursive_absolute(_out)
	var catalog := PartCatalog.load_class(CLASS_ID)
	if catalog == null:
		push_error("讀不到 %s 目錄" % CLASS_ID)
		quit(1)
		return
	# 一、三組染色並排，放大 3 倍（最近鄰，看得清邊）
	var sheet := Image.create(320 * 3, 400, false, Image.FORMAT_RGBA8)
	for i in 3:
		var img := _over(Color("ede4cf"), PartCompose.render(_recipe(catalog, PALETTES[i]), catalog, 1))
		sheet.blit_rect(img, Rect2i(Vector2i.ZERO, img.get_size()), Vector2i(320 * i, 0))
	var big := sheet.duplicate() as Image
	big.resize(sheet.get_width() * 3, sheet.get_height() * 3, Image.INTERPOLATE_NEAREST)
	big.save_png(_out.path_join("stack_x3.png"))
	# 二、每款單層在紙色底與墨色底
	var codes: Array[String] = []
	for slot in catalog.slots():
		codes.append_array(catalog.parts_in(slot))
	var layers := Image.create(320 * codes.size(), 800, false, Image.FORMAT_RGBA8)
	for i in codes.size():
		var single := PartCompose.render(_recipe(catalog, PALETTES[0], codes[i]), catalog, 1)
		var paper := _over(Color("ede4cf"), single)
		var ink := _over(Color("2b2620"), single)
		layers.blit_rect(paper, Rect2i(Vector2i.ZERO, paper.get_size()), Vector2i(320 * i, 0))
		layers.blit_rect(ink, Rect2i(Vector2i.ZERO, ink.get_size()), Vector2i(320 * i, 400))
	layers.save_png(_out.path_join("layers.png"))
	print("單層順序：", codes)
	# 三、160×200 實際尺寸：PartStack（著色器）在 SubViewport 以 0.5 倍畫出，與遊戲內同一條路
	var actual := Image.create(160 * 3, 200, false, Image.FORMAT_RGBA8)
	actual.fill(Color("ede4cf"))
	for i in 3:
		var viewport := SubViewport.new()
		viewport.size = Vector2i(160, 200)
		viewport.transparent_bg = true
		viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
		var stack := PartStack.new()
		stack.size = Vector2(320.0, 400.0)
		stack.scale = Vector2(0.5, 0.5)
		viewport.add_child(stack)
		root.add_child(viewport)
		stack.set_recipe(_recipe(catalog, PALETTES[i]), catalog)
		for f in 4:
			await process_frame
		var shot := viewport.get_texture().get_image()
		shot.convert(Image.FORMAT_RGBA8)
		actual.blend_rect(shot, Rect2i(Vector2i.ZERO, shot.get_size()), Vector2i(160 * i, 0))
		root.remove_child(viewport)
		viewport.queue_free()
	actual.save_png(_out.path_join("actual_160x200.png"))
	print("完成：", _out)
	quit(0)
