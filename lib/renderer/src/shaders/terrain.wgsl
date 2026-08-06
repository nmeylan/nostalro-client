struct CameraUniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye_pos: vec4<f32>,
};

struct LightUniforms {
    light_dir: vec4<f32>,
    diffuse_color: vec4<f32>,
    ambient_color: vec4<f32>,
    shadow_strength: f32,
};

struct FogUniforms {
    color: vec4<f32>,
    near: f32,
    far: f32,
    enabled: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(3) var<uniform> fog: FogUniforms;
@group(1) @binding(0) var ground_texture: texture_2d<f32>;
@group(1) @binding(1) var ground_sampler: sampler;
@group(2) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(2) @binding(1) var lightmap_sampler: sampler;

fn apply_fog(color: vec3<f32>, view_pos: vec3<f32>) -> vec3<f32> {
    if (fog.enabled <= 0.0) {
        return color;
    }
    let fog_amount = clamp(
        (length(view_pos) - fog.near) / (fog.far - fog.near),
        0.0,
        1.0,
    );
    return mix(color, fog.color.rgb, fog_amount);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) lightmap_coord: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) lightmap_coord: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) world_position: vec3<f32>,
    @location(5) view_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.tex_coord = in.tex_coord;
    out.lightmap_coord = in.lightmap_coord;
    out.normal = in.normal;
    out.color = in.color;
    out.world_position = in.position;
    let view_pos = camera.view * vec4<f32>(in.position, 1.0);
    out.view_pos = view_pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(ground_texture, ground_sampler, in.tex_coord);
    let lightmap = textureSample(lightmap_texture, lightmap_sampler, in.lightmap_coord);

    let sunlight = light.diffuse_color.rgb;
    let ambient = light.ambient_color.rgb;

    let n_dot_l = max(dot(normalize(in.normal), normalize(light.light_dir.xyz)), 0.0);
    let shadow = lightmap.a;
    let combined_light = (sunlight * n_dot_l + ambient) * shadow;

    let contrast_correction = clamp(ambient + sunlight - sunlight * ambient, vec3<f32>(0.0), vec3<f32>(1.0));

    var color = clamp(
        in.color.rgb * contrast_correction * combined_light * tex_color.rgb + lightmap.rgb,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );

    color = apply_fog(color, in.view_pos);

    let alpha = tex_color.a * in.color.a;

    return vec4<f32>(color, alpha);
}
