use ragnarok_formats::act::ActFile;

use crate::sprite::SpriteTextures;
use crate::ui_renderer::UiVertex;
use crate::{UiDrawCall, UiTextureRef};

const MSG_FRAME_CRIT: usize = 2;
const MSG_FRAME_CRITBG: usize = 3;

/// Pre-computed rendering data for a single damage number.
/// Built by the client from game-layer `DamageNumber` before calling into the renderer.
pub struct DamageNumberEntry {
    /// Screen X of the entity this number belongs to
    pub screen_x: f32,
    /// Screen Y (top of entity pick bounds)
    pub screen_y: f32,
    pub digits: Vec<u8>,
    pub digit_x_offsets: Vec<f32>,
    pub sprite_action: u8,
    pub color: [f32; 4],
    pub zoom: f32,
    pub y_offset: f32,
    pub x_offset: f32,
    pub uses_msg_sprite: bool,
    pub msg_frames: Vec<usize>,
    pub is_critical: bool,
}

pub fn render_damage_numbers<'a>(
    entries: &[DamageNumberEntry],
    num_textures: &'a SpriteTextures,
    num_act: &ActFile,
    msg_textures: Option<&'a SpriteTextures>,
    draw_calls: &mut Vec<UiDrawCall>,
    inline_textures: &mut Vec<&'a wgpu::BindGroup>,
) {
    for dmg in entries {
        let base_x = dmg.screen_x + dmg.x_offset;
        let base_y = dmg.screen_y - 10.0 - dmg.y_offset;
        let [cr, cg, cb, alpha] = dmg.color;
        let zoom = dmg.zoom;

        if dmg.uses_msg_sprite {
            let msg_tex = match msg_textures {
                Some(t) => t,
                None => continue,
            };
            for &frame_idx in &dmg.msg_frames {
                if frame_idx >= msg_tex.sizes.len() {
                    continue;
                }
                let (tw, th) = msg_tex.sizes[frame_idx];
                let sw = tw as f32 * zoom;
                let sh = th as f32 * zoom;
                let x = base_x - sw / 2.0;
                let y = base_y - sh / 2.0;

                let idx = inline_textures.len();
                inline_textures.push(&msg_tex.bind_groups[frame_idx]);
                draw_calls.push(textured_quad(x, y, sw, sh, [cr, cg, cb, alpha], idx));
            }
            continue;
        }

        let action_idx = dmg.sprite_action as usize;
        if action_idx >= num_act.actions.len() {
            continue;
        }
        let action = &num_act.actions[action_idx];

        // Critical: render critbg behind digits
        if dmg.is_critical {
            if let Some(mt) = msg_textures {
                if MSG_FRAME_CRITBG < mt.sizes.len() {
                    let (tw, th) = mt.sizes[MSG_FRAME_CRITBG];
                    let scale = 0.6 * zoom;
                    let sw = tw as f32 * scale;
                    let sh = th as f32 * scale;
                    let x = base_x - sw / 2.0;
                    let y = base_y - sh / 2.0 - 6.0;
                    let idx = inline_textures.len();
                    inline_textures.push(&mt.bind_groups[MSG_FRAME_CRITBG]);
                    draw_calls.push(textured_quad(x, y, sw, sh, [0.66, 0.66, 0.66, alpha], idx));
                }
            }
        }

        for (i, &digit) in dmg.digits.iter().enumerate() {
            let motion_idx = digit as usize;
            if motion_idx >= action.motions.len() {
                continue;
            }
            let motion = &action.motions[motion_idx];
            if motion.clips.is_empty() {
                continue;
            }
            let clip = &motion.clips[0];
            if clip.sprite_index < 0 {
                continue;
            }

            let tex_idx = if clip.sprite_type == 0 {
                clip.sprite_index as usize
            } else {
                num_textures.indexed_count + clip.sprite_index as usize
            };
            if tex_idx >= num_textures.sizes.len() {
                continue;
            }

            let (tw, th) = num_textures.sizes[tex_idx];
            let sw = tw as f32 * zoom;
            let sh = th as f32 * zoom;

            let x_offset = dmg.digit_x_offsets.get(i).copied().unwrap_or(0.0) * zoom;
            let x = base_x + x_offset - sw / 2.0;
            let y = base_y - sh / 2.0;

            let idx = inline_textures.len();
            inline_textures.push(&num_textures.bind_groups[tex_idx]);
            draw_calls.push(textured_quad(x, y, sw, sh, [cr, cg, cb, alpha], idx));
        }

        // Critical: render crit overlay on top
        if dmg.is_critical {
            if let Some(mt) = msg_textures {
                if MSG_FRAME_CRIT < mt.sizes.len() {
                    let (tw, th) = mt.sizes[MSG_FRAME_CRIT];
                    let sw = tw as f32 * zoom;
                    let sh = th as f32 * zoom;
                    let x = base_x - sw / 2.0;
                    let y = base_y - sh / 2.0;
                    let idx = inline_textures.len();
                    inline_textures.push(&mt.bind_groups[MSG_FRAME_CRIT]);
                    draw_calls.push(textured_quad(x, y, sw, sh, [cr, cg, cb, alpha], idx));
                }
            }
        }
    }
}

fn textured_quad(x: f32, y: f32, w: f32, h: f32, color: [f32; 4], inline_idx: usize) -> UiDrawCall {
    let vertices = vec![
        UiVertex { position: [x,     y],     tex_coord: [0.0, 0.0], color },
        UiVertex { position: [x + w, y],     tex_coord: [1.0, 0.0], color },
        UiVertex { position: [x + w, y + h], tex_coord: [1.0, 1.0], color },
        UiVertex { position: [x,     y + h], tex_coord: [0.0, 1.0], color },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    UiDrawCall {
        vertices,
        indices,
        texture: UiTextureRef::Inline(inline_idx),
    }
}
