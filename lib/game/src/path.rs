use ragnarok_formats::gat::GatFile;

pub use movement::path::PathNode;

pub struct MoveAction {
    pub dest_x: u16,
    pub dest_y: u16,
    pub path: Vec<PathNode>,
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
    let cells: Vec<u16> = gat
        .cells
        .iter()
        .map(|c| u16::from(c.cell_type.is_walkable()))
        .collect();
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
}
