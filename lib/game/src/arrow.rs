pub use crate::effect::{ARROW_SPRITE, SPRITES};

pub fn flight_secs_for_cell_distance(dist_cells: f32) -> f32 {
    const BASE: f32 = 0.192; // 8 frames × 24 ms
    BASE * (dist_cells / 8.0).clamp(0.05, 1.0)
}

/// The nine cells an Arrow Shower rains on: the aimed cell first, then its
/// eight neighbours in the order the original game fans them out.
pub fn arrow_shower_cells(center: (u16, u16)) -> [(i32, i32); 9] {
    const OFFSETS: [(i32, i32); 9] = [
        (0, 0),
        (1, 0),
        (1, 1),
        (1, -1),
        (-1, 0),
        (-1, 1),
        (-1, -1),
        (0, 1),
        (0, -1),
    ];
    OFFSETS.map(|(dx, dy)| (center.0 as i32 + dx, center.1 as i32 + dy))
}

pub struct ArrowProjectile {
    shooter_pos: [f32; 3],
    target_pos: [f32; 3],
    age: f32,
    delay_secs: f32,
    flight_secs: f32,
}

impl ArrowProjectile {
    pub fn new(
        shooter_pos: [f32; 3],
        target_pos: [f32; 3],
        delay_secs: f32,
        flight_secs: f32,
    ) -> Self {
        Self {
            shooter_pos,
            target_pos,
            age: 0.0,
            delay_secs: delay_secs.max(0.0),
            flight_secs: flight_secs.max(0.05),
        }
    }

    pub fn advance(&mut self, delta: f32) {
        self.age += delta;
    }

    pub fn is_visible(&self) -> bool {
        self.age >= self.delay_secs
    }

    pub fn is_done(&self) -> bool {
        self.age >= self.delay_secs + self.flight_secs
    }

    pub fn sprite_path(&self) -> &'static str {
        ARROW_SPRITE
    }

    pub fn target_pos(&self) -> [f32; 3] {
        self.target_pos
    }

    pub fn current_position(&self) -> [f32; 3] {
        let t = ((self.age - self.delay_secs) / self.flight_secs).clamp(0.0, 1.0);
        [
            self.shooter_pos[0] + (self.target_pos[0] - self.shooter_pos[0]) * t,
            self.shooter_pos[1] + (self.target_pos[1] - self.shooter_pos[1]) * t,
            self.shooter_pos[2] + (self.target_pos[2] - self.shooter_pos[2]) * t,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_hidden_then_zips_from_shooter_to_target() {
        assert!((flight_secs_for_cell_distance(16.0) - 0.192).abs() < 1e-6);
        assert!(flight_secs_for_cell_distance(4.0) < 0.192);

        let flight = flight_secs_for_cell_distance(16.0);
        let mut arrow = ArrowProjectile::new([0.0, 0.0, 0.0], [10.0, 0.0, 20.0], 0.5, flight);

        // Hidden during the delay, parked at the shooter.
        assert!(!arrow.is_visible());
        assert_eq!(arrow.current_position(), [0.0, 0.0, 0.0]);
        assert!(!arrow.is_done());

        arrow.advance(0.5 + flight * 0.5);
        assert!(arrow.is_visible());
        let mid = arrow.current_position();
        assert!((mid[0] - 5.0).abs() < 0.001);
        assert!((mid[2] - 10.0).abs() < 0.001);
        assert!(!arrow.is_done());

        arrow.advance(flight * 0.5 + 0.01);
        let end = arrow.current_position();
        assert!((end[0] - 10.0).abs() < 0.001);
        assert!((end[2] - 20.0).abs() < 0.001);
        assert!(arrow.is_done());
    }

    #[test]
    fn arrow_shower_rains_the_aimed_cell_then_its_eight_neighbours() {
        assert_eq!(
            arrow_shower_cells((100, 50)),
            [
                (100, 50),
                (101, 50),
                (101, 51),
                (101, 49),
                (99, 50),
                (99, 51),
                (99, 49),
                (100, 51),
                (100, 49),
            ]
        );

        // A cell fan on the map edge keeps all nine legs; the off-map ones just
        // read as negative.
        let edge = arrow_shower_cells((0, 0));
        assert_eq!(edge[4], (-1, 0));
        assert_eq!(edge[8], (0, -1));
    }
}
