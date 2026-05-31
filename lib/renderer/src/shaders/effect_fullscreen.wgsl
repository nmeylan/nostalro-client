// Full-viewport screen-space overlay. Vertex positions arrive already in
// clip space ([-1, 1] on x/y); the camera uniform at group 0 is bound by the
// dispatcher but intentionally unused here.

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye_pos: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(1) @binding(0) var overlay_texture: texture_2d<f32>;
@group(1) @binding(1) var overlay_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position.xy, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(overlay_texture, overlay_sampler, in.tex_coord) * in.color;
    if tex_color.a < 0.01 {
        discard;
    }
    return tex_color;
}
