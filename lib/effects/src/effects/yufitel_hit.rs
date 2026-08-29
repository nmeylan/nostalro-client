use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &[
    "thunder_pang.bmp",
    "thunder_plazma_blast_a.bmp",
    "thunder_plazma_blast_b.bmp",
    "thunder_ball_d.bmp",
    "thunder_ball_e.bmp",
    "thunder_ball_f.bmp",
    "pokjuk_d.bmp",
    "twirl_soft.bmp",
    "thunder_ball_b.bmp",
    "thunder_ball_c.bmp",
];

const FPS: f32 = 60.0;
/// The launching effect lives 250 ticks; both quad groups are cut off with it.
const TOTAL_FRAMES: f32 = 250.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

const Y_OFFSET: f32 = -5.0;

const BURST_PERIOD_FRAMES: f32 = 20.0;
const BURST_LIFE_FRAMES: f32 = 10.0;
const BURST_GROWTH_PER_FRAME: f32 = 5.0;
/// The flash holds full alpha for one tick, then fades across the rest of its
/// life.
const BURST_FADE_START_FRAME: f32 = 1.0;

const BALL_START_FRAME: f32 = 10.0;
const BALL_SIZE: f32 = 15.0;
const BALL_FRAMES_PER_STEP: f32 = 1.0;
const BALL_ALPHA: f32 = 254.0 / 255.0;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

#[derive(Clone, Copy)]
pub struct YufitelHitParams {
    pub burst: &'static str,
    pub ball_cycle: &'static [&'static str],
}

/// `EF_YUFITELHIT` — the plasma-blast core behind a white pang.
pub const YUFITEL_HIT: YufitelHitParams = YufitelHitParams {
    burst: "thunder_pang.bmp",
    ball_cycle: &[
        "thunder_plazma_blast_a.bmp",
        "thunder_plazma_blast_b.bmp",
        "thunder_ball_d.bmp",
        "thunder_ball_e.bmp",
        "thunder_ball_f.bmp",
    ],
};

/// `EF_YUFITEL2` — a softer core alternating with the crackling ball, behind a
/// colourful pang.
pub const YUFITEL2: YufitelHitParams = YufitelHitParams {
    burst: "pokjuk_d.bmp",
    ball_cycle: &[
        "twirl_soft.bmp",
        "thunder_ball_b.bmp",
        "twirl_soft.bmp",
        "thunder_ball_c.bmp",
        "twirl_soft.bmp",
    ],
};

pub struct YufitelHitEffect {
    params: YufitelHitParams,
    pos: [f32; 3],
    age_frames: f32,
}

impl YufitelHitEffect {
    pub fn new(world_pos: [f32; 3], params: YufitelHitParams) -> Self {
        Self {
            params,
            pos: [world_pos[0], world_pos[1] + Y_OFFSET, world_pos[2]],
            age_frames: 0.0,
        }
    }

    /// Age of the burst launched most recently, or `None` once it has expired.
    fn burst_age(&self) -> Option<f32> {
        let phase = self.age_frames % BURST_PERIOD_FRAMES;
        (phase < BURST_LIFE_FRAMES).then_some(phase)
    }

    fn burst_size(age: f32) -> f32 {
        BURST_GROWTH_PER_FRAME * (age + 1.0)
    }

    fn burst_alpha(age: f32) -> f32 {
        if age <= BURST_FADE_START_FRAME {
            return 1.0;
        }
        ((BURST_LIFE_FRAMES - age) / (BURST_LIFE_FRAMES - BURST_FADE_START_FRAME)).clamp(0.0, 1.0)
    }

    fn ball_alpha(&self) -> f32 {
        if self.age_frames < BALL_START_FRAME {
            return 0.0;
        }
        BALL_ALPHA
    }

    fn ball_texture(&self) -> &'static str {
        let t = (self.age_frames - BALL_START_FRAME).max(0.0);
        let step = (t / BALL_FRAMES_PER_STEP) as usize;
        self.params.ball_cycle[step % self.params.ball_cycle.len()]
    }
}

impl Effect for YufitelHitEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.pos = [pos[0], pos[1] + Y_OFFSET, pos[2]];
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if let Some(age) = self.burst_age() {
            let size = Self::burst_size(age);
            let alpha = Self::burst_alpha(age);
            if size > 0.0 && alpha > 0.0 {
                out.push(EffectPrimitiveDraw::BillboardFlash {
                    pos: self.pos,
                    size: [size, size],
                    uv: UNIT_UV,
                    rotation: 0.0,
                    texture: self.params.burst,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Additive,
                });
            }
        }

        let ball_alpha = self.ball_alpha();
        if ball_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::BillboardFlash {
                pos: self.pos,
                size: [BALL_SIZE, BALL_SIZE],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: self.ball_texture(),
                color: [1.0, 1.0, 1.0, ball_alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut YufitelHitEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn billboards(e: &YufitelHitEffect) -> Vec<(&'static str, f32, BlendKind)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::BillboardFlash {
                    texture,
                    size,
                    blend,
                    ..
                } => (*texture, size[0], *blend),
                other => panic!("expected BillboardFlash, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn flash_grows_alone_then_the_ball_joins_it() {
        let mut e = YufitelHitEffect::new([0.0, 0.0, 0.0], YUFITEL_HIT);
        let first = billboards(&e);
        assert_eq!(first.len(), 1, "only the flash before frame 10");
        assert_eq!(first[0].0, YUFITEL_HIT.burst);
        assert_eq!(first[0].2, BlendKind::Additive);
        assert_eq!(first[0].1, BURST_GROWTH_PER_FRAME, "opens already grown");

        // Still inside the same burst, the quad has grown.
        step(&mut e, 8.0);
        let later = billboards(&e);
        assert!(later[0].1 > first[0].1, "{} > {}", later[0].1, first[0].1);

        // Past frame 10 the flash is gone and the ball is up on its own.
        step(&mut e, 4.0);
        let ball = billboards(&e);
        assert_eq!(ball.len(), 1);
        assert!(YUFITEL_HIT.ball_cycle.contains(&ball[0].0));
        assert_eq!(ball[0].1, BALL_SIZE);

        assert_eq!(e.ball_alpha(), BALL_ALPHA);
        for _ in 0..11 {
            step(&mut e, 20.0);
            assert_eq!(e.ball_alpha(), BALL_ALPHA, "holds until the hard cut");
        }
        assert_eq!(step(&mut e, 20.0), EffectStatus::Dead);
    }

    #[test]
    fn both_quads_track_the_entity() {
        let mut e = YufitelHitEffect::new([0.0, 0.0, 0.0], YUFITEL_HIT);
        step(&mut e, BALL_START_FRAME + 1.0);
        e.set_position([12.0, 3.0, -4.0]);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(!list.primitives.is_empty());
        for p in &list.primitives {
            match p {
                EffectPrimitiveDraw::BillboardFlash { pos, .. } => {
                    assert_eq!(*pos, [12.0, 3.0 + Y_OFFSET, -4.0])
                }
                other => panic!("expected BillboardFlash, got {other:?}"),
            }
        }
    }

    #[test]
    fn flash_recurs_on_its_period() {
        let mut e = YufitelHitEffect::new([0.0, 0.0, 0.0], YUFITEL_HIT);
        step(&mut e, BURST_LIFE_FRAMES + 2.0);
        assert!(e.burst_age().is_none(), "expired between periods");
        step(&mut e, BURST_PERIOD_FRAMES - BURST_LIFE_FRAMES);
        assert!(e.burst_age().is_some(), "relaunched on the next period");
    }

    #[test]
    fn both_variants_cycle_their_own_textures() {
        for params in [YUFITEL_HIT, YUFITEL2] {
            let mut e = YufitelHitEffect::new([0.0, 0.0, 0.0], params);
            step(&mut e, BALL_START_FRAME + 0.5);
            let mut seen = Vec::new();
            for _ in 0..params.ball_cycle.len() {
                seen.push(billboards(&e).last().unwrap().0);
                step(&mut e, 1.0);
            }
            assert_eq!(seen, params.ball_cycle);
            assert!(TEXTURES.contains(&params.burst));
            assert!(params.ball_cycle.iter().all(|t| TEXTURES.contains(t)));
        }
    }

    #[test]
    fn dies_with_the_launching_effect() {
        let mut e = YufitelHitEffect::new([0.0, 0.0, 0.0], YUFITEL_HIT);
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
