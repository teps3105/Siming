class_name AscendedStage
extends SpectacleStage
## 飛升（6 秒）：金色光柱自所在貫天（0–0.8 秒升起），雲層被衝開一個洞，持續到 4.5 秒後淡出。
## 光柱是三層繞鉛直軸朝向鏡頭的帶，有往上流的雜訊紋理與邊緣衰減；往上衝的金屑與雲洞的煙雲是貼圖粒子（Kenney Particle Pack）。

const RISE := 0.8
const FADE_START := 4.5
const END := 6.0

var _beams: Array[MeshInstance3D] = []
var _beam_heights: Array[float] = []
var _glows: Array[MeshInstance3D] = []
var _hole: Array[MeshInstance3D] = []
var _hole_height := 3.0
var _built_height := -1.0


func setup(spec: Dictionary) -> void:
	super.setup(spec)
	var s := float(int(spec["seed"]) % 79)
	var gold := SpectacleKit.GOLD
	var gold_light := SpectacleKit.GOLD_LIGHT
	var layers := [
		[0.3, SpectacleKit.AMBER, gold_light, 0.8, 0.45],
		[0.17, gold, SpectacleKit.PALE, 0.9, 0.5],
		[0.07, gold_light, Color.WHITE, 1.0, 0.6],
	]
	for i in layers.size():
		var layer: Array = layers[i]
		var node := MeshInstance3D.new()
		node.mesh = SpectacleKit.beam_mesh()
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.custom_aabb = AABB(Vector3(-100000.0, -100000.0, -100000.0), Vector3.ONE * 200000.0)
		var material := ShaderMaterial.new()
		material.shader = SpectacleKit.BEAM
		material.render_priority = SpectacleKit.PRIORITY + i
		material.set_shader_parameter("noise", SpectacleKit.noise())
		material.set_shader_parameter("width", float(layer[0]))
		material.set_shader_parameter("edge_color", layer[1])
		material.set_shader_parameter("core_color", layer[2])
		material.set_shader_parameter("flow", 0.9 + 0.5 * i)
		material.set_shader_parameter("noise_scale", 1.6 + 0.8 * i)
		material.set_shader_parameter("top_width", float(layer[4]))
		material.set_shader_parameter("seed", s * 0.13 + i * 0.31)
		node.material_override = material
		add_child(node)
		_systems.append(node)
		_beams.append(node)
	# 落腳處的光暈：加法一層、一般混合一層（亮底上也看得見）。
	_glows.append(_add(SpectacleKit.sprites(self, "circle_05", 1, {"seed": s, "emit_span": 0.001, "life": 60.0,
		"fade_in": 0.3, "fade_out_pow": 0.0, "origin_off": Vector3(0.0, 0.1, 0.0), "size0": 0.75, "size1": 0.75, "aspect": 0.75, "flat_plane": true,
		"color0": SpectacleKit.with_alpha(gold, 0.65), "color1": SpectacleKit.with_alpha(gold, 0.65)})))
	_glows.append(_add(SpectacleKit.sprites(self, "circle_05", 1, {"seed": s, "emit_span": 0.001, "life": 60.0,
		"fade_in": 0.3, "fade_out_pow": 0.0, "origin_off": Vector3(0.0, 0.1, 0.0), "size0": 0.55, "size1": 0.55, "aspect": 0.75, "flat_plane": true,
		"color0": Color(1.0, 0.85, 0.45, 0.9), "color1": Color(1.0, 0.85, 0.45, 0.9)}, true)))
	# 往上衝的金屑：細長的亮筆與小星芒，循環。
	_glows.append(_add(SpectacleKit.sprites(self, "trace_02", 40, {"seed": s + 1.0, "looping": true, "life": 1.5,
		"fade_in": 0.15, "fade_out_pow": 0.8, "origin_off": Vector3(0.0, 0.1, 0.0), "extent": Vector3(0.15, 0.02, 0.15), "shell": 0.5,
		"vel": Vector3(0.0, 2.6, 0.0), "vel_spread": Vector3(0.1, 0.7, 0.1), "stretch": 0.22, "aspect": 1.6, "size0": 0.07, "size1": 0.035,
		"size_rand": 0.4, "color0": Color("FFF2C0"), "color1": gold})))
	_glows.append(_add(SpectacleKit.sprites(self, "star_06", 18, {"seed": s + 2.0, "looping": true, "life": 2.3,
		"fade_in": 0.2, "fade_out_pow": 1.0, "origin_off": Vector3(0.0, 0.1, 0.0), "extent": Vector3(0.2, 0.02, 0.2),
		"vel": Vector3(0.0, 1.3, 0.0), "vel_spread": Vector3(0.1, 0.4, 0.1), "swirl": 1.2, "size0": 0.2, "size1": 0.07,
		"size_rand": 0.4, "color0": gold_light, "color1": gold}, true)))


## 雲洞：高度依光柱的高度定，所以第一次 tick 才建。
func _build_hole(height: float) -> void:
	for node in _hole:
		node.queue_free()
		_systems.erase(node)
	_hole.clear()
	_hole_height = height
	var s := float(int(item["seed"]) % 71)
	var at := Vector3(0.0, height, 0.0)
	_hole.append(_add(SpectacleKit.sprites(self, "smoke_04", 18, {"seed": s, "emit_span": 1.2, "life": 30.0,
		"fade_in": 0.5, "fade_out_pow": 0.0, "origin_off": at, "extent": Vector3(0.3, 0.03, 0.24), "shell": 1.0,
		"radial": 0.95, "drag": 0.9, "swirl": 0.18, "size0": 0.55, "size1": 0.75, "size_rand": 0.3, "spin": 0.5,
		"tint_mix": 0.8, "color0": SpectacleKit.with_alpha(SpectacleKit.INK_LIGHT, 0.82),
		"color1": SpectacleKit.with_alpha(SpectacleKit.INK_LIGHT, 0.82), "tint_b": SpectacleKit.INK})))
	_hole.append(_add(SpectacleKit.sprites(self, "smoke_09", 4, {"seed": s + 1.0, "emit_span": 1.0, "life": 30.0,
		"fade_in": 0.5, "fade_out_pow": 0.0, "origin_off": at, "extent": Vector3(0.05, 0.0, 0.05), "radial": 0.5,
		"drag": 1.0, "size0": 0.7, "size1": 1.05, "spin": 0.3, "color0": SpectacleKit.with_alpha(SpectacleKit.INK, 0.6),
		"color1": SpectacleKit.with_alpha(SpectacleKit.INK, 0.6)})))
	_hole.append(_add(SpectacleKit.sprites(self, "light_01", 1, {"seed": s, "emit_span": 0.001, "life": 60.0,
		"fade_in": 0.8, "fade_out_pow": 0.0, "origin_off": at, "size0": 1.5, "size1": 1.5,
		"color0": Color(1.0, 0.85, 0.45, 0.85), "color1": Color(1.0, 0.85, 0.45, 0.85)}, true)))
	_hole.append(_add(SpectacleKit.sprites(self, "circle_05", 1, {"seed": s, "emit_span": 0.001, "life": 60.0,
		"fade_in": 0.8, "fade_out_pow": 0.0, "origin_off": at, "size0": 0.55, "size1": 0.55, "aspect": 0.8,
		"color0": SpectacleKit.with_alpha(SpectacleKit.GOLD, 0.55), "color1": SpectacleKit.with_alpha(SpectacleKit.GOLD, 0.55)})))


func tick(age: float, ctx: Dictionary) -> void:
	var beam_height: float = ctx["beam_height"]
	if absf(beam_height - _built_height) > 0.25:
		_built_height = beam_height
		_build_hole(minf(3.0, beam_height * 0.75))
	var rise := SpectacleKit.ease_out(age / RISE)
	var fade := 1.0 - span(age, FADE_START, END)
	for beam in _beams:
		SpectacleKit.set_u(beam, "age", age)
		SpectacleKit.set_u(beam, "height", beam_height)
		SpectacleKit.set_u(beam, "rise", rise)
		SpectacleKit.set_u(beam, "master", fade)
	for node in _systems:
		if not _beams.has(node):
			SpectacleKit.set_u(node, "age", age)
			SpectacleKit.set_u(node, "master", fade * (rise if _glows.has(node) else 1.0))
