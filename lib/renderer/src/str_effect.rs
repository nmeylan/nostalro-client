use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::str_effect::{EffectLayer, StrEffectFile};

use crate::camera::Camera;
use crate::effect_sprite::project_billboard;
use crate::sprite::{SpriteBatch, SpriteVertex};
use crate::texture::TextureCache;

/// Cached STR effect file with resolved texture paths per layer.
pub struct StrEffectEntry {
    pub str_file: StrEffectFile,
    /// `texture_paths[layer_idx][tex_idx]` = GRF path for that texture.
    pub texture_paths: Vec<Vec<String>>,
}

/// Caches loaded STR effect files and preloads their textures into
/// [`TextureCache`].
pub struct StrEffectCache {
    entries: HashMap<String, StrEffectEntry>,
}

impl Default for StrEffectCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StrEffectCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    pub fn load(
        &mut self,
        name: &str,
        grf: &GrfArchive,
        texture_cache: &mut TextureCache,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> bool {
        if self.entries.contains_key(name) {
            return true;
        }

        let str_path = format!("data/texture/effect/{name}.str");
        let str_bytes = match grf.read_file(&str_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("STR file missing: {str_path} ({e})");
                return false;
            }
        };
        let str_file = match StrEffectFile::parse(&str_bytes) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("STR parse failed: {str_path} ({e})");
                return false;
            }
        };

        let mut texture_paths = Vec::with_capacity(str_file.layers.len());
        for layer in &str_file.layers {
            let mut paths = Vec::with_capacity(layer.textures.len());
            for tex_name in &layer.textures {
                let tex_path = format!("data/texture/effect/{tex_name}");
                texture_cache.get_or_load(&tex_path, grf, device, queue, false);
                paths.push(tex_path);
            }
            texture_paths.push(paths);
        }

        self.entries.insert(name.to_string(), StrEffectEntry {
            str_file,
            texture_paths,
        });
        true
    }

    pub fn get(&self, name: &str) -> Option<&StrEffectEntry> {
        self.entries.get(name)
    }
}

struct LayerAnim {
    visible: bool,
    texture_index: usize,
    positions: [f32; 8],
    color: [f32; 4],
    angle: f32,
    offset: [f32; 2],
}

fn calculate_layer_anim(layer: &EffectLayer, key_index: f32) -> LayerAnim {
    let invisible = LayerAnim {
        visible: false, texture_index: 0, positions: [0.0; 8],
        color: [1.0; 4], angle: 0.0, offset: [0.0; 2],
    };

    let frames = &layer.frames;
    if frames.is_empty() {
        return invisible;
    }

    let mut from_id: Option<usize> = None;
    let mut to_id: Option<usize> = None;
    let mut last_frame = 0.0f32;
    let mut last_source = 0.0f32;

    for (i, f) in frames.iter().enumerate() {
        if f.frame_index as f32 <= key_index {
            if f.frame_type == 0 { from_id = Some(i); }
            if f.frame_type == 1 { to_id = Some(i); }
        }
        last_frame = last_frame.max(f.frame_index as f32);
        if f.frame_type == 0 {
            last_source = last_source.max(f.frame_index as f32);
        }
    }

    let Some(from_idx) = from_id else {
        return invisible;
    };

    if to_id.is_none() && last_frame < key_index {
        return invisible;
    }

    let from = &frames[from_idx];

    let has_morph = to_id
        .is_some_and(|ti| ti == from_idx + 1 && frames[ti].frame_index == from.frame_index);

    if !has_morph {
        if to_id.is_some() && last_source <= from.frame_index as f32 {
            return invisible;
        }

        return LayerAnim {
            visible: true,
            texture_index: from.texture_index as usize,
            positions: from.positions,
            color: from.color,
            angle: from.angle,
            offset: from.offset,
        };
    }

    let to = &frames[to_id.unwrap()];
    let delta = key_index - from.frame_index as f32;

    let mut color = [0f32; 4];
    for i in 0..4 {
        color[i] = from.color[i] + to.color[i] * delta;
    }

    let mut positions = [0f32; 8];
    for i in 0..8 {
        positions[i] = from.positions[i] + to.positions[i] * delta;
    }

    let angle = from.angle + to.angle * delta;
    let offset = [
        from.offset[0] + to.offset[0] * delta,
        from.offset[1] + to.offset[1] * delta,
    ];

    let tex_count = layer.textures.len().max(1);
    let anim_frame = match to.animation_mode {
        1 => (from.texture_index + to.texture_index * delta) as usize,
        2 => ((from.texture_index + to.delay * delta) as usize).min(tex_count - 1),
        3 => (from.texture_index + to.delay * delta) as usize % tex_count,
        4 => {
            let raw = (from.texture_index - to.delay * delta) as i32;
            ((raw % tex_count as i32 + tex_count as i32) % tex_count as i32) as usize
        }
        _ => 0,
    };

    LayerAnim {
        visible: true,
        texture_index: anim_frame,
        positions,
        color,
        angle,
        offset,
    }
}

/// Description of one STR emitter to render.
pub struct StrEmitterInput<'a> {
    pub str_name: &'a str,
    pub position: [f32; 3],
    pub anim_time: f32,
}

/// Build sprite batches for STR effects projected to screen space.
pub fn build_str_effect_batches<'a>(
    emitters: &[StrEmitterInput<'_>],
    cache: &'a StrEffectCache,
    texture_cache: &'a TextureCache,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Vec<SpriteBatch<'a>> {
    let mut batches = Vec::new();
    let pixel_ratio = 1.0 / 35.0;

    for input in emitters {
        let Some(entry) = cache.get(input.str_name) else { continue };
        let str_file = &entry.str_file;

        let Some((anchor, depth, ppu_scale)) =
            project_billboard(camera, input.position, screen_w, screen_h)
        else { continue };

        let key_index = input.anim_time * str_file.fps as f32;
        let key_index = if str_file.max_key > 0 {
            key_index % str_file.max_key as f32
        } else {
            key_index
        };

        for (layer_idx, layer) in str_file.layers.iter().enumerate() {
            let anim = calculate_layer_anim(layer, key_index);
            if !anim.visible { continue; }

            let layer_paths = &entry.texture_paths[layer_idx];
            if anim.texture_index >= layer_paths.len() { continue; }

            let tex_path = &layer_paths[anim.texture_index];
            let Some(texture) = texture_cache.get(tex_path) else { continue };

            if anim.color[3] < 0.01 { continue; }

            let angle_rad = -anim.angle * std::f32::consts::PI / 180.0;
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();

            let offset_x = (anim.offset[0] - 320.0) * pixel_ratio * ppu_scale;
            let offset_y = -(anim.offset[1] - 320.0) * pixel_ratio * ppu_scale;

            // xy layout: [x0,x1,x2,x3, y0,y1,y2,y3]
            // robrowser vertex order: [0]=TL, [1]=TR, [2]=BR, [3]=BL
            // triangle strip → two triangles
            let xy = &anim.positions;
            let corners = [
                (xy[0], xy[4]),  // TL
                (xy[1], xy[5]),  // TR
                (xy[3], xy[7]),  // BL
                (xy[2], xy[6]),  // BR
            ];
            let uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

            let mut vertices = Vec::with_capacity(4);
            for (i, &(px, py)) in corners.iter().enumerate() {
                let sx = px * pixel_ratio;
                let sy = -py * pixel_ratio;
                let rx = sx * cos_a - sy * sin_a;
                let ry = sx * sin_a + sy * cos_a;
                vertices.push(SpriteVertex {
                    position: [
                        anchor[0] + (rx + offset_x) * ppu_scale,
                        anchor[1] + (ry + offset_y) * ppu_scale,
                        depth - 0.001,
                    ],
                    tex_coord: uvs[i],
                    color: anim.color,
                });
            }

            batches.push(SpriteBatch {
                vertices,
                indices: vec![0, 1, 2, 1, 3, 2],
                texture,
            });
        }
    }
    batches
}
