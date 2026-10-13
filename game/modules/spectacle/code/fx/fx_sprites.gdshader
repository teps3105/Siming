shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never, shadows_disabled, skip_vertex_transform;
// 奇觀粒子（一般混合：煙、雲、火星、金屑）。共用部分見 fx_sprites.gdshaderinc。
#include "res://modules/spectacle/code/fx/fx_sprites.gdshaderinc"

void vertex() {
	vec3 v = sprite_vertex(VERTEX, VERTEX.xy, VERTEX_ID, MODEL_MATRIX, VIEW_MATRIX);
	v_k = v.x;
	v_tint = v.y;
	v_alpha = v.z;
}

void fragment() {
	vec4 t = texture(tex, UV);
	vec4 c = mix(color0, color1, v_k);
	c = mix(c, tint_b, tint_mix * v_tint);
	vec3 rgb = c.rgb * t.rgb + flash * flash_color * t.a;
	ALBEDO = rgb;
	ALPHA = clamp(t.a * c.a * v_alpha * master, 0.0, 1.0);
}
