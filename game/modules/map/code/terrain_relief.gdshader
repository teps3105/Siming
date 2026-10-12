// GDD 5.3 地形圖著色器、8.4 著色器規範、5.1 色票（色值由 uniform 傳入，線性空間）。
//
// 一切只依高度貼圖（相對海平面的高度，陸正海負）：
//   海岸線 = 高度為 0 的等值線；暈渲 = 高度場的逐像素坡向；設色 = 植被底色依高度漸入岩色與雪。
// 合成順序：1 陸地設色 → 2 暈渲 → 3 水（海深、湖）→ 4 海岸線 → 5 地圖模式疊色 → 6 州界
//           → 7 停留／選取 → 8 紙紋與霧 → 9 古地圖化。
// 河流不在此著色，見 river_line.gdshader。
shader_type spatial;
render_mode unshaded, cull_disabled, world_vertex_coords;

uniform vec3 paper;
uniform vec3 ink;
uniform vec3 ink_light;
uniform vec3 gold;
uniform vec3 cyan;
uniform vec3 coast_line;         // 海岸線色
uniform vec3 sea_colors[5];      // 海深色階（淺 → 深）
uniform vec3 sand_color;         // 沙土覆蓋的顏色
uniform vec3 rock_colors[4];     // 高度色階：丘陵土色 → 山地岩色 → 高山灰 → 雪
uniform vec3 shadow_tint = vec3(0.52, 0.58, 0.70); // 陰面乘色（偏花青）
uniform vec3 light_tint = vec3(1.10, 1.07, 0.98);  // 陽面乘色（偏暖）

uniform int map_mode = 0;             // 0 山河、1 天下、3 靈脈
uniform float hover_province = -1.0;
uniform float selected_province = -1.0;
uniform float tint_opacity = 0.0;     // 州郡疊色不透明度
// 高度的消失點（map_view.gd 的 lean）：鏡頭焦點、由焦點指向鏡頭的方向除以 tan(俯角)、眼高；眼高為 0 時不位移。
uniform vec2 vanish_focus;
uniform vec2 vanish_toward;
uniform float vanish_eye = 0.0;
uniform float border_opacity = 1.0;   // 州界與國界的不透明度
uniform vec2 map_size;                // 地圖寬、高（格）
uniform float sea_level = 0.4;        // 海平面（高度 0..1），用來正規化水深
uniform float max_elevation = 0.3;    // 陸地最高點的相對高度
uniform float field_opacity = 0.7;
uniform float fog_start = 1.0e6;
uniform float fog_end = 2.0e6;
uniform float globe_morph = 0.0;      // 0 平面、1 球面
uniform float globe_radius = 1.0;
uniform float globe_relief = 1.0;
uniform float globe_top = 1.0;         // 全圖最高的地表高度
uniform mat4 globe_xform;
uniform vec3 light_dir = vec3(-0.50, 0.62, -0.60); // 指向光源（西北上方）
uniform float emboss_strength = 1.0;     // 浮雕暈渲的強度
instance uniform float copy_offset = 0.0;

uniform sampler2D elevation_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D lake_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D climate_tex : repeat_enable, filter_linear, source_color;
uniform sampler2D cover_tex : repeat_enable, filter_linear;
// 天地演變的漸變（GDD 2.8「畫面」）：換圖後約 3 秒內，高度、湖面、植被底色與地表覆蓋都在前後兩張貼圖之間內插，
// 海岸、雪線與森林邊緣因此是移過去的。平時 terrain_blend 為 1，只取新的一張。
uniform sampler2D elevation_prev_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D lake_prev_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D climate_prev_tex : repeat_enable, filter_linear, source_color;
uniform sampler2D cover_prev_tex : repeat_enable, filter_linear;
uniform float terrain_blend = 1.0;
uniform sampler2D paper_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D water_tex : repeat_enable, filter_linear_mipmap;  // 可平鋪的雜訊：R 波紋、G 泡沫的形狀
uniform sampler2D field_tex : repeat_enable, filter_linear;
uniform sampler2D pid_tex : filter_nearest;   // RG8：州郡 ID 低、高位元組
uniform sampler2D pcol_tex : filter_nearest;  // 256×1：州郡 ID → 淡彩（sRGB）

const float NO_PROVINCE = 65535.0;
// 地形高度：相對高度 × 地圖寬 × 此係數（GDD 5.3）。
const float HEIGHT_GAIN = 0.04;

// 星球視角的高度壓縮（map_view.gd 的 globe_compress）：低地照原樣，越高壓得越多。
const float GLOBE_COMPRESS = 2.0;
float globe_lift(float h) {
	return globe_top * (1.0 - exp(-GLOBE_COMPRESS * h / max(globe_top, 1e-4))) / GLOBE_COMPRESS * globe_relief;
}

float elevation_lod(vec2 uv, float lod) {
	float now = textureLod(elevation_tex, uv, lod).r;
	if (terrain_blend < 1.0) {
		now = mix(textureLod(elevation_prev_tex, uv, lod).r, now, terrain_blend);
	}
	return now;
}

varying vec3 v_wpos;
varying vec2 v_lpos;
varying vec3 v_sdir;

void vertex() {
	float lon = UV.x * TAU;
	float lat = (0.5 - UV.y) * PI;
	vec3 dir = vec3(cos(lat) * cos(lon), sin(lat), -cos(lat) * sin(lon));
	// 邊疆帶（UV 夾在 0／1）高度為 0。
	float inside = step(0.0, VERTEX.z) * step(VERTEX.z, map_size.y);
	// 網格每個遊戲格四個頂點（頂點間距為 2 個紋素），取第 1.5 級的高度以免取樣不足拉出尖刺；
	// 更細的起伏交給著色的浮雕暈渲。
	float e = elevation_lod(UV, 1.5);
	float h1 = max(e, 0.0) * map_size.x * HEIGHT_GAIN * inside;
	vec3 wp = vec3(VERTEX.x, h1, VERTEX.z);
	if (vanish_eye > wp.y) {
		wp.xz += wp.y / (vanish_eye - wp.y) * (wp.xz - vanish_focus - vanish_toward * wp.y);
	}
	if (globe_morph > 0.0) {
		// 星球視角的山沿法線凸出球面：高度先壓縮再乘 globe_relief；球緣不收，山勢在輪廓上看得見。
		float r = globe_radius + globe_lift(h1);
		vec3 sph = (globe_xform * vec4(dir * r, 1.0)).xyz;
		wp = mix(wp, sph, globe_morph);
	}
	v_lpos = vec2(VERTEX.x - copy_offset, VERTEX.z);
	v_sdir = dir;
	v_wpos = wp;
	VERTEX = wp;
}

// 三次 B 樣條取樣（四次雙線性取樣）：放大時高度與其導數都連續，暈渲不出現紋素格。
vec4 bicubic(sampler2D tex, vec2 uv) {
	vec2 size = vec2(textureSize(tex, 0));
	vec2 st = uv * size - 0.5;
	vec2 base = floor(st);
	vec2 f = st - base;
	vec2 f2 = f * f;
	vec2 f3 = f2 * f;
	vec2 w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
	vec2 w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
	vec2 w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
	vec2 w3 = f3 / 6.0;
	vec2 g0 = w0 + w1;
	vec2 g1 = w2 + w3;
	vec2 h0 = (base - 0.5 + w1 / g0) / size;
	vec2 h1 = (base + 1.5 + w3 / g1) / size;
	return (textureLod(tex, vec2(h0.x, h0.y), 0.0) * g0.x + textureLod(tex, vec2(h1.x, h0.y), 0.0) * g1.x) * g0.y
		+ (textureLod(tex, vec2(h0.x, h1.y), 0.0) * g0.x + textureLod(tex, vec2(h1.x, h1.y), 0.0) * g1.x) * g1.y;
}

float elevation_cubic(vec2 uv) {
	float now = bicubic(elevation_tex, uv).r;
	if (terrain_blend < 1.0) {
		now = mix(bicubic(elevation_prev_tex, uv).r, now, terrain_blend);
	}
	return now;
}

float lake_cubic(vec2 uv) {
	float now = bicubic(lake_tex, uv).r;
	if (terrain_blend < 1.0) {
		now = mix(bicubic(lake_prev_tex, uv).r, now, terrain_blend);
	}
	return now;
}

// 三維梯度雜訊（球面座標求值，接縫與極點連續）。用於紋素以下的微細節。
vec3 hash33(ivec3 i) {
	uvec3 q = uvec3(i) * uvec3(1597334673u, 3812015801u, 2798796415u);
	q = (q.x ^ q.y ^ q.z) * uvec3(1597334673u, 3812015801u, 2798796415u);
	return vec3(q) / 4294967295.0 * 2.0 - 1.0;
}

float gnoise(vec3 p) {
	ivec3 i = ivec3(floor(p));
	vec3 f = fract(p);
	vec3 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
	float n000 = dot(hash33(i), f);
	float n100 = dot(hash33(i + ivec3(1, 0, 0)), f - vec3(1.0, 0.0, 0.0));
	float n010 = dot(hash33(i + ivec3(0, 1, 0)), f - vec3(0.0, 1.0, 0.0));
	float n110 = dot(hash33(i + ivec3(1, 1, 0)), f - vec3(1.0, 1.0, 0.0));
	float n001 = dot(hash33(i + ivec3(0, 0, 1)), f - vec3(0.0, 0.0, 1.0));
	float n101 = dot(hash33(i + ivec3(1, 0, 1)), f - vec3(1.0, 0.0, 1.0));
	float n011 = dot(hash33(i + ivec3(0, 1, 1)), f - vec3(0.0, 1.0, 1.0));
	float n111 = dot(hash33(i + ivec3(1, 1, 1)), f - vec3(1.0, 1.0, 1.0));
	return mix(mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y), mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y), u.z);
}

float pid_at(ivec2 t) {
	vec2 rg = texelFetch(pid_tex, t, 0).rg;
	return floor(rg.r * 255.0 + 0.5) + floor(rg.g * 255.0 + 0.5) * 256.0;
}

float bspline2(float d) {
	float a = abs(d);
	return a < 0.5 ? 0.75 - a * a : (a < 1.5 ? 0.5 * (1.5 - a) * (1.5 - a) : 0.0);
}

// 州郡投票：以二次 B 樣條權重對附近 3×3 格的州郡投票。
// 回傳 x = 第一名、y = 第二名、z = 兩者權重差（州界上為 0）。
vec3 vote(vec2 uv) {
	ivec2 sz = textureSize(pid_tex, 0);
	vec2 f = uv * vec2(sz) - 0.5;
	vec2 c = round(f);
	float ids[9];
	float wts[9];
	int n = 0;
	for (int j = -1; j <= 1; j++) {
		float wy = bspline2(f.y - (c.y + float(j)));
		int ty = clamp(int(c.y) + j, 0, sz.y - 1);
		for (int i = -1; i <= 1; i++) {
			float wx = bspline2(f.x - (c.x + float(i)));
			int tx = int(c.x) + i;
			tx = ((tx % sz.x) + sz.x) % sz.x;
			float id = pid_at(ivec2(tx, ty));
			if (id == NO_PROVINCE) {
				continue;
			}
			bool found = false;
			for (int k = 0; k < n; k++) {
				if (ids[k] == id) {
					wts[k] += wx * wy;
					found = true;
				}
			}
			if (!found) {
				ids[n] = id;
				wts[n] = wx * wy;
				n += 1;
			}
		}
	}
	float a = NO_PROVINCE;
	float b = NO_PROVINCE;
	float w1 = 0.0;
	float w2 = 0.0;
	for (int k = 0; k < n; k++) {
		if (wts[k] > w1) {
			w2 = w1;
			b = a;
			w1 = wts[k];
			a = ids[k];
		} else if (wts[k] > w2) {
			w2 = wts[k];
			b = ids[k];
		}
	}
	return vec3(a, b, w1 - w2);
}

vec4 pcol(float p) {
	vec4 c = texelFetch(pcol_tex, ivec2(int(min(p, 255.0)), 0), 0);
	return vec4(pow(c.rgb, vec3(2.2)), c.a);
}

// 以像素為單位的線段覆蓋率（d：到線中心的距離，fw：該量每像素的變化量，w：線寬像素）。
float px_line(float d, float fw, float w) {
	return clamp(0.5 * w - abs(d) / max(fw, 1e-6) + 0.5, 0.0, 1.0);
}

vec3 ramp5(vec3 colors[5], float t) {
	float x = clamp(t, 0.0, 1.0) * 4.0;
	int i = min(int(floor(x)), 3);
	return mix(colors[i], colors[i + 1], x - float(i));
}

// 水面的波紋與泡沫：移植自 binbun「Godot Water」卡通版（CC0，8.10）的 sample_wave 與泡沫算法。
// 原作以深度貼圖取水深、以獨立的水面網格打光；這裡水深取高度貼圖，顏色接在海深色階上，不打光。
const float WATER_TILE_PX = 230.0;                // 一張波紋貼圖在畫面上約佔的寬度（像素）到兩倍
const vec2 WAVE_VELOCITY = vec2(0.012, 0.007);    // 每秒流過幾張貼圖
const float WAVE_LAYER_SCALE = 1.5;               // 反向那一層的縮放
const vec2 WAVE_STRETCH = vec2(1.0, 2.0);  // 波線橫向拉長
const float WAVE_HIGHLIGHT = 0.26;                // 波光混入淺水色的比例
const float WAVE_FOAM_AMOUNT = 0.50;              // 浪頭算進泡沫的比例
const float FOAM_START = 0.15;
const float FOAM_END = 0.30;
const float FOAM_EXPONENT = 2.0;
const float EDGE_FOAM_DEPTH = 0.007;              // 岸邊泡沫帶的水深（高度單位）

// 兩層波紋反向流動，第一層擾動第二層的取樣位置。p 以一張貼圖的寬為 1；回傳 x 波紋、y 泡沫的形狀。
vec2 sample_wave(vec2 p, float lod) {
	vec2 drift = TIME * WAVE_VELOCITY;
	vec2 q = p * WAVE_STRETCH;
	float wave1 = textureLod(water_tex, q * WAVE_LAYER_SCALE - drift, lod).r;
	float wave2 = textureLod(water_tex, q + drift - wave1 * 0.1, lod).r;
	return vec2(wave2, textureLod(water_tex, p * 2.0, lod + 1.0).g);
}

// 波紋畫在地圖上，貼圖的寬取地圖寬的 2 的次方分之一（東西接縫才接得上），
// 隨縮放在相鄰兩級之間淡入淡出，畫面上的波紋大小大致不變。
vec2 water_surface(vec2 wp, float pxw) {
	float n = max(log2(map_size.x / (pxw * WATER_TILE_PX)), 1.0);
	float fine = map_size.x / exp2(ceil(n));
	float texels = float(textureSize(water_tex, 0).x);
	vec2 near = sample_wave(wp / fine, max(log2(pxw / fine * texels), 0.0));
	vec2 far = sample_wave(wp / (fine * 2.0), max(log2(pxw / fine * texels) - 1.0, 0.0));
	return mix(near, far, ceil(n) - n);
}

vec3 field_ramp(float t) {
	float c = clamp(t, 0.0, 1.0);
	return c < 0.5 ? mix(paper, cyan, c * 2.0) : mix(cyan, gold, (c - 0.5) * 2.0);
}

void fragment() {
	vec2 wp = v_lpos;
	vec2 uv = vec2(wp.x / map_size.x, clamp(wp.y / map_size.y, 0.0, 1.0));
	vec3 d = normalize(v_sdir);

	// 每像素涵蓋幾個遊戲格（東西接縫兩側差一整圈，取週期內最短差）。
	vec2 dwx = dFdx(wp);
	vec2 dwy = dFdy(wp);
	dwx.x -= round(dwx.x / map_size.x) * map_size.x;
	dwy.x -= round(dwy.x / map_size.x) * map_size.x;
	float pxw = max(length(abs(dwx) + abs(dwy)), 1e-4);

	// 高度：貼圖（三次 B 樣條）加上紋素以下的微細節。微細節的振幅隨像素涵蓋範圍淡出，遠景不閃爍。
	vec2 texel = 1.0 / vec2(textureSize(elevation_tex, 0));
	float cells_per_texel = map_size.x * texel.x;
	float e_c = elevation_cubic(uv);
	float e_xp = elevation_cubic(uv + vec2(texel.x, 0.0));
	float e_xm = elevation_cubic(uv - vec2(texel.x, 0.0));
	float e_yp = elevation_cubic(uv + vec2(0.0, texel.y));
	float e_ym = elevation_cubic(uv - vec2(0.0, texel.y));
	// 細坡向（稜線、谷線）與曲率：每遊戲格的高度變化。
	vec2 grad = vec2(e_xp - e_xm, e_yp - e_ym) / (2.0 * cells_per_texel);
	float curvature = (e_xp + e_xm + e_yp + e_ym - 4.0 * e_c) / (cells_per_texel * cells_per_texel);
	// 粗坡向（整座山的受光面與背光面）：跨 6 個紋素取差。
	vec2 wide = texel * 6.0;
	vec2 grad_wide = vec2(
		elevation_lod(uv + vec2(wide.x, 0.0), 2.0) - elevation_lod(uv - vec2(wide.x, 0.0), 2.0),
		elevation_lod(uv + vec2(0.0, wide.y), 2.0) - elevation_lod(uv - vec2(0.0, wide.y), 2.0)
	) / (12.0 * cells_per_texel);
	float slope = length(grad);
	vec3 p3 = d * (map_size.x / TAU);
	float micro_fade = 1.0 - smoothstep(0.04, 0.16, pxw);
	float micro = (gnoise(p3 * 5.0) * 0.6 + gnoise(p3 * 11.0 + 7.3) * 0.4) * micro_fade;
	float e = e_c + micro * (slope * 0.09 + 0.00008);

	// 水：海在等值線以下；湖以遮罩（128 為湖岸）。
	float fw_e = max(fwidth(e), 1e-7);
	float sea_mask = 1.0 - smoothstep(-0.5 * fw_e, 0.5 * fw_e, e);
	float lake_v = lake_cubic(uv) + micro * 0.03;
	float fw_l = max(fwidth(lake_v), 1e-5);
	float lake_mask = smoothstep(0.5 - 0.5 * fw_l, 0.5 + 0.5 * fw_l, lake_v);
	float wmask = max(sea_mask, lake_mask);

	// 1 陸地設色：植被底色依高度漸入土色、岩色與雪；雪線隨緯度降低；終年積雪處為冰原，乾熱裸地為沙漠。
	vec3 veg = texture(climate_tex, uv).rgb;
	vec2 cover = texture(cover_tex, uv).rg;
	if (terrain_blend < 1.0) {
		veg = mix(texture(climate_prev_tex, uv).rgb, veg, terrain_blend);
		cover = mix(texture(cover_prev_tex, uv).rg, cover, terrain_blend);
	}
	// 地表覆蓋：R 冰雪、G 沙土。沙土蓋在植被底色上（高處照常漸入岩色），冰雪蓋在高度色階上。
	veg = mix(veg, sand_color, cover.g * 0.9);
	float alt = clamp(max(e, 0.0) / max(max_elevation, 1e-4) + 0.28 * d.y * d.y, 0.0, 1.0);
	// 紙墨設色：植被底色只是薄薄一層淡彩，平地近乎紙色，山才上青綠。
	vec3 land = mix(paper, veg, mix(0.5, 0.85, smoothstep(0.10, 0.45, alt)));
	land = mix(land, rock_colors[0], smoothstep(0.10, 0.34, alt) * 0.55);
	land = mix(land, rock_colors[1], smoothstep(0.30, 0.55, alt) * 0.8);
	land = mix(land, rock_colors[2], smoothstep(0.52, 0.74, alt) * 0.9);
	land = mix(land, rock_colors[3], smoothstep(0.76, 0.94, alt) * 0.8);
	// 冰原：終年積雪的地方（雪原與同樣寒冷的山地）不分高低都覆上雪色，起伏交給暈渲。
	land = mix(land, rock_colors[3], cover.r * 0.92);
	// 陡坡露岩：坡度大的地方植被少。
	land = mix(land, rock_colors[1], smoothstep(0.010, 0.035, slope) * 0.35 * (1.0 - smoothstep(0.72, 0.90, alt)));

	// 2 浮雕暈渲：粗坡向給整座山受光面與背光面，細坡向刻出稜與谷；山越高立體感越強，
	// 平原只留粗坡向，不出現細碎紋理。稜線提亮、谷線壓暗（曲率），像壓印出來的地形。
	// 谷線與背光的陡坡再提一筆墨，像勾出來的山形與皴。
	float mountain = smoothstep(0.10, 0.50, alt);
	float relief = map_size.x * HEIGHT_GAIN * emboss_strength;
	vec3 ldir = normalize(light_dir);
	float fine_gain = relief * mix(0.35, 1.5, mountain);
	float wide_gain = relief * mix(1.3, 2.2, mountain);
	vec3 nrm_fine = normalize(vec3(-grad.x * fine_gain, 1.0, -grad.y * fine_gain));
	vec3 nrm_wide = normalize(vec3(-grad_wide.x * wide_gain, 1.0, -grad_wide.y * wide_gain));
	float shade = (dot(nrm_wide, ldir) - ldir.y) * 1.25 + (dot(nrm_fine, ldir) - ldir.y) * mix(0.30, 0.95, mountain);
	float crease = clamp(curvature * relief * 0.4, -1.0, 1.0) * mountain; // 正為谷、負為稜
	land = mix(land, land * shadow_tint, clamp(-shade * 1.5, 0.0, 0.82));
	land = mix(land, min(land * light_tint + vec3(0.03), vec3(1.0)), clamp(shade * 1.2, 0.0, 0.45));
	land = mix(land, land * shadow_tint, clamp(crease, 0.0, 1.0) * 0.35);
	land = mix(land, min(land * light_tint + vec3(0.06), vec3(1.0)), clamp(-crease, 0.0, 1.0) * 0.25);
	land = mix(land, ink, clamp(crease, 0.0, 1.0) * 0.22 + smoothstep(0.35, 1.0, -shade) * mountain * 0.14);

	// 3 水：水深取平滑後的高度（第 5 級，約 4 個遊戲格的平均），海色隨水深連續變化，沒有分段。
	// 岸邊由實際高度接手，海岸線兩側的深淺才連續。
	float e_smooth = elevation_lod(uv, 5.0);
	float e_water = mix(e, min(e_smooth, e), smoothstep(0.0, 0.012, -e));
	float depth = clamp(-e_water / max(sea_level, 1e-3), 0.0, 1.0);
	float dither = (fract(sin(dot(FRAGCOORD.xy, vec2(12.9898, 78.233))) * 43758.5453) - 0.5) / 255.0;
	vec3 sea = ramp5(sea_colors, 1.0 - exp(-depth * 3.2) + dither);
	vec3 lake = mix(sea_colors[0], sea_colors[1], 0.6);
	float is_lake = lake_mask * (1.0 - sea_mask);
	vec3 water = mix(sea, lake, is_lake);
	// 水面：波光把水色帶向淺水色，外海減半；泡沫是岸邊一圈加上近岸的浪頭，再由雜訊切出形狀，取紙色。
	float off_shore = smoothstep(0.03, 0.35, depth) * (1.0 - is_lake);
	vec2 surface = water_surface(wp, pxw);
	float wave = smoothstep(0.0, 1.0, surface.x);
	water = mix(water, sea_colors[0], wave * WAVE_HIGHLIGHT * mix(1.0, 0.35, off_shore));
	float edge_foam = mix(clamp(1.0 + e / EDGE_FOAM_DEPTH, 0.0, 1.0), 1.0 - smoothstep(0.5, 0.56, lake_v), is_lake);
	float near_shore = (1.0 - smoothstep(0.0, 0.10, depth)) * (1.0 - is_lake);
	float foam = max(edge_foam, wave * WAVE_FOAM_AMOUNT * near_shore);
	float foam_shape = 1.0 - surface.y;
	foam = clamp((foam - FOAM_START) / (FOAM_END - FOAM_START), 0.0, 1.0);
	foam = clamp((foam - foam_shape) / max(1.0 - foam_shape, 1e-3), 0.0, 1.0);
	foam = pow(foam, FOAM_EXPONENT);
	// 岸邊留一圈近乎紙色的淺水。
	float shallows = mix(1.0 - smoothstep(0.0, 0.008, -e), 1.0 - smoothstep(0.5, 0.8, lake_v), is_lake);
	water = mix(water, paper, shallows * 0.5);
	water = mix(water, paper, foam * 0.8);

	vec3 col = mix(land, water, wmask);

	// 4 岸線：海岸與湖岸都勾一道墨線。
	float coast = px_line(e, fw_e, 1.5) * (1.0 - lake_mask);
	float lake_coast = px_line(lake_v - 0.5, fw_l, 1.5) * (1.0 - sea_mask);
	col = mix(col, coast_line, max(coast, lake_coast) * 0.9);

	// 5 地圖模式疊色
	vec3 vt = vote(uv);
	float pid = vt.x;
	float fw_m = 1.5 * pxw;
	float marg = vt.z;
	float is_land_px = (1.0 - wmask) * (pid != NO_PROVINCE ? 1.0 : 0.0);
	vec4 pc = pcol(pid);
	float tint = pc.a * tint_opacity * is_land_px;
	col = mix(col, vec3(dot(col, vec3(0.2126, 0.7152, 0.0722))), 0.4 * tint);
	col = mix(col, col * pc.rgb * 1.3, tint);
	if (map_mode == 3) {
		float fv = texture(field_tex, uv).r;
		col = mix(col, field_ramp(fv) * mix(0.75, 1.0, clamp(0.5 + shade, 0.0, 1.0)), field_opacity * (1.0 - wmask));
	}

	// 6 州界：ink-light 1 px；國界 ink 2 px 加外側淡暈。墨線在 62°–78° 先行淡出。
	bool has_b = vt.y != NO_PROVINCE;
	vec4 pcb = pcol(vt.y);
	bool realm_diff = has_b && (pc.rgb != pcb.rgb || pc.a != pcb.a);
	float land_f = (1.0 - wmask) * (1.0 - smoothstep(0.883, 0.978, abs(d.y)));
	float border_f = land_f * border_opacity;
	if (realm_diff) {
		col = mix(col, ink_light, 0.16 * px_line(marg, fw_m, 6.0) * border_f);
	}
	col = mix(col, ink_light, 0.7 * px_line(marg, fw_m, 1.1) * border_f);
	if (realm_diff) {
		col = mix(col, ink, 0.9 * px_line(marg, fw_m, 2.2) * border_f);
	}

	// 7 停留／選取
	float hot_cov = px_line(marg, fw_m, 2.6);
	bool hov_hit = hover_province >= 0.0 && (vt.x == hover_province || vt.y == hover_province);
	bool sel_hit = selected_province >= 0.0 && (vt.x == selected_province || vt.y == selected_province);
	if (pid != NO_PROVINCE && pid == hover_province) {
		col = mix(col, paper + vec3(0.1), 0.18 * land_f);
	}
	if (pid != NO_PROVINCE && pid == selected_province) {
		col = mix(col, gold, 0.22 * land_f);
	}
	if (has_b && hov_hit) {
		col = mix(col, ink, 0.75 * hot_cov * land_f);
	}
	if (has_b && sel_hit) {
		col = mix(col, gold, 0.95 * hot_cov * land_f);
	}

	// 8 紙紋（極淡，只給印在紙上的質感）與距離霧
	vec3 pap = texture(paper_tex, wp / 22.0).rgb / 0.96;
	col *= mix(vec3(1.0), pap, 0.5);
	float dist = length(CAMERA_POSITION_WORLD - v_wpos);
	col = mix(col, paper, smoothstep(fog_start, fog_end, dist) * 0.85);

	// 9 南北緣之外的邊疆帶淡成紙色。
	float out_y = max(-wp.y, wp.y - map_size.y);
	float fade_y = smoothstep(-2.0, 0.05 * map_size.x, out_y) * (1.0 - globe_morph);
	col = mix(col, paper, fade_y);

	ALBEDO = col;
}
