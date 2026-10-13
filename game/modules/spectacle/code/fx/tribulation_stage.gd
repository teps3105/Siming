class_name TribulationStage
extends SpectacleStage
## 天劫（4 秒）：0–1.4 劫雲聚攏翻湧，1.4–3.0 落雷（新境界 + 1 道），3.0–4.0 依結果收（金光散開／兵解白光／身死黑煙）。
## 劫雲是三層煙雲貼圖粒子（Kenney Particle Pack）；雷是 Storm & Wind 3D Free 的 strike（CC BY 4.0，經修改，
## 見 assets/storm_wind），一道雷一個節點，到時間才生出，由這裡以 age 推進；收尾是粒子的散開與消散。

const CLOUD_START := 0.0
const BOLT_START := 1.4
const BOLT_SPAN := 1.6
const OUTCOME_START := 3.0
const END := 4.0
## 劫雲中心離地多高（半徑為 1 的單位）。
const CLOUD_HEIGHT := 1.5
## 雷的單位高度：StormFX 以「高 4」為本來的比例。
const BOLT_NATIVE_HEIGHT := 4.0
const STEP := 0.03

var _clouds: Array[MeshInstance3D] = []
var _underglow: MeshInstance3D
var _bolts: Array[Dictionary] = []
var _outcome: Array[MeshInstance3D] = []
var _seed := 0
var _flash_peak := 0.0
var _figure: FigureActor


func setup(spec: Dictionary) -> void:
	super.setup(spec)
	_seed = int(spec["seed"])
	var s := float(_seed % 97)
	var heart := Vector3(0.0, CLOUD_HEIGHT, 0.0)
	var common := {"seed": s, "life": 30.0, "fade_out_pow": 0.0, "fade_in": 0.5, "shell": 0.7,
		"from_scale": 1.7, "settle_time": 1.4, "emit_span": 1.0, "size_rand": 0.3, "tint_mix": 0.8}
	# 底層：最暗最寬的雲，壓在下面。
	_clouds.append(_add(SpectacleKit.sprites(self, "smoke_04", 14, common.merged({
		"origin_off": heart + Vector3(0.0, -0.12, 0.0), "extent": Vector3(0.8, 0.14, 0.6), "swirl": 0.45,
		"size0": 0.4, "size1": 0.46, "spin": 0.35, "color0": SpectacleKit.with_alpha(SpectacleKit.INK_LIGHT, 0.85),
		"color1": SpectacleKit.with_alpha(SpectacleKit.INK_LIGHT, 0.85), "tint_b": SpectacleKit.INK_LIGHT}))))
	# 中層：翻湧的主體，逆向旋轉。
	_clouds.append(_add(SpectacleKit.sprites(self, "smoke_07", 16, common.merged({
		"origin_off": heart, "extent": Vector3(0.68, 0.18, 0.5), "swirl": -0.7, "seed": s + 3.0,
		"size0": 0.34, "size1": 0.42, "spin": -0.6, "color0": SpectacleKit.with_alpha(Color("857A66"), 0.8),
		"color1": SpectacleKit.with_alpha(Color("857A66"), 0.8), "tint_b": SpectacleKit.INK}))))
	# 頂層：被閃電照亮的雲頂，色較淺。
	_clouds.append(_add(SpectacleKit.sprites(self, "smoke_01", 10, common.merged({
		"origin_off": heart + Vector3(0.0, 0.24, 0.0), "extent": Vector3(0.5, 0.1, 0.38), "swirl": 0.9, "seed": s + 7.0,
		"size0": 0.28, "size1": 0.36, "spin": 0.8, "color0": SpectacleKit.with_alpha(Color("857A66"), 0.75),
		"color1": SpectacleKit.with_alpha(Color("857A66"), 0.75), "tint_b": SpectacleKit.INK_LIGHT}))))
	_underglow = _add(SpectacleKit.sprites(self, "circle_05", 1, {"emit_span": 0.001, "life": 30.0, "fade_out_pow": 0.0,
		"fade_in": 0.01, "origin_off": heart + Vector3(0.0, -0.3, 0.0), "size0": 1.3, "size1": 1.3, "aspect": 0.7,
		"color0": Color(1.0, 0.86, 0.55, 0.9), "color1": Color(1.0, 0.86, 0.55, 0.9), "master": 0.0}, true))
	_build_bolts()
	_build_outcome()
	_build_figure()


## 當事人立在事件所在格：樣子由人物編號（沒有就由奇觀種子）決定，橋接給得出的欄位（性別、年紀）照用。
func _build_figure() -> void:
	var person := int(item.get("person", -1))
	_figure = FigureActor.new()
	add_child(_figure)
	_figure.setup(FigureActor.look_of(person if person >= 0 else _seed, item.get("given", {})))


## 此刻小人該做的動作與動作內的時間：劫雲聚攏時迎雷，每道落雷各受雷一次，其餘站著。
func figure_action(age: float) -> Dictionary:
	if age < BOLT_START:
		return {"action": "brace", "time": age}
	var struck := _figure.action_length("struck") if _figure != null else 0.75
	var latest := -1.0
	for bolt: Dictionary in _bolts:
		if float(bolt["time"]) <= age:
			latest = float(bolt["time"])
	if latest >= 0.0 and age - latest < struck:
		return {"action": "struck", "time": age - latest}
	return {"action": "stand", "time": age}


func _build_bolts() -> void:
	var count := int(item["bolts"])
	var gap := BOLT_SPAN / count
	for k in count:
		_bolts.append({
			"time": BOLT_START + gap * k,
			"offset": Vector3((SpectacleKit.rand(_seed, 100 + k) - 0.5) * 0.8, 0.02, (SpectacleKit.rand(_seed, 200 + k) - 0.5) * 0.45),
			"fx": null, "t": 0.0, "done": false, "seed": float(_seed % 89) + k * 1.7,
		})


## 三種收尾的粒子都先建好，age 小於出生時間時不顯示。
func _build_outcome() -> void:
	var s := float(_seed % 83)
	var gold := SpectacleKit.GOLD
	var gold_light := SpectacleKit.GOLD_LIGHT
	var origin := Vector3(0.0, 0.12, 0.0)
	match str(item["outcome"]):
		"Passed":
			_outcome.append(_add(SpectacleKit.sprites(self, "circle_02", 1, {"seed": s, "emit_span": 0.001, "life": 1.0,
				"fade_in": 0.03, "fade_out_pow": 1.3, "origin_off": origin, "size0": 0.12, "size1": 1.25, "flat_plane": true,
				"color0": SpectacleKit.with_alpha(gold, 1.0), "color1": SpectacleKit.with_alpha(gold_light, 1.0)})))
			_outcome.append(_add(SpectacleKit.sprites(self, "circle_05", 1, {"seed": s, "emit_span": 0.001, "life": 1.0,
				"fade_in": 0.05, "fade_out_pow": 1.5, "origin_off": origin, "size0": 0.45, "size1": 0.95, "flat_plane": true,
				"color0": Color(1.0, 0.82, 0.4, 0.9), "color1": Color(1.0, 0.82, 0.4, 0.9)}, true)))
			_outcome.append(_add(SpectacleKit.sprites(self, "star_08", 16, {"seed": s + 1.0, "emit_span": 0.12, "life": 0.95,
				"fade_in": 0.04, "fade_out_pow": 1.0, "origin_off": origin, "extent": Vector3(0.05, 0.03, 0.05),
				"radial": 1.15, "vel_spread": Vector3(0.5, 0.55, 0.5), "accel": Vector3(0.0, -0.25, 0.0), "drag": 2.0,
				"size0": 0.34, "size1": 0.08, "size_rand": 0.4, "spin": 4.0, "color0": gold_light, "color1": gold})))
			_outcome.append(_add(SpectacleKit.sprites(self, "trace_02", 18, {"seed": s + 2.0, "emit_span": 0.12, "life": 0.85,
				"fade_in": 0.03, "fade_out_pow": 1.2, "origin_off": origin, "extent": Vector3(0.05, 0.03, 0.05),
				"radial": 1.5, "vel_spread": Vector3(0.7, 0.8, 0.7), "drag": 1.9, "stretch": 0.55, "aspect": 1.6,
				"size0": 0.09, "size1": 0.04, "color0": Color("F7E7A8"), "color1": gold})))
			_outcome.append(_add(SpectacleKit.sprites(self, "spark_01", 14, {"seed": s + 3.0, "emit_span": 0.5, "life": 1.0,
				"fade_in": 0.1, "fade_out_pow": 1.0, "origin_off": origin, "extent": Vector3(0.5, 0.03, 0.4),
				"vel": Vector3(0.0, 0.65, 0.0), "vel_spread": Vector3(0.2, 0.25, 0.2), "size0": 0.13, "size1": 0.02,
				"size_rand": 0.4, "color0": gold_light, "color1": gold})))
		"ShedBody":
			var pale := SpectacleKit.PALE
			var shade := Color("7E8C94")
			_outcome.append(_add(SpectacleKit.sprites(self, "circle_02", 1, {"seed": s, "emit_span": 0.001, "life": 1.0,
				"fade_in": 0.03, "fade_out_pow": 1.2, "origin_off": origin, "size0": 0.14, "size1": 1.32, "flat_plane": true,
				"color0": SpectacleKit.with_alpha(shade, 0.4), "color1": SpectacleKit.with_alpha(shade, 0.4)})))
			_outcome.append(_add(SpectacleKit.sprites(self, "circle_02", 1, {"seed": s, "emit_span": 0.001, "life": 1.0,
				"fade_in": 0.03, "fade_out_pow": 1.2, "origin_off": origin, "size0": 0.12, "size1": 1.18, "flat_plane": true,
				"color0": pale, "color1": pale}, true)))
			_outcome.append(_add(SpectacleKit.sprites(self, "flare_01", 1, {"seed": s, "emit_span": 0.001, "life": 0.9,
				"fade_in": 0.04, "fade_out_pow": 1.4, "origin_off": origin + Vector3(0.0, 0.25, 0.0), "size0": 0.9, "size1": 1.5,
				"color0": Color(1.0, 1.0, 1.0, 1.0), "color1": Color(1.0, 1.0, 1.0, 1.0)}, true)))
			_outcome.append(_add(SpectacleKit.sprites(self, "trace_02", 3, {"seed": s + 1.0, "emit_span": 0.1, "life": 0.9,
				"fade_in": 0.04, "fade_out_pow": 1.0, "origin_off": origin, "vel": Vector3(0.0, 1.6, 0.0),
				"extent": Vector3(0.06, 0.0, 0.06), "stretch": 0.9, "aspect": 2.4, "size0": 0.07, "size1": 0.05,
				"color0": Color(1.0, 1.0, 1.0, 1.0), "color1": pale}, true)))
			_outcome.append(_add(SpectacleKit.sprites(self, "trace_02", 3, {"seed": s + 1.0, "emit_span": 0.1, "life": 0.9,
				"fade_in": 0.04, "fade_out_pow": 1.0, "origin_off": origin, "vel": Vector3(0.0, 1.6, 0.0),
				"extent": Vector3(0.06, 0.0, 0.06), "stretch": 0.9, "aspect": 2.4, "size0": 0.1, "size1": 0.07,
				"color0": SpectacleKit.with_alpha(shade, 0.7), "color1": SpectacleKit.with_alpha(shade, 0.7)})))
			_outcome.append(_add(SpectacleKit.sprites(self, "star_06", 24, {"seed": s + 2.0, "emit_span": 0.55, "life": 1.0,
				"fade_in": 0.1, "fade_out_pow": 1.0, "origin_off": origin, "extent": Vector3(0.45, 0.03, 0.35),
				"vel": Vector3(0.0, 0.75, 0.0), "vel_spread": Vector3(0.15, 0.3, 0.15), "size0": 0.13, "size1": 0.03,
				"size_rand": 0.4, "color0": pale, "color1": Color("B7C4CC")})))
		_:
			var ink := SpectacleKit.INK
			_outcome.append(_add(SpectacleKit.sprites(self, "smoke_04", 12, {"seed": s, "emit_span": 0.5, "life": 1.0,
				"fade_in": 0.12, "fade_out_pow": 0.9, "origin_off": origin, "extent": Vector3(0.28, 0.04, 0.22),
				"vel": Vector3(0.0, 0.62, 0.0), "vel_spread": Vector3(0.28, 0.16, 0.28), "accel": Vector3(0.0, 0.25, 0.0),
				"drag": 0.8, "size0": 0.3, "size1": 0.8, "size_rand": 0.35, "spin": 0.9, "tint_mix": 0.7,
				"color0": SpectacleKit.with_alpha(ink, 0.95), "color1": SpectacleKit.with_alpha(ink, 0.95), "tint_b": SpectacleKit.INK_LIGHT})))
			_outcome.append(_add(SpectacleKit.sprites(self, "smoke_07", 8, {"seed": s + 1.0, "emit_span": 0.5, "life": 1.0,
				"fade_in": 0.12, "fade_out_pow": 0.9, "origin_off": origin, "extent": Vector3(0.2, 0.04, 0.2),
				"vel": Vector3(0.0, 0.85, 0.0), "vel_spread": Vector3(0.2, 0.2, 0.2), "accel": Vector3(0.0, 0.3, 0.0),
				"drag": 0.7, "size0": 0.22, "size1": 0.62, "size_rand": 0.3, "spin": -0.7,
				"color0": SpectacleKit.with_alpha(ink, 0.9), "color1": SpectacleKit.with_alpha(ink, 0.9)})))
			_outcome.append(_add(SpectacleKit.sprites(self, "spark_01", 8, {"seed": s + 2.0, "emit_span": 0.3, "life": 0.9,
				"fade_in": 0.05, "fade_out_pow": 1.0, "origin_off": origin, "extent": Vector3(0.12, 0.03, 0.12),
				"vel": Vector3(0.0, 0.7, 0.0), "vel_spread": Vector3(0.45, 0.35, 0.45), "drag": 1.4, "size0": 0.08,
				"size1": 0.015, "color0": SpectacleKit.GOLD_LIGHT, "color1": SpectacleKit.RED})))


func tick(age: float, ctx: Dictionary) -> void:
	var s := float(ctx["scale"])
	var fade_out := 1.0 - span(age, OUTCOME_START, END)
	for cloud in _clouds:
		SpectacleKit.set_u(cloud, "age", age)
		SpectacleKit.set_u(cloud, "master", fade_out)
	var flash := _flash_at(age)
	SpectacleKit.set_all(_clouds, "flash", flash * 0.55)
	SpectacleKit.set_u(_underglow, "age", age)
	SpectacleKit.set_u(_underglow, "master", flash * 0.7 * fade_out)
	for node in _outcome:
		SpectacleKit.set_u(node, "age", age - OUTCOME_START)
	_tick_bolts(age, s * CLOUD_HEIGHT / BOLT_NATIVE_HEIGHT)
	if _figure != null and ctx.has("camera"):
		var doing := figure_action(age)
		_figure.play(str(doing["action"]), float(doing["time"]))
		_figure.place(global_position, ctx["up"], ctx["camera"], float(ctx["u"]), bool(ctx.get("figure_hidden", false)))


## 各道雷打亮劫雲的強度：每道雷一閃（約 0.1 秒衰減），後面再一次較弱的回閃。
func _flash_at(age: float) -> float:
	var total := 0.0
	for bolt: Dictionary in _bolts:
		var since := age - float(bolt["time"]) - 0.06
		if since >= 0.0:
			total += exp(-since / 0.1)
		if since >= 0.14:
			total += 0.6 * exp(-(since - 0.14) / 0.1)
	return minf(total, 1.0)


func _tick_bolts(age: float, global_scale: float) -> void:
	for bolt: Dictionary in _bolts:
		if bolt["done"]:
			continue
		var target := age - float(bolt["time"])
		if target < 0.0:
			continue
		if bolt["fx"] == null:
			bolt["fx"] = _spawn_bolt(bolt)
		var fx := bolt["fx"] as StormFX
		if not is_instance_valid(fx):
			bolt["done"] = true
			continue
		fx.set_param(&"thickness", global_scale * 2.0)
		while float(bolt["t"]) < target and is_instance_valid(fx):
			var step := minf(STEP, target - float(bolt["t"]))
			fx._process(step)
			bolt["t"] = float(bolt["t"]) + step
		if not is_instance_valid(fx):
			bolt["done"] = true


func _spawn_bolt(bolt: Dictionary) -> StormFX:
	var holder := Node3D.new()
	holder.position = bolt["offset"]
	holder.scale = Vector3.ONE * (CLOUD_HEIGHT / BOLT_NATIVE_HEIGHT)
	add_child(holder)
	var fx := StormFX.add(holder, "holy_strike", {
		"seed": bolt["seed"], "interval": 0.0, "light": false, "particles": true, "one_shot": true, "duration": 0.22,
		"radius": 1.3, "height": BOLT_NATIVE_HEIGHT, "color": Color("FFFBEA"), "edge_color": Color("E6A93A"), "energy": 1.0,
		"transparency": 0})
	fx.set_process(false)
	for material: ShaderMaterial in fx._materials:
		material.render_priority = SpectacleKit.PRIORITY + 3
	return fx
