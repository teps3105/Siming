shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, shadows_disabled, skip_vertex_transform;
// 飛升光柱：一條繞著鉛直軸朝向鏡頭的帶，橫向邊緣衰減、縱向有往上流的雜訊紋理，頂端淡出、底端收攏。
// 雜訊貼圖由程式以引擎內建的 FastNoiseLite 產生（可平鋪）。沿 UV 平移雜訊產生流動的做法，取自 gdquest
// godot-shaders 的 stylized_waterfall（MIT）的思路；程式為專案自寫。

uniform sampler2D noise : repeat_enable, filter_linear_mipmap;
uniform float age = 0.0;
uniform float height = 3.0;
uniform float width = 0.2;
uniform float rise : hint_range(0.0, 1.0) = 1.0;
uniform float master : hint_range(0.0, 2.0) = 1.0;
uniform float flow = 1.1;
uniform float noise_scale = 2.0;
uniform float edge_pow = 1.6;
uniform float top_width = 0.5;
uniform vec4 core_color : source_color = vec4(1.0, 0.97, 0.85, 1.0);
uniform vec4 edge_color : source_color = vec4(0.78, 0.55, 0.15, 1.0);
uniform float seed = 0.0;

varying vec2 puv;

void vertex() {
	float y = VERTEX.y;
	float x = VERTEX.x;
	float ms = length(MODEL_MATRIX[0].xyz);
	vec3 up = normalize((MODEL_MATRIX * vec4(0.0, 1.0, 0.0, 0.0)).xyz);
	vec3 view_dir = -normalize(INV_VIEW_MATRIX[2].xyz);
	vec3 side = normalize(cross(up, view_dir));
	float w = width * ms * mix(1.0, top_width, y) * (1.0 + 0.05 * sin(age * 5.0 + y * 6.0));
	vec3 world = MODEL_MATRIX[3].xyz + up * (y * height * rise * ms) + side * x * w;
	VERTEX = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	puv = vec2(x, y);
}

void fragment() {
	float u = puv.x;
	float v = puv.y;
	float edge = pow(max(1.0 - u * u, 0.0), edge_pow);
	float n1 = texture(noise, vec2(u * 0.6 * noise_scale + seed, v * noise_scale * 1.6 - age * flow)).r;
	float n2 = texture(noise, vec2(u * 0.9 * noise_scale + 0.37 - seed, v * noise_scale * 0.9 - age * flow * 1.8)).r;
	float streak = smoothstep(0.2, 0.85, mix(n1, n2, 0.5));
	float fade_top = 1.0 - smoothstep(0.5, 1.0, v);
	float fade_bot = smoothstep(0.0, 0.05, v);
	float body = edge * (0.4 + 0.9 * streak);
	float a = clamp(body, 0.0, 1.0) * fade_top * fade_bot * master;
	float core = pow(edge, 3.0) * (0.45 + 0.55 * streak);
	ALBEDO = mix(edge_color.rgb, core_color.rgb, clamp(core * 1.5, 0.0, 1.0));
	ALPHA = a;
}
