shader_type canvas_item;
// 標題畫面山影的霧（GDD 9.1）：整幅畫面共用一片慢慢流動的霧場，每層山影照自己的遠近被它染成紙色——
// 越遠遮得越多（空氣遠近），山腰以下霧更重。霧不是一條條橫帶：濃淡由畫面位置的雜訊決定，各層看到的是同一片霧。

uniform float depth = 0.0;      // 這一層的遠近：0 最近、1 最遠
uniform float foot = 0.9;       // 這一層山腳線的高度（畫面高度的比例）
uniform float rise = 0.3;       // 這一層最高的山有多高（同上）
uniform float haze = 0.3;       // 最遠一層整體淡掉多少
uniform float density = 0.9;    // 霧團最濃處遮掉多少
uniform vec2 scale = vec2(2.2, 3.4); // 霧團的大小：畫面高度裡橫、直各有幾團
uniform float drift = 0.012;    // 霧流動的速度
uniform vec4 paper : source_color = vec4(0.95, 0.92, 0.85, 1.0); // 紙色：霧把山色往它推

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float value_noise(vec2 p) {
	vec2 cell = floor(p);
	vec2 t = fract(p);
	t = t * t * (3.0 - 2.0 * t);
	return mix(mix(hash(cell), hash(cell + vec2(1.0, 0.0)), t.x), mix(hash(cell + vec2(0.0, 1.0)), hash(cell + vec2(1.0, 1.0)), t.x), t.y);
}

float fbm(vec2 p) {
	float sum = 0.0;
	float weight = 0.5;
	for (int octave = 0; octave < 4; octave++) {
		sum += value_noise(p) * weight;
		// 每一階轉個角度再放大，雜訊的方格才不會疊成直的、橫的邊。
		p = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * p * 2.03 + vec2(17.0, 9.0);
		weight *= 0.5;
	}
	return sum / 0.9375;
}

void fragment() {
	float aspect = SCREEN_PIXEL_SIZE.y / SCREEN_PIXEL_SIZE.x;
	vec2 p = vec2(SCREEN_UV.x * aspect, SCREEN_UV.y) * scale;
	// 先把位置扭一下再取雜訊，霧團的邊才是捲的；遠近不同的層取的位置略錯開，霧有厚度。
	vec2 warp = vec2(fbm(p * 0.6 + vec2(7.3, TIME * drift * 2.0)), fbm(p * 0.6 + vec2(1.9 - TIME * drift * 1.5, 4.1)));
	float field = fbm(p + warp * 1.6 + vec2(TIME * drift * 6.0, 0.0) + depth * vec2(0.9, 0.5));
	float patches = smoothstep(0.36, 0.7, field);
	float low = smoothstep(foot - rise * 0.9, foot + 0.02, SCREEN_UV.y);
	float fog = haze * depth + patches * density * mix(0.35, 1.0, low);
	fog = clamp(fog, 0.0, 1.0);
	// 霧把山色染成紙色；山體仍不透明、蓋住紙紋，霧濃到快全白才淡出，免得留下一塊沒有紙紋的平色。
	COLOR.rgb = mix(COLOR.rgb, paper.rgb, fog);
	COLOR.a *= smoothstep(0.0, 0.3, 1.0 - fog);
}
