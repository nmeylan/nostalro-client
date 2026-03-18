use crate::path::PathNode;

const DEFAULT_WALK_SPEED: u16 = 150;

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
        self.step_duration = self.calc_step_duration(self.path[0].is_diagonal);
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

    pub fn set_position(&mut self, x: f32, y: f32) {
        self.current_x = x;
        self.current_y = y;
        self.step_start_x = x;
        self.step_start_y = y;
    }

    pub fn position(&self) -> (f32, f32) {
        (self.current_x, self.current_y)
    }

    pub fn destination(&self) -> Option<(u16, u16)> {
        self.path.last().map(|node| (node.x, node.y))
    }

    /// RO direction from current position toward next path node.
    /// Returns 0-7 (S, SW, W, NW, N, NE, E, SE) or None if not moving.
    pub fn movement_direction(&self) -> Option<u8> {
        if !self.moving || self.path_index >= self.path.len() {
            return None;
        }
        let target = &self.path[self.path_index];
        let dx = target.x as f32 - self.current_x;
        let dy = target.y as f32 - self.current_y;
        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return None;
        }
        // RO directions: 0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE
        let dir = match (dx.partial_cmp(&0.0), dy.partial_cmp(&0.0)) {
            (Some(std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Greater)) => 0,   // S
            (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Greater)) => 1,    // SW
            (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Equal)) => 2,      // W
            (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Less)) => 3,       // NW
            (Some(std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Less)) => 4,      // N
            (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Less)) => 5,    // NE
            (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Equal)) => 6,   // E
            (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Greater)) => 7, // SE
            _ => return None,
        };
        Some(dir)
    }
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
    fn position_returns_interpolated_value_during_movement() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![make_path_node(1, 0, false)];
        movement.start_move(path, 0.0);

        movement.update(0.075);
        let (x, _) = movement.position();
        assert!((x - 0.5).abs() < 0.01, "position() should return smooth value, got x={x}");
        assert_eq!(movement.cell_position(), (1, 0));
    }
}
