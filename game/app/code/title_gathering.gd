class_name TitleGathering
extends Control
## 標題畫面的人（GDD 9.1）：白描設色畫法——墨線勾形、淡彩填色。崖頂松下的石台只有三人：富者與貴者對坐下棋，
## 老道站在後面指點，松下一爐香。其餘的人散在山水裡各做各的事（WANDERERS）：山徑上拄杖趕路的貧叟、山腰石臺打坐的僧人、
## 水岸持蓮的女子、霧裡小舟上垂釣的漁翁、土坡上吹笛的童子。連同天上御劍的那一個，男女老少富貴貧賤、儒釋道都在場。
## 各人的身分固定，長相由 CultivatorFigure 依種子生成，每次開啟是不同的幾個人；站得遠的畫得小一點、
## 先畫，被前面的人遮住。衣帶、松針與香煙隨風擺動。版面以 1080 高的畫面為準，貼著畫面左下角（一角構圖），右邊留給標題與選單。

## 站立修士的身高。
const STATURE := 285.0
## 崖頂的臺面（從略高處看下去）：遠緣、近緣（崖唇）各一道線，人站在兩者之間。
## x 是離畫面左緣的距離取負（-1120 是往畫面裡 1120），y 是離上緣的距離。
const FAR_EDGE: Array[Vector2] = [
	Vector2(-1120.0, 868.0), Vector2(-1040.0, 836.0), Vector2(-900.0, 824.0), Vector2(-700.0, 828.0), Vector2(-480.0, 820.0),
	Vector2(-260.0, 824.0), Vector2(-60.0, 812.0), Vector2(80.0, 806.0),
]
const LIP: Array[Vector2] = [
	Vector2(-1120.0, 868.0), Vector2(-1050.0, 898.0), Vector2(-900.0, 906.0), Vector2(-720.0, 896.0), Vector2(-540.0, 908.0),
	Vector2(-340.0, 900.0), Vector2(-150.0, 906.0), Vector2(80.0, 892.0),
]
## 崖尖那一側的邊：自崖尖往下收，崖壁的色塊與墨線共用這一道。
const TIP_EDGE: Array[Vector2] = [
	Vector2(-1120.0, 868.0), Vector2(-1096.0, 930.0), Vector2(-1040.0, 968.0), Vector2(-1075.0, 1040.0), Vector2(-1050.0, 1120.0),
]
## 崖壁的皴：自崖唇往下的幾道裂理（各三點）；畫的時候裁在崖壁裡面。
const CRACKS := [
	[Vector2(-1060.0, 902.0), Vector2(-1020.0, 960.0), Vector2(-1045.0, 1030.0)],
	[Vector2(-1000.0, 906.0), Vector2(-985.0, 950.0), Vector2(-1002.0, 985.0)],
	[Vector2(-900.0, 908.0), Vector2(-878.0, 975.0), Vector2(-900.0, 1050.0)],
	[Vector2(-800.0, 902.0), Vector2(-812.0, 950.0), Vector2(-796.0, 990.0)],
	[Vector2(-700.0, 898.0), Vector2(-722.0, 965.0), Vector2(-695.0, 1040.0)],
	[Vector2(-540.0, 910.0), Vector2(-515.0, 980.0), Vector2(-538.0, 1050.0)],
	[Vector2(-430.0, 906.0), Vector2(-442.0, 950.0), Vector2(-428.0, 985.0)],
	[Vector2(-330.0, 902.0), Vector2(-352.0, 965.0), Vector2(-325.0, 1035.0)],
	[Vector2(-150.0, 908.0), Vector2(-128.0, 975.0), Vector2(-150.0, 1045.0)],
	[Vector2(-40.0, 902.0), Vector2(-52.0, 950.0), Vector2(-36.0, 990.0)],
]
## 松由後往前畫的次序：遠處的松針、枝、幹、樹皮的鱗、近處的松針。
const PINE_ORDER: Array[StringName] = [&"far_needles", &"branches", &"trunk", &"bark", &"near_needles"]
## 松幹的輪廓與三根枝（各三點，第一點是枝根、在幹裡面）；枝根粗、枝梢細。
const TRUNK: Array[Vector2] = [
	Vector2(-140.0, 862.0), Vector2(-152.0, 740.0), Vector2(-112.0, 640.0), Vector2(-150.0, 548.0), Vector2(-235.0, 488.0),
	Vector2(-330.0, 462.0), Vector2(-332.0, 448.0), Vector2(-225.0, 462.0), Vector2(-128.0, 520.0), Vector2(-84.0, 628.0),
	Vector2(-108.0, 742.0), Vector2(-52.0, 866.0),
]
const BRANCHES := [
	[Vector2(-120.0, 600.0), Vector2(-30.0, 560.0), Vector2(40.0, 548.0)],
	[Vector2(-150.0, 540.0), Vector2(-190.0, 440.0), Vector2(-160.0, 372.0)],
	[Vector2(-300.0, 458.0), Vector2(-420.0, 470.0), Vector2(-520.0, 500.0)],
]
const BRANCH_ROOT := 13.0
const BRANCH_TIP := 3.0
## 樹皮的鱗：幹上幾處短弧的位置。
const BARK: Array[Vector2] = [
	Vector2(-118.0, 800.0), Vector2(-96.0, 760.0), Vector2(-128.0, 720.0), Vector2(-104.0, 680.0), Vector2(-112.0, 610.0),
	Vector2(-132.0, 570.0), Vector2(-176.0, 520.0), Vector2(-240.0, 478.0),
]
## 一層層松針：中心、半寬、有幾簇、是不是遠處的（在枝幹後面，畫得淡）。
const NEEDLES := [
	[Vector2(-360.0, 440.0), 170.0, 7, false], [Vector2(-180.0, 368.0), 150.0, 6, true], [Vector2(-500.0, 492.0), 110.0, 5, false],
	[Vector2(-20.0, 540.0), 120.0, 5, false], [Vector2(-270.0, 320.0), 100.0, 4, true], [Vector2(-60.0, 440.0), 110.0, 5, false],
]
## 石台上的三人：腳下的位置、遠近（1 是最前面，小於 1 的站得遠、畫得小）、朝向（1 朝畫面左緣、−1 朝畫面中間）、姿勢、身分。
## 依由遠到近的順序排，照這個順序畫。
const CAST := [
	[Vector2(-545.0, 838.0), 0.94, -1.0, CultivatorFigure.Pose.GESTURE, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.ELDER, "wealth": CultivatorFigure.Wealth.PLAIN,
		"school": CultivatorFigure.School.DAO, "prop": CultivatorFigure.Prop.WHISK,
	}],
	[Vector2(-440.0, 872.0), 1.0, -1.0, CultivatorFigure.Pose.MEDITATE, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.ADULT, "wealth": CultivatorFigure.Wealth.RICH,
		"school": CultivatorFigure.School.RU, "prop": CultivatorFigure.Prop.TABLET,
	}],
	[Vector2(-735.0, 876.0), 1.0, 1.0, CultivatorFigure.Pose.PLAY, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.ADULT, "wealth": CultivatorFigure.Wealth.RICH,
		"school": CultivatorFigure.School.NONE, "prop": CultivatorFigure.Prop.NONE,
	}],
]
## 散在山水裡的人，各做各的事：欄位同 CAST，最後多一欄是腳下的東西——
## ledge 山腰的石臺（僧人打坐）、path 山徑（貧叟拄杖趕路）、boat 霧裡的小舟（漁翁垂釣）、knoll 土坡（童子吹笛）、bank 水岸（女子持蓮）。
## 離得遠，畫得小、墨淡；位置避開標題與選單。
const WANDERERS := [
	[Vector2(-700.0, 500.0), 0.26, -1.0, CultivatorFigure.Pose.STAND, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.ELDER, "wealth": CultivatorFigure.Wealth.POOR,
		"school": CultivatorFigure.School.NONE, "prop": CultivatorFigure.Prop.STAFF,
	}, &"path"],
	[Vector2(-1010.0, 590.0), 0.34, 1.0, CultivatorFigure.Pose.MEDITATE, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.ADULT, "wealth": CultivatorFigure.Wealth.POOR,
		"school": CultivatorFigure.School.SHI, "prop": CultivatorFigure.Prop.BEADS,
	}, &"ledge"],
	[Vector2(-1770.0, 905.0), 0.42, 1.0, CultivatorFigure.Pose.STAND, {
		"sex": CultivatorFigure.Sex.FEMALE, "age": CultivatorFigure.Age.ADULT, "wealth": CultivatorFigure.Wealth.PLAIN,
		"school": CultivatorFigure.School.NONE, "prop": CultivatorFigure.Prop.LOTUS,
	}, &"bank"],
	[Vector2(-1500.0, 1000.0), 0.44, 1.0, CultivatorFigure.Pose.STAND, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.ADULT, "wealth": CultivatorFigure.Wealth.POOR,
		"school": CultivatorFigure.School.NONE, "prop": CultivatorFigure.Prop.NONE, "hair": CultivatorFigure.Hair.HAT,
	}, &"boat"],
	[Vector2(-1195.0, 925.0), 0.5, 1.0, CultivatorFigure.Pose.STAND, {
		"sex": CultivatorFigure.Sex.MALE, "age": CultivatorFigure.Age.CHILD, "wealth": CultivatorFigure.Wealth.PLAIN,
		"school": CultivatorFigure.School.NONE, "prop": CultivatorFigure.Prop.FLUTE,
	}, &"knoll"],
]
## 棋枰畫在第幾個人之前（在對坐兩人的後面、指點者的前面）。
const BOARD_BEFORE := 1
const BOARD := Vector2(-588.0, 868.0)
const BURNER := Vector2(-212.0, 884.0)
## 設色：石的赭灰、松幹的赭、松針與草的墨綠、几的木色與几面、銅爐、棋罐。
const ROCK_WASH := Color("d6ccb3")
const ROCK_SHADE := Color("bdb39b")
const BARK_WASH := Color("b08f6c")
const LEAF := Color("33503f")
const WOOD := Color("a67c52")
const BOARD_FACE := Color("e8d5a4")
const BRONZE := Color("8c8a68")
const WARE := Color("f5efdf")

## 各人的種子（順序同 roster()）；標題畫面每次開啟另取。
var seeds: Array[int] = [101, 117, 103, 105, 108, 114, 121, 111]:
	set(value):
		seeds = value
		_looks.clear()
		var everyone := roster()
		for i in mini(seeds.size(), everyone.size()):
			_looks.append(CultivatorFigure.traits(seeds[i], everyone[i][4]))
		queue_redraw()

var _looks: Array[Dictionary] = []
var _time := 0.0
var _grow := 1.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)
	if _looks.is_empty():
		seeds = seeds


func _process(delta: float) -> void:
	_time += delta
	queue_redraw()


## 畫面上所有的人：石台上的三人在前，散在山水裡的在後。
static func roster() -> Array:
	return CAST + WANDERERS


## 第 index 個人在畫面上佔的範圍（roster() 的順序）。
func figure_rect(index: int) -> Rect2:
	var who: Array = roster()[index]
	var tall: float = STATURE * who[1] * scale_for(get_viewport_rect().size)
	var foot := place(who[0])
	return Rect2(foot.x - tall * 0.3, foot.y - tall, tall * 0.6, tall)


## 崖壁的外形（版面座標）：崖唇、畫面邊緣、底、崖尖那一側的邊。
static func cliff_wall() -> PackedVector2Array:
	var wall := CultivatorFigure.smooth_path(LIP)
	wall.append(Vector2(80.0, 1120.0))
	var tip := CultivatorFigure.smooth_path(TIP_EDGE)
	tip.reverse()
	# 崖尖那一點已經在崖唇的開頭。
	tip.resize(tip.size() - 1)
	wall.append_array(tip)
	return wall


## 崖壁的裂理（版面座標）：每一道都裁在崖壁裡面。
static func cliff_cracks() -> Array[PackedVector2Array]:
	var wall := cliff_wall()
	var cracks: Array[PackedVector2Array] = []
	for points: Array in CRACKS:
		var crack: Array[Vector2] = []
		crack.assign(points)
		var inside := Geometry2D.intersect_polyline_with_polygon(CultivatorFigure.smooth_path(crack), wall)
		cracks.append(inside[0] if not inside.is_empty() else PackedVector2Array())
	return cracks


## 版面座標換成畫面座標：整組貼著畫面左下角，畫面較窄時縮小，不壓到右邊的標題與選單。
func place(point: Vector2) -> Vector2:
	var bounds := get_viewport_rect().size
	var grow := scale_for(bounds)
	return Vector2(-point.x * grow, bounds.y - (1080.0 - point.y) * grow)


## 版面的縮放：以 1920×1080 為準，取高與寬兩邊較小的比例。
static func scale_for(bounds: Vector2) -> float:
	return minf(bounds.y / 1080.0, bounds.x / 1920.0 * 0.96)


func _draw() -> void:
	var bounds := get_viewport_rect().size
	_grow = scale_for(bounds)
	var ink := Color(Ink.INK, 0.92)
	var faint := Color(Ink.INK, 0.5)
	# 散在山水裡的人離得遠，先畫；腳下各有一小塊地或一葉舟。
	for i in WANDERERS.size():
		var index := CAST.size() + i
		if index >= _looks.size():
			break
		var who: Array = WANDERERS[i]
		var near := clampf(inverse_lerp(0.2, 1.0, who[1]), 0.0, 1.0)
		_footing(who[5], who[0], who[1], who[2], Color(Ink.INK, lerpf(0.45, 0.9, near)))
		CultivatorFigure.draw(self, place(who[0]), STATURE * who[1] * _grow, -who[2], _looks[index], who[3], _time + index * 1.9, Color(Ink.INK, lerpf(0.6, 0.95, near)), lerpf(0.8, 1.0, near))
	_cliff(ink, faint)
	_pine(ink, faint)
	# 崖腳沒入雲霧：往下漸漸蓋上紙色。
	var fog_top := place(Vector2(0.0, 930.0)).y
	draw_polygon(
		PackedVector2Array([Vector2(0.0, fog_top), Vector2(bounds.x, fog_top), Vector2(bounds.x, bounds.y), Vector2(0.0, bounds.y)]),
		PackedColorArray([Color(Ink.PAPER, 0.0), Color(Ink.PAPER, 0.0), Color(Ink.PAPER, 0.95), Color(Ink.PAPER, 0.95)])
	)
	for i in mini(CAST.size(), _looks.size()):
		if i == BOARD_BEFORE:
			_board(ink, faint)
		var who: Array = CAST[i]
		# 站得遠的人線淡一點。
		var tone := Color(Ink.INK, lerpf(0.62, 0.95, inverse_lerp(0.9, 1.0, who[1])))
		CultivatorFigure.draw(self, place(who[0]), STATURE * who[1] * _grow, -who[2], _looks[i], who[3], _time + i * 1.9, tone)
	_burner(ink, faint)


## 散在山水裡的人腳下的東西：at 是腳下的位置、size 是那個人的遠近、facing 是朝向。
func _footing(kind: StringName, at: Vector2, size: float, facing: float, ink: Color) -> void:
	if kind == &"boat":
		# 一葉扁舟：船身、幾道水紋、一根釣竿與垂下的線。
		var hull: Array[Vector2] = []
		for point: Vector2 in [Vector2(-150.0, -22.0), Vector2(-95.0, 16.0), Vector2(95.0, 16.0), Vector2(160.0, -26.0), Vector2(95.0, -4.0), Vector2(-95.0, -4.0)]:
			hull.append(at + point * size)
		_ware(CultivatorFigure.smooth_outline(hull), Color(WOOD, 0.9), ink, 1.6)
		for ripple in 3:
			var from := at + Vector2(-190.0 + 150.0 * ripple, 30.0 + 9.0 * ripple) * size
			draw_line(place(from), place(from + Vector2(90.0 + 20.0 * sin(_time * 0.7 + ripple), 0.0) * size), Color(ink, ink.a * 0.5), _stroke(1.1), true)
		var grip := at + Vector2(22.0 * facing, -150.0) * size
		var tip := at + Vector2(215.0 * facing, -310.0) * size
		draw_line(place(grip), place(tip), ink, _stroke(1.4), true)
		draw_line(place(tip), place(tip + Vector2(6.0 * sin(_time * 1.1), 335.0) * size), Color(ink, ink.a * 0.6), _stroke(1.0), true)
		return
	# 一小塊地：頂上一道墨線，底下的淡彩往下淡掉，沒入霧裡；山腰的石臺窄而陡，土坡與水岸寬而平。
	var half := 150.0 if kind == &"ledge" else 230.0 if kind == &"path" else 190.0
	var deep := 120.0 if kind == &"ledge" else 70.0
	var crown := PackedVector2Array()
	var shades := PackedColorArray()
	for step in 13:
		var t := step / 12.0
		crown.append(at + Vector2(lerpf(-half, half, t), 10.0 - 16.0 * sin(t * PI) + 5.0 * sin(t * 9.0 + at.x)) * size)
		shades.append(Color(ROCK_WASH, 0.95 * sin(t * PI)))
	var body := _placed(crown)
	var under := PackedVector2Array()
	for step in range(12, -1, -1):
		under.append(place(crown[step] + Vector2(0.0, deep * size)))
		shades.append(Color(ROCK_WASH, 0.0))
	var shape := body.duplicate()
	shape.append_array(under)
	draw_polygon(shape, shades)
	var tones := PackedColorArray()
	for step in 13:
		tones.append(Color(ink, ink.a * sin(step / 12.0 * PI)))
	draw_polyline_colors(body, tones, _stroke(1.8), true)
	for tuft: float in [-0.7, 0.55]:
		var root := at + Vector2(half * tuft, 2.0) * size
		for blade in 3:
			draw_line(place(root + Vector2(blade * 7.0, 0.0) * size), place(root + Vector2(blade * 7.0 + (blade - 1) * 9.0, -30.0) * size), Color(LEAF, 0.7), _stroke(1.1), true)


## 崖：臺面與崖壁填一層淡底，遠緣一道淡線，崖唇一道頓挫的墨線，崖壁幾道皴與苔點。
func _cliff(ink: Color, faint: Color) -> void:
	var far := CultivatorFigure.smooth_path(FAR_EDGE)
	var lip := CultivatorFigure.smooth_path(LIP)
	var top := far.duplicate()
	var near := lip.duplicate()
	near.reverse()
	top.append_array(near)
	draw_colored_polygon(_placed(cliff_wall()), ROCK_SHADE)
	draw_colored_polygon(_placed(top), ROCK_WASH)
	draw_polyline(_placed(far), faint, _stroke(1.6), true)
	draw_polyline(_placed(lip), ink, _stroke(3.2), true)
	draw_polyline(_placed(CultivatorFigure.smooth_path(TIP_EDGE)), ink, _stroke(2.6), true)
	var cracks := cliff_cracks()
	for i in cracks.size():
		if cracks[i].size() > 1:
			draw_polyline(_placed(cracks[i]), faint if i % 2 == 1 else ink, _stroke(1.8 if i % 2 == 0 else 1.2), true)
	# 苔點與臺面上的幾莖草。
	for i in 14:
		var spot := Vector2(-1060.0 + 78.0 * i + 22.0 * sin(i * 2.7), 916.0 + 12.0 * sin(i * 1.9))
		draw_circle(place(spot), (2.2 + 1.2 * sin(i * 3.1)) * _grow, Color(LEAF, 0.7))
	for tuft: Vector2 in [Vector2(-1085.0, 872.0), Vector2(-810.0, 886.0), Vector2(-640.0, 838.0), Vector2(-370.0, 836.0), Vector2(-240.0, 892.0), Vector2(-20.0, 880.0)]:
		for blade in 4:
			var bend := (blade - 1.5) * 5.0 + sin(_time * 1.3 + tuft.x) * 2.0
			draw_polyline(_placed(PackedVector2Array([tuft + Vector2(blade * 3.0, 0.0), tuft + Vector2(blade * 3.0 + bend * 0.5, -9.0), tuft + Vector2(blade * 3.0 + bend, -16.0 - 3.0 * (blade % 2))])), Color(LEAF, 0.85), _stroke(1.4), true)


## 松：照 PINE_ORDER 由後往前畫。枝是根粗梢細的一段、填色勾線，枝根被後畫的幹蓋住；幹勾輪廓、點幾處樹皮的鱗；
## 松針一簇簇放射的短線，遠處的淡。
func _pine(ink: Color, faint: Color) -> void:
	for layer in PINE_ORDER:
		match layer:
			&"far_needles", &"near_needles":
				var far := layer == &"far_needles"
				for i in NEEDLES.size():
					if NEEDLES[i][3] == far:
						_needles(NEEDLES[i][0], NEEDLES[i][1], NEEDLES[i][2], i, Color(LEAF, 0.6 if far else 0.95))
			&"branches":
				for branch: Array in BRANCHES:
					var limb: Array[Vector2] = []
					limb.assign(branch)
					_ware(_tapered(CultivatorFigure.smooth_path(limb), BRANCH_ROOT, BRANCH_TIP), BARK_WASH, ink, 2.0)
			&"trunk":
				_ware(CultivatorFigure.smooth_outline(TRUNK), BARK_WASH, ink, 2.6)
			&"bark":
				for scale: Vector2 in BARK:
					draw_polyline(_placed(PackedVector2Array([scale + Vector2(-9.0, -7.0), scale + Vector2(3.0, 0.0), scale + Vector2(-8.0, 9.0)])), faint, _stroke(1.4), true)


## 沿一條線做成一段根粗梢細的外形（版面座標）。
static func _tapered(path: PackedVector2Array, root: float, tip: float) -> PackedVector2Array:
	var near := PackedVector2Array()
	var far := PackedVector2Array()
	var last := path.size() - 1
	for i in path.size():
		var across := (path[mini(i + 1, last)] - path[maxi(i - 1, 0)]).normalized().orthogonal() * lerpf(root, tip, float(i) / last) * 0.5
		near.append(path[i] + across)
		far.append(path[i] - across)
	far.reverse()
	near.append_array(far)
	return near


## 一層松針：一排針簇，每簇由一點往上半圈放射出去的短線。
func _needles(centre: Vector2, half: float, tufts: int, index: int, ink: Color) -> void:
	var sway := sin(_time * 0.8 + index * 1.7) * 4.0
	for k in tufts:
		var t := (k + 0.5) / tufts
		var at := centre + Vector2(lerpf(-half, half, t) + sway, -18.0 * sin(t * PI) + 6.0 * sin(k * 2.3 + index))
		var reach := half / tufts * 1.7
		for n in 17:
			var angle := PI * (1.04 + 0.92 * n / 16.0)
			var tip := at + Vector2(cos(angle), sin(angle) * 0.85) * reach * (0.8 + 0.2 * sin(n * 1.9 + k))
			draw_line(place(at), place(tip), ink, _stroke(1.5), true)


## 棋枰：一張矮几（看得到几面），几面上畫格線、擺幾枚黑白子，兩旁各一隻棋罐。
func _board(ink: Color, faint: Color) -> void:
	var half := 58.0
	var deep := 15.0
	var top := BOARD.y - 46.0
	var corners: Array[Vector2] = [
		Vector2(BOARD.x - half + deep * 0.6, top - deep), Vector2(BOARD.x + half + deep * 0.6, top - deep),
		Vector2(BOARD.x + half - deep * 0.6, top + deep), Vector2(BOARD.x - half - deep * 0.6, top + deep),
	]
	# 几腳先畫，几面蓋在上面。
	for corner: Vector2 in [corners[0], corners[1], corners[3], corners[2]]:
		var foot := Vector2(corner.x + signf(corner.x - BOARD.x) * 3.0, BOARD.y + (corner.y - top) * 0.6)
		_ware(PackedVector2Array([corner + Vector2(-5.0, 4.0), corner + Vector2(5.0, 4.0), foot + Vector2(4.0, 0.0), foot + Vector2(-4.0, 0.0)]), WOOD, ink, 1.8)
	_ware(PackedVector2Array([corners[3], corners[2], corners[2] + Vector2(0.0, 9.0), corners[3] + Vector2(0.0, 9.0)]), WOOD, ink, 1.8)
	_ware(PackedVector2Array(corners), BOARD_FACE, ink, 2.0)
	for i in range(1, 6):
		var t := i / 6.0
		draw_line(place(corners[0].lerp(corners[1], t)), place(corners[3].lerp(corners[2], t)), faint, _stroke(1.0), true)
		draw_line(place(corners[0].lerp(corners[3], t)), place(corners[1].lerp(corners[2], t)), faint, _stroke(1.0), true)
	# 棋子：落在格線的交點上，黑子填墨、白子勾圈。
	var stones := [Vector2i(1, 2), Vector2i(2, 4), Vector2i(3, 1), Vector2i(3, 3), Vector2i(4, 2), Vector2i(5, 4), Vector2i(2, 2), Vector2i(4, 5)]
	for i in stones.size():
		var u: float = stones[i].x / 6.0
		var v: float = stones[i].y / 6.0
		var at := place(corners[0].lerp(corners[1], u).lerp(corners[3].lerp(corners[2], u), v))
		draw_circle(at, 3.6 * _grow, ink if i % 2 == 0 else Color(0.985, 0.97, 0.93))
		if i % 2 == 1:
			draw_arc(at, 3.6 * _grow, 0.0, TAU, 12, ink, _stroke(1.0), true)
	for side: float in [-1.0, 1.0]:
		var jar := Vector2(BOARD.x + side * 92.0, BOARD.y + 14.0)
		_ware(CultivatorFigure.smooth_outline([
			jar + Vector2(-13.0, 0.0), jar + Vector2(-19.0, -11.0), jar + Vector2(-12.0, -22.0), jar + Vector2(12.0, -22.0), jar + Vector2(19.0, -11.0), jar + Vector2(13.0, 0.0),
		]), WARE, ink, 1.8)
		draw_line(place(jar + Vector2(-11.0, -19.0)), place(jar + Vector2(11.0, -19.0)), faint, _stroke(1.0), true)


## 香爐：三足小爐，一縷煙往上飄散。
func _burner(ink: Color, faint: Color) -> void:
	for foot: float in [-11.0, 0.0, 11.0]:
		draw_line(place(BURNER + Vector2(foot, -8.0)), place(BURNER + Vector2(foot * 1.25, 0.0)), ink, _stroke(2.0), true)
	_ware(CultivatorFigure.smooth_outline([
		BURNER + Vector2(-14.0, -7.0), BURNER + Vector2(-21.0, -20.0), BURNER + Vector2(-15.0, -32.0), BURNER + Vector2(15.0, -32.0),
		BURNER + Vector2(21.0, -20.0), BURNER + Vector2(14.0, -7.0),
	]), BRONZE, ink, 2.0)
	draw_line(place(BURNER + Vector2(-17.0, -27.0)), place(BURNER + Vector2(17.0, -27.0)), faint, _stroke(1.0), true)
	var lid := _placed(CultivatorFigure.smooth_path([BURNER + Vector2(-13.0, -33.0), BURNER + Vector2(-6.0, -43.0), BURNER + Vector2(0.0, -48.0), BURNER + Vector2(6.0, -43.0), BURNER + Vector2(13.0, -33.0)]))
	draw_polyline(lid, ink, _stroke(1.8), true)
	# 煙：兩道並行的細線，越往上越淡、越散。
	for strand: float in [-1.0, 1.0]:
		var smoke: Array[Vector2] = []
		for i in 9:
			var t := i / 8.0
			smoke.append(BURNER + Vector2(sin(_time * 0.9 + t * 5.0) * 24.0 * t + 36.0 * t * t + strand * (2.0 + 7.0 * t), -50.0 - 230.0 * t))
		var curl := _placed(CultivatorFigure.smooth_path(smoke))
		for i in curl.size() - 1:
			draw_line(curl[i], curl[i + 1], Color(Ink.INK, 0.5 * (1.0 - float(i) / curl.size())), _stroke(1.2), true)


## 一件器物：填底色、勾墨線（版面座標）。
func _ware(outline: PackedVector2Array, wash: Color, ink: Color, width: float) -> void:
	var shape := _placed(outline)
	draw_colored_polygon(shape, wash)
	shape.append(shape[0])
	draw_polyline(shape, ink, _stroke(width), true)


## 線寬：版面上的像素換成畫面上的，至少 1.2 像素。
func _stroke(width: float) -> float:
	return maxf(width * _grow, 1.2)


func _placed(outline: PackedVector2Array) -> PackedVector2Array:
	var points := PackedVector2Array()
	for point in outline:
		points.append(place(point))
	return points
