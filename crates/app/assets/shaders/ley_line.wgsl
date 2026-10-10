// GDD 5.4 靈脈模式額外圖層：靈脈線（流動光點）與洞天標記。9.4：色值由 uniform 傳入。
#import bevy_pbr::{
    mesh_functions::{get_world_from_local, mesh_position_local_to_world},
    view_transformations::position_world_to_clip,
}

struct LeyParams {
    cyan: vec4<f32>,
    gold: vec4<f32>,
    ink: vec4<f32>,
    view: vec4<f32>, // x 高度縮放, y 寬度, z 時間, w 模式（0 線帶, 1 圓盤）
};

@group(2) @binding(0) var<uniform> L: LeyParams;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>, // 側向（線帶）或向外（圓盤）單位向量，水平
    @location(2) uv: vec2<f32>,     // y：側向／半徑倍率
    @location(3) uv_b: vec2<f32>,   // 線帶：(側向 -1..1, 沿線長度)；圓盤：(半徑比 0..1, 0)
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv_b: vec2<f32>,
};

@vertex
fn vertex(v: Vertex) -> VOut {
    var o: VOut;
    var p = v.position;
    p.y = p.y * L.view.x + 0.25;
    p = p + vec3<f32>(v.normal.x, 0.0, v.normal.z) * v.uv.y * L.view.y;
    let wp = mesh_position_local_to_world(get_world_from_local(v.instance_index), vec4<f32>(p, 1.0));
    o.clip = position_world_to_clip(wp.xyz);
    o.uv_b = v.uv_b;
    return o;
}

@fragment
fn fragment(in: VOut) -> @location(0) vec4<f32> {
    let time = L.view.z;
    if (L.view.w > 0.5) {
        let r = in.uv_b.x;
        let ring = smoothstep(0.66, 0.74, r);
        let col = mix(L.gold.rgb, L.ink.rgb, ring * 0.85);
        let alpha = 1.0 - smoothstep(0.93, 1.0, r);
        return vec4<f32>(col, alpha);
    }
    let across = abs(in.uv_b.x);
    let core = 1.0 - smoothstep(0.25, 1.0, across);
    // 沿線流動的光點
    let ph = fract(in.uv_b.y / 7.0 - time * 0.25);
    let dot_ = smoothstep(0.0, 0.05, ph) * (1.0 - smoothstep(0.10, 0.22, ph));
    let col = mix(L.cyan.rgb, L.gold.rgb, dot_);
    return vec4<f32>(col, core * (0.6 + 0.4 * dot_));
}
