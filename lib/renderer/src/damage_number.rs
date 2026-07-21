use ragnarok_formats::damage_number::{DamageNumberQuad, TextureSource};

use crate::sprite::SpriteTextures;
use crate::ui_renderer::UiVertex;
use crate::{UiDrawCall, UiTextureRef};

pub fn render_damage_number_quads<'a>(
    quads: &[DamageNumberQuad],
    num_textures: &'a SpriteTextures,
    msg_textures: Option<&'a SpriteTextures>,
    draw_calls: &mut Vec<UiDrawCall>,
    inline_textures: &mut Vec<&'a wgpu::BindGroup>,
) {
    for quad in quads {
        let bind_group = match quad.source {
            TextureSource::Number => {
                if quad.tex_idx >= num_textures.bind_groups.len() {
                    continue;
                }
                &num_textures.bind_groups[quad.tex_idx]
            }
            TextureSource::Message => {
                let mt = match msg_textures {
                    Some(t) => t,
                    None => continue,
                };
                if quad.tex_idx >= mt.bind_groups.len() {
                    continue;
                }
                &mt.bind_groups[quad.tex_idx]
            }
        };
        let idx = inline_textures.len();
        inline_textures.push(bind_group);
        draw_calls.push(textured_quad(
            quad.x, quad.y, quad.w, quad.h, quad.color, idx,
        ));
    }
}

fn textured_quad(x: f32, y: f32, w: f32, h: f32, color: [f32; 4], inline_idx: usize) -> UiDrawCall {
    let vertices = vec![
        UiVertex {
            position: [x, y],
            tex_coord: [0.0, 0.0],
            color,
        },
        UiVertex {
            position: [x + w, y],
            tex_coord: [1.0, 0.0],
            color,
        },
        UiVertex {
            position: [x + w, y + h],
            tex_coord: [1.0, 1.0],
            color,
        },
        UiVertex {
            position: [x, y + h],
            tex_coord: [0.0, 1.0],
            color,
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    UiDrawCall {
        vertices,
        indices,
        texture: UiTextureRef::Inline(inline_idx),
    }
}
