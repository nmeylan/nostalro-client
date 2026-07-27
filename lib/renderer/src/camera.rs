pub struct Camera {
    pub target: glam::Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    /// Where the camera is heading; `target`/`distance`/`yaw`/`pitch` chase these
    /// every frame through [`Camera::interpolate`].
    pub dest_target: glam::Vec3,
    pub dest_distance: f32,
    pub dest_yaw: f32,
    pub dest_pitch: f32,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub shake_offset: glam::Vec3,
    /// Ground height under the eye, keeping it from sinking through terrain.
    /// World Y grows downwards, so this is an upper bound on the eye's Y.
    pub eye_floor: Option<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: glam::Vec3::ZERO,
            distance: DEFAULT_DISTANCE,
            yaw: 0.0,
            pitch: DEFAULT_PITCH_DEG.to_radians(),
            dest_target: glam::Vec3::ZERO,
            dest_distance: DEFAULT_DISTANCE,
            dest_yaw: 0.0,
            dest_pitch: DEFAULT_PITCH_DEG.to_radians(),
            fov_y: 15_f32.to_radians(),
            aspect: 4.0 / 3.0,
            near: 1.0,
            far: 5000.0,
            shake_offset: glam::Vec3::ZERO,
            eye_floor: None,
        }
    }
}

/// Indoor maps clamp the zoom to a narrow band and forbid free rotation, matching
/// the original client's fixed indoor camera.
pub const INDOOR_MIN_DISTANCE: f32 = 150.0;
pub const INDOOR_MAX_DISTANCE: f32 = 300.0;
pub const OUTDOOR_MIN_DISTANCE: f32 = 50.0;
pub const OUTDOOR_MAX_DISTANCE: f32 = 1500.0;
pub const DEFAULT_DISTANCE: f32 = 200.0;

const DEFAULT_PITCH_DEG: f32 = 55.0;
const INDOOR_YAW_DEG: f32 = -45.0;
const INDOOR_YAW_HALF_DEG: f32 = 20.0;
const PITCH_CENTER_DEG: f32 = 45.0;
const OUTDOOR_PITCH_HALF_DEG: f32 = 20.0;
const INDOOR_PITCH_HALF_DEG: f32 = 10.0;
/// Without Ctrl the band's floor is measured from 70° instead of 45°.
const PITCH_NARROW_CENTER_DEG: f32 = 70.0;
const YAW_DRAG_DEG_PER_PIXEL: f32 = 1.0;
const PITCH_DRAG_DEG_PER_PIXEL: f32 = 0.5;
const DISTANCE_DRAG_PER_PIXEL: f32 = 0.5;
const WHEEL_DISTANCE_STEP: f32 = 24.3;
const WHEEL_PITCH_STEP_DEG: f32 = 5.0;
/// Distance below which the bypassed zoom clamp still refuses to go, so the
/// projection never degenerates.
const BYPASS_MIN_DISTANCE: f32 = 5.0;
const STAR_GAZE_DISTANCE_FACTOR: f32 = 3.0;
const STAR_GAZE_MIN_PITCH_DEG: f32 = 50.0;

const INTERP_STEP: f32 = 1.0 / 60.0;
const INTERP_MAX_STEPS: f32 = 4.0;

const ANGLE_BIG_DEG: f32 = 5.5;
const ANGLE_SMALL_DEG: f32 = 0.06;
const ANGLE_BIG_DIVISOR: f32 = 10.0;
const ANGLE_SMALL_DIVISOR: f32 = 4.0;
const DISTANCE_BIG: f32 = 5.0;
const DISTANCE_SMALL: f32 = 0.04;
const DISTANCE_BIG_DIVISOR: f32 = 1.0;
const DISTANCE_SMALL_DIVISOR: f32 = 0.6;
const TARGET_BIG: f32 = 5.0;
const TARGET_SMALL: f32 = 0.08;
const TARGET_BIG_DIVISOR: f32 = 9.0;
const TARGET_SMALL_DIVISOR: f32 = 4.0;

/// One frame's move towards a destination: a big error is divided down hard, a
/// small one softly, and anything under `small` snaps.
fn interpolation_step(error: f32, big: f32, small: f32, big_div: f32, small_div: f32) -> f32 {
    if error.abs() > big {
        error / big_div
    } else if error.abs() > small {
        error / small_div
    } else {
        error
    }
}

fn wrap_pi(angle: f32) -> f32 {
    let wrapped = angle.rem_euclid(std::f32::consts::TAU);
    if wrapped > std::f32::consts::PI {
        wrapped - std::f32::consts::TAU
    } else {
        wrapped
    }
}

/// Which modifiers are held and what the current map/player allow, resolved by
/// the caller and handed to every camera adjustment.
#[derive(Clone, Copy, Default, Debug)]
pub struct CameraControl {
    pub indoor: bool,
    /// GM accounts may rotate freely on indoor maps.
    pub gm: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// Star Gladiator sitting without Demon of the Sun, Moon and Stars: zooms out
    /// three times as far to watch the sky.
    pub star_gazing: bool,
    /// Escape hatch for screenshots: drop the angle bands entirely.
    pub unbounded: bool,
}

/// Yaw, pitch and distance are remembered per environment and restored on map
/// entry.
#[derive(Clone, Copy, Debug)]
pub struct SavedCameraView {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for SavedCameraView {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: DEFAULT_PITCH_DEG.to_radians(),
            distance: DEFAULT_DISTANCE,
        }
    }
}

fn pitch_half_width_deg(indoor: bool) -> f32 {
    if indoor {
        INDOOR_PITCH_HALF_DEG
    } else {
        OUTDOOR_PITCH_HALF_DEG
    }
}

fn distance_range(indoor: bool) -> (f32, f32) {
    if indoor {
        (INDOOR_MIN_DISTANCE, INDOOR_MAX_DISTANCE)
    } else {
        (OUTDOOR_MIN_DISTANCE, OUTDOOR_MAX_DISTANCE)
    }
}

impl Camera {
    pub fn with_aspect(aspect: f32) -> Self {
        Self {
            aspect,
            ..Default::default()
        }
    }

    /// Aim the camera at a cell; the current target glides there.
    pub fn set_target(&mut self, x: f32, y: f32, z: f32) {
        self.dest_target = glam::Vec3::new(x, y, z);
    }

    pub fn snap_target(&mut self) {
        self.target = self.dest_target;
    }

    pub fn saved_view(&self) -> SavedCameraView {
        SavedCameraView {
            yaw: self.dest_yaw,
            pitch: self.dest_pitch,
            distance: self.dest_distance,
        }
    }

    /// Restore the environment's saved angles and distance, then pull the
    /// destination distance to the default so entry glides into framing.
    pub fn on_map_enter(&mut self, indoor: bool, saved: SavedCameraView, default_distance: f32) {
        let half = pitch_half_width_deg(indoor);
        let pitch = saved
            .pitch
            .to_degrees()
            .clamp(PITCH_CENTER_DEG - half, PITCH_CENTER_DEG + half)
            .to_radians();
        self.pitch = pitch;
        self.dest_pitch = pitch;
        let yaw = if indoor {
            INDOOR_YAW_DEG.to_radians()
        } else {
            saved.yaw
        };
        self.yaw = yaw;
        self.dest_yaw = yaw;
        self.distance = saved.distance;
        self.dest_distance = default_distance;
    }

    pub fn apply_drag(&mut self, dx: f32, dy: f32, control: CameraControl) {
        if !control.alt && !control.shift {
            self.rotate(dx * YAW_DRAG_DEG_PER_PIXEL, control);
        }
        if !control.alt && control.shift {
            self.adjust_pitch(dy * PITCH_DRAG_DEG_PER_PIXEL, control);
        }
        if control.ctrl {
            self.adjust_distance(-dy * DISTANCE_DRAG_PER_PIXEL, control);
        }
    }

    pub fn apply_wheel(&mut self, notches: f32, control: CameraControl) {
        if control.shift {
            self.adjust_pitch(-notches * WHEEL_PITCH_STEP_DEG, control);
        } else {
            self.adjust_distance(notches * WHEEL_DISTANCE_STEP, control);
        }
    }

    fn rotate(&mut self, delta_deg: f32, control: CameraControl) {
        let yaw = self.dest_yaw + delta_deg.to_radians();
        self.dest_yaw = if control.indoor && !control.gm && !control.unbounded {
            let fixed = INDOOR_YAW_DEG.to_radians();
            let half = INDOOR_YAW_HALF_DEG.to_radians();
            yaw.clamp(fixed - half, fixed + half)
        } else {
            yaw
        };
    }

    fn adjust_pitch(&mut self, delta_deg: f32, control: CameraControl) {
        let mut deg = self.dest_pitch.to_degrees() + delta_deg;
        if !control.unbounded {
            let half = pitch_half_width_deg(control.indoor);
            deg = deg.min(PITCH_CENTER_DEG + half);
            // Ctrl widens the floor, and the angle it reaches survives release
            // because nothing re-clamps until the next adjustment.
            let floor = if control.ctrl && self.dest_distance <= OUTDOOR_MAX_DISTANCE {
                PITCH_CENTER_DEG - half
            } else {
                PITCH_NARROW_CENTER_DEG - half
            };
            deg = deg.max(floor);
        }
        self.dest_pitch = deg.to_radians();
    }

    fn adjust_distance(&mut self, delta: f32, control: CameraControl) {
        let (min, max) = distance_range(control.indoor);
        let distance = self.dest_distance + delta;
        self.dest_distance = if control.star_gazing {
            let distance = distance.clamp(min, max * STAR_GAZE_DISTANCE_FACTOR);
            if distance > max {
                self.dest_pitch = self.dest_pitch.max(STAR_GAZE_MIN_PITCH_DEG.to_radians());
            }
            distance
        } else if control.ctrl || control.unbounded {
            distance.max(BYPASS_MIN_DISTANCE)
        } else {
            distance.clamp(min, max)
        };
    }

    /// Advance the current view towards its destination: one step per frame at
    /// the nominal 60Hz, more only when a frame ran long. Rounding rather than
    /// accumulating keeps a jittery frame clock from turning into a jittery
    /// camera.
    pub fn interpolate(&mut self, delta: f32) {
        let steps = (delta / INTERP_STEP).round().clamp(1.0, INTERP_MAX_STEPS) as u32;
        for _ in 0..steps {
            self.interpolate_once();
        }
    }

    fn interpolate_once(&mut self) {
        self.dest_yaw = wrap_pi(self.dest_yaw);
        let yaw_error = wrap_pi(self.dest_yaw - self.yaw).to_degrees();
        self.yaw = wrap_pi(self.yaw + Self::angle_step(yaw_error).to_radians());

        let pitch_error = (self.dest_pitch - self.pitch).to_degrees();
        self.pitch += Self::angle_step(pitch_error).to_radians();

        self.distance += interpolation_step(
            self.dest_distance - self.distance,
            DISTANCE_BIG,
            DISTANCE_SMALL,
            DISTANCE_BIG_DIVISOR,
            DISTANCE_SMALL_DIVISOR,
        );

        for axis in 0..3 {
            self.target[axis] += interpolation_step(
                self.dest_target[axis] - self.target[axis],
                TARGET_BIG,
                TARGET_SMALL,
                TARGET_BIG_DIVISOR,
                TARGET_SMALL_DIVISOR,
            );
        }
    }

    fn angle_step(error_deg: f32) -> f32 {
        interpolation_step(
            error_deg,
            ANGLE_BIG_DEG,
            ANGLE_SMALL_DEG,
            ANGLE_BIG_DIVISOR,
            ANGLE_SMALL_DIVISOR,
        )
    }

    /// Eye position before the terrain clamp; sample the ground here to fill
    /// [`Camera::eye_floor`].
    pub fn eye_unclamped(&self) -> glam::Vec3 {
        self.target
            + glam::Vec3::new(
                self.distance * self.pitch.cos() * self.yaw.sin(),
                -self.distance * self.pitch.sin(),
                -self.distance * self.pitch.cos() * self.yaw.cos(),
            )
    }

    pub fn eye(&self) -> glam::Vec3 {
        let mut eye = self.eye_unclamped();
        if let Some(floor) = self.eye_floor
            && eye.y > floor
        {
            eye.y = floor;
        }
        eye
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(
            self.eye() + self.shake_offset,
            self.target + self.shake_offset,
            glam::Vec3::NEG_Y,
        )
    }

    pub fn projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn view_projection(&self) -> glam::Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn right_vector(&self) -> glam::Vec3 {
        self.view_matrix().row(0).truncate()
    }

    pub fn world_to_screen(
        &self,
        wx: f32,
        wy: f32,
        wz: f32,
        screen_w: f32,
        screen_h: f32,
    ) -> Option<(f32, f32)> {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let sx = (ndc.x + 1.0) * 0.5 * screen_w;
        let sy = (1.0 - ndc.y) * 0.5 * screen_h;
        Some((sx, sy))
    }

    pub fn is_world_pos_visible(&self, wx: f32, wy: f32, wz: f32, ndc_margin: f32) -> bool {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return false;
        }
        let ndc = clip.truncate() / clip.w;
        let m = 1.0 + ndc_margin;
        ndc.x.abs() <= m && ndc.y.abs() <= m && ndc.z >= 0.0 && ndc.z <= 1.0
    }

    pub fn world_to_screen_with_depth(
        &self,
        wx: f32,
        wy: f32,
        wz: f32,
        screen_w: f32,
        screen_h: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let sx = (ndc.x + 1.0) * 0.5 * screen_w;
        let sy = (1.0 - ndc.y) * 0.5 * screen_h;
        Some((sx, sy, ndc.z, clip.w))
    }

    pub fn perspective_scale(&self, wx: f32, wy: f32, wz: f32, screen_h: f32) -> f32 {
        let clip = self.view_projection() * glam::Vec4::new(wx, wy, wz, 1.0);
        if clip.w <= 0.0 {
            return 1.0;
        }
        let proj_y = self.projection_matrix().col(1).y;
        proj_y / clip.w * screen_h / 2.0
    }

    /// 0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE
    pub fn direction_index(&self) -> u8 {
        let angle = self.yaw.rem_euclid(std::f32::consts::TAU);
        let sector = ((angle + std::f32::consts::FRAC_PI_8) / std::f32::consts::FRAC_PI_4) as u8;
        sector % 8
    }

    pub fn screen_to_ray(
        &self,
        screen_x: f32,
        screen_y: f32,
        screen_w: f32,
        screen_h: f32,
    ) -> (glam::Vec3, glam::Vec3) {
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
        assert!(eye.y < camera.target.y);
        assert!(eye.z < camera.target.z);
    }

    #[test]
    fn view_projection_transforms_target_near_center() {
        let camera = Camera::default();
        let vp = camera.view_projection();
        let clip = vp * glam::Vec4::new(camera.target.x, camera.target.y, camera.target.z, 1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(ndc.x.abs() < 0.1, "ndc.x = {}", ndc.x);
        assert!(ndc.y.abs() < 0.5, "ndc.y = {}", ndc.y);
    }

    #[test]
    fn yaw_rotates_eye_around_target() {
        let mut camera = Camera::default();
        let eye0 = camera.eye();
        camera.yaw = std::f32::consts::FRAC_PI_2;
        let eye90 = camera.eye();
        assert!((eye90.x - eye0.x).abs() > 50.0);
        assert!((eye90.y - eye0.y).abs() < 0.01);
    }

    #[test]
    fn screen_center_ray_hits_near_target() {
        let camera = Camera::default();
        let (origin, dir) = camera.screen_to_ray(400.0, 300.0, 800.0, 600.0);
        assert!(dir.y > 0.0, "dir.y = {}", dir.y);
        if dir.y.abs() > 1e-6 {
            let t = -origin.y / dir.y;
            let hit = origin + dir * t;
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
        camera.yaw = std::f32::consts::FRAC_PI_2;
        assert_eq!(camera.direction_index(), 2);
        camera.yaw = std::f32::consts::PI;
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
            assert!(
                ndc_bias < 0.01,
                "ndc_bias too large at distance {distance}: {ndc_bias}"
            );
        }
    }

    #[test]
    fn interpolation_bands_divide_then_snap() {
        let big = interpolation_step(90.0, ANGLE_BIG_DEG, ANGLE_SMALL_DEG, 10.0, 4.0);
        let small = interpolation_step(1.0, ANGLE_BIG_DEG, ANGLE_SMALL_DEG, 10.0, 4.0);
        let tiny = interpolation_step(0.01, ANGLE_BIG_DEG, ANGLE_SMALL_DEG, 10.0, 4.0);
        assert!((big - 9.0).abs() < 1e-5);
        assert!((small - 0.25).abs() < 1e-5);
        assert!((tiny - 0.01).abs() < 1e-9);
    }

    #[test]
    fn yaw_interpolation_turns_the_short_way_and_settles() {
        let mut camera = Camera::default();
        camera.yaw = 10_f32.to_radians();
        camera.dest_yaw = 350_f32.to_radians();
        camera.interpolate(1.0 / 60.0);
        assert!(
            camera.yaw < 10_f32.to_radians(),
            "yaw = {}",
            camera.yaw.to_degrees()
        );

        for _ in 0..600 {
            camera.interpolate(1.0 / 60.0);
        }
        let error = wrap_pi(camera.dest_yaw - camera.yaw).to_degrees();
        assert!(error.abs() < 1e-3, "residual yaw error {error}");
    }

    #[test]
    fn diagonal_target_moves_glide_on_both_axes() {
        let mut camera = Camera::default();
        camera.set_target(100.0, 0.0, 100.0);
        camera.interpolate(1.0 / 60.0);
        assert!(camera.target.x > 0.0 && camera.target.x < 100.0);
        assert!((camera.target.x - camera.target.z).abs() < 1e-4);

        for _ in 0..600 {
            camera.interpolate(1.0 / 60.0);
        }
        assert!((camera.target.x - 100.0).abs() < 1e-2);
        assert!((camera.target.z - 100.0).abs() < 1e-2);
    }

    #[test]
    fn pitch_band_is_narrow_until_ctrl_and_the_reached_angle_persists() {
        let mut camera = Camera::default();
        let outdoor = CameraControl::default();
        camera.apply_wheel(
            20.0,
            CameraControl {
                shift: true,
                ..outdoor
            },
        );
        assert!((camera.dest_pitch.to_degrees() - 50.0).abs() < 1e-4);

        let ctrl = CameraControl {
            shift: true,
            ctrl: true,
            ..outdoor
        };
        camera.apply_wheel(20.0, ctrl);
        assert!((camera.dest_pitch.to_degrees() - 25.0).abs() < 1e-4);

        camera.apply_drag(40.0, 0.0, outdoor);
        camera.interpolate(1.0);
        assert!(
            (camera.dest_pitch.to_degrees() - 25.0).abs() < 1e-4,
            "a Ctrl-widened pitch must survive release, got {}",
            camera.dest_pitch.to_degrees()
        );

        camera.apply_wheel(
            1.0,
            CameraControl {
                shift: true,
                ..outdoor
            },
        );
        assert!((camera.dest_pitch.to_degrees() - 50.0).abs() < 1e-4);
    }

    #[test]
    fn wheel_zooms_out_within_the_band_and_ctrl_bypasses_it() {
        let mut camera = Camera::default();
        let outdoor = CameraControl::default();
        camera.apply_wheel(1.0, outdoor);
        assert!((camera.dest_distance - (DEFAULT_DISTANCE + 24.3)).abs() < 1e-3);

        camera.apply_wheel(-100.0, outdoor);
        assert!((camera.dest_distance - OUTDOOR_MIN_DISTANCE).abs() < 1e-3);

        camera.apply_wheel(
            -1.0,
            CameraControl {
                ctrl: true,
                ..outdoor
            },
        );
        assert!(camera.dest_distance < OUTDOOR_MIN_DISTANCE);
    }

    #[test]
    fn indoor_entry_faces_the_fixed_angle_and_rotation_stays_banded() {
        let mut camera = Camera::default();
        camera.dest_yaw = std::f32::consts::PI;
        camera.on_map_enter(true, SavedCameraView::default(), DEFAULT_DISTANCE);
        assert!((camera.yaw.to_degrees() - INDOOR_YAW_DEG).abs() < 1e-4);
        assert!((camera.dest_yaw.to_degrees() - INDOOR_YAW_DEG).abs() < 1e-4);

        let indoor = CameraControl {
            indoor: true,
            ..Default::default()
        };
        camera.apply_drag(500.0, 0.0, indoor);
        assert!((camera.dest_yaw.to_degrees() - (INDOOR_YAW_DEG + 20.0)).abs() < 1e-4);

        camera.apply_drag(500.0, 0.0, CameraControl { gm: true, ..indoor });
        assert!(camera.dest_yaw.to_degrees() > INDOOR_YAW_DEG + 20.0);
    }

    #[test]
    fn map_entry_glides_to_the_default_distance() {
        let mut camera = Camera::default();
        let saved = SavedCameraView {
            yaw: 100_f32.to_radians(),
            pitch: 60_f32.to_radians(),
            distance: 800.0,
        };
        camera.on_map_enter(false, saved, DEFAULT_DISTANCE);
        assert!((camera.distance - 800.0).abs() < 1e-4);
        assert!((camera.dest_distance - DEFAULT_DISTANCE).abs() < 1e-4);
        assert!((camera.pitch.to_degrees() - 60.0).abs() < 1e-4);

        camera.on_map_enter(true, saved, DEFAULT_DISTANCE);
        assert!((camera.pitch.to_degrees() - 55.0).abs() < 1e-4);
    }

    #[test]
    fn leaving_an_indoor_map_restores_the_outdoor_yaw() {
        let mut camera = Camera::default();
        camera.apply_drag(100.0, 0.0, CameraControl::default());
        let outdoor_view = camera.saved_view();
        assert!((outdoor_view.yaw.to_degrees() - 100.0).abs() < 1e-4);

        camera.on_map_enter(true, SavedCameraView::default(), DEFAULT_DISTANCE);
        assert!((camera.dest_yaw.to_degrees() - INDOOR_YAW_DEG).abs() < 1e-4);

        camera.on_map_enter(false, outdoor_view, DEFAULT_DISTANCE);
        assert!((camera.yaw.to_degrees() - 100.0).abs() < 1e-4);
        assert!((camera.dest_yaw.to_degrees() - 100.0).abs() < 1e-4);
    }

    #[test]
    fn star_gazing_zooms_three_times_out_and_raises_the_pitch() {
        let mut camera = Camera::default();
        let control = CameraControl {
            star_gazing: true,
            ..Default::default()
        };
        camera.apply_wheel(1000.0, control);
        assert!(
            (camera.dest_distance - OUTDOOR_MAX_DISTANCE * 3.0).abs() < 1e-3,
            "distance = {}",
            camera.dest_distance
        );
        assert!(camera.dest_pitch.to_degrees() >= 50.0);
    }

    #[test]
    fn eye_never_sinks_below_the_ground_under_it() {
        let mut camera = Camera::default();
        let free = camera.eye().y;
        camera.eye_floor = Some(free - 10.0);
        assert!((camera.eye().y - (free - 10.0)).abs() < 1e-4);
        camera.eye_floor = Some(free + 10.0);
        assert!((camera.eye().y - free).abs() < 1e-4);
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
