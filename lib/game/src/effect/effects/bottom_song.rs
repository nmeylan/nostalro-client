//! BottomSong family — Bard/Dancer song icons hovering above the actor.
//!
//! Original game dispatcher Bottom_Music(texture, F1) builds a bottom-song primitive
//! primitive with one active cell: a textured quad floating above
//! 6 units above the actor's feet, rendered upright via
//! the cloud render. The visible cue is the song icon (music note, apple,
//! kiss, etc.) above the bard.
//!
//! Faithful to original game `Bottom_Music()` per-variant choices:
//!   * **Texture pools** — F1=2 picks per-spawn from melody_a/b, F1=5
//!     from red/blue/yellow gemstone, F1=8 from spell_01..08. The
//!     spawn-time choice is hashed from the world position so it's
//!     stable for the song's lifetime.
//!   * **flag1[2] render mode** — the cloud render switches blend+tint on
//!     this flag (set per F1 in `Bottom_Music`): default 0 = additive
//!     white, 3 = alpha-blend gold (200/200/100), 4 = additive white,
//!     7 = alpha-blend white, 9 with flag1[3]=6 = alpha-blend white.
//!   * **Vertical bob** — `height[8]` (sin-oscillation amount)
//!     is 0 in original game for these songs, but the bob is kept here per
//!     direct user request: slow ~4 s cycle, ±1 unit world.
//!
//! `GI[1..3]` trail cells (F1=2's three extras with `alpha_b=0`) and
//! the F1=4 horizontal nudge `height[9]=10` (4-unit X offset on corners)
//! are dropped — the trail cells render invisible in the cloud render pass
//! anyway, and the F1=4 nudge is sub-pixel for a 30-unit quad.
//!
//! Dispatch-shape kin (`Bottom_Magnus` → 4-sided pillar via `Frustum`
//! lives in `bottom_magnus.rs`; `Bottom_Vertical`, `Bottom_Light`,
//! `Bottom_LandProtector`, `Bottom_Hermode`, `Bottom_Spr`, `Bottom_Out`)
//! use distinct primitives (bottom2, gi5, …) and need new
//! `EffectPrimitiveDraw` variants — deferred to follow-up sessions.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

/// Per-variant BottomSong parameters. Each EF_BOTTOM_* maps to one
/// [`BottomSongParams`] derived from the original game's `Bottom_Music()`
/// recipe.
#[derive(Clone, Copy, Debug)]
pub struct BottomSongParams {
    /// Per-spawn texture pool. Length-1 slices behave as a static
    /// texture; longer slices pick one entry per spawn (hashed from
    /// world_pos) — matches original game's `random(N)` selection for F1=2/5/8.
    pub textures: &'static [&'static str],
    /// Billboard half-extent in world units (full width = `2 * radius`).
    /// Derived from the original game's first orbit distance: F1=1 → 5.0,
    /// F1=4 → 15.0, F1=6 → 8.0, F1=9 → 1.5,
    /// default (0/2/3/5/7/8) → 3.0.
    pub radius: f32,
    /// Blend mode picked from the cloud render pass's first rect-render
    /// switch arg — `0` in original game = alpha blend, `1` = additive.
    pub blend: BlendKind,
    /// RGB tint (0..1). Alpha is driven by the fade-in envelope, not
    /// this field. Per the cloud flag1[2] dispatch:
    /// default/4/7 = (1, 1, 1); case 3 (Richmankim) = (200, 200, 100)/255.
    pub tint_rgb: [f32; 3],
}

const FRAMES_PER_SECOND: f32 = 60.0;
/// Original game ramps alpha from 0 to 200 over ~(200 / alpha speed)
/// frames; we approximate the visible fade-in with 30 frames (0.5 s),
/// matching the cadence of the gif references.
const FADE_IN_FRAMES: f32 = 30.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// Original game first vertex-color alpha = 200 for every Bottom_Music variant.
const BASE_ALPHA: f32 = 200.0 / 255.0;
/// The icon floats 6 units above the actor’s feet — above the
/// actor's feet (native RO `-Y` = up).
const VERTICAL_OFFSET: f32 = -6.0;
/// Vertical bob amplitude (world units). The original game's
/// `GI[0].height[8]` controls a sin-oscillation amount that's 0 in
/// `Bottom_Music`; we add a small hand-tuned wobble anyway because the
/// reference gifs show perceptible motion.
const BOB_AMPLITUDE: f32 = 1.0;
/// Vertical bob frequency in rad/s. ~1.5 Hz → one full cycle per
/// ~4 seconds, slow enough to read as breathing rather than vibrating.
const BOB_FREQ_RAD_PER_SEC: f32 = std::f32::consts::TAU * 0.25;
/// Standard UV layout: full texture from (0,0) top-left to (1,1)
/// bottom-right, matching the Billboard primitive's `uv[TL, TR, BL, BR]`
/// corner order.
const FULL_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
/// Richmankim's flag1[2]=3 case: textured rect tinted `(200, 200, 100)`.
const RICHMAN_GOLD: [f32; 3] = [200.0 / 255.0, 200.0 / 255.0, 100.0 / 255.0];

/// `Bottom_Music("cross_old.bmp", 9)` — F1=9: flag1[2]=9, flag1[3]=6 →
/// alpha-blended white textured rect (alpha blend, white).
/// distance = 1.5.
pub const GOSPEL: BottomSongParams = BottomSongParams {
    textures: &["cross_old.bmp"],
    radius: 1.5,
    blend: BlendKind::Alpha,
    tint_rgb: WHITE,
};
/// `Bottom_Music("curse.bmp", 6)` — F1=6: flag1[2]=4 → additive white.
/// distance = 8.0.
pub const EVILLAND: BottomSongParams = BottomSongParams {
    textures: &["curse.bmp"],
    radius: 8.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("kiss.bmp", 4)` — F1=4 hits the default flag1[2]=0 case
/// (no explicit set in `Bottom_Music` for F1=4). distance = 15.0.
pub const FORTUNEKISS: BottomSongParams = BottomSongParams {
    textures: &["kiss.bmp"],
    radius: 15.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("zz.bmp", 1)` — F1=1 hits the default case.
/// distance = 5.0.
pub const LULLABY: BottomSongParams = BottomSongParams {
    textures: &["zz.bmp"],
    radius: 5.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("pocket.bmp", 3)` — F1=3: flag1[2]=3 → alpha blend
/// with gold tint (200, 200, 100).
pub const RICHMANKIM: BottomSongParams = BottomSongParams {
    textures: &["pocket.bmp"],
    radius: 3.0,
    blend: BlendKind::Alpha,
    tint_rgb: RICHMAN_GOLD,
};
/// `Bottom_Music("", 2)` — F1=2 picks `melody_a`/`melody_b` per spawn
/// (`random(2)` in original game). Default flag1[2]=0.
pub const DRUMBATTLEFIELD: BottomSongParams = BottomSongParams {
    textures: &["melody_a.bmp", "melody_b.bmp"],
    radius: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("twirl.bmp", 7)` — F1=7: flag1[2]=7 → alpha blend white.
pub const RINGNIBELUNGEN: BottomSongParams = BottomSongParams {
    textures: &["twirl.bmp"],
    radius: 3.0,
    blend: BlendKind::Alpha,
    tint_rgb: WHITE,
};
/// `Bottom_Music("", 5)` — F1=5 picks red/blue/yellow gemstone per
/// spawn (`random(3)`). flag1[2]=4 → additive white.
pub const INTOABYSS: BottomSongParams = BottomSongParams {
    textures: &["redgemstone.bmp", "bluegemstone.bmp", "yellowgemstone.bmp"],
    radius: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("melody_b.bmp")` — F1 defaults to 0, flag1[2]=0.
pub const WHISTLE: BottomSongParams = BottomSongParams {
    textures: &["melody_b.bmp"],
    radius: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("", 8)` — F1=8 picks spell_01..08 per spawn
/// (`random(8)`). Default flag1[2]=0.
pub const POEMBRAGI: BottomSongParams = BottomSongParams {
    textures: &[
        "spell_01.bmp",
        "spell_02.bmp",
        "spell_03.bmp",
        "spell_04.bmp",
        "spell_05.bmp",
        "spell_06.bmp",
        "spell_07.bmp",
        "spell_08.bmp",
    ],
    radius: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("idun_apple.bmp", 6)` — F1=6: flag1[2]=4 → additive white.
pub const APPLEIDUN: BottomSongParams = BottomSongParams {
    textures: &["idun_apple.bmp"],
    radius: 8.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};
/// `Bottom_Music("melody_a.bmp")` — F1 defaults to 0.
pub const HUMMING: BottomSongParams = BottomSongParams {
    textures: &["melody_a.bmp"],
    radius: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
};

/// All textures any BottomSong variant might bind. Pool members listed
/// individually so the renderer preloads every random-pick option.
pub const TEXTURES: &[&str] = &[
    "cross_old.bmp",
    "curse.bmp",
    "kiss.bmp",
    "zz.bmp",
    "pocket.bmp",
    "melody_a.bmp",
    "melody_b.bmp",
    "twirl.bmp",
    "redgemstone.bmp",
    "bluegemstone.bmp",
    "yellowgemstone.bmp",
    "idun_apple.bmp",
    "spell_01.bmp",
    "spell_02.bmp",
    "spell_03.bmp",
    "spell_04.bmp",
    "spell_05.bmp",
    "spell_06.bmp",
    "spell_07.bmp",
    "spell_08.bmp",
];

pub struct BottomSongEffect {
    world_pos: [f32; 3],
    params: BottomSongParams,
    age: f32,
    /// Spawn-time phase offset for the vertical bob, derived from the
    /// world position so stacked songs don't oscillate in lockstep.
    /// Stored in radians.
    bob_phase: f32,
    /// Pool-selected texture for this spawn. original game's
    /// `random(N)` for F1=2/5/8 is reproduced here as a position-hashed
    /// pick — same spawn → same texture, different spawns → variety.
    texture: &'static str,
}

impl BottomSongEffect {
    pub fn new(world_pos: [f32; 3], params: BottomSongParams) -> Self {
        let bob_phase = pseudo_random_angle(&world_pos);
        // Hash the spawn position to pick a texture from the pool. We
        // can't trust `params.textures` to be non-empty if a caller
        // forgets to populate it, but the catch keeps us from
        // panicking — fall back to a stable placeholder.
        let texture = if params.textures.is_empty() {
            "alpha_center.tga"
        } else {
            let idx = (pseudo_random_index(&world_pos)) % params.textures.len();
            params.textures[idx]
        };
        Self {
            world_pos,
            params,
            age: 0.0,
            bob_phase,
            texture,
        }
    }
}

impl Effect for BottomSongEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        // BottomSong durations are minutes long; the holder kills us when
        // the spec's `duration_ms` expires, so we never self-terminate.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fade = (self.age / FADE_IN_SECS).clamp(0.0, 1.0);
        let alpha = BASE_ALPHA * fade;
        let side = self.params.radius * 2.0;
        let bob = (self.age * BOB_FREQ_RAD_PER_SEC + self.bob_phase).sin()
            * BOB_AMPLITUDE;
        let [tr, tg, tb] = self.params.tint_rgb;
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0],
                self.world_pos[1] + VERTICAL_OFFSET + bob,
                self.world_pos[2],
            ],
            size: [side, side],
            uv: FULL_UV,
            // Always upright on screen — the camera-facing billboard
            // gives the icon its viewing angle, and any screen-space
            // roll here would flip the apple / music notes.
            rotation: 0.0,
            texture: self.texture,
            color: [tr, tg, tb, alpha],
            blend: self.params.blend,
        });
    }
}

/// Deterministic-but-varied angle in [0, 2π). Hashing the spawn position
/// gives nearby spawns visibly different rotations without bringing in a
/// dependency on `rand`.
fn pseudo_random_angle(pos: &[f32; 3]) -> f32 {
    let n = (position_hash(pos) % 360) as f32;
    n.to_radians()
}

/// Pool-index hash — same shape as [`pseudo_random_angle`] but returns a
/// raw integer so callers can take it modulo their pool size.
fn pseudo_random_index(pos: &[f32; 3]) -> usize {
    position_hash(pos) as usize
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
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

    fn step(effect: &mut BottomSongEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None });
    }

    fn draws(effect: &BottomSongEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn bottom_song_emits_one_upright_billboard_per_frame() {
        // Sociable test: a freshly-spawned BottomSong yields a Billboard
        // primitive (camera-facing quad), not a flat GroundDisc. The
        // billboard sits ~6 units above the actor's feet (native RO
        // `-Y` = up) so the icon hovers, matching `imgs/250-300/284.gif`.
        // X/Z stay locked to the actor; Y is `VERTICAL_OFFSET + bob` so
        // it lands within ±BOB_AMPLITUDE of the baseline.
        let mut e = BottomSongEffect::new([5.0, 0.0, 7.0], WHISTLE);
        step(&mut e, 1.0 / 60.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::Billboard {
                pos,
                size,
                texture,
                blend,
                ..
            } => {
                assert_eq!(pos[0], 5.0);
                assert_eq!(pos[2], 7.0);
                let y_min = VERTICAL_OFFSET - BOB_AMPLITUDE - 1e-3;
                let y_max = VERTICAL_OFFSET + BOB_AMPLITUDE + 1e-3;
                assert!(
                    pos[1] >= y_min && pos[1] <= y_max,
                    "icon Y ({}) within bob band [{y_min}, {y_max}]",
                    pos[1],
                );
                assert!((size[0] - WHISTLE.radius * 2.0).abs() < f32::EPSILON);
                assert_eq!(*texture, "melody_b.bmp");
                assert_eq!(*blend, BlendKind::Additive, "Whistle is F1=0 default → additive");
            }
            other => panic!("expected Billboard, got {other:?}"),
        }
    }

    #[test]
    fn richmankim_uses_alpha_blend_with_gold_tint() {
        // Regression: original game's cloud render flag1[2]=3 picks alpha blend
        // with RGB tint (200, 200, 100). Earlier impl forced every
        // BottomSong to BlendKind::Alpha + white — Richmankim looked
        // identical to every other song.
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], RICHMANKIM);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, blend, .. } => {
                assert_eq!(*blend, BlendKind::Alpha);
                assert!((color[0] - RICHMAN_GOLD[0]).abs() < 1e-4, "R tint: {}", color[0]);
                assert!((color[1] - RICHMAN_GOLD[1]).abs() < 1e-4, "G tint: {}", color[1]);
                assert!((color[2] - RICHMAN_GOLD[2]).abs() < 1e-4, "B tint: {}", color[2]);
            }
            other => panic!("expected Billboard, got {other:?}"),
        }
    }

    #[test]
    fn poembragi_pool_picks_one_of_eight_spell_bmps() {
        // Sociable test: F1=8 in original game does `random(8)` over spell_01..08.
        // Our deterministic per-spawn hash must select one of those
        // eight, and different world positions must visibly cover
        // multiple options (not always the same one). Spawning at 32
        // distinct positions and collecting the set proves coverage.
        use std::collections::HashSet;
        let mut chosen = HashSet::new();
        for i in 0..32 {
            let pos = [i as f32 * 1.7, 0.0, i as f32 * 2.3];
            let e = BottomSongEffect::new(pos, POEMBRAGI);
            chosen.insert(e.texture);
        }
        for tex in chosen.iter() {
            assert!(
                POEMBRAGI.textures.contains(tex),
                "picked texture {tex} not in pool",
            );
        }
        assert!(
            chosen.len() >= 4,
            "expected ≥4 distinct spell_* textures across 32 spawns, got {}",
            chosen.len(),
        );
    }

    #[test]
    fn bottom_song_vertical_bob_moves_icon_over_time() {
        // Sociable test: sampling the icon Y across one bob cycle
        // (~4 seconds) yields both a peak above and a trough below the
        // baseline VERTICAL_OFFSET. Confirms the wobble plumbing is live
        // — without it the icon would sit static and read as broken.
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], HUMMING);
        let mut min_y: f32 = f32::INFINITY;
        let mut max_y: f32 = f32::NEG_INFINITY;
        for _ in 0..240 {
            step(&mut e, 1.0 / 60.0);
            if let EffectPrimitiveDraw::Billboard { pos, .. } = &draws(&e)[0] {
                min_y = min_y.min(pos[1]);
                max_y = max_y.max(pos[1]);
            }
        }
        assert!(min_y < VERTICAL_OFFSET, "saw a trough: min_y={min_y}");
        assert!(max_y > VERTICAL_OFFSET, "saw a peak: max_y={max_y}");
        assert!(
            (max_y - min_y) > BOB_AMPLITUDE,
            "spread over the 4 s window should exceed amplitude ({})",
            BOB_AMPLITUDE,
        );
    }

    #[test]
    fn bottom_song_alpha_fades_in_then_holds() {
        // Sociable test: spawn → step into the fade window, then well past
        // it, and check alpha climbs from 0 to BASE_ALPHA.
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], LULLABY);
        step(&mut e, 0.0);
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a0.abs() < 1e-4, "starts fully transparent at spawn");

        // Mid fade-in (~0.25 s).
        step(&mut e, FADE_IN_SECS * 0.5);
        let a_mid = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_mid > a0 && a_mid < BASE_ALPHA, "rising: {a_mid}");

        // Past the fade-in window — clamped at BASE_ALPHA.
        step(&mut e, FADE_IN_SECS);
        let a_full = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(
            (a_full - BASE_ALPHA).abs() < 1e-4,
            "held at BASE_ALPHA: {a_full}",
        );
    }
}
