extends SceneTree
## 修士剪影對照表：把指定身分與隨機種子的修士畫在一張紙上存成 PNG，用來看演算法生成的變化（GDD 5.8）。
## 第一排是八仙那樣的男女老少富貴貧賤，第二排是儒、釋、道與幾個全由種子決定的人，各擺不同姿勢；第三排是同一個老者換六種鬍鬚樣式的放大臉。
## 用法：godot --path game -s res://tests/harness/figure_sheet.gd -- --out=<檔案>

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")
const COLUMNS := 8
const CELL := Vector2(143.0, 320.0)
## 第一排：男、女、老、少、富、貴、貧、賤。
const EIGHT := [
	{"sex": F.Sex.MALE, "age": F.Age.ADULT, "wealth": F.Wealth.PLAIN, "school": F.School.DAO, "prop": F.Prop.SWORD_BACK},
	{"sex": F.Sex.FEMALE, "age": F.Age.ADULT, "wealth": F.Wealth.PLAIN, "school": F.School.NONE, "prop": F.Prop.LOTUS},
	{"sex": F.Sex.MALE, "age": F.Age.ELDER, "wealth": F.Wealth.PLAIN, "school": F.School.DAO, "prop": F.Prop.WHISK},
	{"sex": F.Sex.MALE, "age": F.Age.CHILD, "wealth": F.Wealth.PLAIN, "school": F.School.NONE, "prop": F.Prop.FLUTE},
	{"sex": F.Sex.MALE, "age": F.Age.ADULT, "wealth": F.Wealth.RICH, "school": F.School.NONE, "prop": F.Prop.FAN},
	{"sex": F.Sex.MALE, "age": F.Age.ADULT, "wealth": F.Wealth.RICH, "school": F.School.RU, "prop": F.Prop.TABLET},
	{"sex": F.Sex.MALE, "age": F.Age.ELDER, "wealth": F.Wealth.POOR, "school": F.School.NONE, "prop": F.Prop.STAFF},
	{"sex": F.Sex.MALE, "age": F.Age.ADULT, "wealth": F.Wealth.POOR, "school": F.School.NONE, "prop": F.Prop.BASKET},
]
## 第二排：儒、釋（僧、尼）、道，其餘全由種子決定。
const SCHOOLS := [
	{"school": F.School.RU, "age": F.Age.ADULT, "sex": F.Sex.MALE}, {"school": F.School.SHI, "sex": F.Sex.MALE}, {"school": F.School.SHI, "sex": F.Sex.FEMALE},
	{"school": F.School.DAO, "sex": F.Sex.FEMALE}, {}, {}, {}, {},
]


class Sheet:
	extends Control

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Ink.PAPER, true)
		var poses := F.Pose.values()
		for row in 2:
			for column in COLUMNS:
				var index := row * COLUMNS + column
				var given: Dictionary = EIGHT[column] if row == 0 else SCHOOLS[column]
				var look := F.traits(100 + index, given)
				var pose: int = poses[0] if row == 0 else poses[column % poses.size()]
				var at := Vector2((column + 0.5) * CELL.x, (row + 1) * CELL.y - 40.0)
				F.draw(self, at, 215.0, -1.0, look, pose, index * 0.37, Color(Ink.INK, 0.94))
		# 第三排：放大看臉。六種鬍鬚樣式——老者束髮與壯年戴冠（冠纓垂在頷下）相間。
		var styles := F.Whiskers.values()
		for column in styles.size():
			var elder := column % 2 == 0
			var look := F.traits(11, {
				"sex": F.Sex.MALE, "age": F.Age.ELDER if elder else F.Age.ADULT, "hair": F.Hair.TOPKNOT if elder else F.Hair.CROWN,
				"prop": F.Prop.NONE, "beard": 0.2 if elder else 0.12, "whiskers": styles[column],
			})
			var at := Vector2((column + 0.5) * size.x / styles.size(), CELL.y * 2.0 + 700.0)
			F.draw(self, at, 640.0, -1.0, look, F.Pose.STAND, 0.4, Color(Ink.INK, 0.94))


func _initialize() -> void:
	var out := "user://figure_sheet.png"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
	# 畫在離屏的視口上，紙的大小不受視窗與螢幕限制。
	var paper := SubViewport.new()
	paper.size = Vector2i(int(CELL.x) * COLUMNS, int(CELL.y) * 3)
	paper.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(paper)
	var sheet := Sheet.new()
	sheet.size = Vector2(paper.size)
	paper.add_child(sheet)
	for i in 4:
		await process_frame
	paper.get_texture().get_image().save_png(out)
	quit(0)
