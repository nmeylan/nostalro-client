struct UiUniforms {
    screen_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> ui: UiUniforms;
@group(1) @binding(0) var ui_texture: texture_2d<f32>;
@group(1) @binding(1) var ui_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
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
    let ndc_x = in.position.x / ui.screen_size.x * 2.0 - 1.0;
    let ndc_y = 1.0 - in.position.y / ui.screen_size.y * 2.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(ui_texture, ui_sampler, in.tex_coord) * in.color;
}
