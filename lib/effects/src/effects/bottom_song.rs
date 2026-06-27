use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

#[derive(Clone, Copy, Debug)]
pub struct BottomSongParams {
    pub textures: &'static [&'static str],
    /// Non-empty only for Intoabyss (F1=5): no `gemstone.bmp` exists; a random pick selects the actual item SPR.
    pub sprites: &'static [&'static str],
    pub distance: f32,
    pub blend: BlendKind,
    pub tint_rgb: [f32; 3],
    pub spin: bool,
    pub cells: u8,
    pub x_nudge: f32,
}

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_IN_FRAMES: f32 = 30.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// Cell-0 base alpha (out of 255); trail cells fall 50 each.
const ALPHA_B0: f32 = 200.0;
const VERTICAL_OFFSET: f32 = -6.0;
const BOB_AMPLITUDE: f32 = 3.0;
const BOB_SPEED_DEG_PER_FRAME: f32 = 1.0;
const PULSE_SPEED_DEG_PER_FRAME: f32 = 5.0;
const PULSE_AMPLITUDE: f32 = 0.05;
const SPIN_SPEED_DEG_PER_FRAME: f32 = 10.0;
const EDGE_PER_DISTANCE: f32 = std::f32::consts::SQRT_2;
const FULL_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const LIGHT_BLUE: [f32; 3] = [130.0 / 255.0, 130.0 / 255.0, 250.0 / 255.0];
const RICHMAN_GOLD: [f32; 3] = [200.0 / 255.0, 200.0 / 255.0, 100.0 / 255.0];

/// Intoabyss item sprites: no `gemstone.bmp` exists; a random pick over 715/716/717.
pub const GEMSTONE_SPRITES: &[&str] = &[
    "data/sprite/아이템/옐로우젬스톤",
    "data/sprite/아이템/레드젬스톤",
    "data/sprite/아이템/블루젬스톤",
];
pub const SPRITES: &[&str] = GEMSTONE_SPRITES;
const GEMSTONE_SIZE: f32 = 1.0;

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
    rot_start_deg: f32,
    texture: &'static str,
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
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fade = (self.age / FADE_IN_SECS).clamp(0.0, 1.0);
        let frames = self.age * FRAMES_PER_SECOND;

        let bob = BOB_AMPLITUDE
            * (self.rot_start_deg + BOB_SPEED_DEG_PER_FRAME * frames)
                .to_radians()
                .sin();
        let pulse = 1.0 + PULSE_AMPLITUDE * (PULSE_SPEED_DEG_PER_FRAME * frames).to_radians().sin();
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
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomSongEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn bottom_song_emits_one_corner_radius_billboard() {
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
                let y_min = VERTICAL_OFFSET - BOB_AMPLITUDE - 1e-2;
                let y_max = VERTICAL_OFFSET + BOB_AMPLITUDE + 1e-2;
                assert!(
                    pos[1] >= y_min && pos[1] <= y_max,
                    "icon Y {} in bob band",
                    pos[1]
                );
                let expected = WHISTLE.distance * EDGE_PER_DISTANCE;
                assert!(
                    (size[0] - expected).abs() < WHISTLE.distance * 0.1,
                    "edge {} ≈ distance*√2 ({expected})",
                    size[0],
                );
                assert_eq!(*texture, "melody_b.bmp");
                assert_eq!(*blend, BlendKind::Additive);
            }
            other => panic!("expected Billboard, got {other:?}"),
        }
    }

    #[test]
    fn richmankim_is_additive_with_gold_tint() {
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
        let mut e = BottomSongEffect::new([0.0, 0.0, 0.0], DRUMBATTLEFIELD);
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4, "Drumbattlefield = 4 echo cells");
        let (mut prev_a, mut prev_sz) = (f32::NEG_INFINITY, f32::INFINITY);
        for p in &prims {
            let EffectPrimitiveDraw::Billboard {
                color, size, blend, ..
            } = p
            else {
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
        use std::collections::HashSet;
        let mut chosen = HashSet::new();
        for i in 0..32 {
            let pos = [i as f32 * 1.7, 0.0, i as f32 * 2.3];
            chosen.insert(BottomSongEffect::new(pos, POEMBRAGI).texture);
        }
        for tex in chosen.iter() {
            assert!(POEMBRAGI.textures.contains(tex), "picked {tex} not in pool");
        }
        assert!(
            chosen.len() >= 4,
            "expected ≥4 distinct, got {}",
            chosen.len()
        );
    }

    #[test]
    fn intoabyss_emits_a_gemstone_item_sprite_not_a_texture() {
        use std::collections::HashSet;
        let mut chosen = HashSet::new();
        for i in 0..24 {
            let pos = [i as f32 * 1.3, 0.0, i as f32 * 2.9];
            let mut e = BottomSongEffect::new(pos, INTOABYSS);
            step(&mut e, FADE_IN_SECS);
            match &draws(&e)[0] {
                EffectPrimitiveDraw::SpriteParticle {
                    sprite_path,
                    blend,
                    position,
                    ..
                } => {
                    assert!(
                        GEMSTONE_SPRITES.contains(sprite_path),
                        "{sprite_path} not a gem"
                    );
                    assert_eq!(*blend, BlendKind::Alpha, "F1=5 flag1[2]=4 → alpha");
                    assert!((position[1] - VERTICAL_OFFSET).abs() <= BOB_AMPLITUDE + 1e-2);
                    chosen.insert(*sprite_path);
                }
                other => panic!("expected SpriteParticle, got {other:?}"),
            }
        }
        assert!(
            chosen.len() >= 2,
            "expected ≥2 distinct gems, got {}",
            chosen.len()
        );
    }

    #[test]
    fn bottom_song_bob_covers_full_vertical_range_over_a_cycle() {
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
        assert!(
            (max_y - min_y) > BOB_AMPLITUDE,
            "spread {} > amplitude",
            max_y - min_y
        );
    }

    #[test]
    fn ringnibelungen_spins_over_time() {
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
