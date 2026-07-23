use models::enums::EnumWithNumberValue;
use models::enums::effect_id::EffectId;
use ragnarok_effects::effect_queue::EffectQueue;
use ragnarok_effects::spec::EffectSpec;
use ragnarok_effects::table::{custom_duration_ms, effect_spec};
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::rsw::{RswFile, RswObject};

pub const AMBIENT_KEY_BASE: u32 = 0xFFF0_0000;

struct AmbientEmitter {
    world_pos: [f32; 3],
    effect_id: EffectId,
    key: u32,
    persistent: bool,
    emit_cooldown_s: f32,
    size_scale: f32,
    timer_s: f32,
    spawned: bool,
}

pub fn ambient_effect_assets(rsw: &RswFile) -> (Vec<&'static str>, Vec<String>) {
    let mut spr = Vec::new();
    let mut str_names = Vec::new();
    for obj in &rsw.objects {
        let RswObject::Effect(eff) = obj else {
            continue;
        };
        let Ok(id) = EffectId::try_from_value(eff.effect_type as usize) else {
            continue;
        };
        match effect_spec(id) {
            Some(EffectSpec::Spr { sprite, .. }) | Some(EffectSpec::SprBurst { sprite, .. }) => {
                spr.push(sprite)
            }
            Some(EffectSpec::Str { file, .. }) => str_names.push(file.to_string()),
            _ => {}
        }
    }
    (spr, str_names)
}

#[derive(Default)]
pub struct AmbientEffectScheduler {
    emitters: Vec<AmbientEmitter>,
}

impl AmbientEffectScheduler {
    pub fn empty() -> Self {
        Self::default()
    }

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
            let Ok(effect_id) = EffectId::try_from_value(eff.effect_type as usize) else {
                skipped += 1;
                continue;
            };
            let Some(spec) = effect_spec(effect_id) else {
                skipped += 1;
                continue;
            };
            let (duration_ms, is_spr, size_scale) = match &spec {
                EffectSpec::Spr { duration_ms, .. } => (*duration_ms, true, 1.0),
                EffectSpec::SprBurst { duration_ms, .. } => {
                    let sz = if eff.param[0] > 0.0 {
                        eff.param[0] / 100.0
                    } else {
                        1.0
                    };
                    (*duration_ms, false, sz)
                }
                EffectSpec::Str { duration_ms, .. } => (*duration_ms, false, 1.0),
                EffectSpec::Custom => (custom_duration_ms(effect_id), false, 1.0),
                EffectSpec::Noop => {
                    skipped += 1;
                    continue;
                }
            };
            let y_offset = if is_spr { -gnd.zoom } else { 0.0 };
            let world = [
                eff.position[0] * scale_factor + center_x,
                eff.position[1] * scale_factor + y_offset,
                eff.position[2] * scale_factor + center_z,
            ];
            let emit_cooldown_s = (eff.emit_speed.max(0.1)) / 60.0;
            let key = AMBIENT_KEY_BASE + emitters.len() as u32;
            emitters.push(AmbientEmitter {
                world_pos: world,
                effect_id,
                key,
                persistent: duration_ms == u32::MAX,
                emit_cooldown_s,
                size_scale,
                timer_s: emit_cooldown_s,
                spawned: false,
            });
        }
        if ragnarok_profiling::debug::trace_effects() {
            if skipped > 0 {
                tracing::info!(
                    "AmbientEffectScheduler: skipped {skipped} RSW effects (unknown/noop)"
                );
            }
            tracing::info!("AmbientEffectScheduler: {} emitters", emitters.len());
        }
        Self { emitters }
    }

    pub fn update(
        &mut self,
        dt: f32,
        is_visible: &dyn Fn([f32; 3]) -> bool,
        queue: &mut EffectQueue,
    ) {
        for e in &mut self.emitters {
            let visible = is_visible(e.world_pos);

            if e.persistent {
                if visible && !e.spawned {
                    queue.spawn_at_keyed_scaled(e.effect_id, e.world_pos, e.key, e.size_scale);
                    e.spawned = true;
                } else if !visible && e.spawned {
                    queue.despawn(e.key);
                    e.spawned = false;
                }
            } else if visible {
                e.timer_s += dt;
                if e.timer_s >= e.emit_cooldown_s {
                    e.timer_s = 0.0;
                    queue.spawn_at_keyed_scaled(e.effect_id, e.world_pos, e.key, e.size_scale);
                }
            }
        }
    }

    pub fn clear(&mut self, queue: &mut EffectQueue) {
        for e in &mut self.emitters {
            if e.spawned {
                queue.despawn(e.key);
                e.spawned = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_effects::spec::Attach;
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
    fn schedules_known_rsw_effects_and_emits_near_camera() {
        let rsw = make_rsw_with_params(vec![
            (47, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]),
            (44, [0.0, 0.0, 0.0], [35.0, 0.0, 0.0, 0.0]),
            (109, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]),
            (9999, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]),
        ]);
        let gnd = make_gnd();
        let mut sched = AmbientEffectScheduler::from_rsw(&rsw, &gnd);

        let mut queue = EffectQueue::new();
        sched.update(0.1, &|_p| true, &mut queue);

        let reqs = queue.drain();
        assert_eq!(reqs.len(), 3);
        for r in &reqs {
            assert!(matches!(r.attach, Attach::WorldPos(_)));
            assert!(r.key.is_some());
        }
        let torch = reqs
            .iter()
            .find(|r| r.effect_id == EffectId::Torch)
            .unwrap();
        assert_eq!(
            torch.size_scale,
            Some(1.0),
            "torch param[0] must not shrink it"
        );
        let smoke = reqs
            .iter()
            .find(|r| r.effect_id == EffectId::Smoke)
            .unwrap();
        assert_eq!(
            smoke.size_scale,
            Some(0.35),
            "smoke reads param[0] as size %"
        );
        assert!(reqs.iter().any(|r| r.effect_id == EffectId::Bubble));
    }

    #[test]
    fn far_camera_emits_nothing() {
        let rsw = make_rsw(vec![(47, [0.0, 0.0, 0.0]), (44, [0.0, 0.0, 0.0])]);
        let gnd = make_gnd();
        let mut sched = AmbientEffectScheduler::from_rsw(&rsw, &gnd);

        let mut queue = EffectQueue::new();
        sched.update(0.1, &|_p| false, &mut queue);
        assert!(queue.drain().is_empty());
    }
}
