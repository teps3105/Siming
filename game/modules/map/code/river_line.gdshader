// GDD 5.3「河流」：製圖線。帶狀網格在頂點著色器依螢幕像素向兩側展開，
// 因此線寬不隨縮放變粗；各等級的不透明度由鏡頭高度決定（map_view.gd）。
shader_type spatial;
render_mode unshaded, cull_disabled, world_vertex_coords, blend_mix, depth_draw_never;

uniform vec3 line_color;
// 高度的消失點，同 terrain_relief.gdshader。
uniform vec2 vanish_focus;
uniform vec2 vanish_toward;
uniform float vanish_eye = 0.0;
uniform vec3 level_width_px = vec3(1.0, 1.5, 2.0); // 小河、大河、江
uniform vec3 level_alpha = vec3(1.0);              // 小河、大河、江
uniform vec2 map_size;
// 當佈景用時的雲霧（同 terrain_relief.gdshader）：河線在霧裡淡掉。
uniform float mist = 0.0;
uniform float mist_veil = 0.0;
uniform float globe_morph = 0.0;
uniform float globe_radius = 1.0;
uniform float globe_relief = 1.0;
uniform float globe_top = 1.0;
uniform mat4 globe_xform;
instance uniform float copy_offset = 0.0;

// 星球視角的高度壓縮（map_view.gd 的 globe_compress）：低地照原樣，越高壓得越多。
const float GLOBE_COMPRESS = 2.0;
float globe_lift(float h) {
	return globe_top * (1.0 - exp(-GLOBE_COMPRESS * h / max(globe_top, 1e-4))) / GLOBE_COMPRESS * globe_relief;
}

varying float v_edge_px;
varying float v_half_px;
varying float v_alpha;

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
	// 朝鏡頭拉近 1.5%，避免與地形深度相爭；拉近量與距離成正比，遠近都有效。
	return mix(world, cam, 0.015);
}

void vertex() {
	// NORMAL = 沿河切線、UV = (side, 等級)、UV2.x = 收細係數。
	vec3 p0 = place(VERTEX, CAMERA_POSITION_WORLD);
	vec3 p1 = place(VERTEX + NORMAL * 0.25, CAMERA_POSITION_WORLD);
	mat4 vp = PROJECTION_MATRIX * VIEW_MATRIX;
	vec4 c0 = vp * vec4(p0, 1.0);
	vec4 c1 = vp * vec4(p1, 1.0);
	vec2 s0 = c0.xy / c0.w;
	vec2 s1 = c1.xy / c1.w;
	vec2 dir = (s1 - s0) * VIEWPORT_SIZE;
	dir = length(dir) > 1e-6 ? normalize(dir) : vec2(1.0, 0.0);
	vec2 nrm = vec2(-dir.y, dir.x);

	float level = UV.y;
	float width = level < 1.5 ? level_width_px.x : (level < 2.5 ? level_width_px.y : level_width_px.z);
	v_alpha = level < 1.5 ? level_alpha.x : (level < 2.5 ? level_alpha.y : level_alpha.z);
	v_half_px = 0.5 * width * UV2.x;
	// 多展開 1 px 供抗鋸齒。
	float extent = v_half_px + 1.0;
	v_edge_px = UV.x * extent;
	c0.xy += nrm * UV.x * extent * 2.0 / VIEWPORT_SIZE * c0.w;
	POSITION = c0;
}

void fragment() {
	float coverage = clamp(v_half_px - abs(v_edge_px) + 0.5, 0.0, 1.0);
	ALBEDO = line_color;
	float veil = (1.0 - smoothstep(0.08, 0.52, SCREEN_UV.x)) * mist_veil;
	ALPHA = coverage * v_alpha * mix(1.0, 0.45, mist) * (1.0 - veil);
}
