shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, shadows_disabled;
// Lightning's particles: a quad turned to face the camera, drawn as a hot spark (shape 0) or a soft
// mote (1). Each particle's fade in and out comes in as COLOR.
#include "storm.gdshaderinc"

uniform float shape : hint_range(0.0, 1.0, 1.0) = 0.0;

void vertex() {
	MODELVIEW_MATRIX = VIEW_MATRIX * mat4(INV_VIEW_MATRIX[0], INV_VIEW_MATRIX[1], INV_VIEW_MATRIX[2], MODEL_MATRIX[3]);
	MODELVIEW_MATRIX = MODELVIEW_MATRIX * mat4(vec4(length(MODEL_MATRIX[0].xyz), 0.0, 0.0, 0.0),
			vec4(0.0, length(MODEL_MATRIX[1].xyz), 0.0, 0.0), vec4(0.0, 0.0, length(MODEL_MATRIX[2].xyz), 0.0),
			vec4(0.0, 0.0, 0.0, 1.0));
}

void fragment() {
	float r = length(UV * 2.0 - 1.0);
	float s = shape < 0.5 ? exp(-r * r * 10.0) + exp(-r * r * 40.0) : exp(-r * r * 5.0) * 0.8;
	vec4 o = storm_color(shape < 0.5 ? 1.0 - r : 0.4, s * COLOR.a * step(r, 1.0), FRAGCOORD.xy);
	if (o.a < 0.0) {
		discard;
	}
	ALBEDO = o.rgb;
	ALPHA = o.a;
}
