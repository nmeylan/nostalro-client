use super::energy_drain::hash01;
use super::spike_burst::fade_in_out;
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const NEEDLE_TEXTURE: &str = "lens1.tga";
pub const SMOKE_TEXTURE: &str = "smoke.tga";
pub const TEXTURES: &[&str] = &[NEEDLE_TEXTURE, SMOKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const ICE_TINT: [f32; 3] = [0.88, 0.94, 1.0];
const BODY_LIFT: f32 = 3.0;
const WORLD_SCALE: f32 = 0.12;

pub const TOTAL_DURATION_MS: u32 = 550;

const NUM_SHARDS: usize = 9;
const SHARD_LIFE: f32 = 15.0;
const SHARD_FADE_START: f32 = 5.0;
const SHARD_MAX_ALPHA: f32 = 200.0 / 255.0;
const SHARD_FADE_IN: f32 = 1.0;
const SHARD_EXTEND_FRAMES: f32 = 4.0;

const SMOKE_BIRTHS: [f32; 2] = [0.0, 7.0];
const SMOKE_LIFE: f32 = 25.0;
const SMOKE_FADE_START: f32 = 16.7;
const SMOKE_FADE_IN: f32 = 3.0;
const SMOKE_MAX_ALPHA: f32 = 0.8;
const SMOKE_HALF_INIT: f32 = 1.0;
const SMOKE_GROWTH_FAST: f32 = 6.0;
const SMOKE_GROWTH_SLOW: f32 = 0.5;
const SMOKE_CHANGE_FRAME: f32 = 10.0;

const TOTAL_FRAMES: f32 = 33.0;

fn shard_dims(a: usize, seed: u32) -> (f32, f32, f32) {
    if a % 3 == 0 {
        (
            9.0,
            90.0 + hash01(seed) * 10.0,
            12.0 + hash01(seed ^ 0x11) * 4.5,
        )
    } else {
        (
            5.0,
            65.0 + hash01(seed) * 5.0,
            9.0 + hash01(seed ^ 0x11) * 4.5,
        )
    }
}

fn shard_radius_px(speed: f32, frame: f32) -> f32 {
    let accel = -(speed / SHARD_LIFE) / 1.5;
    (speed * frame + accel * frame * frame * 0.5).max(0.0)
}

pub struct ColdHitEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl ColdHitEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    fn collect_shards(&self, out: &mut EffectDrawList, frame: f32) {
        let alpha = fade_in_out(
            frame,
            SHARD_MAX_ALPHA,
            SHARD_FADE_IN,
            SHARD_FADE_START,
            SHARD_LIFE,
        );
        if alpha <= 0.0 {
            return;
        }
        let cx = self.world_pos[0];
        let cy = self.world_pos[1] - BODY_LIFT;
        let cz = self.world_pos[2];
        let extend = (frame / SHARD_EXTEND_FRAMES).clamp(0.0, 1.0);
        for a in 0..NUM_SHARDS {
            let seed = a as u32 * 0x9E37;
            let roll_deg = (a as f32) * 40.0 - 15.0 + hash01(seed ^ 0x55) * 60.0;
            let roll_rad = roll_deg.to_radians();
            let (width, length, speed) = shard_dims(a, seed);
            let half_w = width * WORLD_SCALE;
            let full_len = length * WORLD_SCALE * extend;
            if full_len <= 0.0 {
                continue;
            }
            let (sin_r, cos_r) = roll_rad.sin_cos();
            let radius = shard_radius_px(speed, frame) * WORLD_SCALE;
            let pos = [cx + radius * sin_r, cy - radius * cos_r, cz];
            out.push(EffectPrimitiveDraw::BillboardFlash {
                pos,
                size: [half_w, full_len],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: roll_rad,
                texture: NEEDLE_TEXTURE,
                color: [ICE_TINT[0], ICE_TINT[1], ICE_TINT[2], alpha],
                blend: BlendKind::Alpha,
            });
        }
    }

    fn collect_smoke(&self, out: &mut EffectDrawList, frame: f32) {
        for (i, &birth) in SMOKE_BIRTHS.iter().enumerate() {
            let local = frame - birth;
            if local < 0.0 || local > SMOKE_LIFE {
                continue;
            }
            let alpha = fade_in_out(
                local,
                SMOKE_MAX_ALPHA,
                SMOKE_FADE_IN,
                SMOKE_FADE_START,
                SMOKE_LIFE,
            );
            if alpha <= 0.0 {
                continue;
            }
            let half = if local <= SMOKE_CHANGE_FRAME {
                SMOKE_HALF_INIT + SMOKE_GROWTH_FAST * local
            } else {
                SMOKE_HALF_INIT
                    + SMOKE_GROWTH_FAST * SMOKE_CHANGE_FRAME
                    + SMOKE_GROWTH_SLOW * (local - SMOKE_CHANGE_FRAME)
            };
            let full = 2.0 * half * WORLD_SCALE;
            let s = i as u32 * 0x51ED;
            let ox = (hash01(s) - 0.5) * 2.0 * WORLD_SCALE;
            let oy = (hash01(s ^ 0x9) - 0.5) * 2.0 * WORLD_SCALE;
            out.push(EffectPrimitiveDraw::BillboardFlash {
                pos: [
                    self.world_pos[0] + ox,
                    self.world_pos[1] - BODY_LIFT + oy,
                    self.world_pos[2],
                ],
                size: [full, full],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: SMOKE_TEXTURE,
                color: [ICE_TINT[0], ICE_TINT[1], ICE_TINT[2], alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

impl Effect for ColdHitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.frame() > TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        self.collect_smoke(out, frame);
        self.collect_shards(out, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn run_to(c: &mut ColdHitEffect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx {
                delta,
                ..Default::default()
            });
        }
    }

    fn draws(c: &ColdHitEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn count_tex(c: &ColdHitEffect, tex: &str) -> usize {
        draws(c)
            .iter()
            .filter(|p| {
                matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, blend, .. }
                if *texture == tex && *blend == BlendKind::Alpha)
            })
            .count()
    }

    #[test]
    fn emits_nine_needles_and_a_smoke_cluster() {
        let mut c = ColdHitEffect::new([0.0; 3]);
        run_to(&mut c, 3.0);
        assert_eq!(count_tex(&c, NEEDLE_TEXTURE), NUM_SHARDS);
        assert!(count_tex(&c, SMOKE_TEXTURE) >= 1, "smoke born at frame 0");
        run_to(&mut c, 12.0);
        assert_eq!(
            count_tex(&c, SMOKE_TEXTURE),
            SMOKE_BIRTHS.len(),
            "both puffs alive"
        );
        run_to(&mut c, SHARD_LIFE + 1.0);
        assert_eq!(
            count_tex(&c, NEEDLE_TEXTURE),
            0,
            "needles gone after their life"
        );
    }

    fn first_shard(c: &ColdHitEffect) -> (f32, f32) {
        draws(c)
            .into_iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::BillboardFlash {
                    size,
                    color,
                    texture,
                    ..
                } if texture == NEEDLE_TEXTURE => Some((size[1], color[3])),
                _ => None,
            })
            .expect("ice needle")
    }

    #[test]
    fn shards_extend_then_fade_out() {
        let mut c = ColdHitEffect::new([0.0; 3]);
        run_to(&mut c, 1.0);
        let (len_early, _) = first_shard(&c);
        run_to(&mut c, SHARD_EXTEND_FRAMES);
        let (len_full, a_hold) = first_shard(&c);
        assert!(
            len_full > len_early,
            "needle extends ({len_early} → {len_full})"
        );
        run_to(&mut c, SHARD_FADE_START + 2.0);
        let (_, a_late) = first_shard(&c);
        assert!(
            a_late < a_hold,
            "needle fades after frame {SHARD_FADE_START}"
        );
    }

    #[test]
    fn dies_after_duration() {
        let mut c = ColdHitEffect::new([0.0; 3]);
        run_to(&mut c, TOTAL_FRAMES + 2.0);
        assert_eq!(c.update(&EffectUpdateCtx::default()), EffectStatus::Dead);
    }
}
