struct Ripple {
    base_phase: f32,
    span_phase: f32,
    amplitude: f32,
    _pad: f32,
};

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> ripple: Ripple;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let corner = corners[index];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(corner, 0.0, 1.0);
    out.uv = vec2<f32>((corner.x + 1.0) * 0.5, (1.0 - corner.y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let phase = ripple.base_phase + ripple.span_phase * (1.0 - in.uv.y);
    let shift = sin(phase) * ripple.amplitude;
    let uv = vec2<f32>(clamp(in.uv.x - shift, 0.0, 1.0), in.uv.y);
    return textureSample(scene, scene_sampler, uv);
}
