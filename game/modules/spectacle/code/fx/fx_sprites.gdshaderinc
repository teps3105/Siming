// 奇觀粒子的共用部分：一個網格是一批四邊形，每個四邊形的位置、大小與透明度全由頂點著色器依
// 「奇觀的 age（秒）」算出，沒有 CPU 端的粒子狀態；暫停時 age 不動，粒子就停住，測試與截圖可以直接跳到任一刻。
// 每個粒子由編號雜湊出隨機數：出生時間、起點、速度、大小；四邊形朝向鏡頭（正交鏡頭下就是視圖的 XY 平面），
// 可沿速度方向拉長成拖尾。做法參考 Storm & Wind 3D Free 的 debris.gdshader（CC BY 4.0）把運動放在 GPU 上的思路，
// 程式為專案自寫；貼圖是 Kenney Particle Pack（CC0）。

uniform sampler2D tex : source_color, filter_linear_mipmap, repeat_disable;
uniform float age = 0.0;
uniform float seed = 0.0;
uniform float master : hint_range(0.0, 2.0) = 1.0;
// 閃電打亮：把 flash_color 加到色上（0..1）。
uniform float flash : hint_range(0.0, 2.0) = 0.0;
uniform vec3 flash_color = vec3(1.0, 0.92, 0.7);
// 出生在這麼多秒內陸續發生；非循環的粒子活 life 秒後消失，循環的每 life 秒重生一次。
uniform float emit_span = 0.5;
uniform float life = 1.0;
uniform bool looping = false;
// 起點：以 origin_off 為中心、extent 為半徑的方盒（shell=0）或橢球（shell=1），單位同節點（奇觀半徑為 1）。
uniform vec3 extent = vec3(0.1);
uniform float shell : hint_range(0.0, 1.0) = 0.0;
uniform vec3 origin_off = vec3(0.0);
uniform vec3 vel = vec3(0.0);
uniform vec3 vel_spread = vec3(0.0);
// 沿起點水平方向向外的速度；負值向內。
uniform float radial = 0.0;
uniform vec3 accel = vec3(0.0);
uniform float drag = 0.0;
// 繞 Y 軸（以 origin_off 為心）的角速度，與起點離心的距離從 from_scale 倍在 settle_time 秒內收到 1 倍。
uniform float swirl = 0.0;
uniform float from_scale = 1.0;
uniform float settle_time = 0.001;
uniform float size0 = 0.2;
uniform float size1 = 0.2;
uniform float size_rand : hint_range(0.0, 1.0) = 0.0;
// 四邊形長寬比（長 / 寬）與沿速度方向的拖尾（每單位速度加長多少）。
uniform float aspect = 1.0;
uniform float stretch = 0.0;
uniform float spin = 0.0;
uniform vec4 color0 : source_color = vec4(1.0);
uniform vec4 color1 : source_color = vec4(1.0);
// 每個粒子隨機混向 tint_b 的程度。
uniform vec4 tint_b : source_color = vec4(1.0);
uniform float tint_mix : hint_range(0.0, 1.0) = 0.0;
uniform float fade_in = 0.1;
uniform float fade_out_pow = 1.0;
// 曲線起點模式（了結的火星）：粒子由貝茲弧 pa、pc、pb（世界座標）的兩端各自向中點燒的位置出生。
uniform bool curve_emit = false;
uniform vec3 pa = vec3(0.0);
uniform vec3 pc = vec3(0.0);
uniform vec3 pb = vec3(0.0);
uniform float burn_seconds = 2.25;
uniform float world_scale = 1.0;
// 貼在地面上（節點的 XZ 平面）而不是朝向鏡頭：環與地面光暈用。
uniform bool flat_plane = false;

varying float v_alpha;
varying float v_k;
varying float v_tint;

float h11(float n) {
	return fract(sin(n * 127.1 + seed * 311.7) * 43758.5453);
}

vec4 rnd4(float id) {
	return vec4(h11(id * 4.0 + 1.0), h11(id * 4.0 + 2.0), h11(id * 4.0 + 3.0), h11(id * 4.0 + 4.0));
}

vec3 bez(float f) {
	float g = 1.0 - f;
	return g * g * pa + 2.0 * g * f * pc + f * f * pb;
}

vec3 sprite_vertex(inout vec3 VERTEX_IO, in vec2 corner, in int vid, in mat4 model, in mat4 view) {
	float id = float(vid / 4);
	vec4 r = rnd4(id);
	vec4 r2 = rnd4(id + 977.0);
	float pt;
	float alive = 1.0;
	if (looping) {
		pt = mod(age + r.x * life, life);
	} else {
		pt = age - r.x * emit_span;
		alive = (pt >= 0.0 && pt <= life) ? 1.0 : 0.0;
		pt = clamp(pt, 0.0, life);
	}
	float k = pt / life;
	float ms = curve_emit ? world_scale : length(model[0].xyz);

	vec3 dv = vec3(r.y, r.z, r2.x) * 2.0 - 1.0;
	vec3 dir = normalize(dv + vec3(1e-4));
	vec3 off = mix(dv, dir * pow(r2.y, 0.5), shell) * extent;
	vec3 rdir = normalize(vec3(off.x, 0.0, off.z) + vec3(1e-4, 0.0, 0.0));
	vec3 v0 = vel + (vec3(r2.z, r2.w, r.w) * 2.0 - 1.0) * vel_spread + rdir * radial;
	float ek = drag > 0.0 ? (1.0 - exp(-drag * pt)) / drag : pt;
	float sc = mix(from_scale, 1.0, smoothstep(0.0, 1.0, clamp(pt / settle_time, 0.0, 1.0)));
	vec3 motion = off * sc + v0 * ek + 0.5 * accel * pt * pt;
	float ang = swirl * pt;
	float sa = sin(ang);
	float ca = cos(ang);
	motion.xz = vec2(motion.x * ca - motion.z * sa, motion.x * sa + motion.z * ca);
	vec3 vnow = v0 * (drag > 0.0 ? exp(-drag * pt) : 1.0) + accel * pt;

	vec3 center;
	if (curve_emit) {
		float born = age - pt;
		alive *= (born >= 0.0 && born <= burn_seconds) ? 1.0 : 0.0;
		float reach = 0.5 * clamp(born / burn_seconds, 0.0, 1.0);
		float f = r.w < 0.5 ? reach : 1.0 - reach;
		vec3 wp = bez(f) + (motion + origin_off) * ms;
		center = (view * vec4(wp, 1.0)).xyz;
	} else {
		center = (view * model * vec4(motion + origin_off, 1.0)).xyz;
	}

	float sz0 = mix(size0, size1, k) * (1.0 + size_rand * (r2.z * 2.0 - 1.0)) * alive;
	float sz = sz0 * ms;
	vec2 axis = vec2(cos(r.x * 6.2831 + spin * (r2.w - 0.5) * pt), sin(r.x * 6.2831 + spin * (r2.w - 0.5) * pt));
	float length_k = aspect;
	float extra = 0.0;
	if (stretch > 0.0) {
		vec3 vw = curve_emit ? vnow * ms : (model * vec4(vnow, 0.0)).xyz;
		vec2 vd = (view * vec4(vw, 0.0)).xy;
		float sp = length(vd);
		axis = sp > 1e-4 ? vd / sp : vec2(0.0, 1.0);
		extra = stretch * sp;
	}
	vec2 perp = vec2(-axis.y, axis.x);
	// corner.y 沿 axis（貼圖上緣在 axis 前端）。
	vec2 o = perp * corner.x * sz + axis * corner.y * (sz * length_k + extra * ms * alive);
	if (flat_plane && !curve_emit) {
		vec2 o2 = perp * corner.x * sz0 + axis * corner.y * sz0 * length_k;
		VERTEX_IO = (view * model * vec4(motion + origin_off + vec3(o2.x, 0.0, o2.y), 1.0)).xyz;
	} else {
		VERTEX_IO = center + vec3(o, 0.0);
	}
	return vec3(k, r.z, alive * smoothstep(0.0, max(fade_in, 1e-4), pt) * pow(max(1.0 - k, 0.0), fade_out_pow));
}
