use models::enums::skill_enums::SkillEnum;

/// Texture tiled across the scope square.
pub const SCOPE_TEXTURE: &str = "magic_target.tga";

const FRAMES_PER_SECOND: f32 = 60.0;
const BASE_ALPHA: f32 = 128.0 / 255.0;
const ALPHA_SWING: f32 = 40.0 / 255.0;
const ROTATION_PER_FRAME: f32 = 0.5;

pub const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
pub const HOSTILE_RED: [f32; 3] = [1.0, 0.0, 0.0];

/// Side of the scope square, in cells, for a ground-placed `skill_id`.
pub fn scope_size(skill_id: u16) -> u16 {
    let extent = match SkillEnum::from_id(skill_id as u32) {
        SkillEnum::MgNapalmbeat
        | SkillEnum::WzFrostnova
        | SkillEnum::BsHammerfall
        | SkillEnum::HtBlitzbeat
        | SkillEnum::HtDetecting
        | SkillEnum::PrBenedictio
        | SkillEnum::AsGrimtooth
        | SkillEnum::AsVenomdust
        | SkillEnum::SnFalconassault
        | SkillEnum::HwGanbantein => 3,
        SkillEnum::MgFireball
        | SkillEnum::MgThunderstorm
        | SkillEnum::WzHeavendrive
        | SkillEnum::WzQuagmire
        | SkillEnum::PrSanctuary
        | SkillEnum::HwGravitation => 5,
        SkillEnum::PrMagnus | SkillEnum::CrSlimpitcher => 7,
        SkillEnum::WzMeteor | SkillEnum::WzStormgust => 9,
        SkillEnum::WzVermilion => 11,
        _ => 0,
    };
    extent + 2
}

/// The square of ground cells marked under a placed cast for as long as it is
/// being cast. The scope texture spans the whole square and spins about its
/// centre, so every cell samples a rotating slice of it.
pub struct CastScope {
    pub cell_x: u16,
    pub cell_y: u16,
    pub size: u16,
    pub color_rgb: [f32; 3],
    rotation: f32,
    rot_speed: f32,
    remaining: f32,
}

impl CastScope {
    pub fn new(skill_id: u16, cell_x: u16, cell_y: u16, hostile: bool, duration_secs: f32) -> Self {
        let size = scope_size(skill_id);
        Self {
            cell_x,
            cell_y,
            size,
            color_rgb: if hostile { HOSTILE_RED } else { WHITE },
            rotation: 0.0,
            rot_speed: ROTATION_PER_FRAME / size as f32 * FRAMES_PER_SECOND,
            remaining: duration_secs,
        }
    }

    /// Advances the spin; returns false once the cast it marks is over.
    pub fn tick(&mut self, delta: f32) -> bool {
        self.rotation += self.rot_speed * delta;
        self.remaining -= delta;
        self.remaining > 0.0
    }

    pub fn color(&self) -> [f32; 4] {
        let [r, g, b] = self.color_rgb;
        let alpha = BASE_ALPHA + self.rotation.sin() * ALPHA_SWING;
        [r, g, b, alpha]
    }

    /// Lowest cell of the square, which the size-odd ones centre on `cell_x`/`cell_y`.
    pub fn origin_cell(&self) -> (i32, i32) {
        let half = (self.size / 2) as i32;
        (self.cell_x as i32 - half, self.cell_y as i32 - half)
    }

    /// Texture corners for the cell at column `col`, row `row` of the square, in
    /// the corner order [`MapCoordinates::cell_corners_world`] returns.
    pub fn cell_uv(&self, col: u16, row: u16) -> [[f32; 2]; 4] {
        let size = self.size as f32;
        let half = size / 2.0;
        let (sin_r, cos_r) = self.rotation.sin_cos();
        let map = |u: f32, v: f32| {
            [
                (u * cos_r + v * sin_r + half) / size,
                (-u * sin_r + v * cos_r + half) / size,
            ]
        };
        let (u0, v0) = (col as f32 - half, row as f32 - half);
        let (u1, v1) = (u0 + 1.0, v0 + 1.0);
        [map(u0, v0), map(u1, v0), map(u0, v1), map(u1, v1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_covers_the_skill_footprint_plus_a_cell_of_margin() {
        assert_eq!(scope_size(SkillEnum::WzStormgust.id() as u16), 11);
        assert_eq!(scope_size(SkillEnum::WzVermilion.id() as u16), 13);
        assert_eq!(scope_size(SkillEnum::AcShower.id() as u16), 2);
        assert_eq!(scope_size(SkillEnum::AlWarp.id() as u16), 2);
    }

    #[test]
    fn a_scope_sits_on_its_cell_and_spins_and_pulses_until_the_cast_ends() {
        let mut scope = CastScope::new(SkillEnum::WzStormgust.id() as u16, 100, 80, false, 1.0);
        assert_eq!(scope.origin_cell(), (95, 75));
        let first = scope.cell_uv(0, 0);

        assert!(scope.tick(0.5));
        assert_ne!(scope.cell_uv(0, 0), first);
        let alpha = scope.color()[3];
        assert!((BASE_ALPHA - ALPHA_SWING..=BASE_ALPHA + ALPHA_SWING).contains(&alpha));

        assert!(!scope.tick(0.5));
    }

    #[test]
    fn opposing_casters_get_a_red_scope() {
        let mine = CastScope::new(SkillEnum::WzMeteor.id() as u16, 10, 10, false, 1.0);
        let theirs = CastScope::new(SkillEnum::WzMeteor.id() as u16, 10, 10, true, 1.0);

        assert_eq!(mine.color_rgb, WHITE);
        assert_eq!(theirs.color_rgb, HOSTILE_RED);
    }
}
