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

struct PointLight {
    position: vec4<f32>,
    color_range: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(0) var ground_texture: texture_2d<f32>;
@group(1) @binding(1) var ground_sampler: sampler;
@group(2) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(2) @binding(1) var lightmap_sampler: sampler;

fn point_light_attenuation(d: f32, r: f32) -> f32 {
    let n = min(d, r) / (r + 1e-4);
    let a = saturate(1.0 - n * n);
    return a * a;
}

fn point_light_contribution(world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    let count = arrayLength(&point_lights);
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let lp = point_lights[i].position.xyz;
        let lc = point_lights[i].color_range.rgb;
        let lr = point_lights[i].color_range.a;
        if (lr <= 0.0) { continue; }
        let to_frag = world_pos - lp;
        let d = length(to_frag);
        if (d >= lr) { continue; }
        let dir = to_frag / max(d, 1e-4);
        let lambert = max(-dot(dir, normal), 0.0);
        acc += lc * lambert * point_light_attenuation(d, lr);
    }
    return acc;
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
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(ground_texture, ground_sampler, in.tex_coord);
    let lightmap = textureSample(lightmap_texture, lightmap_sampler, in.lightmap_coord);

    let n_dot_l = max(dot(normalize(in.normal), normalize(light.light_dir.xyz)), 0.0);
    let diffuse = light.diffuse_color.rgb * n_dot_l;

    var color = tex_color.rgb * lightmap.rgb * diffuse * in.color.rgb;
    color += tex_color.rgb * light.ambient_color.rgb * in.color.rgb;

    let pl = point_light_contribution(in.world_position, normalize(in.normal));
    color += tex_color.rgb * pl * in.color.rgb;

    let alpha = tex_color.a * in.color.a;

    return vec4<f32>(color, alpha);
}
