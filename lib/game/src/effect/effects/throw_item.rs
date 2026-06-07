//! `EF_THROWITEM` family (Throwitem .. Throwitem10) — ballistic-arc item
//! projectiles. A spinning, camera-facing item-icon quad flies from the
//! caster to the target on a parabola-like arc, trails a few fading
//! after-images, then fades out on landing.
//!
//! Reference gifs (`ro-effects/effects/imgs/<bucket>/<id>.gif`) are
//! uninformative here — they were captured with no throw target, so the arc
//! is degenerate and the dark icon quad is barely visible. The motion is
//! reconstructed from the original game's observed throwing arc.
//!
//!
//! One `ThrowItemEffect` struct covers all ten ids via a [`ThrowItemParams`]
//! table. The arc variant is selected per effect:
//!   * F1 0/1/10/11 — default arc (peak 30, step `dist/25`, slow spin)
//!   * F1 2 (stone) — low arc (peak 10, fixed step 2, slow spin)
//!   * F1 3 (coin)  — flat hop (peak 1, fixed step 3, fast spin)
//! `Throwitem4` is a composite: two staggered projectiles (molotov launches
//! at frame 5, the second item at frame 10).
//!
//! Texture note: each effect throws the matching item icon. The classic GRF
//! stores item icons under their Korean names in
//! `data/texture/유저인터페이스/item/`.
//!
//! Of the ten, six icons are present
//! (298 염산병, 308 돌, 600 베넘나이프, 613 수리검, 614 쿠나이_독, 616
//! `effect/coin_a.bmp`); 615 names the faithful 풍마_뇌우 (absent here) with
//! the present 풍마_대차륜 sibling as a `|`-separated alias fallback; and
//! 299/539/541 have no classic icon (they render nothing).
//!
//!
//! A texture field may list `|`-separated alias candidates — the first one
//! present in the GRF wins (resolved in the renderer's `texture_lookup`). Per
//! candidate: a name containing a path resolves relative to `data/texture/`;
//! a bare name resolves under `data/texture/effect/` (see
//! `effect_texture_paths`).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

/// Item icons to preload. Korean names live under the item dir; `coin_a` is
/// an effect-dir texture referenced by its bare name.
pub const TEXTURES: &[&str] = &[
    "유저인터페이스/item/염산병.bmp",
    "유저인터페이스/item/돌.bmp",
    "유저인터페이스/item/베넘나이프.bmp",
    "유저인터페이스/item/수리검.bmp",
    "유저인터페이스/item/쿠나이_독.bmp",
    "유저인터페이스/item/풍마_뇌우.bmp",
    "유저인터페이스/item/풍마_대차륜.bmp",
    "coin_a.bmp",
];

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const FPS: f32 = 60.0;
const FRAME_DT: f32 = 1.0 / FPS;

/// Throw originates at hand/chest height (native coords: −Y is up): 8 units
/// above the caster's feet.
const HAND_Y: f32 = -8.0;

/// Per-frame alpha decay once the projectile has landed (20 out of 255 per
/// frame).
const LAND_FADE_PER_FRAME: f32 = 20.0 / 255.0;

/// Backstop lifetime. The effect self-terminates (`EffectStatus::Dead`) when
/// every projectile has landed and faded, so this is only a cap for the
/// holder: worst case ≈ 10f delay + 25f flight + ~13f fade.
const MAX_TOTAL_FRAMES: f32 = 90.0;
pub const TOTAL_DURATION_MS: u32 = (MAX_TOTAL_FRAMES / FPS * 1000.0) as u32;

/// Tuning for one thrown item.
#[derive(Clone, Copy)]
pub struct ThrowItemParams {
    pub texture: &'static str,
    /// The quad's corner radius. The quad is
    /// a diamond with its 4 corners at this radius, so the rendered
    /// item's side length is `distance * √2` (see `collect_draws`).
    pub distance: f32,
    /// Arc peak height above the caster→target line.
    pub arc_peak: f32,
    /// Screen-space spin added per frame, in degrees.
    pub spin_deg: f32,
    /// Frame the projectile starts flying (staggers composite throws).
    pub launch_delay_frames: u32,
    /// Horizontal step per frame. `None` = `ground_dist / 25` (default arc);
    /// `Some(v)` = fixed step (stone/coin).
    pub fixed_step: Option<f32>,
}

const fn default_throw(texture: &'static str, launch_delay_frames: u32) -> ThrowItemParams {
    ThrowItemParams {
        texture,
        distance: 3.0,
        arc_peak: 30.0,
        spin_deg: 10.0,
        launch_delay_frames,
        fixed_step: None,
    }
}

const fn stone_throw(texture: &'static str) -> ThrowItemParams {
    ThrowItemParams {
        texture,
        distance: 2.0,
        arc_peak: 10.0,
        spin_deg: 10.0,
        launch_delay_frames: 0,
        fixed_step: Some(2.0),
    }
}

const fn coin_throw(texture: &'static str) -> ThrowItemParams {
    ThrowItemParams {
        texture,
        distance: 4.0,
        arc_peak: 1.0,
        spin_deg: 45.0,
        launch_delay_frames: 0,
        fixed_step: Some(3.0),
    }
}

// 298 Acid Terror — throws an acid bottle (염산병).
pub const THROW_BOTTLES: ThrowItemParams = default_throw("유저인터페이스/item/염산병.bmp", 0);
// 299 fireworks throw — no classic icon (renders nothing).
pub const THROW_ITEM2: ThrowItemParams = default_throw("throwitem2.bmp", 0);
// 308 Throw Stone (돌).
pub const THROW_STONE: ThrowItemParams = stone_throw("유저인터페이스/item/돌.bmp");
// 539 Throw Random Bottle — composite; no classic icon (disabled).
pub const THROW_MOLOTOV: ThrowItemParams = default_throw("molotov.bmp", 5);
pub const THROW_ITEM3: ThrowItemParams = default_throw("throwitem3.bmp", 10);
// 541 — no classic icon (disabled).
pub const THROW_ITEM4: ThrowItemParams = default_throw("throwitem4.bmp", 0);
// 600 Throw Venom Knife (베넘나이프).
pub const THROW_ITEM6: ThrowItemParams = coin_throw("유저인터페이스/item/베넘나이프.bmp");
// 613 Throw Shuriken (수리검).
pub const THROW_ITEM7: ThrowItemParams = coin_throw("유저인터페이스/item/수리검.bmp");
// 614 Throw Kunai (쿠나이_독).
pub const THROW_ITEM8: ThrowItemParams = coin_throw("유저인터페이스/item/쿠나이_독.bmp");
// 615 Throw Fuuma Shuriken. The faithful icon is the thunderstorm variant
// (풍마_뇌우), absent from this GRF — fall back to the Grand Wheel huuma
// shuriken (풍마_대차륜), the closest present sibling, via the `|` alias list.
pub const THROW_ITEM9: ThrowItemParams = coin_throw(concat!(
    "유저인터페이스/item/풍마_뇌우.bmp",
    "|",
    "유저인터페이스/item/풍마_대차륜.bmp",
));
// 616 Throw Money (effect/coin_a.bmp).
pub const THROW_COIN: ThrowItemParams = coin_throw("coin_a.bmp");

/// One in-flight projectile. State is integrated per fixed frame, matching
/// the original game's throw motion.
struct Projectile {
    params: ThrowItemParams,
    sin_h: f32,
    cos_h: f32,
    step: f32,
    steps_to_target: f32,
    y_lerp_per_frame: f32,

    pos: [f32; 3],
    base_y: f32,
    spin_deg: f32,
    alpha: f32,
    flight_frame: u32,
    started: bool,
    landed: bool,
    dead: bool,
    /// Recent positions for the fading after-image trail.
    history: Vec<[f32; 3]>,
}

impl Projectile {
    fn new(from: [f32; 3], to: [f32; 3], params: ThrowItemParams) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let ground_dist = (dx * dx + dz * dz).sqrt();
        let (sin_h, cos_h) = if ground_dist > 1e-3 {
            dx.atan2(dz).sin_cos()
        } else {
            (0.0, 1.0)
        };

        let dist = ground_dist.max(1e-3);
        let step = params.fixed_step.unwrap_or(dist / 25.0).max(1e-3);
        let steps_to_target = (dist / step).ceil().max(1.0);
        let y_lerp_per_frame = (to[1] - from[1]) / steps_to_target;
        let base_y = from[1] + HAND_Y;

        Self {
            params,
            sin_h,
            cos_h,
            step,
            steps_to_target,
            y_lerp_per_frame,
            pos: [from[0], base_y, from[2]],
            base_y,
            spin_deg: 0.0,
            alpha: 1.0,
            flight_frame: 0,
            started: false,
            landed: false,
            dead: false,
            history: Vec::with_capacity(8),
        }
    }

    fn step_frame(&mut self) {
        self.started = true;
        self.spin_deg = (self.spin_deg + self.params.spin_deg) % 360.0;

        let b = self.flight_frame as f32 / self.steps_to_target;
        if b > 1.05 {
            self.landed = true;
            self.alpha -= LAND_FADE_PER_FRAME;
            if self.alpha <= 0.0 {
                self.alpha = 0.0;
                self.dead = true;
            }
        } else {
            self.base_y += self.y_lerp_per_frame;
            let hump = (std::f32::consts::PI * b.min(1.0)).sin() * self.params.arc_peak;
            // −Y is up: subtract the hump so the projectile rises.
            self.pos[1] = self.base_y - hump;
            self.pos[0] += self.step * self.sin_h;
            self.pos[2] += self.step * self.cos_h;
        }

        self.history.push(self.pos);
        if self.history.len() > 8 {
            self.history.remove(0);
        }
        self.flight_frame += 1;
    }
}

pub struct ThrowItemEffect {
    projectiles: Vec<Projectile>,
    frame: u32,
    time_accum: f32,
}

impl ThrowItemEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], variants: &[ThrowItemParams]) -> Self {
        Self {
            projectiles: variants
                .iter()
                .map(|p| Projectile::new(from, to, *p))
                .collect(),
            frame: 0,
            time_accum: 0.0,
        }
    }

    fn tick(&mut self) {
        self.frame += 1;
        for p in &mut self.projectiles {
            if !p.dead && self.frame >= p.params.launch_delay_frames {
                p.step_frame();
            }
        }
    }
}

impl Effect for ThrowItemEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.time_accum += ctx.delta;
        while self.time_accum >= FRAME_DT {
            self.time_accum -= FRAME_DT;
            self.tick();
        }
        if self.projectiles.iter().all(|p| p.dead) {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // After-image alpha falloff for the three trailing quads, sampled
        // every other history frame (the trail updates on
        // alternating frames with a sharp first drop).
        const TRAIL_ALPHA: [f32; 3] = [0.4, 0.25, 0.1];

        for p in &self.projectiles {
            if !p.started || p.dead || p.alpha <= 0.0 {
                continue;
            }
            // The quad is a diamond with corners at `distance`;
            // its side length (the rendered item's width) is `distance * √2`.
            let side = p.params.distance * std::f32::consts::SQRT_2;
            let size = [side, side];
            let rotation = p.spin_deg.to_radians();

            for (k, factor) in TRAIL_ALPHA.iter().enumerate() {
                let idx = p.history.len().checked_sub(3 + 2 * k);
                if let Some(i) = idx {
                    out.push(EffectPrimitiveDraw::Billboard {
                        pos: p.history[i],
                        size,
                        uv: UNIT_UV,
                        rotation,
                        texture: p.params.texture,
                        color: [1.0, 1.0, 1.0, p.alpha * factor],
                        blend: BlendKind::Alpha,
                    });
                }
            }

            out.push(EffectPrimitiveDraw::Billboard {
                pos: p.pos,
                size,
                uv: UNIT_UV,
                rotation,
                texture: p.params.texture,
                color: [1.0, 1.0, 1.0, p.alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut ThrowItemEffect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn billboards(e: &ThrowItemEffect) -> Vec<([f32; 3], &'static str, [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, texture, color, .. } => (*pos, *texture, *color),
                _ => panic!("throw item only emits Billboard"),
            })
            .collect()
    }

    /// The full-alpha lead quad (the projectile itself), ignoring trail
    /// after-images.
    fn lead(e: &ThrowItemEffect) -> ([f32; 3], &'static str) {
        let b = billboards(e);
        let (pos, tex, _) = b.iter().max_by(|a, c| a.2[3].total_cmp(&c.2[3])).copied().unwrap();
        (pos, tex)
    }

    #[test]
    fn variants_use_their_own_texture() {
        for (params, tex) in [
            (THROW_STONE, "유저인터페이스/item/돌.bmp"),
            (THROW_COIN, "coin_a.bmp"),
        ] {
            let mut e = ThrowItemEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 20.0], &[params]);
            step(&mut e, FRAME_DT);
            assert_eq!(lead(&e).1, tex);
        }
    }

    #[test]
    fn projectile_arcs_up_then_returns_and_advances_toward_target() {
        // Native coords: −Y is up, so the hump drives pos.y below the start
        // height mid-flight and back toward it near landing. Horizontal
        // distance from the caster grows monotonically.
        let mut e = ThrowItemEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 30.0], &[THROW_STONE]);
        let mut ys = Vec::new();
        let mut dists = Vec::new();
        for _ in 0..15 {
            step(&mut e, FRAME_DT);
            let (pos, _) = lead(&e);
            ys.push(pos[1]);
            dists.push(pos[2]);
        }
        let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
        assert!(min_y < HAND_Y - 1.0, "projectile rises above hand height mid-arc");
        assert!(*ys.last().unwrap() > min_y, "comes back down toward the end");
        assert!(dists.windows(2).all(|w| w[1] >= w[0]), "advances toward target");
    }

    #[test]
    fn composite_throwitem4_staggers_its_two_projectiles() {
        let mut e = ThrowItemEffect::new(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 20.0],
            &[THROW_MOLOTOV, THROW_ITEM3],
        );
        // Molotov launches at frame 5, the second item at frame 10. After 7
        // frames only the molotov is in flight.
        for _ in 0..7 {
            step(&mut e, FRAME_DT);
        }
        let started: usize = e.projectiles.iter().filter(|p| p.started).count();
        assert_eq!(started, 1, "only the molotov has launched at frame 7");
    }

    #[test]
    fn lands_and_dies() {
        let mut e = ThrowItemEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 20.0], &[THROW_COIN]);
        let mut status = EffectStatus::Running;
        for _ in 0..(MAX_TOTAL_FRAMES as u32) {
            status = step(&mut e, FRAME_DT);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
