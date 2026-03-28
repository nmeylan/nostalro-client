pub struct Camera {
    pub target: glam::Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: glam::Vec3::ZERO,
            distance: 200.0,
            yaw: 0.0,
            pitch: 55_f32.to_radians(),
            fov_y: 15_f32.to_radians(),
            aspect: 4.0 / 3.0,
            near: 1.0,
            far: 5000.0,
        }
    }
}

impl Camera {
    pub fn set_target(&mut self, x: f32, y: f32, z: f32) {
        self.target = glam::Vec3::new(x, y, z);
    }

    pub fn eye(&self) -> glam::Vec3 {
        self.target
            + glam::Vec3::new(
                self.distance * self.pitch.cos() * self.yaw.sin(),
                -self.distance * self.pitch.sin(),
                -self.distance * self.pitch.cos() * self.yaw.cos(),
            )
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(self.eye(), self.target, glam::Vec3::NEG_Y)
    }

    pub fn projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn view_projection(&self) -> glam::Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Project a world position to screen coordinates. Returns None if behind camera.
    pub fn world_to_screen(&self, wx: f32, wy: f32, wz: f32, screen_w: f32, screen_h: f32) -> Option<(f32, f32)> {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let sx = (ndc.x + 1.0) * 0.5 * screen_w;
        let sy = (1.0 - ndc.y) * 0.5 * screen_h;
        Some((sx, sy))
    }

    /// Project a world position to screen coordinates plus NDC depth for depth testing.
    pub fn world_to_screen_with_depth(&self, wx: f32, wy: f32, wz: f32, screen_w: f32, screen_h: f32) -> Option<(f32, f32, f32, f32)> {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let sx = (ndc.x + 1.0) * 0.5 * screen_w;
        let sy = (1.0 - ndc.y) * 0.5 * screen_h;
        Some((sx, sy, ndc.z, clip.w))
    }

    /// Pixels per world unit at a given world position (perspective scale).
    pub fn perspective_scale(&self, wx: f32, wy: f32, wz: f32, screen_h: f32) -> f32 {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return 1.0;
        }
        let proj_y = self.projection_matrix().col(1).y;
        proj_y / clip.w * screen_h / 2.0
    }

    /// Convert camera yaw to an 8-direction index (0-7).
    /// 0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE
    pub fn direction_index(&self) -> u8 {
        let angle = self.yaw.rem_euclid(std::f32::consts::TAU);
        // Each sector is PI/4 (45 degrees), offset by half a sector
        let sector = ((angle + std::f32::consts::FRAC_PI_8) / std::f32::consts::FRAC_PI_4) as u8;
        sector % 8
    }

    /// Unproject screen coordinates to a world-space ray (origin, direction).
    pub fn screen_to_ray(&self, screen_x: f32, screen_y: f32, screen_w: f32, screen_h: f32) -> (glam::Vec3, glam::Vec3) {
        let ndc_x = (2.0 * screen_x / screen_w) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / screen_h);
        let inv_vp = self.view_projection().inverse();

        let near = inv_vp * glam::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far = inv_vp * glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
        let near = near.truncate() / near.w;
        let far = far.truncate() / far.w;
        let dir = (far - near).normalize();
        (near, dir)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub eye_pos: [f32; 4],
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        let eye = camera.eye();
        Self {
            view_proj: camera.view_projection().to_cols_array_2d(),
            view: camera.view_matrix().to_cols_array_2d(),
            proj: camera.projection_matrix().to_cols_array_2d(),
            eye_pos: [eye.x, eye.y, eye.z, 1.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_eye_at_default_is_above_and_behind_target() {
        let camera = Camera::default();
        let eye = camera.eye();
        // In native RO coords, -Y = above
        assert!(eye.y < camera.target.y);
        // Default yaw 0 => eye is behind target on -Z side
        assert!(eye.z < camera.target.z);
    }

    #[test]
    fn view_projection_transforms_target_near_center() {
        let camera = Camera::default();
        let vp = camera.view_projection();
        let clip = vp * glam::Vec4::new(
            camera.target.x,
            camera.target.y,
            camera.target.z,
            1.0,
        );
        let ndc = clip.truncate() / clip.w;
        // Target should project near screen center
        assert!(ndc.x.abs() < 0.1, "ndc.x = {}", ndc.x);
        assert!(ndc.y.abs() < 0.5, "ndc.y = {}", ndc.y);
    }

    #[test]
    fn yaw_rotates_eye_around_target() {
        let mut camera = Camera::default();
        let eye0 = camera.eye();
        camera.yaw = std::f32::consts::FRAC_PI_2;
        let eye90 = camera.eye();
        // Eye should have moved significantly in X
        assert!((eye90.x - eye0.x).abs() > 50.0);
        // Y stays the same (yaw only changes horizontal angle)
        assert!((eye90.y - eye0.y).abs() < 0.01);
    }

    #[test]
    fn screen_center_ray_hits_near_target() {
        let camera = Camera::default();
        let (origin, dir) = camera.screen_to_ray(400.0, 300.0, 800.0, 600.0);
        // Ray should point toward ground (+Y in native RO) since camera is above (-Y)
        assert!(dir.y > 0.0, "dir.y = {}", dir.y);
        // Intersect with y=0 plane
        if dir.y.abs() > 1e-6 {
            let t = -origin.y / dir.y;
            let hit = origin + dir * t;
            // Should hit near the camera target (0,0,0)
            assert!(hit.x.abs() < 5.0, "hit.x = {}", hit.x);
            assert!(hit.z.abs() < 5.0, "hit.z = {}", hit.z);
        }
    }

    #[test]
    fn world_to_screen_target_projects_near_center() {
        let camera = Camera::default();
        let t = camera.target;
        let result = camera.world_to_screen(t.x, t.y, t.z, 800.0, 600.0);
        let (sx, sy) = result.expect("target should be visible");
        assert!((sx - 400.0).abs() < 50.0, "sx = {sx}");
        assert!((sy - 300.0).abs() < 200.0, "sy = {sy}");
    }

    #[test]
    fn direction_index_at_yaw_zero() {
        let camera = Camera::default();
        assert_eq!(camera.direction_index(), 0);
    }

    #[test]
    fn direction_index_rotates_with_yaw() {
        let mut camera = Camera::default();
        camera.yaw = std::f32::consts::FRAC_PI_2; // 90 degrees
        assert_eq!(camera.direction_index(), 2);
        camera.yaw = std::f32::consts::PI; // 180 degrees
        assert_eq!(camera.direction_index(), 4);
    }

    #[test]
    fn depth_bias_stays_bounded_across_zoom_levels() {
        let mut camera = Camera::default();
        const VIEW_SPACE_BIAS: f32 = 4.0;

        for &distance in &[50.0, 200.0, 500.0, 1000.0, 1500.0] {
            camera.distance = distance;
            let t = camera.target;
            let (_, _, _, clip_w) = camera
                .world_to_screen_with_depth(t.x, t.y, t.z, 800.0, 600.0)
                .expect("target should be visible");

            let ndc_bias = camera.near * VIEW_SPACE_BIAS / (clip_w * clip_w);
            let approx_world_bias = ndc_bias * clip_w * clip_w / camera.near;
            assert!(
                (approx_world_bias - VIEW_SPACE_BIAS).abs() < 0.01,
                "at distance {distance}: world bias = {approx_world_bias}, expected {VIEW_SPACE_BIAS}"
            );
            assert!(ndc_bias > 0.0);
            assert!(ndc_bias < 0.01, "ndc_bias too large at distance {distance}: {ndc_bias}");
        }
    }

    #[test]
    fn camera_uniform_matches_matrices() {
        let camera = Camera::default();
        let uniform = CameraUniform::from_camera(&camera);
        let vp_from_uniform = glam::Mat4::from_cols_array_2d(&uniform.view_proj);
        let vp_direct = camera.view_projection();
        for i in 0..16 {
            assert!(
                (vp_from_uniform.to_cols_array()[i] - vp_direct.to_cols_array()[i]).abs() < 1e-6,
            );
        }
    }
}
