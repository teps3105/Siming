class_name TerrainSwatches
extends RefCounted
## 地形類別的代表色（GDD 5.1「地形圖設色」）：例簽與命簿的色塊共用這一套。

const COLORS := {
	"terrain_plains": Color("BAC690"),
	"terrain_hills": Color("C9B58C"),
	"terrain_mountain": Color("A49D97"),
	"terrain_forest": Color("7AA06C"),
	"terrain_desert": Color("D9C59A"),
	"terrain_snow": Color("F1F0EA"),
	"terrain_swamp": Color("8FA287"),
	"terrain_water": Color("A3C3C4"),
}
## 模擬的地形代碼（0 海 .. 8 澤）對應的字串鍵。
const KEYS: Array[String] = [
	"terrain_water", "terrain_water", "terrain_plains", "terrain_hills", "terrain_mountain",
	"terrain_forest", "terrain_snow", "terrain_desert", "terrain_swamp",
]


static func color(key: String) -> Color:
	return COLORS[key]


## 色塊：底色加淡墨細框。
static func chip(item: CanvasItem, rect: Rect2, key: String) -> void:
	item.draw_rect(rect, COLORS[key], true)
	item.draw_rect(rect, Color(Ink.INK_LIGHT, 0.6), false, 1.0)
