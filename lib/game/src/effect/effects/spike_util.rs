//! Shared math for QuadHorn "spike" effects (ice/stone blades).
//!
//! The QuadHorn spikes all behave the same way: a blade
//! whose apex direction is set by a pitch (tilt) + yaw
//! (rotation), which rises along that direction for the first
//! few frames and then freezes, holding full alpha until a
//! short linear fade-out tail at the end of its life. FrostDiver, Grimtooth,
//! EarthSpike, IceWall and GrimToothAtk only differ in how the spikes are
//! placed and sized, so the per-spike motion lives here.

pub const FRAMES_PER_SECOND: f32 = 60.0;

/// Apex direction (unit vector) × `speed_per_s`.
///
/// `tilt_x_deg` is the apex pitch — 90° points straight up
/// (native RO `-Y = up`), 100° leans slightly backward. `rotation_y_deg`
/// yaws the tilted blade around the world up-axis.
pub fn apex_velocity(tilt_x_deg: f32, rotation_y_deg: f32, speed_per_s: f32) -> [f32; 3] {
    let yaw = rotation_y_deg.to_radians();
    let tilt = tilt_x_deg.to_radians();
    let (sin_t, cos_t) = tilt.sin_cos();
    let (sin_y, cos_y) = yaw.sin_cos();
    let dir = [cos_t * sin_y, -sin_t, cos_t * cos_y];
    [
        dir[0] * speed_per_s,
        dir[1] * speed_per_s,
        dir[2] * speed_per_s,
    ]
}

/// Advance `base` along `velocity` for `dt`, but only while `age` is inside
/// the speed-limit window `[0, speed_limit_s)`. Once the window closes the
/// spike is frozen in place (the brief rise window then a fixed hold).
pub fn rise_step(base: &mut [f32; 3], velocity: [f32; 3], age: f32, dt: f32, speed_limit_s: f32) {
    let effective_dt = if age >= speed_limit_s {
        0.0
    } else if age + dt > speed_limit_s {
        speed_limit_s - age
    } else {
        dt
    };
    base[0] += velocity[0] * effective_dt;
    base[1] += velocity[1] * effective_dt;
    base[2] += velocity[2] * effective_dt;
}

/// Hold `peak` alpha until `fade_out_frames` before the end of `duration`,
/// then ramp linearly to 0 (fade starts 10 frames before the end).
pub fn fade_tail_alpha(age: f32, duration: f32, peak: f32, fade_out_frames: f32) -> f32 {
    let fade_start_frame = (duration * FRAMES_PER_SECOND - fade_out_frames).max(0.0);
    let fade_start_s = fade_start_frame / FRAMES_PER_SECOND;
    if age <= fade_start_s {
        peak
    } else {
        let t = ((age - fade_start_s) / (duration - fade_start_s)).clamp(0.0, 1.0);
        peak * (1.0 - t)
    }
}
