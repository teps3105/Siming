shader_type spatial;
render_mode unshaded, cull_disabled;
// 人物的賽璐璐水墨平塗與墨線（8.11「著色」）。同一支著色器畫兩道：
// outline_px = 0 是填色（只畫正面）：底色與陰影色兩階明暗，光從畫面左上前方固定照下來；
// outline_px > 0 是墨線（翻面殼，只畫背面）：頂點沿法線往外推，寬度以螢幕像素計。
// 頂點色 R 是擺動權重，以傳入的 age 算位移，不讀引擎時間；G 是緣邊遮罩，B 是腰帶遮罩。

uniform vec4 base_color : source_color = vec4(1.0);
uniform vec4 accent_color : source_color = vec4(1.0);
uniform vec4 sash_color : source_color = vec4(1.0);
uniform vec4 ink_color : source_color = vec4(0.1686, 0.1490, 0.1255, 1.0);
// 衣紋與配件之後由畫家畫成小貼圖；沒給時當白色。
uniform sampler2D albedo_tex : source_color, filter_linear, repeat_enable, hint_default_white;
// 整個部件只畫墨色（臉上的眉眼口）。
uniform bool flat_ink = false;
uniform float outline_px = 0.0;
// N·L 低於門檻的地方是陰影色；臉與皮膚的門檻放低，避免碎影。
uniform float shade_threshold = 0.0;
uniform vec3 shadow_tint = vec3(0.70, 0.72, 0.84);
// 固定光向（視圖空間：x 右、y 上、z 朝向鏡頭）：畫面左上前方。
uniform vec3 light_dir = vec3(-0.45, 0.55, 0.70);
uniform float age = 0.0;
uniform float sway_amount = 0.012;
// 沿視線往鏡頭拉近多少（世界單位）：人物後仰時不被後方的地形蓋住；身體各處拉近同樣的距離，自身的遮擋不變。
uniform float depth_pull = 0.0;


void vertex() {
	VERTEX += vec3(sin(age * 3.1 + VERTEX.y * 9.0 + VERTEX.x * 5.0),
			0.15 * sin(age * 2.3 + VERTEX.x * 7.0),
			cos(age * 2.6 + VERTEX.y * 8.0)) * sway_amount * COLOR.r;
	// 只要寫了 POSITION，引擎就不再算預設的位置，所以兩道都要自己算。
	vec4 clip = PROJECTION_MATRIX * (MODELVIEW_MATRIX * vec4(VERTEX, 1.0));
	if (outline_px > 0.0) {
		vec3 n = normalize(mat3(MODELVIEW_MATRIX) * NORMAL);
		vec2 dir = (PROJECTION_MATRIX * vec4(n, 0.0)).xy;
		if (length(dir) > 1e-5) {
			dir = normalize(dir);
		}
		clip.xy += dir * outline_px * 2.0 / VIEWPORT_SIZE * clip.w;
	}
	POSITION = clip;
}


void fragment() {
	if (outline_px > 0.0) {
		if (FRONT_FACING) {
			discard;
		}
		ALBEDO = ink_color.rgb;
	} else {
		if (!FRONT_FACING) {
			discard;
		}
		if (flat_ink) {
			ALBEDO = ink_color.rgb;
		} else {
			vec3 col = base_color.rgb;
			col = mix(col, accent_color.rgb, step(0.5, COLOR.g));
			col = mix(col, sash_color.rgb, step(0.5, COLOR.b));
			col *= texture(albedo_tex, UV).rgb;
			float lit = dot(normalize(NORMAL), normalize(light_dir));
			float edge = max(fwidth(lit), 0.02);
			float t = smoothstep(shade_threshold - edge, shade_threshold + edge, lit);
			ALBEDO = mix(col * shadow_tint, col, t);
		}
	}
	// 深度只寫差值：先取引擎給的深度，再加上「沿視線往鏡頭拉近」造成的變化，與投影矩陣的深度方向約定無關。
	if (depth_pull > 0.0) {
		vec3 q = VERTEX;
		if (PROJECTION_MATRIX[3][3] > 0.5) {
			q.z += depth_pull;
		} else {
			q -= normalize(VERTEX) * depth_pull;
		}
		vec4 here = PROJECTION_MATRIX * vec4(VERTEX, 1.0);
		vec4 pulled = PROJECTION_MATRIX * vec4(q, 1.0);
		DEPTH = FRAGCOORD.z + (pulled.z / pulled.w - here.z / here.w);
	} else {
		DEPTH = FRAGCOORD.z;
	}
}
