use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::rsw::{RswFile, RswObject};

use crate::effect_table::{EffectKind, effect_kind};

fn resolve_str_file(pattern: &str, rand_range: Option<(u32, u32)>) -> String {
    if let Some((lo, hi)) = rand_range {
        let n = lo + (rand_u32() % (hi - lo + 1));
        pattern.replace("%d", &n.to_string())
    } else {
        pattern.to_string()
    }
}

fn rand_u32() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::Instant::now().hash(&mut h);
    std::thread::current().id().hash(&mut h);
    h.finish() as u32
}

/// One spawned RSW ambient effect emitter. Position is already in renderer
/// world coordinates (centered, y from height grid). Units match the
/// ground/water meshes.
pub struct EffectEmitter {
    pub kind: EffectKind,
    pub position: [f32; 3],
    pub color: [f32; 4],
    /// Time since last emission (seconds). When >= emit_cooldown, a new
    /// effect cycle starts and this resets to 0.
    pub anim_time: f32,
    /// Cooldown between effect re-emissions (seconds), from RSW emit_speed.
    pub emit_cooldown: f32,
    /// Per-emitter size multiplier derived from RSW params.
    pub size_scale: f32,
    /// Resolved STR filename for `EffectKind::Str` emitters (pattern with
    /// `%d` replaced by a random value from `rand_range`).
    pub str_file: Option<String>,
    /// Whether the initial burst of particles has been spawned (Smoke3D only).
    has_emitted: bool,
    pub particles: Vec<Particle>,
}

/// One simulated 3D smoke particle. Lives between `age = 0` and `age =
/// lifetime`, then dies.
pub struct Particle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub age: f32,
    pub lifetime: f32,
}

pub struct EffectManager {
    pub emitters: Vec<EffectEmitter>,
}

impl Default for EffectManager {
    fn default() -> Self {
        Self::empty()
    }
}

impl EffectManager {
    pub fn empty() -> Self {
        Self {
            emitters: Vec::new(),
        }
    }

    /// Walk the RSW objects and spawn one emitter per `RswObject::Effect`
    /// whose type is mapped in `effect_table::effect_kind`. RSW positions
    /// are translated into renderer world coordinates the same way model
    /// instances are (see `ModelRenderer::from_rsw`).
    pub fn from_rsw(rsw: &RswFile, gnd: &GndFile) -> Self {
        let scale_factor = gnd.zoom / 10.0;
        let center_x = gnd.width as f32 * gnd.zoom / 2.0;
        let center_z = gnd.height as f32 * gnd.zoom / 2.0;

        let mut emitters = Vec::new();
        let mut skipped = 0usize;
        for obj in &rsw.objects {
            let RswObject::Effect(eff) = obj else {
                continue;
            };
            let Some(kind) = effect_kind(eff.effect_type) else {
                skipped += 1;
                continue;
            };
            // Param interpretation is effect-type-specific.
            let (color, size_scale) = match &kind {
                EffectKind::Spr { .. } => {
                    // Sprite uses its own colour: SPR pixel * ACT clip color, no RSW param tint.
                    ([1.0, 1.0, 1.0, 1.0], 1.0)
                }
                EffectKind::Smoke3D { .. } => {
                    // param[0] = size percentage, param[1] = emission delay modifier
                    let sz = if eff.param[0] > 0.0 {
                        eff.param[0] / 100.0
                    } else {
                        1.0
                    };
                    ([1.0, 1.0, 1.0, 1.0], sz)
                }
                EffectKind::Str { .. } => ([1.0, 1.0, 1.0, 1.0], 1.0),
            };
            let str_file = match &kind {
                EffectKind::Str {
                    file_pattern,
                    rand_range,
                } => Some(resolve_str_file(file_pattern, *rand_range)),
                _ => None,
            };
            let y_offset = match &kind {
                EffectKind::Spr { .. } => -gnd.zoom,
                _ => 0.0,
            };
            let world = [
                eff.position[0] * scale_factor + center_x,
                eff.position[1] * scale_factor + y_offset,
                eff.position[2] * scale_factor + center_z,
            ];
            // Original game: emit_speed is in frames (60fps), default 360.
            // Initial counter starts at emitSpeed - random(24), so first emit
            // happens after 0..23 frames (0..0.4s). We convert to seconds.
            let emit_cooldown_secs = eff.emit_speed.max(0.1) / 60.0;
            // Random initial offset: 0..23 frames = 0..0.383s
            let initial_offset = (rand_u32() % 24) as f32;
            emitters.push(EffectEmitter {
                kind,
                position: world,
                color,
                anim_time: emit_cooldown_secs - initial_offset,
                emit_cooldown: emit_cooldown_secs,
                size_scale,
                str_file,
                has_emitted: false,
                particles: Vec::new(),
            });
        }
        if skipped > 0 {
            tracing::debug!("EffectManager: skipped {skipped} RSW effects with unmapped types");
        }
        tracing::info!("EffectManager: spawned {} emitters", emitters.len());

        Self { emitters }
    }

    /// Advance animation time and simulate 3D smoke particles.
    pub fn update(&mut self, dt: f32) {
        for emitter in &mut self.emitters {
            emitter.anim_time += dt;

            match emitter.kind {
                EffectKind::Smoke3D {
                    duration_ms,
                    pos_z_start,
                    burst_count_range,
                    speed_range,
                    ..
                } => {
                    let lifetime = (duration_ms / 1000.0).max(1e-3);

                    if !emitter.has_emitted {
                        emitter.has_emitted = true;
                        let (lo, hi) = burst_count_range;
                        let count = lo + (rand_u32() % (hi - lo + 1));
                        let (slo, shi) = speed_range;
                        for _ in 0..count {
                            let r = (rand_u32() % 1000) as f32 / 1000.0;
                            let speed = slo + r * (shi - slo);
                            emitter.particles.push(Particle {
                                position: [
                                    emitter.position[0],
                                    emitter.position[1] - pos_z_start,
                                    emitter.position[2],
                                ],
                                velocity: [0.0, -speed * 60.0, 0.0],
                                age: 0.0,
                                lifetime,
                            });
                        }
                    }

                    // Re-emit after cooldown (all particles dead)
                    if emitter.anim_time >= emitter.emit_cooldown && emitter.particles.is_empty() {
                        emitter.anim_time = 0.0;
                        emitter.has_emitted = false;
                    }

                    emitter.particles.retain_mut(|p| {
                        p.age += dt;
                        if p.age >= p.lifetime {
                            return false;
                        }
                        p.position[0] += p.velocity[0] * dt;
                        p.position[1] += p.velocity[1] * dt;
                        p.position[2] += p.velocity[2] * dt;
                        true
                    });
                }
                EffectKind::Str { .. } => {
                    // Original game re-launches the STR effect every emit_cooldown.
                    // anim_time resets so the STR plays from the beginning.
                    if emitter.emit_cooldown > 0.0 && emitter.anim_time >= emitter.emit_cooldown {
                        emitter.anim_time -= emitter.emit_cooldown;
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::rsw::{LightSettings, RswEffect, WaterSettings};

    fn make_rsw_with_params(effects: Vec<(u32, [f32; 3], [f32; 4])>) -> RswFile {
        let objects = effects
            .into_iter()
            .map(|(t, pos, param)| {
                RswObject::Effect(RswEffect {
                    name: String::from("test"),
                    position: pos,
                    effect_type: t,
                    emit_speed: 4.0,
                    param,
                })
            })
            .collect();
        RswFile {
            version: (2, 1),
            ini_file: String::new(),
            gnd_file: String::new(),
            gat_file: String::new(),
            source_file: None,
            water: WaterSettings {
                level: None,
                water_type: None,
                wave_height: None,
                wave_speed: None,
                wave_pitch: None,
                anim_speed: None,
            },
            light: LightSettings {
                longitude: None,
                latitude: None,
                diffuse: None,
                ambient: None,
                shadow_map_alpha: None,
            },
            ground_top: None,
            ground_bottom: None,
            ground_left: None,
            ground_right: None,
            objects,
        }
    }

    fn make_gnd() -> GndFile {
        GndFile {
            version: (1, 7),
            width: 10,
            height: 10,
            zoom: 10.0,
            textures: vec![],
            lightmaps: vec![],
            surfaces: vec![],
            cells: (0..100)
                .map(|_| ragnarok_formats::gnd::GndCell {
                    height_sw: 0.0,
                    height_se: 0.0,
                    height_nw: 0.0,
                    height_ne: 0.0,
                    surface_up: -1,
                    surface_south: -1,
                    surface_east: -1,
                })
                .collect(),
        }
    }

    fn make_rsw(effects: Vec<(u32, [f32; 3])>) -> RswFile {
        make_rsw_with_params(
            effects
                .into_iter()
                .map(|(t, pos)| (t, pos, [0.0; 4]))
                .collect(),
        )
    }

    #[test]
    fn smoke_params_set_size_not_color() {
        let rsw = make_rsw_with_params(vec![
            (44, [0.0, 0.0, 0.0], [35.0, 35.0, 0.0, 0.0]),
            (47, [0.0, 0.0, 0.0], [1.0, 0.6, 0.0, 0.0]),
        ]);
        let gnd = make_gnd();
        let mgr = EffectManager::from_rsw(&rsw, &gnd);

        let smoke = &mgr.emitters[0];
        assert!(matches!(smoke.kind, EffectKind::Smoke3D { .. }));
        assert_eq!(smoke.color, [1.0, 1.0, 1.0, 1.0]);
        assert!((smoke.size_scale - 0.35).abs() < 0.001);

        let torch = &mgr.emitters[1];
        assert!(matches!(torch.kind, EffectKind::Spr { .. }));
        assert_eq!(torch.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(torch.size_scale, 1.0);
    }

    #[test]
    fn manager_spawns_known_effects_and_simulates_smoke() {
        let rsw = make_rsw(vec![
            (47, [0.0, 0.0, 0.0]),   // torch (SPR)
            (44, [10.0, 0.0, 10.0]), // smoke (3D)
            (109, [0.0, 0.0, 0.0]),  // bubble (STR — kept but not rendered)
            (9999, [0.0, 0.0, 0.0]), // unknown — dropped
        ]);
        let gnd = make_gnd();

        let mut mgr = EffectManager::from_rsw(&rsw, &gnd);
        assert_eq!(mgr.emitters.len(), 3);

        // After advancing time, the 3D smoke emitter should have particles
        // but the SPR torch and STR bubble should not (no particle sim).
        mgr.update(0.1);
        let smoke = mgr
            .emitters
            .iter()
            .find(|e| matches!(e.kind, EffectKind::Smoke3D { .. }))
            .expect("smoke emitter");
        assert!(
            !smoke.particles.is_empty(),
            "smoke should have particles after update"
        );

        for emitter in &mgr.emitters {
            if !matches!(emitter.kind, EffectKind::Smoke3D { .. }) {
                assert!(emitter.particles.is_empty());
            }
        }
    }
}
