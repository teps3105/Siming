class_name RebirthStage
extends SpectacleStage
## 轉世（5.13 棋子演出的光效）：一道光自上方落到事件格（定場 LAND 秒），落地後地面一圈光暈散開再淡出。
## 光柱、光點與光暈都重用飛升的材質（fx_beam、fx_sprites）與 Kenney 貼圖；節點原點在事件格地表，縮放等於身高，
## 所以子節點以「身高為 1」擺放。時間由演出的 age 決定（含四段時間線的整體淡入淡出）。

## 光點起落的高度（身高倍數）與柱高。
const DROP_FROM := 3.4
const SHAFT_HEIGHT := 3.6
## 落地後地面光暈的存續秒數。
const RING_LIFE := 3.4

var _total := 0.0
var _shaft: MeshInstance3D
var _falling: Array[MeshInstance3D] = []
var _ground: Array[MeshInstance3D] = []


func setup(spec: Dictionary) -> void:
	super.setup(spec)
	_total = float(spec["total"])
	var s := float(int(spec.get("seed", 0)) % 79)
	var land := EventScript.LIGHT_LAND
	_shaft = MeshInstance3D.new()
	_shaft.mesh = SpectacleKit.beam_mesh()
	_shaft.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_shaft.custom_aabb = AABB(Vector3(-100000.0, -100000.0, -100000.0), Vector3.ONE * 200000.0)
	var material := ShaderMaterial.new()
	material.shader = SpectacleKit.BEAM
	material.render_priority = SpectacleKit.PRIORITY
	material.set_shader_parameter("noise", SpectacleKit.noise())
	material.set_shader_parameter("width", 0.16)
	material.set_shader_parameter("edge_color", SpectacleKit.GOLD_LIGHT)
	material.set_shader_parameter("core_color", Color.WHITE)
	material.set_shader_parameter("flow", -1.2)
	material.set_shader_parameter("noise_scale", 1.8)
	material.set_shader_parameter("top_width", 0.7)
	material.set_shader_parameter("seed", s * 0.13)
	material.set_shader_parameter("height", SHAFT_HEIGHT)
	_shaft.material_override = material
	add_child(_shaft)
	_systems.append(_shaft)
	# 落下的光點：自上方勻速落到地面，落地那刻消失。
	var fall := {"emit_span": 0.001, "life": land, "fade_in": 0.2, "fade_out_pow": 0.0,
		"origin_off": Vector3(0.0, DROP_FROM, 0.0), "vel": Vector3(0.0, -(DROP_FROM - 0.1) / land, 0.0)}
	_falling.append(_add(SpectacleKit.sprites(self, "light_01", 1, fall.merged({"seed": s, "size0": 0.9, "size1": 0.9,
		"color0": SpectacleKit.with_alpha(SpectacleKit.PALE, 0.95), "color1": SpectacleKit.with_alpha(SpectacleKit.PALE, 0.95)}), true)))
	_falling.append(_add(SpectacleKit.sprites(self, "circle_05", 1, fall.merged({"seed": s + 1.0, "size0": 0.2, "size1": 0.2,
		"color0": Color.WHITE, "color1": Color.WHITE}))))
	# 地面光暈：落地後一圈環向外散開、一層暖光在原地漲開，皆淡出（加法一層、一般混合一層，亮底上也看得見）。
	var ring := {"emit_span": 0.001, "life": RING_LIFE, "fade_in": 0.05, "fade_out_pow": 0.8,
		"origin_off": Vector3(0.0, 0.3, 0.0), "flat_plane": true}
	_ground.append(_add(SpectacleKit.sprites(self, "circle_02", 1, ring.merged({"seed": s + 2.0, "size0": 0.2, "size1": 1.5,
		"color0": SpectacleKit.with_alpha(SpectacleKit.GOLD_LIGHT, 0.95), "color1": SpectacleKit.with_alpha(SpectacleKit.GOLD_LIGHT, 0.95)}), true)))
	_ground.append(_add(SpectacleKit.sprites(self, "circle_02", 1, ring.merged({"seed": s + 3.0, "size0": 0.2, "size1": 1.5,
		"color0": SpectacleKit.with_alpha(SpectacleKit.GOLD, 1.0), "color1": SpectacleKit.with_alpha(SpectacleKit.GOLD, 1.0)}))))
	_ground.append(_add(SpectacleKit.sprites(self, "circle_02", 1, ring.merged({"seed": s + 5.0, "size0": 0.15, "size1": 1.2,
		"color0": SpectacleKit.with_alpha(SpectacleKit.AMBER, 0.85), "color1": SpectacleKit.with_alpha(SpectacleKit.AMBER, 0.85)}))))
	_ground.append(_add(SpectacleKit.sprites(self, "circle_05", 1, ring.merged({"seed": s + 4.0, "size0": 0.15, "size1": 1.0,
		"color0": SpectacleKit.with_alpha(SpectacleKit.PALE, 0.8), "color1": SpectacleKit.with_alpha(SpectacleKit.PALE, 0.8)}), true)))


func tick(age: float, _ctx: Dictionary) -> void:
	var land := EventScript.LIGHT_LAND
	var master := EventScript.opacity_at(_total, age)
	var shaft := span(age, 0.0, land) * (1.0 - span(age, land, land + 1.0))
	SpectacleKit.set_u(_shaft, "age", age)
	SpectacleKit.set_u(_shaft, "master", shaft * master)
	for node in _falling:
		SpectacleKit.set_u(node, "age", age)
		SpectacleKit.set_u(node, "master", master)
	for node in _ground:
		SpectacleKit.set_u(node, "age", age - land)
		SpectacleKit.set_u(node, "master", master)
