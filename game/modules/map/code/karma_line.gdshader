// GDD 5.4 因果線：二次貝茲弧，由頂點著色器依螢幕像素展開成帶狀，線寬不隨縮放變粗。
// 一條線一個 MultiMesh 實例；資料藏在實例的變換矩陣裡（保有完整的 32 位元浮點，位置不失準）：
//   第 0 欄 (欠債方 x、z，債主 x)，第 1 欄 (債主 z，欠債方地表高、債主地表高)，
//   第 2 欄 (強度，種類 + 8 × 不在世的一端，亮度狀態)，第 3 欄未用。
// 網格是一條多段的帶：UV = (沿線比例 t, 左右 -1／+1)。
shader_type spatial;
render_mode unshaded, cull_disabled, depth_test_disabled, depth_draw_never, blend_mix;

uniform vec3 kind_colors[5];            // 殺業、恩義、情緣、師徒、怨（線性空間）
uniform vec3 casing_color = vec3(0.9);  // 線外一圈淡紙色的描邊，讓暗色的線在墨底上看得清
uniform float casing_alpha = 0.5;
uniform float px_scale = 1.0;           // 介面單位
uniform float flow_speed = 30.0;        // 流動光點每秒走的像素（乘介面單位）
uniform float line_alpha = 1.0;         // 整體不透明度
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
const float ARC_HEIGHT = 0.3;           // 弧高為兩點距離的 0.3 倍
const float ARC_RATIO = 1.204;          // 弧長 / 弦長（弧高 0.3 倍弦長的拋物線）
const float MIN_WIDTH = 1.0;
const float MAX_WIDTH = 4.0;
const float STRENGTH_CAP = 100.0;
const float FLOW_PERIOD = 40.0;

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

vec2 bezier(vec2 a, vec2 c, vec2 b, float t) {
	float u = 1.0 - t;
	return u * u * a + 2.0 * u * t * c + t * t * b;
}

varying float v_edge_px;
varying float v_half_px;
varying float v_s;
varying float v_t;
varying flat float v_kind;
varying flat float v_dead;
varying flat float v_state;

void vertex() {
	vec2 a = vec2(MODEL_MATRIX[0][0], MODEL_MATRIX[0][1]);
	vec2 b = vec2(MODEL_MATRIX[0][2], MODEL_MATRIX[1][0]);
	float ha = MODEL_MATRIX[1][1];
	float hb = MODEL_MATRIX[1][2];
	float strength = MODEL_MATRIX[2][0];
	float code = MODEL_MATRIX[2][1];
	v_state = MODEL_MATRIX[2][2];
	v_dead = floor(code / 8.0 + 0.01);
	v_kind = code - v_dead * 8.0;
	// 讓債主那端落在欠債方端點最近的東西副本。
	b.x += round((a.x - b.x) / map_size.x) * map_size.x;
	vec2 d = b - a;
	float len = length(d);
	vec2 nrm = len > 1e-4 ? vec2(-d.y, d.x) / len : vec2(0.0, 1.0);
	// 二次貝茲的控制點離弦中點 2 × 弧高，曲線的最高點才是弧高。
	vec2 c = 0.5 * (a + b) + nrm * (2.0 * ARC_HEIGHT * len);

	float t = UV.x;
	float t1 = min(t + 0.02, 1.0);
	float t0 = t1 - 0.02;
	vec3 cam = CAMERA_POSITION_WORLD;
	vec2 p = bezier(a, c, b, t);
	vec2 pa = bezier(a, c, b, t0);
	vec2 pb = bezier(a, c, b, t1);
	vec3 w0 = place(vec3(pa.x + copy_offset, mix(ha, hb, t0), pa.y), cam);
	vec3 w1 = place(vec3(pb.x + copy_offset, mix(ha, hb, t1), pb.y), cam);
	vec3 wp = place(vec3(p.x + copy_offset, mix(ha, hb, t), p.y), cam);
	mat4 vp = PROJECTION_MATRIX * VIEW_MATRIX;
	vec4 c0 = vp * vec4(w0, 1.0);
	vec4 c1 = vp * vec4(w1, 1.0);
	vec4 cp = vp * vec4(wp, 1.0);
	vec2 dir = (c1.xy / c1.w - c0.xy / c0.w) * VIEWPORT_SIZE;
	dir = length(dir) > 1e-6 ? normalize(dir) : vec2(1.0, 0.0);
	vec2 side = vec2(-dir.y, dir.x);

	float width = mix(MIN_WIDTH, MAX_WIDTH, clamp(strength / STRENGTH_CAP, 0.0, 1.0)) * px_scale;
	if (v_state > 0.5 && v_state < 1.5) {
		width += 1.0 * px_scale;
	}
	v_half_px = 0.5 * width;
	// 外加描邊 1 px 與抗鋸齒 1 px。
	float extent = v_half_px + 2.0 * px_scale;
	v_edge_px = UV.y * extent;
	// 沿線的像素位置：弦長的螢幕像素乘弧長比例。
	vec4 ca = vp * vec4(place(vec3(a.x + copy_offset, ha, a.y), cam), 1.0);
	vec4 cb = vp * vec4(place(vec3(b.x + copy_offset, hb, b.y), cam), 1.0);
	v_s = t * length((cb.xy / cb.w - ca.xy / ca.w) * VIEWPORT_SIZE * 0.5) * ARC_RATIO;
	v_t = t;
	cp.xy += side * UV.y * extent * 2.0 / VIEWPORT_SIZE * cp.w;
	POSITION = cp;
}

// 依種類的線型（不靠顏色的區別，5.1）：沿線的像素位置 x → 覆蓋率；grow 為描邊向兩端多撐出的像素。
float seg(float x, float lo, float hi, float grow) {
	return clamp(min(x - (lo - grow), (hi + grow) - x) + 0.5, 0.0, 1.0);
}

float pattern(float kind, float s, float grow) {
	if (kind < 0.5) {
		return 1.0;                                     // 殺業：實線
	} else if (kind < 1.5) {
		return seg(mod(s, 22.0), 0.0, 16.0, grow);       // 恩義：長劃
	} else if (kind < 2.5) {
		return seg(mod(s, 9.0), 0.0, 3.0, grow);         // 情緣：點
	} else if (kind < 3.5) {
		float x = mod(s, 28.0);
		return max(seg(x, 0.0, 12.0, grow), seg(x, 17.0, 20.0, grow)); // 師徒：劃加點
	}
	return seg(mod(s, 12.0), 0.0, 6.0, grow);           // 怨：短劃
}

void fragment() {
	float e = abs(v_edge_px) / px_scale;
	float half_w = v_half_px / px_scale;
	float core_cov = clamp(half_w - e + 0.5, 0.0, 1.0);
	float casing_cov = clamp(half_w + 1.0 - e + 0.5, 0.0, 1.0);
	float s = v_s / px_scale;
	float on = pattern(v_kind, s, 0.0);
	float on_casing = pattern(v_kind, s, 1.0);
	vec3 line = kind_colors[int(v_kind + 0.5)];
	// 流動光點：由欠債方（t = 0）流向債主，每 FLOW_PERIOD px 一粒。
	float phase = mod(s - TIME * flow_speed, FLOW_PERIOD);
	float dot_d = length(vec2(min(phase, FLOW_PERIOD - phase), e));
	float spark = clamp(half_w + 1.4 - dot_d + 0.5, 0.0, 1.0);
	// 不在世的一端：只畫靠活著那端的一半，末端淡出。
	float alpha = 1.0;
	if (v_dead > 0.5 && v_dead < 1.5) {
		alpha = clamp((v_t - 0.5) * 2.0, 0.0, 1.0);
	} else if (v_dead > 1.5) {
		alpha = clamp((0.5 - v_t) * 2.0, 0.0, 1.0);
	}
	float body = core_cov * on;
	float edge = casing_cov * on_casing;
	vec3 col = mix(casing_color, line, clamp(body / max(edge, 1e-3), 0.0, 1.0));
	col = mix(col, mix(line, vec3(1.0), 0.4), spark);
	float a = max(max(edge * casing_alpha, body), spark * 0.9);
	ALBEDO = col;
	ALPHA = a * alpha * line_alpha;
}
