//! BottomSong family — Bard/Dancer song icons hovering above the actor.
//!
//! Each song is one camera-facing cloud quad with one active cell,
//! animated per frame. The behaviour:
//!
//!   * **Size** — the quad's 4 corners sit at radius
//!     `Rx = distance + sin(pulse) * distance * 0.05` and 90° apart, so
//!     `distance` is the corner-radius (half-diagonal); the on-screen edge is
//!     `distance * √2`. There is **no** separate size term and **no** orbit —
//!     the corners *are* the quad. `distance` per F1: 1 → 5, 4 → 15, 6 → 8,
//!     9 → 1.5, else → 3. The pulse phase advances 5°/frame → a ±5% size pulse.
//!   * **Bob** — icon Y `= (pos.y - 6) + 3 * sin(phase)` with the phase
//!     `= random(360)` at spawn, advancing `+1`/frame → ±3-unit vertical
//!     bob on a 6-second cycle.
//!   * **Rotation** — fixed at 45° (axis-aligned) and is static
//!     except F1=7 (RingNibelungen), which spins it `+= 10`/frame.
//!   * **Echo** — only F1=2 (Drumbattlefield) activates 3 extra cells;
//!     they shift every 6 frames to a steady state of 4
//!     concentric copies at `distance + 0.5*i` and alpha `200 - 50*i`
//!     (a faint expanding ripple). Other songs use cell 0 only.
//!   * **Blend + tint** — the blend mode and tint are chosen per F1:
//!     some songs draw **additive** (the texture adds onto the scene,
//!     brightening) and some draw **alpha** (the texture composites over
//!     the scene),
//!     with an `(r,g,b)` tint multiplied into the texture. The set per F1:
//!       - F1 0/1/2/4/8 → additive, tint (130,130,250) light-blue
//!       - F1 3 (Richmankim) → additive, tint (200,200,100) gold
//!       - F1 5/6 (Intoabyss/EvilLand/AppleIdun) → alpha, white
//!       - F1 7 (RingNibelungen) → additive, white
//!       - F1 9 (Gospel) → additive, white
//!   * **Texture pools** — F1=2 picks per-spawn from melody_a/b, F1=8 from
//!     spell_01..08 (a random pick); reproduced here as a
//!     position-hashed pick so the song's texture is stable for its lifetime.
//!   * **Gemstone sprites (F1=5, Intoabyss)** — there is no `gemstone.bmp`;
//!     a random pick selects an actual gemstone **item sprite** (715 yellow /
//!     716 red / 717 blue, resolved via `idnum2itemresnametable.txt`),
//!     rendered as a `SpriteParticle` rather than a textured quad.
//!   * **F1=4 nudge** — FortuneKiss shifts the corners +4 on
//!     world X (the icon sits slightly off-centre).
//!
//! Sibling song families (`Bottom_Vertical`, `Bottom_Light`, `Bottom_Out`,
//! `Bottom_Magnus`, …) use distinct primitives and live in sibling files.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

/// Per-variant BottomSong parameters.
#[derive(Clone, Copy, Debug)]
pub struct BottomSongParams {
    /// Per-spawn texture pool for the Billboard `.bmp` icon path. Length-1
    /// slices behave as a static texture; longer slices pick one per spawn
    /// (hashed from world_pos) — the random pick for
    /// F1=2/8. Empty when `sprites` is used instead.
    pub textures: &'static [&'static str],
    /// Per-spawn SPR pool for the sprite-particle path. Non-empty only for
    /// Intoabyss (F1=5), which randomly picks a gemstone **item sprite**
    /// (715/716/717), not a texture — the classic client renders the actual
    /// gemstone item SPR, there is no `gemstone.bmp`.
    pub sprites: &'static [&'static str],
    /// The quad's corner-radius (half-diagonal)
    /// in world units. The on-screen edge length is `distance * √2`.
    pub distance: f32,
    /// Blend mode: additive or alpha. See module docs.
    pub blend: BlendKind,
    /// RGB tint (0..1) that multiplies the texture. Alpha comes from the
    /// per-cell base alpha and the fade-in envelope, not this field.
    pub tint_rgb: [f32; 3],
    /// F1=7 (RingNibelungen): the icon spins (rotation `+= 10`/frame).
    pub spin: bool,
    /// Active echo cells. 1 for every song except F1=2 (Drumbattlefield), which
    /// runs 4 concentric expanding/fading copies.
    pub cells: u8,
    /// F1=4 (FortuneKiss): icon nudged +4 on world X.
    pub x_nudge: f32,
}

const FRAMES_PER_SECOND: f32 = 60.0;
/// The original game does not ramp the song's alpha (it spawns at 200), but a
/// short fade-in reads better against the abrupt pop and matches the gif
/// cadence; kept as a gentle 0.5 s envelope.
const FADE_IN_FRAMES: f32 = 30.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// Cell-0 base alpha (out of 255); trail cells fall 50 each.
const ALPHA_B0: f32 = 200.0;
/// Icon Y `= pos.y - 6.0` — icon floats 6 units above the feet
/// (native RO `-Y` = up).
const VERTICAL_OFFSET: f32 = -6.0;
/// Icon Y `+= 3.0 * sin(phase)` — ±3-unit vertical bob.
const BOB_AMPLITUDE: f32 = 3.0;
/// Bob phase `+= 1`/frame → one bob cycle per 360 frames (6 s).
const BOB_SPEED_DEG_PER_FRAME: f32 = 1.0;
/// Pulse phase `+= 5`/frame → the ±5% size pulse phase.
const PULSE_SPEED_DEG_PER_FRAME: f32 = 5.0;
/// `Rx += sin(pulse) * distance * 0.05` — pulse amplitude.
const PULSE_AMPLITUDE: f32 = 0.05;
/// F1=7 rotation `+= 10`/frame.
const SPIN_SPEED_DEG_PER_FRAME: f32 = 10.0;
/// distance is the corner-radius; on-screen edge = `distance * √2`.
const EDGE_PER_DISTANCE: f32 = std::f32::consts::SQRT_2;
/// Standard UV layout for the Billboard `uv[TL, TR, BL, BR]` corner order.
const FULL_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
/// Light-blue tint (130, 130, 250) used by most songs.
const LIGHT_BLUE: [f32; 3] = [130.0 / 255.0, 130.0 / 255.0, 250.0 / 255.0];
/// Richmankim gold tint (200, 200, 100).
const RICHMAN_GOLD: [f32; 3] = [200.0 / 255.0, 200.0 / 255.0, 100.0 / 255.0];

/// Intoabyss gemstone **item sprites** (a random pick over 715/716/717), looked
/// up via `idnum2itemresnametable.txt` → Korean resource names under
/// `data/sprite/아이템/`. Rendered as `SpriteParticle` (no `.bmp` exists).
pub const GEMSTONE_SPRITES: &[&str] = &[
    "data/sprite/아이템/옐로우젬스톤", // 715 yellow
    "data/sprite/아이템/레드젬스톤",   // 716 red
    "data/sprite/아이템/블루젬스톤",   // 717 blue
];
/// SPR paths any BottomSong variant might bind (preloaded via
/// `custom_effect_sprite_paths`).
pub const SPRITES: &[&str] = GEMSTONE_SPRITES;
/// Native-size multiplier for the gemstone item sprite (icon-sized).
const GEMSTONE_SIZE: f32 = 1.0;

/// `cross_old.bmp`, F1=9 (Gospel) → additive
/// white, distance 1.5.
pub const GOSPEL: BottomSongParams = BottomSongParams {
    textures: &["cross_old.bmp"],
    distance: 1.5,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// `curse.bmp`, F1=6 → alpha white, distance 8.
pub const EVILLAND: BottomSongParams = BottomSongParams {
    textures: &["curse.bmp"],
    distance: 8.0,
    blend: BlendKind::Alpha,
    tint_rgb: WHITE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// `kiss.bmp`, F1=4 → additive light-blue,
/// distance 15, +4 X-nudge.
pub const FORTUNEKISS: BottomSongParams = BottomSongParams {
    textures: &["kiss.bmp"],
    distance: 15.0,
    blend: BlendKind::Additive,
    tint_rgb: LIGHT_BLUE,
    spin: false,
    cells: 1,
    x_nudge: 4.0,
    sprites: &[],
};
/// `zz.bmp`, F1=1 → additive light-blue, distance 5.
pub const LULLABY: BottomSongParams = BottomSongParams {
    textures: &["zz.bmp"],
    distance: 5.0,
    blend: BlendKind::Additive,
    tint_rgb: LIGHT_BLUE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// `pocket.bmp`, F1=3 → additive gold, distance 3.
pub const RICHMANKIM: BottomSongParams = BottomSongParams {
    textures: &["pocket.bmp"],
    distance: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: RICHMAN_GOLD,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// F1=2 — picks melody_a/b per spawn; additive
/// light-blue; 4 concentric echo cells.
pub const DRUMBATTLEFIELD: BottomSongParams = BottomSongParams {
    textures: &["melody_a.bmp", "melody_b.bmp"],
    distance: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: LIGHT_BLUE,
    spin: false,
    cells: 4,
    x_nudge: 0.0,
    sprites: &[],
};
/// `twirl.bmp`, F1=7 → additive white; spins.
pub const RINGNIBELUNGEN: BottomSongParams = BottomSongParams {
    textures: &["twirl.bmp"],
    distance: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: WHITE,
    spin: true,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// F1=5 — a random pick selects a gemstone **item sprite**
/// (715 yellow / 716 red / 717 blue), rendered as a `SpriteParticle`; the
/// classic client has no `gemstone.bmp`. Alpha white, distance 3.
pub const INTOABYSS: BottomSongParams = BottomSongParams {
    textures: &[],
    distance: 3.0,
    blend: BlendKind::Alpha,
    tint_rgb: WHITE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: GEMSTONE_SPRITES,
};
/// `melody_b.bmp`, F1=0 → additive light-blue, distance 3.
pub const WHISTLE: BottomSongParams = BottomSongParams {
    textures: &["melody_b.bmp"],
    distance: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: LIGHT_BLUE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// F1=8 — picks spell_01..08 per spawn;
/// additive light-blue, distance 3.
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
    distance: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: LIGHT_BLUE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// `idun_apple.bmp`, F1=6 → alpha white, distance 8.
pub const APPLEIDUN: BottomSongParams = BottomSongParams {
    textures: &["idun_apple.bmp"],
    distance: 8.0,
    blend: BlendKind::Alpha,
    tint_rgb: WHITE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
};
/// `melody_a.bmp`, F1=0 → additive light-blue, distance 3.
pub const HUMMING: BottomSongParams = BottomSongParams {
    textures: &["melody_a.bmp"],
    distance: 3.0,
    blend: BlendKind::Additive,
    tint_rgb: LIGHT_BLUE,
    spin: false,
    cells: 1,
    x_nudge: 0.0,
    sprites: &[],
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
    /// Bob phase `= random(360)` at spawn (degrees) — the bob's initial phase,
    /// hashed from world_pos so stacked songs don't bob in lockstep.
    rot_start_deg: f32,
    /// Pool-selected texture for the Billboard path (a position-hashed
    /// random pick). Unused when `sprite` is `Some`.
    texture: &'static str,
    /// Pool-selected gemstone item SPR for the sprite path (Intoabyss only).
    /// `Some` switches `collect_draws` to a `SpriteParticle`.
    sprite: Option<&'static str>,
}

impl BottomSongEffect {
    pub fn new(world_pos: [f32; 3], params: BottomSongParams) -> Self {
        let rot_start_deg = (position_hash(&world_pos) % 360) as f32;
        let idx = pseudo_random_index(&world_pos);
        let sprite = if params.sprites.is_empty() {
            None
        } else {
            Some(params.sprites[idx % params.sprites.len()])
        };
        let texture = if params.textures.is_empty() {
            "alpha_center.tga"
        } else {
            params.textures[idx % params.textures.len()]
        };
        Self {
            world_pos,
            params,
            age: 0.0,
            rot_start_deg,
            texture,
            sprite,
        }
    }
}

impl Effect for BottomSongEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        // BottomSong durations are minutes long; the holder kills us when the
        // spec's `duration_ms` expires, so we never self-terminate.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fade = (self.age / FADE_IN_SECS).clamp(0.0, 1.0);
        let frames = self.age * FRAMES_PER_SECOND;

        let bob = BOB_AMPLITUDE
            * (self.rot_start_deg + BOB_SPEED_DEG_PER_FRAME * frames)
                .to_radians()
                .sin();
        let pulse =
            1.0 + PULSE_AMPLITUDE * (PULSE_SPEED_DEG_PER_FRAME * frames).to_radians().sin();
        let rotation = if self.params.spin {
            (SPIN_SPEED_DEG_PER_FRAME * frames).to_radians()
        } else {
            0.0
        };

        let pos = [
            self.world_pos[0] + self.params.x_nudge,
            self.world_pos[1] + VERTICAL_OFFSET + bob,
            self.world_pos[2],
        ];
        let [tr, tg, tb] = self.params.tint_rgb;

        // Intoabyss renders the gemstone item SPR, not a texture billboard.
        if let Some(sprite_path) = self.sprite {
            let alpha = (ALPHA_B0 / 255.0) * fade;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path,
                position: pos,
                action_index: 0,
                motion_index: 0,
                size_scale: GEMSTONE_SIZE * pulse,
                color: [tr, tg, tb, alpha],
                blend: self.params.blend,
                aim_target: None,
                no_depth: false,
            });
            return;
        }

        // Steady-state echo: cell i sits at distance + 0.5*i, alpha - 50*i.
        // Drawn far→near so the brightest copy lands on top.
        for i in (0..self.params.cells.max(1)).rev() {
            let i_f = i as f32;
            let rx = (self.params.distance + 0.5 * i_f) * pulse;
            let side = rx * EDGE_PER_DISTANCE;
            let alpha = ((ALPHA_B0 - 50.0 * i_f).max(0.0) / 255.0) * fade;
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [side, side],
                uv: FULL_UV,
                rotation,
                texture: self.texture,
                color: [tr, tg, tb, alpha],
                blend: self.params.blend,
            });
        }
    }
}

/// Pool-index hash — same shape as the bob seed but returns the raw integer
/// so callers can take it modulo their pool size.
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
    fn bottom_song_emits_one_corner_radius_billboard() {
        // Sociable test: a freshly-spawned single-cell song yields one
        // camera-facing Billboard above the actor. The on-screen edge is
        // `distance * √2` (the corner-radius geometry), not the old
        // `distance * 2`, and the icon hovers ~6 units up (native RO -Y = up)
        // within the ±3-unit bob band.
        let mut e = BottomSongEffect::new([5.0, 0.0, 7.0], WHISTLE);
        step(&mut e, 1.0 / 60.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::Billboard { pos, size, texture, blend, .. } => {
                assert_eq!(pos[0], 5.0);
                assert_eq!(pos[2], 7.0);
                let y_min = VERTICAL_OFFSET - BOB_AMPLITUDE - 1e-2;
                let y_max = VERTICAL_OFFSET + BOB_AMPLITUDE + 1e-2;
                assert!(pos[1] >= y_min && pos[1] <= y_max, "icon Y {} in bob band", pos[1]);
                let expected = WHISTLE.distance * EDGE_PER_DISTANCE;
                assert!(
                    (size[0] - expected).abs() < WHISTLE.distance * 0.1,
                    "edge {} ≈ distance*√2 ({expected})",
                    size[0],
                );
                assert_eq!(*texture, "melody_b.bmp");
                assert_eq!(*blend, BlendKind::Additive, "F1=0 flag1[2]=2 → additive");
            }
            other => panic!("expected Billboard, got {other:?}"),
        }
    }

    #[test]
    fn richmankim_is_additive_with_gold_tint() {
        // Regression: Richmankim draws additive with RGB (200,200,100).
        // An earlier impl had the blend mode inverted (alpha) and is
        // corrected here.
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], RICHMANKIM);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, blend, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert!((color[0] - RICHMAN_GOLD[0]).abs() < 1e-4, "R {}", color[0]);
                assert!((color[1] - RICHMAN_GOLD[1]).abs() < 1e-4, "G {}", color[1]);
                assert!((color[2] - RICHMAN_GOLD[2]).abs() < 1e-4, "B {}", color[2]);
            }
            other => panic!("expected Billboard, got {other:?}"),
        }
    }

    #[test]
    fn drumbattlefield_emits_four_concentric_fading_cells() {
        // F1=2 activates 3 extra cells: a steady-state ripple of 4 copies at
        // distance + 0.5*i and alpha 200/150/100/50. Drawn far→near so the
        // brightest is on top; each successive copy is larger and dimmer.
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], DRUMBATTLEFIELD);
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4, "Drumbattlefield = 4 echo cells");
        // Drawn far→near: across the list alpha rises and size shrinks, so the
        // brightest, smallest cell lands on top.
        let (mut prev_a, mut prev_sz) = (f32::NEG_INFINITY, f32::INFINITY);
        for p in &prims {
            let EffectPrimitiveDraw::Billboard { color, size, blend, .. } = p else {
                panic!("expected Billboard, got {p:?}");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert!(color[3] >= prev_a - 1e-4, "alpha rises far→near");
            assert!(size[0] <= prev_sz + 1e-4, "size shrinks far→near");
            prev_a = color[3];
            prev_sz = size[0];
        }
    }

    #[test]
    fn poembragi_pool_picks_one_of_eight_spell_bmps() {
        // F1=8 does `random(8)` over spell_01..08; our deterministic per-spawn
        // hash must select one of those eight and cover several across spawns.
        use std::collections::HashSet;
        let mut chosen = HashSet::new();
        for i in 0..32 {
            let pos = [i as f32 * 1.7, 0.0, i as f32 * 2.3];
            chosen.insert(BottomSongEffect::new(pos, POEMBRAGI).texture);
        }
        for tex in chosen.iter() {
            assert!(POEMBRAGI.textures.contains(tex), "picked {tex} not in pool");
        }
        assert!(chosen.len() >= 4, "expected ≥4 distinct, got {}", chosen.len());
    }

    #[test]
    fn intoabyss_emits_a_gemstone_item_sprite_not_a_texture() {
        // F1=5 has no `gemstone.bmp`; `random(3)` picks an actual gemstone item
        // SPR (715/716/717). The draw must be a SpriteParticle bound to one of
        // the three 아이템 gemstone sprites, alpha-blended, hovering above the
        // actor — never a Billboard. Several positions cover >1 of the three.
        use std::collections::HashSet;
        let mut chosen = HashSet::new();
        for i in 0..24 {
            let pos = [i as f32 * 1.3, 0.0, i as f32 * 2.9];
            let mut e = BottomSongEffect::new(pos, INTOABYSS);
            step(&mut e, FADE_IN_SECS);
            match &draws(&e)[0] {
                EffectPrimitiveDraw::SpriteParticle { sprite_path, blend, position, .. } => {
                    assert!(GEMSTONE_SPRITES.contains(sprite_path), "{sprite_path} not a gem");
                    assert_eq!(*blend, BlendKind::Alpha, "F1=5 flag1[2]=4 → alpha");
                    assert!((position[1] - VERTICAL_OFFSET).abs() <= BOB_AMPLITUDE + 1e-2);
                    chosen.insert(*sprite_path);
                }
                other => panic!("expected SpriteParticle, got {other:?}"),
            }
        }
        assert!(chosen.len() >= 2, "expected ≥2 distinct gems, got {}", chosen.len());
    }

    #[test]
    fn bottom_song_bob_covers_full_vertical_range_over_a_cycle() {
        // Sampling the icon Y across one full 6-second bob cycle yields both a
        // peak above and a trough below the baseline, with a spread of ~2×
        // amplitude — proves the `3*sin(phase)` bob plumbing is live.
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], HUMMING);
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for _ in 0..360 {
            step(&mut e, 1.0 / 60.0);
            if let EffectPrimitiveDraw::Billboard { pos, .. } = &draws(&e)[0] {
                min_y = min_y.min(pos[1]);
                max_y = max_y.max(pos[1]);
            }
        }
        assert!(min_y < VERTICAL_OFFSET, "saw a trough: {min_y}");
        assert!(max_y > VERTICAL_OFFSET, "saw a peak: {max_y}");
        assert!((max_y - min_y) > BOB_AMPLITUDE, "spread {} > amplitude", max_y - min_y);
    }

    #[test]
    fn ringnibelungen_spins_over_time() {
        // F1=7 advances rotation 10°/frame → the Billboard rotation
        // grows from 0. A non-spinning song stays at rotation 0.
        let mut spin = BottomSongEffect::new([0.0, 0.0, 0.0], RINGNIBELUNGEN);
        let mut still = BottomSongEffect::new([0.0, 0.0, 0.0], WHISTLE);
        step(&mut spin, 0.5);
        step(&mut still, 0.5);
        let spin_rot = match &draws(&spin)[0] {
            EffectPrimitiveDraw::Billboard { rotation, .. } => *rotation,
            _ => unreachable!(),
        };
        let still_rot = match &draws(&still)[0] {
            EffectPrimitiveDraw::Billboard { rotation, .. } => *rotation,
            _ => unreachable!(),
        };
        assert!(spin_rot.abs() > 0.1, "RingNibelungen spins: {spin_rot}");
        assert_eq!(still_rot, 0.0, "Whistle is static");
    }

    #[test]
    fn bottom_song_alpha_fades_in_then_holds() {
        // Spawn → step into the fade window, then past it; alpha climbs from 0
        // to cell-0 base alpha (200/255).
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], LULLABY);
        step(&mut e, 0.0);
        let full = ALPHA_B0 / 255.0;
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a0.abs() < 1e-4, "starts transparent");
        step(&mut e, FADE_IN_SECS * 0.5);
        let a_mid = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_mid > a0 && a_mid < full, "rising: {a_mid}");
        step(&mut e, FADE_IN_SECS);
        let a_full = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a_full - full).abs() < 1e-4, "held at alphaB: {a_full}");
    }
}
