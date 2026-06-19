struct CameraUniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye_pos: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_xz: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.world_xz = in.position.xz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let cell: f32 = 50.0;
    let ix = floor(in.world_xz.x / cell);
    let iz = floor(in.world_xz.y / cell);
    let chk = (ix + iz) - 2.0 * floor((ix + iz) * 0.5);
    let dark = vec3<f32>(0.18, 0.18, 0.20);
    let light = vec3<f32>(0.30, 0.30, 0.32);
    let color = mix(dark, light, chk);
    return vec4<f32>(color, 1.0);
}
