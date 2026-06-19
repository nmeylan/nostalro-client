use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::str_effect::{EffectLayer, StrEffectFile};

use crate::camera::Camera;
use crate::effect_sprite::project_billboard;
use crate::sprite::{SpriteBatch, SpriteVertex};
use crate::texture::{TextureCache, create_texture_bind_group};

/// Cached STR effect file with resolved texture paths per layer.
pub struct StrEffectEntry {
    pub str_file: StrEffectFile,
    /// `texture_paths[layer_idx][tex_idx]` = GRF path for that texture.
    pub texture_paths: Vec<Vec<String>>,
    /// Cloned bind groups for each texture, indexed parallel to `texture_paths`.
    /// `None` entries indicate the texture failed to load.
    pub bind_groups: Vec<Vec<Option<wgpu::BindGroup>>>,
}

/// Caches loaded STR effect files and their black-keyed textures.
/// Textures are loaded directly (not via [`TextureCache`]) so we can convert
/// the all-black background pixels into transparent ones, matching the
/// original game's additive-blend STR effect rendering.
pub struct StrEffectCache {
    entries: HashMap<String, StrEffectEntry>,
    /// Names that failed to resolve once. Re-requesting one returns `false`
    /// without retrying the load or re-emitting the warning — stress spawns and
    /// per-frame respawns of asset-missing effects would otherwise flood the log.
    missing: std::collections::HashSet<String>,
}

impl Default for StrEffectCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StrEffectCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            missing: std::collections::HashSet::new(),
        }
    }

    pub fn load(
        &mut self,
        name: &str,
        aliases: &[&str],
        grf: &GrfArchive,
        texture_cache: &mut TextureCache,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> bool {
        if self.entries.contains_key(name) {
            return true;
        }
        if self.missing.contains(name) {
            return false;
        }
        // Try the primary first, then each alias. The cache keys by primary —
        // so once any candidate resolves, future lookups by primary find it.
        let mut last_err: Option<String> = None;
        for candidate in std::iter::once(name).chain(aliases.iter().copied()) {
            match try_load(candidate, grf, texture_cache, device, queue) {
                Ok(entry) => {
                    if candidate != name {
                        tracing::debug!("STR resolved via alias: {name} -> {candidate}");
                    }
                    self.entries.insert(name.to_string(), entry);
                    return true;
                }
                Err(e) => last_err = Some(e),
            }
        }
        tracing::warn!(
            "STR file missing (tried {} name(s)): {} ({})",
            1 + aliases.len(),
            name,
            last_err.unwrap_or_default()
        );
        self.missing.insert(name.to_string());
        false
    }

    pub fn get(&self, name: &str) -> Option<&StrEffectEntry> {
        self.entries.get(name)
    }
}

fn try_load(
    name: &str,
    grf: &GrfArchive,
    texture_cache: &mut TextureCache,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<StrEffectEntry, String> {
    let str_path = format!("data/texture/effect/{name}.str");
    let str_bytes = grf
        .read_file(&str_path)
        .map_err(|e| format!("{str_path}: {e}"))?;
    let str_file = StrEffectFile::parse(&str_bytes).map_err(|e| format!("parse: {e}"))?;

    let layout = &texture_cache.bind_group_layout;
    let mut texture_paths = Vec::with_capacity(str_file.layers.len());
    let mut bind_groups = Vec::with_capacity(str_file.layers.len());
    for layer in &str_file.layers {
        let mut paths = Vec::with_capacity(layer.textures.len());
        let mut bgs: Vec<Option<wgpu::BindGroup>> = Vec::with_capacity(layer.textures.len());
        for tex_name in &layer.textures {
            let tex_path = format!("data/texture/effect/{tex_name}");
            let bg = load_str_texture(&tex_path, grf, device, queue, layout);
            bgs.push(bg);
            paths.push(tex_path);
        }
        texture_paths.push(paths);
        bind_groups.push(bgs);
    }

    Ok(StrEffectEntry {
        str_file,
        texture_paths,
        bind_groups,
    })
}

fn load_str_texture(
    path: &str,
    grf: &GrfArchive,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> Option<wgpu::BindGroup> {
    let data = match grf.read_file(path) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("STR texture missing: {path} ({e})");
            return None;
        }
    };

    let img = match image::load_from_memory(&data) {
        Ok(i) => i,
        Err(_) => {
            let fmt = if path.ends_with(".tga") {
                image::ImageFormat::Tga
            } else if path.ends_with(".bmp") {
                image::ImageFormat::Bmp
            } else if path.ends_with(".png") {
                image::ImageFormat::Png
            } else {
                tracing::warn!("STR texture decode failed: {path}");
                return None;
            };
            match image::load_from_memory_with_format(&data, fmt) {
                Ok(i) => i,
                Err(e) => {
                    tracing::warn!("STR texture decode failed: {path} ({e})");
                    return None;
                }
            }
        }
    };

    let mut rgba = img.to_rgba8();
    // STR textures use two transparency conventions:
    //   - magenta (FF00FF) for BMP color-keyed pixels (RO convention)
    //   - pure black for additive-blend layers (also key off black so the
    //     background doesn't add brightness)
    ragnarok_formats::apply_magenta_transparency(rgba.as_mut());
    for px in rgba.pixels_mut() {
        if px[0] == 0 && px[1] == 0 && px[2] == 0 {
            px[3] = 0;
        }
    }

    Some(create_texture_bind_group(
        device, queue, &rgba, layout, path,
    ))
}

struct LayerAnim {
    visible: bool,
    texture_index: usize,
    positions: [f32; 8],
    color: [f32; 4],
    angle: f32,
    offset: [f32; 2],
    /// D3DBLEND source factor for this frame. Unused today - the sprite
    /// pipeline's two blend states only key off `blend_dst`. Wired when we
    /// add a dedicated effect-primitive pipeline that honors both factors.
    #[allow(dead_code)]
    blend_src: i32,
    /// D3DBLEND destination factor for this frame. `6` (D3DBLEND_INVSRCALPHA)
    /// triggers alpha blending; anything else stays additive.
    blend_dst: i32,
}

/// STR frame `angle` (`rz`) is a brand angle: 1024 units is one full turn, not
/// a degree value. The original game converts it with `DegToRad(rz / (1024/360))`.
fn str_angle_to_radians(raw: f32) -> f32 {
    raw * std::f32::consts::TAU / 1024.0
}

fn calculate_layer_anim(layer: &EffectLayer, key_index: f32) -> LayerAnim {
    let invisible = LayerAnim {
        visible: false,
        texture_index: 0,
        positions: [0.0; 8],
        color: [1.0; 4],
        angle: 0.0,
        offset: [0.0; 2],
        blend_src: 5,
        blend_dst: 2,
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
            if f.frame_type == 0 {
                from_id = Some(i);
            }
            if f.frame_type == 1 {
                to_id = Some(i);
            }
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

    let has_morph =
        to_id.is_some_and(|ti| ti == from_idx + 1 && frames[ti].frame_index == from.frame_index);

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
            blend_src: from.blend_src,
            blend_dst: from.blend_dst,
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
        blend_src: from.blend_src,
        blend_dst: from.blend_dst,
    }
}

/// Description of one STR emitter to render.
pub struct StrEmitterInput<'a> {
    pub str_name: &'a str,
    pub position: [f32; 3],
    pub anim_time: f32,
}

/// Build sprite batches for STR effects projected to screen space.
///
/// `zoom` is the map's world-units-per-cell (from `MapCoordinates::zoom`).
/// Callers without a map (e.g. standalone effect previewers) can pass `10.0`,
/// the conventional default.
pub fn build_str_effect_batches<'a>(
    emitters: &[StrEmitterInput<'_>],
    cache: &'a StrEffectCache,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
    zoom: f32,
) -> Vec<SpriteBatch<'a>> {
    let mut batches = Vec::new();

    for input in emitters {
        let Some(entry) = cache.get(input.str_name) else {
            continue;
        };
        let str_file = &entry.str_file;

        let Some((anchor, depth, ppu)) =
            project_billboard(camera, input.position, screen_w, screen_h)
        else {
            continue;
        };

        let key_index = input.anim_time * str_file.fps as f32;
        if str_file.max_key > 0 && key_index >= str_file.max_key as f32 {
            continue;
        }

        for (layer_idx, layer) in str_file.layers.iter().enumerate() {
            let anim = calculate_layer_anim(layer, key_index);
            if !anim.visible {
                continue;
            }

            let layer_bgs = &entry.bind_groups[layer_idx];
            if anim.texture_index >= layer_bgs.len() {
                continue;
            }

            let Some(texture) = layer_bgs[anim.texture_index].as_ref() else {
                continue;
            };

            let color = [
                (anim.color[0] / 255.0).clamp(0.0, 1.0),
                (anim.color[1] / 255.0).clamp(0.0, 1.0),
                (anim.color[2] / 255.0).clamp(0.0, 1.0),
                (anim.color[3] / 255.0).clamp(0.0, 1.0),
            ];
            if color[3] < 0.01 {
                continue;
            }

            // Rotation is NOT negated: STR layer offsets are placed in a
            // screen-down Y space (offset_y below is not negated either), the
            // same convention the original game rotates in, so its formula maps
            // directly. Negating spun multi-direction effects (e.g. the
            // criticalwound claws) outward instead of crossing at centre.
            let angle_rad = str_angle_to_radians(anim.angle);
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();

            // Match `sprite_projection::project_entity_screen`'s `ppu * zoom / 75`
            // so STR effects and entity sprites share a world-units-per-pixel ratio.
            // The `/35` reference (one cell = 35 STR pixels) is encoded by the
            // sprite pipeline's `/75` divisor + the sprite art's authored size.
            let scale = ppu * zoom / 75.0;
            // Layer offset is a screen-space pixel position on the original
            // game's 640x480 reference frame. X centres on 320; Y uses 320
            // too, which folds the 240 frame centre together with the original
            // dispatcher's default -80 vertical lift (it anchors the clip 80px
            // above the actor's projected centre) that our emitter omits.
            let offset_x = (anim.offset[0] - 320.0) * scale;
            let offset_y = (anim.offset[1] - 320.0) * scale;

            // xy layout: [x0,x1,x2,x3, y0,y1,y2,y3]
            // vertex order: [0]=TL, [1]=TR, [2]=BR, [3]=BL
            // triangle strip → two triangles
            let xy = &anim.positions;
            let corners = [
                (xy[0], xy[4]), // TL
                (xy[1], xy[5]), // TR
                (xy[3], xy[7]), // BL
                (xy[2], xy[6]), // BR
            ];
            let uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

            let mut vertices = Vec::with_capacity(4);
            for (i, &(px, py)) in corners.iter().enumerate() {
                let sx = px * scale;
                let sy = py * scale;
                let rx = sx * cos_a - sy * sin_a;
                let ry = sx * sin_a + sy * cos_a;
                vertices.push(SpriteVertex {
                    position: [
                        anchor[0] + rx + offset_x,
                        anchor[1] + ry + offset_y,
                        depth - 0.001,
                    ],
                    tex_coord: uvs[i],
                    color,
                });
            }

            // Per-frame D3D blend factor → SpriteBatch's binary additive/alpha
            // toggle. The sprite renderer today only exposes two pipelines:
            //   * additive: (SrcAlpha, One)
            //   * alpha:    (SrcAlpha, OneMinusSrcAlpha)
            // D3DBLEND_INVSRCALPHA = 6, which is the giveaway for normal alpha
            // blending. Anything else collapses to additive (glow). When we
            // grow a dedicated effect-primitive pipeline (Phase C-2) we'll
            // honor the full (src, dst) pair.
            let additive = anim.blend_dst != 6;
            batches.push(SpriteBatch {
                vertices,
                indices: vec![0, 1, 2, 1, 3, 2],
                texture,
                additive,
            });
        }
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::str_angle_to_radians;

    #[test]
    fn str_angle_decodes_brand_units_not_degrees() {
        // 1024 units == one full turn.
        assert!((str_angle_to_radians(1024.0) - std::f32::consts::TAU).abs() < 1e-4);
        assert!((str_angle_to_radians(256.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        // criticalwound's claw layers: 85.33 / -156.44 / -312.89 -> 30 / -55 / -110 deg.
        let deg = |r: f32| r.to_degrees();
        assert!((deg(str_angle_to_radians(85.333)) - 30.0).abs() < 0.5);
        assert!((deg(str_angle_to_radians(-156.444)) + 55.0).abs() < 0.5);
        assert!((deg(str_angle_to_radians(-312.889)) + 110.0).abs() < 0.5);
    }
}
