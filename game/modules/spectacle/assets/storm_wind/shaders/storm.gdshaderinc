// Shared by every Storm & Wind shader. StormFX sets these uniforms; the including shader sets its
// render_mode, ends with storm_color() or storm_alpha() and discards where alpha is below 0.

uniform vec4 color : source_color = vec4(0.85, 0.93, 1.0, 1.0);       // bolts' cores, wind lines
uniform vec4 edge_color : source_color = vec4(0.35, 0.55, 1.0, 1.0);  // the glow round bolts, dust
uniform float energy : hint_range(0.0, 4.0) = 1.4;   // over 1 blooms under an Environment's glow
uniform float intensity : hint_range(0.0, 2.0) = 1.0;
uniform float age = 0.0;                             // seconds, times the node's speed
uniform float seed = 0.0;
uniform float flash : hint_range(0.0, 2.0) = 1.0;    // a bolt's brightness, stroke by stroke
uniform float fired = -100.0;                        // `age` at the last strike or burst
uniform float radius = 1.5;
uniform float height = 3.0;
// 0 smooth, 1 dither (the PS1's 4x4 ordered dither, as solid pixels), 2 cutout
uniform int transparency : hint_range(0, 2) = 0;
uniform float dither_size : hint_range(1.0, 8.0, 1.0) = 1.0;   // screen pixels per dither cell

// Integer hash: a float hash of large coordinates tears noise into steps on some GPUs.
float storm_hash_i(ivec2 i) {
	uvec2 q = uvec2(i) * uvec2(1597334673u, 3812015801u);
	uint n = (q.x ^ q.y) * 1597334673u;
	return float(n) * (1.0 / 4294967295.0);
}

// Four randoms for piece `id` in pass `pass`.
vec4 storm_hash4(float id, float pass) {
	int k = int(id) * 4 + int(seed * 37.0) * 7919;
	int p = int(pass);
	return vec4(storm_hash_i(ivec2(k, p)), storm_hash_i(ivec2(k + 1, p)),
			storm_hash_i(ivec2(k + 2, p)), storm_hash_i(ivec2(k + 3, p)));
}

// i wrapped into 0..n-1, safe for negatives (% on negative ints is undefined in shaders).
int storm_wrap(int i, float n) {
	return i - int(n) * int(floor((float(i) + 0.5) / n));
}

// Value noise; with tile > 0 it tiles across x every `tile` units, for round meshes' seams.
float storm_noise(vec2 p, float tile) {
	vec2 fl = floor(p);
	vec2 f = p - fl;
	f = f * f * (3.0 - 2.0 * f);
	int x0 = int(fl.x);
	int x1 = x0 + 1;
	int y0 = int(fl.y) + int(seed * 37.0) * 101;
	if (tile > 0.0) {
		x0 = storm_wrap(x0, tile);
		x1 = storm_wrap(x1, tile);
	}
	return mix(mix(storm_hash_i(ivec2(x0, y0)), storm_hash_i(ivec2(x1, y0)), f.x),
			mix(storm_hash_i(ivec2(x0, y0 + 1)), storm_hash_i(ivec2(x1, y0 + 1)), f.x), f.y);
}

// 0..1, spread to cover the range.
float storm_fbm(vec2 p, float tile) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 3; i++) {
		v += a * storm_noise(p, tile);
		p = p * 2.0 + vec2(0.0, 17.0);
		tile *= 2.0;
		a *= 0.5;
	}
	return clamp((v - 0.12) / 0.68, 0.0, 1.0);
}

// A ribbon vertex in view space: `p` on the line (model space), `t` the line's way, pushed
// `side * width` across it and across the eye ray, so the ribbon always faces the camera.
vec3 storm_ribbon(mat4 modelview, vec3 p, vec3 t, float side, float width) {
	vec3 pv = (modelview * vec4(p, 1.0)).xyz;
	vec3 tv = (modelview * vec4(t, 0.0)).xyz;
	// Modified: the map camera is orthographic, so every eye ray is the view axis.
	vec3 across = cross(tv, vec3(0.0, 0.0, -1.0));
	float l = length(across);
	across = l > 1e-6 ? across / l : vec3(1.0, 0.0, 0.0);
	return pv + across * side * width;
}

// A camera-facing quad's corner in view space: `center` in model space, `corner` -1..1 scaled
// by `extent` along `axis` (a unit view-space x,y) and across it.
vec3 storm_billboard(mat4 modelview, vec3 center, vec2 corner, vec2 extent, vec2 axis) {
	vec3 v = (modelview * vec4(center, 1.0)).xyz;
	return v + vec3(axis * corner.y * extent.y + vec2(axis.y, -axis.x) * corner.x * extent.x, 0.0);
}

float storm_bayer2(vec2 a) {
	a = floor(a);
	return fract(dot(a, vec2(0.5, a.y * 0.75)));
}

float storm_bayer4(vec2 a) {
	return storm_bayer2(0.5 * a) * 0.25 + storm_bayer2(a);
}

// How much covers the pixel, after intensity and the transparency mode: below 0 discards.
float storm_alpha(float a, vec2 frag) {
	a = clamp(a * intensity, 0.0, 1.0);
	if (transparency == 1) {
		a = a > storm_bayer4(frag / dither_size) * 0.94 + 0.03 ? 1.0 : -1.0;
	} else if (transparency == 2) {
		a = a >= 0.5 ? 1.0 : -1.0;
	}
	return a;
}

// heat 1 is the core colour, 0 the edge's; a is how much covers the pixel.
vec4 storm_color(float heat, float a, vec2 frag) {
	vec4 c = mix(edge_color, color, clamp(heat, 0.0, 1.0));
	return vec4(c.rgb * energy, storm_alpha(a * c.a, frag));
}
