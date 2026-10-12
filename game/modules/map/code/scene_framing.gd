class_name SceneFraming
extends RefCounted
## GDD 9.1 標題畫面的取景：在世界裡找一塊山、平地、水各佔一份、山在遠處的地方。
## 把高度圖縮成粗格、分成山／平地／水三類並做成累計表，任何一塊矩形的三類比例都是幾次查表；
## 候選焦點鋪滿鏡頭可到的範圍，各算一個偏離目標的分數，再從分數最好的幾個裡隨機挑一個——
## 所以每次取的地方不同，構圖相近。

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
## 從分數最好的幾個候選裡隨機挑：最多這麼多個，且分數不比最好的差超過這個值。
const POOL := 10
const TOLERANCE := 0.08

var _w := 0
var _h := 0
## 一個粗格是幾個遊戲格。
var _cell := 1.0
## 累計表，(_w + 1) × (_h + 1)：左上到該格為止的山、水格數。
var _mountain := PackedInt32Array()
var _water := PackedInt32Array()


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


## 偏離目標構圖的程度，越小越好。
func score(focus: Vector2, extent: Vector2) -> float:
	var got := shares(focus, extent)
	var bands := mountain_bands(focus, extent)
	var off := absf(got.x - TARGET.x) + absf(got.y - TARGET.y) + absf(got.z - TARGET.z)
	return off + 0.5 * maxf(FAR_MOUNTAIN - bands.x, 0.0) + 0.5 * maxf(bands.y - NEAR_MOUNTAIN, 0.0)


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
