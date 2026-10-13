class_name PartCompose
extends RefCounted
## 一串部件的 CPU 合成（GDD 8.12）：與著色器同一條公式，不需要畫面，headless 測試與並排圖用，也是「同一串兩次逐像素相同」的判準。
## 遊戲內畫面走 PartStack 的著色器；這裡每個輸出像素取畫布座標中心點的最近一格（step = 1 時與 GPU 逐像素對得上）。

const HALO_ALPHA := 0.85


## 把 recipe 疊成一張圖（畫布尺寸除以 step，向上取整）。
static func render(recipe: Dictionary, catalog: PartCatalog, step := 1) -> Image:
	var w := ceili(float(catalog.canvas.x) / step)
	var h := ceili(float(catalog.canvas.y) / step)
	var out := PackedFloat32Array()
	out.resize(w * h * 4)
	var grade: Dictionary = recipe.get("grade", PartTint.no_grade())
	_halo(out, w, h, step, recipe, catalog, grade)
	var atlas := catalog.atlas_image()
	var atlas_w := atlas.get_width()
	var bytes := atlas.get_data()
	for item: Dictionary in recipe["items"]:
		var code: String = item["part"]
		var sc := float(item.get("scale", 1.0))
		for layer: Dictionary in catalog.layers_of(code):
			var stops: Array[Color] = []
			var fixed: bool = layer["mode"] == "fixed"
			if not fixed:
				stops = PartTint.stops_of(item.get("tints", {}).get(layer["tint"]))
			_blit(out, w, h, step, layer, sc, catalog.pivot, bytes, atlas_w, fixed, stops, grade)
	var data := PackedByteArray()
	data.resize(w * h * 4)
	for i in w * h:
		var a := out[i * 4 + 3]
		if a > 0.0:
			data[i * 4] = clampi(roundi(out[i * 4] / a * 255.0), 0, 255)
			data[i * 4 + 1] = clampi(roundi(out[i * 4 + 1] / a * 255.0), 0, 255)
			data[i * 4 + 2] = clampi(roundi(out[i * 4 + 2] / a * 255.0), 0, 255)
			data[i * 4 + 3] = clampi(roundi(a * 255.0), 0, 255)
	return Image.create_from_data(w, h, false, Image.FORMAT_RGBA8, data)


static func _over(out: PackedFloat32Array, at: int, color: Color, alpha: float) -> void:
	var keep := 1.0 - alpha
	out[at] = color.r * alpha + out[at] * keep
	out[at + 1] = color.g * alpha + out[at + 1] * keep
	out[at + 2] = color.b * alpha + out[at + 2] * keep
	out[at + 3] = alpha + out[at + 3] * keep


static func _blit(out: PackedFloat32Array, w: int, h: int, step: int, layer: Dictionary, sc: float, pivot: Vector2, bytes: PackedByteArray, atlas_w: int, fixed: bool, stops: Array[Color], grade: Dictionary) -> void:
	var rect: Rect2i = layer["rect"]
	var offset: Vector2 = layer["offset"]
	# 子層在畫布上的範圍（含縮放），換成輸出像素的範圍。
	var top_left := pivot + (offset - pivot) * sc
	var bottom_right := pivot + (offset + Vector2(rect.size) - pivot) * sc
	var x0 := maxi(0, floori(top_left.x / step))
	var x1 := mini(w - 1, ceili(bottom_right.x / step))
	var y0 := maxi(0, floori(top_left.y / step))
	var y1 := mini(h - 1, ceili(bottom_right.y / step))
	var plain := float(grade.get("desat", 0.0)) == 0.0 and float(grade.get("wash_amount", 0.0)) == 0.0 and float(grade.get("fade", 0.0)) == 0.0
	for y in range(y0, y1 + 1):
		var cy := (float(y) + 0.5) * step
		var sy := floori((pivot.y + (cy - pivot.y) / sc) - offset.y)
		if sy < 0 or sy >= rect.size.y:
			continue
		for x in range(x0, x1 + 1):
			var cx := (float(x) + 0.5) * step
			var sx := floori((pivot.x + (cx - pivot.x) / sc) - offset.x)
			if sx < 0 or sx >= rect.size.x:
				continue
			var at := ((rect.position.y + sy) * atlas_w + rect.position.x + sx) * 4
			var a8 := bytes[at + 3]
			if a8 == 0:
				continue
			var a := float(a8) / 255.0
			var color: Color
			if fixed:
				color = Color(float(bytes[at]) / 255.0 / a, float(bytes[at + 1]) / 255.0 / a, float(bytes[at + 2]) / 255.0 / a)
			else:
				color = PartTint.key_color(float(bytes[at]) / 255.0 / a, stops)
			if not plain:
				color = PartTint.grade(color, grade)
			_over(out, (y * w + x) * 4, color, a)


static func _halo(out: PackedFloat32Array, w: int, h: int, step: int, recipe: Dictionary, catalog: PartCatalog, grade: Dictionary) -> void:
	var rings := int(recipe.get("halo", 0))
	if rings <= 0 or catalog.halo.is_empty():
		return
	var center := Vector2(float(catalog.halo["center"][0]), float(catalog.halo["center"][1]))
	var base_radius := float(catalog.halo["radius"])
	var spacing := float(catalog.halo["step"])
	var width := float(catalog.halo["width"])
	var color := PartTint.grade(Color(PartTint.GOLD), grade)
	for y in h:
		for x in w:
			var d := Vector2((float(x) + 0.5) * step, (float(y) + 0.5) * step).distance_to(center)
			var best := 0.0
			var ring := roundi((d - base_radius) / spacing)
			if ring >= 0 and ring < rings:
				best = clampf(width * 0.5 + 0.5 * step - absf(d - (base_radius + ring * spacing)), 0.0, 1.0)
			if best > 0.0:
				_over(out, (y * w + x) * 4, color, best * HALO_ALPHA)
