shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never;
// 地圖人物的棋子面片（8.11）：貼一張已畫好的 2D 人物（CultivatorFigure），不吃場景的光。
// 後仰的面片會被身後較高的地形蓋住，所以只改深度、不改畫面位置：沿視線把整片往鏡頭拉近 depth_pull（世界單位）。

uniform sampler2D card_texture : source_color, filter_linear;
uniform float depth_pull = 0.0;


void fragment() {
	vec4 c = texture(card_texture, UV);
	if (c.a < 0.01) {
		discard;
	}
	ALBEDO = c.rgb;
	ALPHA = c.a;
	// 深度只寫差值：先取引擎給的深度，再加上「沿視線往鏡頭拉近」造成的變化，與投影矩陣的深度方向約定無關。
	if (depth_pull > 0.0) {
		vec3 q = VERTEX;
		if (PROJECTION_MATRIX[3][3] > 0.5) {
			q.z += depth_pull;
		} else {
			q -= normalize(VERTEX) * depth_pull;
		}
		vec4 here = PROJECTION_MATRIX * vec4(VERTEX, 1.0);
		vec4 pulled = PROJECTION_MATRIX * vec4(q, 1.0);
		DEPTH = FRAGCOORD.z + (pulled.z / pulled.w - here.z / here.w);
	} else {
		DEPTH = FRAGCOORD.z;
	}
}
