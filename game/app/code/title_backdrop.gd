class_name TitleBackdrop
extends Control
## 標題畫面的背景（GDD 9.1）：一幅淡墨山水。山形取自世界的地形——把取景那一塊地由近到遠分成幾帶，
## 每帶取一道山稜的剖面（SceneFraming.ridges），畫成一層層的山影：遠的山腳高、墨淡，近的山腳低、墨濃；
## 每層自山稜往下漸淡，山腳沒入雲裡。取的地比畫面寬（WIDER 倍），每層又把山往兩旁鋪開、墊上一道隨地形起伏的底，所以山連綿橫過整幅、不斷在哪一段；
## 各層的高度另乘一道構圖用的包絡：中間偏左那一段保留全高、兩旁略低，所以最高的主峰在中間偏左；山稜抹平過，起伏和緩；高山上有幾筆皴，矮的遠山只有淡影；
## 畫面右邊（標題與選單後面）的山影淡下去。霧是整幅共用的一片霧場（title_fog.gdshader）：每層山影各是一個子節點，
## 照自己的遠近被霧染成紙色（山體本身不透明、蓋住紙紋，霧濃到幾乎全白時才淡出），越遠越淡、山腰以下霧更重，霧團慢慢流動。

## 由近到遠幾層、每層幾欄。
const LAYERS := 7
const COLUMNS := 192
## 山稜取的地比畫面涵蓋的寬幾倍、深幾倍：一幅裡看得到的山頭多，才連綿。
const WIDER := Vector2(1.7, 1.5)
## 每一欄先取左右各 SPREAD 欄裡最高的（山往兩旁鋪開、填掉缺口），再墊一道底：左右各 SWELL 欄的平均乘 SWELL_LIFT，
## 與這一層最高處的 FLOOR 倍，取大的。
const SPREAD := 5
const SWELL := 28
const SWELL_LIFT := 0.85
const FLOOR := 0.55
## 以下 x 是最近一層、y 是最遠一層的值，中間的層內插；長度都是畫面高度的比例。
## 山腳線的高度（離上緣）、最高的山有多高。
const FOOT := Vector2(0.90, 0.50)
const RISE := Vector2(0.22, 0.42)
## 山稜處的墨色濃度。
const TONE := Vector2(0.50, 0.30)
## 山稜抹平：每一欄取左右各 SMOOTH_REACH 欄的平均，做 SMOOTH_PASSES 遍。
const SMOOTH_REACH := 2
const SMOOTH_PASSES := 2
## 構圖的包絡：畫面寬度的這一段（主山）保留全高，往兩旁在 MAIN_EDGE 的寬度內降到 FLANK_LIFT 倍。
const MAIN_SPAN := Vector2(0.28, 0.52)
const MAIN_EDGE := 0.18
const FLANK_LIFT := 0.6
## 標題與選單後面：畫面寬度的這一段之間，山影淡到剩 1 − OPEN_FADE。
const OPEN := Vector2(0.60, 0.92)
const OPEN_FADE := 0.42
## 每層自山稜往下多深才淡到沒有（至少這麼深），與中間那一排的位置、濃度。
const FALL_MIN := 0.12
const MID_AT := 0.34
const MID_TONE := 0.62
## 高過這個（正規化後）的山才畫皴，矮的遠山無皴。
const TEXTURE_ABOVE := 0.38
## 霧：最遠一層整體淡掉多少、霧團最濃處遮掉多少、霧團的大小（畫面高度裡橫直各幾團）、流動的速度。
const FOG := preload("res://app/code/title_fog.gdshader")
const FOG_HAZE := 0.3
const FOG_DENSITY := 0.9
const FOG_SCALE := Vector2(2.2, 3.4)
const FOG_DRIFT := 0.012


## 一層山影：自己一個節點，才能各帶一份霧的參數。
class RidgeLayer:
	extends Control

	var backdrop: TitleBackdrop
	var index := 0

	func _draw() -> void:
		backdrop._draw_range(self, index)

## 由近到遠各層山稜的高度（0..1，對全圖最高點）；設定後重畫。
var ridges: Array[PackedFloat32Array] = []:
	set(value):
		ridges = value
		_top = 0.0
		_smooth.clear()
		for ridge in ridges:
			var eased := _ranged(ridge)
			for pass_index in SMOOTH_PASSES:
				var next := PackedFloat32Array()
				next.resize(eased.size())
				for column in eased.size():
					var sum := 0.0
					for offset in range(-SMOOTH_REACH, SMOOTH_REACH + 1):
						sum += eased[clampi(column + offset, 0, eased.size() - 1)]
					next[column] = sum / (SMOOTH_REACH * 2 + 1)
				eased = next
			for column in eased.size():
				_top = maxf(_top, eased[column])
			_smooth.append(eased)
		_meshes.clear()
		for child in _ranges:
			child.queue_redraw()

var _top := 0.0
## 鋪開、乘上包絡、墊底並抹平後的各層山稜。
var _smooth: Array[PackedFloat32Array] = []
var _meshes: Array[ArrayMesh] = []
var _built_for := Vector2.ZERO
var _ranges: Array[RidgeLayer] = []


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	# 由遠到近各一個子節點：遠的先畫。
	_ranges.resize(LAYERS)
	for layer in range(LAYERS - 1, -1, -1):
		var child := RidgeLayer.new()
		child.backdrop = self
		child.index = layer
		child.mouse_filter = Control.MOUSE_FILTER_IGNORE
		var fog := ShaderMaterial.new()
		fog.shader = FOG
		fog.set_shader_parameter("depth", layer / maxf(LAYERS - 1.0, 1.0))
		fog.set_shader_parameter("foot", _between(FOOT, layer))
		fog.set_shader_parameter("rise", _between(RISE, layer))
		fog.set_shader_parameter("haze", FOG_HAZE)
		fog.set_shader_parameter("density", FOG_DENSITY)
		fog.set_shader_parameter("scale", FOG_SCALE)
		fog.set_shader_parameter("drift", FOG_DRIFT)
		fog.set_shader_parameter("paper", Ink.PAPER)
		child.material = fog
		add_child(child)
		child.set_anchors_preset(Control.PRESET_FULL_RECT)
		_ranges[layer] = child


## 第 layer 層山影的霧（著色器材質）。
func fog_of(layer: int) -> ShaderMaterial:
	return _ranges[layer].material


## 一道山稜鋪成連綿的山：每欄取鄰近最高的、乘上構圖的包絡，再墊上隨地形起伏的底。
static func _ranged(ridge: PackedFloat32Array) -> PackedFloat32Array:
	var size := ridge.size()
	var spread := PackedFloat32Array()
	spread.resize(size)
	var top := 0.0
	for column in size:
		for offset in range(-SPREAD, SPREAD + 1):
			# 離得越遠的山頭算得越矮，鋪開後還是山的形狀。
			spread[column] = maxf(spread[column], ridge[clampi(column + offset, 0, size - 1)] * (1.0 - 0.5 * absf(offset) / SPREAD))
		# 構圖的包絡：主山那一段全高，兩旁壓低；底墊在包絡之後，兩旁壓低了也不會斷。
		var across := column / (size - 1.0)
		var inside := smoothstep(MAIN_SPAN.x - MAIN_EDGE, MAIN_SPAN.x, across) * (1.0 - smoothstep(MAIN_SPAN.y, MAIN_SPAN.y + MAIN_EDGE, across))
		spread[column] *= lerpf(FLANK_LIFT, 1.0, inside)
		top = maxf(top, spread[column])
	var ranged := PackedFloat32Array()
	ranged.resize(size)
	for column in size:
		var sum := 0.0
		for offset in range(-SWELL, SWELL + 1):
			sum += spread[clampi(column + offset, 0, size - 1)]
		# 底本身也帶一點起伏，不是一條平線。
		var roll := 0.7 + 0.18 * sin(column * 0.093 + top * 40.0) + 0.12 * sin(column * 0.237 + top * 90.0) + 0.06 * sin(column * 0.51 + top * 170.0)
		ranged[column] = maxf(spread[column], maxf(sum / (SWELL * 2 + 1) * SWELL_LIFT, top * FLOOR * roll))
	return ranged


## 第 layer 層山腳線的高度（畫面座標）。
func foot_y(layer: int) -> float:
	return _between(FOOT, layer) * get_viewport_rect().size.y


## 第 layer 層第 column 欄山稜的位置（畫面座標）。
func crest(layer: int, column: int) -> Vector2:
	var bounds := get_viewport_rect().size
	return Vector2(bounds.x * column / (COLUMNS - 1.0), foot_y(layer) - _lift(layer, column) * _between(RISE, layer) * bounds.y)


## 主峰頂的位置（畫面座標）：各層山稜裡畫得最高的那一點。
func main_peak() -> Vector2:
	var peak := Vector2(0.0, INF)
	for layer in ridges.size():
		for column in COLUMNS:
			var at := crest(layer, column)
			if at.y < peak.y:
				peak = at
	return peak


## 畫面橫向 x（畫面座標）處山影的濃度倍率：左邊與中間是 1，標題與選單後面剩 1 − OPEN_FADE。
func tone_at(x: float) -> float:
	return 1.0 - OPEN_FADE * smoothstep(OPEN.x, OPEN.y, x / get_viewport_rect().size.x)


## 抹平並對這一景最高的山正規化後的高度（0..1）。
func _lift(layer: int, column: int) -> float:
	if _top <= 0.0:
		return 0.0
	var ridge := _smooth[layer]
	var at := column * (ridge.size() - 1.0) / (COLUMNS - 1.0)
	var low := int(at)
	var high := mini(low + 1, ridge.size() - 1)
	return lerpf(ridge[low], ridge[high], at - low) / _top


func _between(pair: Vector2, layer: int) -> float:
	return lerpf(pair.x, pair.y, layer / maxf(LAYERS - 1.0, 1.0))


func _draw() -> void:
	var bounds := get_viewport_rect().size
	var full := Rect2(Vector2.ZERO, bounds)
	draw_rect(full, Ink.PAPER)
	if Ink.paper_texture != null:
		draw_texture_rect_region(Ink.paper_texture, full, full, Ink.PAPER)


## 一層山影畫在它自己的節點上：山影，再疊皴。
func _draw_range(canvas: CanvasItem, layer: int) -> void:
	if layer >= ridges.size():
		return
	var bounds := get_viewport_rect().size
	if _meshes.is_empty() or _built_for != bounds:
		_build(bounds)
	canvas.draw_mesh(_meshes[layer], null)
	_texture_strokes(canvas, layer, bounds)


## 每層一片網格：山稜、中間、底三排頂點，墨色自上往下淡掉；山體不透明，紙紋不透上來。
func _build(bounds: Vector2) -> void:
	_built_for = bounds
	_meshes.clear()
	for layer in ridges.size():
		var tone := _between(TONE, layer)
		var rise := _between(RISE, layer) * bounds.y
		var points := PackedVector2Array()
		var colors := PackedColorArray()
		var indices := PackedInt32Array()
		for column in COLUMNS:
			var top := crest(layer, column)
			var lift := _lift(layer, column)
			var fall := maxf(rise * lift, FALL_MIN * bounds.y)
			# 高的山墨重一點，矮的淡。
			var ink := tone * lerpf(0.5, 1.15, lift) * tone_at(top.x)
			points.append(top)
			points.append(top + Vector2(0.0, fall * MID_AT))
			points.append(top + Vector2(0.0, fall))
			# 山體是蓋住紙紋的實色（紙色調進墨色），只有山腳那一排透明、沒入霧裡。
			colors.append(Color(Ink.PAPER.lerp(Ink.INK_LIGHT, ink), 1.0))
			colors.append(Color(Ink.PAPER.lerp(Ink.INK_LIGHT, ink * MID_TONE), 1.0))
			colors.append(Color(Ink.PAPER, 0.0))
			if column > 0:
				var at := column * 3
				for row in 2:
					indices.append_array(PackedInt32Array([at - 3 + row, at + row, at + row + 1, at - 3 + row, at + row + 1, at - 2 + row]))
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = points
		arrays[Mesh.ARRAY_COLOR] = colors
		arrays[Mesh.ARRAY_INDEX] = indices
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		_meshes.append(mesh)


## 高山上的皴：自山稜順著山坡往下的幾筆，越矮越短；山稜另勾一道淡線。
func _texture_strokes(canvas: CanvasItem, layer: int, bounds: Vector2) -> void:
	var tone := _between(TONE, layer)
	var rise := _between(RISE, layer) * bounds.y
	var width := maxf(bounds.y / 1080.0 * 1.4, 1.0)
	var line := PackedVector2Array()
	var shades := PackedColorArray()
	for column in COLUMNS:
		var top := crest(layer, column)
		var lift := _lift(layer, column)
		var fade := tone_at(top.x) * smoothstep(TEXTURE_ABOVE - 0.12, TEXTURE_ABOVE + 0.2, lift)
		line.append(top)
		shades.append(Color(Ink.INK, tone * 0.9 * fade))
		if lift < TEXTURE_ABOVE or column % 3 != 0 or column == 0 or column == COLUMNS - 1:
			continue
		# 往山坡低的那一邊斜下去。
		var lean := signf(_lift(layer, column - 1) - _lift(layer, column + 1))
		var reach := rise * lift * (0.10 + 0.05 * sin(column * 2.3 + layer))
		var from := top + Vector2(0.0, reach * (0.15 + 0.5 * absf(sin(column * 1.7))))
		canvas.draw_line(from, from + Vector2(lean * reach * 0.45, reach), Color(Ink.INK, tone * 0.3 * fade), width, true)
	canvas.draw_polyline_colors(line, shades, width, true)
