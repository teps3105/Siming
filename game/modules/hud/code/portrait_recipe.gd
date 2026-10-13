class_name PortraitRecipe
extends RefCounted
## 修士頭像的組合規則（GDD 8.12、5.8）：人物資料（橋接 person()）→ 一串部件代號與染色（PartStack 的 recipe）。純函式：
## 同樣的資料一定回同一串；存檔不記這一串，由資料當場推得。
##
## 樣子取自 CultivatorFigure.traits()（種子是人物編號，性別、門派、貧富由資料指定或由種子取），
## 年紀一律以「成年」取，髮式與膚髮衣色因此不隨年紀換；年紀只由壽元用掉的比例決定少、壯、老三段：
## 少——無鬚、臉略縮；壯——原樣；老——疊皺紋、髮鬚轉白、無鬚者可能新留鬚。
## 臉型、口鼻、眉由人物編號取，眼由魂的種子取（同一個魂轉世，眼不變）。
## 入魔（role == Demonic）疊面紋、瞳色改 violet、整張偏紫；身故整張褪成淡墨，不換部件。境界光暈圈數由境界或果位定。

const CLASS_ID := "portrait"
## 壽元用掉的比例低於 YOUNG_BELOW 是少，達 ELDER_FROM 是老。
const YOUNG_BELOW := 0.22
const ELDER_FROM := 0.62
## 少者臉部各格縮小的比例。
const FACE_SHRINK := 0.94
## 老者髮色往灰白靠的程度。
const ELDER_WHITE := 0.85
const HAIR_WHITE := Color("c9c9c4")
const VIOLET := Color("6B4A8E")
const IRIS_VIOLET := Color("7A4FA3")
const IRISES: Array[Color] = [Color("3b2a22"), Color("2f3b36"), Color("3a3a4a"), Color("55402a")]
const SCHOOLS: Array[String] = ["none", "ru", "shi", "dao"]
const WEALTH: Array[String] = ["poor", "plain", "rich"]
const HAIRS: Array[String] = ["topknot", "crown", "loose", "hat", "twin_buns", "high_bun", "shaven", "scholar_cap"]
const BEARDS: Array[String] = ["moustache", "goatee", "three", "five", "full"]
## 境界光暈最多幾圈。
const MAX_RINGS := 9

enum Stage { YOUNG, PRIME, ELDER }


## 少、壯、老：孩童恆為少，其餘看壽元用掉的比例。
static func stage(info: Dictionary) -> int:
	if info.get("role", "") == "Child":
		return Stage.YOUNG
	var lifespan := float(info.get("lifespan", 0))
	if lifespan <= 0.0:
		return Stage.PRIME
	var used := float(info.get("age", 0)) / lifespan
	if used < YOUNG_BELOW:
		return Stage.YOUNG
	return Stage.ELDER if used >= ELDER_FROM else Stage.PRIME


## 橋接資料換成 traits() 的指定項：性別、門派、貧富（只有身分明確的才指定，其餘由種子取）。
static func given_of(info: Dictionary) -> Dictionary:
	var given := {
		"sex": CultivatorFigure.Sex.FEMALE if info.get("gender", "Male") == "Female" else CultivatorFigure.Sex.MALE,
		"age": CultivatorFigure.Age.ADULT,
	}
	match info.get("role", ""):
		"Monk":
			given["school"] = CultivatorFigure.School.SHI
		"Scholar", "Official":
			given["school"] = CultivatorFigure.School.RU
		"Cultivator":
			given["school"] = CultivatorFigure.School.DAO if not String(info.get("power_name", "")).is_empty() else CultivatorFigure.School.NONE
		"Child", "Demonic", "Commoner", "Ruler", "Royal", "General", "GeneralCandidate":
			given["school"] = CultivatorFigure.School.NONE
	match info.get("role", ""):
		"Ruler", "Royal":
			given["wealth"] = CultivatorFigure.Wealth.RICH
		"Commoner":
			given["wealth"] = CultivatorFigure.Wealth.POOR
		_:
			if info.get("is_leader", false):
				given["wealth"] = CultivatorFigure.Wealth.RICH
	return given


static func _pick(seed_value: int, salt: int, count: int) -> int:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value * 7919 + salt * 104729 + 17
	return int(rng.randi() % count)


## 境界或果位換成光暈圈數；沒有境界者為 0。
static func halo_rings(info: Dictionary) -> int:
	if int(info.get("realm", -1)) >= 0:
		return mini(int(info["realm"]) + 1, MAX_RINGS)
	if int(info.get("fruit", -1)) >= 0:
		return mini(int(info["fruit"]) + 1, MAX_RINGS)
	return 0


static func from_person(info: Dictionary) -> Dictionary:
	var id := int(info.get("id", 0))
	var soul := int(info.get("soul", id))
	var given := given_of(info)
	var look := CultivatorFigure.traits(id, given)
	var colors: Dictionary = look["colors"]
	var life_stage := stage(info)
	var demon: bool = info.get("role", "") == "Demonic"
	var dead: bool = not info.get("alive", true)
	var female: bool = look["sex"] == CultivatorFigure.Sex.FEMALE
	var sex_letter := "f" if female else "m"

	var whiskers: int = look["whiskers"]
	if life_stage == Stage.ELDER and whiskers == CultivatorFigure.Whiskers.NONE and not female and look["school"] != CultivatorFigure.School.SHI:
		var aged := given.duplicate()
		aged["age"] = CultivatorFigure.Age.ELDER
		whiskers = CultivatorFigure.traits(id, aged)["whiskers"]
	if life_stage == Stage.YOUNG:
		whiskers = CultivatorFigure.Whiskers.NONE

	var hair_color: Color = colors[CultivatorFigure.HAIR]
	if life_stage == Stage.ELDER:
		hair_color = hair_color.lerp(HAIR_WHITE, ELDER_WHITE)
	var iris: Color = IRIS_VIOLET if demon else IRISES[_pick(soul, 4, IRISES.size())]
	var lip := Color(colors[CultivatorFigure.LIP], 1.0)
	var tints := {
		"hair": hair_color,
		"skin": colors[CultivatorFigure.SKIN],
		"robe": colors[CultivatorFigure.ROBE],
		"trim": colors[CultivatorFigure.TRIM],
		"iris": iris,
		"lip": lip,
		"violet": VIOLET,
		"gold": PartTint.GOLD,
	}
	var shrink := FACE_SHRINK if life_stage == Stage.YOUNG else 1.0
	var hair_name: String = HAIRS[look["hair"]]
	var items: Array[Dictionary] = []
	items.append(_item("back_hair." + hair_name, ["hair"], tints))
	items.append(_item("collar.%s_%s" % [SCHOOLS[look["school"]], WEALTH[look["wealth"]]], ["robe", "trim", "gold"], tints))
	items.append(_item("face.%s%d" % [sex_letter, _pick(id, 1, 3)], ["skin"], tints, shrink))
	if life_stage == Stage.ELDER:
		items.append(_item("wrinkle.%d" % _pick(id, 5, 2), [], tints, shrink))
	items.append(_item("mouth.%d" % _pick(id, 2, 6), ["skin", "lip"], tints, shrink))
	items.append(_item("eyes.%s%d" % [sex_letter, _pick(soul, 3, 4)], ["iris"], tints, shrink))
	items.append(_item("brows.%d" % _pick(id, 6, 6), ["hair"], tints, shrink))
	if whiskers != CultivatorFigure.Whiskers.NONE:
		items.append(_item("beard." + BEARDS[whiskers - 1], ["hair"], tints))
	if demon:
		items.append(_item("mark.%d" % _pick(id, 7, 3), ["violet"], tints))
	items.append(_item("front_hair." + hair_name, ["hair"], tints))
	var hat_kind: int = {CultivatorFigure.Hair.HAT: 0, CultivatorFigure.Hair.CROWN: 1, CultivatorFigure.Hair.SCHOLAR_CAP: 2}.get(look["hair"], -1)
	if hat_kind >= 0:
		var hat_tints := tints.duplicate()
		hat_tints["robe"] = colors[CultivatorFigure.HAT]
		var hat_names := ["straw", "crown", "scholar"]
		var rich := 1 if look["wealth"] == CultivatorFigure.Wealth.RICH else 0
		items.append(_item("hat.%s%d" % [hat_names[hat_kind], rich], ["robe", "gold"], hat_tints))

	var grade := PartTint.no_grade()
	if demon:
		grade["wash"] = VIOLET
		grade["wash_amount"] = 0.22
	if dead:
		grade["desat"] = 0.75
		grade["fade"] = 0.5
	return {"items": items, "grade": grade, "halo": halo_rings(info)}


## 一款的代號與它用到的染色欄位（只留用到的，recipe 才不帶多餘的字）。
static func _item(code: String, channels: Array, tints: Dictionary, scale := 1.0) -> Dictionary:
	var used := {}
	for channel: String in channels:
		used[channel] = tints[channel]
	var item := {"part": "%s/%s" % [CLASS_ID, code], "tints": used}
	if scale != 1.0:
		item["scale"] = scale
	return item


## 這一串的部件代號（依序），測試與除錯用。
static func codes(recipe: Dictionary) -> Array[String]:
	var out: Array[String] = []
	for item: Dictionary in recipe["items"]:
		out.append(item["part"])
	return out


static func key(recipe: Dictionary) -> String:
	return PartStack.recipe_key(recipe)
