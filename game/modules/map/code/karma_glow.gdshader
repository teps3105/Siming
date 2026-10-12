// GDD 5.4 魂光：每位在世有名之人一點發光的圓。一個實例一個四邊形，由頂點著色器依螢幕像素展開。
// 資料在實例的變換矩陣裡：第 0 欄 (x、z、地表高)，第 1 欄 (淨業力，種類，亮度狀態)。
// 種類：0 魂光、1 選取環、2 前世所在地點（虛線圈）。亮度狀態：0 一般、1 強調、2 淡去。
shader_type spatial;
render_mode unshaded, cull_disabled, depth_test_disabled, depth_draw_never, blend_mix;

uniform vec3 gold = vec3(0.6, 0.4, 0.1);
uniform vec3 red = vec3(0.5, 0.05, 0.03);
uniform vec3 pale = vec3(0.9, 0.85, 0.7);
uniform float px_scale = 1.0;
uniform float base_px = 2.4;            // 淨業力為 0 的魂光核心半徑（像素，乘介面單位）
uniform float span_px = 4.2;            // 淨業力最大時再加的半徑
uniform float karma_ref = 120.0;        // 此淨業力絕對值以上算最大
uniform vec2 vanish_focus;
uniform vec2 vanish_toward;
uniform float vanish_eye = 0.0;
uniform vec2 map_size;
uniform float globe_morph = 0.0;
uniform float globe_radius = 1.0;
uniform float globe_relief = 1.0;
uniform float globe_top = 1.0;
uniform mat4 globe_xform;
instance uniform float copy_offset = 0.0;

const float GLOBE_COMPRESS = 2.0;
const float HALO = 2.6;                 // 光暈外緣是核心半徑的幾倍

float globe_lift(float h) {
	return globe_top * (1.0 - exp(-GLOBE_COMPRESS * h / max(globe_top, 1e-4))) / GLOBE_COMPRESS * globe_relief;
}

vec3 place(vec3 p, vec3 cam) {
	vec3 flat_pos = p;
	if (vanish_eye > p.y) {
		flat_pos.xz += p.y / (vanish_eye - p.y) * (p.xz - vanish_focus - vanish_toward * p.y);
	}
	vec3 world = flat_pos;
	if (globe_morph > 0.0) {
		float lon = (p.x - copy_offset) / map_size.x * TAU;
		float lat = (0.5 - p.z / map_size.y) * PI;
		vec3 dir = vec3(cos(lat) * cos(lon), sin(lat), -cos(lat) * sin(lon));
		float r = globe_radius + globe_lift(p.y);
		world = mix(flat_pos, (globe_xform * vec4(dir * r, 1.0)).xyz, globe_morph);
	}
	return mix(world, cam, 0.015);
}

varying vec2 v_corner;
varying flat float v_karma;
varying flat float v_kind;
varying flat float v_state;

void vertex() {
	// 第 0 欄 = (x, z, 地表高)
	vec3 centre = vec3(MODEL_MATRIX[0][0], MODEL_MATRIX[0][2], MODEL_MATRIX[0][1]);
	v_karma = MODEL_MATRIX[1][0];
	v_kind = MODEL_MATRIX[1][1];
	v_state = MODEL_MATRIX[1][2];
	vec3 p = place(vec3(centre.x + copy_offset, centre.y, centre.z), CAMERA_POSITION_WORLD);
	vec4 c = PROJECTION_MATRIX * VIEW_MATRIX * vec4(p, 1.0);
	float core = (base_px + span_px * sqrt(clamp(abs(v_karma) / karma_ref, 0.0, 1.0))) * px_scale;
	if (v_kind > 0.5) {
		core = 12.0 * px_scale;
	}
	float reach = v_kind > 0.5 ? core * 1.3 : core * HALO;
	v_corner = UV * 2.0 - 1.0;
	c.xy += v_corner * reach * 2.0 / VIEWPORT_SIZE * c.w;
	v_corner *= (reach / core);
	POSITION = c;
}

void fragment() {
	float r = length(v_corner);
	vec3 tint = pale;
	float strength = clamp(abs(v_karma) / 20.0, 0.0, 1.0);
	if (v_karma > 0.0) {
		tint = mix(pale, gold, strength);
	} else if (v_karma < 0.0) {
		tint = mix(pale, red, strength);
	}
	float a;
	vec3 col = tint;
	if (v_kind < 0.5) {
		// r 以核心半徑為 1：核心是實心圓，外面是光暈。
		float halo = exp(-r * r * 1.1) * 0.28;
		float disc = 1.0 - smoothstep(0.85, 1.0, r);
		col = mix(tint, vec3(1.0, 0.96, 0.86), 0.55 * (1.0 - smoothstep(0.0, 0.8, r)));
		a = max(halo, disc);
	} else if (v_kind < 1.5) {
		a = (1.0 - smoothstep(0.0, 0.09, abs(r - 0.85))) * 0.95;
		col = vec3(1.0, 0.95, 0.8);
	} else {
		float ang = atan(v_corner.y, v_corner.x);
		float dash = step(0.5, fract(ang / TAU * 12.0));
		a = (1.0 - smoothstep(0.0, 0.09, abs(r - 0.85))) * dash * 0.9;
		col = vec3(0.85, 0.85, 0.9);
	}
	float dim = v_state > 1.5 ? 0.22 : 1.0;
	ALBEDO = col;
	ALPHA = a * dim;
}
