class_name CultivatorFigure
extends RefCounted
## 修士剪影演算法（GDD 5.8）：一個種子決定一名修士的樣子（traits），姿勢另給（Pose），
## 兩者合起來算出剪影——同一名修士換姿勢仍認得出來，換種子就是另一個人。
## 座標以成人站立身高為 1：原點在腳下（坐姿是坐處），往上是 −y；人朝 −x 那一邊、身體半轉向看畫的人
## （看得到胸前與兩隻手，臉是七分面）；衣袖、髮帶、衣擺隨 time 擺動。
##
## 做法分三步：
## 一、定身分——性別、年紀（少、壯、老）、貧富（貧、常、富）、門派（散修、儒、釋、道）；可由種子決定，也可指定。
##     男女老少富貴貧賤與儒釋道都由這四項組合出來。
## 二、身分決定樣子——幾頭身與身高、肩腰寬、袖長與衣擺、髮式、鬚、持物、有無大氅與佩玉。
## 三、依姿勢的關節（腰、頸、肘、手）把各部位接上去：每個部位先給幾個控制點，再以 Catmull-Rom 曲線
##     補成圓順的輪廓（shapes：多邊形）；衣領、腰帶、衣褶、袖口這些線另外算（folds：折線）。
## 畫法是白描設色：部位由後往前一層層畫，每一層先把自己的多邊形併成一塊、填上這個部位的顏色蓋住後面
## （由上往下漸深），疊上色帶（領緣、腰帶、下襬的緣邊、衣褶的暗面、腮紅），再以墨線勾邊、畫衣紋。
## 顏色由身分決定範圍、種子取值：門派定衣色的色系，貧富定鮮濁與有無緣邊，年紀定髮色。

enum Sex { MALE, FEMALE }
enum Age { CHILD, ADULT, ELDER }
enum Wealth { POOR, PLAIN, RICH }
enum School { NONE, RU, SHI, DAO }
## 鬍鬚的樣式：無、八字髭、山羊鬚（頦下一綹）、三綹（頦下一綹加兩頰各一綹）、五綹（三綹加兩撇髭）、絡腮（沿著下頷一圈短鬚）。
enum Whiskers { NONE, MOUSTACHE, GOATEE, THREE, FIVE, FULL }
enum Hair { TOPKNOT, CROWN, LOOSE, HAT, TWIN_BUNS, HIGH_BUN, SHAVEN, SCHOLAR_CAP }
enum Prop { NONE, SWORD_BACK, WHISK, GOURD, STAFF, FAN, FLUTE, LOTUS, BASKET, TABLET, BEADS, SCROLL }
enum Pose { STAND, GESTURE, PLAY, MEDITATE, RIDE }

## 圓以幾邊形近似，曲線每段補幾點。
const ROUND := 16
const CURVE_STEPS := 4
## 貝茲曲線每一段切成幾小段。
const BEZIER_STEPS := 8
## 部件的曲線資料：每個部件一個 JSON 檔。
const PARTS_DIR := "res://modules/ui_kit/resources/figure/"
## 一層或一道色帶是哪個部位（顏色查 traits 的 colors）；一層加上 OPEN 表示不沿輪廓勾線
## （線由這一層自己給，用在接著身體、不該整圈勾邊的袖子）。
const SKIN := 0
const ROBE := 1
const INNER := 2
const SASH := 3
const TRIM := 4
const CAPE := 5
const HAIR := 6
const BEARD := 7
const HAT := 8
const PROP := 9
const SHOE := 10
const SHADE := 11
const BLUSH := 12
const LIP := 13
const SKIRT := 14
const SHAWL := 15
const OPEN := 32
## 曲線資料檔裡的部位名稱。
const PART_NAMES := {
	"SKIN": SKIN, "ROBE": ROBE, "INNER": INNER, "SASH": SASH, "TRIM": TRIM, "CAPE": CAPE, "HAIR": HAIR,
	"BEARD": BEARD, "HAT": HAT, "PROP": PROP, "SHOE": SHOE, "SHADE": SHADE, "BLUSH": BLUSH, "LIP": LIP,
	"SKIRT": SKIRT, "SHAWL": SHAWL,
}
## 前後層次，由後往前。每一塊畫在哪一層寫在這張表上，不靠程式裡出現的先後：
## 身後的持物、大氅、披帛、朝前那隻手的上臂、腿腳、袍、垂穗、腦後（披髮與飄帶）、頸、衣領（圍在頸根外面）、臉、
## 頷下（冠纓、笠帶、鬢邊垂髮）、鬍鬚、頭髮、冠帽、橫在身前的前臂、靠外那隻手的持物、靠外那隻手、朝前那隻手的持物。
enum Layer { BACK_PROP, CAPE, SHAWL, FAR_ARM, LEGS, ROBE, TASSEL, BEHIND_HEAD, NECK, COLLAR, FACE, UNDER_CHIN, BEARD, HAIR, HEADWEAR, FOREARM, NEAR_PROP, NEAR_ARM, FRONT_PROP }
## 各部位由上往下加深多少。
const DEPTH := {SKIN: 0.05, ROBE: 0.2, CAPE: 0.24, HAIR: 0.0, SHOE: 0.0, SKIRT: 0.24, SHAWL: 0.04}
## 背在身後或掛在腰後的持物；其餘拿在手上，畫在身前。
const BEHIND_PROPS: Array[int] = [Prop.SWORD_BACK, Prop.GOURD]
## 各姿勢的關節：腰、頸，朝前那隻手（_f，離看畫的人遠）與靠外那隻手（_b，離看畫的人近）的肘與手；
## seated 是坐姿，wind 是衣擺被風吹開的程度，point 是朝前那隻手並指（指點、掐訣）。
const POSES := {
	Pose.STAND: {
		"hip": Vector2(0.02, -0.50), "neck": Vector2(0.0, -0.84),
		"elbow_f": Vector2(-0.06, -0.66), "hand_f": Vector2(-0.015, -0.52),
		"elbow_b": Vector2(0.09, -0.66), "hand_b": Vector2(0.07, -0.51), "seated": false, "wind": 0.05, "point": false,
	},
	Pose.GESTURE: {
		"hip": Vector2(0.02, -0.50), "neck": Vector2(-0.01, -0.84),
		"elbow_f": Vector2(-0.15, -0.70), "hand_f": Vector2(-0.31, -0.79),
		"elbow_b": Vector2(0.09, -0.66), "hand_b": Vector2(0.07, -0.51), "seated": false, "wind": 0.06, "point": true,
	},
	Pose.PLAY: {
		"hip": Vector2(0.05, -0.10), "neck": Vector2(-0.03, -0.44),
		"elbow_f": Vector2(-0.17, -0.31), "hand_f": Vector2(-0.35, -0.22),
		"elbow_b": Vector2(0.12, -0.26), "hand_b": Vector2(0.03, -0.14), "seated": true, "wind": 0.02, "point": true,
	},
	Pose.MEDITATE: {
		"hip": Vector2(0.02, -0.10), "neck": Vector2(0.01, -0.47),
		"elbow_f": Vector2(-0.13, -0.27), "hand_f": Vector2(-0.05, -0.15),
		"elbow_b": Vector2(0.15, -0.27), "hand_b": Vector2(0.07, -0.15), "seated": true, "wind": 0.02, "point": false,
	},
	Pose.RIDE: {
		"hip": Vector2(0.03, -0.48), "neck": Vector2(-0.05, -0.81),
		"elbow_f": Vector2(-0.17, -0.71), "hand_f": Vector2(-0.33, -0.64),
		"elbow_b": Vector2(0.11, -0.69), "hand_b": Vector2(0.23, -0.60), "seated": false, "wind": 0.24, "point": true,
	},
}


## 一名修士的特徵。身分四項（sex、age、wealth、school）由種子決定，given 裡有指定的照指定；
## 其餘由身分決定範圍、再由種子取值：
## heads 幾頭身、stature 身高倍數、shoulder 半肩寬、girth 半腰寬、flare 衣擺半寬、sleeve 袖長（窄袖到廣袖）、
## hair 髮式、beard 鬚長（0 是無鬚）、whiskers 鬍鬚的樣式、prop 持物、ribbon 髮帶長、nose 鼻的高低、pleats 衣褶數、
## cape 是否披大氅、stoop 駝背的程度、swing 重心偏哪一腳（−1 到 1）、tilt 頭的傾斜（弧度）、
## eyes 眼尾的高低、jaw 臉的長短。given 也可以直接指定這些（如指定髮式或持物）。
static func traits(figure_seed: int, given := {}) -> Dictionary:
	var rng := RandomNumberGenerator.new()
	# 相鄰的種子直接當亂數種子，頭幾個取值會很像；先打散。
	rng.seed = hash(figure_seed * 2654435761)
	# 每一項都先取亂數再套指定值：指定了某一項，其他項的取值不跟著變。
	var sex: int = _pick(rng, [Sex.MALE, Sex.MALE, Sex.MALE, Sex.FEMALE, Sex.FEMALE])
	var age: int = _pick(rng, [Age.CHILD, Age.ADULT, Age.ADULT, Age.ADULT, Age.ADULT, Age.ELDER, Age.ELDER])
	var wealth: int = _pick(rng, [Wealth.POOR, Wealth.PLAIN, Wealth.PLAIN, Wealth.RICH])
	var school: int = _pick(rng, [School.NONE, School.NONE, School.RU, School.SHI, School.DAO, School.DAO])
	sex = given.get("sex", sex)
	age = given.get("age", age)
	wealth = given.get("wealth", wealth)
	school = given.get("school", school)
	var female := sex == Sex.FEMALE
	var child := age == Age.CHILD
	var elder := age == Age.ELDER
	var poor := wealth == Wealth.POOR
	var rich := wealth == Wealth.RICH

	var hair_pool: Array = [Hair.TOPKNOT, Hair.LOOSE, Hair.HAT]
	if school == School.SHI:
		hair_pool = [Hair.SHAVEN]
	elif child:
		hair_pool = [Hair.TWIN_BUNS, Hair.TWIN_BUNS, Hair.TOPKNOT]
	elif female:
		hair_pool = [Hair.HIGH_BUN, Hair.HIGH_BUN, Hair.HIGH_BUN, Hair.LOOSE] if not poor else [Hair.HIGH_BUN, Hair.LOOSE, Hair.HAT]
	elif school == School.RU:
		hair_pool = [Hair.SCHOLAR_CAP, Hair.SCHOLAR_CAP, Hair.CROWN] if not poor else [Hair.SCHOLAR_CAP, Hair.TOPKNOT]
	elif school == School.DAO:
		hair_pool = [Hair.CROWN, Hair.TOPKNOT] if rich else [Hair.TOPKNOT, Hair.TOPKNOT, Hair.LOOSE]
	elif rich:
		hair_pool = [Hair.CROWN, Hair.CROWN, Hair.TOPKNOT]
	elif poor:
		hair_pool = [Hair.HAT, Hair.LOOSE, Hair.TOPKNOT]

	var prop_pool: Array = [Prop.SWORD_BACK, Prop.FLUTE, Prop.GOURD, Prop.NONE]
	if school == School.SHI:
		prop_pool = [Prop.BEADS, Prop.BEADS, Prop.STAFF, Prop.NONE]
	elif school == School.RU:
		prop_pool = [Prop.FAN, Prop.SCROLL, Prop.TABLET] if rich else [Prop.FAN, Prop.SCROLL, Prop.SCROLL, Prop.NONE]
	elif school == School.DAO:
		prop_pool = [Prop.WHISK, Prop.SWORD_BACK, Prop.GOURD, Prop.WHISK]
	elif child:
		prop_pool = [Prop.FLUTE, Prop.LOTUS, Prop.BASKET, Prop.NONE]
	elif poor:
		prop_pool = [Prop.STAFF, Prop.GOURD, Prop.BASKET, Prop.NONE]
	elif rich:
		prop_pool = [Prop.FAN, Prop.TABLET, Prop.FLUTE, Prop.LOTUS]
	elif female:
		prop_pool = [Prop.LOTUS, Prop.FLUTE, Prop.SWORD_BACK, Prop.BASKET, Prop.FAN]
	if elder and school != School.RU:
		prop_pool.append(Prop.STAFF)

	var beard_chance := 0.0
	if not female and not child:
		beard_chance = (0.9 if elder else 0.3) * (0.5 if school == School.SHI else 1.0)
	var slim := 0.86 if female or child else 1.0
	var look := {
		"sex": sex, "age": age, "wealth": wealth, "school": school,
		"heads": rng.randf_range(4.6, 5.3) if child else rng.randf_range(6.5, 7.6),
		"stature": rng.randf_range(0.60, 0.70) if child else rng.randf_range(0.90, 0.98) * (1.0 if not elder else 0.97) if female else rng.randf_range(0.94, 1.06),
		# 女子溜肩：肩比男子窄一截。
		"shoulder": rng.randf_range(0.085, 0.13) * slim * (0.88 if female and not child else 1.0),
		"girth": rng.randf_range(0.07, 0.115) * slim * (1.12 if rich and not female else 1.0),
		"flare": rng.randf_range(0.14, 0.24) * (1.2 if female else 1.0),
		"sleeve": rng.randf_range(0.03, 0.08) if poor else rng.randf_range(0.16, 0.26) if rich else rng.randf_range(0.07, 0.19),
		"hair": hair_pool[rng.randi_range(0, hair_pool.size() - 1)],
		"beard": (rng.randf_range(0.14, 0.24) if elder else rng.randf_range(0.05, 0.13)) if rng.randf() < beard_chance else 0.0,
		"prop": prop_pool[rng.randi_range(0, prop_pool.size() - 1)],
		"ribbon": rng.randf_range(0.15, 0.45),
		"nose": rng.randf_range(0.8, 1.25) * (0.7 if child else 1.0),
		"pleats": rng.randi_range(3, 5) if rich else rng.randi_range(1, 3),
		"cape": rich and rng.randf() < 0.7 or wealth == Wealth.PLAIN and rng.randf() < 0.2,
		"stoop": rng.randf_range(0.5, 1.0) if elder else 0.0,
		"swing": rng.randf_range(-1.0, 1.0),
		"tilt": rng.randf_range(-0.16, 0.12),
		"eyes": rng.randf_range(-0.07, 0.07),
		"jaw": rng.randf_range(0.92, 1.1) * (0.88 if child else 1.0),
	}
	# 鬍鬚的樣式：老者多留長鬚（三綹、五綹），儒、道偏清鬚，貧者多絡腮。
	var styles: Array = [Whiskers.MOUSTACHE, Whiskers.GOATEE, Whiskers.FULL, Whiskers.THREE]
	if elder:
		styles = [Whiskers.THREE, Whiskers.FIVE, Whiskers.FIVE, Whiskers.GOATEE]
	elif poor:
		styles = [Whiskers.FULL, Whiskers.FULL, Whiskers.GOATEE, Whiskers.MOUSTACHE]
	elif school == School.RU or school == School.DAO:
		styles = [Whiskers.THREE, Whiskers.FIVE, Whiskers.GOATEE, Whiskers.MOUSTACHE]
	var style: int = styles[rng.randi_range(0, styles.size() - 1)]
	look["whiskers"] = style if look["beard"] > 0.0 else Whiskers.NONE
	for key: String in given:
		look[key] = given[key]
	if not given.has("colors"):
		look["colors"] = _palette(rng, look)
	return look


## 一名修士各部位的顏色：門派定衣色的色系，孩童另有幾個鮮色；女子（僧尼除外）上襦取淡色、裙取濃色、披帛另取一色，
## 一眼看得出上下兩截；貧者褪成麻布的顏色、沒有緣邊，
## 富者較鮮、緣邊用金；年老髮白。所有顏色都往紙色靠一點，才和淡墨的背景合得來。
static func _palette(rng: RandomNumberGenerator, look: Dictionary) -> Dictionary:
	var paper := Color("f2ebd9")
	var robes: Array = [Color("a8463f"), Color("54756a"), Color("b79eb2"), Color("dcc67a"), Color("4f6b85"), Color("8a6a4c")]
	match look["school"]:
		School.DAO:
			robes = [Color("8c999e"), Color("dbe3e0"), Color("5c6373"), Color("4d6b85")]
		School.RU:
			robes = [Color("dbe3e0"), Color("a8c2bd"), Color("54739a"), Color("9e7354")]
		School.SHI:
			robes = [Color("99948a"), Color("80664d"), Color("c7a35c")]
	if look["school"] != School.SHI:
		if look["age"] == Age.CHILD:
			robes.append_array([Color("cc5447"), Color("6fa873")])
	var accents: Array = [Color("a8463f"), Color("3f4552"), Color("4d6b85"), Color("c7a35c"), Color("54756a"), Color("6b4a6e")]
	var robe: Color = robes[rng.randi_range(0, robes.size() - 1)]
	var sash: Color = accents[rng.randi_range(0, accents.size() - 1)]
	var trim: Color = accents[rng.randi_range(0, accents.size() - 1)]
	var cape: Color = accents[rng.randi_range(0, accents.size() - 1)]
	var gold := rng.randf() < 0.6
	var tan := rng.randf()
	var grey := rng.randf_range(0.5, 1.0)
	# 女裝的三個顏色：不論男女都先取亂數，其他項的取值才不跟著性別變。
	var upper: Color = [Color("efe6d2"), Color("f0d9d2"), Color("dfe8d8"), Color("dbe3ea")][rng.randi_range(0, 3)]
	var skirt: Color = [Color("b8433c"), Color("4f7f66"), Color("46648f"), Color("7d5890"), Color("c99a3a")][rng.randi_range(0, 4)]
	var shawl: Color = [Color("f3e3b0"), Color("efc0c4"), Color("cfe4d6"), Color("dccdea")][rng.randi_range(0, 3)]
	var dress: bool = look["sex"] == Sex.FEMALE and look["school"] != School.SHI
	if dress:
		robe = upper
	else:
		skirt = robe
	if look["school"] == School.SHI:
		trim = [Color("b0563a"), Color("c99a3c"), Color("7a4a3a")][rng.randi_range(0, 2)]
	match look["wealth"]:
		Wealth.POOR:
			robe = robe.lerp(Color("a89980"), 0.4 if dress else 0.7)
			skirt = skirt.lerp(Color("a89980"), 0.6 if dress else 0.7)
			sash = Color("7a6a55")
			if look["school"] != School.SHI:
				trim = Color("8f8068")
		Wealth.RICH:
			robe.s = minf(robe.s * 1.15, 1.0)
			skirt.s = minf(skirt.s * 1.15, 1.0)
			if gold and look["school"] != School.SHI:
				trim = Color("cca34d")
	var hair := Color("201e1e")
	if look["age"] == Age.ELDER:
		hair = hair.lerp(Color("c9c9c4"), grey)
	var hat := Color("d1b87a")
	match look["hair"]:
		Hair.SCHOLAR_CAP:
			hat = Color("2f2f36")
		Hair.CROWN:
			hat = Color("cca34d") if look["wealth"] == Wealth.RICH else Color("9ec7b3")
	var props := {
		Prop.SWORD_BACK: Color("5a4038"), Prop.WHISK: Color("f0ede4"), Prop.GOURD: Color("cc9952"), Prop.STAFF: Color("806142"),
		Prop.FAN: Color("f2eedb"), Prop.FLUTE: Color("9ead66"), Prop.LOTUS: Color("eeb3bd"), Prop.BASKET: Color("b89461"),
		Prop.TABLET: Color("ede6cc"), Prop.BEADS: Color("6b4230"), Prop.SCROLL: Color("e6dbb8"),
	}
	var skin := Color("f6e2cf").lerp(Color("d9b894"), tan * tan)
	if look["age"] == Age.ELDER:
		skin = skin.lerp(Color("cfc6b8"), 0.2)
	var soft: bool = look["sex"] == Sex.FEMALE or look["age"] == Age.CHILD
	return {
		SKIN: skin, ROBE: robe.lerp(paper, 0.16), INNER: Color("f7f4ec"), SASH: sash.lerp(paper, 0.12), TRIM: trim.lerp(paper, 0.12),
		CAPE: cape.lerp(paper, 0.3), HAIR: hair, BEARD: hair.lerp(Color("e6e4dd"), 0.25), HAT: hat,
		PROP: props.get(look["prop"], Color("f2eedb")), SHOE: Color("332e2e"),
		BLUSH: Color(0.86, 0.45, 0.4, 0.3 if soft else 0.12), LIP: Color(0.74, 0.3, 0.28, 0.9 if soft else 0.45),
		SKIRT: skirt.lerp(paper, 0.1), SHAWL: Color(shawl, 0.92),
	}


## 這名修士擺這個姿勢時的剪影：一組實心多邊形（成人身高為 1 的座標，未乘 stature）。
static func shapes(look: Dictionary, pose: Variant, time := 0.0) -> Array[PackedVector2Array]:
	var fills: Array[PackedVector2Array] = []
	var lines: Array[PackedVector2Array] = []
	_build(look, pose, time, fills, lines, [], [])
	return fills


## 剪影裡面的線（衣領、腰帶、衣褶、袖口、袈裟、扇骨）：一組折線。
static func folds(look: Dictionary, pose: Variant, time := 0.0) -> Array[PackedVector2Array]:
	var fills: Array[PackedVector2Array] = []
	var lines: Array[PackedVector2Array] = []
	_build(look, pose, time, fills, lines, [], [])
	return lines


## 整個剪影的範圍。
static func bounds_of(polygons: Array[PackedVector2Array]) -> Rect2:
	var box := Rect2(polygons[0][0], Vector2.ZERO)
	for polygon in polygons:
		for point in polygon:
			box = box.expand(point)
	return box


## 畫在 canvas 上：at 是腳下（坐處），height 是成人站立身高的像素，facing 為 −1 時朝左、1 時朝右；
## ink 是線的墨色，alpha 是整個人的不透明度。
static func draw(canvas: CanvasItem, at: Vector2, height: float, facing: float, look: Dictionary, pose: Variant, time: float, ink: Color, alpha := 1.0) -> void:
	var grow: float = height * look["stature"]
	canvas.draw_set_transform(at, 0.0, Vector2(-grow * facing, grow))
	# 輪廓線約身高的 0.9%，至少 1.6 像素（座標已放大 grow 倍）。
	paint(canvas, look, pose, time, ink, maxf(0.009, 1.6 / grow), alpha)
	canvas.draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


## 以 canvas 目前的座標畫（身高為 1）：stroke 是輪廓線寬，衣紋細一些。
static func paint(canvas: CanvasItem, look: Dictionary, pose: Variant, time: float, ink: Color, stroke: float, alpha := 1.0) -> void:
	var fills: Array[PackedVector2Array] = []
	var lines: Array[PackedVector2Array] = []
	var bands: Array = []
	var cuts: Array = []
	_build(look, pose, time, fills, lines, bands, cuts)
	var colors: Dictionary = look["colors"]
	for cut: Array in _ordered(cuts):
		var part: int = cut[2] % OPEN
		var base: Color = colors[part]
		base.a *= alpha
		for piece in _unite(fills.slice(cut[3], cut[4])):
			# 暈染：顏色略略滲出墨線之外。
			if part != SKIN and part != HAIR:
				for bleed in _bleed(piece, stroke * 1.1):
					canvas.draw_colored_polygon(bleed, Color(base, base.a * 0.3))
			canvas.draw_polygon(piece, _graded(piece, base, DEPTH.get(part, 0.1)))
			if cut[2] < OPEN and part != HAIR:
				_outline(canvas, piece, ink, stroke)
		for i in range(cut[7], cut[8]):
			var band: Array = bands[i]
			var tint: Color = Color(base.darkened(0.32), 0.42) if band[1] == SHADE else colors[band[1]]
			tint.a *= alpha
			canvas.draw_polyline(band[0], tint, band[2], true)
		for i in range(cut[5], cut[6]):
			_brush(canvas, lines[i], ink, stroke * 0.8)


## 暈染用的外擴輪廓。外擴後圍出洞（兩縷細帶靠得太近、中間的縫被吃掉）時整塊不畫：洞也是個多邊形，照畫會把縫填成一整片灰葉。
static func _bleed(piece: PackedVector2Array, grow: float) -> Array[PackedVector2Array]:
	var grown: Array[PackedVector2Array] = []
	for polygon in Geometry2D.offset_polygon(piece, grow):
		if Geometry2D.is_polygon_clockwise(polygon):
			return []
		grown.append(polygon)
	return grown


## 這個人某個部位的色塊（由後往前）：看剪影裡哪一塊是裙、哪一塊是披帛用。
static func pieces(look: Dictionary, pose: Variant, part: int, time := 0.0) -> Array[PackedVector2Array]:
	var fills: Array[PackedVector2Array] = []
	var cuts: Array = []
	_build(look, pose, time, fills, [], [], cuts)
	var found: Array[PackedVector2Array] = []
	for cut: Array in _ordered(cuts):
		if cut[2] % OPEN == part:
			found.append_array(fills.slice(cut[3], cut[4]))
	return found


## 這個人由後往前的每一層：[層次, 部位]。
static func layers(look: Dictionary, pose: Variant, time := 0.0) -> Array:
	var cuts: Array = []
	_build(look, pose, time, [], [], [], cuts)
	var found: Array = []
	for cut: Array in _ordered(cuts):
		found.append([cut[1], cut[2] % OPEN])
	return found


## 把各層依層次表排好（同一層次的照加入的先後）：[排序鍵, 層次, 部位, 多邊形起, 迄, 線起, 迄, 色帶起, 迄]。
static func _ordered(cuts: Array) -> Array:
	var records: Array = []
	var stage := 0
	var from: Array[int] = [0, 0, 0]
	for cut: Array in cuts:
		if cut.size() == 1:
			stage = cut[0]
			continue
		var layer: int = cut[4] if cut[4] >= 0 else stage
		if cut[0] > from[0] or cut[1] > from[1] or cut[2] > from[2]:
			records.append([layer * 4096 + records.size(), layer, cut[3], from[0], cut[0], from[1], cut[1], from[2], cut[2]])
		from = [cut[0], cut[1], cut[2]]
	records.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	return records


## 勾一塊的輪廓：整圈一道細線，朝下、背光的那幾段再壓一道粗的——線有輕重，不是均勻的描邊。
static func _outline(canvas: CanvasItem, piece: PackedVector2Array, ink: Color, stroke: float) -> void:
	var centre := Vector2.ZERO
	for point in piece:
		centre += point
	centre /= piece.size()
	var whole := piece.duplicate()
	whole.append(piece[0])
	canvas.draw_polyline(whole, ink, stroke * 0.6, true)
	var run := PackedVector2Array()
	for i in whole.size() - 1:
		var outward := (whole[i] + whole[i + 1]) * 0.5 - centre
		if outward.normalized().dot(Vector2(0.35, 0.94)) > 0.25:
			if run.is_empty():
				run.append(whole[i])
			run.append(whole[i + 1])
		elif run.size() > 1:
			canvas.draw_polyline(run, ink, stroke * 1.2, true)
			run = PackedVector2Array()
		else:
			run = PackedVector2Array()
	if run.size() > 1:
		canvas.draw_polyline(run, ink, stroke * 1.2, true)


## 一筆：起筆重、收筆輕（前半段粗，後半段細）。
static func _brush(canvas: CanvasItem, line: PackedVector2Array, ink: Color, stroke: float) -> void:
	canvas.draw_polyline(line, ink, stroke * 0.55, true)
	if line.size() < 3:
		return
	canvas.draw_polyline(line.slice(0, line.size() / 2 + 1), ink, stroke, true)


## 一塊的各頂點顏色：由上往下加深 depth。
static func _graded(piece: PackedVector2Array, base: Color, depth: float) -> PackedColorArray:
	var top := piece[0].y
	var bottom := piece[0].y
	for point in piece:
		top = minf(top, point.y)
		bottom = maxf(bottom, point.y)
	var deep := base.darkened(depth)
	var colors := PackedColorArray()
	for point in piece:
		var t := (point.y - top) / maxf(bottom - top, 0.001)
		colors.append(base.lerp(deep, t * t))
	return colors


## 把一層的多邊形併成不相疊的幾塊（只留外輪廓）。
static func _unite(polygons: Array) -> Array[PackedVector2Array]:
	var pieces: Array[PackedVector2Array] = []
	for polygon: PackedVector2Array in polygons:
		var whole := polygon.duplicate()
		if Geometry2D.is_polygon_clockwise(whole):
			whole.reverse()
		var apart: Array[PackedVector2Array] = []
		for piece in pieces:
			var outers: Array[PackedVector2Array] = []
			var holes := 0
			for merged in Geometry2D.merge_polygons(whole, piece):
				if Geometry2D.is_polygon_clockwise(merged):
					holes += 1
				else:
					outers.append(merged)
			# 合併後圍出洞（兩縷飄帶交叉成葉形）時不合併：只留外輪廓會把洞填成一整片。
			if outers.size() == 1 and holes == 0:
				whole = outers[0]
			else:
				apart.append(piece)
		apart.append(whole)
		pieces = apart
	return pieces


## 到這裡為止新加的多邊形、線與色帶算一層，part 是這一層的部位。
## layer 是它在層次表上的位置；不給就用 _stage 指定的那一層。
static func _cut(cuts: Array, fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, part: int, layer := -1) -> void:
	cuts.append([fills.size(), lines.size(), bands.size(), part, layer])


## 接下來的各層，沒有另外指定的都畫在 layer 這一層。
static func _stage(cuts: Array, layer: Layer) -> void:
	cuts.append([layer])


static var _parts := {}


static func _pick(rng: RandomNumberGenerator, pool: Array) -> int:
	return pool[rng.randi_range(0, pool.size() - 1)]


static func _build(look: Dictionary, pose: Variant, time: float, fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, cuts: Array) -> void:
	var joints: Dictionary = pose if pose is Dictionary else POSES[pose]
	var seated: bool = joints["seated"]
	var female: bool = look["sex"] == Sex.FEMALE
	var poor: bool = look["wealth"] == Wealth.POOR
	var monk: bool = look["school"] == School.SHI
	# 女裝（僧尼除外）：上襦下裙，裙繫在胸下；披帛搭在兩臂、垂在身外。
	var dress := female and not monk
	# 重心：腰往一邊送、肩往另一邊回；年老駝背，頸往前、往下。
	var shift: Vector2 = Vector2(0.024 * float(look["swing"]), 0.0) * (0.3 if seated else 1.0)
	var hip: Vector2 = joints["hip"] + shift
	var lean: Vector2 = Vector2(-0.04, 0.022) * float(look["stoop"]) - shift * 0.8
	var neck: Vector2 = joints["neck"] + lean
	var s: float = look["shoulder"]
	var g: float = look["girth"]
	var f: float = look["flare"]
	var sway := sin(time * 2.6)
	var sway_slow := sin(time * 1.7 + 1.3)
	var w: float = joints["wind"] * (1.0 + 0.35 * sway_slow)
	var radius: float = 0.5 / look["heads"]
	var tilt: float = float(look["tilt"]) - 0.12 * float(look["stoop"])
	var head: Vector2 = neck + Vector2(-0.014 + tilt * radius * 0.8, -radius * 1.12)
	var elbow_f: Vector2 = joints["elbow_f"] + lean * 0.6
	var elbow_b: Vector2 = joints["elbow_b"] + lean * 0.6
	var hand_f: Vector2 = joints["hand_f"] + lean * 0.3
	var hand_b: Vector2 = joints["hand_b"] + lean * 0.3
	var blowing: bool = look["prop"] == Prop.FLUTE and pose is int and pose == Pose.STAND
	if blowing:
		# 站著拿笛的人把笛橫在嘴邊吹：兩手抬到臉前，不垂在腰間（垂著的一管看起來像佩劍）。
		hand_f = head + Vector2(-radius * 1.25, radius * 0.6)
		elbow_f = neck + Vector2(-0.10, 0.15)
		hand_b = hand_f + Vector2(0.13, 0.035)
		elbow_b = neck + Vector2(0.10, 0.15)
	# 身體半轉：朝前那一肩收窄、靠外那一肩完整；重心偏的那一邊肩略低。
	var tip: float = 0.012 * float(look["swing"])
	# 女子溜肩：兩肩再垂下一點。
	var slope := 0.014 if dress else 0.0
	var far := neck + Vector2(-s * 0.74, 0.05 - tip + slope)
	var near := neck + Vector2(s * 1.02, 0.058 + tip + slope)
	# 貧者短褐：衣擺只到小腿，底下露出腿腳，下緣破碎。
	var hem := -0.14 if poor and not seated and not dress else 0.0
	var rag := 0.014 if poor and not dress else 0.0
	var behind: bool = look["prop"] in BEHIND_PROPS
	# 步行：衣擺的前緣跟著較前的腳、後緣跟著較後的腳（沒有 foot_f、foot_b 就全為 0，與舊畫法相同）。
	var step_a: float = (joints.get("foot_f", Vector2.ZERO) as Vector2).x
	var step_b: float = (joints.get("foot_b", Vector2.ZERO) as Vector2).x
	var hem_front := minf(step_a, step_b) * 0.8
	var hem_back := maxf(step_a, step_b) * 0.8
	var hem_mid := (hem_front + hem_back) * 0.5

	# 身後的東西先畫：背著或掛著的持物、披在背後的大氅、女子的披帛、朝前那隻手（上臂在身後）。
	_stage(cuts, Layer.BACK_PROP)
	if behind:
		_held_prop(fills, lines, look, hip, neck, hand_b, time)
	_cut(cuts, fills, lines, bands, PROP)
	_stage(cuts, Layer.CAPE)
	if look["cape"] and not seated:
		# 大氅的肩頭有個最小寬度：肩窄的人兩個點擠在一起，輪廓會打結。
		var drape := maxf(s, 0.095)
		fills.append(_simple(smooth_outline([
			neck + Vector2(drape * 0.2, 0.034), neck + Vector2(drape * 0.7, 0.014), neck + Vector2(drape * 1.02 + 0.02, 0.068 + tip + slope), hip + Vector2(g + 0.07 + w * 0.6, -0.05),
			Vector2(hip.x + f + 0.07 + w * 2.2, -0.10 - w * 0.5), Vector2(hip.x + f + 0.02 + w * 1.6 + 0.015 * sway, -0.04),
			Vector2(hip.x + f * 0.6, -0.18), hip + Vector2(g * 0.5, -0.02),
		])))
		lines.append(PackedVector2Array([near + Vector2(0.03, 0.12), hip + Vector2(g + 0.08 + w, 0.1), Vector2(hip.x + f + 0.03 + w * 1.6, -0.12)]))
	_cut(cuts, fills, lines, bands, CAPE)
	_stage(cuts, Layer.SHAWL)
	if dress and not poor:
		# 披帛靠外的一端：自臂彎垂到身後，飄在輪廓外面。
		fills.append(_ribbon(elbow_b + Vector2(0.03, -0.02), Vector2(0.20 + w * 1.5, 0.50 if not seated else 0.2), 0.055, time, 1.8, 0.45))
	_cut(cuts, fills, lines, bands, SHAWL)
	_stage(cuts, Layer.FAR_ARM)
	# 吹笛時上臂折在身體後面看不到，只畫前臂（折得太急的整隻袖子輪廓會自己交叉）。
	_arm(fills, lines, far, elbow_f, hand_f, look["sleeve"], sway, joints["point"], blowing, bands, cuts)
	_cut(cuts, fills, lines, bands, ROBE)
	_stage(cuts, Layer.LEGS)
	if not seated:
		if poor and not dress:
			# 腿腳露在短褐底下，赤足；重心在一腳，另一腳略往前。
			for leg: float in [-0.05, 0.035]:
				var stride: Vector2 = joints.get("foot_f" if leg < 0.0 else "foot_b", Vector2.ZERO)
				var ankle := Vector2(hip.x + leg - (0.02 if leg < 0.0 else 0.0), -0.012 + (0.006 if leg < 0.0 else 0.0)) + stride
				fills.append(_limb(Vector2(hip.x + leg * 0.9, hem - 0.03), ankle, 0.024, 0.016))
				fills.append(smooth_outline([ankle + Vector2(0.02, -0.012), ankle + Vector2(0.022, 0.012), ankle + Vector2(-0.06, 0.012), ankle + Vector2(-0.068, 0.0), ankle + Vector2(-0.02, -0.014)]))
				_cut(cuts, fills, lines, bands, SKIN)
		else:
			# 鞋尖：衣擺前緣露出翹頭履。
			# 步行等動作帶 foot_f、foot_b（相對立姿的雙腳位移）：兩隻鞋各自擺；沒有這兩個鍵時只畫一隻，與舊畫法相同。
			for foot_key: String in (["foot_f", "foot_b"] if joints.has("foot_f") else [""]):
				var toe := Vector2(hip.x - f * 0.5, 0.0) + (joints[foot_key] as Vector2 if foot_key != "" else Vector2.ZERO)
				fills.append(smooth_outline([toe + Vector2(0.03, 0.006), toe + Vector2(-0.05, 0.006), toe + Vector2(-0.085, -0.012), toe + Vector2(-0.075, -0.034), toe + Vector2(-0.04, -0.018)]))
				_cut(cuts, fills, lines, bands, SHOE)

	_stage(cuts, Layer.ROBE)
	# 袍：衣領、肩、胸、腰，再到衣擺（站）或盤起的腿（坐）。
	var body: Array[Vector2] = [
		neck + Vector2(-0.03, 0.0), neck + Vector2(-s * 0.5, 0.02), far, neck + Vector2(-s * 0.84, 0.16),
	]
	if seated:
		body.append_array([
			hip + Vector2(-g - 0.005, -0.06), hip + Vector2(-0.16, -0.022), Vector2(hip.x - 0.29, -0.105), Vector2(hip.x - 0.365, -0.06),
			Vector2(hip.x - 0.355, -0.012), Vector2(hip.x - 0.29, 0.0), Vector2(hip.x - 0.05, 0.008), Vector2(hip.x + g + f * 0.45, 0.0),
			Vector2(hip.x + g + f * 0.6 + w, -0.022), Vector2(hip.x + g + 0.06, -0.075), hip + Vector2(g + 0.012, -0.04),
		])
	else:
		# 衣擺下緣是一道往下鼓的弧（從略高處看下去的圓擺）。
		body.append_array([
			hip + Vector2(-g, -0.03), hip + Vector2(-g - 0.012, 0.06), Vector2(hip.x - f * 0.55 + hem_front * 0.5, -0.24 + hem * 0.4), Vector2(hip.x - f * 0.74 + hem_front, -0.045 + hem),
			Vector2(hip.x - f * 0.5 + hem_front, -0.008 + hem - rag), Vector2(hip.x - f * 0.05 + hem_mid, 0.012 + 0.004 * sway + hem + rag), Vector2(hip.x + f * 0.42 + hem_back * 0.6, 0.006 + hem - rag),
			Vector2(hip.x + f * 0.8 + w + hem_back, -0.006 - w * 0.12 + hem + rag), Vector2(hip.x + f + w * 1.7 + 0.02 * sway + hem_back, -0.035 - w * 0.45 + hem),
			Vector2(hip.x + f * 0.8 + w + hem_back * 0.5, -0.22 + hem * 0.4), hip + Vector2(g + 0.03 + w * 0.3, 0.05), hip + Vector2(g + 0.006, -0.03),
		])
	body.append_array([neck + Vector2(s * 0.98, 0.17), near, neck + Vector2(s * 0.7, 0.018), neck + Vector2(0.03, 0.0)])
	fills.append(_simple(smooth_outline(body)))

	# 腰帶的高度。女裝的裙繫在胸下：裙是另一塊、另一個顏色，蓋在上襦外面，自胸下往下放開。
	var waist_y := hip.y - (0.05 if seated else 0.03)
	var waist_half := g * 0.98
	if dress:
		waist_y = neck.y + 0.125
		waist_half = maxf(g, s * 0.84) + 0.004
		_cut(cuts, fills, lines, bands, ROBE)
		var skirt: Array[Vector2] = [Vector2(hip.x - waist_half, waist_y)]
		if seated:
			skirt.append_array([
				hip + Vector2(-g - 0.008, -0.06), hip + Vector2(-0.165, -0.026), Vector2(hip.x - 0.295, -0.11), Vector2(hip.x - 0.37, -0.06),
				Vector2(hip.x - 0.36, -0.01), Vector2(hip.x - 0.29, 0.003), Vector2(hip.x - 0.05, 0.011), Vector2(hip.x + g + f * 0.46, 0.003),
				Vector2(hip.x + g + f * 0.62 + w, -0.022), Vector2(hip.x + g + 0.064, -0.077), hip + Vector2(g + 0.016, -0.04),
			])
		else:
			skirt.append_array([
				Vector2(hip.x - g * 1.08 - 0.012, hip.y), Vector2(hip.x - f * 0.62, -0.24), Vector2(hip.x - f * 0.8, -0.045),
				Vector2(hip.x - f * 0.52, -0.004), Vector2(hip.x - f * 0.05, 0.016 + 0.004 * sway), Vector2(hip.x + f * 0.44, 0.01),
				Vector2(hip.x + f * 0.84 + w, -0.002 - w * 0.12), Vector2(hip.x + f * 1.06 + w * 1.7 + 0.02 * sway, -0.03 - w * 0.45),
				Vector2(hip.x + f * 0.86 + w, -0.22), Vector2(hip.x + g * 1.1 + 0.034 + w * 0.3, hip.y + 0.02),
			])
		skirt.append(Vector2(hip.x + waist_half, waist_y))
		fills.append(_simple(smooth_outline(skirt)))
	if monk:
		# 袈裟：自靠外的肩斜披到前腰，兩道邊，中間是袈裟的顏色。
		bands.append([PackedVector2Array([neck + Vector2(s * 0.82, 0.065), neck + Vector2(-s * 0.05, 0.18), Vector2(hip.x - g * 0.9, waist_y + 0.075)]), TRIM, 0.045])
		lines.append(PackedVector2Array([neck + Vector2(s * 0.75, 0.04), neck + Vector2(-s * 0.1, 0.16), Vector2(hip.x - g, waist_y + 0.05)]))
		lines.append(PackedVector2Array([neck + Vector2(s * 0.9, 0.09), neck + Vector2(0.0, 0.20), Vector2(hip.x - g * 0.85, waist_y + 0.10)]))
		# 頸上一串念珠，垂到胸前。
		for i in 9:
			var t := i / 8.0
			lines.append(_circle(neck + Vector2(lerpf(s * 0.45, -s * 0.5, t), 0.03 + 0.11 * sin(t * PI)), 0.009))
	else:
		# 交領：兩襟在胸前交成一個「y」，身體半轉，交點偏向朝前那一邊；領緣是裡衣的顏色，腰帶另一色。
		# 坐著時兩袖攏在身前，領緣只畫到胸口，不伸到袖子後面。
		var lapel_end := neck + Vector2(-s * 0.52, 0.2) if seated and not dress else Vector2(hip.x - g * 0.58, waist_y)
		bands.append([PackedVector2Array([neck + Vector2(0.03, 0.012), neck + Vector2(-s * 0.26, 0.09), neck + Vector2(-s * 0.44, 0.15), lapel_end]), INNER, 0.02])
		bands.append([PackedVector2Array([Vector2(hip.x - waist_half * 0.96, waist_y + 0.014), Vector2(hip.x, waist_y + 0.022), Vector2(hip.x + waist_half * 0.96, waist_y + 0.014)]), SASH, 0.028])
		lines.append(PackedVector2Array([neck + Vector2(0.024, 0.006), neck + Vector2(-s * 0.3, 0.085), neck + Vector2(-s * 0.5, 0.15), Vector2(hip.x - g * 0.66, waist_y)]))
		lines.append(PackedVector2Array([neck + Vector2(-0.028, 0.008), neck + Vector2(-s * 0.26, 0.07)]))
		# 腰帶：上下兩道（貧者只一條草繩）。
		lines.append(PackedVector2Array([Vector2(hip.x - waist_half, waist_y), Vector2(hip.x, waist_y + 0.008), Vector2(hip.x + waist_half, waist_y)]))
		if not poor:
			lines.append(PackedVector2Array([Vector2(hip.x - waist_half, waist_y + 0.028), Vector2(hip.x, waist_y + 0.036), Vector2(hip.x + waist_half, waist_y + 0.028)]))
	if not seated:
		# 衣褶：自腰帶往衣擺散開，越靠後越被風帶開；長短不一。
		# 裙褶比袍褶密。
		var pleats: int = look["pleats"] + (3 if dress else 0)
		for i in pleats:
			var t := (i + 1.0) / (pleats + 1.0)
			var top := Vector2(hip.x + lerpf(-g * 0.7, g * 0.8, t), waist_y + 0.04 + 0.1 * (i % 2))
			var bottom := Vector2(hip.x + lerpf(-f * 0.45, f * 0.7 + w * 1.2, t), -0.012 + hem)
			var middle := top.lerp(bottom, 0.5) + Vector2(0.014 * sway * t - 0.01, 0.0)
			lines.append(PackedVector2Array([top, middle, bottom]))
			# 衣褶背光的一面。
			bands.append([PackedVector2Array([top + Vector2(0.009, 0.01), middle + Vector2(0.011, 0.0), bottom + Vector2(0.012, -0.02)]), SHADE, 0.018])
		if poor:
			# 補丁：衣上兩塊方布。
			for patch: Vector2 in [Vector2(hip.x - g * 0.5, -0.36), Vector2(hip.x + g * 0.6, -0.25)]:
				lines.append(PackedVector2Array([patch, patch + Vector2(0.035, 0.004), patch + Vector2(0.032, 0.04), patch + Vector2(-0.003, 0.036), patch]))
		elif not monk and not dress:
			# 腰間的佩玉：一條絲繩垂一塊玉，再一綹穗。
			var cord := Vector2(hip.x - g * 0.45, waist_y + 0.03)
			lines.append(PackedVector2Array([cord, cord + Vector2(0.006 * sway, 0.09)]))
			lines.append(_circle(cord + Vector2(0.006 * sway, 0.105), 0.014))
			lines.append(PackedVector2Array([cord + Vector2(0.006 * sway, 0.12), cord + Vector2(0.012 * sway, 0.19)]))
		if not poor:
			# 下襬的緣邊；富者衣上幾朵團花。
			bands.append([smooth_path([
				Vector2(hip.x - f * 0.56, -0.036), Vector2(hip.x - f * 0.05, -0.008 + 0.004 * sway), Vector2(hip.x + f * 0.42, -0.014),
				Vector2(hip.x + f * 0.8 + w, -0.028 - w * 0.12),
			]), TRIM, 0.022])
			if look["wealth"] == Wealth.RICH:
				for spot: Vector2 in [Vector2(f * 0.12, -0.30), Vector2(-f * 0.22, -0.17), Vector2(f * 0.45 + w * 0.6, -0.14)]:
					bands.append([_circle(Vector2(hip.x + spot.x, spot.y), 0.018), TRIM, 0.006])
	else:
		# 盤腿：膝頭一道弧，袍角攤在地上。
		lines.append(PackedVector2Array([Vector2(hip.x - 0.10, -0.035), Vector2(hip.x - 0.24, -0.075), Vector2(hip.x - 0.345, -0.045)]))
		lines.append(PackedVector2Array([Vector2(hip.x + g * 0.3, -0.03), Vector2(hip.x + g + f * 0.35, -0.004)]))
		bands.append([PackedVector2Array([Vector2(hip.x - 0.11, -0.027), Vector2(hip.x - 0.24, -0.065), Vector2(hip.x - 0.335, -0.037)]), SHADE, 0.018])
	_cut(cuts, fills, lines, bands, SKIRT if dress else ROBE)
	_stage(cuts, Layer.TASSEL)
	if dress:
		# 胸下繫裙的兩條帶子垂在身前；披帛朝前的一端自臂彎垂在裙前。
		var knot := Vector2(hip.x - g * 0.3, waist_y + 0.03)
		fills.append(_ribbon(knot, Vector2(-0.015 + w * 0.6, 0.26 if not seated else 0.12), 0.02, time, 2.1))
		fills.append(_ribbon(knot + Vector2(0.012, 0.0), Vector2(0.02 + w * 0.8, 0.22 if not seated else 0.1), 0.018, time + 0.5, 2.5))
		_cut(cuts, fills, lines, bands, SASH)
		if not poor:
			fills.append(_ribbon(elbow_f + Vector2(-0.02, 0.03), Vector2(-0.12 + w * 0.5, 0.46 if not seated else 0.14), 0.05, time + 0.9, 2.1, 0.45))
			_cut(cuts, fills, lines, bands, SHAWL)
	elif not seated and not monk:
		# 腰帶的垂穗，往後飄。
		fills.append(_ribbon(Vector2(hip.x + g * 0.6, waist_y + 0.02), Vector2(0.10 + w, 0.26), 0.03, time, 2.1))
		if not poor:
			fills.append(_ribbon(Vector2(hip.x + g * 0.5, waist_y + 0.02), Vector2(0.05 + w * 0.8, 0.30), 0.022, time + 0.5, 2.5))
	_cut(cuts, fills, lines, bands, SASH)

	# 頸、頭（七分面與五官）、鬚、髮。
	_stage(cuts, Layer.FACE)
	var neck_at := fills.size()
	fills.append(_limb(head + Vector2(0.03, radius * 0.5), neck + Vector2(0.004, 0.03), radius * 0.4, radius * 0.5))
	_cut(cuts, fills, lines, bands, SKIN, Layer.NECK)
	# 衣領圍在頸根外面：遠側的一襟先畫，近側的一襟自頸後繞到身前壓在它上面，交成「y」。
	# 一般人露出裡衣的白領；僧衣與貧者的短褐沒有裡衣，領子就是衣服本身的顏色。
	var collar := ROBE if monk or poor else INNER
	fills.append(_limb(neck + Vector2(-0.03, 0.01), neck + Vector2(-s * 0.24, 0.084), 0.007, 0.007))
	_cut(cuts, fills, lines, bands, collar, Layer.COLLAR)
	fills.append(smooth_outline([
		neck + Vector2(0.03, 0.006), neck + Vector2(0.022, 0.03), neck + Vector2(-0.004, 0.048), neck + Vector2(-s * 0.3, 0.102),
		neck + Vector2(-s * 0.34, 0.09), neck + Vector2(-0.01, 0.033), neck + Vector2(0.01, 0.018), neck + Vector2(0.022, 0.0),
	]))
	_cut(cuts, fills, lines, bands, collar, Layer.COLLAR)
	var head_from := [fills.size(), lines.size(), bands.size()]
	_face(fills, lines, bands, look, head, radius, tilt)
	_cut(cuts, fills, lines, bands, SKIN)
	if dress and not poor and look["age"] != Age.CHILD:
		# 花鈿：眉心一點紅。
		fills.append(_disc(head + Vector2(-0.36, -0.42).rotated(tilt) * radius, radius * 0.075))
		_cut(cuts, fills, lines, bands, LIP + OPEN)
	_whiskers(fills, lines, bands, cuts, look, head, radius, tilt, sway)
	_stage(cuts, Layer.HEADWEAR)
	_hair(fills, lines, bands, cuts, look, head, radius, time, w, tilt)
	_roll_head(fills, lines, bands, neck_at, head_from, neck, float(joints.get("head_roll", 0.0)))

	# 身前的東西最後畫：朝前那隻手的前臂與袖（橫在身前）、靠外那隻手與它拿的東西、朝前那隻手拿的東西。
	_stage(cuts, Layer.FOREARM)
	_arm(fills, lines, far, elbow_f, hand_f, look["sleeve"], sway, joints["point"], true, bands, cuts)
	_cut(cuts, fills, lines, bands, ROBE)
	_stage(cuts, Layer.NEAR_PROP)
	if not behind:
		_held_prop(fills, lines, look, hip, neck, hand_b, time)
	_cut(cuts, fills, lines, bands, PROP)
	_stage(cuts, Layer.NEAR_ARM)
	_arm(fills, lines, near, elbow_b, hand_b, look["sleeve"], sway_slow, false, blowing, bands, cuts)
	_cut(cuts, fills, lines, bands, ROBE)
	_stage(cuts, Layer.FRONT_PROP)
	_front_prop(fills, lines, look, hand_f, time)
	_cut(cuts, fills, lines, bands, PROP)
	_move_body(fills, lines, bands, joints)


## 整個人當剛體移動（姿勢字典帶 body_roll 才用）：繞 body_pivot 轉 body_roll 弧度（正值是頭往後倒），再整體上移 body_lift。
## 倒地與浮空用；各部位仍照直立的關節畫好，最後一起搬。
static func _move_body(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, joints: Dictionary) -> void:
	if not joints.has("body_roll"):
		return
	var roll: float = joints["body_roll"]
	var pivot: Vector2 = joints["body_pivot"]
	var lift := Vector2(0.0, -float(joints["body_lift"]))
	# 倒地時衣擺攤在地上：低於地面的點壓到地面（浮空時不壓）。
	var floor_y: float = -0.004 if absf(roll) > 0.05 else 1e9
	var move := func(shape: PackedVector2Array) -> PackedVector2Array:
		var out := PackedVector2Array()
		for point in shape:
			var moved := (point - pivot).rotated(roll) + pivot + lift
			if moved.y > floor_y:
				# 壓成近乎貼地的一層（單調壓縮，不讓點疊成同一高度而無法三角化）。
				moved.y = floor_y + (moved.y - floor_y) * 0.3
			out.append(moved)
		return out
	for i in fills.size():
		fills[i] = move.call(fills[i])
	for i in lines.size():
		lines[i] = move.call(lines[i])
	for band: Array in bands:
		band[0] = move.call(band[0])


## 頭整個繞頸根轉 roll 弧度（正值是頭往後仰、臉朝上）：頸、臉、鬚、髮、冠帽一起轉，衣領不動。
## 姿勢字典帶 head_roll 才會用到（3D 動作的後仰，超出 tilt 這種個人習慣的小傾斜）。
static func _roll_head(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, neck_at: int, from: Array, pivot: Vector2, roll: float) -> void:
	if is_zero_approx(roll):
		return
	var turn := func(shape: PackedVector2Array) -> PackedVector2Array:
		var out := PackedVector2Array()
		for point in shape:
			out.append((point - pivot).rotated(roll) + pivot)
		return out
	fills[neck_at] = turn.call(fills[neck_at])
	for i in range(from[0], fills.size()):
		fills[i] = turn.call(fills[i])
	for i in range(from[1], lines.size()):
		lines[i] = turn.call(lines[i])
	for i in range(from[2], bands.size()):
		bands[i][0] = turn.call(bands[i][0])


## 七分面：臉的輪廓是一個多邊形，眉、眼、鼻、口各一兩筆線。tilt 是頭的傾斜。
static func _face(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, look: Dictionary, head: Vector2, radius: float, tilt: float) -> void:
	var jaw: float = look["jaw"]
	var child: bool = look["age"] == Age.CHILD
	var outline: Array[Vector2] = []
	for point: Vector2 in [
		Vector2(0.0, -1.0), Vector2(-0.5, -0.92), Vector2(-0.8, -0.55), Vector2(-0.88, -0.15), Vector2(-0.85, 0.2), Vector2(-0.74, 0.55 * jaw),
		Vector2(-0.48, 0.94 * jaw), Vector2(-0.14, 1.0 * jaw), Vector2(0.34, 0.8), Vector2(0.7, 0.42), Vector2(0.92, -0.1), Vector2(0.85, -0.6), Vector2(0.5, -0.92),
	]:
		outline.append(head + point.rotated(tilt) * radius)
	fills.append(smooth_outline(outline))
	# 眼的高度：孩童的五官偏下。
	var e := 0.1 if child else -0.02
	var slant: float = look["eyes"]
	var nose: float = look["nose"]
	var strokes: Array = [
		# 眉：遠的那道短，近的那道長；老者眉尾垂下來。
		[Vector2(-0.8, e - 0.2), Vector2(-0.56, e - 0.25 + slant)],
		[Vector2(-0.36, e - 0.25 + slant), Vector2(-0.12, e - 0.3), Vector2(0.12, e - 0.2 - slant)],
		# 眼：各一道微彎的線。
		[Vector2(-0.76, e + slant * 0.5), Vector2(-0.66, e - 0.04), Vector2(-0.56, e)],
		[Vector2(-0.32, e), Vector2(-0.16, e - 0.05), Vector2(0.02, e - slant * 0.6)],
		# 鼻：一道往前下的線，鼻尖勾回來。
		[Vector2(-0.47, e + 0.04), Vector2(-0.54 - 0.1 * nose, e + 0.36), Vector2(-0.42, e + 0.4)],
		# 口。
		[Vector2(-0.6, e + 0.6 * jaw), Vector2(-0.46, e + 0.63 * jaw), Vector2(-0.32, e + 0.59 * jaw)],
	]
	if look["age"] == Age.ELDER:
		strokes.append([Vector2(0.12, e - 0.2 - slant), Vector2(0.24, e - 0.02)])
		strokes.append([Vector2(-0.28, e + 0.3), Vector2(-0.2, e + 0.5 * jaw)])
	for stroke: Array in strokes:
		var line := PackedVector2Array()
		for point: Vector2 in stroke:
			line.append(head + point.rotated(tilt) * radius)
		lines.append(line)
	# 設色：腮紅、唇色、兩點眼珠。
	var spots: Array = [
		[[Vector2(-0.34, e + 0.3), Vector2(-0.1, e + 0.33)], BLUSH, 0.3],
		[[Vector2(-0.58, e + 0.62 * jaw), Vector2(-0.46, e + 0.65 * jaw), Vector2(-0.34, e + 0.61 * jaw)], LIP, 0.07],
		[[Vector2(-0.67, e - 0.005), Vector2(-0.655, e)], HAIR, 0.085],
		[[Vector2(-0.17, e - 0.012), Vector2(-0.15, e - 0.008)], HAIR, 0.1],
	]
	for spot: Array in spots:
		var dab := PackedVector2Array()
		for point: Vector2 in spot[0]:
			dab.append(head + point.rotated(tilt) * radius)
		bands.append([dab, spot[1], spot[2] * radius])


## 一隻手：手由袖口伸出來（point 為真時並起兩指），袖子順著上臂、前臂，袖口往下垂。袖子在肩頭接著身體，
## 那一端不勾線——外側的線接上肩線，內側的線從腋下起筆；fore_only 時只畫前臂那一段，肘那一端不勾線。
static func _arm(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], shoulder: Vector2, elbow: Vector2, hand: Vector2, sleeve: float, sway: float, point: bool, fore_only: bool, bands: Array, cuts: Array) -> void:
	var along := (hand - elbow).normalized()
	# 袖子垂下來的那一側：以前臂為準，上臂取同一側（沿著手臂同一邊），輪廓才不會在肘彎打結。
	var low_f := along.orthogonal()
	var low_u := (elbow - shoulder).normalized().orthogonal()
	if low_f.dot(_low_side(along)) < 0.0:
		low_f = -low_f
		low_u = -low_u
	var low_e := low_u + low_f
	low_e = low_e.normalized() if low_e.length() > 0.6 else low_f
	var across := along.orthogonal()
	var wrist := hand - along * 0.02
	# 袖口垂下來，但不垂到地面以下（坐著時袖子攤在腿上）。
	var drop := Vector2(0.012 * sway, minf(sleeve, maxf(0.02, -0.045 - wrist.y)))
	# 前臂快要直直舉起時袖子滑向手肘、貼著手臂，不再往下垂（垂下來會跟前臂疊在一起打結）。
	drop *= 1.0 - 0.85 * smoothstep(0.55, 0.95, -along.y)
	fills.append(smooth_outline([wrist + across * 0.017, hand + along * 0.03 + across * 0.012, hand + along * 0.036 - across * 0.011, wrist - across * 0.017]))
	if point:
		fills.append(_limb(hand + along * 0.02, hand + along * 0.075, 0.008, 0.004))
		fills.append(_limb(hand + along * 0.02 + across * 0.008, hand + along * 0.068 + across * 0.012, 0.007, 0.004))
	_cut(cuts, fills, lines, bands, SKIN)
	var edge: Array[Vector2] = [
		elbow - low_e * 0.04, wrist - low_f * 0.026, wrist + low_f * 0.03 + drop,
		elbow.lerp(wrist, 0.5) + low_f * 0.045 + drop * 0.8, elbow + low_e * 0.05 + drop * 0.3,
	]
	var outline: Array[Vector2] = edge.duplicate()
	if not fore_only:
		outline.push_front(shoulder - low_u * 0.036)
		outline.append(shoulder + low_u * 0.045)
		edge.push_front(shoulder.lerp(elbow, 0.35) - low_u * 0.038)
		edge.append(shoulder + low_u * 0.045)
	fills.append(_simple(smooth_outline(outline)))
	lines.append(smooth_path(edge))
	# 袖口一道弧，肘彎一道褶。
	lines.append(PackedVector2Array([wrist - low_f * 0.02 - along * 0.012, wrist + low_f * 0.008 + drop * 0.5 - along * 0.022, wrist + low_f * 0.026 + drop * 0.95 - along * 0.008]))
	lines.append(PackedVector2Array([elbow - low_e * 0.012, elbow + low_e * 0.03 + drop * 0.2 + along * 0.03]))
	# 袖底背光的一面；長袖的袖口有緣邊。
	bands.append([PackedVector2Array([elbow + low_e * 0.03 + drop * 0.25, elbow.lerp(wrist, 0.5) + low_f * 0.028 + drop * 0.7, wrist + low_f * 0.016 + drop * 0.85]), SHADE, 0.02])
	if sleeve > 0.085:
		bands.append([PackedVector2Array([wrist - low_f * 0.016 - along * 0.004, wrist + low_f * 0.008 + drop * 0.5 - along * 0.012, wrist + low_f * 0.022 + drop * 0.9]), TRIM, 0.016])
	_cut(cuts, fills, lines, bands, ROBE + OPEN)


## 輪廓自己交叉時（手臂大幅甩開、折回，袖子垂下的一側與上臂對不上）三角化會失敗，整塊畫不出來：
## 交叉的輪廓取聯集（Clipper 的正向填充），留面積最大的那一塊外輪廓；本來就沒交叉的原樣不動。
static func _simple(shape: PackedVector2Array) -> PackedVector2Array:
	if not Geometry2D.triangulate_polygon(shape).is_empty():
		return shape
	var best := PackedVector2Array()
	var best_area := 0.0
	for piece in Geometry2D.offset_polygon(shape, 0.0005):
		var area := absf(_area(piece))
		if area > best_area and not Geometry2D.is_polygon_clockwise(piece):
			best = piece
			best_area = area
	return best if not best.is_empty() else Geometry2D.convex_hull(shape)


static func _area(shape: PackedVector2Array) -> float:
	var total := 0.0
	for i in shape.size():
		var a := shape[i]
		var b := shape[(i + 1) % shape.size()]
		total += a.x * b.y - b.x * a.y
	return total * 0.5


## 手臂朝 direction 伸時，袖子垂下來的那一側；手臂接近直直垂著時取身後那一邊。
static func _low_side(direction: Vector2) -> Vector2:
	var side := direction.orthogonal()
	if absf(side.y) < 0.45:
		return side if side.x > 0.0 else -side
	return side if side.y > 0.0 else -side


## 背著、掛著或拿在靠外那隻手上的持物：背劍、拂塵、葫蘆、杖、花籃。
static func _held_prop(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], look: Dictionary, hip: Vector2, neck: Vector2, hand: Vector2, time: float) -> void:
	match look["prop"]:
		Prop.SWORD_BACK:
			# 斜背在身後：鞘、劍格、劍柄、劍首，劍首垂一綹劍穗。
			var tip := hip + Vector2(-0.07, -0.02)
			var hilt := neck + Vector2(0.16, -0.17)
			var along := (hilt - tip).normalized()
			var guard := tip.lerp(hilt, 0.76)
			fills.append(_limb(tip, guard, 0.006, 0.014))
			fills.append(_limb(guard - along.orthogonal() * 0.034, guard + along.orthogonal() * 0.034, 0.009, 0.009))
			fills.append(_limb(guard, hilt, 0.008, 0.008))
			fills.append(_disc(hilt, 0.014))
			fills.append(_ribbon(hilt, Vector2(0.07, 0.15), 0.02, time, 2.8))
		Prop.WHISK:
			# 拂塵：一支柄，塵尾分兩綹垂下。
			var tip := hand + Vector2(-0.03, -0.25)
			fills.append(_limb(hand + Vector2(0.01, 0.04), tip, 0.007, 0.006))
			fills.append(_disc(tip, 0.011))
			fills.append(_ribbon(tip, Vector2(0.17, 0.22), 0.055, time, 1.6))
			fills.append(_ribbon(tip, Vector2(0.11, 0.27), 0.04, time + 0.6, 1.9))
		Prop.GOURD:
			# 腰後掛一隻葫蘆：兩節、一個塞子、一圈繫繩。
			var top := hip + Vector2(look["girth"] + 0.03, 0.035)
			fills.append(_disc(top, 0.027))
			fills.append(_disc(top + Vector2(0.006, 0.052), 0.04))
			fills.append(_limb(top + Vector2(0.0, -0.022), top + Vector2(-0.004, -0.045), 0.008, 0.006))
			lines.append(_circle(top + Vector2(0.003, 0.024), 0.02))
		Prop.STAFF:
			# 一根略彎的杖。貧者是丁字拐、僧人是掛環的錫杖，其餘杖頭盤一個節、掛一隻小葫蘆。
			var foot := Vector2(hand.x + 0.04, 0.0)
			var top := Vector2(hand.x + 0.035, neck.y - (0.24 if look["wealth"] != Wealth.POOR else 0.02))
			var mid := Vector2(hand.x + 0.025, top.y * 0.5)
			fills.append(_limb(foot, mid, 0.011, 0.01))
			fills.append(_limb(mid, top, 0.01, 0.009))
			if look["wealth"] == Wealth.POOR:
				fills.append(_limb(top + Vector2(-0.05, 0.004), top + Vector2(0.05, -0.004), 0.012, 0.012))
			elif look["school"] == School.SHI:
				fills.append(_disc(top + Vector2(0.0, -0.03), 0.034))
				lines.append(_circle(top + Vector2(0.0, -0.03), 0.02))
				for ring: float in [-1.0, 1.0]:
					fills.append(_disc(top + Vector2(0.03 * ring, 0.012), 0.013))
			else:
				fills.append(smooth_outline([top + Vector2(-0.012, 0.0), top + Vector2(-0.035, -0.035), top + Vector2(-0.01, -0.062), top + Vector2(0.03, -0.045), top + Vector2(0.02, -0.01)]))
				fills.append(_disc(top + Vector2(0.03, 0.045), 0.014))
				fills.append(_disc(top + Vector2(0.032, 0.07), 0.02))
		Prop.BASKET:
			# 花籃：提樑、籃身，籃口露出幾朵花。
			var rim := hand + Vector2(0.03, 0.10)
			fills.append(_limb(hand, rim + Vector2(-0.055, 0.0), 0.005, 0.005))
			fills.append(_limb(hand, rim + Vector2(0.055, 0.0), 0.005, 0.005))
			fills.append(smooth_outline([rim + Vector2(-0.062, 0.0), rim + Vector2(-0.05, 0.06), rim + Vector2(0.0, 0.075), rim + Vector2(0.05, 0.06), rim + Vector2(0.062, 0.0)]))
			for bloom: Vector2 in [Vector2(-0.035, -0.012), Vector2(0.0, -0.022), Vector2(0.036, -0.01)]:
				fills.append(_disc(rim + bloom, 0.017))
			lines.append(PackedVector2Array([rim + Vector2(-0.055, 0.025), rim + Vector2(0.055, 0.025)]))


## 拿在朝前那隻手上的持物：扇、笛、荷花、笏板、念珠、書卷。
static func _front_prop(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], look: Dictionary, hand: Vector2, time: float) -> void:
	match look["prop"]:
		Prop.FAN:
			var fan: Array[Vector2] = [hand]
			for i in 9:
				var rib := hand + Vector2.from_angle(deg_to_rad(188.0 + 13.0 * i)) * 0.13
				fan.append(rib)
				if i % 2 == 1:
					lines.append(PackedVector2Array([hand.lerp(rib, 0.2), rib]))
			fills.append(PackedVector2Array(fan))
		Prop.FLUTE:
			# 一管橫笛，笛尾繫一綹穗。
			fills.append(_limb(hand + Vector2(-0.17, -0.05), hand + Vector2(0.11, 0.03), 0.007, 0.007))
			fills.append(_ribbon(hand + Vector2(0.10, 0.03), Vector2(0.03, 0.12), 0.014, time, 2.6))
		Prop.LOTUS:
			# 一枝荷花：長梗、一片荷葉、五瓣花。
			var bloom := hand + Vector2(-0.05, -0.27)
			fills.append(_limb(hand + Vector2(0.01, 0.05), bloom, 0.005, 0.004))
			fills.append(smooth_outline([hand + Vector2(-0.03, -0.12), hand + Vector2(-0.10, -0.15), hand + Vector2(-0.12, -0.12), hand + Vector2(-0.06, -0.10)]))
			for i in 5:
				var toward := Vector2.from_angle(deg_to_rad(-90.0 + (i - 2) * 34.0))
				var side := toward.orthogonal()
				fills.append(smooth_outline([bloom, bloom + toward * 0.045 + side * 0.02, bloom + toward * 0.085, bloom + toward * 0.045 - side * 0.02]))
		Prop.TABLET:
			# 笏板：一塊長板斜持在胸前。
			fills.append(_limb(hand + Vector2(0.006, 0.04), hand + Vector2(-0.035, -0.23), 0.016, 0.021))
		Prop.BEADS:
			# 一串念珠垂在手下。
			for i in 12:
				fills.append(_disc(hand + Vector2(sin(TAU * i / 12.0) * 0.03, 0.065 + cos(TAU * i / 12.0) * 0.055), 0.008))
		Prop.SCROLL:
			# 一卷書：卷身與兩端的軸頭。
			fills.append(_limb(hand + Vector2(-0.075, -0.03), hand + Vector2(0.06, 0.022), 0.02, 0.02))
			fills.append(_limb(hand + Vector2(-0.092, -0.037), hand + Vector2(0.077, 0.029), 0.006, 0.006))
			lines.append(PackedVector2Array([hand + Vector2(-0.04, -0.034), hand + Vector2(-0.052, 0.004)]))


## 鬍鬚：整部鬍鬚是一塊、一道輪廓、一層——髭在嘴上、鬚在頦下、髯在兩頰沿著下頷，連成一片，
## 下緣依樣式分出幾個尖；鬚絲是塊裡面的淡線，不另成一層。鬢屬於頭髮（見 _hair）。
static func _whiskers(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, cuts: Array, look: Dictionary, head: Vector2, radius: float, tilt: float, sway: float) -> void:
	var style: int = look["whiskers"]
	if style == Whiskers.NONE:
		return
	var length: float = maxf(look["beard"], 0.05)
	var drift := Vector2(0.012 * sway, 0.0)
	var mass: Array[Vector2] = []
	# 鬚絲的走向：[根, 尖]。
	var strokes: Array = []
	match style:
		Whiskers.GOATEE:
			# 山羊鬚：頦下一綹，收成一個尖。
			var tip := _on_face(look, head, radius, tilt, -0.32, 1.0) + Vector2(0.0, length) + drift
			mass = [
				_on_face(look, head, radius, tilt, -0.56, 0.84), _on_face(look, head, radius, tilt, -0.3, 0.9), _on_face(look, head, radius, tilt, -0.06, 0.94),
				_on_face(look, head, radius, tilt, -0.12, 1.0) + Vector2(0.0, length * 0.3) + drift * 0.3,
				_on_face(look, head, radius, tilt, -0.24, 1.0) + Vector2(0.0, length * 0.72) + drift * 0.7, tip,
				_on_face(look, head, radius, tilt, -0.4, 1.0) + Vector2(0.0, length * 0.72) + drift * 0.7,
				_on_face(look, head, radius, tilt, -0.52, 0.96) + Vector2(0.0, length * 0.3) + drift * 0.3,
			]
			strokes = [[_on_face(look, head, radius, tilt, -0.3, 0.98), tip]]
		Whiskers.THREE, Whiskers.FIVE:
			# 長髯：自遠頰沿下頷到近頰連成一片，下緣分三個尖。輪廓與鬚絲都是曲線資料。
			_draw_part(part_data("whiskers")["long"]["pieces"], _face_frame(look, head, radius, tilt, length, drift), radius, fills, lines, bands, cuts, false)
		Whiskers.FULL:
			# 絡腮：沿著下頷一圈短鬚，下緣參差。
			var depth := minf(length, 0.07)
			mass = [
				_on_face(look, head, radius, tilt, 0.6, 0.3), _on_face(look, head, radius, tilt, 0.4, 0.62), _on_face(look, head, radius, tilt, 0.04, 0.84),
				_on_face(look, head, radius, tilt, -0.34, 0.92), _on_face(look, head, radius, tilt, -0.7, 0.72),
			]
			for hang: Array in [[-0.84, 0.7, 0.3], [-0.6, 0.9, 1.0], [-0.34, 1.0, 0.75], [-0.1, 1.02, 1.0], [0.2, 0.94, 0.7], [0.44, 0.78, 0.9], [0.74, 0.4, 0.3]]:
				mass.append(_on_face(look, head, radius, tilt, hang[0], hang[1]) + Vector2(0.004 * sway, depth * hang[2]))
	if not mass.is_empty():
		fills.append(smooth_outline(mass))
	for stroke: Array in strokes:
		var root: Vector2 = stroke[0]
		var tip: Vector2 = stroke[1]
		bands.append([PackedVector2Array([root, root.lerp(tip, 0.5) + drift * 0.3, root.lerp(tip, 0.9)]), SHADE, radius * 0.035])
	if style == Whiskers.MOUSTACHE or style == Whiskers.FIVE or style == Whiskers.FULL:
		# 髭：嘴上兩撇，往嘴角兩邊垂；五綹的髭長，垂過下巴。
		var droop := 0.3 if style == Whiskers.FIVE else 0.0
		for whisker: Array in [[-0.5, 0.5, -0.62, 0.6, -0.7, 0.78 + droop], [-0.42, 0.5, -0.28, 0.6, -0.16, 0.78 + droop]]:
			bands.append([PackedVector2Array([
				_on_face(look, head, radius, tilt, whisker[0], whisker[1]), _on_face(look, head, radius, tilt, whisker[2], whisker[3]),
				_on_face(look, head, radius, tilt, whisker[4], whisker[5]),
			]), BEARD, radius * 0.09])
	_cut(cuts, fills, lines, bands, BEARD, Layer.BEARD)


## 臉上的一點（頭半徑為 1，x 負的是面朝的那一邊）。下巴那一側的下半張臉跟著臉的長短伸縮，和臉的輪廓一致。
static func _on_face(look: Dictionary, head: Vector2, radius: float, tilt: float, x: float, y: float) -> Vector2:
	var stretch := smoothstep(0.3, 0.7, y) * (1.0 - smoothstep(0.0, 0.4, x))
	return head + Vector2(x, y * lerpf(1.0, look["jaw"], stretch)).rotated(tilt) * radius


## 讀一個部件的曲線資料（讀過的留著）。
static func part_data(part: String) -> Dictionary:
	if not _parts.has(part):
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(PARTS_DIR + part + ".json"))
		_parts[part] = parsed if parsed is Dictionary else {}
	return _parts[part]


## 三次貝茲曲線連成的線。nodes 每一項是 [進來的控制點, 節點, 出去的控制點]；closed 是首尾相接圍成一塊。
static func bezier(nodes: Array, closed := true) -> PackedVector2Array:
	var curve := PackedVector2Array()
	var count := nodes.size()
	for i in (count if closed else count - 1):
		var from: Array = nodes[i]
		var to: Array = nodes[(i + 1) % count]
		var start: Vector2 = from[1]
		for step in BEZIER_STEPS:
			curve.append(start.bezier_interpolate(from[2], to[0], to[1], step / float(BEZIER_STEPS)))
	if not closed:
		curve.append(nodes[count - 1][1])
	return curve


## 照曲線資料畫一個部件。資料檔的格式：
##   {"<部件名>": {"pieces": [ <一層>, ... ]}}
##   一層 = {"layer": 層次表上的名稱, "part": 部位名稱（決定顏色）, "open": 不自動勾輪廓（可省略）,
##           "fills": [ 封閉曲線, ... ]   —— 同一層的幾塊會併成一個輪廓、勾一道邊,
##           "lines": [ 開口曲線, ... ]   —— 墨線,
##           "bands": [ {"curve": 開口曲線, "part": 部位名稱, "width": 寬度} ] —— 色帶；part 寫 SHADE 是這一層底色的暗面 }
##   曲線 = [ 節點, ... ]；節點 = [進來的控制點偏移, 位置, 出去的控制點偏移]，三項都是 [x, y, 垂x, 垂y]。
## frame 把 (x, y, 垂x, 垂y) 換成畫面上的位置，由呼叫的一方決定座標怎麼解讀（臉上的部件見 _face_frame，
## 拿在手上的東西見 _rigid_frame）；unit 是 width 的單位長。seal 為否時最後一層留給呼叫的一方收尾（還要往裡面加東西）。
static func _draw_part(pieces: Array, frame: Callable, unit: float, fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, cuts: Array, seal := true) -> void:
	for index in pieces.size():
		var piece: Dictionary = pieces[index]
		for nodes: Array in piece.get("fills", []):
			fills.append(bezier(_placed_nodes(nodes, frame)))
		for nodes: Array in piece.get("lines", []):
			lines.append(bezier(_placed_nodes(nodes, frame), false))
		for band: Dictionary in piece.get("bands", []):
			bands.append([bezier(_placed_nodes(band["curve"], frame), false), PART_NAMES[band["part"]], float(band["width"]) * unit])
		if seal or index < pieces.size() - 1:
			_cut(cuts, fills, lines, bands, PART_NAMES[piece["part"]] + (OPEN if piece.get("open", false) else 0), Layer[piece["layer"]])


static func _placed_nodes(nodes: Array, frame: Callable) -> Array:
	var placed: Array = []
	for node: Array in nodes:
		var at: Array = node[1]
		var trio: Array = []
		for offset: Array in [node[0], [0.0, 0.0, 0.0, 0.0], node[2]]:
			trio.append(frame.call(at[0] + offset[0], at[1] + offset[1], at[2] + offset[2], at[3] + offset[3]))
		placed.append(trio)
	return placed


## 臉上的座標：x、y 是臉上的位置（頭半徑為 1，x 負的是面朝的那一邊，y 往下，見 _on_face），
## 垂x、垂y 乘 drop（垂下來的長度，例如鬚長）；垂得越低的點被 drift 帶得越遠（隨風擺）。
static func _face_frame(look: Dictionary, head: Vector2, radius: float, tilt: float, drop := 0.0, drift := Vector2.ZERO) -> Callable:
	return func(x: float, y: float, hang_x: float, hang_y: float) -> Vector2:
		return _on_face(look, head, radius, tilt, x, y) + Vector2(hang_x, hang_y) * drop + drift * hang_y


## 剛體的座標：x、y 以 origin 為原點、轉 angle、乘 size；垂x、垂y 乘 drop，不跟著轉（往下垂的穗子）。
static func _rigid_frame(origin: Vector2, angle: float, size: float, drop := 0.0, drift := Vector2.ZERO) -> Callable:
	return func(x: float, y: float, hang_x: float, hang_y: float) -> Vector2:
		return origin + Vector2(x, y).rotated(angle) * size + Vector2(hang_x, hang_y) * drop + drift * hang_y


## 髮式，以及從腦後飄出去的髮帶。
static func _hair(fills: Array[PackedVector2Array], lines: Array[PackedVector2Array], bands: Array, cuts: Array, look: Dictionary, head: Vector2, radius: float, time: float, wind: float, tilt: float) -> void:
	if look["hair"] == Hair.SHAVEN:
		return
	var back := head + Vector2(radius * 0.7, -radius * 0.5)
	# 頭上的髮：自額前的髮際蓋過頭頂與後腦，鬢角繞過耳前，貼到頸後。
	var cap: Array[Vector2] = []
	for point: Vector2 in [
		Vector2(-0.74, -0.56), Vector2(-0.5, -0.97), Vector2(0.0, -1.08), Vector2(0.55, -0.99), Vector2(0.94, -0.6), Vector2(1.04, -0.05),
		Vector2(0.9, 0.6), Vector2(0.6, 0.86), Vector2(0.5, 0.45), Vector2(0.47, 0.0), Vector2(0.3, -0.42), Vector2(-0.2, -0.64),
	]:
		cap.append(head + point.rotated(tilt) * radius)
	fills.append(smooth_outline(cap))
	if look["whiskers"] in [Whiskers.THREE, Whiskers.FIVE, Whiskers.FULL]:
		# 鬢：耳前一道，自髮際連到下頷角，接上髯。
		fills.append(_limb(_on_face(look, head, radius, tilt, 0.44, -0.05), _on_face(look, head, radius, tilt, 0.5, 0.6), radius * 0.1, radius * 0.07))
	_cut(cuts, fills, lines, bands, HAIR, Layer.HAIR)
	match look["hair"]:
		Hair.TOPKNOT:
			# 束髮：頂上一個髻，橫插一支簪。
			var knot := head + Vector2(radius * 0.12, -radius * 1.22)
			fills.append(smooth_outline([knot + Vector2(-radius * 0.42, radius * 0.25), knot + Vector2(-radius * 0.36, -radius * 0.3), knot + Vector2(0.0, -radius * 0.52), knot + Vector2(radius * 0.4, -radius * 0.25), knot + Vector2(radius * 0.4, radius * 0.3)]))
			fills.append(_limb(knot + Vector2(-radius * 0.95, -radius * 0.05), knot + Vector2(radius * 0.85, -radius * 0.02), 0.0035, 0.0045))
			fills.append(_disc(knot + Vector2(radius * 0.88, -radius * 0.02), 0.007))
			_cut(cuts, fills, lines, bands, HAIR)
		Hair.CROWN:
			# 戴冠：一頂高冠，橫貫一支簪，一條冠纓垂到胸前。
			var base := head + Vector2(radius * 0.05, -radius * 0.86)
			fills.append(smooth_outline([
				base + Vector2(-radius * 0.44, 0.02), base + Vector2(-radius * 0.42, -radius * 0.7), base + Vector2(-radius * 0.2, -radius * 1.08),
				base + Vector2(radius * 0.24, -radius * 1.08), base + Vector2(radius * 0.46, -radius * 0.7), base + Vector2(radius * 0.44, 0.02),
			]))
			fills.append(_limb(base + Vector2(-radius * 0.95, -radius * 0.45), base + Vector2(radius * 0.95, -radius * 0.42), 0.004, 0.004))
			lines.append(PackedVector2Array([base + Vector2(-radius * 0.34, -radius * 0.42), base + Vector2(radius * 0.42, -radius * 0.42)]))
			_cut(cuts, fills, lines, bands, HAT)
			var knot := _on_face(look, head, radius, tilt, -0.2, 1.02)
			bands.append([PackedVector2Array([
				_on_face(look, head, radius, tilt, 0.36, -0.7), _on_face(look, head, radius, tilt, 0.42, 0.1), _on_face(look, head, radius, tilt, 0.2, 0.8), knot,
			]), HAT, radius * 0.07])
			fills.append(_ribbon(knot, Vector2(0.004, 0.07), 0.008, time, 2.2))
			fills.append(_ribbon(knot, Vector2(0.016, 0.06), 0.008, time + 0.5, 2.6))
			_cut(cuts, fills, lines, bands, HAT, Layer.UNDER_CHIN)
		Hair.LOOSE:
			# 披髮：兩綹長髮垂到背後，耳前鬢邊一綹。
			var drift := wind * 0.8 + 0.01 * sin(time * 2.2)
			fills.append(smooth_outline([
				head + Vector2(-radius * 0.25, -radius * 1.04), head + Vector2(radius * 0.9, -radius * 0.6), head + Vector2(radius * 1.3 + drift * 0.4, radius * 1.4),
				head + Vector2(radius * 1.45 + drift, radius * 3.2), head + Vector2(radius * 1.1 + drift * 1.6, radius * 4.9),
				head + Vector2(radius * 0.85 + drift, radius * 3.3), head + Vector2(radius * 0.5, radius * 1.6), head + Vector2(radius * 0.25, radius * 0.2),
			]))
			fills.append(smooth_outline([
				head + Vector2(radius * 0.6, radius * 0.4), head + Vector2(radius * 1.7 + drift * 1.2, radius * 2.4),
				head + Vector2(radius * 1.9 + drift * 2.0, radius * 4.0), head + Vector2(radius * 1.2 + drift, radius * 2.6),
			]))
			_cut(cuts, fills, lines, bands, HAIR, Layer.BEHIND_HEAD)
			fills.append(_ribbon(_on_face(look, head, radius, tilt, 0.46, -0.1), Vector2(0.004, radius * 1.9), 0.012, time, 2.0))
			_cut(cuts, fills, lines, bands, HAIR, Layer.UNDER_CHIN)
			return
		Hair.HAT:
			# 斗笠：笠沿微微上翹，頷下一條繫帶。
			fills.append(smooth_outline([
				head + Vector2(-radius * 2.45, -radius * 0.42), head + Vector2(-radius * 1.2, -radius * 0.85), head + Vector2(-radius * 0.15, -radius * 1.8),
				head + Vector2(radius * 0.15, -radius * 1.8), head + Vector2(radius * 1.2, -radius * 0.85), head + Vector2(radius * 2.45, -radius * 0.42),
				head + Vector2(radius * 1.2, -radius * 0.5), head + Vector2(0.0, -radius * 0.62), head + Vector2(-radius * 1.2, -radius * 0.5),
			]))
			lines.append(PackedVector2Array([head + Vector2(-radius * 0.5, -radius * 1.25), head + Vector2(-radius * 1.4, -radius * 0.62)]))
			lines.append(PackedVector2Array([head + Vector2(radius * 0.5, -radius * 1.25), head + Vector2(radius * 1.4, -radius * 0.62)]))
			_cut(cuts, fills, lines, bands, HAT)
			fills.append(_ribbon(head + Vector2(-radius * 0.45, radius * 0.8), Vector2(0.03 + wind * 0.5, 0.14), 0.012, time, 2.4))
			_cut(cuts, fills, lines, bands, HAT, Layer.UNDER_CHIN)
			return
		Hair.TWIN_BUNS:
			# 雙髻：左右各一個髻，各繫一條帶。
			for side: float in [-1.0, 1.0]:
				var bun := head + Vector2(radius * 0.66 * side + radius * 0.08, -radius * 1.02)
				fills.append(smooth_outline([bun + Vector2(-radius * 0.36, radius * 0.2), bun + Vector2(-radius * 0.3, -radius * 0.3), bun + Vector2(radius * 0.05, -radius * 0.45), bun + Vector2(radius * 0.38, -radius * 0.2), bun + Vector2(radius * 0.32, radius * 0.25)]))
				fills.append(_ribbon(bun + Vector2(radius * 0.2, 0.0), Vector2(look["ribbon"] * (0.3 + wind), look["ribbon"] * 0.35), 0.014, time + side, 3.8))
			_cut(cuts, fills, lines, bands, HAIR)
		Hair.HIGH_BUN:
			# 高髻：頂上一個高高的髻，斜插一支步搖，髻根簪一朵花，鬢邊垂一綹髮，腦後一個小髻，長髮垂到背後。
			fills.append(smooth_outline([
				head + Vector2(-radius * 0.35, -radius * 0.85), head + Vector2(-radius * 0.5, -radius * 1.65), head + Vector2(-radius * 0.1, -radius * 2.25),
				head + Vector2(radius * 0.5, -radius * 2.05), head + Vector2(radius * 0.65, -radius * 1.3), head + Vector2(radius * 0.45, -radius * 0.8),
			]))
			fills.append(_disc(head + Vector2(radius * 1.05, -radius * 0.25), radius * 0.42))
			fills.append(_limb(head + Vector2(-radius * 1.15, -radius * 1.75), head + Vector2(radius * 0.6, -radius * 1.35), 0.004, 0.004))
			fills.append(_ribbon(head + Vector2(-radius * 1.1, -radius * 1.72), Vector2(-0.006, radius * 0.9), 0.012, time, 3.0))
			_cut(cuts, fills, lines, bands, HAIR)
			fills.append(_disc(head + Vector2(-radius * 0.52, -radius * 0.98), radius * 0.2))
			fills.append(_disc(head + Vector2(-radius * 0.3, -radius * 1.2), radius * 0.13))
			_cut(cuts, fills, lines, bands, SHAWL)
			fills.append(_ribbon(_on_face(look, head, radius, tilt, 0.46, -0.1), Vector2(0.006, radius * 2.0), 0.012, time + 0.4, 2.2))
			_cut(cuts, fills, lines, bands, HAIR, Layer.UNDER_CHIN)
			var flow := wind * 0.8 + 0.01 * sin(time * 2.2)
			fills.append(smooth_outline([
				head + Vector2(radius * 0.2, -radius * 0.7), head + Vector2(radius * 1.0, -radius * 0.2), head + Vector2(radius * 1.25 + flow * 0.4, radius * 1.6),
				head + Vector2(radius * 1.3 + flow, radius * 3.4), head + Vector2(radius * 0.95 + flow * 1.5, radius * 4.6),
				head + Vector2(radius * 0.75 + flow, radius * 3.2), head + Vector2(radius * 0.55, radius * 1.5), head + Vector2(radius * 0.35, radius * 0.3),
			]))
			_cut(cuts, fills, lines, bands, HAIR, Layer.BEHIND_HEAD)
			return
		Hair.SCHOLAR_CAP:
			# 儒巾：方頂的巾，腦後兩條長帶。
			fills.append(smooth_outline([
				head + Vector2(-radius * 0.78, -radius * 0.62), head + Vector2(-radius * 0.8, -radius * 1.45), head + Vector2(-radius * 0.1, -radius * 1.7),
				head + Vector2(radius * 0.75, -radius * 1.55), head + Vector2(radius * 0.92, -radius * 0.7), head + Vector2(0.0, -radius * 0.82),
			]))
			lines.append(PackedVector2Array([head + Vector2(-radius * 0.74, -radius * 0.92), head + Vector2(radius * 0.86, -radius * 0.98)]))
			_cut(cuts, fills, lines, bands, HAT)
			fills.append(_ribbon(back, Vector2(0.05 + wind * 1.5, 0.24), 0.026, time, 2.4))
			fills.append(_ribbon(back + Vector2(0.01, 0.0), Vector2(0.09 + wind * 1.8, 0.2), 0.022, time + 0.7, 2.8))
			_cut(cuts, fills, lines, bands, HAT, Layer.BEHIND_HEAD)
			return
	fills.append(_ribbon(back, Vector2(look["ribbon"] * (0.6 + wind * 2.0), look["ribbon"] * (0.45 - wind)), 0.024, time, 3.4))
	fills.append(_ribbon(back + Vector2(0.0, 0.012), Vector2(look["ribbon"] * (0.45 + wind * 2.0), look["ribbon"] * (0.6 - wind)), 0.018, time + 0.8, 4.1))
	_cut(cuts, fills, lines, bands, TRIM, Layer.BEHIND_HEAD)


## 把控制點連成圓順的封閉輪廓（Catmull-Rom）。
static func smooth_outline(points: Array[Vector2]) -> PackedVector2Array:
	var out := PackedVector2Array()
	var count := points.size()
	for i in count:
		var from := points[i]
		var to := points[(i + 1) % count]
		for k in CURVE_STEPS:
			out.append(from.cubic_interpolate(to, points[(i - 1 + count) % count], points[(i + 2) % count], float(k) / CURVE_STEPS))
	return out


## 把控制點連成圓順的一筆（不封口）。
static func smooth_path(points: Array[Vector2]) -> PackedVector2Array:
	var out := PackedVector2Array()
	var last := points.size() - 1
	for i in last:
		for k in CURVE_STEPS:
			out.append(points[i].cubic_interpolate(points[i + 1], points[maxi(i - 1, 0)], points[mini(i + 2, last)], float(k) / CURVE_STEPS))
	out.append(points[last])
	return out


## 兩端寬度不同的一段（四邊形）。
static func _limb(from: Vector2, to: Vector2, from_half: float, to_half: float) -> PackedVector2Array:
	var side := (to - from).normalized().orthogonal()
	return PackedVector2Array([from - side * from_half, to - side * to_half, to + side * to_half, from + side * from_half])


static func _disc(center: Vector2, radius: float) -> PackedVector2Array:
	var points := PackedVector2Array()
	for i in ROUND:
		points.append(center + Vector2.from_angle(TAU * i / ROUND) * radius)
	return points


## 一圈線（折線首尾相接）。
static func _circle(center: Vector2, radius: float) -> PackedVector2Array:
	var points := _disc(center, radius)
	points.append(points[0])
	return points


## 一條飄帶：由 from 伸到 from + reach，越遠越細、擺得越大；taper 是尾端收掉多少寬度，布帛收得少、尾端是圓的。
static func _ribbon(from: Vector2, reach: Vector2, width: float, time: float, speed: float, taper := 0.88) -> PackedVector2Array:
	var side := reach.normalized().orthogonal()
	var upper := PackedVector2Array()
	var lower := PackedVector2Array()
	for i in 11:
		var t := i / 10.0
		var centre := from + reach * t + side * sin(time * speed - t * 4.0) * 0.05 * t
		var half := width * (1.0 - t * taper) * 0.5
		upper.append(centre - side * half)
		lower.append(centre + side * half)
	if taper < 0.8:
		upper.append(from + reach * 1.03 + side * sin(time * speed - 4.0) * 0.05)
	lower.reverse()
	return upper + lower
