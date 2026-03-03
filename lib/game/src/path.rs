use ragnarok_formats::gat::GatFile;

pub use movement::path::PathNode;

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
