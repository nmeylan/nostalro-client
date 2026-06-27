use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const WORLD_SCALE: f32 = 0.35;
const BUBBLE_TEXTURE: &str = "bubble_a.bmp";
pub const TEXTURES: &[&str] = &[BUBBLE_TEXTURE];

const SPAWN_HEIGHT: f32 = 50.0;
const FALL_FRAMES: f32 = 50.0;
const FALL_SPEED_PER_FRAME: f32 = 1.0;
const SPIN_DEG_PER_FRAME: f32 = -15.0;

const ALPHA_RISE_PER_FRAME: f32 = 20.0;
const ALPHA_MAX_255: f32 = 230.0;
const ALPHA_FALL_PER_FRAME: f32 = 10.0;

const WOBBLE_FRACTION: f32 = 0.05;

const BUBBLE_COLOR: [f32; 3] = [130.0 / 255.0, 130.0 / 255.0, 250.0 / 255.0];
const BUBBLE_SIZE: f32 = 2.5;

const ECHO_ALPHA_DROP_255: [f32; 3] = [100.0, 125.0, 150.0];
const ECHO_LAG_FRAMES: usize = 2;

const FADE_FRAMES: f32 = ALPHA_MAX_255 / ALPHA_FALL_PER_FRAME;
const TOTAL_FRAMES: f32 = FALL_FRAMES + FADE_FRAMES + ECHO_ALPHA_DROP_255.len() as f32;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

#[derive(Clone, Copy)]
struct Snapshot {
    pos: [f32; 3],
    spin_rad: f32,
    size: f32,
    alpha_255: f32,
}

pub struct BubbleDropEffect {
    anchor: [f32; 3],
    history: Vec<Snapshot>,
    age_frames: f32,
}

impl BubbleDropEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        Self {
            anchor,
            history: Vec::new(),
            age_frames: 0.0,
        }
    }

    fn main_state(&self, f: f32) -> Snapshot {
        let fall = (FALL_SPEED_PER_FRAME * f).min(FALL_SPEED_PER_FRAME * FALL_FRAMES);
        let y = self.anchor[1] + (-SPAWN_HEIGHT + fall) * WORLD_SCALE;
        let phase = (SPIN_DEG_PER_FRAME * f).to_radians();
        let size = BUBBLE_SIZE * (1.0 + phase.sin() * WOBBLE_FRACTION);
        let alpha = if f <= FALL_FRAMES {
            (ALPHA_RISE_PER_FRAME * f).min(ALPHA_MAX_255)
        } else {
            (ALPHA_MAX_255 - ALPHA_FALL_PER_FRAME * (f - FALL_FRAMES)).max(0.0)
        };
        Snapshot {
            pos: [self.anchor[0], y, self.anchor[2]],
            spin_rad: phase,
            size,
            alpha_255: alpha,
        }
    }
}

impl Effect for BubbleDropEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        self.history.push(self.main_state(self.age_frames));
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.history.is_empty() {
            return;
        }
        let push = |out: &mut EffectDrawList, s: Snapshot, alpha_255: f32| {
            if alpha_255 <= 0.0 {
                return;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: s.pos,
                size: [s.size, s.size],
                uv: UNIT_UV,
                rotation: s.spin_rad,
                texture: BUBBLE_TEXTURE,
                color: [
                    BUBBLE_COLOR[0],
                    BUBBLE_COLOR[1],
                    BUBBLE_COLOR[2],
                    alpha_255 / 255.0,
                ],
                blend: BlendKind::Additive,
            });
        };

        let last = self.history.len() - 1;
        for (i, drop) in ECHO_ALPHA_DROP_255.iter().enumerate() {
            let lag = (i + 1) * ECHO_LAG_FRAMES;
            if last >= lag {
                let s = self.history[last - lag];
                push(out, s, s.alpha_255 - drop);
            }
        }
        let main = self.history[last];
        push(out, main, main.alpha_255);
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

    fn step(e: &mut BubbleDropEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn bubbles(e: &BubbleDropEffect) -> Vec<([f32; 3], f32)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, color, .. } => Some((*pos, color[3])),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn bubble_falls_over_frames() {
        let mut e = BubbleDropEffect::new([0.0; 3]);
        step(&mut e, 5);
        let early = bubbles(&e).last().unwrap().0[1];
        step(&mut e, 20);
        let later = bubbles(&e).last().unwrap().0[1];
        assert!(later > early, "bubble descends: {early} -> {later}");
    }

    #[test]
    fn echoes_trail_with_decreasing_alpha() {
        let mut e = BubbleDropEffect::new([0.0; 3]);
        step(&mut e, 24);
        let draws = bubbles(&e);
        assert!(
            draws.len() >= 4,
            "main + 3 echoes visible, got {}",
            draws.len()
        );
        let main_alpha = draws.last().unwrap().1;
        assert!(
            draws[..draws.len() - 1]
                .iter()
                .all(|(_, a)| *a < main_alpha),
            "echoes dimmer than main: {draws:?}"
        );
    }

    #[test]
    fn alpha_ramps_then_fades_and_terminates() {
        let mut e = BubbleDropEffect::new([0.0; 3]);
        step(&mut e, FALL_FRAMES as u32);
        let peak = bubbles(&e).last().unwrap().1;
        let status = step(&mut e, (FADE_FRAMES as u32) + 5);
        assert!(peak > 0.5, "alpha ramps up to cap during the fall: {peak}");
        assert_eq!(status, EffectStatus::Dead);
    }
}
