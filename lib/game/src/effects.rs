use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::rsw::{RswFile, RswObject};

use crate::effect_table::{EffectKind, effect_kind};

/// One spawned RSW ambient effect emitter. Position is already in renderer
/// world coordinates (centered, y from height grid). Units match the
/// ground/water meshes.
pub struct EffectEmitter {
    pub kind: EffectKind,
    pub position: [f32; 3],
    /// Per-emitter color tint from `RswEffect::param[0..3]`. Defaults to
    /// white if the emitter doesn't override it.
    pub color: [f32; 4],
    /// Time accumulator since spawn (seconds).
    pub anim_time: f32,
    /// Particles per second; used by 3D smoke emitters. For Spr emitters
    /// this is purely informational (animation runs at fixed FPS from the
    /// effect kind).
    pub emit_rate: f32,
    /// Time of next particle emission (seconds since spawn). For non-3D
    /// emitters this is unused.
    next_emit_at: f32,
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
        Self { emitters: Vec::new() }
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
            let RswObject::Effect(eff) = obj else { continue };
            let Some(kind) = effect_kind(eff.effect_type) else {
                skipped += 1;
                continue;
            };
            let color = if eff.param[0] > 0.0 || eff.param[1] > 0.0 || eff.param[2] > 0.0 {
                [eff.param[0], eff.param[1], eff.param[2], 1.0]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };
            let world = [
                eff.position[0] * scale_factor + center_x,
                eff.position[1] * scale_factor,
                eff.position[2] * scale_factor + center_z,
            ];
            let emit_rate = eff.emit_speed.max(0.1);
            emitters.push(EffectEmitter {
                kind,
                position: world,
                color,
                anim_time: 0.0,
                emit_rate,
                next_emit_at: 0.0,
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

            let EffectKind::Smoke3D { duration_ms, pos_z_start, pos_z_end, .. } = emitter.kind else {
                continue;
            };
            let lifetime = (duration_ms / 1000.0).max(1e-3);

            // Spawn new particles at emit_rate per second.
            let interval = 1.0 / emitter.emit_rate;
            while emitter.anim_time >= emitter.next_emit_at {
                let dy = (pos_z_end - pos_z_start) / lifetime;
                emitter.particles.push(Particle {
                    position: [
                        emitter.position[0],
                        emitter.position[1] - pos_z_start,
                        emitter.position[2],
                    ],
                    velocity: [0.0, -dy, 0.0],
                    age: 0.0,
                    lifetime,
                });
                emitter.next_emit_at += interval;
                if emitter.next_emit_at < emitter.anim_time - interval {
                    // Catch up after a long pause without exploding particle count
                    emitter.next_emit_at = emitter.anim_time;
                }
            }

            // Simulate and cull dead particles.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::rsw::{LightSettings, RswEffect, WaterSettings};

    fn make_rsw(effects: Vec<(u32, [f32; 3])>) -> RswFile {
        let objects = effects
            .into_iter()
            .map(|(t, pos)| {
                RswObject::Effect(RswEffect {
                    name: String::from("test"),
                    position: pos,
                    effect_type: t,
                    emit_speed: 4.0,
                    param: [0.0; 4],
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
                level: None, water_type: None, wave_height: None,
                wave_speed: None, wave_pitch: None, anim_speed: None,
            },
            light: LightSettings {
                longitude: None, latitude: None, diffuse: None,
                ambient: None, shadow_map_alpha: None,
            },
            ground_top: None, ground_bottom: None,
            ground_left: None, ground_right: None,
            objects,
        }
    }

    fn make_gnd() -> GndFile {
        GndFile {
            version: (1, 7),
            width: 10, height: 10, zoom: 10.0,
            textures: vec![], lightmaps: vec![],
            surfaces: vec![],
            cells: (0..100).map(|_| ragnarok_formats::gnd::GndCell {
                height_sw: 0.0, height_se: 0.0, height_nw: 0.0, height_ne: 0.0,
                surface_up: -1, surface_south: -1, surface_east: -1,
            }).collect(),
        }
    }

    #[test]
    fn manager_spawns_known_effects_and_simulates_smoke() {
        let rsw = make_rsw(vec![
            (47, [0.0, 0.0, 0.0]),    // torch (SPR)
            (44, [10.0, 0.0, 10.0]),  // smoke (3D)
            (109, [0.0, 0.0, 0.0]),   // bubble (STR — kept but not rendered)
            (9999, [0.0, 0.0, 0.0]),  // unknown — dropped
        ]);
        let gnd = make_gnd();

        let mut mgr = EffectManager::from_rsw(&rsw, &gnd);
        assert_eq!(mgr.emitters.len(), 3);

        // After advancing time, the 3D smoke emitter should have particles
        // but the SPR torch and STR bubble should not (no particle sim).
        mgr.update(0.1);
        let smoke = mgr.emitters.iter()
            .find(|e| matches!(e.kind, EffectKind::Smoke3D { .. }))
            .expect("smoke emitter");
        assert!(!smoke.particles.is_empty(), "smoke should have particles after update");

        for emitter in &mgr.emitters {
            if !matches!(emitter.kind, EffectKind::Smoke3D { .. }) {
                assert!(emitter.particles.is_empty());
            }
        }
    }
}
