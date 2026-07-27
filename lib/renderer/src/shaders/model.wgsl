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

struct FogUniforms {
    color: vec4<f32>,
    near: f32,
    far: f32,
    factor: f32,
    enabled: f32,
};

struct CellLightUniforms {
    cell_size: f32,
    width: f32,
    height: f32,
    enabled: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(0) @binding(3) var<uniform> fog: FogUniforms;
@group(0) @binding(4) var cell_light_texture: texture_2d<f32>;
@group(0) @binding(5) var cell_light_sampler: sampler;
@group(0) @binding(6) var<uniform> cell_light: CellLightUniforms;
@group(1) @binding(0) var model_texture: texture_2d<f32>;
@group(1) @binding(1) var model_sampler: sampler;

fn cell_light_at(world_pos: vec3<f32>) -> vec3<f32> {
    if (cell_light.enabled <= 0.0) {
        return vec3<f32>(0.0);
    }
    let cell = floor(vec2<f32>(world_pos.x, world_pos.z) / cell_light.cell_size);
    let dims = vec2<f32>(cell_light.width, cell_light.height);
    if (any(cell < vec2<f32>(0.0)) || any(cell >= dims)) {
        return vec3<f32>(0.0);
    }
    return textureSample(cell_light_texture, cell_light_sampler, (cell + 0.5) / dims).rgb;
}

fn apply_fog(color: vec3<f32>, view_z: f32) -> vec3<f32> {
    if (fog.enabled <= 0.0) {
        return color;
    }
    let depth = abs(view_z);
    let fog_amount = smoothstep(fog.near, fog.far, depth);
    return mix(color, fog.color.rgb, fog_amount);
}

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
    @location(3) alpha: f32,
    @location(4) lit_scale: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) alpha: f32,
    @location(3) world_position: vec3<f32>,
    @location(4) view_z: f32,
    @location(5) lit_scale: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.tex_coord = in.tex_coord;
    out.normal = in.normal;
    out.alpha = in.alpha;
    out.world_position = in.position;
    out.lit_scale = in.lit_scale;
    let view_pos = camera.view * vec4<f32>(in.position, 1.0);
    out.view_z = view_pos.z;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(model_texture, model_sampler, in.tex_coord);

    if tex_color.a < 0.01 {
        discard;
    }

    let n_dot_l = max(dot(normalize(in.normal), normalize(light.light_dir.xyz)), 0.0);
    let diffuse = light.diffuse_color.rgb * n_dot_l;
    let lighting = diffuse + light.ambient_color.rgb;

    var color = tex_color.rgb * (lighting * in.lit_scale + cell_light_at(in.world_position));
    let pl = point_light_contribution(in.world_position, normalize(in.normal));
    color += tex_color.rgb * pl;

    color = apply_fog(color, in.view_z);

    return vec4<f32>(color, tex_color.a * in.alpha);
}
