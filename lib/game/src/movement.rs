use crate::path::PathNode;

const DEFAULT_WALK_SPEED: u16 = 150;

/// How long the rendered position eases toward a corrected logical position.
const CORRECTION_BLEND: f32 = 0.15;

pub struct MovementState {
    current_x: f32,
    current_y: f32,
    step_start_x: f32,
    step_start_y: f32,
    path: Vec<PathNode>,
    path_index: usize,
    move_start_time: f32,
    step_duration: f32,
    moving: bool,
    speed: u16,
    // Visual-only offset from the logical position, left over from the last position
    // correction. Decays to zero over CORRECTION_BLEND so corrections ease in rather than snap.
    correction_offset: (f32, f32),
    correction_remaining: f32,
}

impl MovementState {
    pub fn new(x: u16, y: u16) -> Self {
        Self {
            current_x: x as f32,
            current_y: y as f32,
            step_start_x: x as f32,
            step_start_y: y as f32,
            path: Vec::new(),
            path_index: 0,
            move_start_time: 0.0,
            step_duration: 0.0,
            moving: false,
            speed: DEFAULT_WALK_SPEED,
            correction_offset: (0.0, 0.0),
            correction_remaining: 0.0,
        }
    }

    pub fn start_move(&mut self, path: Vec<PathNode>, start_time: f32) {
        if path.is_empty() {
            return;
        }
        self.step_start_x = self.current_x;
        self.step_start_y = self.current_y;
        self.path = path;
        self.path_index = 0;
        self.move_start_time = start_time;
        self.moving = true;
        let full_duration = self.calc_step_duration(self.path[0].is_diagonal);
        let dx = self.path[0].x as f32 - self.step_start_x;
        let dy = self.path[0].y as f32 - self.step_start_y;
        let actual_dist = (dx * dx + dy * dy).sqrt();
        let normal_dist = if self.path[0].is_diagonal {
            std::f32::consts::SQRT_2
        } else {
            1.0
        };
        self.step_duration = if actual_dist > 0.01 {
            full_duration * (actual_dist / normal_dist)
        } else {
            full_duration
        };
    }

    pub fn update(&mut self, elapsed: f32) -> (f32, f32) {
        if !self.moving || self.path.is_empty() {
            return (self.current_x, self.current_y);
        }

        loop {
            let step_elapsed = elapsed - self.move_start_time;
            if step_elapsed < self.step_duration {
                let t = step_elapsed / self.step_duration;
                let target = &self.path[self.path_index];
                let dx = target.x as f32 - self.step_start_x;
                let dy = target.y as f32 - self.step_start_y;
                self.current_x = self.step_start_x + dx * t;
                self.current_y = self.step_start_y + dy * t;
                return (self.current_x, self.current_y);
            }

            let node = &self.path[self.path_index];
            self.current_x = node.x as f32;
            self.current_y = node.y as f32;
            self.step_start_x = self.current_x;
            self.step_start_y = self.current_y;
            self.path_index += 1;

            if self.path_index >= self.path.len() {
                self.moving = false;
                return (self.current_x, self.current_y);
            }

            self.move_start_time += self.step_duration;
            self.step_duration = self.calc_step_duration(self.path[self.path_index].is_diagonal);
        }
    }

    fn calc_step_duration(&self, is_diagonal: bool) -> f32 {
        if is_diagonal {
            (self.speed as f32 / 0.7) / 1000.0
        } else {
            self.speed as f32 / 1000.0
        }
    }

    pub fn is_moving(&self) -> bool {
        self.moving
    }

    pub fn cell_position(&self) -> (u16, u16) {
        (self.current_x.round() as u16, self.current_y.round() as u16)
    }

    pub fn stop(&mut self) {
        self.moving = false;
        self.path.clear();
        self.path_index = 0;
    }

    /// Hard teleport: jump instantly, no visual blending (warp, spawn, respawn).
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.moving = false;
        self.path.clear();
        self.path_index = 0;
        self.current_x = x;
        self.current_y = y;
        self.step_start_x = x;
        self.step_start_y = y;
        self.correction_offset = (0.0, 0.0);
        self.correction_remaining = 0.0;
    }

    /// Snap the logical position to a corrected cell while keeping the rendered sprite where it
    /// was; the leftover discrepancy decays over CORRECTION_BLEND so the sprite eases in instead
    /// of teleporting. Used when a server move re-anchors an entity onto the grid.
    pub fn correct_to_cell(&mut self, x: f32, y: f32) {
        let (rendered_x, rendered_y) = self.position();
        self.moving = false;
        self.path.clear();
        self.path_index = 0;
        self.current_x = x;
        self.current_y = y;
        self.step_start_x = x;
        self.step_start_y = y;
        let (dx, dy) = (rendered_x - x, rendered_y - y);
        if dx.abs() > 0.001 || dy.abs() > 0.001 {
            self.correction_offset = (dx, dy);
            self.correction_remaining = CORRECTION_BLEND;
        }
    }

    /// Decay the visual correction offset. Call every frame with the frame delta.
    pub fn decay_correction(&mut self, delta: f32) {
        if self.correction_remaining > 0.0 {
            self.correction_remaining = (self.correction_remaining - delta).max(0.0);
        }
    }

    pub fn position(&self) -> (f32, f32) {
        let frac = (self.correction_remaining / CORRECTION_BLEND).clamp(0.0, 1.0);
        (
            self.current_x + self.correction_offset.0 * frac,
            self.current_y + self.correction_offset.1 * frac,
        )
    }

    pub fn set_speed(&mut self, speed: u16) {
        self.speed = speed;
    }

    pub fn destination(&self) -> Option<(u16, u16)> {
        self.path.last().map(|node| (node.x, node.y))
    }

    pub fn movement_direction(&self) -> Option<u8> {
        if !self.moving || self.path_index >= self.path.len() {
            return None;
        }
        let target = &self.path[self.path_index];
        let dx = target.x as f32 - self.current_x;
        let dy = target.y as f32 - self.current_y;
        direction_from_delta(dx, dy)
    }
}

pub fn direction_from_positions(src_x: u16, src_y: u16, dst_x: u16, dst_y: u16) -> Option<u8> {
    let dx = dst_x as f32 - src_x as f32;
    let dy = dst_y as f32 - src_y as f32;
    direction_from_delta(dx, dy)
}

fn direction_from_delta(dx: f32, dy: f32) -> Option<u8> {
    if dx.abs() < 0.01 && dy.abs() < 0.01 {
        return None;
    }
    let dir = match (dx.partial_cmp(&0.0), dy.partial_cmp(&0.0)) {
        (Some(std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Greater)) => 0, // S
        (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Greater)) => 1,  // SW
        (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Equal)) => 2,    // W
        (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Less)) => 3,     // NW
        (Some(std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Less)) => 4,    // N
        (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Less)) => 5,  // NE
        (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Equal)) => 6, // E
        (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Greater)) => 7, // SE
        _ => return None,
    };
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_path_node(x: u16, y: u16, is_diagonal: bool) -> PathNode {
        PathNode {
            id: 0,
            parent_id: 0,
            x,
            y,
            g_cost: 0,
            f_cost: 0,
            is_open: false,
            is_diagonal,
        }
    }

    #[test]
    fn movement_interpolates_along_straight_path() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![
            make_path_node(1, 0, false),
            make_path_node(2, 0, false),
            make_path_node(3, 0, false),
        ];
        movement.start_move(path, 0.0);
        assert!(movement.is_moving());

        // Halfway through first step (150ms = 0.15s per step, so 0.075s = halfway)
        let (x, y) = movement.update(0.075);
        assert!((x - 0.5).abs() < 0.01, "x={x}");
        assert!(y.abs() < 0.01, "y={y}");

        // Complete first step
        let (x, _) = movement.update(0.15);
        assert!((x - 1.0).abs() < 0.01, "x={x}");

        // Complete all steps (slightly past 3*0.15 to avoid float precision edge)
        let (x, _) = movement.update(0.46);
        assert!((x - 3.0).abs() < 0.01, "x={x}");
        assert!(!movement.is_moving());
    }

    #[test]
    fn movement_diagonal_step_takes_longer() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![make_path_node(1, 1, true)];
        movement.start_move(path, 0.0);

        // Diagonal duration = 150 / 0.7 / 1000 ≈ 0.2143s
        let (x, y) = movement.update(0.1);
        assert!(movement.is_moving());
        // Should be about 46.7% through
        assert!(x > 0.4 && x < 0.5, "x={x}");
        assert!(y > 0.4 && y < 0.5, "y={y}");

        let (x, y) = movement.update(0.25);
        assert!(!movement.is_moving());
        assert!((x - 1.0).abs() < 0.01);
        assert!((y - 1.0).abs() < 0.01);
    }

    #[test]
    fn movement_not_moving_returns_current_position() {
        let movement = MovementState::new(5, 10);
        assert!(!movement.is_moving());
        assert_eq!(movement.cell_position(), (5, 10));
    }

    #[test]
    fn stop_cancels_movement_mid_path() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![
            make_path_node(1, 0, false),
            make_path_node(2, 0, false),
            make_path_node(3, 0, false),
        ];
        movement.start_move(path, 0.0);
        assert!(movement.is_moving());

        movement.update(0.075);
        movement.stop();
        assert!(!movement.is_moving());

        let (x, y) = movement.update(1.0);
        assert!(
            (x - 0.5).abs() < 0.01,
            "should stay at stopped position, got x={x}"
        );
        assert!(y.abs() < 0.01);
        assert!(!movement.is_moving());
    }

    #[test]
    fn set_position_clears_movement_state() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![make_path_node(1, 0, false), make_path_node(2, 0, false)];
        movement.start_move(path, 0.0);
        assert!(movement.is_moving());

        movement.set_position(50.0, 50.0);
        assert!(!movement.is_moving());

        let (x, y) = movement.update(1.0);
        assert!(
            (x - 50.0).abs() < 0.01,
            "should stay at new position, got x={x}"
        );
        assert!(
            (y - 50.0).abs() < 0.01,
            "should stay at new position, got y={y}"
        );
    }

    #[test]
    fn position_returns_interpolated_value_during_movement() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![make_path_node(1, 0, false)];
        movement.start_move(path, 0.0);

        movement.update(0.075);
        let (x, _) = movement.position();
        assert!(
            (x - 0.5).abs() < 0.01,
            "position() should return smooth value, got x={x}"
        );
        assert_eq!(movement.cell_position(), (1, 0));
    }

    #[test]
    fn correct_to_cell_blends_visual_offset_without_moving_logical() {
        let mut movement = MovementState::new(0, 0);
        // Advance halfway into a step so the rendered position is mid-cell (~0.5).
        movement.start_move(vec![make_path_node(1, 0, false)], 0.0);
        movement.update(0.075);
        assert!((movement.position().0 - 0.5).abs() < 0.01);

        // Correct the logical position back to cell 0: logical snaps, rendered eases from 0.5.
        movement.correct_to_cell(0.0, 0.0);
        assert_eq!(movement.cell_position(), (0, 0)); // logical is exactly the cell
        assert!(
            (movement.position().0 - 0.5).abs() < 0.01,
            "rendered should still be at the pre-correction position"
        );

        // Decay over the blend window: rendered eases toward the logical cell and lands on it.
        movement.decay_correction(CORRECTION_BLEND / 2.0);
        let mid = movement.position().0;
        assert!(mid > 0.0 && mid < 0.5, "should be easing in, got {mid}");
        movement.decay_correction(CORRECTION_BLEND);
        assert!(
            movement.position().0.abs() < 0.001,
            "offset should be fully decayed, got {}",
            movement.position().0
        );
    }

    #[test]
    fn direction_from_positions_all_eight_directions() {
        // S: dst_y > src_y, same x
        assert_eq!(direction_from_positions(5, 5, 5, 8), Some(0));
        // SW
        assert_eq!(direction_from_positions(5, 5, 3, 8), Some(1));
        // W: dst_x < src_x, same y
        assert_eq!(direction_from_positions(5, 5, 2, 5), Some(2));
        // NW
        assert_eq!(direction_from_positions(5, 5, 3, 3), Some(3));
        // N: dst_y < src_y, same x
        assert_eq!(direction_from_positions(5, 5, 5, 2), Some(4));
        // NE
        assert_eq!(direction_from_positions(5, 5, 8, 3), Some(5));
        // E: dst_x > src_x, same y
        assert_eq!(direction_from_positions(5, 5, 8, 5), Some(6));
        // SE
        assert_eq!(direction_from_positions(5, 5, 8, 8), Some(7));
    }

    #[test]
    fn direction_from_positions_same_position_returns_none() {
        assert_eq!(direction_from_positions(5, 5, 5, 5), None);
    }

    #[test]
    fn repath_mid_movement_with_set_position_aligns_step_start() {
        let mut movement = MovementState::new(100, 100);
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        movement.start_move(path, 0.0);

        // Advance to midway through first step (position ~100.5)
        movement.update(0.075);
        let (x, _) = movement.position();
        assert!((x - 100.5).abs() < 0.01);

        // Simulate re-path during chase: cell_position rounds to 101
        let (cx, cy) = movement.cell_position();
        assert_eq!((cx, cy), (101, 100));

        // Align position to cell before starting new path
        movement.set_position(cx as f32, cy as f32);
        let new_path = vec![make_path_node(102, 100, false)];
        movement.start_move(new_path, 0.075);

        // Halfway through new step: should be at 101.5
        let (x, _) = movement.update(0.075 + 0.075);
        assert!((x - 101.5).abs() < 0.01, "expected 101.5, got {x}");
    }

    #[test]
    fn repath_mid_movement_uses_proportional_first_step() {
        let mut movement = MovementState::new(100, 100);
        let path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
        ];
        movement.start_move(path, 0.0);

        // Advance to 70% through first step → position ~100.7
        movement.update(0.105);
        let (x, _) = movement.position();
        assert!((x - 100.7).abs() < 0.01, "setup: expected 100.7, got {x}");

        // Re-path from current position (100.7) toward 103.
        // Path nodes skip from 101 (next cell) onward.
        let new_path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
            make_path_node(103, 100, false),
        ];
        movement.start_move(new_path, 0.105);

        // First step: 100.7 → 101.0 = 0.3 cells.
        // Proportional duration = 150ms * 0.3 = 45ms.
        // At 22.5ms into the step (~halfway): position should be ~100.85
        let (x, _) = movement.update(0.105 + 0.0225);
        assert!((x - 100.85).abs() < 0.02, "expected ~100.85, got {x}");

        // At 45ms the first step completes, entity is at 101.0
        // At 45 + 75ms = 120ms, entity is halfway through second step: ~101.5
        let (x, _) = movement.update(0.105 + 0.045 + 0.075);
        assert!((x - 101.5).abs() < 0.02, "expected ~101.5, got {x}");
    }
}
