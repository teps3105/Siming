extends Control
## 遊戲入口。目前只裝配模擬並顯示一張地形預覽，用來確認 Rust 橋接可用；
## 正式的地圖與介面由 map、hud 模組接手（GDD 6.4）。

const PREVIEW_SEED := 20261011
const PREVIEW_SIZE := "small"

## GDD 5.1 生態底色的近似值，僅供預覽。索引為地形代碼（GDD 2.2）。
const TERRAIN_COLORS: Array[Color] = [
	Color("5f7f8f"), # 海
	Color("7fa3ad"), # 湖
	Color("c9c08f"), # 平原
	Color("b89b6a"), # 丘陵
	Color("7d7468"), # 山
	Color("6f8a5c"), # 林
	Color("e8e6df"), # 雪
	Color("d8c59a"), # 漠
	Color("7f8f76"), # 澤
]
const RIVER_COLOR := Color("3f5f73")

@onready var _sim: SimHost = $SimHost
@onready var _map: TextureRect = $Map
@onready var _status: Label = $Status


func _ready() -> void:
	var error := _sim.create_world(PREVIEW_SEED, PREVIEW_SIZE)
	if not error.is_empty():
		_status.text = "世界生成失敗：%s" % error
		return
	_map.texture = ImageTexture.create_from_image(_terrain_image(_sim.world))
	_sim.advanced.connect(_refresh_status.unbind(1))
	_refresh_status()


func _refresh_status() -> void:
	var world := _sim.world
	_status.text = "%s　州郡 %d　人口 %d　有名之人 %d　雜湊 %s" % [
		world.date_text(),
		world.province_count(),
		int(world.total_population()),
		world.alive_person_count(),
		world.hash_hex(),
	]


func _terrain_image(world: SimWorld) -> Image:
	var size := world.map_size()
	var heights := world.height_map()
	var terrain := world.terrain_map()
	var rivers := world.river_map()
	var sea := world.sea_level()
	var image := Image.create_empty(size.x, size.y, false, Image.FORMAT_RGB8)
	for y in size.y:
		for x in size.x:
			var i := y * size.x + x
			var color := TERRAIN_COLORS[terrain[i]]
			if terrain[i] > 1:
				color = color.darkened(clampf((heights[i] - sea) * 0.6, 0.0, 0.4))
				if rivers[i] > 0:
					color = RIVER_COLOR
			image.set_pixel(x, y, color)
	return image
