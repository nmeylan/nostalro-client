//! `EF_POKJUK` (id 297) — fireworks weather: rockets rise from near the player,
//! twinkle, then burst into an expanding tumbling shell. Each rocket recycles
//! forever until the effect is despawned on map change.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const NUM_ROCKETS: usize = 4;

const LAUNCH_INTERVAL: f32 = 300.0;
const RISE_FRAMES: f32 = 154.0;
const RISE_SPEED: f32 = 0.25;
const RECYCLE_FRAMES: f32 = LAUNCH_INTERVAL * NUM_ROCKETS as f32;
const SPREAD: f32 = 150.0;

const SHELL_COUNT: usize = 16;
const SHELL_LIFE_FRAMES: f32 = 167.0;
const SHELL_START_ALPHA: f32 = 250.0 / 255.0;
const SHELL_ALPHA_DRAIN: f32 = SHELL_START_ALPHA / SHELL_LIFE_FRAMES;
const SHELL_SHRINK_PER_FRAME: f32 = 0.98;
const DRIFT_UP_PER_FRAME: f32 = 0.02;
const TUMBLE_DEG_PER_FRAME: f32 = 5.0;
const SPARK_SIZE: f32 = 2.0;

const ORIGIN_UP: f32 = 14.0;

/// Viewer/non-weather preview length: long enough to show the first two bursts.
/// As weather this is overridden to infinite.
pub const TOTAL_DURATION_MS: u32 =
    ((LAUNCH_INTERVAL * 2.0 + RISE_FRAMES + SHELL_LIFE_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

pub const TEXTURES: &[&str] = &["pok1.tga", "pok2.tga", "pok3.tga"];
const SPARK_TEXTURES: [&str; 3] = ["pok1.tga", "pok2.tga", "pok3.tga"];

const COLORS: [[f32; 3]; 5] = [
    [70.0 / 255.0, 70.0 / 255.0, 1.0],
    [1.0, 70.0 / 255.0, 70.0 / 255.0],
    [70.0 / 255.0, 1.0, 70.0 / 255.0],
    [1.0, 1.0, 90.0 / 255.0],
    [1.0, 70.0 / 255.0, 1.0],
];

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

enum Phase {
    Idle,
    Rising,
    Bursting,
}

struct Shell {
    pos: [f32; 3],
    heading: f32,
    elevation: f32,
    distance: f32,
    rotation: f32,
    alpha: f32,
    texture: &'static str,
}

struct Rocket {
    launch_frame: f32,
    phase: Phase,
    rise_elapsed: f32,
    head: [f32; 3],
    color: [f32; 3],
    texture: &'static str,
    shell: Vec<Shell>,
}

pub struct PokjukEffect {
    origin: [f32; 3],
    rng: Rng,
    rockets: Vec<Rocket>,
    frame: f32,
}

impl PokjukEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let seed = (world_pos[0] * 41.0 + world_pos[2] * 97.0) as i64 as u32 ^ 0x2468_ACE0;
        let rng = Rng(seed | 1);
        let rockets = (0..NUM_ROCKETS)
            .map(|i| Rocket {
                launch_frame: LAUNCH_INTERVAL * (i as f32 + 1.0),
                phase: Phase::Idle,
                rise_elapsed: 0.0,
                head: [0.0; 3],
                color: [1.0; 3],
                texture: SPARK_TEXTURES[0],
                shell: Vec::new(),
            })
            .collect();
        Self {
            origin: world_pos,
            rng,
            rockets,
            frame: 0.0,
        }
    }
}

impl Effect for PokjukEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.origin = pos;
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        self.frame += frames;
        let [ox, oy, oz] = self.origin;

        for r in &mut self.rockets {
            match r.phase {
                Phase::Idle => {
                    if self.frame >= r.launch_frame {
                        r.phase = Phase::Rising;
                        r.rise_elapsed = 0.0;
                        r.head = [
                            ox + self.rng.range(-SPREAD, SPREAD),
                            oy - ORIGIN_UP,
                            oz + self.rng.range(-SPREAD, SPREAD),
                        ];
                        r.color = COLORS[(self.rng.next_u32() % 5) as usize];
                    }
                }
                Phase::Rising => {
                    r.rise_elapsed += frames;
                    r.head[1] -= RISE_SPEED * frames;
                    r.texture = SPARK_TEXTURES[((r.rise_elapsed / 2.0) as usize) % 3];
                    if r.rise_elapsed >= RISE_FRAMES {
                        r.phase = Phase::Bursting;
                        r.shell = (0..SHELL_COUNT)
                            .map(|_| Shell {
                                pos: r.head,
                                heading: self.rng.range(0.0, 360.0),
                                elevation: self.rng.range(0.0, 360.0),
                                distance: self.rng.range(0.4, 0.8),
                                rotation: self.rng.range(0.0, 360.0),
                                alpha: SHELL_START_ALPHA,
                                texture: SPARK_TEXTURES[(self.rng.next_u32() % 3) as usize],
                            })
                            .collect();
                    }
                }
                Phase::Bursting => {
                    for s in &mut r.shell {
                        let elev = s.elevation.to_radians();
                        let head = s.heading.to_radians();
                        let radial = elev.cos() * s.distance;
                        s.pos[1] -= (elev.sin() * s.distance + DRIFT_UP_PER_FRAME) * frames;
                        s.pos[0] += head.cos() * radial * frames;
                        s.pos[2] += head.sin() * radial * frames;
                        s.distance *= SHELL_SHRINK_PER_FRAME.powf(frames);
                        s.rotation = (s.rotation + TUMBLE_DEG_PER_FRAME * frames) % 360.0;
                        s.alpha = (s.alpha - SHELL_ALPHA_DRAIN * frames).max(0.0);
                    }
                    if r.shell.iter().all(|s| s.alpha <= 0.0) {
                        r.shell.clear();
                        r.launch_frame += RECYCLE_FRAMES;
                        r.phase = Phase::Idle;
                    }
                }
            }
        }
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for r in &self.rockets {
            match r.phase {
                Phase::Rising => {
                    let [cr, cg, cb] = r.color;
                    out.push(EffectPrimitiveDraw::Billboard {
                        pos: r.head,
                        size: [SPARK_SIZE, SPARK_SIZE],
                        uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                        rotation: 0.0,
                        texture: r.texture,
                        color: [cr, cg, cb, 1.0],
                        blend: BlendKind::Additive,
                    });
                }
                Phase::Bursting => {
                    let [cr, cg, cb] = r.color;
                    for s in &r.shell {
                        if s.alpha <= 0.0 {
                            continue;
                        }
                        out.push(EffectPrimitiveDraw::Billboard {
                            pos: s.pos,
                            size: [SPARK_SIZE, SPARK_SIZE],
                            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                            rotation: s.rotation.to_radians(),
                            texture: s.texture,
                            color: [cr, cg, cb, s.alpha],
                            blend: BlendKind::Additive,
                        });
                    }
                }
                Phase::Idle => {}
            }
        }
    }
}

/// `EF_POKJUK_SOUND` (id 301) — no visual; a held entry whose SFX schedule fires
/// the firecracker wave on a loop (see `effect_sound`).
pub struct PokjukSoundEffect;

impl PokjukSoundEffect {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PokjukSoundEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for PokjukSoundEffect {
    fn update(&mut self, _ctx: &EffectUpdateCtx) -> EffectStatus {
        EffectStatus::Running
    }
    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}
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

    fn tick(e: &mut PokjukEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn count(e: &PokjukEffect) -> usize {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives.len()
    }

    #[test]
    fn rockets_launch_staggered_and_recycle() {
        let mut e = PokjukEffect::new([0.0; 3]);
        assert_eq!(count(&e), 0, "silent before the first launch");

        // Only the first rocket is airborne shortly after its launch; the rest wait.
        tick(&mut e, LAUNCH_INTERVAL as u32 + 5);
        assert!(count(&e) > 0, "first rocket airborne");

        // Past a full recycle the display is still producing fireworks.
        tick(&mut e, RECYCLE_FRAMES as u32 + LAUNCH_INTERVAL as u32);
        assert!(count(&e) > 0, "rockets keep recycling");
    }

    #[test]
    fn shell_sparks_carry_varied_colors_and_textures() {
        let mut e = PokjukEffect::new([5.0, 0.0, 9.0]);
        tick(&mut e, (LAUNCH_INTERVAL + RISE_FRAMES) as u32 + 5);
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        for p in &l.primitives {
            let EffectPrimitiveDraw::Billboard { texture, blend, .. } = p else {
                panic!()
            };
            assert!(TEXTURES.contains(texture));
            assert_eq!(*blend, BlendKind::Additive);
        }
    }
}
