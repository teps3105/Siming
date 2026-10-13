class_name SpotStage
extends SpectacleStage
## 速度 >= 20× 時的簡化版：一個亮起再淡出的光點（Kenney 光暈貼圖，加法混合加一圈一般混合讓亮底上也看得見）。

var _glow: MeshInstance3D
var _core: MeshInstance3D
var _halo: MeshInstance3D


func setup(spec: Dictionary) -> void:
	super.setup(spec)
	var color := SpectacleKit.PALE
	match str(spec.get("of", "")):
		"Ascended":
			color = SpectacleKit.GOLD_LIGHT
		"ThreadResolved":
			color = KarmaLayer.KIND_COLORS[clampi(int(spec.get("thread_kind", 0)), 0, KarmaLayer.KIND_COLORS.size() - 1)].lerp(SpectacleKit.PALE, 0.4)
	var life := float(spec["duration"])
	var base := {"emit_span": 0.001, "life": life, "fade_in": 0.08, "fade_out_pow": 1.2,
		"vel": Vector3(0.0, 0.5, 0.0), "extent": Vector3.ZERO, "origin_off": Vector3(0.0, 0.45, 0.0)}
	_halo = _add(SpectacleKit.sprites(self, "circle_05", 1, base.merged({"size0": 1.2, "size1": 0.8,
		"color0": SpectacleKit.with_alpha(color, 0.5), "color1": SpectacleKit.with_alpha(color, 0.5)}), true))
	_glow = _add(SpectacleKit.sprites(self, "light_01", 1, base.merged({"size0": 1.4, "size1": 0.9,
		"color0": SpectacleKit.with_alpha(color, 1.0), "color1": SpectacleKit.with_alpha(color, 1.0)}), true))
	_core = _add(SpectacleKit.sprites(self, "circle_05", 1, base.merged({"size0": 0.34, "size1": 0.25,
		"color0": SpectacleKit.with_alpha(color.darkened(0.15), 0.95), "color1": SpectacleKit.with_alpha(color.darkened(0.15), 0.95)})))


func tick(age: float, _ctx: Dictionary) -> void:
	_all("age", age)
