shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, shadows_disabled, skip_vertex_transform;
// Lightning: a bolt's strands as ribbons turned to the camera. Each vertex has NORMAL the strand's
// way, UV (how far the bolt has to reach to get there, 0 to 1; -1 or 1 across), UV2 (half-width in
// metres, brightness) and COLOR.r the strand's own flicker.
#include "storm.gdshaderinc"

uniform float grow : hint_range(0.0, 1.0) = 1.0;   // how far the bolt has reached
uniform float thickness : hint_range(0.0, 4.0) = 1.0;

varying float bright;
varying float jitter;

void vertex() {
	bright = UV2.y;
	jitter = COLOR.r;
	VERTEX = storm_ribbon(MODELVIEW_MATRIX, VERTEX, NORMAL, UV.y, UV2.x * thickness);
	NORMAL = vec3(0.0, 0.0, 1.0);
}

void fragment() {
	float across = abs(UV.y);
	float core = smoothstep(0.3, 0.1, across);
	float halo = exp(-across * across * 7.0) * 0.55;
	float reach = UV.x;
	float shown = smoothstep(grow + 0.02, grow, reach);
	float tip = grow < 1.0 ? exp(-abs(grow - reach) * 25.0) * 0.8 : 0.0;
	float flick = 0.8 + 0.2 * sin(age * 70.0 + jitter * 40.0);
	float lit = max(flash * flick, tip) * shown;
	vec4 o = storm_color(core * min(lit, 1.0) * 1.2, (core + halo) * bright * lit, FRAGCOORD.xy);
	if (o.a < 0.0) {
		discard;
	}
	ALBEDO = o.rgb;
	ALPHA = o.a;
}
