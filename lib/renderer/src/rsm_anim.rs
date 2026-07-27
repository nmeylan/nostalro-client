//! Keyframe evaluation for RSM node animation (animated map props).

use ragnarok_formats::rsm::{RsmFile, RsmNode};

/// The original game advances one animation tick per rendered frame at its 60 Hz
/// cap, so a tick is 1/60 s of animation.
pub const ANIM_TICKS_PER_SECOND: f32 = 60.0;

/// Keyframe units advanced per tick, per unit of `anim_speed`.
const UNITS_PER_TICK: f32 = 100.0;

pub const ANIM_TYPE_STATIC: i32 = 0;
pub const ANIM_TYPE_ONCE: i32 = 1;
pub const ANIM_TYPE_LOOP: i32 = 2;

pub const DEFAULT_ANIM_TYPE: i32 = 0;
pub const DEFAULT_ANIM_SPEED: f32 = 2.0;

/// Whether any node carries a track that actually moves. A model with at most
/// one key per track evaluates to the same matrix at every frame, so it needs no
/// per-frame work even when its RSW object asks for animation.
pub fn model_has_moving_track(rsm: &RsmFile) -> bool {
    rsm.nodes
        .iter()
        .any(|n| n.rot_keyframes.len() > 1 || n.translation_keyframes.len() > 1)
}

/// Advances a model's motion counter by `dt` seconds. Type 1 plays once and
/// holds the last frame; type 2 loops.
pub fn advance_motion(cur: f32, anim_type: i32, anim_speed: f32, anim_len: u32, dt: f32) -> f32 {
    let step = anim_speed * UNITS_PER_TICK * dt * ANIM_TICKS_PER_SECOND;
    let len = if anim_len == 0 { 1.0 } else { anim_len as f32 };

    match anim_type {
        ANIM_TYPE_ONCE => {
            let next = cur + step;
            if next >= len { cur } else { next }
        }
        ANIM_TYPE_LOOP => {
            let next = cur + step;
            if next >= len {
                next.rem_euclid(len)
            } else {
                next
            }
        }
        _ => cur,
    }
}

/// A node's local transform at `frame`: translation * rotation * scale.
pub fn node_local_matrix(node: &RsmNode, frame: f32) -> glam::Mat4 {
    translation_matrix(node, frame) * rotation_matrix(node, frame) * scale_matrix(node, frame)
}

/// Locates the pair of keys bracketing `frame` and the blend factor between
/// them. Clamps at both ends: before the first key and past the last key the
/// nearest key is held rather than extrapolated.
fn find_span<T>(keys: &[T], frame_of: impl Fn(&T) -> i32, frame: f32) -> (usize, usize, f32) {
    for i in 1..keys.len() {
        let next_frame = frame_of(&keys[i]) as f32;
        if next_frame > frame {
            let prev_frame = frame_of(&keys[i - 1]) as f32;
            let span = next_frame - prev_frame;
            let t = if span > 0.0 {
                ((frame - prev_frame) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            return (i - 1, i, t);
        }
    }
    let last = keys.len() - 1;
    (last, last, 0.0)
}

fn vec3(v: &ragnarok_formats::Vec3) -> glam::Vec3 {
    glam::Vec3::new(v[0], v[1], v[2])
}

fn translation_matrix(node: &RsmNode, frame: f32) -> glam::Mat4 {
    let keys = &node.translation_keyframes;
    match keys.len() {
        0 => glam::Mat4::from_translation(vec3(&node.translation2)),
        1 => glam::Mat4::from_translation(vec3(&keys[0].position)),
        _ => {
            let (a, b, t) = find_span(keys, |k| k.frame, frame);
            let pos = vec3(&keys[a].position).lerp(vec3(&keys[b].position), t);
            glam::Mat4::from_translation(pos)
        }
    }
}

fn rotation_matrix(node: &RsmNode, frame: f32) -> glam::Mat4 {
    let keys = &node.rot_keyframes;
    match keys.len() {
        0 => match (node.rotation_angle, node.rotation_axis) {
            (Some(angle), Some(axis)) => {
                let axis = vec3(&axis);
                if axis.length_squared() > 0.0 {
                    glam::Mat4::from_axis_angle(axis.normalize(), angle)
                } else {
                    glam::Mat4::IDENTITY
                }
            }
            _ => glam::Mat4::IDENTITY,
        },
        1 => glam::Mat4::from_quat(raw_quat(&keys[0].quaternion)),
        _ => {
            let (a, b, t) = find_span(keys, |k| k.frame, frame);
            if keys[a].quaternion[3] == 1.0 && keys[b].quaternion[3] == 1.0 {
                return glam::Mat4::IDENTITY;
            }
            let qa = unit_quat(&keys[a].quaternion);
            let qb = unit_quat(&keys[b].quaternion);
            glam::Mat4::from_quat(qa.slerp(qb, t))
        }
    }
}

/// Scale tracks are not interpolated — the first key is held for the whole
/// animation.
fn scale_matrix(node: &RsmNode, _frame: f32) -> glam::Mat4 {
    match node.scale_keyframes.first() {
        Some(key) => glam::Mat4::from_scale(vec3(&key.scale)),
        None => match node.scale {
            Some(scale) => glam::Mat4::from_scale(vec3(&scale)),
            None => glam::Mat4::IDENTITY,
        },
    }
}

fn raw_quat(q: &[f32; 4]) -> glam::Quat {
    glam::Quat::from_xyzw(q[0], q[1], q[2], q[3])
}

/// `slerp` requires unit quaternions; a handful of shipped keys are not.
fn unit_quat(q: &[f32; 4]) -> glam::Quat {
    let q = raw_quat(q);
    if q.length_squared() > 1e-12 {
        q.normalize()
    } else {
        glam::Quat::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_formats::rsm::RotKeyframe;

    use super::*;

    fn node_with_rot_keys(keys: Vec<RotKeyframe>) -> RsmNode {
        RsmNode {
            name: "n".into(),
            parent_name: String::new(),
            texture_ids: vec![],
            texture_names: vec![],
            local_transform: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation1: None,
            translation2: [0.0, 0.0, 0.0],
            rotation_angle: None,
            rotation_axis: None,
            scale: None,
            vertices: vec![],
            tex_vertices: vec![],
            faces: vec![],
            scale_keyframes: vec![],
            rot_keyframes: keys,
            translation_keyframes: vec![],
            textures_keyframes: vec![],
        }
    }

    /// Quarter turn about Y at frame 0, half turn at frame 100.
    fn quarter_then_half() -> Vec<RotKeyframe> {
        let q0 = glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let q1 = glam::Quat::from_rotation_y(std::f32::consts::PI);
        vec![
            RotKeyframe {
                frame: 0,
                quaternion: [q0.x, q0.y, q0.z, q0.w],
            },
            RotKeyframe {
                frame: 100,
                quaternion: [q1.x, q1.y, q1.z, q1.w],
            },
        ]
    }

    #[test]
    fn rotation_track_slerps_midframe_and_clamps_past_the_last_key() {
        let node = node_with_rot_keys(quarter_then_half());
        let point = glam::Vec3::new(1.0, 0.0, 0.0);

        let mid = node_local_matrix(&node, 50.0).transform_point3(point);
        let expected = glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 * 1.5) * point;
        assert!(
            (mid - expected).length() < 1e-4,
            "mid={mid:?} expected={expected:?}"
        );

        // Well past the last key: holds the last key instead of extrapolating.
        let past = node_local_matrix(&node, 5000.0).transform_point3(point);
        let last = glam::Quat::from_rotation_y(std::f32::consts::PI) * point;
        assert!((past - last).length() < 1e-4, "past={past:?} last={last:?}");
    }

    #[test]
    fn loop_wraps_and_play_once_holds() {
        let looped = advance_motion(9500.0, ANIM_TYPE_LOOP, 2.0, 9600, 1.0 / 60.0);
        assert!((looped - 100.0).abs() < 1e-3, "looped={looped}");

        let held = advance_motion(9500.0, ANIM_TYPE_ONCE, 2.0, 9600, 1.0 / 60.0);
        assert!((held - 9500.0).abs() < 1e-3, "held={held}");

        let static_ = advance_motion(1234.0, ANIM_TYPE_STATIC, 2.0, 9600, 1.0 / 60.0);
        assert!((static_ - 1234.0).abs() < 1e-6);
    }
}
