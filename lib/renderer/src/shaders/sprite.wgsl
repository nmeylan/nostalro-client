struct SpriteUniforms {
    screen_size: vec2<f32>,
    zoom: f32,
    _pad: f32,
    pan: vec2<f32>,
    _pad2: vec2<f32>,
    world_light: vec4<f32>,
    fog_color: vec4<f32>,
    fog_near: f32,
    fog_far: f32,
    fog_enabled: f32,
    _pad3: f32,
    clip_near: f32,
    clip_far: f32,
    _pad4: vec2<f32>,
};

@group(0) @binding(0) var<uniform> sprite: SpriteUniforms;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// Inverts ndc_z = far * (d - near) / (d * (far - near)).
fn eye_distance(ndc_z: f32) -> f32 {
    let near = sprite.clip_near;
    let far = sprite.clip_far;
    return (far * near) / max(far - ndc_z * (far - near), 1e-4);
}

fn apply_fog(color: vec3<f32>, ndc_z: f32) -> vec3<f32> {
    if (sprite.fog_enabled <= 0.0) {
        return color;
    }
    let fog_amount = clamp(
        (eye_distance(ndc_z) - sprite.fog_near) / (sprite.fog_far - sprite.fog_near),
        0.0,
        1.0,
    );
    return mix(color, sprite.fog_color.rgb, fog_amount);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) ndc_z: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let pos = (in.position.xy + sprite.pan) * sprite.zoom;
    let ndc_x = pos.x / sprite.screen_size.x * 2.0 - 1.0;
    let ndc_y = 1.0 - pos.y / sprite.screen_size.y * 2.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, in.position.z, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    out.ndc_z = in.position.z;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(sprite_texture, sprite_sampler, in.tex_coord);
    if tex.a < 0.01 || in.color.a <= 0.0 {
        discard;
    }
    let tex_color = tex * in.color;
    let lit = tex_color.rgb * sprite.world_light.rgb;
    return vec4<f32>(apply_fog(lit, in.ndc_z), tex_color.a);
}
