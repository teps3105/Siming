class_name PartTint
extends RefCounted
## 部件染色的公式（GDD 8.12）：明度鍵的灰階 → 墨、陰影、底色、高光四個停點；整張圖的色調（grade）。
## 著色器（part_tint.gdshader）與 CPU 合成（PartCompose）用同一條公式。

const INK := Color("2B2620")
const PAPER := Color("EDE4CF")
## 境界光暈的金色（與 Ink.GOLD 同色）。
const GOLD := Color("C9A246")
## 陰影往墨靠、高光往紙白靠的程度。
const SHADOW_MIX := 0.28
const LIGHT_MIX := 0.38


## 由底色導出 [陰影, 底色, 高光]。
static func ramp(base: Color) -> Array[Color]:
	var shadow := base.lerp(INK, SHADOW_MIX)
	var light := base.lerp(Color("FBF8EE"), LIGHT_MIX)
	shadow.a = 1.0
	light.a = 1.0
	return [shadow, Color(base, 1.0), light]


## 染色參數：Color（底色，其餘自動導出）或 [陰影, 底色, 高光]。
static func ramp_of(tint: Variant) -> Array[Color]:
	if tint is Color:
		return ramp(tint)
	if tint is Array and tint.size() == 3:
		var out: Array[Color] = [tint[0], tint[1], tint[2]]
		return out
	return ramp(Color.WHITE)


## 明度鍵的灰階值（0..1）換成顏色：stops 是 [墨, 陰影, 底色, 高光]。
static func key_color(value: float, stops: Array[Color]) -> Color:
	var p := clampf(value, 0.0, 1.0) * 3.0
	var i := mini(int(p), 2)
	return stops[i].lerp(stops[i + 1], p - float(i))


static func stops_of(tint: Variant) -> Array[Color]:
	var r := ramp_of(tint)
	var out: Array[Color] = [INK, r[0], r[1], r[2]]
	return out


## 不調色的整張圖色調。desat 去飽和、wash／wash_amount 偏色、fade 往紙色褪。
static func no_grade() -> Dictionary:
	return {"desat": 0.0, "wash": Color(0, 0, 0, 1), "wash_amount": 0.0, "fade": 0.0}


static func grade(color: Color, g: Dictionary) -> Color:
	var lum := color.r * 0.299 + color.g * 0.587 + color.b * 0.114
	var c := color.lerp(Color(lum, lum, lum, color.a), float(g.get("desat", 0.0)))
	c = c.lerp(Color(g.get("wash", Color.BLACK), color.a), float(g.get("wash_amount", 0.0)))
	c = c.lerp(Color(PAPER, color.a), float(g.get("fade", 0.0)))
	return c
