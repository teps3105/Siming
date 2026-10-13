class_name SpectacleBrush
extends RefCounted
## 奇觀的筆法（GDD 5.1 水墨設色）：淡墨層層暈染的雲、帶粗細的飛白筆觸、朱紅與金的點睛，不用粒子。
## 全是靜態函式：給畫布、奇觀資料、螢幕位置與時間，畫出該時刻的樣子；不留狀態，同一個種子畫出同一個樣子。

const INK := Color("2B2620")
const PALE := Color("F3EBD8")
const RED := Color("B23A2E")
const GOLD := Color("C9A246")
const GOLD_LIGHT := Color("E8CF86")


## 0..1 的偽隨機：同一個種子與序號永遠同一個值。
static func rand(seed_value: int, index: int) -> float:
	return fposmod(sin(float(seed_value) * 12.9898 + float(index) * 78.233) * 43758.5453, 1.0)


static func ease_out(t: float) -> float:
	var u := 1.0 - clampf(t, 0.0, 1.0)
	return 1.0 - u * u * u


static func with_alpha(color: Color, alpha: float) -> Color:
	return Color(color.r, color.g, color.b, color.a * clampf(alpha, 0.0, 1.0))


## 一道兩端粗細不同的筆觸（沿折線，逐段畫成四邊形）。
static func stroke(c: CanvasItem, points: PackedVector2Array, w0: float, w1: float, color: Color) -> void:
	var n := points.size()
	if n < 2 or color.a <= 0.003:
		return
	var left := PackedVector2Array()
	var right := PackedVector2Array()
	for i in n:
		var dir := points[mini(i + 1, n - 1)] - points[maxi(i - 1, 0)]
		if dir.length_squared() < 1e-6:
			dir = Vector2.RIGHT
		var normal := dir.normalized().orthogonal()
		var half := maxf(lerpf(w0, w1, float(i) / (n - 1)), 0.6) * 0.5
		left.append(points[i] + normal * half)
		right.append(points[i] - normal * half)
	var colors := PackedColorArray([color, color, color, color])
	for i in n - 1:
		c.draw_primitive(PackedVector2Array([left[i], left[i + 1], right[i + 1], right[i]]), colors, PackedVector2Array())


## 淡墨暈染的圓：由外向內疊幾層，邊緣柔、中心濃。
static func wash(c: CanvasItem, center: Vector2, radius: float, color: Color, alpha: float) -> void:
	var layers := 9
	for i in layers:
		var f := 1.0 - 0.9 * float(i) / layers
		c.draw_circle(center, radius * f, with_alpha(color, alpha * 0.11))


# ── 天劫 ──────────────────────────────────────────────────────

## 4 秒：0–1.4 劫雲聚（暗色渦旋），1.4–3.0 落雷（新境界 + 1 道），3.0–4.0 依結果收（金光散開／兵解白光／身死黑煙）。
static func tribulation(c: CanvasItem, item: Dictionary, ground: Vector2, radius: float, u: float, age: float, t: float) -> void:
	var seed_value: int = item["seed"]
	var bolts: int = item["bolts"]
	var outcome: String = item["outcome"]
	var gather := ease_out(t / 0.35)
	var after := clampf((t - 0.75) / 0.25, 0.0, 1.0)
	var cloud_alpha := (1.0 - after) * minf(1.0, t / 0.12)
	var heart := ground + Vector2(0.0, -radius * 0.95)
	# 劫雲：一圈圈淡墨團繞著中心旋轉，越靠中心越濃。
	for ring in 3:
		var ring_r := radius * (0.95 - 0.28 * ring) * gather
		var count := 7 - ring
		for i in count:
			var a := TAU * (float(i) / count + 0.17 * ring) + age * (0.7 + 0.4 * ring) * (1.0 if ring % 2 == 0 else -1.0)
			var jitter := 0.75 + 0.5 * rand(seed_value, ring * 10 + i)
			var at := heart + Vector2(cos(a) * ring_r * 1.15, sin(a) * ring_r * 0.5)
			wash(c, at, radius * (0.42 - 0.07 * ring) * jitter * gather, INK, cloud_alpha * (0.5 + 0.2 * ring))
	# 渦旋：兩道逆時針的飛白筆觸，粗尾細，繞著雲心收攏。
	for arm in 2:
		var pts := PackedVector2Array()
		for k in 18:
			var f := float(k) / 17.0
			var a := -age * 1.4 + arm * PI + f * 3.4
			var r := radius * (1.0 - f * 0.78) * gather
			pts.append(heart + Vector2(cos(a) * r * 1.15, sin(a) * r * 0.5))
		stroke(c, pts, 6.0 * u, 1.0 * u, with_alpha(INK, cloud_alpha * 0.55))
	# 落雷：朱紅暈邊、淡紙色芯，鋸齒的一道，各閃約 0.2 秒。
	var span_start := 0.35 * 4.0
	var span := 0.4 * 4.0
	var gap := span / bolts
	var flash := minf(0.22, gap * 0.9)
	for k in bolts:
		var local := age - (span_start + gap * k)
		var hit_x := (rand(seed_value, 100 + k) - 0.5) * radius * 0.7
		var hit := ground + Vector2(hit_x, (rand(seed_value, 200 + k) - 0.5) * radius * 0.2)
		if local >= 0.0:
			# 劈過之處在地上留一小團焦墨。
			c.draw_circle(hit, radius * 0.07, with_alpha(INK, 0.28 * (1.0 - after)))
		if local < 0.0 or local > flash:
			continue
		var fade := 1.0 - local / flash
		var top := heart + Vector2((rand(seed_value, 300 + k) - 0.5) * radius * 0.9, 0.0)
		var pts := PackedVector2Array([top])
		var steps := 8
		for s in range(1, steps):
			var f := float(s) / steps
			var side := (rand(seed_value, 400 + k * 10 + s) - 0.5) * radius * 0.28
			pts.append(top.lerp(hit, f) + Vector2(side, 0.0))
		pts.append(hit)
		stroke(c, pts, 17.0 * u, 17.0 * u, with_alpha(RED, 0.35 * fade))
		stroke(c, pts, 8.0 * u, 5.0 * u, with_alpha(PALE, 0.9 * fade))
		stroke(c, pts, 3.0 * u, 2.0 * u, with_alpha(Color.WHITE, fade))
	# 收尾。
	if after > 0.0:
		match outcome:
			"Passed":
				_gold_scatter(c, ground, radius, u, seed_value, after)
			"ShedBody":
				_white_flash(c, ground, radius, u, after)
			_:
				_black_smoke(c, ground, radius, seed_value, after)


## 渡過：金光自落點向四方散開的飛白筆觸，與一圈漸淡的金暈。
static func _gold_scatter(c: CanvasItem, at: Vector2, radius: float, u: float, seed_value: int, f: float) -> void:
	var fade := 1.0 - f * f
	wash(c, at, radius * (0.25 + 0.55 * ease_out(f)), GOLD, fade * 0.75)
	for i in 14:
		var a := TAU * (float(i) + rand(seed_value, 500 + i)) / 14.0
		var near := radius * (0.1 + 0.25 * ease_out(f))
		var far := radius * (0.3 + 0.75 * ease_out(f)) * (0.7 + 0.5 * rand(seed_value, 520 + i))
		var dir := Vector2(cos(a), sin(a) * 0.6)
		stroke(c, PackedVector2Array([at + dir * near, at + dir * (near + far) * 0.5, at + dir * far]), 4.0 * u, 0.8 * u, with_alpha(GOLD_LIGHT, fade))
	c.draw_circle(at, radius * 0.06 * (1.0 - f), with_alpha(PALE, fade))


## 兵解：一道白光自身處升起，外擴兩圈細環，其餘留白。
static func _white_flash(c: CanvasItem, at: Vector2, radius: float, u: float, f: float) -> void:
	var fade := 1.0 - f * f
	wash(c, at, radius * (0.2 + 0.35 * ease_out(f)), PALE, fade * 0.95)
	for i in 2:
		var r := radius * (0.15 + (0.6 + 0.25 * i) * ease_out(f))
		c.draw_arc(at, r, 0.0, TAU, 40, with_alpha(PALE, fade * (0.9 - 0.3 * i)), (3.0 - i) * u)
	stroke(c, PackedVector2Array([at, at + Vector2(0.0, -radius * 0.45 * ease_out(f)), at + Vector2(0.0, -radius * 0.9 * ease_out(f))]), 5.0 * u, 1.0 * u, with_alpha(Color.WHITE, fade))


## 身死：黑煙團團上騰，愈升愈大愈淡。
static func _black_smoke(c: CanvasItem, at: Vector2, radius: float, seed_value: int, f: float) -> void:
	for i in 9:
		var delay := rand(seed_value, 600 + i) * 0.3
		var g := clampf((f - delay) / 0.7, 0.0, 1.0)
		if g <= 0.0:
			continue
		var drift := (rand(seed_value, 620 + i) - 0.5) * radius * 0.7
		var puff := at + Vector2(drift * g, -radius * (0.15 + 0.9 * g) * (0.6 + 0.6 * rand(seed_value, 640 + i)))
		wash(c, puff, radius * (0.12 + 0.28 * g), INK, (1.0 - g) * 0.85)


# ── 飛升 ──────────────────────────────────────────────────────

## 6 秒：金色光柱自所在貫天（0–0.8 秒升起），雲層被衝開一個洞，持續到 4.5 秒後淡出。
static func ascended(c: CanvasItem, item: Dictionary, ground: Vector2, radius: float, u: float, age: float, _t: float) -> void:
	var seed_value: int = item["seed"]
	var rise := ease_out(age / 0.8)
	var fade := 1.0 - clampf((age - 4.5) / 1.5, 0.0, 1.0)
	var top_y := lerpf(ground.y, -40.0, rise)
	var half := radius * 0.2
	# 雲洞：光柱頂端那一帶的淡墨雲環向外推開，中間露出金光。
	var hole_c := Vector2(ground.x, minf(ground.y - radius * 1.5, ground.y))
	var open := ease_out((age - 0.3) / 1.6)
	for i in 12:
		var a := TAU * float(i) / 12.0 + 0.3 * rand(seed_value, i)
		var r := radius * (0.25 + 0.95 * open) * (0.9 + 0.2 * rand(seed_value, 20 + i))
		var at := hole_c + Vector2(cos(a) * r * 1.3, sin(a) * r * 0.55)
		wash(c, at, radius * (0.5 + 0.2 * rand(seed_value, 40 + i)), INK, fade * minf(1.0, age / 0.5) * 0.4)
	wash(c, hole_c, radius * (0.2 + 0.5 * open), GOLD_LIGHT, fade * 0.5)
	# 光柱：由外而內三層，金→淡金→近白；下濃上淡。
	var layers := [[1.0, GOLD, 0.30], [0.62, GOLD_LIGHT, 0.45], [0.26, PALE, 0.85]]
	for layer: Array in layers:
		var w: float = half * float(layer[0]) * (1.0 + 0.04 * sin(age * 6.0))
		var col: Color = layer[1]
		var a0: float = float(layer[2]) * fade
		var strips := 10
		for s in strips:
			var y0 := lerpf(ground.y, top_y, float(s) / strips)
			var y1 := lerpf(ground.y, top_y, float(s + 1) / strips)
			var c0 := with_alpha(col, a0 * (1.0 - 0.7 * float(s) / strips))
			var c1 := with_alpha(col, a0 * (1.0 - 0.7 * float(s + 1) / strips))
			c.draw_primitive(PackedVector2Array([Vector2(ground.x - w, y0), Vector2(ground.x + w, y0), Vector2(ground.x + w, y1), Vector2(ground.x - w, y1)]),
				PackedColorArray([c0, c0, c1, c1]), PackedVector2Array())
	# 柱邊的飛白：兩道細長的金筆，一長一短。
	for side in 2:
		var x := ground.x + (half * 1.05) * (1.0 if side == 0 else -1.0) + (rand(seed_value, 70 + side) - 0.5) * 3.0 * u
		stroke(c, PackedVector2Array([Vector2(x, ground.y), Vector2(x + 2.0 * u, lerpf(ground.y, top_y, 0.5)), Vector2(x, lerpf(ground.y, top_y, 0.9 - 0.25 * side))]), 2.2 * u, 0.8 * u, with_alpha(GOLD, 0.75 * fade))
	# 升騰的金屑：短短一筆，沿柱上浮。
	for i in 9:
		var phase := fposmod(age * 0.35 + rand(seed_value, 90 + i), 1.0)
		var x := ground.x + (rand(seed_value, 110 + i) - 0.5) * half * 1.6
		var y := lerpf(ground.y, top_y, phase)
		stroke(c, PackedVector2Array([Vector2(x, y), Vector2(x, y - 9.0 * u)]), 2.0 * u, 0.8 * u, with_alpha(GOLD_LIGHT, fade * (1.0 - phase) * rise))
	# 落腳處一圈金暈。
	wash(c, ground, radius * 0.45, GOLD, fade * 0.7 * rise)


# ── 宿緣了結 ──────────────────────────────────────────────────

## 3 秒：宿緣線自兩端向中點燃盡（前 2.25 秒），中點炸開光點（後 0.75 秒）。顏色依宿緣種類。
static func resolved(c: CanvasItem, item: Dictionary, a: Vector2, b: Vector2, u: float, age: float, _t: float) -> void:
	var seed_value: int = item["seed"]
	var kind_color: Color = KarmaLayer.KIND_COLORS[clampi(int(item["thread_kind"]), 0, KarmaLayer.KIND_COLORS.size() - 1)]
	var length := a.distance_to(b)
	var mid := (a + b) * 0.5
	var control := mid + Vector2(0.0, -maxf(length * 0.3, 24.0 * u)) * 2.0
	var burn := clampf(age / 2.25, 0.0, 1.0)
	var reach := burn * 0.5
	# 全線的取點：二次貝茲弧。
	var samples := 28
	var arc := PackedVector2Array()
	for i in samples + 1:
		var f := float(i) / samples
		arc.append((1.0 - f) * (1.0 - f) * a + 2.0 * (1.0 - f) * f * control + f * f * b)
	var left := PackedVector2Array()
	var middle := PackedVector2Array()
	var right := PackedVector2Array()
	for i in samples + 1:
		var f := float(i) / samples
		if f <= reach:
			left.append(arc[i])
		elif f >= 1.0 - reach:
			right.append(arc[i])
		else:
			middle.append(arc[i])
	var live := 1.0 - clampf((age - 2.25) / 0.2, 0.0, 1.0)
	# 兩端的焦墨殘痕，淡得很快。
	var char_alpha := 0.5 * (1.0 - clampf((age - 1.0) / 2.0, 0.0, 1.0))
	stroke(c, left, 2.4 * u, 2.4 * u, with_alpha(INK, char_alpha))
	stroke(c, right, 2.4 * u, 2.4 * u, with_alpha(INK, char_alpha))
	# 尚未燒到的線：淡紙色描邊襯底，種類色壓上，越近燒處越細。
	if middle.size() >= 2:
		stroke(c, middle, 7.0 * u, 7.0 * u, with_alpha(PALE, 0.6 * live))
		stroke(c, middle, 4.5 * u, 4.5 * u, with_alpha(kind_color, 0.95 * live))
	# 燒處：兩顆火星，朱紅與金交替閃。
	if live > 0.0:
		for end in 2:
			var f := reach if end == 0 else 1.0 - reach
			var at: Vector2 = (1.0 - f) * (1.0 - f) * a + 2.0 * (1.0 - f) * f * control + f * f * b
			var flick := 0.8 + 0.2 * sin(age * 40.0 + end * 2.0)
			c.draw_circle(at, 14.0 * u * flick, with_alpha(RED, 0.35 * live))
			c.draw_circle(at, 7.5 * u * flick, with_alpha(GOLD_LIGHT, 0.95 * live))
	# 兩端的端點：小小一圈墨環。
	for end: Vector2 in [a, b]:
		c.draw_arc(end, 5.0 * u, 0.0, TAU, 16, with_alpha(INK, 0.6 * (1.0 - burn * 0.8)), 1.5 * u)
	# 中點炸開：光點向外迸，各拖一小筆，一圈淡環。
	var burst := clampf((age - 2.25) / 0.75, 0.0, 1.0)
	if burst > 0.0 or age > 2.15:
		var f := clampf((age - 2.15) / 0.85, 0.0, 1.0)
		var at := arc[samples / 2]
		var fade := 1.0 - f
		for i in 12:
			var ang := TAU * (float(i) + rand(seed_value, i)) / 12.0
			var dir := Vector2(cos(ang), sin(ang))
			var dist := (40.0 + 90.0 * rand(seed_value, 30 + i)) * u * ease_out(f)
			stroke(c, PackedVector2Array([at + dir * dist * 0.5, at + dir * dist]), 4.0 * u, 1.0 * u, with_alpha(kind_color if i % 2 == 0 else GOLD_LIGHT, fade))
			c.draw_circle(at + dir * dist, 4.0 * u * fade + 0.5, with_alpha(PALE, fade))
		c.draw_arc(at, 80.0 * u * ease_out(f) + 2.0, 0.0, TAU, 40, with_alpha(kind_color, fade * 0.85), 2.6 * u)
		wash(c, at, 46.0 * u * (0.4 + 0.6 * f), GOLD_LIGHT, fade)


# ── 光點（速度 >= 20× 的簡化版）────────────────────────────────

static func spot(c: CanvasItem, item: Dictionary, at: Vector2, u: float, t: float) -> void:
	var color := PALE
	match str(item.get("of", "")):
		"Ascended":
			color = GOLD_LIGHT
		"ThreadResolved":
			color = KarmaLayer.KIND_COLORS[clampi(int(item.get("thread_kind", 0)), 0, KarmaLayer.KIND_COLORS.size() - 1)].lerp(PALE, 0.4)
	var fade := 1.0 - t
	var lifted := at + Vector2(0.0, -10.0 * u * t)
	c.draw_circle(lifted, 7.0 * u * (0.6 + 0.4 * fade), with_alpha(color, 0.25 * fade))
	c.draw_circle(lifted, 3.2 * u, with_alpha(color, 0.9 * fade))
