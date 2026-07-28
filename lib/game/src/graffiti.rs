/// Side of the ground area a Graffiti decal covers, in cells.
pub const GRAFFITI_CELLS: f32 = 18.0;

pub struct Graffiti {
    pub creator_aid: u32,
    pub cell_x: u16,
    pub cell_y: u16,
    /// Facing of the caster when the message was written; the decal is laid out
    /// along it rather than along the world axes.
    pub yaw: f32,
    pub message: String,
}

/// Ground-plane corners of the decal, laid out around `center` and rotated by
/// `yaw`. `cell_size` is the world width of one GAT cell. U runs backwards so the
/// text reads the right way round from above.
pub fn decal_quad(center: [f32; 3], yaw: f32, cell_size: f32) -> ([[f32; 3]; 4], [[f32; 2]; 4]) {
    let half = GRAFFITI_CELLS * cell_size * 0.5;
    let (s, c) = yaw.sin_cos();
    let [cx, cy, cz] = center;
    let corner = |dx: f32, dz: f32| [cx + dx * c - dz * s, cy, cz + dx * s + dz * c];
    let corners = [
        corner(-half, -half),
        corner(half, -half),
        corner(half, half),
        corner(-half, half),
    ];
    let uv = [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    (corners, uv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrotated_decal_spans_the_cell_block_around_its_centre() {
        let (corners, uv) = decal_quad([100.0, 5.0, 200.0], 0.0, 5.0);

        assert_eq!(corners[0], [55.0, 5.0, 155.0]);
        assert_eq!(corners[2], [145.0, 5.0, 245.0]);
        assert!(corners.iter().all(|c| c[1] == 5.0));
        assert_eq!(uv[0][0], 1.0);
    }

    #[test]
    fn a_quarter_turn_rotates_the_footprint_about_the_centre() {
        let (corners, _) = decal_quad([0.0, 0.0, 0.0], std::f32::consts::FRAC_PI_2, 1.0);

        assert!((corners[0][0] - 9.0).abs() < 1e-4);
        assert!((corners[0][2] + 9.0).abs() < 1e-4);
    }
}
