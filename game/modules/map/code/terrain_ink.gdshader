// GDD 5.3 水墨著色器（地形）、8.4 著色器規範、5.1 色票（色值全由 uniform 傳入，線性空間）。
//
// 合成順序（5.3 步驟）：1 底色×紙紋 → 4 水（海深、海岸雙線）→ 5 地圖模式疊色 / 連續場
// → 2 上墨（坡度/高度 × 皴法）→ 3 山脊線 → 6 邊界 → 7 霧 → 8 古地圖化（ink 權重隨高度縮放降為 0）。
// 上墨置於疊色之後，使墨色不被淡彩蓋掉。河流不在此著色，見 river_line.gdshader。
shader_type spatial;
render_mode unshaded, cull_disabled, world_vertex_coords;

uniform vec3 paper;
uniform vec3 ink;
uniform vec3 ink_light;
uniform vec3 water;
uniform vec3 gold;
uniform vec3 cyan;
uniform vec3 terrain_colors[9];
uniform vec3 sea_colors[5];      // 海深色階（淺 → 深）
uniform vec3 mountain_colors[5]; // 山地高度色階（低 → 高）

uniform float height_scale = 1.0;
uniform float flatten = 0.0;          // 古地圖化權重
uniform int map_mode = 0;             // 0 山河、1 天下、3 靈脈
uniform float hover_province = -1.0;
uniform float selected_province = -1.0;
uniform float tint_opacity = 0.0;     // 州郡疊色不透明度
uniform vec2 map_size;                // 地圖寬、高（格）
uniform float max_height = 1.0;       // 最高點（高度縮放 = 1）
uniform float field_opacity = 0.7;    // 連續場不透明度
uniform float fog_start = 1.0e6;
uniform float fog_end = 2.0e6;
uniform float globe_morph = 0.0;      // 0 平面、1 球面
uniform float globe_radius = 1.0;
uniform float globe_relief = 2.0;     // 球面高度位移倍率
uniform mat4 globe_xform;             // 球面局部座標 → 世界座標
uniform vec4 light = vec4(-0.56, 0.66, -0.47, 1.0); // xyz 指向光源、w 山影強度
instance uniform float copy_offset = 0.0;           // 本副本的東西位移（格）

uniform sampler2D paper_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D stroke_tex : repeat_enable, filter_linear_mipmap;
uniform sampler2D terrain_tex : repeat_enable, filter_linear;
uniform sampler2D field_tex : repeat_enable, filter_linear;
uniform sampler2D cls_a_tex : repeat_enable, filter_linear; // 平原、丘陵、山脈、森林
uniform sampler2D cls_b_tex : repeat_enable, filter_linear; // 雪原、沙漠、沼澤
uniform sampler2D pid_tex : filter_nearest;                 // RG8：州郡 ID 低、高位元組
uniform sampler2D pcol_tex : filter_nearest;                // 256×1：州郡 ID → 淡彩（sRGB）

const float NO_PROVINCE = 65535.0;
// 極區（GDD 5.3「極區」）：|緯度| 自 72° 起漸變為統一極冰，84° 起完全是極冰；高度同步壓平。
const float POL_SIN0 = 0.95105652;
const float POL_SIN1 = 0.99452190;

varying vec3 v_wpos;
varying vec2 v_grad;
varying float v_h1;
varying vec2 v_lpos;
varying vec3 v_sdir;
varying float v_curv;

float polar_weight(float sin_lat) {
	return smoothstep(POL_SIN0, POL_SIN1, abs(sin_lat));
}

void vertex() {
	// 球面方向由經緯直接算出，極點列的所有頂點得到同一個方向。
	float lon = UV.x * TAU;
	float lat = (0.5 - UV.y) * PI;
	vec3 dir = vec3(cos(lat) * cos(lon), sin(lat), -cos(lat) * sin(lon));
	float flat_k = 1.0 - polar_weight(dir.y);
	float h1 = VERTEX.y * flat_k;
	vec3 wp = vec3(VERTEX.x, h1 * height_scale, VERTEX.z);
	if (globe_morph > 0.0) {
		float r = globe_radius + h1 * height_scale * globe_relief;
		vec3 sph = (globe_xform * vec4(dir * r, 1.0)).xyz;
		wp = mix(wp, sph, globe_morph);
	}
	v_lpos = vec2(VERTEX.x - copy_offset, VERTEX.z);
	v_grad = -NORMAL.xz / max(NORMAL.y, 0.05) * flat_k;
	v_h1 = h1;
	v_sdir = dir;
	v_curv = UV2.x * flat_k;
	v_wpos = wp;
	VERTEX = wp;
}

// 三維值雜訊（GDD 5.3「球面空間」規則：程序紋理一律在 3D 球面座標求值）。
float hash3(ivec3 i) {
	uint h = uint(i.x) * 374761393u + uint(i.y) * 668265263u + uint(i.z) * 2147483647u;
	h = (h ^ (h >> 13u)) * 1274126177u;
	h = h ^ (h >> 16u);
	return float(h & 0xffffffu) / 16777215.0;
}

float vnoise3(vec3 p) {
	ivec3 i = ivec3(floor(p));
	vec3 f = fract(p);
	vec3 u = f * f * (3.0 - 2.0 * f);
	float x00 = mix(hash3(i), hash3(i + ivec3(1, 0, 0)), u.x);
	float x10 = mix(hash3(i + ivec3(0, 1, 0)), hash3(i + ivec3(1, 1, 0)), u.x);
	float x01 = mix(hash3(i + ivec3(0, 0, 1)), hash3(i + ivec3(1, 0, 1)), u.x);
	float x11 = mix(hash3(i + ivec3(0, 1, 1)), hash3(i + ivec3(1, 1, 1)), u.x);
	return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

float fbm3(vec3 p) {
	return 0.65 * vnoise3(p) + 0.35 * vnoise3(p * 2.13 + vec3(5.2, 1.3, 8.7));
}

// 三平面取樣：以球面座標 p（單位 = 赤道一格）與混合權重 w 取樣平鋪貼圖。
vec3 tri_paper(vec3 p, vec3 w, float scale) {
	vec3 q = p / scale;
	return w.x * texture(paper_tex, q.yz).rgb
		+ w.y * texture(paper_tex, q.xz + vec2(0.31, 0.57)).rgb
		+ w.z * texture(paper_tex, q.xy + vec2(0.73, 0.19)).rgb;
}

vec2 rot2(vec2 v) {
	return vec2(v.x * 0.82 + v.y * 0.57, -v.x * 0.57 + v.y * 0.82);
}

float tri_stroke(vec3 p, vec3 w, float scale) {
	vec3 q = p / scale;
	return w.x * texture(stroke_tex, rot2(q.yz)).r
		+ w.y * texture(stroke_tex, rot2(q.xz) + vec2(0.41, 0.23)).r
		+ w.z * texture(stroke_tex, rot2(q.xy) + vec2(0.13, 0.67)).r;
}

float bspline2(float d) {
	float a = abs(d);
	return a < 0.5 ? 0.75 - a * a : (a < 1.5 ? 0.5 * (1.5 - a) * (1.5 - a) : 0.0);
}

float pid_at(ivec2 t) {
	vec2 rg = texelFetch(pid_tex, t, 0).rg;
	return floor(rg.r * 255.0 + 0.5) + floor(rg.g * 255.0 + 0.5) * 256.0;
}

// 州郡投票（GDD 5.3 區域邊界平滑）：以二次 B 樣條權重對附近 3×3 格的州郡投票。
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
	return clamp(0.5 * w - abs(d) / max(fw, 1e-5) + 0.5, 0.0, 1.0);
}

vec3 sea_ramp(float t) {
	float x = clamp(t, 0.0, 1.0) * 4.0;
	int i = min(int(floor(x)), 3);
	return mix(sea_colors[i], sea_colors[i + 1], smoothstep(0.0, 1.0, x - float(i)));
}

vec3 mountain_ramp(float t) {
	float x = clamp(t, 0.0, 1.0) * 4.0;
	int i = min(int(floor(x)), 3);
	return mix(mountain_colors[i], mountain_colors[i + 1], smoothstep(0.0, 1.0, x - float(i)));
}

// 連續場色階：紙 → 青 → 金。
vec3 field_ramp(float t) {
	float c = clamp(t, 0.0, 1.0);
	return c < 0.5 ? mix(paper, cyan, c * 2.0) : mix(cyan, gold, (c - 0.5) * 2.0);
}

void fragment() {
	float hs = height_scale;
	vec2 wp = v_lpos;

	// 球面空間：平面 → 圓柱面（東西週期、無極點）、球面 → 單位球，以過渡權重內插。
	vec3 d = normalize(v_sdir);
	float rad = map_size.x / TAU;
	float lon_x = wp.x / map_size.x * TAU;
	vec3 cyl_n = vec3(cos(lon_x), 0.0, -sin(lon_x));
	vec3 p3 = mix(vec3(rad * cyl_n.x, wp.y - 0.5 * map_size.y, rad * cyl_n.z), rad * d, globe_morph);
	vec3 tw0 = pow(abs(normalize(mix(cyl_n, d, globe_morph))), vec3(4.0));
	vec3 tw = tw0 / (tw0.x + tw0.y + tw0.z);
	float cos_lat = max(sqrt(max(1.0 - d.y * d.y, 0.0)), 0.3);

	float n_a = fbm3(p3 / 14.0);
	float n_b = fbm3(p3 / 14.0 + vec3(37.1, 61.7, 12.9));
	float n_c = fbm3(p3 / 5.0 + vec3(13.3, 77.1, 41.7));
	float n_d = fbm3(p3 / 5.0 + vec3(71.9, 29.4, 3.1));
	vec3 pap = tri_paper(p3, tw, 18.0) / 0.96;
	float st = tri_stroke(p3, tw, 48.0);

	// 手繪感：取樣座標以雜訊扭曲（約 ±0.4 格）；東西向位移依 1/cos(緯度) 放大。
	vec2 warp = (vec2(n_a, n_b) - 0.5) * 1.3 + (vec2(n_c, n_d) - 0.5) * 0.7;
	warp.x *= mix(1.0, 1.0 / cos_lat, globe_morph);
	vec2 wuv = (wp + warp) / map_size;
	wuv.y = clamp(wuv.y, 0.0, 1.0);
	// 東西接縫兩側差一整圈：導數取週期內最短差。
	vec2 dwx = dFdx(wp);
	vec2 dwy = dFdy(wp);
	dwx.x -= round(dwx.x / map_size.x) * map_size.x;
	dwy.x -= round(dwy.x / map_size.x) * map_size.x;
	float pxw = max(length(abs(dwx) + abs(dwy)), 1e-4);
	float pol = polar_weight(d.y);

	// 海岸：有號場 a，0.5 為海岸線。
	vec2 uv_c = (wp + warp * 0.4) / map_size;
	uv_c.y = clamp(uv_c.y, 0.0, 1.0);
	vec4 tt = texture(terrain_tex, uv_c);
	float a = tt.a;
	float fw_a = max(fwidth(a), 1e-4);
	float wmask = 1.0 - smoothstep(0.5 - fw_a * 0.75, 0.5 + fw_a * 0.75, a);
	float depth = tt.b;
	vec2 uv_f = (wp + warp * 0.3) / map_size;
	uv_f.y = clamp(uv_f.y, 0.0, 1.0);
	float fv = texture(field_tex, uv_f).r;
	float curv = v_curv * hs;

	// 1 底色 × 紙紋：類別權重乘上 3D 雜訊起伏後取 1.8 次方再正規化，交界寬而柔。
	vec4 wa = texture(cls_a_tex, wuv);
	vec4 wb = texture(cls_b_tex, wuv);
	vec4 ja = vec4(1.0) + (vec4(n_a, n_b, n_c, n_d) - 0.5) * 1.4;
	vec3 jb = vec3(1.0) + (vec3(n_d, n_a, n_b) - 0.5) * 1.4;
	vec4 ka = pow(max(wa * ja, vec4(0.0)), vec4(1.8));
	vec3 kb = pow(max(wb.rgb * jb, vec3(0.0)), vec3(1.8));
	float ksum = max(ka.x + ka.y + ka.z + ka.w + kb.x + kb.y + kb.z, 1e-6);
	vec4 cw_a = ka / ksum;
	vec3 cw_b = kb / ksum;
	vec3 cls_col = terrain_colors[2] * cw_a.x + terrain_colors[3] * cw_a.y + terrain_colors[4] * cw_a.z
		+ terrain_colors[5] * cw_a.w + terrain_colors[6] * cw_b.x + terrain_colors[7] * cw_b.y
		+ terrain_colors[8] * cw_b.z;
	// 高度連續色階：低地保留生態色，往上漸入赭石 → 石綠 → 石青 → 雪（雪線隨緯度降低）。
	float hn = v_h1 / max(max_height, 1e-3);
	float lat_k = d.y * d.y;
	float alt = clamp(hn + 0.30 * lat_k + (n_c - 0.5) * 0.10, 0.0, 1.0);
	float rock_w = smoothstep(0.20, 0.60, alt) * (1.0 - 0.6 * cw_b.y);
	vec3 relief_col = mountain_ramp(smoothstep(0.15, 0.95, alt));
	vec3 biome = mix(cls_col, relief_col, rock_w * 0.9);
	biome = biome * (0.95 + 0.10 * n_a) * mix(vec3(1.03, 1.0, 0.96), vec3(0.97, 1.0, 1.03), n_b);
	vec3 grain = mix(vec3(1.0), pap, 0.95 - 0.5 * flatten);
	vec3 land_col = biome * grain;
	land_col = mix(land_col, mix(land_col, paper, 0.25), flatten);

	// 4 水：海深五階柔和銜接；8 位元水深加入像素抖動打散色帶。
	float dither = (hash3(ivec3(ivec2(FRAGCOORD.xy), 7)) - 0.5) / 255.0;
	float sea_t = 0.92 * pow(clamp(depth + dither, 0.0, 1.0), 0.55);
	vec3 water_col = sea_ramp(sea_t);
	water_col = water_col * mix(vec3(1.0), pap, 0.5) * (0.96 + 0.08 * n_a) * (0.985 + 0.03 * n_c);

	vec3 col = mix(land_col, water_col, wmask);

	// 5 地圖模式疊色
	vec3 vt = vote(wuv);
	float pid = vt.x;
	float fw_m = 1.5 * pxw;
	float marg = vt.z;
	float is_land_px = (1.0 - wmask) * (pid != NO_PROVINCE ? 1.0 : 0.0);
	vec4 pc = pcol(pid);
	float tint = pc.a * tint_opacity * is_land_px;
	col = mix(col, vec3(dot(col, vec3(0.2126, 0.7152, 0.0722))), 0.4 * tint);
	col = mix(col, col * pc.rgb * 1.3, tint);
	if (map_mode == 3) {
		col = mix(col, field_ramp(fv) * mix(vec3(1.0), pap, 0.5), field_opacity * (1.0 - wmask));
	}

	// 2 上墨 / 3 山脊線（8：隨高度縮放降為 0）
	vec2 g = v_grad * hs;
	float slope = length(g);
	float ink_amt = max(smoothstep(0.30, 0.80, slope), smoothstep(0.55, 0.85, hn) * 0.8);
	float k = smoothstep(0.28, 0.72, st * 0.9 + (ink_amt - 0.35) * 0.9);
	float w_ink = smoothstep(0.05, 0.30, hs) * (1.0 - wmask);
	float ridge = smoothstep(0.10, 0.32, curv) * smoothstep(0.05, 0.25, slope);
	float ink_a = max(k * ink_amt * 0.5, ridge * (0.35 + 0.35 * st)) * w_ink;
	// 山影：陰面染冷（花青調）、陽面微暖，色相不變灰。
	vec3 nrm = normalize(vec3(-g.x, 1.0, -g.y));
	vec3 ldir = normalize(light.xyz);
	float shade = (dot(nrm, ldir) - ldir.y) * light.w;
	float w_land = (1.0 - wmask) * smoothstep(0.02, 0.2, hs);
	col = mix(col, col * vec3(0.60, 0.69, 0.80), clamp(-shade * 1.7, 0.0, 0.7) * w_land);
	col = mix(col, min(col * vec3(1.07, 1.03, 0.94), vec3(1.0)), clamp(shade * 1.2, 0.0, 0.5) * w_land);
	col = mix(col, mix(ink_light, ink, 0.5), clamp(ink_a, 0.0, 1.0));

	// 棚緣等深線：極淡的 ink-light 細線（只在水面上）。
	float fw_s = max(fwidth(sea_t), 1e-4);
	col = mix(col, ink_light, 0.16 * px_line(sea_t - 0.42, fw_s, 1.0) * wmask);

	// 海岸雙線（ink-light）
	float c1 = px_line(a - 0.5, fw_a, 1.3);
	float c2 = px_line(a - (0.5 - min(3.5 * fw_a, 0.45)), fw_a, 1.0) * 0.55;
	col = mix(col, ink_light, max(c1 * 0.9, c2));

	// 6 邊界：州界 ink-light 1 px；國界 ink 2 px 加外側淡暈。墨線在 62°–78° 先行淡出。
	bool has_b = vt.y != NO_PROVINCE;
	vec4 pca = pcol(vt.x);
	vec4 pcb = pcol(vt.y);
	bool realm_diff = has_b && (pca.rgb != pcb.rgb || pca.a != pcb.a);
	float land_f = (1.0 - wmask) * (1.0 - smoothstep(0.883, 0.978, abs(d.y)));
	if (realm_diff) {
		col = mix(col, ink_light, 0.16 * px_line(marg, fw_m, 6.0) * land_f);
	}
	col = mix(col, ink_light, 0.85 * px_line(marg, fw_m, 1.2) * land_f);
	if (realm_diff) {
		col = mix(col, ink, 0.9 * px_line(marg, fw_m, 2.2) * land_f);
	}

	// 停留／選取高亮
	float hot_cov = px_line(marg, fw_m, 2.6);
	bool hov_hit = hover_province >= 0.0 && (vt.x == hover_province || vt.y == hover_province);
	bool sel_hit = selected_province >= 0.0 && (vt.x == selected_province || vt.y == selected_province);
	if (pid != NO_PROVINCE && pid == hover_province) {
		col = mix(col, paper + vec3(0.1), 0.22 * land_f);
	}
	if (pid != NO_PROVINCE && pid == selected_province) {
		col = mix(col, gold, 0.24 * land_f);
	}
	if (has_b && hov_hit) {
		col = mix(col, ink, 0.75 * hot_cov * land_f);
	}
	if (has_b && sel_hit) {
		col = mix(col, gold, 0.95 * hot_cov * land_f);
	}

	// 極區：統一的極冰風格，州郡邊界、海岸線、山墨在極點收斂處不再星芒。
	vec3 ice = mix(terrain_colors[6], water, 0.12) * mix(vec3(1.0), pap, 0.8) * (0.96 + 0.08 * n_a);
	col = mix(col, ice, pol);

	// 7 霧
	float dist = length(CAMERA_POSITION_WORLD - v_wpos);
	float fog = smoothstep(fog_start, fog_end, dist);
	col = mix(col, paper, fog * 0.85);
	// 南北緣之外是「極地邊疆」：極冰向外延伸並淡成紙色；星球上沒有邊緣。
	float out_y = max(-wp.y, wp.y - map_size.y);
	float fade_y = smoothstep(-2.0, 0.05 * map_size.x, out_y);
	float fade = fade_y * (1.0 - globe_morph);
	col = mix(col, mix(paper * pap, paper, fade_y), fade * mix(0.92, 1.0, fade_y));

	ALBEDO = col;
}
