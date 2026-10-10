class_name InkIcons
extends RefCounted
## 程式繪製的線條圖示（GDD 5.7 原則「圖像加文字」）。圖示一律與文字並用，不單獨當按鈕。
## center 為圖示中心，r 為半徑。

const WIDTH := 1.6


static func draw(item: CanvasItem, icon: StringName, center: Vector2, r: float, color: Color) -> void:
	match icon:
		&"play":
			item.draw_colored_polygon(_at(center, r, [Vector2(-0.5, -0.8), Vector2(0.85, 0.0), Vector2(-0.5, 0.8)]), color)
		&"pause":
			for x: float in [-0.6, 0.2]:
				item.draw_rect(Rect2(center + Vector2(x, -0.75) * r, Vector2(0.4, 1.5) * r), color, true)
		&"close":
			_line(item, center, r, [Vector2(-0.7, -0.7), Vector2(0.7, 0.7)], color)
			_line(item, center, r, [Vector2(-0.7, 0.7), Vector2(0.7, -0.7)], color)
		&"center":
			item.draw_arc(center, r * 0.6, 0.0, TAU, 24, color, WIDTH, true)
			for dir: Vector2 in [Vector2.UP, Vector2.DOWN, Vector2.LEFT, Vector2.RIGHT]:
				item.draw_line(center + dir * r * 0.35, center + dir * r, color, WIDTH, true)
		&"shanhe":
			# 山與水：兩座山峰，下面一道水紋。
			_line(item, center, r, [Vector2(-1.0, 0.35), Vector2(-0.35, -0.75), Vector2(0.1, 0.0), Vector2(0.45, -0.4), Vector2(1.0, 0.35)], color)
			_line(item, center, r, [Vector2(-1.0, 0.8), Vector2(-0.5, 0.6), Vector2(0.0, 0.8), Vector2(0.5, 0.6), Vector2(1.0, 0.8)], color)
		&"tianxia":
			# 疆域：一圈界線，中間一座都城。
			_line(item, center, r, [
				Vector2(-0.9, -0.3), Vector2(-0.3, -0.9), Vector2(0.6, -0.7), Vector2(0.95, 0.1),
				Vector2(0.4, 0.9), Vector2(-0.6, 0.75), Vector2(-0.9, -0.3),
			], color)
			item.draw_rect(Rect2(center - Vector2(0.22, 0.22) * r, Vector2(0.44, 0.44) * r), color, true)
		&"sanjiao":
			# 三教：三個相扣的圓。
			for dir: Vector2 in [Vector2(0.0, -0.45), Vector2(-0.42, 0.3), Vector2(0.42, 0.3)]:
				item.draw_arc(center + dir * r, r * 0.45, 0.0, TAU, 20, color, WIDTH, true)
		&"lingmai":
			# 靈脈：一條脈絡串起幾個節點。
			var path := [Vector2(-0.95, 0.6), Vector2(-0.3, -0.05), Vector2(0.3, 0.3), Vector2(0.95, -0.6)]
			_line(item, center, r, path, color)
			for p: Vector2 in path:
				item.draw_circle(center + p * r, r * 0.2, color)
		&"yinguo":
			# 因果：兩個結，一線相牽。
			item.draw_arc(center + Vector2(-0.55, 0.45) * r, r * 0.35, 0.0, TAU, 16, color, WIDTH, true)
			item.draw_arc(center + Vector2(0.55, -0.45) * r, r * 0.35, 0.0, TAU, 16, color, WIDTH, true)
			_line(item, center, r, [Vector2(-0.3, 0.2), Vector2(0.0, -0.25), Vector2(0.05, 0.2), Vector2(0.3, -0.2)], color)
		&"yeqi":
			# 業氣：一簇火。
			_line(item, center, r, [
				Vector2(0.0, -1.0), Vector2(0.55, -0.1), Vector2(0.6, 0.5), Vector2(0.0, 0.95),
				Vector2(-0.6, 0.5), Vector2(-0.45, -0.1), Vector2(-0.1, 0.25), Vector2(0.0, -1.0),
			], color)


static func _at(center: Vector2, r: float, points: Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	for p: Vector2 in points:
		out.append(center + p * r)
	return out


static func _line(item: CanvasItem, center: Vector2, r: float, points: Array, color: Color) -> void:
	item.draw_polyline(_at(center, r, points), color, WIDTH, true)
