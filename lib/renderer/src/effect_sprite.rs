use std::collections::HashMap;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::spr::SprFile;

use crate::camera::Camera;
use crate::sprite::{SpriteBatch, SpriteTextures, build_clip_quad, scale_clip_vertices, upload_sprite_textures};

/// Loaded effect sprite (SPR + ACT + GPU-uploaded textures), shared across
/// all emitters that point at the same `sprite_path`.
pub struct EffectSpriteEntry {
    pub textures: SpriteTextures,
    pub act: ActFile,
}

/// Caches effect sprites by GRF path key (without extension). Loaded lazily
/// on first request and held for the life of the cache.
pub struct EffectSpriteCache {
    entries: HashMap<String, EffectSpriteEntry>,
}

impl Default for EffectSpriteCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectSpriteCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// Load `<path>.spr` + `<path>.act` from the GRF and upload textures.
    /// Returns `false` if either file is missing or fails to parse.
    pub fn load(
        &mut self,
        path: &str,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> bool {
        if self.entries.contains_key(path) {
            return true;
        }

        let spr_path = format!("{path}.spr");
        let act_path = format!("{path}.act");

        let spr_bytes = match grf.read_file(&spr_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Effect SPR missing: {spr_path} ({e})");
                return false;
            }
        };
        let spr = match SprFile::parse(&spr_bytes) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Effect SPR parse failed: {spr_path} ({e})");
                return false;
            }
        };
        let act_bytes = match grf.read_file(&act_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Effect ACT missing: {act_path} ({e})");
                return false;
            }
        };
        let act = match ActFile::parse(&act_bytes) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Effect ACT parse failed: {act_path} ({e})");
                return false;
            }
        };

        let (images, indexed_count) = spr.to_rgba_images();
        let textures = upload_sprite_textures(&images, indexed_count, device, queue, layout);
        self.entries.insert(path.to_string(), EffectSpriteEntry { textures, act });
        true
    }

    pub fn get(&self, path: &str) -> Option<&EffectSpriteEntry> {
        self.entries.get(path)
    }
}

/// One emitter projected into screen space, ready to be drawn.
pub struct EmitterDraw<'a> {
    pub sprite: &'a EffectSpriteEntry,
    pub screen_anchor: [f32; 2],
    pub depth: f32,
    pub sprite_scale: f32,
    pub motion_index: usize,
    pub color: [f32; 4],
}

/// Build sprite batches for a list of emitter draw entries. The caller is
/// responsible for animation timing (selecting `motion_index`) and per-
/// particle alpha (encoded in `color[3]`).
pub fn build_emitter_batches<'a>(
    draws: &[EmitterDraw<'a>],
) -> Vec<SpriteBatch<'a>> {
    let mut batches = Vec::new();
    for draw in draws {
        if draw.sprite.act.actions.is_empty() {
            continue;
        }
        let action = &draw.sprite.act.actions[0];
        if action.motions.is_empty() {
            continue;
        }
        let motion = &action.motions[draw.motion_index % action.motions.len()];
        for clip in &motion.clips {
            let Some((mut vertices, indices, tex_idx)) =
                build_clip_quad(clip, &draw.sprite.textures, draw.screen_anchor, draw.depth, [0, 0])
            else {
                continue;
            };
            if tex_idx >= draw.sprite.textures.bind_groups.len() {
                continue;
            }
            scale_clip_vertices(&mut vertices, draw.screen_anchor, draw.sprite_scale, 0.0);
            for v in &mut vertices {
                v.color[0] *= draw.color[0];
                v.color[1] *= draw.color[1];
                v.color[2] *= draw.color[2];
                v.color[3] *= draw.color[3];
            }
            batches.push(SpriteBatch {
                vertices,
                indices,
                texture: &draw.sprite.textures.bind_groups[tex_idx],
                additive: false,
            });
        }
    }
    batches
}

/// Helper: project a world-space anchor into a screen anchor / depth /
/// per-pixel scale for a billboarded effect.
pub fn project_billboard(
    camera: &Camera,
    world_pos: [f32; 3],
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, f32)> {
    let (sx, sy, ndc_z, _clip_w) = camera.world_to_screen_with_depth(
        world_pos[0], world_pos[1], world_pos[2], screen_w, screen_h,
    )?;
    let ppu = camera.perspective_scale(world_pos[0], world_pos[1], world_pos[2], screen_h);
    Some(([sx, sy], ndc_z, ppu))
}

/// Renderer-side description of a single SPR-based effect emitter.
pub enum SpriteEffectEmitter<'a> {
    Spr {
        sprite_path: &'a str,
        duration_ms: f32,
        position: [f32; 3],
        color: [f32; 4],
        size_scale: f32,
        anim_time: f32,
    },
    Smoke3D {
        sprite_path: &'a str,
        alpha_max: f32,
        color: [f32; 4],
        size_scale: f32,
        anim_speed: f32,
        particles: Vec<([f32; 3], f32, f32)>,
    },
}

/// Collect [`EmitterDraw`] entries from emitters. Shared between the game
/// client and rsw-viewer so the projection / animation logic is not
/// duplicated.
pub fn collect_sprite_effect_draws<'a>(
    emitters: &[SpriteEffectEmitter<'a>],
    cache: &'a EffectSpriteCache,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Vec<EmitterDraw<'a>> {
    let mut draws = Vec::new();
    for emitter in emitters {
        match emitter {
            SpriteEffectEmitter::Spr {
                sprite_path, duration_ms, position, color, size_scale, anim_time,
            } => {
                let Some(sprite) = cache.get(sprite_path) else { continue };
                let Some((anchor, depth, ppu)) =
                    project_billboard(camera, *position, screen_w, screen_h)
                else { continue };
                let sprite_scale = ppu / 7.5;
                let action = sprite.act.actions.first();
                let motion_count = action.map(|a| a.motions.len()).unwrap_or(0);
                if motion_count == 0 { continue; }
                let act_delay_ms = sprite.act.delays.first().copied().unwrap_or(0.0) * 25.0;
                let frame_delay_ms = if act_delay_ms > 0.0 {
                    act_delay_ms
                } else {
                    (duration_ms / motion_count as f32).max(1.0)
                };
                let motion_index = ((anim_time * 1000.0) / frame_delay_ms) as usize % motion_count;
                draws.push(EmitterDraw {
                    sprite,
                    screen_anchor: anchor,
                    depth,
                    sprite_scale: sprite_scale * size_scale,
                    motion_index,
                    color: *color,
                });
            }
            SpriteEffectEmitter::Smoke3D {
                sprite_path, alpha_max, color, size_scale, anim_speed, particles,
            } => {
                let Some(sprite) = cache.get(sprite_path) else { continue };
                let action = sprite.act.actions.first();
                let motion_count = action.map(|a| a.motions.len()).unwrap_or(0);
                if motion_count == 0 { continue; }
                let frames_per_sec = 60.0 / anim_speed.max(1.0);
                for &(pos, age, lifetime) in particles {
                    let t = (age / lifetime).clamp(0.0, 1.0);
                    let alpha = (1.0 - t) * alpha_max;
                    if alpha <= 0.01 { continue; }
                    let Some((anchor, depth, ppu)) =
                        project_billboard(camera, pos, screen_w, screen_h)
                    else { continue };
                    let sprite_scale = ppu / 7.5;
                    let motion_index = (age * frames_per_sec) as usize % motion_count;
                    draws.push(EmitterDraw {
                        sprite,
                        screen_anchor: anchor,
                        depth,
                        sprite_scale: sprite_scale * size_scale,
                        motion_index,
                        color: [color[0], color[1], color[2], color[3] * alpha],
                    });
                }
            }
        }
    }
    draws.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));
    draws
}

