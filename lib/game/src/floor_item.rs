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

impl FloorItem {
    pub fn world_position(&self) -> (f32, f32) {
        (
            self.x as f32 + self.sub_x as f32 / 16.0,
            self.y as f32 + self.sub_y as f32 / 16.0,
        )
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
        // sub_x=6 → 6/16 = 0.375, sub_y=3 → 3/16 = 0.1875
        assert!((wx - 100.375).abs() < 0.001);
        assert!((wy - 200.1875).abs() < 0.001);
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
