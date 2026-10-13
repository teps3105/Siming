shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, shadows_disabled;
// Where a bolt meets the ground, on a disc: a pool of light under each stroke, rays of crackle, a
// 本檔經修改：焦痕（scorch）縮小並淡化，避免落雷處留下一個黑盤（專案用）。
// ring thrown out from the strike, and a scorch mark that outlasts it.
#include "storm.gdshaderinc"

uniform float scorch : hint_range(0.0, 1.0) = 0.0;

void fragment() {
	vec2 p = UV * 2.0 - 1.0;
	float r = length(p);
	float turn = atan(p.y, p.x) / TAU + 0.5;
	float since = age - fired;
	float pool = exp(-r * r * 10.0) * flash;
	float rays = pow(storm_noise(vec2(turn * 20.0, floor(fired * 13.0) + r * 3.0), 20.0), 4.0)
			* exp(-r * 2.2) * smoothstep(0.02, 0.1, r) * flash * 2.0;
	float k = clamp(since * 2.4, 0.0, 1.0);
	float ring = smoothstep(0.07, 0.0, abs(r - k * 0.95)) * (1.0 - k) * step(0.0, since) * 0.9;
	float glow = clamp(pool + rays + ring, 0.0, 1.0) * step(r, 1.0);
	vec3 lit = mix(edge_color.rgb, color.rgb, clamp(pool + rays * 0.5, 0.0, 1.0)) * energy;
	float burn = scorch * smoothstep(0.3, 0.08, r + (storm_fbm(p * 3.0, 0.0) - 0.5) * 0.4) * 0.4;
	float a = glow + burn * (1.0 - glow);
	vec3 c = (lit * glow + vec3(0.17, 0.13, 0.09) * burn * (1.0 - glow)) / max(a, 1e-4);
	a = storm_alpha(a, FRAGCOORD.xy);
	if (a < 0.0) {
		discard;
	}
	ALBEDO = c;
	ALPHA = a;
}
