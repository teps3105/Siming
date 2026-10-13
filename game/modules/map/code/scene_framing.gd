class_name SceneFraming
extends RefCounted
## GDD 9.1 標題畫面的取景：在世界裡找一塊山、平地、水各佔一份、山在遠處、主山在中間偏左的地方。
## 把高度圖縮成粗格、分成山／平地／水三類並做成累計表，任何一塊矩形的三類比例都是幾次查表；
## 候選焦點鋪滿鏡頭可到的範圍，各算一個偏離目標的分數，再從分數最好的幾個裡隨機挑一個——
## 所以每次取的地方不同，構圖相近。取好景後，ridges 給出那一塊地由近到遠的幾道山稜剖面，背景照著畫成山影。

## 目標比例：x 山、y 平地、z 水。
const TARGET := Vector3(0.3, 0.4, 0.3)
## 取景時的鏡頭高度（地圖寬的比例）。
const VIEW_Z_FRAC := 0.2
## 相對高度（對全圖最高點）超過這個值算山。
const MOUNTAIN_ALT := 0.22
## 粗格的寬度上限（格）。
const GRID_W := 192
## 候選焦點的間距：畫面可見範圍的幾分之一。
const STEPS := 8.0
## 畫面最遠與最近各取這個比例的一帶，比較山的多寡。
const BAND := 0.45
## 遠處那一帶至少要有這個比例的山、近處那一帶至多這個比例，超出的部分計入分數。
const FAR_MOUNTAIN := 0.5
const NEAR_MOUNTAIN := 0.1
## 主山的位置：遠處那一帶裡，畫面寬度的這一段（左緣為 0、右緣為 1）至少要有這個比例的山；
## 右邊那一段（SIDE_WEST 到右緣，標題與選單後面）的山要比它少至少 MAIN_LEAD，主山才是眾山之主、右邊才開闊。
const MAIN_SPAN := Vector2(0.24, 0.56)
const MAIN_MOUNTAIN := 0.7
const SIDE_WEST := 0.68
const MAIN_LEAD := 0.4
## 主峰要是全景最高：主山那一段的最高點要比兩旁（左緣到 FLANK_EAST、SIDE_WEST 到右緣）的最高點高出 PEAK_LEAD（對全圖最高點）。
## 比高時橫向取 PEAK_COLUMNS 欄、縱向取 PEAK_ROWS 列。
const FLANK_EAST := 0.16
const PEAK_LEAD := 0.12
const PEAK_COLUMNS := 32
const PEAK_ROWS := 12
## 從分數最好的幾個候選裡隨機挑：最多這麼多個，且分數不比最好的差超過這個值。
const POOL := 10
const TOLERANCE := 0.08
## 取山稜剖面時，每一帶由近到遠取幾列來比高。
const RIDGE_ROWS := 14

var _w := 0
var _h := 0
## 一個粗格是幾個遊戲格。
var _cell := 1.0
## 累計表，(_w + 1) × (_h + 1)：左上到該格為止的山、水格數。
var _mountain := PackedInt32Array()
var _water := PackedInt32Array()
## 高度圖原樣留著（取山稜剖面用）：尺寸、一個遊戲格是幾個取樣、全圖最高點。
var _elevation := PackedFloat32Array()
var _size := Vector2i.ZERO
var _per_cell := 1.0
var _top := 0.0


func _init(world: SimWorld) -> void:
	var textures := world.map_textures()
	var size: Vector2i = textures["elevation_size"]
	var elevation: PackedFloat32Array = textures["elevation"]
	var lake: PackedByteArray = textures["lake"]
	var step := maxi(ceili(float(size.x) / GRID_W), 1)
	_w = size.x / step
	_h = size.y / step
	_cell = float(world.grid_size().x) / float(_w)
	var top := 0.0
	for value in elevation:
		top = maxf(top, value)
	_elevation = elevation
	_size = size
	_per_cell = float(size.x) / float(world.grid_size().x)
	_top = top
	_mountain.resize((_w + 1) * (_h + 1))
	_water.resize((_w + 1) * (_h + 1))
	for y in _h:
		var row_mountain := 0
		var row_water := 0
		for x in _w:
			var at := (y * step + step / 2) * size.x + x * step + step / 2
			var height := elevation[at]
			if height <= 0.0 or lake[at] >= 128:
				row_water += 1
			elif height > top * MOUNTAIN_ALT:
				row_mountain += 1
			var below := (y + 1) * (_w + 1) + x + 1
			_mountain[below] = _mountain[below - (_w + 1)] + row_mountain
			_water[below] = _water[below - (_w + 1)] + row_water


## 鏡頭高度 z、畫面寬高比 aspect 時，畫面在地面上涵蓋的範圍（東西向、南北向）。
static func ground_extent(z: float, aspect: float) -> Vector2:
	var view := MapView.view_height(z)
	return Vector2(view * aspect, view / sin(MapView.pitch()))


## 以 focus 為中心、涵蓋 extent 的畫面裡，山、平地、水各佔多少（x、y、z，合計為 1）。
func shares(focus: Vector2, extent: Vector2) -> Vector3:
	return _shares(focus.x - extent.x * 0.5, focus.x + extent.x * 0.5, focus.y - extent.y * 0.5, focus.y + extent.y * 0.5)


## 畫面最遠一帶與最近一帶裡山的比例（x 遠、y 近）。
func mountain_bands(focus: Vector2, extent: Vector2) -> Vector2:
	var west := focus.x - extent.x * 0.5
	var east := focus.x + extent.x * 0.5
	var north := focus.y - extent.y * 0.5
	var south := focus.y + extent.y * 0.5
	var far := _shares(west, east, north, north + extent.y * BAND)
	var near := _shares(west, east, south - extent.y * BAND, south)
	return Vector2(far.x, near.x)


## 畫面最遠一帶裡，主山那一段與右邊那一段山的比例（x 主山、y 右邊）。
func main_mountain(focus: Vector2, extent: Vector2) -> Vector2:
	var west := focus.x - extent.x * 0.5
	var north := focus.y - extent.y * 0.5
	var south := north + extent.y * BAND
	var main := _shares(west + extent.x * MAIN_SPAN.x, west + extent.x * MAIN_SPAN.y, north, south)
	var side := _shares(west + extent.x * SIDE_WEST, west + extent.x, north, south)
	return Vector2(main.x, side.x)


## 以 focus 為中心、涵蓋 extent 的畫面裡，由近到遠 count 道山稜的剖面：把地面由南（近）到北（遠）分成 count 帶，
## 每帶在畫面橫向的每一欄取最高的高度（0..1，對全圖最高點；水面是 0）。回傳由近到遠，每道 columns 個值、由西到東。
func ridges(focus: Vector2, extent: Vector2, count: int, columns: int) -> Array[PackedFloat32Array]:
	var found: Array[PackedFloat32Array] = []
	var west := focus.x - extent.x * 0.5
	var south := focus.y + extent.y * 0.5
	var depth := extent.y / count
	for band in count:
		var ridge := PackedFloat32Array()
		ridge.resize(columns)
		for column in columns:
			var x := posmod(int((west + extent.x * (column + 0.5) / columns) * _per_cell), _size.x)
			var best := 0.0
			for row in RIDGE_ROWS:
				var y := clampi(int((south - depth * (band + (row + 0.5) / RIDGE_ROWS)) * _per_cell), 0, _size.y - 1)
				best = maxf(best, _elevation[y * _size.x + x])
			ridge[column] = clampf(best / maxf(_top, 1e-6), 0.0, 1.0)
		found.append(ridge)
	return found


## 畫面裡主山那一段與兩旁的最高點（x 主山、y 兩旁；0..1，對全圖最高點）。
func peaks(focus: Vector2, extent: Vector2) -> Vector2:
	var west := focus.x - extent.x * 0.5
	var north := focus.y - extent.y * 0.5
	var found := Vector2.ZERO
	for column in PEAK_COLUMNS:
		var across := (column + 0.5) / PEAK_COLUMNS
		var main := across >= MAIN_SPAN.x and across < MAIN_SPAN.y
		if not main and across >= FLANK_EAST and across < SIDE_WEST:
			continue
		var x := posmod(int((west + extent.x * across) * _per_cell), _size.x)
		for row in PEAK_ROWS:
			var y := clampi(int((north + extent.y * (row + 0.5) / PEAK_ROWS) * _per_cell), 0, _size.y - 1)
			var height := _elevation[y * _size.x + x] / maxf(_top, 1e-6)
			if main:
				found.x = maxf(found.x, height)
			else:
				found.y = maxf(found.y, height)
	return found


## 偏離目標構圖的程度，越小越好。
func score(focus: Vector2, extent: Vector2) -> float:
	var got := shares(focus, extent)
	var bands := mountain_bands(focus, extent)
	var off := absf(got.x - TARGET.x) + absf(got.y - TARGET.y) + absf(got.z - TARGET.z)
	var main := main_mountain(focus, extent)
	var placed := maxf(MAIN_MOUNTAIN - main.x, 0.0) + maxf(main.y + MAIN_LEAD - main.x, 0.0)
	var high := peaks(focus, extent)
	var tallest := 2.0 * maxf(high.y + PEAK_LEAD - high.x, 0.0)
	return off + 0.5 * maxf(FAR_MOUNTAIN - bands.x, 0.0) + 0.5 * maxf(bands.y - NEAR_MOUNTAIN, 0.0) + placed + tallest


## 挑一個焦點：rows 是焦點的列座標範圍（MapView.focus_row_limits）。
func pick(extent: Vector2, rows: Vector2, rng: RandomNumberGenerator) -> Vector2:
	var width := _w * _cell
	var step := extent / STEPS
	var found: Array[Vector3] = []
	var y := rows.x
	while y <= rows.y:
		var x := 0.0
		while x < width:
			found.append(Vector3(x, y, score(Vector2(x, y), extent)))
			x += step.x
		y += step.y
	if found.is_empty():
		return Vector2(width * 0.5, (rows.x + rows.y) * 0.5)
	found.sort_custom(func(a: Vector3, b: Vector3) -> bool: return a.z < b.z)
	var pool := 1
	while pool < mini(POOL, found.size()) and found[pool].z <= found[0].z + TOLERANCE:
		pool += 1
	var chosen := found[rng.randi_range(0, pool - 1)]
	return Vector2(chosen.x, chosen.y)


## 世界座標的矩形（東西向可跨接縫）裡三類的比例。
func _shares(west: float, east: float, north: float, south: float) -> Vector3:
	var x0 := floori(west / _cell)
	var x1 := maxi(ceili(east / _cell), x0 + 1)
	var y0 := clampi(floori(north / _cell), 0, _h - 1)
	var y1 := clampi(ceili(south / _cell), y0 + 1, _h)
	var mountain := 0
	var water := 0
	# 東西向繞一圈：拆成不跨接縫的幾段。
	var x := x0
	while x < x1:
		var from := posmod(x, _w)
		var count := mini(x1 - x, _w - from)
		mountain += _sum(_mountain, from, y0, from + count, y1)
		water += _sum(_water, from, y0, from + count, y1)
		x += count
	var total := float((x1 - x0) * (y1 - y0))
	return Vector3(mountain / total, (total - mountain - water) / total, water / total)


func _sum(table: PackedInt32Array, x0: int, y0: int, x1: int, y1: int) -> int:
	var stride := _w + 1
	return table[y1 * stride + x1] - table[y0 * stride + x1] - table[y1 * stride + x0] + table[y0 * stride + x0]
