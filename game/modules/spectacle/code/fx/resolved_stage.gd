class_name ResolvedStage
extends SpectacleStage
## 宿緣了結（3 秒）：宿緣線自兩端向中點燃盡（前 2.25 秒，燒盡處留焦墨），燒處迸出拖尾的火星與餘燼，
## 中點炸開（2.15 秒起）：環、星芒、火花與一團墨煙（亮底上也看得見）。顏色依宿緣種類。
## 這個舞台不跟著事件格擺放：兩端與中點的世界座標每幀由 Spectacle 給（ctx 的 a、b、control）。

const BURN := 2.25
const BURST_START := 2.15
const END := 3.0

var _ribbon: MeshInstance3D
var _embers: Array[MeshInstance3D] = []
var _burst: Node3D
var _burst_systems: Array[MeshInstance3D] = []
var _kind_color := SpectacleKit.RED


func setup(spec: Dictionary) -> void:
	super.setup(spec)
	var s := float(int(spec["seed"]) % 61)
	_kind_color = KarmaLayer.KIND_COLORS[clampi(int(spec["thread_kind"]), 0, KarmaLayer.KIND_COLORS.size() - 1)]
	_ribbon = MeshInstance3D.new()
	_ribbon.mesh = SpectacleKit.thread_mesh()
	_ribbon.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_ribbon.custom_aabb = AABB(Vector3(-100000.0, -100000.0, -100000.0), Vector3.ONE * 200000.0)
	var material := ShaderMaterial.new()
	material.shader = SpectacleKit.THREAD
	material.render_priority = SpectacleKit.PRIORITY
	material.set_shader_parameter("kind_color", _kind_color)
	material.set_shader_parameter("pale", SpectacleKit.PALE)
	material.set_shader_parameter("ink", SpectacleKit.INK)
	material.set_shader_parameter("hot", SpectacleKit.GOLD_LIGHT)
	_ribbon.material_override = material
	add_child(_ribbon)
	# 火星：沿燒處出生（曲線起點模式），細長的拖尾加小亮點，從熱金冷到朱紅再到墨。
	var curve := {"curve_emit": true, "looping": true, "burn_seconds": BURN}
	_embers.append(SpectacleKit.sprites(self, "trace_02", 40, curve.merged({"seed": s, "life": 0.9, "fade_in": 0.03,
		"fade_out_pow": 1.0, "extent": Vector3(0.03, 0.02, 0.03), "vel": Vector3(0.0, 0.55, 0.0),
		"vel_spread": Vector3(0.4, 0.35, 0.4), "accel": Vector3(0.0, 0.5, 0.0), "drag": 1.4, "stretch": 0.5, "aspect": 1.4,
		"size0": 0.045, "size1": 0.015, "size_rand": 0.4, "color0": SpectacleKit.GOLD_LIGHT, "color1": Color("8E2B22")}),
		false, SpectacleKit.PRIORITY + 2, true))
	_embers.append(SpectacleKit.sprites(self, "star_06", 14, curve.merged({"seed": s + 1.0, "life": 1.3, "fade_in": 0.05,
		"fade_out_pow": 1.0, "extent": Vector3(0.04, 0.02, 0.04), "vel": Vector3(0.0, 0.3, 0.0),
		"vel_spread": Vector3(0.25, 0.2, 0.25), "accel": Vector3(0.0, 0.35, 0.0), "drag": 1.0, "size0": 0.09, "size1": 0.03,
		"size_rand": 0.4, "color0": Color("FFF2C0"), "color1": SpectacleKit.RED}), true, SpectacleKit.PRIORITY + 3, true))
	_embers.append(SpectacleKit.sprites(self, "smoke_07", 10, curve.merged({"seed": s + 2.0, "life": 1.2, "fade_in": 0.2,
		"fade_out_pow": 1.0, "extent": Vector3(0.03, 0.02, 0.03), "vel": Vector3(0.0, 0.35, 0.0),
		"vel_spread": Vector3(0.15, 0.1, 0.15), "drag": 0.8, "size0": 0.07, "size1": 0.18, "size_rand": 0.3, "spin": 0.8,
		"color0": SpectacleKit.with_alpha(SpectacleKit.INK, 0.5), "color1": SpectacleKit.with_alpha(SpectacleKit.INK, 0.5)}),
		false, SpectacleKit.PRIORITY + 1, true))
	for node in _embers:
		_systems.append(node)
	_build_burst(s)


func _build_burst(s: float) -> void:
	_burst = Node3D.new()
	add_child(_burst)
	var kind := _kind_color
	var ink := SpectacleKit.INK
	var gold_light := SpectacleKit.GOLD_LIGHT
	# 墨煙墊底：亮底上的爆點靠它襯出來。
	_burst_systems.append(SpectacleKit.sprites(_burst, "smoke_07", 7, {"seed": s, "emit_span": 0.08, "life": 0.9,
		"fade_in": 0.04, "fade_out_pow": 1.0, "extent": Vector3(0.1, 0.05, 0.1), "radial": 0.45, "drag": 1.3,
		"vel_spread": Vector3(0.1, 0.15, 0.1), "size0": 0.4, "size1": 1.0, "size_rand": 0.3, "spin": 1.0,
		"color0": SpectacleKit.with_alpha(ink, 0.6), "color1": SpectacleKit.with_alpha(ink, 0.6)}, false, SpectacleKit.PRIORITY))
	# 兩圈環：外一圈墨、內一圈種類色。
	_burst_systems.append(SpectacleKit.sprites(_burst, "circle_02", 1, {"seed": s, "emit_span": 0.001, "life": 0.85,
		"fade_in": 0.02, "fade_out_pow": 1.2, "size0": 0.2, "size1": 1.5, "flat_plane": true,
		"color0": SpectacleKit.with_alpha(ink, 0.7), "color1": SpectacleKit.with_alpha(ink, 0.7)}, false, SpectacleKit.PRIORITY + 1))
	_burst_systems.append(SpectacleKit.sprites(_burst, "circle_02", 1, {"seed": s, "emit_span": 0.001, "life": 0.85,
		"fade_in": 0.02, "fade_out_pow": 1.2, "size0": 0.16, "size1": 1.3, "flat_plane": true,
		"color0": SpectacleKit.with_alpha(kind, 1.0), "color1": SpectacleKit.with_alpha(kind, 1.0)}, false, SpectacleKit.PRIORITY + 2))
	# 星芒：種類色一層、加法的亮金一層。
	_burst_systems.append(SpectacleKit.sprites(_burst, "star_08", 1, {"seed": s, "emit_span": 0.001, "life": 0.6,
		"fade_in": 0.02, "fade_out_pow": 1.0, "size0": 1.5, "size1": 0.4,
		"color0": SpectacleKit.with_alpha(kind, 1.0), "color1": SpectacleKit.with_alpha(kind, 1.0)}, false, SpectacleKit.PRIORITY + 3))
	_burst_systems.append(SpectacleKit.sprites(_burst, "star_08", 1, {"seed": s, "emit_span": 0.001, "life": 0.5,
		"fade_in": 0.02, "fade_out_pow": 1.0, "size0": 1.1, "size1": 0.3, "color0": gold_light, "color1": gold_light},
		true, SpectacleKit.PRIORITY + 4))
	# 火花：向四方迸、各拖一筆，一半種類色、一半金。
	_burst_systems.append(SpectacleKit.sprites(_burst, "trace_02", 18, {"seed": s + 1.0, "emit_span": 0.08, "life": 0.8,
		"fade_in": 0.02, "fade_out_pow": 1.0, "extent": Vector3(0.03, 0.02, 0.03), "radial": 1.7, "drag": 2.0,
		"vel_spread": Vector3(0.9, 0.6, 0.9), "stretch": 0.5, "aspect": 1.6, "size0": 0.1, "size1": 0.05,
		"tint_mix": 1.0, "color0": kind, "color1": kind, "tint_b": gold_light}, false, SpectacleKit.PRIORITY + 3))
	_burst_systems.append(SpectacleKit.sprites(_burst, "spark_01", 12, {"seed": s + 2.0, "emit_span": 0.1, "life": 0.8,
		"fade_in": 0.02, "fade_out_pow": 1.0, "extent": Vector3(0.04, 0.02, 0.04), "radial": 1.2, "drag": 1.6,
		"vel_spread": Vector3(0.7, 0.5, 0.7), "accel": Vector3(0.0, -0.3, 0.0), "size0": 0.14, "size1": 0.03, "size_rand": 0.4,
		"color0": Color.WHITE, "color1": gold_light}, true, SpectacleKit.PRIORITY + 4))


func tick(age: float, ctx: Dictionary) -> void:
	var a: Vector3 = ctx["a"]
	var b: Vector3 = ctx["b"]
	var control: Vector3 = ctx["control"]
	var burn := clampf(age / BURN, 0.0, 1.0)
	var live := 1.0 - span(age, BURN, BURN + 0.2)
	var material := _ribbon.material_override as ShaderMaterial
	material.set_shader_parameter("pa", a)
	material.set_shader_parameter("pc", control)
	material.set_shader_parameter("pb", b)
	material.set_shader_parameter("half_width", float(ctx["line_half_width"]))
	material.set_shader_parameter("reach", burn * 0.5)
	material.set_shader_parameter("live", live)
	material.set_shader_parameter("char_alpha", 0.5 * (1.0 - span(age, 1.0, 3.0)))
	for node in _embers:
		SpectacleKit.set_u(node, "age", age)
		SpectacleKit.set_u(node, "pa", a)
		SpectacleKit.set_u(node, "pc", control)
		SpectacleKit.set_u(node, "pb", b)
		SpectacleKit.set_u(node, "world_scale", float(ctx["scale"]))
	# 中點炸開：節點放在弧的中點，縮放同半徑。
	_burst.global_transform = Transform3D(Basis.IDENTITY.scaled(Vector3.ONE * float(ctx["scale"])), 0.25 * a + 0.5 * control + 0.25 * b)
	for node in _burst_systems:
		SpectacleKit.set_u(node, "age", age - BURST_START)
