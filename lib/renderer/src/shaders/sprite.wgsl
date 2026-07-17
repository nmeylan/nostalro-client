struct SpriteUniforms {
    screen_size: vec2<f32>,
    zoom: f32,
    _pad: f32,
    pan: vec2<f32>,
    _pad2: vec2<f32>,
    world_light: vec4<f32>,
};

@group(0) @binding(0) var<uniform> sprite: SpriteUniforms;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

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
    let pos = (in.position.xy + sprite.pan) * sprite.zoom;
    let ndc_x = pos.x / sprite.screen_size.x * 2.0 - 1.0;
    let ndc_y = 1.0 - pos.y / sprite.screen_size.y * 2.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, in.position.z, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(sprite_texture, sprite_sampler, in.tex_coord) * in.color;
    if tex_color.a < 0.01 {
        discard;
    }
    return vec4<f32>(tex_color.rgb * sprite.world_light.rgb, tex_color.a);
}
