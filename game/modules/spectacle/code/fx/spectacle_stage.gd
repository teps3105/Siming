class_name SpectacleStage
extends Node3D
## 一個奇觀在 3D 世界裡的舞台：節點的原點在事件所在格的地表、Y 軸向上、縮放等於奇觀半徑（世界單位），
## 所以子節點用「半徑為 1」的單位擺放；每幀由 Spectacle 依鏡頭換算位置與縮放後呼叫 tick。
## 時間完全由 age（奇觀已進行的秒數）決定，暫停時 age 不動，特效就停住。

var item := {}
var _systems: Array[MeshInstance3D] = []


## 建好這個奇觀所有的粒子與網格。
func setup(spec: Dictionary) -> void:
	item = spec


## age 是奇觀已進行的秒數；ctx 有 scale（半徑，世界單位）、u（介面單位）、px_world（一像素合多少世界單位）。
func tick(_age: float, _ctx: Dictionary) -> void:
	pass


func _add(mesh_instance: MeshInstance3D) -> MeshInstance3D:
	_systems.append(mesh_instance)
	return mesh_instance


## 設定同一個著色器參數給全部粒子。
func _all(key: String, value: Variant) -> void:
	for node in _systems:
		SpectacleKit.set_u(node, key, value)


## 區段內 0..1 的進度。
static func span(age: float, from: float, to: float) -> float:
	return clampf((age - from) / maxf(to - from, 1e-4), 0.0, 1.0)
