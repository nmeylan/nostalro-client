use ragnarok_formats::gat::GatFile;

/// Handles coordinate conversion between GAT cells, GND cells, and world positions.
/// GAT and GND grids may have different resolutions; this struct encapsulates the ratio.
pub struct MapCoordinates {
    zoom: f32,
    gat_width: i32,
    gat_height: i32,
    gnd_width: i32,
    gnd_height: i32,
}

impl MapCoordinates {
    pub fn new(zoom: f32, gat_width: i32, gat_height: i32, gnd_width: i32, gnd_height: i32) -> Self {
        Self { zoom, gat_width, gat_height, gnd_width, gnd_height }
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Convert GAT cell position to world coordinates (wx, wy, wz).
    /// Accepts fractional cell values (e.g. cell + 0.5 for cell center).
    pub fn cell_to_world(&self, cell_x: f32, cell_y: f32) -> (f32, f32, f32) {
        let gnd_cell_x = cell_x * (self.gnd_width as f32 / self.gat_width as f32);
        let gnd_cell_y = cell_y * (self.gnd_height as f32 / self.gat_height as f32);
        let wx = (gnd_cell_x * self.zoom).clamp(0.0, self.gnd_width as f32 * self.zoom);
        let wz = (gnd_cell_y * self.zoom).clamp(0.0, self.gnd_height as f32 * self.zoom);
        (wx, 0.0, wz)
    }

    /// Convert world coordinates to GAT cell position.
    pub fn world_to_cell(&self, wx: f32, wz: f32) -> (i32, i32) {
        let gnd_cell_x = wx / self.zoom;
        let gnd_cell_y = wz / self.zoom;
        let cell_x = (gnd_cell_x * (self.gat_width as f32 / self.gnd_width as f32)) as i32;
        let cell_y = (gnd_cell_y * (self.gat_height as f32 / self.gnd_height as f32)) as i32;
        (cell_x, cell_y)
    }

    pub fn is_valid_cell(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.gat_width && y < self.gat_height
    }

    /// Compute world-space corners of a GAT cell, with per-corner heights from GAT.
    pub fn cell_corners_world(&self, gat: &GatFile, cx: i32, cy: i32) -> [[f32; 3]; 4] {
        let c0 = self.cell_to_world(cx as f32, cy as f32);
        let c1 = self.cell_to_world(cx as f32 + 1.0, cy as f32);
        let c2 = self.cell_to_world(cx as f32, cy as f32 + 1.0);
        let c3 = self.cell_to_world(cx as f32 + 1.0, cy as f32 + 1.0);
        let cell = &gat.cells[(cy * gat.width + cx) as usize];
        let y_off = -0.2;
        [
            [c0.0, cell.height_sw + y_off, c0.2],
            [c1.0, cell.height_se + y_off, c1.2],
            [c2.0, cell.height_nw + y_off, c2.2],
            [c3.0, cell.height_ne + y_off, c3.2],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_world_roundtrip_with_gat_gnd_ratio() {
        // GAT 2x GND resolution, zoom = 10
        let coords = MapCoordinates::new(10.0, 200, 200, 100, 100);
        // GAT cell (80, 60) → GND cell (40, 30) → world (400, 300)
        let (wx, wy, wz) = coords.cell_to_world(80.0, 60.0);
        assert!((wx - 400.0).abs() < 0.01);
        assert_eq!(wy, 0.0);
        assert!((wz - 300.0).abs() < 0.01);
        // Inverse
        let (cx, cy) = coords.world_to_cell(wx, wz);
        assert_eq!(cx, 80);
        assert_eq!(cy, 60);
    }

    #[test]
    fn cell_to_world_clamps_to_map_bounds() {
        let coords = MapCoordinates::new(5.0, 100, 100, 100, 100);
        let (wx, _, wz) = coords.cell_to_world(-10.0, 200.0);
        assert_eq!(wx, 0.0);
        assert_eq!(wz, 500.0);
    }

    #[test]
    fn is_valid_cell_checks_bounds() {
        let coords = MapCoordinates::new(5.0, 100, 80, 50, 40);
        assert!(coords.is_valid_cell(0, 0));
        assert!(coords.is_valid_cell(99, 79));
        assert!(!coords.is_valid_cell(-1, 0));
        assert!(!coords.is_valid_cell(100, 0));
        assert!(!coords.is_valid_cell(0, 80));
    }
}
