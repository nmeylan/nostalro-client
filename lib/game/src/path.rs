use ragnarok_formats::gat::GatFile;

pub use movement::path::PathNode;

pub struct MoveAction {
    pub dest_x: u16,
    pub dest_y: u16,
    pub path: Vec<PathNode>,
}

/// Compute the destination cell within `range` of `(target_x, target_y)`,
/// in the direction of `(px, py)`. Returns a cell at Chebyshev distance
/// `range` from the target, or the target itself if the player is at the same position.
pub fn compute_destination_within_range(
    px: i32,
    py: i32,
    target_x: i32,
    target_y: i32,
    range: i32,
) -> (i32, i32) {
    let dx = px - target_x;
    let dy = py - target_y;

    let abs_dx = dx.abs() as f64;
    let abs_dy = dy.abs() as f64;

    if abs_dx == 0.0 && abs_dy == 0.0 {
        return (target_x, target_y);
    }

    // Normalize direction vector and scale by range
    let max_dist = abs_dx.max(abs_dy);
    let dir_x = (dx as f64 / max_dist) * range as f64;
    let dir_y = (dy as f64 / max_dist) * range as f64;

    (
        target_x + dir_x.round() as i32,
        target_y + dir_y.round() as i32,
    )
}

/// Try to move to a cell within `range` of `(dest_x, dest_y)` in the direction
/// of `(src_x, src_y)`. If the computed destination is blocked, tries all 8 neighbors.
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

    // Try the computed destination first
    if let Some(action) = try_move_to(gat, src_x, src_y, dest_x, dest_y) {
        return Some(action);
    }

    // Try all 8 neighbors (matching original game offset order)
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
        walkable[3] = false; // (1,1) unwalkable
        let data = build_gat_bytes(2, 2, &walkable);
        let gat = GatFile::parse(&data).unwrap();
        // Unwalkable destination
        assert!(try_move_to(&gat, 0, 0, 1, 1).is_none());
        // Out of bounds
        assert!(try_move_to(&gat, 0, 0, 5, 5).is_none());
    }

    #[test]
    fn path_search_navigates_around_blocked_cells() {
        let mut walkable = vec![true; 16];
        walkable[1] = false; // (1,0)
        walkable[5] = false; // (1,1)
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
        // Player at (10, 5), target at (0, 0), range=3
        // Normalized: (1.0, 0.5) * 3 = (3.0, 1.5) → round to (3, 2)
        let (dx, dy) = compute_destination_within_range(10, 5, 0, 0, 3);
        assert_eq!((dx, dy), (3, 2));
    }

    #[test]
    fn compute_destination_within_range_south_west() {
        // Player at (-5, -8), target at (0, 0), range=2
        // Normalized: (-0.625, -1.0) * 2 = (-1.25, -2.0) → round to (-1, -2)
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
        // Player directly north of target
        let (dx, dy) = compute_destination_within_range(0, 10, 0, 0, 3);
        assert_eq!((dx, dy), (0, 3));
    }

    #[test]
    fn try_move_to_range_stops_within_range() {
        // 5x5 map, all walkable. Player at (0,0), target at (4,4), range=2.
        // compute_destination: (-4,-4) normalized * 2 = (-2,-2) → dest=(2,2)
        let walkable = vec![true; 25];
        let data = build_gat_bytes(5, 5, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        let action = try_move_to_range(&gat, 0, 0, 4, 4, 2);
        assert!(action.is_some());
        let action = action.unwrap();
        // Should be at (2,2), not at (4,4) (target cell)
        assert_eq!(action.dest_x, 2);
        assert_eq!(action.dest_y, 2);
    }
}
