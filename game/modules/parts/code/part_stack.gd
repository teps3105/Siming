class_name PartStack
extends Control
## 疊圖節點（GDD 8.12）：給一串部件代號與染色參數（recipe），由後往前畫；每個子層一個著色器呼叫，把明度鍵換成三色。
## recipe：
##   items  由後往前的 [{part: 代號, tints: {染色欄位: Color 或 [陰影, 底色, 高光]}, scale: 縮放（繞類的 pivot，預設 1）}]
##   grade  整張圖的色調（PartTint.no_grade 的格式）
##   halo   背後的光暈圈數（程式畫，不是部件）
## 節點大小就是畫面大小，畫布依比例縮放填滿。同一個 recipe 不重建。

const SHADER := preload("res://modules/parts/code/part_tint.gdshader")

var catalog: PartCatalog
var _recipe := {}
var _key := ""
## 每個子層的 [節點, 畫布上的左上角, 畫布上的大小]。
var _views: Array = []


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	clip_contents = true
	texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS


## 同一串的 recipe 對應同一個字串（快取與「同一人同一串」的判準）。
static func recipe_key(recipe: Dictionary) -> String:
	return var_to_str(recipe)


func recipe() -> Dictionary:
	return _recipe


func set_recipe(recipe: Dictionary, part_catalog: PartCatalog) -> void:
	var key := recipe_key(recipe)
	if key == _key and part_catalog == catalog:
		return
	_key = key
	_recipe = recipe
	catalog = part_catalog
	_rebuild()


func _rebuild() -> void:
	for view: Array in _views:
		remove_child(view[0])
		(view[0] as Node).queue_free()
	_views.clear()
	if catalog == null or _recipe.is_empty():
		queue_redraw()
		return
	var grade: Dictionary = _recipe.get("grade", PartTint.no_grade())
	for item: Dictionary in _recipe["items"]:
		var sc := float(item.get("scale", 1.0))
		for layer: Dictionary in catalog.layers_of(item["part"]):
			var rect := TextureRect.new()
			rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
			rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			rect.stretch_mode = TextureRect.STRETCH_SCALE
			rect.texture = catalog.region_texture(layer)
			rect.material = _material(layer, item.get("tints", {}), grade)
			add_child(rect)
			var offset: Vector2 = layer["offset"]
			var span := Vector2((layer["rect"] as Rect2i).size)
			var top_left := catalog.pivot + (offset - catalog.pivot) * sc
			_views.append([rect, top_left, span * sc])
	_layout()
	queue_redraw()


func _material(layer: Dictionary, tints: Dictionary, grade: Dictionary) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	var fixed: bool = layer["mode"] == "fixed"
	mat.set_shader_parameter("is_fixed", fixed)
	var stops := PartTint.stops_of(tints.get(layer["tint"])) if not fixed else PartTint.stops_of(Color.WHITE)
	mat.set_shader_parameter("ink_color", stops[0])
	mat.set_shader_parameter("shadow_color", stops[1])
	mat.set_shader_parameter("base_color", stops[2])
	mat.set_shader_parameter("light_color", stops[3])
	mat.set_shader_parameter("desat", float(grade.get("desat", 0.0)))
	mat.set_shader_parameter("wash_color", Color(grade.get("wash", Color.BLACK)))
	mat.set_shader_parameter("wash_amount", float(grade.get("wash_amount", 0.0)))
	mat.set_shader_parameter("fade", float(grade.get("fade", 0.0)))
	mat.set_shader_parameter("paper_color", PartTint.PAPER)
	return mat


func _scale() -> Vector2:
	if catalog == null:
		return Vector2.ONE
	return Vector2(size.x / float(catalog.canvas.x), size.y / float(catalog.canvas.y))


func _layout() -> void:
	var s := _scale()
	for view: Array in _views:
		var node: Control = view[0]
		node.position = (view[1] as Vector2) * s
		node.size = (view[2] as Vector2) * s


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_layout()
		queue_redraw()


func _draw() -> void:
	if catalog == null or _recipe.is_empty():
		return
	var rings := int(_recipe.get("halo", 0))
	if rings <= 0 or catalog.halo.is_empty():
		return
	var s := _scale()
	var center := Vector2(float(catalog.halo["center"][0]), float(catalog.halo["center"][1])) * s
	var grade: Dictionary = _recipe.get("grade", PartTint.no_grade())
	var color := PartTint.grade(PartTint.GOLD, grade)
	color.a = PartCompose.HALO_ALPHA
	for i in rings:
		var radius := (float(catalog.halo["radius"]) + i * float(catalog.halo["step"])) * s.x
		draw_arc(center, radius, 0.0, TAU, 96, color, float(catalog.halo["width"]) * s.x, true)
