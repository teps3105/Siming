class_name PartCatalog
extends RefCounted
## 部件目錄（GDD 8.12）：讀某一類的 catalog.json 與圖集，回答「這一格有哪些款、各在圖集的哪裡、定位點在哪」。
## 圖集 atlas.png 以原始位元組讀入（不走匯入管線，匯入設定為 keep），RGB 已預乘 alpha。

const ROOT := "res://modules/parts/resources/"

static var _cache := {}

var class_id := ""
var canvas := Vector2i(1, 1)
var pivot := Vector2.ZERO
var halo := {}
## 這一類目前是不是程式畫的佔位部件（目錄的 `placeholder`）。
var placeholder := false
var channels: Array = []
var _slots: Array = []
var _parts := {}
var _atlas_path := ""
var _image: Image
var _texture: ImageTexture
var _regions := {}


static func load_class(id: String) -> PartCatalog:
	if _cache.has(id):
		return _cache[id]
	var catalog := PartCatalog.new()
	if not catalog._read(id):
		return null
	_cache[id] = catalog
	return catalog


func _read(id: String) -> bool:
	var path := ROOT + id + "/catalog.json"
	var data: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not data is Dictionary:
		push_error("部件目錄讀不到：%s" % path)
		return false
	class_id = id
	canvas = Vector2i(int(data["canvas"][0]), int(data["canvas"][1]))
	pivot = Vector2(float(data["pivot"][0]), float(data["pivot"][1]))
	halo = data.get("halo", {})
	placeholder = bool(data.get("placeholder", false))
	channels = data.get("channels", [])
	_slots = data["slots"]
	_atlas_path = ROOT + id + "/" + String(data["atlas"])
	for code: String in data["parts"]:
		var layers: Array[Dictionary] = []
		for raw: Dictionary in data["parts"][code]["layers"]:
			var rect: Array = raw["rect"]
			var offset: Array = raw["offset"]
			layers.append({
				"rect": Rect2i(int(rect[0]), int(rect[1]), int(rect[2]), int(rect[3])),
				"offset": Vector2(float(offset[0]), float(offset[1])),
				"mode": String(raw["mode"]),
				"tint": String(raw["tint"]) if raw["tint"] != null else "",
			})
		var anchor: Array = data["parts"][code]["anchor"]
		var hairs: Array[String] = []
		for style: Variant in data["parts"][code].get("hairs", []):
			hairs.append(String(style))
		_parts[code] = {"layers": layers, "anchor": Vector2(float(anchor[0]), float(anchor[1])), "hairs": hairs}
	return true


## 格的名稱，由後往前。
func slots() -> Array[String]:
	var ordered: Array = _slots.duplicate()
	ordered.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return int(a["order"]) < int(b["order"]))
	var out: Array[String] = []
	for slot: Dictionary in ordered:
		out.append(String(slot["id"]))
	return out


func slot_order(slot: String) -> int:
	for s: Dictionary in _slots:
		if s["id"] == slot:
			return int(s["order"])
	return -1


func slot_count(slot: String) -> int:
	for s: Dictionary in _slots:
		if s["id"] == slot:
			return int(s["count"])
	return 0


static func slot_of(code: String) -> String:
	return code.get_slice("/", 1).get_slice(".", 0)


## 某一格的全部款（部件代號）。
func parts_in(slot: String) -> Array[String]:
	var out: Array[String] = []
	for code: String in _parts:
		if slot_of(code) == slot:
			out.append(code)
	return out


func has_part(code: String) -> bool:
	return _parts.has(code)


func part_count() -> int:
	return _parts.size()


## 一款由後往前的子層：rect（圖集位置）、offset（在畫布上的左上角）、mode（keyed／fixed）、tint（染色欄位名，定色為空）。
func layers_of(code: String) -> Array[Dictionary]:
	return _parts[code]["layers"]


func anchor_of(code: String) -> Vector2:
	return _parts[code]["anchor"]


## 一款冠帽可搭配的髮式（目錄的 `hairs`，款名同前後髮的末段，如 `crown`）；沒有宣告者為空。
func hairs_of(code: String) -> Array[String]:
	return _parts[code]["hairs"]


func atlas_image() -> Image:
	if _image == null:
		_image = Image.new()
		var err := _image.load_png_from_buffer(FileAccess.get_file_as_bytes(_atlas_path))
		if err != OK:
			push_error("部件圖集讀不到：%s" % _atlas_path)
			_image = Image.create(1, 1, false, Image.FORMAT_RGBA8)
		_image.convert(Image.FORMAT_RGBA8)
	return _image


func atlas_texture() -> ImageTexture:
	if _texture == null:
		var copy := atlas_image().duplicate() as Image
		copy.generate_mipmaps()
		_texture = ImageTexture.create_from_image(copy)
	return _texture


## 一個子層在圖集上的區域貼圖。
func region_texture(layer: Dictionary) -> AtlasTexture:
	var rect: Rect2i = layer["rect"]
	var key := str(rect)
	if not _regions.has(key):
		var tex := AtlasTexture.new()
		tex.atlas = atlas_texture()
		tex.region = Rect2(rect)
		_regions[key] = tex
	return _regions[key]
