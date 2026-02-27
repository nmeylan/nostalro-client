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
    pub fn eye(&self) -> glam::Vec3 {
        self.target
            + glam::Vec3::new(
                self.distance * self.pitch.cos() * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                self.distance * self.pitch.cos() * self.yaw.cos(),
            )
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(self.eye(), self.target, glam::Vec3::Y)
    }

    pub fn projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn view_projection(&self) -> glam::Mat4 {
        self.projection_matrix() * self.view_matrix()
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
        // Pitch ~55 degrees => eye is above target
        assert!(eye.y > camera.target.y);
        // Default yaw 0 => eye is behind target on +Z side
        assert!(eye.z > camera.target.z);
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
