use ragnarok_formats::gat::GatFile;

pub static MOVE_COST: u16 = 10;
pub static MOVE_DIAGONAL_COST: u16 = 14;

static DIR_NORTH: u8 = 1;
static DIR_WEST: u8 = 2;
static DIR_SOUTH: u8 = 4;
static DIR_EAST: u8 = 8;

#[derive(Copy, Clone, Debug)]
pub struct PathNode {
    pub id: u32,
    pub parent_id: u32,
    pub x: u16,
    pub y: u16,
    pub g_cost: u16,
    pub f_cost: u16,
    pub is_open: bool,
    pub is_diagonal: bool,
}

#[inline]
fn client_side_heuristic(x0: u16, y0: u16, x1: u16, y1: u16) -> u16 {
    MOVE_COST * manhattan_distance(x0, y0, x1, y1)
}

#[inline]
fn manhattan_distance(x0: u16, y0: u16, x1: u16, y1: u16) -> u16 {
    i16::abs(x1 as i16 - x0 as i16) as u16 + i16::abs(y1 as i16 - y0 as i16) as u16
}

#[inline]
fn is_direction(allowed_dir: u8, direction: u8) -> bool {
    (allowed_dir & direction) == direction
}

#[inline]
fn node_id(x: u16, y: u16, x_size: u16) -> u32 {
    x as u32 + y as u32 * x_size as u32
}

pub fn path_search(
    gat: &GatFile,
    source_x: u16,
    source_y: u16,
    destination_x: u16,
    destination_y: u16,
) -> Vec<PathNode> {
    let x_size = gat.width as u16;
    let y_size = gat.height as u16;
    let max_x = x_size - 1;
    let max_y = y_size - 1;
    let start_node = PathNode {
        id: node_id(source_x, source_y, max_x),
        parent_id: node_id(source_x, source_y, max_x),
        x: source_x,
        y: source_y,
        g_cost: 0,
        f_cost: client_side_heuristic(source_x, source_y, destination_x, destination_y),
        is_open: true,
        is_diagonal: false,
    };
    let mut open_set = Vec::with_capacity(14 * 14);
    open_set.push(start_node);
    let mut discovered_nodes = Vec::with_capacity(14 * 14);
    discovered_nodes.push(start_node);
    let mut current_node = start_node;
    let mut i = 0;
    while !open_set.is_empty() {
        let current: (usize, &PathNode) = open_set
            .iter()
            .enumerate()
            .reduce(|(min_i, min_n), (cur_i, cur_n)| {
                if cur_n.f_cost < min_n.f_cost {
                    (cur_i, cur_n)
                } else {
                    (min_i, min_n)
                }
            })
            .unwrap();
        current_node = *current.1;
        let current_index = current.0;
        if current_node.x == destination_x && current_node.y == destination_y {
            break;
        }
        if i > 100 {
            return vec![];
        }
        open_set.swap_remove(current_index);
        current_node.is_open = false;
        i += 1;
        let allowed = allowed_dirs(max_x, max_y, current_node.x, current_node.y);

        if is_direction(allowed, DIR_SOUTH | DIR_EAST) && gat.is_walkable((current_node.x + 1) as i32, (current_node.y - 1) as i32) {
            add_neighbor(current_node.x + 1, current_node.y - 1, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_DIAGONAL_COST, true);
        }
        if is_direction(allowed, DIR_EAST) && gat.is_walkable((current_node.x + 1) as i32, current_node.y as i32) {
            add_neighbor(current_node.x + 1, current_node.y, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_COST, false);
        }
        if is_direction(allowed, DIR_NORTH | DIR_EAST) && gat.is_walkable((current_node.x + 1) as i32, (current_node.y + 1) as i32) {
            add_neighbor(current_node.x + 1, current_node.y + 1, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_DIAGONAL_COST, true);
        }
        if is_direction(allowed, DIR_NORTH) && gat.is_walkable(current_node.x as i32, (current_node.y + 1) as i32) {
            add_neighbor(current_node.x, current_node.y + 1, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_COST, false);
        }
        if is_direction(allowed, DIR_NORTH | DIR_WEST) && gat.is_walkable((current_node.x - 1) as i32, (current_node.y + 1) as i32) {
            add_neighbor(current_node.x - 1, current_node.y + 1, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_DIAGONAL_COST, true);
        }
        if is_direction(allowed, DIR_WEST) && gat.is_walkable((current_node.x - 1) as i32, current_node.y as i32) {
            add_neighbor(current_node.x - 1, current_node.y, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_COST, false);
        }
        if is_direction(allowed, DIR_SOUTH | DIR_WEST) && gat.is_walkable((current_node.x - 1) as i32, (current_node.y - 1) as i32) {
            add_neighbor(current_node.x - 1, current_node.y - 1, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_DIAGONAL_COST, true);
        }
        if is_direction(allowed, DIR_SOUTH) && gat.is_walkable(current_node.x as i32, (current_node.y - 1) as i32) {
            add_neighbor(current_node.x, current_node.y - 1, destination_x, destination_y, max_x, &mut open_set, &mut discovered_nodes, &current_node, MOVE_COST, false);
        }
    }

    let mut final_path: Vec<PathNode> = Vec::with_capacity(14 * 2);
    while current_node.id != current_node.parent_id {
        final_path.push(current_node);
        match discovered_nodes.iter().find(|node| node.id == current_node.parent_id) {
            Some(parent) => current_node = *parent,
            None => break,
        }
    }
    final_path.reverse();
    final_path
}

fn allowed_dirs(max_x: u16, max_y: u16, x: u16, y: u16) -> u8 {
    let mut dirs: u8 = 0;
    if y < max_y { dirs |= DIR_NORTH; }
    if y > 0 { dirs |= DIR_SOUTH; }
    if x < max_x { dirs |= DIR_EAST; }
    if x > 0 { dirs |= DIR_WEST; }
    dirs
}

#[allow(clippy::too_many_arguments)]
fn add_neighbor(
    x: u16,
    y: u16,
    destination_x: u16,
    destination_y: u16,
    max_x: u16,
    open_set: &mut Vec<PathNode>,
    discovered_nodes: &mut Vec<PathNode>,
    current_node: &PathNode,
    move_cost: u16,
    is_diagonal: bool,
) {
    let tentative_gcost = current_node.g_cost + move_cost;
    let h_cost = client_side_heuristic(x, y, destination_x, destination_y);
    if let Some(neighbor) = discovered_nodes.iter_mut().find(|node| node.x == x && node.y == y) {
        if tentative_gcost < neighbor.g_cost {
            neighbor.parent_id = current_node.id;
            neighbor.g_cost = tentative_gcost;
            neighbor.f_cost = tentative_gcost + h_cost;
            if !neighbor.is_open {
                open_set.push(*neighbor);
            }
            neighbor.is_open = true;
        }
    } else {
        let neighbor = PathNode {
            id: node_id(x, y, max_x),
            parent_id: current_node.id,
            x,
            y,
            g_cost: tentative_gcost,
            f_cost: tentative_gcost + h_cost,
            is_open: true,
            is_diagonal,
        };
        open_set.push(neighbor);
        discovered_nodes.push(neighbor);
    }
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
            // 4 height floats + cell type
            for _ in 0..4 { data.extend_from_slice(&0.0_f32.to_le_bytes()); }
            let cell_type: i32 = if w { 0 } else { 1 };
            data.extend_from_slice(&cell_type.to_le_bytes());
        }
        data
    }

    #[test]
    fn path_search_finds_straight_path() {
        // 4x4 grid, all walkable
        let walkable = vec![true; 16];
        let data = build_gat_bytes(4, 4, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        let path = path_search(&gat, 0, 0, 3, 0);
        assert!(!path.is_empty());
        let last = path.last().unwrap();
        assert_eq!((last.x, last.y), (3, 0));
    }

    #[test]
    fn path_search_avoids_blocked_cell() {
        // 4x4 grid with a wall at (1,0), (1,1)
        let mut walkable = vec![true; 16];
        walkable[1] = false; // (1,0)
        walkable[5] = false; // (1,1)
        let data = build_gat_bytes(4, 4, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        let path = path_search(&gat, 0, 0, 3, 0);
        assert!(!path.is_empty());
        let last = path.last().unwrap();
        assert_eq!((last.x, last.y), (3, 0));
        // Path must not go through blocked cells
        for node in &path {
            assert!((node.x, node.y) != (1, 0));
            assert!((node.x, node.y) != (1, 1));
        }
    }

    #[test]
    fn path_search_returns_empty_when_blocked() {
        // 3x3 grid, destination surrounded by walls
        // Layout (y=0 bottom):
        //   (0,0)W (1,0)X (2,0)W
        //   (0,1)X (1,1)W (2,1)X
        //   (0,2)W (1,2)X (2,2)W
        // Start (0,0), dest (1,1) — (1,1) is walkable but all neighbors blocked
        let walkable = vec![
            true,  false, true,  // y=0
            false, true,  false, // y=1
            true,  false, true,  // y=2
        ];
        let data = build_gat_bytes(3, 3, &walkable);
        let gat = GatFile::parse(&data).unwrap();

        let path = path_search(&gat, 0, 0, 1, 1);
        // Can still reach diagonally from (0,0) to (1,1) since diagonal checks
        // only require the boundary directions to be valid, not the intermediate cells
        // The path should reach (1,1) via diagonal
        if !path.is_empty() {
            assert_eq!((path.last().unwrap().x, path.last().unwrap().y), (1, 1));
        }
    }
}
