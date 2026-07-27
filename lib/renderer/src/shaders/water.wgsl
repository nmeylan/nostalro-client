struct CameraUniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye_pos: vec4<f32>,
};

struct WaterUniforms {
    wave_height: f32,
    wave_pitch_per_unit: f32,
    wave_offset: f32,
    opacity: f32,
    ambient_tint: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};

struct PointLight {
    position: vec4<f32>,
    color_range: vec4<f32>,
};

struct FogUniforms {
    color: vec4<f32>,
    near: f32,
    far: f32,
    factor: f32,
    enabled: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(2) var<storage, read> _point_lights: array<PointLight>;
@group(0) @binding(3) var<uniform> fog: FogUniforms;
@group(1) @binding(0) var water_texture: texture_2d<f32>;
@group(1) @binding(1) var water_sampler: sampler;
@group(2) @binding(0) var<uniform> water: WaterUniforms;

struct LightUniforms {
    light_dir: vec4<f32>,
    diffuse_color: vec4<f32>,
    ambient_color: vec4<f32>,
    shadow_strength: f32,
};

fn apply_fog(color: vec3<f32>, view_z: f32) -> vec3<f32> {
    if (fog.enabled <= 0.0) {
        return color;
    }
    let depth = abs(view_z);
    let fog_amount = smoothstep(fog.near, fog.far, depth);
    return mix(color, fog.color.rgb, fog_amount);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) view_z: f32,
};

const PI: f32 = 3.14159265;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var pos = in.position;
    let phase = water.wave_offset + water.wave_pitch_per_unit * (in.position.x + in.position.z);
    pos.y += sin(PI / 180.0 * phase) * water.wave_height;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.tex_coord = in.tex_coord;
    let view_pos = camera.view * vec4<f32>(pos, 1.0);
    out.view_z = view_pos.z;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(water_texture, water_sampler, in.tex_coord);
    let tinted = mix(tex_color.rgb, tex_color.rgb * light.ambient_color.rgb, water.ambient_tint);
    let fogged = apply_fog(tinted, in.view_z);
    return vec4<f32>(fogged, water.opacity);
}
