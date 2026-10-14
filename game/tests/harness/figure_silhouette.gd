extends RefCounted
## 棋子的剪影（8.11 驗收）：把一個動作的代表格算成 64 像素高的黑白遮罩，兩兩比較不同像素的比例。
## 腳底原點相同、不做對位（地圖上同一格站的就是這個位置）；flip 乘在 x 上（轉身的翻面）。

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")
## 身高 1 對應 64 像素；取景：x −0.65 到 0.75、y −1.15 到 0.05。
const PX := 64.0
const X0 := -0.65
const Y0 := -1.15
const COLS := 90
const ROWS := 77
## 每個動作最有代表性的一格。
const REP := {
	"stand": 0, "brace": 12, "struck": 14, "walk": 5, "fall": 22, "rise": 27, "strike": 32, "seal": 24, "bow": 24,
	"kneel": 28, "bless": 22, "sit": 24, "slump": 60, "turn": 10, "away": 30, "clasp": 24, "offer": 30, "receive": 24,
}
const SEED := 11


## 動作 action 第 frame 格的遮罩：COLS × ROWS 個 0 或 1。
static func mask(look: Dictionary, action: String, frame: int) -> PackedByteArray:
	var pose := FigureMotion.frame_pose(action, frame)
	var flip := float(pose.get("flip", 1.0))
	var out := PackedByteArray()
	out.resize(COLS * ROWS)
	for polygon in F.shapes(look, pose, 0.4):
		var points := PackedVector2Array()
		var box := Rect2(polygon[0] * Vector2(flip, 1.0), Vector2.ZERO)
		for p in polygon:
			var q := Vector2(p.x * flip, p.y)
			points.append(q)
			box = box.expand(q)
		var c0 := maxi(int(floor((box.position.x - X0) * PX)), 0)
		var c1 := mini(int(ceil((box.end.x - X0) * PX)), COLS - 1)
		var r0 := maxi(int(floor((box.position.y - Y0) * PX)), 0)
		var r1 := mini(int(ceil((box.end.y - Y0) * PX)), ROWS - 1)
		for r in range(r0, r1 + 1):
			for c in range(c0, c1 + 1):
				if out[r * COLS + c] == 0 and Geometry2D.is_point_in_polygon(Vector2(X0 + (c + 0.5) / PX, Y0 + (r + 0.5) / PX), points):
					out[r * COLS + c] = 1
	return out


## 兩個遮罩不同的像素數占聯集的比例（0 完全一樣、1 完全不重疊）。
static func difference(a: PackedByteArray, b: PackedByteArray) -> float:
	var union := 0
	var diff := 0
	for i in a.size():
		if a[i] == 1 or b[i] == 1:
			union += 1
			if a[i] != b[i]:
				diff += 1
	return float(diff) / float(maxi(union, 1))


static func masks_of(names: Array) -> Dictionary:
	var look := F.traits(SEED)
	var out := {}
	for name: String in names:
		out[name] = mask(look, name, int(REP[name]))
	return out
