shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, shadows_disabled, skip_vertex_transform;
// 宿緣了結的線：靜態的帶狀網格（UV.x 是沿線 0..1，UV.y 是橫向 -1..1），弧線由 pa、pc、pb 三個世界座標的二次貝茲
// 在頂點著色器算出，朝向鏡頭的寬度也在此展開；從兩端向中點燒盡，燒過處留焦墨，燒處最亮。
// 帶狀網格轉向鏡頭的算法參考 Storm & Wind 3D Free 的 storm_ribbon（CC BY 4.0）。

uniform vec3 pa = vec3(0.0);
uniform vec3 pc = vec3(0.0);
uniform vec3 pb = vec3(0.0);
uniform float half_width = 1.0;
uniform float reach : hint_range(0.0, 0.5) = 0.0;
uniform float live : hint_range(0.0, 1.0) = 1.0;
uniform float char_alpha : hint_range(0.0, 1.0) = 0.5;
uniform vec4 kind_color : source_color = vec4(0.7, 0.2, 0.18, 1.0);
uniform vec4 pale : source_color = vec4(0.95, 0.92, 0.84, 1.0);
uniform vec4 ink : source_color = vec4(0.17, 0.15, 0.12, 1.0);
uniform vec4 hot : source_color = vec4(0.91, 0.81, 0.53, 1.0);

varying vec2 puv;

void vertex() {
	float f = UV.x;
	float g = 1.0 - f;
	vec3 p = g * g * pa + 2.0 * g * f * pc + f * f * pb;
	vec3 t = 2.0 * g * (pc - pa) + 2.0 * f * (pb - pc);
	vec3 pv = (VIEW_MATRIX * vec4(p, 1.0)).xyz;
	vec3 tv = (VIEW_MATRIX * vec4(t, 0.0)).xyz;
	vec3 across = cross(tv, vec3(0.0, 0.0, -1.0));
	float l = length(across);
	across = l > 1e-6 ? across / l : vec3(1.0, 0.0, 0.0);
	VERTEX = pv + across * UV.y * half_width;
	puv = UV;
}

void fragment() {
	float f = puv.x;
	float side = abs(puv.y);
	float prof = 1.0 - smoothstep(0.6, 1.0, side);
	bool burning = f >= reach && f <= 1.0 - reach;
	float d = min(abs(f - reach), abs(f - (1.0 - reach)));
	vec3 col;
	float a;
	if (burning) {
		vec3 base = mix(pale.rgb, kind_color.rgb, smoothstep(0.95, 0.4, side));
		float heat = exp(-d * 26.0) * live;
		col = mix(base, hot.rgb, heat);
		a = prof * live;
	} else {
		col = ink.rgb;
		a = prof * char_alpha;
	}
	ALBEDO = col;
	ALPHA = clamp(a, 0.0, 1.0);
}
