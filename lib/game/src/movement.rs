use crate::path::PathNode;

const DEFAULT_WALK_SPEED: u16 = 150;

const CORRECTION_BLEND: f32 = 0.15;

pub struct MovementState {
    current_x: f32,
    current_y: f32,
    source_x: f32,
    source_y: f32,
    source_time: f32,
    path: Vec<PathNode>,
    node_times: Vec<f32>,
    seg_index: usize,
    moving: bool,
    speed: u16,
    correction_offset: (f32, f32),
    correction_remaining: f32,
}

impl MovementState {
    pub fn new(x: u16, y: u16) -> Self {
        Self {
            current_x: x as f32,
            current_y: y as f32,
            source_x: x as f32,
            source_y: y as f32,
            source_time: 0.0,
            path: Vec::new(),
            node_times: Vec::new(),
            seg_index: 0,
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
        self.source_x = self.current_x;
        self.source_y = self.current_y;
        self.source_time = start_time;
        self.path = path;
        self.seg_index = 0;
        self.moving = true;
        self.build_times();
    }

    fn build_times(&mut self) {
        self.node_times.clear();
        let mut t = self.source_time;
        let mut px = self.source_x;
        let mut py = self.source_y;
        for node in &self.path {
            t += step_duration(self.speed, px, py, node);
            self.node_times.push(t);
            px = node.x as f32;
            py = node.y as f32;
        }
    }

    fn replay(&self, t: f32) -> (f32, f32, bool, usize) {
        if self.path.is_empty() {
            return (self.current_x, self.current_y, true, 0);
        }
        if t <= self.source_time {
            return (self.source_x, self.source_y, false, 0);
        }
        let mut px = self.source_x;
        let mut py = self.source_y;
        let mut pt = self.source_time;
        for (i, node) in self.path.iter().enumerate() {
            let nt = self.node_times[i];
            if t < nt {
                let seg = nt - pt;
                let frac = if seg > 1e-6 { (t - pt) / seg } else { 1.0 };
                let x = px + (node.x as f32 - px) * frac;
                let y = py + (node.y as f32 - py) * frac;
                return (x, y, false, i);
            }
            px = node.x as f32;
            py = node.y as f32;
            pt = nt;
        }
        let last = self.path.last().unwrap();
        (last.x as f32, last.y as f32, true, self.path.len() - 1)
    }

    pub fn update(&mut self, elapsed: f32) -> (f32, f32) {
        if !self.moving || self.path.is_empty() {
            return (self.current_x, self.current_y);
        }
        let (x, y, done, seg) = self.replay(elapsed);
        self.current_x = x;
        self.current_y = y;
        self.seg_index = seg;
        if done {
            self.moving = false;
        }
        (self.current_x, self.current_y)
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
        self.node_times.clear();
        self.seg_index = 0;
    }

    pub fn set_position(&mut self, x: f32, y: f32) {
        self.moving = false;
        self.path.clear();
        self.node_times.clear();
        self.seg_index = 0;
        self.current_x = x;
        self.current_y = y;
        self.source_x = x;
        self.source_y = y;
        self.correction_offset = (0.0, 0.0);
        self.correction_remaining = 0.0;
    }

    pub fn correct_to_cell(&mut self, x: f32, y: f32) {
        let (rendered_x, rendered_y) = self.position();
        self.moving = false;
        self.path.clear();
        self.node_times.clear();
        self.seg_index = 0;
        self.current_x = x;
        self.current_y = y;
        self.source_x = x;
        self.source_y = y;
        let (dx, dy) = (rendered_x - x, rendered_y - y);
        if dx.abs() > 0.001 || dy.abs() > 0.001 {
            self.correction_offset = (dx, dy);
            self.correction_remaining = CORRECTION_BLEND;
        }
    }

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
        if !self.moving || self.seg_index >= self.path.len() {
            return None;
        }
        let target = &self.path[self.seg_index];
        let dx = target.x as f32 - self.current_x;
        let dy = target.y as f32 - self.current_y;
        direction_from_delta(dx, dy)
    }
}

fn step_duration(speed: u16, prev_x: f32, prev_y: f32, node: &PathNode) -> f32 {
    let full = if node.is_diagonal {
        // diagonal traversal costs 1.4x a straight step on the server (14/10),
        // not √2 — must match exactly or the body drifts and snaps on diagonals
        speed as f32 * 1.4 / 1000.0
    } else {
        speed as f32 / 1000.0
    };
    let dx = node.x as f32 - prev_x;
    let dy = node.y as f32 - prev_y;
    let actual = (dx * dx + dy * dy).sqrt();
    let nominal = if node.is_diagonal {
        std::f32::consts::SQRT_2
    } else {
        1.0
    };
    if actual > 0.01 {
        full * (actual / nominal)
    } else {
        full
    }
}

pub fn direction_from_positions(src_x: u16, src_y: u16, dst_x: u16, dst_y: u16) -> Option<u8> {
    let dx = dst_x as f32 - src_x as f32;
    let dy = dst_y as f32 - src_y as f32;
    direction_from_delta(dx, dy)
}

pub fn direction_from_delta(dx: f32, dy: f32) -> Option<u8> {
    if dx.abs() < 0.01 && dy.abs() < 0.01 {
        return None;
    }
    let dir = match (dx.partial_cmp(&0.0), dy.partial_cmp(&0.0)) {
        (Some(std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Greater)) => 0,
        (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Greater)) => 1,
        (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Equal)) => 2,
        (Some(std::cmp::Ordering::Less), Some(std::cmp::Ordering::Less)) => 3,
        (Some(std::cmp::Ordering::Equal), Some(std::cmp::Ordering::Less)) => 4,
        (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Less)) => 5,
        (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Equal)) => 6,
        (Some(std::cmp::Ordering::Greater), Some(std::cmp::Ordering::Greater)) => 7,
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

        let (x, y) = movement.update(0.075);
        assert!((x - 0.5).abs() < 0.01, "x={x}");
        assert!(y.abs() < 0.01, "y={y}");

        let (x, _) = movement.update(0.15);
        assert!((x - 1.0).abs() < 0.01, "x={x}");

        let (x, _) = movement.update(0.46);
        assert!((x - 3.0).abs() < 0.01, "x={x}");
        assert!(!movement.is_moving());
    }

    #[test]
    fn movement_diagonal_step_takes_longer() {
        let mut movement = MovementState::new(0, 0);
        let path = vec![make_path_node(1, 1, true)];
        movement.start_move(path, 0.0);

        let (x, y) = movement.update(0.1);
        assert!(movement.is_moving());
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
        movement.start_move(vec![make_path_node(1, 0, false)], 0.0);
        movement.update(0.075);
        assert!((movement.position().0 - 0.5).abs() < 0.01);

        movement.correct_to_cell(0.0, 0.0);
        assert_eq!(movement.cell_position(), (0, 0));
        assert!(
            (movement.position().0 - 0.5).abs() < 0.01,
            "rendered should still be at the pre-correction position"
        );

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
    fn server_confirm_reconciles_drift_then_walks_from_authoritative_cell() {
        let mut movement = MovementState::new(100, 100);
        movement.start_move(vec![make_path_node(105, 100, false)], 0.0);
        movement.update(0.3);
        let drifted = movement.position().0;
        assert!(drifted > 100.0 && drifted < 105.0);

        // server confirms a move that actually starts at cell 102
        movement.correct_to_cell(102.0, 100.0);
        movement.start_move(vec![make_path_node(103, 100, false)], 0.0);

        assert_eq!(movement.cell_position(), (102, 100));
        assert!(
            (movement.position().0 - drifted).abs() < 0.01,
            "rendered eases from the drifted spot, got {}",
            movement.position().0
        );

        movement.decay_correction(CORRECTION_BLEND);
        movement.update(0.075);
        assert!(
            (movement.position().0 - 102.5).abs() < 0.05,
            "should walk from the authoritative cell, got {}",
            movement.position().0
        );
    }

    #[test]
    fn direction_from_positions_all_eight_directions() {
        assert_eq!(direction_from_positions(5, 5, 5, 8), Some(0));
        assert_eq!(direction_from_positions(5, 5, 3, 8), Some(1));
        assert_eq!(direction_from_positions(5, 5, 2, 5), Some(2));
        assert_eq!(direction_from_positions(5, 5, 3, 3), Some(3));
        assert_eq!(direction_from_positions(5, 5, 5, 2), Some(4));
        assert_eq!(direction_from_positions(5, 5, 8, 3), Some(5));
        assert_eq!(direction_from_positions(5, 5, 8, 5), Some(6));
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

        movement.update(0.075);
        let (x, _) = movement.position();
        assert!((x - 100.5).abs() < 0.01);

        let (cx, cy) = movement.cell_position();
        assert_eq!((cx, cy), (101, 100));

        movement.set_position(cx as f32, cy as f32);
        let new_path = vec![make_path_node(102, 100, false)];
        movement.start_move(new_path, 0.075);

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

        movement.update(0.105);
        let (x, _) = movement.position();
        assert!((x - 100.7).abs() < 0.01, "setup: expected 100.7, got {x}");

        let new_path = vec![
            make_path_node(101, 100, false),
            make_path_node(102, 100, false),
            make_path_node(103, 100, false),
        ];
        movement.start_move(new_path, 0.105);

        let (x, _) = movement.update(0.105 + 0.0225);
        assert!((x - 100.85).abs() < 0.02, "expected ~100.85, got {x}");

        let (x, _) = movement.update(0.105 + 0.045 + 0.075);
        assert!((x - 101.5).abs() < 0.02, "expected ~101.5, got {x}");
    }
}
