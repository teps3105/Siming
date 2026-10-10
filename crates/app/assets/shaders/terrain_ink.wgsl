// GDD 5.3 水墨著色器（地形）、9.4 著色器規範、5.1 色票（色值全由 uniform 傳入）。
// 相容 WebGL2：不使用 storage buffer / compute；整數貼圖一律以 textureLoad 讀取。
//
// 合成順序（5.3 步驟）：1 底色×紙紋 → 4 水（海岸雙線、河流）→ 5 地圖模式疊色 / 連續場
// → 2 上墨（坡度/高度 × 皴法）→ 3 山脊線 → 6 邊界 → 7 霧 → 8 古地圖化（ink 權重隨高度縮放降為 0）。
// 上墨置於疊色之後，使墨色不被淡彩蓋掉。

#import bevy_pbr::{
    mesh_functions::{get_world_from_local, mesh_position_local_to_world},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::view,
}

struct InkParams {
    paper: vec4<f32>,
    paper_dark: vec4<f32>,
    ink: vec4<f32>,
    ink_light: vec4<f32>,
    water: vec4<f32>,
    water_deep: vec4<f32>,
    gold: vec4<f32>,
    cyan: vec4<f32>,
    seal_red: vec4<f32>,
    terrain: array<vec4<f32>, 9>,
    sea: array<vec4<f32>, 5>,  // 海深色階（淺 -> 深）
    ramp: array<vec4<f32>, 5>, // 山地高度色階（低 -> 高）
    vw: vec4<f32>, // x 高度縮放, y 古地圖化, z 模式, w 時間
    sel: vec4<f32>,  // x 停留, y 選取, z 疊色不透明度, w 因果權重
    map: vec4<f32>,  // x 寬, y 高, z 最高點, w 靈脈場不透明度
    fog: vec4<f32>,  // x 霧起點, y 霧終點, z 邊緣淡出寬
    globe: vec4<f32>, // x 球面過渡權重(0 平面, 1 球面), y 球半徑, z 球面高度位移倍率, w 東西環繞
    light: vec4<f32>, // xyz 指向光源的單位向量, w 山影強度（燈光層掛鉤）
    grot: mat4x4<f32>, // 球面局部座標 -> 世界座標
};

@group(2) @binding(0) var<uniform> P: InkParams;
@group(2) @binding(1) var paper_tex: texture_2d<f32>;
@group(2) @binding(2) var rep_samp: sampler;
@group(2) @binding(3) var stroke_tex: texture_2d<f32>;
@group(2) @binding(4) var cls_a_tex: texture_2d<f32>; // 地形類別權重：平原、丘陵、山脈、森林
@group(2) @binding(5) var pid_tex: texture_2d<u32>;
@group(2) @binding(6) var pcol_tex: texture_2d<f32>;
@group(2) @binding(7) var field_tex: texture_2d<f32>;
@group(2) @binding(8) var clamp_samp: sampler;
@group(2) @binding(9) var terrain_tex: texture_2d<f32>;
@group(2) @binding(10) var cls_b_tex: texture_2d<f32>; // 雪原、沙漠、沼澤

const NO_PROVINCE: u32 = 65535u;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) curv: vec2<f32>, // x：高度場曲率（縮放 = 1），CPU 逐頂點算好
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) wpos: vec3<f32>, // 世界座標（霧用）
    @location(1) grad: vec2<f32>, // 縮放 = 1 的高度梯度 (dh/dx, dh/dz)
    @location(2) h1: f32,         // 縮放 = 1 的高度
    @location(3) lpos: vec3<f32>, // 地圖局部平面座標（只供資料貼圖查表；各東西副本與球面共用）
    @location(4) sdir: vec3<f32>, // 單位球方向（頂點由經緯算出；內插後正規化，極點與接縫皆連續）
    @location(5) curv: f32,
};

const PI: f32 = 3.14159265;
// 極區（GDD 5.3「極區」）：|緯度| 自 72° 起漸變為統一極冰，84° 起完全是極冰；高度同步壓平。
const POL_SIN0: f32 = 0.95105652; // sin 72°
const POL_SIN1: f32 = 0.99452190; // sin 84°

fn polar_weight(sin_lat: f32) -> f32 {
    return smoothstep(POL_SIN0, POL_SIN1, abs(sin_lat));
}

@vertex
fn vertex(v: Vertex) -> VOut {
    var o: VOut;
    let hs = P.vw.x;
    // 球面方向由經緯直接算出（不依賴網格頂點的球面位置），極點列的所有頂點得到同一個 dir。
    let lon = v.uv.x * 2.0 * PI;
    let lat = (0.5 - v.uv.y) * PI;
    let dir = vec3<f32>(cos(lat) * cos(lon), sin(lat), -cos(lat) * sin(lon));
    // 極區高度壓平：極點列各頂點高度不同會在球面上拉出星芒狀尖刺。
    let flat_k = 1.0 - polar_weight(dir.y);
    let h1 = v.position.y * flat_k;
    let local = vec3<f32>(v.position.x, h1 * hs, v.position.z);
    var wp = mesh_position_local_to_world(get_world_from_local(v.instance_index), vec4<f32>(local, 1.0)).xyz;
    // GDD 5.3 星球視角：平面座標 <-> 球面座標的頂點內插（等距長方投影的 UV -> 經緯）。
    let morph = P.globe.x;
    if (morph > 0.0) {
        let r = P.globe.y + h1 * hs * P.globe.z;
        let sph = (P.grot * vec4<f32>(dir * r, 1.0)).xyz;
        wp = mix(wp, sph, morph);
    }
    o.clip = position_world_to_clip(wp);
    o.wpos = wp;
    o.lpos = v.position;
    o.grad = -v.normal.xz / max(v.normal.y, 0.05) * flat_k;
    o.h1 = h1;
    o.sdir = dir;
    o.curv = v.curv.x * flat_k;
    return o;
}

// ---------------------------------------------------------------- 工具

// 三維值雜訊（GDD 5.3「球面空間」規則：全部程序紋理在 3D 球面座標求值，不在 UV 求值）。
fn hash3(i: vec3<i32>) -> f32 {
    var h = bitcast<u32>(i.x) * 374761393u + bitcast<u32>(i.y) * 668265263u + bitcast<u32>(i.z) * 2147483647u;
    h = (h ^ (h >> 13u)) * 1274126177u;
    h = h ^ (h >> 16u);
    return f32(h & 0xffffffu) / 16777215.0;
}

fn vnoise3(p: vec3<f32>) -> f32 {
    let i = vec3<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let x00 = mix(hash3(i), hash3(i + vec3<i32>(1, 0, 0)), u.x);
    let x10 = mix(hash3(i + vec3<i32>(0, 1, 0)), hash3(i + vec3<i32>(1, 1, 0)), u.x);
    let x01 = mix(hash3(i + vec3<i32>(0, 0, 1)), hash3(i + vec3<i32>(1, 0, 1)), u.x);
    let x11 = mix(hash3(i + vec3<i32>(0, 1, 1)), hash3(i + vec3<i32>(1, 1, 1)), u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    return 0.65 * vnoise3(p) + 0.35 * vnoise3(p * 2.13 + vec3<f32>(5.2, 1.3, 8.7));
}

// 三平面取樣：以球面座標 p（單位 = 赤道一格）與混合權重 w 取樣平鋪貼圖，無 UV 極點壓縮。
fn tri_paper(p: vec3<f32>, w: vec3<f32>, scale: f32) -> vec3<f32> {
    let q = p / scale;
    return w.x * textureSample(paper_tex, rep_samp, q.yz).rgb
        + w.y * textureSample(paper_tex, rep_samp, q.xz + vec2<f32>(0.31, 0.57)).rgb
        + w.z * textureSample(paper_tex, rep_samp, q.xy + vec2<f32>(0.73, 0.19)).rgb;
}

fn rot2(v: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(v.x * 0.82 + v.y * 0.57, -v.x * 0.57 + v.y * 0.82);
}

fn tri_stroke(p: vec3<f32>, w: vec3<f32>, scale: f32) -> f32 {
    let q = p / scale;
    return w.x * textureSample(stroke_tex, rep_samp, rot2(q.yz)).r
        + w.y * textureSample(stroke_tex, rep_samp, rot2(q.xz) + vec2<f32>(0.41, 0.23)).r
        + w.z * textureSample(stroke_tex, rep_samp, rot2(q.xy) + vec2<f32>(0.13, 0.67)).r;
}

// 州郡投票（GDD 5.3 區域邊界平滑）：以二次 B 樣條權重（支撐 3 格）對附近 3×3 格的州郡投票，
// 取最高者為該像素的州郡；第一、二名權重差 m 在州界上為 0、約每格 1.5 單位線性增加。
// 這是各州郡指示函數模糊後的等值線：格點的階梯被磨成圓順曲線，且逐像素求值，任何縮放都無階梯。
struct Vt {
    a: u32,
    b: u32,
    m: f32,
};

fn bspline2(d: f32) -> f32 {
    let a = abs(d);
    return select(select(0.0, 0.5 * (1.5 - a) * (1.5 - a), a < 1.5), 0.75 - a * a, a < 0.5);
}

fn vote(uv: vec2<f32>) -> Vt {
    let sz = vec2<f32>(textureDimensions(pid_tex));
    let f = uv * sz - 0.5;
    let c = round(f);
    var ids: array<u32, 9>;
    var wts: array<f32, 9>;
    var n = 0;
    for (var j = -1; j <= 1; j++) {
        let wy = bspline2(f.y - (c.y + f32(j)));
        let ty = clamp(i32(c.y) + j, 0, i32(sz.y) - 1);
        for (var i = -1; i <= 1; i++) {
            let wx = bspline2(f.x - (c.x + f32(i)));
            var tx = i32(c.x) + i;
            if (P.globe.w > 0.5) {
                tx = ((tx % i32(sz.x)) + i32(sz.x)) % i32(sz.x);
            } else {
                tx = clamp(tx, 0, i32(sz.x) - 1);
            }
            let id = textureLoad(pid_tex, vec2<i32>(tx, ty), 0).r;
            if (id == NO_PROVINCE) {
                continue;
            }
            var found = false;
            for (var k = 0; k < n; k++) {
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
    var o: Vt;
    o.a = NO_PROVINCE;
    o.b = NO_PROVINCE;
    var w1 = 0.0;
    var w2 = 0.0;
    for (var k = 0; k < n; k++) {
        if (wts[k] > w1) {
            w2 = w1;
            o.b = o.a;
            w1 = wts[k];
            o.a = ids[k];
        } else if (wts[k] > w2) {
            w2 = wts[k];
            o.b = ids[k];
        }
    }
    o.m = w1 - w2;
    return o;
}

fn pcol(p: u32) -> vec4<f32> {
    return textureLoad(pcol_tex, vec2<i32>(i32(min(p, 255u)), 0), 0);
}

// 以像素為單位的線段覆蓋率（d：到線中心的距離，fw：該距離量每像素的變化量，w：線寬像素）。
fn px_line(d: f32, fw: f32, w: f32) -> f32 {
    return clamp(0.5 * w - abs(d) / max(fw, 1e-5) + 0.5, 0.0, 1.0);
}

// 五階色階的平滑內插（墨分五色：階與階之間以 smoothstep 柔和銜接）。
fn sea_ramp(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0) * 4.0;
    let i = min(i32(floor(x)), 3);
    return mix(P.sea[i].rgb, P.sea[i + 1].rgb, smoothstep(0.0, 1.0, x - f32(i)));
}

fn mountain_ramp(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0) * 4.0;
    let i = min(i32(floor(x)), 3);
    return mix(P.ramp[i].rgb, P.ramp[i + 1].rgb, smoothstep(0.0, 1.0, x - f32(i)));
}

fn ramp(t: f32) -> vec3<f32> {
    let c = clamp(t, 0.0, 1.0);
    return select(
        mix(P.cyan.rgb, P.gold.rgb, (c - 0.5) * 2.0),
        mix(P.paper.rgb, P.cyan.rgb, c * 2.0),
        c < 0.5,
    );
}

fn is_hot(b: Vt, id: f32) -> bool {
    return id >= 0.0 && (f32(b.a) == id || f32(b.b) == id);
}

// ---------------------------------------------------------------- 片元

@fragment
fn fragment(in: VOut) -> @location(0) vec4<f32> {
    let hs = P.vw.x;
    let flat_w = P.vw.y;
    let mode = i32(round(P.vw.z));
    let mapd = P.map.xy;
    let wp = in.lpos.xz;

    // 球面空間（GDD 5.3）：所有程序紋理（紙紋、筆觸、雲紋、扭曲）以連續的 3D 位置求值。
    // 平面 → 圓柱面（東西週期、無極點）、球面 → 單位球，兩者以過渡權重內插，過渡時不跳變。
    let d = normalize(in.sdir);
    let morph = P.globe.x;
    let rad = P.map.x / (2.0 * PI);
    let lon_x = wp.x / P.map.x * 2.0 * PI;
    let cyl_n = vec3<f32>(cos(lon_x), 0.0, -sin(lon_x));
    let p3 = mix(vec3<f32>(rad * cyl_n.x, wp.y - 0.5 * mapd.y, rad * cyl_n.z), rad * d, morph);
    let tw0 = pow(abs(normalize(mix(cyl_n, d, morph))), vec3<f32>(4.0));
    let tw = tw0 / (tw0.x + tw0.y + tw0.z);
    let cos_lat = max(sqrt(max(1.0 - d.y * d.y, 0.0)), 0.3);

    let n_a = fbm3(p3 / 14.0);
    let n_b = fbm3(p3 / 14.0 + vec3<f32>(37.1, 61.7, 12.9));
    let n_c = fbm3(p3 / 5.0 + vec3<f32>(13.3, 77.1, 41.7));
    let n_d = fbm3(p3 / 5.0 + vec3<f32>(71.9, 29.4, 3.1));
    let pap = tri_paper(p3, tw, 18.0) / 0.96;
    let st = tri_stroke(p3, tw, 48.0);

    // 不規則的手繪感：取樣座標以雜訊扭曲（約 ±0.4 格）。東西向位移依 1/cos(緯度) 放大，
    // 使球面上的實際位移各向同性（受 clamp 限制）。
    var warp = (vec2<f32>(n_a, n_b) - 0.5) * 1.3 + (vec2<f32>(n_c, n_d) - 0.5) * 0.7;
    warp.x = warp.x * mix(1.0, 1.0 / cos_lat, morph);
    let wuv = (wp + warp) / mapd;
    // 東西接縫兩側 wuv 差一整圈：導數取週期內最短差，避免接縫處一像素寬的假邊界線。
    var dwx = dpdx(wp);
    var dwy = dpdy(wp);
    if (P.globe.w > 0.5) {
        dwx.x = dwx.x - round(dwx.x / mapd.x) * mapd.x;
        dwy.x = dwy.x - round(dwy.x / mapd.x) * mapd.x;
    }
    let pxw = max(length(abs(dwx) + abs(dwy)), 1e-4);
    let pol = polar_weight(d.y);

    // 海岸：有號場 a，0.5 為海岸線。
    let uv_c = (wp + warp * 0.4) / mapd;
    let a = textureSample(terrain_tex, clamp_samp, uv_c).a;
    let fw_a = max(fwidth(a), 1e-4);
    let wmask = 1.0 - smoothstep(0.5 - fw_a * 0.75, 0.5 + fw_a * 0.75, a);
    let depth = textureSample(terrain_tex, clamp_samp, uv_c).b; // 正規化水深（B 樣條平滑）
    let rv = textureSample(terrain_tex, clamp_samp, (wp + warp * 0.25) / mapd).g;
    let fv = textureSample(field_tex, clamp_samp, (wp + warp * 0.3) / mapd).r;
    let curv = in.curv * hs;

    // 1 底色 × 紙紋
    // 地形類別：格點大小的權重貼圖雙線性取樣（取樣座標已被 3D 雜訊扭曲），
    // 取高次方再正規化 = 在內插權重的等值線上切開，邊緣圓順、有機，不貼格線。
    // 生態過渡帶：權重乘上 3D 雜訊起伏（零權重維持零），再取 1.8 次方——交界寬而柔、邊緣不規則，
    // 不加墨暈（墨線只留給海岸、州界與河流）。
    let wa = textureSample(cls_a_tex, clamp_samp, wuv);
    let wb = textureSample(cls_b_tex, clamp_samp, wuv);
    let ja = vec4<f32>(1.0) + (vec4<f32>(n_a, n_b, n_c, n_d) - 0.5) * 1.4;
    let jb = vec3<f32>(1.0) + (vec3<f32>(n_d, n_a, n_b) - 0.5) * 1.4;
    let ka = pow(max(wa * ja, vec4<f32>(0.0)), vec4<f32>(1.8));
    let kb = pow(max(wb.rgb * jb, vec3<f32>(0.0)), vec3<f32>(1.8));
    let ksum = max(ka.x + ka.y + ka.z + ka.w + kb.x + kb.y + kb.z, 1e-6);
    let cw_a = ka / ksum;
    let cw_b = kb / ksum;
    let cls_col = P.terrain[2].rgb * cw_a.x + P.terrain[3].rgb * cw_a.y + P.terrain[4].rgb * cw_a.z + P.terrain[5].rgb * cw_a.w
        + P.terrain[6].rgb * cw_b.x + P.terrain[7].rgb * cw_b.y + P.terrain[8].rgb * cw_b.z;
    // 高度連續色階：低地保留生態色，往上漸入赭石 → 石綠 → 石青 → 雪（雪線隨緯度降低）。
    let hn = in.h1 / max(P.map.z, 1e-3);
    let lat_k = d.y * d.y;
    let alt = clamp(hn + 0.30 * lat_k + (n_c - 0.5) * 0.10, 0.0, 1.0);
    let rock_w = smoothstep(0.20, 0.60, alt) * (1.0 - 0.6 * cw_b.y); // 沙漠留在低處
    let relief_col = mountain_ramp(smoothstep(0.15, 0.95, alt));
    var biome = mix(cls_col, relief_col, rock_w * 0.9);
    // 大尺度 3D 色彩起伏：平原不再是平塗；冷暖與明度兩軸緩緩漂移。
    biome = biome * (0.95 + 0.10 * n_a) * mix(vec3<f32>(1.03, 1.0, 0.96), vec3<f32>(0.97, 1.0, 1.03), n_b);
    let grain = mix(vec3<f32>(1.0), pap, 0.95 - 0.5 * flat_w);
    var land_col = biome * grain;
    land_col = mix(land_col, mix(land_col, P.paper.rgb, 0.25), flat_w); // 古地圖化：淡彩平塗

    // 4 水：海深五階（淺棚 -> 深淵）柔和銜接；棚緣一條極淡的等深墨線。
    // 8 位元水深在緩坡上會出現階梯狀色帶：加入 ±0.5/255 的像素抖動打散。
    let dither = (hash3(vec3<i32>(vec2<i32>(in.clip.xy), 7)) - 0.5) / 255.0;
    let sea_t = 0.92 * pow(clamp(depth + dither, 0.0, 1.0), 0.55);
    var water_col = sea_ramp(sea_t);
    water_col = water_col * mix(vec3<f32>(1.0), pap, 0.5) * (0.96 + 0.08 * n_a) * (0.985 + 0.03 * n_c);

    var col = mix(land_col, water_col, wmask);

    // 河流：中心填 water 色，外緣 ink-light。
    // rv 為到河緣的有號距離（紋素，內正外負；範圍 ±2），以 fwidth 換算像素：
    // 外緣至少 0.8 px 寬（遠景不斷線），內側 ~1 px 為 ink-light 外緣，其內填水色。
    let e_r = (rv - 0.5) * 4.0;
    let fw_e = max(fwidth(e_r), 1e-4);
    let r_rim = clamp((e_r + 0.4 * fw_e) / fw_e + 0.5, 0.0, 1.0);
    let r_fill = clamp((e_r - 0.9 * fw_e) / fw_e + 0.5, 0.0, 1.0);
    col = mix(col, P.ink_light.rgb, r_rim * 0.55 * (1.0 - wmask));
    col = mix(col, P.water.rgb * mix(vec3<f32>(1.0), pap, 0.4), r_fill * (1.0 - wmask));

    // 5 地圖模式疊色
    let vt = vote(wuv);
    let pid = vt.a;
    // 州界到像素的距離（格）≈ m / 1.5；換成螢幕像素：每像素的格數 = pxw。
    let fw_m = 1.5 * pxw;
    let marg = vt.m;
    let is_land_px = (1.0 - wmask) * select(0.0, 1.0, pid != NO_PROVINCE);
    let pc = pcol(pid);
    // 州界兩側各約 0.1 格不上色：像素網格的階梯被墨線與淡暈蓋住，色塊邊緣不貼格線。
    let tint = pc.a * P.sel.z * is_land_px;
    // 地圖模式疊色：先把底圖去飽和，疊上的淡彩才不混濁。
    col = mix(col, vec3<f32>(dot(col, vec3<f32>(0.2126, 0.7152, 0.0722))), 0.4 * tint);
    col = mix(col, col * pc.rgb * 1.3, tint);
    if (mode == 3) {
        col = mix(col, ramp(fv) * mix(vec3<f32>(1.0), pap, 0.5), P.map.w * (1.0 - wmask));
    }

    // 2 上墨 / 3 山脊線（8：隨高度縮放降為 0）
    let g = in.grad * hs;
    let slope = length(g);
    let ink_amt = max(smoothstep(0.30, 0.80, slope), smoothstep(0.55, 0.85, hn) * 0.8);
    let k = smoothstep(0.28, 0.72, st * 0.9 + (ink_amt - 0.35) * 0.9);
    let w_ink = smoothstep(0.05, 0.30, hs) * (1.0 - wmask);
    let ridge = smoothstep(0.10, 0.32, curv) * smoothstep(0.05, 0.25, slope);
    let ink_a = max(k * ink_amt * 0.5, ridge * (0.35 + 0.35 * st)) * w_ink;
    // 山影：光向由 uniform 指定（預設西北）。陰面染冷（花青調）、陽面微暖，色相不變灰。
    let nrm = normalize(vec3<f32>(-g.x, 1.0, -g.y));
    let ldir = normalize(P.light.xyz);
    let shade = (dot(nrm, ldir) - ldir.y) * P.light.w;
    let w_land = (1.0 - wmask) * smoothstep(0.02, 0.2, hs);
    col = mix(col, col * vec3<f32>(0.60, 0.69, 0.80), clamp(-shade * 1.7, 0.0, 0.7) * w_land);
    col = mix(col, min(col * vec3<f32>(1.07, 1.03, 0.94), vec3<f32>(1.0)), clamp(shade * 1.2, 0.0, 0.5) * w_land);
    col = mix(col, mix(P.ink_light.rgb, P.ink.rgb, 0.5), clamp(ink_a, 0.0, 1.0));

    // 棚緣等深線：極淡的 ink-light 細線（只在水面上）。
    let fw_s = max(fwidth(sea_t), 1e-4);
    col = mix(col, P.ink_light.rgb, 0.16 * px_line(sea_t - 0.42, fw_s, 1.0) * wmask);

    // 海岸雙線（ink-light）
    let c1 = px_line(a - 0.5, fw_a, 1.3);
    let c2 = px_line(a - (0.5 - min(3.5 * fw_a, 0.45)), fw_a, 1.0) * 0.55;
    col = mix(col, P.ink_light.rgb, max(c1 * 0.9, c2));

    // 6 邊界
    // 邊線的位置來自 CPU 預先算好的「餘裕」場（第一名與第二名州郡權重差，邊界上為 0，C1 連續），
    // 以 fwidth 換算像素距離，得到子像素精度、無階梯的抗鋸齒墨線；州郡 ID 只用來判斷是否為國界。
    let has_b = vt.b != NO_PROVINCE;
    let realm_diff = has_b && (any(pcol(vt.a).rgb != pcol(vt.b).rgb) || pcol(vt.a).a != pcol(vt.b).a);
    // 墨線在 62°–78° 先行淡出（比極冰晚一步接手）：州界在極點收斂，保留會形成放射狀的線束。
    let land_f = (1.0 - wmask) * (1.0 - smoothstep(0.883, 0.978, abs(d.y)));
    if (realm_diff) {
        col = mix(col, P.ink_light.rgb, 0.16 * px_line(marg, fw_m, 6.0) * land_f); // 國界外側淡色暈
    }
    col = mix(col, P.ink_light.rgb, 0.85 * px_line(marg, fw_m, 1.2) * land_f); // 州界：ink-light，1 px
    if (realm_diff) {
        col = mix(col, P.ink.rgb, 0.9 * px_line(marg, fw_m, 2.2) * land_f); // 國界：ink，2 px
    }

    // 8 停留／選取高亮
    let hov = P.sel.x;
    let sel = P.sel.y;
    let b_hot_on = has_b;
    let hot_cov = px_line(marg, fw_m, 2.6);
    if (pid != NO_PROVINCE && f32(pid) == hov) {
        col = mix(col, P.paper.rgb + vec3<f32>(0.1), 0.22 * land_f);
    }
    if (pid != NO_PROVINCE && f32(pid) == sel) {
        col = mix(col, P.gold.rgb, 0.24 * land_f);
    }
    if (b_hot_on && is_hot(vt, hov)) {
        col = mix(col, P.ink.rgb, 0.75 * hot_cov * land_f);
    }
    if (b_hot_on && is_hot(vt, sel)) {
        col = mix(col, P.gold.rgb, 0.95 * hot_cov * land_f);
    }

    // 極區：統一的極冰／海冰風格（GDD 5.3），州郡邊界、海岸線、河流、山墨在極點收斂處不再星芒。
    let ice = mix(P.terrain[6].rgb, P.water.rgb, 0.12) * mix(vec3<f32>(1.0), pap, 0.8) * (0.96 + 0.08 * n_a);
    col = mix(col, ice, pol);

    // 7 霧
    let dist = length(view.world_position.xyz - in.wpos);
    let fog = smoothstep(P.fog.x, P.fog.y, dist);
    col = mix(col, P.paper.rgb, fog * 0.85);
    // 地圖邊緣淡出為紙色（古地圖暈邊）
    // 東西環繞的世界只在南北緣淡出。
    let dx_edge = select(min(wp.x, mapd.x - wp.x), 1.0e6, P.globe.w > 0.5);
    let fade_x = 1.0 - smoothstep(0.0, P.fog.z, dx_edge);
    // 南北緣之外是「極地邊疆」：極冰向外延伸，在 FRONTIER 寬內漸漸淡成紙色（與清除色相同），
    // 畫面因此不會突然露出空白；星球上沒有邊緣，極區由極冰接手（否則極點會出現一塊亮斑）。
    let out_y = max(-wp.y, wp.y - mapd.y);
    let fade_y = smoothstep(-2.0, 0.05 * mapd.x, out_y);
    let fade = max(fade_x, fade_y) * (1.0 - morph);
    col = mix(col, mix(P.paper.rgb * pap, P.paper.rgb, fade_y), fade * mix(0.92, 1.0, fade_y));

    return vec4<f32>(col, 1.0);
}
