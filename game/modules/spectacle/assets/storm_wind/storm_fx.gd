@tool
class_name StormFX
extends Node3D
## Lightning and wind in 3D. Lightning: a strike from the sky, an arc between points, ball
## lightning, a nova along the ground and a storm cloud, in six colours. Wind: a gust, a swirl, a
## tornado and a burst, of air, leaves, snow, sand, petals or ash. Shaders and meshes made in code:
## no textures, and what the wind carries moves on the GPU.
##
##     StormFX.add(self, "storm_strike")
##     StormFX.add(self, "leaves_gust", {rotation = Vector3(0, PI / 2, 0)})   # blows along its -Z
##     var arc := StormFX.add(hand, "arcane_arc")
##     arc.aim([enemy.global_position, next_enemy.global_position])           # jumps one to the next
##     StormFX.add(self, "sand_burst", {one_shot = true})                      # one burst, then frees itself
##
## Options set a property of the node or else a uniform of its shaders (shaders/storm.gdshaderinc).
##
## Modified for Siming (modified copy of Storm & Wind 3D Free by heyheythere, CC BY 4.0): only the
## strike kind and its shaders are kept (bolt, strike, spark), SHADERS points at this folder, and
## storm_ribbon turns ribbons to a fixed view axis because the map camera is orthographic.

signal finished
## A strike, nova or burst landing, where it lands (global).
signal struck(at: Vector3)

const SHADERS := "res://modules/spectacle/assets/storm_wind/shaders/%s.gdshader"
## Each kind's own shader: the kind is installed when it is.
const KINDS := {
	"strike": "strike", "arc": "arc", "orb": "orb", "nova": "nova", "cloud": "cloud",
	"gust": "line", "swirl": "swirl", "tornado": "tornado", "burst": "burst",
}
const LIGHTNING := ["strike", "arc", "orb", "nova", "cloud"]
## The kinds that go off every `interval` (or on strike()) rather than run on.
const STRIKES := ["strike", "nova", "cloud", "burst"]
## Lightning's colours: the bolt's core and its glow.
const BOLT_STYLES := {
	"storm": {color = Color(0.86, 0.94, 1.0), edge_color = Color(0.3, 0.55, 1.0)},
	"arcane": {color = Color(0.97, 0.88, 1.0), edge_color = Color(0.62, 0.26, 1.0)},
	"fire": {color = Color(1.0, 0.93, 0.66), edge_color = Color(1.0, 0.32, 0.06)},
	"toxic": {color = Color(0.93, 1.0, 0.72), edge_color = Color(0.36, 0.95, 0.12)},
	"holy": {color = Color(1.0, 1.0, 0.9), edge_color = Color(1.0, 0.76, 0.22)},
	"void": {color = Color(0.05, 0.0, 0.09), edge_color = Color(0.66, 0.2, 1.0)},
}
## Wind's: the lines' colour and the dust's (funnels, haze, ground; its alpha how thick), then what
## it carries: debris.gdshader's shape, how many (times the kind's), half-size in metres, two tints,
## how far it falls (below 0 rises) and wanders, how much it streaks, and the haze (0 to 1).
const WIND_STYLES := {
	"air": {color = Color(0.94, 0.97, 1.0, 0.9), edge_color = Color(0.72, 0.78, 0.86, 0.35), shape = 0,
		pieces = 0.5, size = 0.02, tint_a = Color(0.9, 0.92, 0.95, 0.5), tint_b = Color(0.8, 0.82, 0.86, 0.4),
		fall = 0.0, flutter = 0.1, stretch = 2.0, haze = 0.0, lines = 1.4},
	"leaves": {color = Color(1.0, 0.97, 0.88, 0.75), edge_color = Color(0.5, 0.4, 0.26, 0.4), shape = 1,
		pieces = 0.8, size = 0.1, tint_a = Color(0.98, 0.58, 0.12), tint_b = Color(0.72, 0.16, 0.06),
		fall = 0.6, flutter = 0.35, stretch = 0.0, haze = 0.0, lines = 1.0},
	"snow": {color = Color(0.92, 0.96, 1.0, 0.85), edge_color = Color(0.86, 0.91, 1.0, 0.5), shape = 2,
		pieces = 2.2, size = 0.045, tint_a = Color(1.0, 1.0, 1.0), tint_b = Color(0.8, 0.88, 1.0, 0.8),
		fall = 0.8, flutter = 0.25, stretch = 0.0, haze = 0.5, lines = 1.0},
	"sand": {color = Color(1.0, 0.9, 0.7, 0.75), edge_color = Color(0.8, 0.62, 0.38, 0.6), shape = 0,
		pieces = 3.0, size = 0.022, tint_a = Color(0.95, 0.8, 0.55), tint_b = Color(0.7, 0.5, 0.3),
		fall = 0.2, flutter = 0.08, stretch = 3.0, haze = 1.0, lines = 1.0},
	"petals": {color = Color(1.0, 0.96, 0.98, 0.75), edge_color = Color(0.95, 0.75, 0.82, 0.25), shape = 3,
		pieces = 1.0, size = 0.065, tint_a = Color(1.0, 0.72, 0.84), tint_b = Color(1.0, 0.88, 0.93),
		fall = 0.4, flutter = 0.4, stretch = 0.0, haze = 0.0, lines = 1.0},
	"ash": {color = Color(0.86, 0.82, 0.8, 0.6), edge_color = Color(0.3, 0.27, 0.26, 0.55), shape = 4,
		pieces = 1.6, size = 0.045, tint_a = Color(0.42, 0.4, 0.4), tint_b = Color(1.0, 0.5, 0.12),
		fall = -0.3, flutter = 0.2, stretch = 0.0, haze = 0.6, lines = 0.8},
}
## Per wind kind: its lines, its pieces (times the style's), and the seconds a piece takes through.
const WIND_KINDS := {
	"gust": {lines = 18, pieces = 80, period = 2.2},
	"swirl": {lines = 9, pieces = 50, period = 1.8},
	"tornado": {lines = 12, pieces = 140, period = 2.6},
	"burst": {lines = 20, pieces = 70, period = 0.9},
}
const LEADER := 0.06   # seconds a bolt takes to reach the ground before its first stroke

## Dither or cutout for every StormFX whose transparency is "default": set once, before adding any,
## for a PS1-style game (dither) or one with no sorted transparency (cutout).
static var default_transparency := 0

@export_enum("strike", "arc", "orb", "nova", "cloud", "gust", "swirl", "tornado", "burst") var kind := "strike":
	set(v):
		kind = v
		_rebuild()
## Lightning takes storm to void, wind air to ash. Picking one sets the colours to the style's.
@export_enum("storm", "arcane", "fire", "toxic", "holy", "void", "air", "leaves", "snow", "sand", "petals", "ash")
var style := "storm":
	set(v):
		style = v
		var s := _style()
		color = s.color
		edge_color = s.edge_color
		_rebuild()
@export var color := Color(0.86, 0.94, 1.0):
	set(v):
		color = v
		set_param(&"color", v)
		_light_color()
@export var edge_color := Color(0.3, 0.55, 1.0):
	set(v):
		edge_color = v
		set_param(&"edge_color", v)
		_light_color()
## The area it covers: a strike's ground glow, a cloud's or a gust's half-width, a tornado's top.
@export_range(0.1, 20.0, 0.05, "or_greater", "suffix:m") var radius := 1.5:
	set(v):
		radius = v
		_rebuild()
## Where a strike comes from, a cloud sits, the top of a gust, swirl or tornado.
@export_range(0.1, 40.0, 0.05, "or_greater", "suffix:m") var height := 4.0:
	set(v):
		height = v
		_rebuild()
## An arc's points in the node's space, jumped through in order from the node; aim() takes global ones.
@export var targets := PackedVector3Array([Vector3(3, 0, 0)]):
	set(v):
		targets = v
		_place_ends()
## Seconds between strikes, novas and bursts, give or take 40%; 0 for only when strike() is called.
@export_range(0.0, 20.0, 0.05, "or_greater", "suffix:s") var interval := 1.8
@export_range(0.0, 4.0, 0.05) var speed := 1.0
@export_range(0.0, 2.0, 0.05) var intensity := 1.0
@export var particles := true:
	set(v):
		particles = v
		_rebuild()
## Lightning's OmniLight3D, flashing with each stroke; void's takes light away.
@export var light := true:
	set(v):
		light = v
		_rebuild()
@export_enum("default:-1", "smooth:0", "dither:1", "cutout:2") var transparency := -1:
	set(v):
		transparency = v
		set_param(&"transparency", _transparency())
@export var autoplay := true
## Plays once, holds for `duration`, fades and frees itself, instead of running on: one strike,
## nova or burst.
@export var one_shot := false
## Seconds a one-shot holds at full strength.
@export_range(0.0, 30.0, 0.1, "or_greater", "suffix:s") var duration := 1.5
## Seconds wind takes to rise; lightning forms by its leader.
@export_range(0.0, 5.0, 0.05, "suffix:s") var appear := 0.5

enum { IDLE, FORMING, HOLDING, FADING }

var seed := 0.0:
	set(v):
		seed = v
		set_param(&"seed", v)
var _params := {}
var _dirty := true
var _materials: Array[ShaderMaterial] = []
var _parts: Array[Node3D] = []
var _bolt: MeshInstance3D
var _ground: MeshInstance3D
var _ends: MeshInstance3D
var _sparks: CPUParticles3D
var _light: OmniLight3D
var _rng := RandomNumberGenerator.new()
var _age := 0.0
var _phase := IDLE
var _t := 0.0
var _fade := 0.6
var _grow := 1.0
var _strength := 1.0
var _fired := -100.0
var _next := 0.0
var _shots := 0
var _strokes := PackedFloat32Array()
var _flick := 1.0
var _impact := Vector3.ZERO


## The kinds whose shader is installed.
static func kinds() -> PackedStringArray:
	return PackedStringArray(KINDS.keys().filter(func(k: String) -> bool:
		return ResourceLoader.exists(SHADERS % KINDS[k])))


## The styles a kind comes in.
static func styles(of_kind: String) -> PackedStringArray:
	return PackedStringArray((BOLT_STYLES if of_kind in LIGHTNING else WIND_STYLES).keys())


## Every installed effect by name, style_kind: "storm_strike", "sand_tornado"...
static func presets() -> PackedStringArray:
	var out := PackedStringArray()
	for k in kinds():
		for s in styles(k):
			out.append("%s_%s" % [s, k])
	return out


## Adds an effect to `parent`, playing, and returns it. `name` is a preset ("storm_strike") or a
## kind ("tornado", in its first style). Options as in set_option().
static func add(parent: Node, name: String, options := {}) -> StormFX:
	var fx := StormFX.new()
	var at := name.find("_")
	fx.kind = name.substr(at + 1)
	fx.style = name.left(at) if at > 0 else styles(fx.kind)[0]
	fx.seed = randf() * 10.0
	for key in options:
		fx.set_option(key, options[key])
	parent.add_child(fx)
	return fx


func set_option(key: StringName, value: Variant) -> void:
	if key in self:
		set(key, value)
	else:
		set_param(key, value)


## Sets a uniform on all of its shaders, particles' included; it holds through a rebuild.
func set_param(uniform: StringName, value: Variant) -> void:
	_params[uniform] = value
	for m in _materials:
		m.set_shader_parameter(uniform, value)


func get_param(uniform: StringName) -> Variant:
	if uniform in _params:
		return _params[uniform]
	return _materials[0].get_shader_parameter(uniform) if _materials else null


## An arc's points as global positions.
func aim(points: Array) -> void:
	targets = PackedVector3Array(points.map(func(p: Vector3) -> Vector3: return to_local(p)))


## Starts it again from nothing; a strike, nova or burst goes off at once.
func play() -> void:
	_age = 0.0
	_t = 0.0
	_shots = 0
	_fired = -100.0
	_next = 0.0
	_rng.seed = hash(seed)
	var form := appear if not kind in LIGHTNING else 0.0
	_phase = FORMING if form > 0.0 else HOLDING
	_grow = 0.0 if form > 0.0 else 1.0
	if _sparks:
		_sparks.emitting = particles and not kind in STRIKES
	set_param(&"fired", _fired)
	_update(0.0)


## Fades out over `fade` seconds, then emits finished and frees itself.
func stop(fade := 0.6) -> void:
	_phase = FADING
	_t = 0.0
	_fade = max(fade, 0.001)
	if _sparks and not kind in STRIKES:
		_sparks.emitting = false


## A strike, nova or burst now; an arc or orb flares and jumps.
func strike() -> void:
	if not kind in STRIKES:
		_flick = 1.4
		_shape()
		return
	_fired = _age
	_shots += 1
	_next = _age + interval * _rng.randf_range(0.6, 1.4)
	set_param(&"fired", _fired)
	_impact = Vector3.ZERO
	if kind == "cloud":
		var a := _rng.randf() * TAU
		_impact = Vector3(cos(a), 0.0, sin(a)) * sqrt(_rng.randf()) * radius * 0.8
	if kind in LIGHTNING:
		_strokes = PackedFloat32Array([0.0])
		var t := 0.0
		for i in _rng.randi_range(1, 3):
			t += _rng.randf_range(0.07, 0.18)
			_strokes.append(t)
		_shape()
	if _ground and kind == "cloud":
		_ground.position = _impact + Vector3(0, 0.02, 0)
	if _light:
		_light.position = _impact + Vector3(0, 1.0, 0)
	if _sparks and particles and kind in LIGHTNING:
		_sparks.position = _impact
		_sparks.restart()
	struck.emit(to_global(_impact) if is_inside_tree() else _impact)


## How strong it is now, 0 to 1: forming, holding or fading.
func strength() -> float:
	return _strength


func _enter_tree() -> void:
	if _dirty:
		_build()


func _ready() -> void:
	if autoplay or Engine.is_editor_hint():
		play()
		_settle()
	else:
		_phase = IDLE
		_update(0.0)


## Starts a running effect formed, its wind already blowing: what the scene shows on load.
func _settle() -> void:
	if one_shot and not Engine.is_editor_hint():
		return
	_phase = HOLDING
	_grow = 1.0
	_age = 4.0 + seed
	_next = _age + interval * _rng.randf() * 0.5
	if _sparks and not kind in STRIKES:
		_sparks.preprocess = _sparks.lifetime
		_sparks.restart()
	_update(0.0)


func _process(delta: float) -> void:
	_t += delta
	match _phase:
		FORMING:
			_grow = clampf(_t / appear, 0.0, 1.0)
			if _t >= appear:
				_phase = HOLDING
				_t = 0.0
		HOLDING:
			if one_shot and _t >= duration:
				if Engine.is_editor_hint():
					play()
				else:
					stop()
		FADING:
			if _t >= _fade:
				_phase = IDLE
				finished.emit()
				if not Engine.is_editor_hint():
					queue_free()
	_update(delta)


func _update(delta: float) -> void:
	_age += delta * speed
	var env := 0.0
	match _phase:
		FORMING:
			env = _grow
		HOLDING:
			env = 1.0
		FADING:
			env = 1.0 - clampf(_t / _fade, 0.0, 1.0)
	_strength = env
	var going := _phase == FORMING or _phase == HOLDING
	if kind in STRIKES and going and _age >= _next and (_shots == 0 if one_shot else interval > 0.0):
		strike()
	set_param(&"age", _age)
	set_param(&"intensity", intensity * env)
	if not kind in LIGHTNING:
		return
	var flash := 0.0
	var since := _age - _fired
	if kind in STRIKES:
		set_param(&"grow", clampf(since / LEADER, 0.0, 1.0))
		for s in _strokes:
			var t := since - LEADER - s
			if t >= 0.0:
				flash += exp(-t / 0.07)
		flash = minf(flash, 1.3)
		set_param(&"scorch", clampf(1.0 - (since - 0.5) / 3.0, 0.0, 1.0) if _shots > 0 else 0.0)
	else:
		if _age >= _next:
			_shape()
			_next = _age + _rng.randf_range(0.04, 0.09)
			_flick = _rng.randf_range(0.7, 1.0)
		flash = _flick
		_flick = minf(_flick, 1.0)
	set_param(&"flash", flash)
	if _light:
		var base := 5.0 if kind in STRIKES else 1.6
		_light.light_energy = base * flash * intensity * env * (0.6 if style == "void" else 1.0)


func _style() -> Dictionary:
	var all := BOLT_STYLES if kind in LIGHTNING else WIND_STYLES
	return all.get(style, all.values()[0])


func _transparency() -> int:
	return transparency if transparency >= 0 else default_transparency


func _light_color() -> void:
	if _light:
		_light.light_color = edge_color if style == "void" else color.lerp(edge_color, 0.5)


func _rebuild() -> void:
	if is_inside_tree():
		_build()
	else:
		_dirty = true


## The parts, particles and light for the kind, style, size and switches.
func _build() -> void:
	_dirty = false
	for n in _parts:
		n.free()
	_parts.clear()
	_materials.clear()
	_bolt = null
	_ground = null
	_ends = null
	_sparks = null
	_light = null
	if not kind in KINDS or not ResourceLoader.exists(SHADERS % KINDS[kind]):
		push_error("StormFX: unknown kind %s, pick one of %s" % [kind, kinds()])
		return
	_rng.seed = hash(seed)
	if kind in LIGHTNING:
		_build_lightning()
	else:
		_build_wind()
	_update(0.0)


func _build_lightning() -> void:
	_bolt = _mesh_part(ArrayMesh.new(), "bolt")
	match kind:
		"strike", "nova":
			_ground = _mesh_part(_disc(), kind)
			_ground.scale = Vector3(radius, 1.0, radius)
		"cloud":
			_mesh_part(_quads(26), "cloud")
			_ground = _mesh_part(_disc(), "strike")
			_ground.scale = Vector3.ONE * radius * 0.45
			if particles:
				_mesh_part(_quads(int(radius * radius * 30.0)), "debris", {motion = 4, shape = 6, size = 0.012,
					stretch = 6.0, period = 0.55, tint_a = Color(0.7, 0.78, 0.9, 0.35), tint_b = Color(0.8, 0.85, 0.95, 0.25),
					height = height - 0.3})
		"arc":
			_ends = _mesh_part(ArrayMesh.new(), "arc")
		"orb":
			_mesh_part(_quads(1), "orb")
	if particles:
		_sparks = _make_sparks()
		_attach(_sparks)
	if light:
		_light = OmniLight3D.new()
		_light.position = Vector3(0, 1.0, 0) if kind in STRIKES else Vector3.ZERO
		_light.omni_range = radius * 3.0 + (4.0 if kind in STRIKES else 1.5)
		_light.light_negative = style == "void"
		_light.shadow_enabled = false
		_attach(_light)
		_light_color()
	_place_ends()
	_shape()


func _build_wind() -> void:
	var s := _style()
	var w: Dictionary = WIND_KINDS[kind]
	var lines := _mesh_part(_ribbons(_wind_paths(roundi(w.lines * s.lines))), "line", _line_params())
	lines.sorting_offset = 0.5
	match kind:
		"swirl":
			_mesh_part(_disc(), "swirl").scale = Vector3(radius * 1.1, 1.0, radius * 1.1)
		"tornado":
			_mesh_part(_funnel(), "tornado").scale = Vector3(radius, height, radius)
			_mesh_part(_funnel(), "tornado", {layer = 1.0}).scale = Vector3(radius * 0.55, height * 0.95, radius * 0.55)
		"burst":
			_mesh_part(_disc(), "burst", {period = w.period}).scale = Vector3(radius * 1.15, 1.0, radius * 1.15)
	var motion: int = ["gust", "swirl", "tornado", "burst"].find(kind)
	var haze: float = s.haze
	if kind == "tornado":
		haze = maxf(haze, 0.5)
	if haze > 0.0:
		var puffs := {motion = motion if kind != "tornado" else 1, shape = 5, size = radius * (0.7 if kind == "gust" else 0.5),
			period = w.period * 1.6, flutter = 0.1}
		if kind == "tornado":
			puffs.radius = radius * 0.45
			puffs.height = 0.6
		_mesh_part(_quads(roundi((7 if kind == "gust" else 5) * haze) + 2), "debris", puffs)
	if particles and s.pieces > 0.0:
		_mesh_part(_quads(roundi(w.pieces * s.pieces)), "debris", {motion = motion, shape = s.shape,
			size = s.size, tint_a = s.tint_a, tint_b = s.tint_b, period = w.period, fall = s.fall,
			flutter = s.flutter, stretch = s.stretch})


func _line_params() -> Dictionary:
	match kind:
		"gust":
			return {period = 1.4, trail = 0.35, scatter = Vector3(radius * 0.25, height * 0.12, radius * 0.4)}
		"swirl":
			return {period = 1.3, trail = 0.45, scatter = Vector3(0, height * 0.05, 0)}
		"tornado":
			return {period = 1.5, trail = 0.3, scatter = Vector3(0, height * 0.04, 0)}
	return {period = WIND_KINDS.burst.period, trail = 0.5, burst = 1.0}


## A MeshInstance3D of `mesh` drawn with `shader`, attached, bounded wide enough for what its
## shader moves out of the mesh.
func _mesh_part(mesh: Mesh, shader: String, params := {}) -> MeshInstance3D:
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mi.material_override = _material(shader, params)
	var e := radius * 2.0 + 2.0
	if kind == "arc":
		for p in targets:
			e = maxf(e, p.length() + 1.0)
	mi.custom_aabb = AABB(Vector3(-e, -1.0, -e), Vector3(e * 2.0, maxf(height, e) + 3.0, e * 2.0))
	_attach(mi)
	return mi


func _attach(n: Node3D) -> void:
	add_child(n, false, Node.INTERNAL_MODE_BACK)
	_parts.append(n)


func _material(shader: String, params: Dictionary) -> ShaderMaterial:
	var m := ShaderMaterial.new()
	m.shader = load(SHADERS % shader)
	m.set_shader_parameter(&"color", color)
	m.set_shader_parameter(&"edge_color", edge_color)
	m.set_shader_parameter(&"seed", seed)
	m.set_shader_parameter(&"radius", radius)
	m.set_shader_parameter(&"height", height)
	m.set_shader_parameter(&"transparency", _transparency())
	for key in params:
		m.set_shader_parameter(key, params[key])
	for key in _params:
		m.set_shader_parameter(key, _params[key])
	_materials.append(m)
	return m


func _make_sparks() -> CPUParticles3D:
	var p := CPUParticles3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE * (0.07 if kind != "orb" else 0.1)
	p.mesh = quad
	p.material_override = _material("spark", {shape = 1.0 if kind == "orb" else 0.0})
	p.local_coords = true
	p.lifetime = 0.7
	p.direction = Vector3.UP
	p.gravity = Vector3(0, -9.0, 0)
	p.scale_amount_min = 0.5
	p.scale_amount_max = 1.2
	var fade := Gradient.new()
	fade.offsets = PackedFloat32Array([0.0, 0.1, 0.6, 1.0])
	fade.colors = PackedColorArray([Color(1, 1, 1, 0), Color.WHITE, Color.WHITE, Color(1, 1, 1, 0)])
	p.color_ramp = fade
	match kind:
		"arc":
			p.amount = 24
			p.emission_shape = CPUParticles3D.EMISSION_SHAPE_POINTS
			p.spread = 180.0
			p.initial_velocity_min = 0.8
			p.initial_velocity_max = 2.5
		"orb":
			p.amount = 20
			p.lifetime = 1.4
			p.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE_SURFACE
			p.emission_sphere_radius = radius * 0.3
			p.spread = 180.0
			p.initial_velocity_min = 0.2
			p.initial_velocity_max = 0.6
			p.gravity = Vector3.ZERO
		_:
			p.amount = 48 if kind == "nova" else 32
			p.one_shot = true
			p.explosiveness = 0.95
			p.emitting = false
			p.emission_shape = CPUParticles3D.EMISSION_SHAPE_SPHERE
			p.emission_sphere_radius = radius * (0.5 if kind == "nova" else 0.15)
			p.spread = 75.0
			p.initial_velocity_min = 2.0
			p.initial_velocity_max = 5.0
	return p


## An arc's flares and sparks on its points, its light between the first two.
func _place_ends() -> void:
	if kind != "arc" or not _ends:
		return
	var points := PackedVector3Array([Vector3.ZERO]) + targets
	_ends.mesh = _quads(points.size(), points)
	if _sparks:
		_sparks.emission_points = points
	if _light:
		_light.position = (points[0] + points[mini(1, points.size() - 1)]) * 0.5
	_shape()


## A new shape for the bolts.
func _shape() -> void:
	if not _bolt:
		return
	var w: float = 0.16
	var strands := []
	match kind:
		"strike", "cloud":
			var top := _impact + Vector3(_rng.randf_range(-0.3, 0.3) * radius, height, _rng.randf_range(-0.3, 0.3) * radius)
			if kind == "cloud":
				top.y = height - radius * 0.1
				set_param(&"flash_at", top)
			strands = _bolt_tree(top, _impact + Vector3(0, 0.02, 0), 6, 4, w, false)
		"nova":
			var n := 9
			for i in n:
				var a := (i + _rng.randf_range(-0.3, 0.3)) / n * TAU
				var end := Vector3(cos(a), 0.0, sin(a)) * radius * _rng.randf_range(0.75, 1.0) + Vector3(0, 0.04, 0)
				strands.append_array(_bolt_tree(Vector3(0, 0.12, 0), end, 5, 1, w * 0.75, true))
		"arc":
			var points := PackedVector3Array([Vector3.ZERO]) + targets
			for i in points.size() - 1:
				var a := points[i]
				var b := points[i + 1]
				var d := maxi(4, mini(6, int(a.distance_to(b)) + 3))
				strands.append_array(_bolt_tree(a, b, d, 1, w, false))
				strands.append({points = _jag(a, b, d, 0.3, false), from = 0.0, to = 1.0, width = w * 0.5, bright = 0.6, taper = 1.0})
		"orb":
			for i in 6:
				var dir := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(-1, 1), _rng.randf_range(-1, 1)).normalized()
				strands.append({points = _jag(Vector3.ZERO, dir * radius * _rng.randf_range(0.55, 0.95), 4, 0.3, false),
					from = 0.0, to = 1.0, width = w * 0.5, bright = 0.8, taper = 0.1})
	_bolt.mesh = _ribbons(strands)


## A bolt from a to b with `branches` forks, as strands for _ribbons().
func _bolt_tree(a: Vector3, b: Vector3, depth: int, branches: int, width: float, flat: bool) -> Array:
	var main := _jag(a, b, depth, 0.22, flat)
	var strands := [{points = main, from = 0.0, to = 1.0, width = width, bright = 1.0, taper = 0.6}]
	var length := a.distance_to(b)
	var way := (b - a) / length
	var n := main.size()
	for k in branches:
		var i := _rng.randi_range(int(n * 0.15), int(n * 0.7))
		var out := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(-1, 1), _rng.randf_range(-1, 1))
		out = (out - way * out.dot(way)).normalized()
		if flat:
			out.y *= 0.1
		var reach := length * _rng.randf_range(0.15, 0.35)
		var end := main[i] + (way * 0.7 + out * 0.6).normalized() * reach
		if flat:
			end.y = 0.04
		var from := float(i) / (n - 1)
		strands.append({points = _jag(main[i], end, depth - 2, 0.3, flat), from = from,
			to = minf(from + reach / length, 1.0), width = width * 0.55, bright = 0.7, taper = 0.15})
	return strands


## A jagged line from a to b: each segment's middle pushed off it `depth` times over, by up to
## `jag` of its length.
func _jag(a: Vector3, b: Vector3, depth: int, jag: float, flat: bool) -> PackedVector3Array:
	var pts := PackedVector3Array([a, b])
	for d in depth:
		var out := PackedVector3Array()
		for i in pts.size() - 1:
			var p := pts[i]
			var q := pts[i + 1]
			var way := (q - p).normalized()
			var off := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(-1, 1), _rng.randf_range(-1, 1))
			off -= way * off.dot(way)
			if flat:
				off.y *= 0.15
			var mid := (p + q) * 0.5 + off * p.distance_to(q) * jag
			if flat:
				mid.y = maxf(mid.y, 0.03)
			out.append(p)
			out.append(mid)
		out.append(pts[-1])
		pts = out
	return pts


## Wind lines' paths for the kind, as strands for _ribbons(), each with its phase as `bright`.
func _wind_paths(count: int) -> Array:
	var strands := []
	for i in count:
		var pts := PackedVector3Array()
		match kind:
			"gust":
				var x := _rng.randf_range(-1, 1) * radius
				var y := _rng.randf_range(0.12, 0.85) * height
				var l := radius * 1.6
				var waves := _rng.randf_range(0.6, 1.5)
				var amp := _rng.randf_range(0.05, 0.25)
				var ph := _rng.randf() * TAU
				var loop := _rng.randf() < 0.35
				var at := _rng.randf_range(0.35, 0.65)
				var ring := _rng.randf_range(0.18, 0.35)
				for j in 81:
					var f := j / 80.0
					var wob := sin(f * TAU * waves + ph) * amp
					var p := Vector3(x + wob * 0.6, y + wob * 0.5, l - 2.0 * l * f)
					if loop:
						var th := clampf((f - at) / 0.12, 0.0, 1.0) * TAU
						p += Vector3(0, ring * (1.0 - cos(th)), ring * sin(th))
					pts.append(p)
			"swirl", "tornado":
				var tall := kind == "tornado"
				var top := height * (_rng.randf_range(0.7, 1.0) if tall else _rng.randf_range(0.3, 0.6))
				var turns := _rng.randf_range(2.5, 3.5) if tall else _rng.randf_range(1.2, 2.0)
				var r0 := _rng.randf_range(0.4, 1.0)
				var a0 := _rng.randf() * TAU
				for j in 81:
					var f := j / 80.0
					var y := 0.1 + (top - 0.1) * f
					var r := radius * ((0.12 + 0.88 * pow(y / height, 1.5)) * 1.15 + 0.08) if tall else radius * r0 * (1.0 - 0.2 * f)
					var a := a0 + f * turns * TAU
					pts.append(Vector3(cos(a) * r, y, sin(a) * r))
			_:
				var a0 := (i + _rng.randf_range(-0.4, 0.4)) / count * TAU
				var y0 := _rng.randf_range(0.1, 0.6)
				var bend := _rng.randf_range(-0.3, 0.3)
				var far := radius * _rng.randf_range(0.9, 1.3)
				for j in 31:
					var f := j / 30.0
					var a := a0 + f * bend
					var r := lerpf(0.2, far, f)
					pts.append(Vector3(cos(a) * r, y0 + sin(f * PI) * 0.2, sin(a) * r))
		strands.append({points = pts, from = 0.0, to = 1.0, width = 0.035, bright = _rng.randf(), taper = 1.0})
	return strands


## Strands as camera-facing ribbons (bolt.gdshader, line.gdshader): per vertex NORMAL the strand's
## way, UV (from..to by length along it, -1 or 1 across), UV2 (half-width, bright), COLOR.r a
## random per strand.
func _ribbons(strands: Array) -> ArrayMesh:
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var colors := PackedColorArray()
	var idx := PackedInt32Array()
	for s: Dictionary in strands:
		var pts: PackedVector3Array = s.points
		var n := pts.size()
		var run := PackedFloat32Array([0.0])
		for i in range(1, n):
			run.append(run[i - 1] + pts[i].distance_to(pts[i - 1]))
		var total := maxf(run[n - 1], 1e-4)
		var own := Color(_rng.randf(), 0, 0)
		var base := verts.size()
		for i in n:
			var way := (pts[mini(i + 1, n - 1)] - pts[maxi(i - 1, 0)]).normalized()
			var f := run[i] / total
			var w: float = s.width * lerpf(1.0, s.taper, f)
			for side in [-1.0, 1.0]:
				verts.append(pts[i])
				normals.append(way)
				uvs.append(Vector2(lerpf(s.from, s.to, f), side))
				uv2s.append(Vector2(w, s.bright))
				colors.append(own)
		for i in n - 1:
			var k := base + i * 2
			idx.append_array([k, k + 1, k + 2, k + 1, k + 3, k + 2])
	var mesh := ArrayMesh.new()
	if idx.is_empty():
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = idx
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


## `count` quads for a shader to place and turn to the camera: corners all on the origin (or on
## `at`), UV the corner -1..1, UV2.x the quad's number, COLOR.r a random.
func _quads(count: int, at := PackedVector3Array()) -> ArrayMesh:
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var colors := PackedColorArray()
	var idx := PackedInt32Array()
	for i in count:
		var own := Color(_rng.randf(), 0, 0)
		for c in [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, 1)]:
			verts.append(at[i] if i < at.size() else Vector3.ZERO)
			uvs.append(c)
			uv2s.append(Vector2(i, 0))
			colors.append(own)
		idx.append_array([i * 4, i * 4 + 1, i * 4 + 2, i * 4, i * 4 + 2, i * 4 + 3])
	var mesh := ArrayMesh.new()
	if count == 0:
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2s
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = idx
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _disc() -> PlaneMesh:
	var plane := PlaneMesh.new()
	plane.size = Vector2(2.0, 2.0)
	return plane


## A funnel a unit high, narrow at its foot and a unit wide at its top.
func _funnel() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var seg := 40
	var rings := 16
	for j in rings + 1:
		var v := float(j) / rings
		var r := 0.1 + 0.9 * pow(v, 1.4)
		for i in seg + 1:
			var a := TAU * i / seg
			st.set_normal(Vector3(cos(a), -0.4, sin(a)).normalized())
			st.set_uv(Vector2(float(i) / seg, v))
			st.add_vertex(Vector3(cos(a) * r, v, sin(a) * r))
	for j in rings:
		for i in seg:
			var k := j * (seg + 1) + i
			for x in [k, k + 1, k + seg + 1, k + 1, k + seg + 2, k + seg + 1]:
				st.add_index(x)
	return st.commit()
