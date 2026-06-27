use std::collections::HashMap;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::spr::SprFile;
use ragnarok_game::effect::{EffectDrawList, EffectPrimitiveDraw};

use crate::camera::Camera;
use crate::effect::queue::{BlendBucket, DrawRecord, PipelineKind, view_z};
use crate::sprite::{
    SpriteBatch, SpriteTextures, build_clip_quad, rotate_sprite_vertices, scale_clip_vertices,
    upload_sprite_textures,
};

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
        Self {
            entries: HashMap::new(),
        }
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
        self.entries
            .insert(path.to_string(), EffectSpriteEntry { textures, act });
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
    pub action_index: usize,
    pub color: [f32; 4],
    /// `true` → additive blend (Hit debris, anything wanting overlap to
    /// accumulate to brighter colors); `false` → standard alpha blend
    /// (Smoke, Snow, Firefly — all the existing emitter callers).
    /// particle1.spr is flagged additive in the original game, so its
    /// emitters set this; the others use plain alpha. Verified against the
    /// original game's on-screen blending.
    pub additive: bool,
}

/// Build sprite batches for a list of emitter draw entries. The caller is
/// responsible for animation timing (selecting `motion_index`) and per-
/// particle alpha (encoded in `color[3]`).
pub fn build_emitter_batches<'a>(draws: &[EmitterDraw<'a>]) -> Vec<SpriteBatch<'a>> {
    let mut batches = Vec::new();
    for draw in draws {
        if draw.sprite.act.actions.is_empty() {
            continue;
        }
        let action = &draw.sprite.act.actions[draw.action_index % draw.sprite.act.actions.len()];
        if action.motions.is_empty() {
            continue;
        }
        let motion = &action.motions[draw.motion_index % action.motions.len()];
        for clip in &motion.clips {
            let Some((mut vertices, indices, tex_idx)) = build_clip_quad(
                clip,
                &draw.sprite.textures,
                draw.screen_anchor,
                draw.depth,
                [0, 0],
            ) else {
                continue;
            };
            if tex_idx >= draw.sprite.textures.bind_groups.len() {
                continue;
            }
            scale_clip_vertices(&mut vertices, draw.screen_anchor, draw.sprite_scale, [0.0, 0.0]);
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
                additive: draw.additive,
            });
        }
    }
    batches
}

/// Walk an `EffectDrawList`, find every `EffectPrimitiveDraw::SpriteParticle`
/// entry, and produce one [`DrawRecord`] per sprite clip ready for the
/// unified effect dispatch. Records use screen-space vertices and dispatch
/// through the [`PipelineKind::Sprite`] pipeline; depth is the world-space
/// `view_z` of the particle anchor so the renderer can sort sprite
/// particles against billboards and 3D primitives consistently.
pub fn prepare_sprite_particle_records<'cache>(
    list: &EffectDrawList,
    cache: &'cache EffectSpriteCache,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Vec<DrawRecord<'cache>> {
    let mut records: Vec<DrawRecord<'cache>> = Vec::new();
    for (emission, prim) in list.primitives.iter().enumerate() {
        let EffectPrimitiveDraw::SpriteParticle {
            sprite_path,
            position,
            action_index,
            motion_index,
            size_scale,
            color,
            blend,
            aim_target,
            no_depth,
        } = prim
        else {
            continue;
        };
        let Some(sprite) = cache.get(sprite_path) else {
            continue;
        };
        let Some((anchor, depth, ppu)) =
            project_billboard(camera, *position, screen_w, screen_h)
        else {
            continue;
        };
        if sprite.act.actions.is_empty() {
            continue;
        }
        let action = &sprite.act.actions[action_index % sprite.act.actions.len()];
        let motion_count = action.motions.len();
        if motion_count == 0 {
            continue;
        }
        let motion = &action.motions[motion_index % motion_count];
        let sprite_scale = (ppu / 7.5) * size_scale;
        let view_depth = view_z(camera, *position);
        let blend_bucket = match (BlendBucket::from_blend_kind(*blend), *no_depth) {
            (BlendBucket::Alpha, true) => BlendBucket::AlphaNoDepth,
            (BlendBucket::Additive, true) => BlendBucket::AdditiveNoDepth,
            (bucket, _) => bucket,
        };
        for clip in &motion.clips {
            let Some((mut vertices, indices, tex_idx)) =
                build_clip_quad(clip, &sprite.textures, anchor, depth, [0, 0])
            else {
                continue;
            };
            if tex_idx >= sprite.textures.bind_groups.len() {
                continue;
            }
            scale_clip_vertices(&mut vertices, anchor, sprite_scale, [0.0, 0.0]);
            if let Some(target) = aim_target {
                if let Some((tx, ty)) = camera.world_to_screen(target[0], target[1], target[2], screen_w, screen_h) {
                    let dx = tx - anchor[0];
                    let dy = ty - anchor[1];
                    let angle = dy.atan2(dx) - std::f32::consts::FRAC_PI_2;
                    rotate_sprite_vertices(&mut vertices, anchor, angle);
                }
            }
            for v in &mut vertices {
                v.color[0] *= color[0];
                v.color[1] *= color[1];
                v.color[2] *= color[2];
                v.color[3] *= color[3];
            }
            records.push(DrawRecord::new(
                view_depth,
                emission as u32,
                blend_bucket,
                PipelineKind::Sprite,
                vertices,
                indices,
                &sprite.textures.bind_groups[tex_idx],
            ));
        }
    }
    records
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
        world_pos[0],
        world_pos[1],
        world_pos[2],
        screen_w,
        screen_h,
    )?;
    let ppu = camera.perspective_scale(world_pos[0], world_pos[1], world_pos[2], screen_h);
    Some(([sx, sy], ndc_z, ppu))
}

/// Constant world-space distance a camera-facing effect quad is nudged toward
/// the camera so it renders over coincident ground without the depth-precision
/// flicker a zero offset would cause.
///
/// A *fixed* NDC offset can't do this: perspective depth is non-linear, so a
/// constant NDC delta is a few units near the camera but tens of world units
/// at map-scale distances — large enough to yank a quad in front of the caster
/// and suppress all occlusion. Converting a fixed *world* distance to NDC
/// (`ndc_bias = near * units / clip_w²`, with `clip_w` ≈ view-space distance)
/// keeps the nudge a constant world distance at every zoom, so the caster
/// still occludes the back half of a body-centred ring (Chookgi) while the
/// quad reliably beats the ground it sits on.
pub const BILLBOARD_DEPTH_BIAS_UNITS: f32 = 1.0;

/// Project like [`project_billboard`] but pull the returned depth toward the
/// camera by [`BILLBOARD_DEPTH_BIAS_UNITS`] world units (zoom-independent).
pub fn project_billboard_biased(
    camera: &Camera,
    world_pos: [f32; 3],
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, f32)> {
    let (sx, sy, ndc_z, clip_w) = camera.world_to_screen_with_depth(
        world_pos[0],
        world_pos[1],
        world_pos[2],
        screen_w,
        screen_h,
    )?;
    let ppu = camera.perspective_scale(world_pos[0], world_pos[1], world_pos[2], screen_h);
    let ndc_z = ndc_z - camera.near * BILLBOARD_DEPTH_BIAS_UNITS / (clip_w * clip_w);
    Some(([sx, sy], ndc_z, ppu))
}

/// World-space depth nudge applied to entity sprites (`project_entity_screen`).
/// A camera-facing effect quad that wants to occlude *against the body* must
/// use the same value so the comparison is purely front/back, not biased by a
/// mismatched nudge.
pub const ENTITY_DEPTH_BIAS_UNITS: f32 = 4.0;

/// Project a [`BillboardDepthAnchored`] quad: screen anchor / scale come from
/// `screen_pos`, but the returned depth is computed from `depth_pos` (the
/// quad's ground anchor) biased by [`ENTITY_DEPTH_BIAS_UNITS`] to match the
/// entity sprite. Returns `(screen_anchor, ndc_z, ppu, view_z)`.
///
/// [`BillboardDepthAnchored`]: ragnarok_game::effect::EffectPrimitiveDraw::BillboardDepthAnchored
pub fn project_billboard_depth_anchored(
    camera: &Camera,
    screen_pos: [f32; 3],
    depth_pos: [f32; 3],
    screen_w: f32,
    screen_h: f32,
) -> Option<([f32; 2], f32, f32, f32)> {
    let (sx, sy, _ndc_z, _clip_w) =
        camera.world_to_screen_with_depth(screen_pos[0], screen_pos[1], screen_pos[2], screen_w, screen_h)?;
    let ppu = camera.perspective_scale(screen_pos[0], screen_pos[1], screen_pos[2], screen_h);
    let (_, _, ndc_z, clip_w) =
        camera.world_to_screen_with_depth(depth_pos[0], depth_pos[1], depth_pos[2], screen_w, screen_h)?;
    let ndc_z = ndc_z - camera.near * ENTITY_DEPTH_BIAS_UNITS / (clip_w * clip_w);
    Some(([sx, sy], ndc_z, ppu, view_z(camera, depth_pos)))
}

/// Per-particle data for a `SpriteEffectEmitter::Smoke3D`. `alpha_override`
/// is set when the holder has already computed the particle's
/// instantaneous alpha (e.g. a twinkle keyframe sawtooth); the renderer
/// uses it verbatim and skips the linear-fade-plus-sin² fallback.
#[derive(Clone, Copy, Debug)]
pub struct Smoke3DParticle {
    pub pos: [f32; 3],
    pub age: f32,
    pub lifetime: f32,
    pub alpha_override: Option<f32>,
}

/// Renderer-side description of a single SPR-based effect emitter.
pub enum SpriteEffectEmitter<'a> {
    Spr {
        sprite_path: &'a str,
        duration_ms: f32,
        position: [f32; 3],
        color: [f32; 4],
        size_scale: f32,
        /// Animation speed: motion advances every N ticks at
        /// 60 fps, so higher values slow the animation down. 1.0 = one
        /// motion per game frame (16.67 ms each).
        anim_speed: f32,
        /// `true` loops the motion list; `false`
        /// plays once and holds the final motion.
        repeat: bool,
        anim_time: f32,
        /// ACT action index to play.
        action_index: usize,
    },
    Smoke3D {
        sprite_path: &'a str,
        alpha_max: f32,
        color: [f32; 4],
        size_scale: f32,
        anim_speed: f32,
        /// Linearly shrink each particle's rendered size to 0 over its
        /// lifetime (Steal's gold-coin shrink).
        size_shrink: bool,
        /// Oscillate per-particle alpha around the linear fade envelope
        /// (Firefly's pulsing twinkle). Ignored
        /// when a particle supplies `alpha_override`.
        twinkle: bool,
        particles: Vec<Smoke3DParticle>,
    },
}

/// Collect [`EmitterDraw`] entries from emitters. Shared between the game
/// client and rsw-viewer so the projection / animation logic is not
/// duplicated.
pub fn collect_sprite_effect_draws<'cache>(
    emitters: &[SpriteEffectEmitter<'_>],
    cache: &'cache EffectSpriteCache,
    camera: &Camera,
    screen_w: f32,
    screen_h: f32,
) -> Vec<EmitterDraw<'cache>> {
    let mut draws = Vec::new();
    for emitter in emitters {
        match emitter {
            SpriteEffectEmitter::Spr {
                sprite_path,
                duration_ms: _,
                position,
                color,
                size_scale,
                anim_speed,
                repeat,
                anim_time,
                action_index,
            } => {
                let Some(sprite) = cache.get(sprite_path) else {
                    continue;
                };
                let Some((anchor, depth, ppu)) =
                    project_billboard(camera, *position, screen_w, screen_h)
                else {
                    continue;
                };
                let sprite_scale = ppu / 7.5;
                if sprite.act.actions.is_empty() {
                    continue;
                }
                let action = &sprite.act.actions[action_index % sprite.act.actions.len()];
                let motion_count = action.motions.len();
                if motion_count == 0 {
                    continue;
                }
                // Effect sprites ignore the .act file's per-frame delay and
                // instead advance motion every `anim_speed` ticks at 60 fps,
                // matching the original game's animation cadence.
                const FRAME_MS_60FPS: f32 = 1000.0 / 60.0;
                let frame_delay_ms = FRAME_MS_60FPS * anim_speed.max(1.0);
                let raw_motion = ((anim_time * 1000.0) / frame_delay_ms) as usize;
                let motion_index = if *repeat {
                    raw_motion % motion_count
                } else {
                    raw_motion.min(motion_count - 1)
                };
                draws.push(EmitterDraw {
                    sprite,
                    screen_anchor: anchor,
                    depth,
                    sprite_scale: sprite_scale * size_scale,
                    motion_index,
                    action_index: *action_index,
                    color: *color,
                    additive: false,
                });
            }
            SpriteEffectEmitter::Smoke3D {
                sprite_path,
                alpha_max,
                color,
                size_scale,
                anim_speed,
                size_shrink,
                twinkle,
                particles,
            } => {
                let Some(sprite) = cache.get(sprite_path) else {
                    continue;
                };
                let action = sprite.act.actions.first();
                let motion_count = action.map(|a| a.motions.len()).unwrap_or(0);
                if motion_count == 0 {
                    continue;
                }
                let frames_per_sec = 60.0 / anim_speed.max(1.0);
                for particle in particles {
                    let Smoke3DParticle { pos, age, lifetime, alpha_override } = *particle;
                    let t = (age / lifetime).clamp(0.0, 1.0);
                    let alpha = match alpha_override {
                        Some(a) => a,
                        None => {
                            let envelope = (1.0 - t) * alpha_max;
                            if *twinkle {
                                // sin² pulse around the linear envelope —
                                // legacy approximation for emitters with
                                // no keyframe schedule (none currently
                                // hit this branch with twinkle=true).
                                let phase = age * 2.5 * std::f32::consts::TAU;
                                let pulse = 0.4 + 0.6 * phase.sin().powi(2);
                                envelope * pulse
                            } else {
                                envelope
                            }
                        }
                    };
                    if alpha <= 0.01 {
                        continue;
                    }
                    let Some((anchor, depth, ppu)) =
                        project_billboard(camera, pos, screen_w, screen_h)
                    else {
                        continue;
                    };
                    let sprite_scale = ppu / 7.5;
                    let per_particle_size = if *size_shrink { (1.0 - t).max(0.0) } else { 1.0 };
                    let motion_index = (age * frames_per_sec) as usize % motion_count;
                    draws.push(EmitterDraw {
                        sprite,
                        screen_anchor: anchor,
                        depth,
                        sprite_scale: sprite_scale * size_scale * per_particle_size,
                        motion_index,
                        action_index: 0,
                        color: [color[0], color[1], color[2], color[3] * alpha],
                        additive: false,
                    });
                }
            }
        }
    }
    draws.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    draws
}
