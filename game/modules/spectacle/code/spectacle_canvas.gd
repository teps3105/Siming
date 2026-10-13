class_name SpectacleCanvas
extends Control
## 奇觀的畫布：鋪滿畫面、不吃輸入，每次重畫請 Spectacle 把佇列畫上來。

var layer: Spectacle


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)


func _draw() -> void:
	if layer != null:
		layer.paint(self)
