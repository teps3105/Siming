shader_type canvas_item;
// 部件庫的染色（GDD 8.12）：圖集的 RGB 已預乘 alpha；明度鍵的灰階由墨、陰影、底色、高光四個停點換色，
// 墨線維持 ink。is_fixed 的子層直接畫定色。之後套整張圖的色調（身故褪成淡墨、入魔偏紫）。
// 與 part_tint.gd 的 key_color／grade 是同一條公式（CPU 合成以它為準，測試與並排圖用）。
uniform vec4 ink_color;
uniform vec4 shadow_color;
uniform vec4 base_color;
uniform vec4 light_color;
uniform bool is_fixed = false;
uniform float desat = 0.0;
uniform vec4 wash_color;
uniform float wash_amount = 0.0;
uniform float fade = 0.0;
uniform vec4 paper_color;

void fragment() {
	vec4 t = texture(TEXTURE, UV);
	if (t.a < 0.001) {
		COLOR = vec4(0.0);
	} else {
		vec3 g = t.rgb / t.a;
		vec3 c = g;
		if (!is_fixed) {
			float p = clamp(g.r, 0.0, 1.0) * 3.0;
			if (p < 1.0) {
				c = mix(ink_color.rgb, shadow_color.rgb, p);
			} else if (p < 2.0) {
				c = mix(shadow_color.rgb, base_color.rgb, p - 1.0);
			} else {
				c = mix(base_color.rgb, light_color.rgb, p - 2.0);
			}
		}
		float lum = dot(c, vec3(0.299, 0.587, 0.114));
		c = mix(c, vec3(lum), desat);
		c = mix(c, wash_color.rgb, wash_amount);
		c = mix(c, paper_color.rgb, fade);
		COLOR = vec4(c, t.a);
	}
}
