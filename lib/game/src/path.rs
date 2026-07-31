use ragnarok_formats::gat::GatFile;

pub use movement::path::PathNode;

pub struct MoveAction {
    pub dest_x: u16,
    pub dest_y: u16,
    pub path: Vec<PathNode>,
}

/// Cell reach as the server measures it: circular, minus a bonus that makes a
/// straight line read one cell shorter than it is.
pub fn reach_cell_distance(dx: i32, dy: i32) -> i32 {
    let squared = (dx * dx + dy * dy) as f64;
    (squared.sqrt() - 0.1).max(0.0) as i32
}

pub fn in_attack_range(px: i32, py: i32, target_x: i32, target_y: i32, range: i32) -> bool {
    reach_cell_distance(px - target_x, py - target_y) <= range.max(0)
}

pub fn compute_destination_within_range(
    px: i32,
    py: i32,
    target_x: i32,
    target_y: i32,
    range: i32,
) -> (i32, i32) {
    let dx = px - target_x;
    let dy = py - target_y;

    let length = ((dx * dx + dy * dy) as f64).sqrt();
    if length == 0.0 {
        return (target_x, target_y);
    }

    let dir_x = (dx as f64 / length) * range as f64;
    let dir_y = (dy as f64 / length) * range as f64;

    (
        target_x + dir_x.round() as i32,
        target_y + dir_y.round() as i32,
    )
}

pub fn try_move_to_range(
    gat: &GatFile,
    src_x: u16,
    src_y: u16,
    dest_x: i32,
    dest_y: i32,
    range: i32,
) -> Option<MoveAction> {
    let (dest_x, dest_y) =
        compute_destination_within_range(src_x as i32, src_y as i32, dest_x, dest_y, range);

    if let Some(action) = try_move_to(gat, src_x, src_y, dest_x, dest_y) {
        return Some(action);
    }

    let offsets = [
        (1, 0),
        (1, 1),
        (1, -1),
        (-1, 0),
        (-1, 1),
        (-1, -1),
        (0, 1),
        (0, -1),
    ];
    for (dx, dy) in offsets {
        let nx = dest_x + dx;
        let ny = dest_y + dy;
        if let Some(action) = try_move_to(gat, src_x, src_y, nx, ny) {
            return Some(action);
        }
    }

    None
}

pub fn try_move_to(
    gat: &GatFile,
    src_x: u16,
    src_y: u16,
    dest_x: i32,
    dest_y: i32,
) -> Option<MoveAction> {
    if !gat.is_walkable(dest_x, dest_y) {
        return None;
    }
    let path = path_search(gat, src_x, src_y, dest_x as u16, dest_y as u16);
    if path.is_empty() {
        return None;
    }
    Some(MoveAction {
        dest_x: dest_x as u16,
        dest_y: dest_y as u16,
        path,
    })
}

pub fn path_search(
    gat: &GatFile,
    source_x: u16,
    source_y: u16,
    destination_x: u16,
    destination_y: u16,
) -> Vec<PathNode> {
    let cells: Vec<u16> = gat.cells.iter().map(|c| c.cell_flags).collect();
    movement::path::path_search_client_side_algorithm(
        gat.width as u16,
        gat.height as u16,
        &cells,
        source_x,
        source_y,
        destination_x,
        destination_y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_gat_bytes(width: i32, height: i32, walkable: &[bool]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"GRAT");
        data.push(1);
        data.push(2);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        for &w in walkable {
            for _ in 0..4 {
                data.extend_from_slice(&0.0_f32.to_le_bytes());
            }
            let cell_type: i32 = if w { 0 } else { 1 };
            data.extend_from_slice(&cell_type.to_le_bytes());
        }
        data
    }

    #[test]
    fn try_move_to_walkable_returns_path() {
        let walkable = vec![true; 9];
        let data = build_gat_bytes(3, 3, &walkable);
        let gat = GatFile::parse(&data).unwrap();
        let action = try_move_to(&gat, 0, 0, 2, 2);
        assert!(action.is_some());
        let action = action.unwrap();
        assert_eq!(action.dest_x, 2);
        assert_eq!(action.dest_y, 2);
        assert!(!action.path.is_empty());
    }

    #[test]
    fn try_move_to_unwalkable_returns_none() {
        let mut walkable = vec![true; 4];
        walkable[3] = false;
        let data = build_gat_bytes(2, 2, &walkable);
        let gat = GatFile::parse(&data).unwrap();
        assert!(try_move_to(&gat, 0, 0, 1, 1).is_none());
        assert!(try_move_to(&gat, 0, 0, 5, 5).is_none());
    }

    #[test]
    fn path_search_navigates_around_blocked_cells() {
        let mut walkable = vec![true; 16];
        walkable[1] = false;
        walkable[5] = false;
        let data = build_gat_bytes(4, 4, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        let path = path_search(&gat, 0, 0, 3, 0);
        assert!(!path.is_empty());
        assert_eq!((path.last().unwrap().x, path.last().unwrap().y), (3, 0));
        for node in &path {
            assert!((node.x, node.y) != (1, 0));
            assert!((node.x, node.y) != (1, 1));
        }
    }

    #[test]
    fn compute_destination_within_range_north_east() {
        let (dx, dy) = compute_destination_within_range(10, 5, 0, 0, 3);
        assert_eq!((dx, dy), (3, 1));
    }

    #[test]
    fn approach_cell_is_within_reach_on_every_diagonal() {
        for range in 1..=9 {
            let (dx, dy) = compute_destination_within_range(40, 40, 20, 20, range);
            assert!(
                in_attack_range(dx, dy, 20, 20, range),
                "range {range} lands on ({dx},{dy}), out of reach"
            );
        }
    }

    #[test]
    fn reach_is_circular_with_the_straight_line_bonus() {
        assert!(in_attack_range(6, 0, 0, 0, 5));
        assert!(!in_attack_range(7, 0, 0, 0, 5));
        assert!(in_attack_range(3, 3, 0, 0, 5));
        assert!(!in_attack_range(5, 5, 0, 0, 5));
        assert!(in_attack_range(1, 1, 0, 0, 1));
    }

    #[test]
    fn compute_destination_within_range_south_west() {
        let (dx, dy) = compute_destination_within_range(-5, -8, 0, 0, 2);
        assert_eq!((dx, dy), (-1, -2));
    }

    #[test]
    fn compute_destination_within_range_same_position() {
        let (dx, dy) = compute_destination_within_range(5, 5, 5, 5, 3);
        assert_eq!((dx, dy), (5, 5));
    }

    #[test]
    fn compute_destination_within_range_axis_aligned() {
        let (dx, dy) = compute_destination_within_range(0, 10, 0, 0, 3);
        assert_eq!((dx, dy), (0, 3));
    }

    #[test]
    fn try_move_to_range_stops_within_range() {
        let walkable = vec![true; 25];
        let data = build_gat_bytes(5, 5, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        let action = try_move_to_range(&gat, 0, 0, 4, 4, 2);
        assert!(action.is_some());
        let action = action.unwrap();
        assert_eq!(action.dest_x, 3);
        assert_eq!(action.dest_y, 3);
    }
}
