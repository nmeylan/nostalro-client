pub struct FloorItem {
    pub id: u32,
    pub item_id: u16,
    pub is_identified: bool,
    pub x: i16,
    pub y: i16,
    pub sub_x: u8,
    pub sub_y: u8,
    pub count: i16,
    pub name: String,
    pub resource_name: Option<String>,
    pub drop_time: f32,
    pub is_falling: bool,
    pub initial_y: f32,
}

/// World units the item is lifted above its cell when it drops.
const DROP_LIFT: f32 = 15.0;
const DROP_SPEED: f32 = -0.6;
const DROP_GRAVITY: f32 = 0.083;

/// Ticks of the blink cycle, and the first tick that flashes.
const BLINK_CYCLE_TICKS: i64 = 92;
const BLINK_FIRST_TICK: i64 = 90;

const TICK_MS: f32 = 24.0;

impl FloorItem {
    pub fn world_position(&self) -> (f32, f32) {
        (
            self.x as f32 + self.sub_x as f32 / 16.0,
            self.y as f32 + self.sub_y as f32 / 16.0,
        )
    }

    fn ticks_since_drop(&self, elapsed: f32) -> f32 {
        (elapsed - self.drop_time) * 1000.0 / TICK_MS
    }

    /// World height of the drop arc, clamped so the item never sinks below
    /// `ground_y`. World y grows downward, so the lift subtracts.
    pub fn drop_height(&self, elapsed: f32, ground_y: f32) -> f32 {
        if !self.is_falling {
            return ground_y;
        }
        let t = self.ticks_since_drop(elapsed);
        let y = self.initial_y - DROP_LIFT + (DROP_SPEED + DROP_GRAVITY * t) * t;
        y.min(ground_y)
    }

    /// Whether the item is in the two-tick flash of its own blink cycle.
    pub fn blink_active(&self, elapsed: f32) -> bool {
        let ticks = self.ticks_since_drop(elapsed) as i64;
        ticks.rem_euclid(BLINK_CYCLE_TICKS) >= BLINK_FIRST_TICK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_position_computes_correctly() {
        let item = FloorItem {
            id: 1,
            item_id: 501,
            is_identified: true,
            x: 100,
            y: 200,
            sub_x: 6,
            sub_y: 3,
            count: 1,
            name: String::new(),
            resource_name: None,
            drop_time: 0.0,
            is_falling: false,
            initial_y: 0.0,
        };
        let (wx, wy) = item.world_position();
        assert!((wx - 100.375).abs() < 0.001);
        assert!((wy - 200.1875).abs() < 0.001);
    }

    fn dropped(drop_time: f32, ground_y: f32) -> FloorItem {
        FloorItem {
            id: 1,
            item_id: 501,
            is_identified: true,
            x: 100,
            y: 100,
            sub_x: 8,
            sub_y: 8,
            count: 1,
            name: String::new(),
            resource_name: None,
            drop_time,
            is_falling: true,
            initial_y: ground_y,
        }
    }

    #[test]
    fn drop_arc_rises_then_lands_after_420ms() {
        let ground = 100.0;
        let item = dropped(0.0, ground);

        assert_eq!(item.drop_height(0.0, ground), ground - 15.0);
        // Drifts ~1.08 further up until tick 3.6 (86 ms), then falls back.
        let peak = item.drop_height(0.0868, ground);
        assert!((peak - (ground - 16.08)).abs() < 0.01, "peak {peak}");
        assert!(item.drop_height(0.2, ground) < ground);
        assert!(item.drop_height(0.415, ground) < ground);
        assert_eq!(item.drop_height(0.421, ground), ground);
        assert_eq!(item.drop_height(10.0, ground), ground);

        let resting = FloorItem {
            is_falling: false,
            ..dropped(0.0, ground)
        };
        assert_eq!(resting.drop_height(0.05, ground), ground);
    }

    #[test]
    fn blink_runs_per_item_so_two_drops_are_out_of_phase() {
        let early = dropped(0.0, 0.0);
        let late = dropped(1.0, 0.0);

        // Ticks 90 and 91 of the 92-tick cycle: 2160..2208 ms after the drop.
        assert!(!early.blink_active(2.0));
        assert!(early.blink_active(2.17));
        assert!(!early.blink_active(2.3));
        assert!(!late.blink_active(2.17));
        assert!(late.blink_active(3.17));
    }

    #[test]
    fn world_position_zero_sub_cell() {
        let item = FloorItem {
            id: 1,
            item_id: 501,
            is_identified: true,
            x: 50,
            y: 75,
            sub_x: 0,
            sub_y: 0,
            count: 1,
            name: String::new(),
            resource_name: None,
            drop_time: 0.0,
            is_falling: false,
            initial_y: 0.0,
        };
        let (wx, wy) = item.world_position();
        assert!((wx - 50.0).abs() < 0.001);
        assert!((wy - 75.0).abs() < 0.001);
    }
}
