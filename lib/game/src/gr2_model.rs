//! Runtime skeleton posing and animation sampling for GR2 models: builds a bind
//! pose from the skeleton hierarchy and evaluates B-spline animation curves into
//! per-bone skinning matrices. See <https://en.wikipedia.org/wiki/Skeletal_animation>.

use std::rc::Rc;

use glam::{Mat3, Mat4, Quat, Vec3};
use ragnarok_formats::gr2::model::{Gr2Curve, Gr2File, Gr2Skeleton, Gr2Transform};

use crate::entity::EntityState;

struct BindTransform {
    position: Vec3,
    rotation: Quat,
    scale_shear: Mat3,
}

impl BindTransform {
    fn from_gr2(t: &Gr2Transform) -> Self {
        BindTransform {
            position: Vec3::from(t.position),
            rotation: Quat::from_xyzw(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3]),
            scale_shear: mat3_from_row_major(&t.scale_shear),
        }
    }

    fn matrix(&self) -> Mat4 {
        local_matrix(self.position, self.rotation, self.scale_shear)
    }
}

/// A GR2 skeleton prepared for posing: parent links, bind-pose transforms, and
/// inverse bind-world matrices for skinning. Root bones are placed relative to
/// the model's initial placement.
pub struct SkeletonPose {
    names: Vec<String>,
    parents: Vec<i32>,
    bind: Vec<BindTransform>,
    inverse_world: Vec<Mat4>,
    root_placement: Mat4,
}

impl SkeletonPose {
    /// Build the pose for `model_index`, using its bound skeleton and initial
    /// placement.
    pub fn from_model(file: &Gr2File, model_index: usize) -> Option<Self> {
        let model = file.models.get(model_index)?;
        let skeleton = file.skeletons.get(model.skeleton_index?)?;
        Some(Self::new(
            skeleton,
            local_matrix(
                Vec3::from(model.initial_placement.position),
                Quat::from_xyzw(
                    model.initial_placement.rotation[0],
                    model.initial_placement.rotation[1],
                    model.initial_placement.rotation[2],
                    model.initial_placement.rotation[3],
                ),
                mat3_from_row_major(&model.initial_placement.scale_shear),
            ),
        ))
    }

    pub fn new(skeleton: &Gr2Skeleton, root_placement: Mat4) -> Self {
        let mut names = Vec::with_capacity(skeleton.bones.len());
        let mut parents = Vec::with_capacity(skeleton.bones.len());
        let mut bind = Vec::with_capacity(skeleton.bones.len());
        let mut inverse_world = Vec::with_capacity(skeleton.bones.len());
        for bone in &skeleton.bones {
            names.push(bone.name.clone());
            parents.push(bone.parent_index);
            bind.push(BindTransform::from_gr2(&bone.transform));
            inverse_world.push(Mat4::from_cols_array(&bone.inverse_world));
        }
        SkeletonPose {
            names,
            parents,
            bind,
            inverse_world,
            root_placement,
        }
    }

    pub fn bone_count(&self) -> usize {
        self.parents.len()
    }

    /// Skinning palette for the bind pose (no animation applied).
    pub fn bind_palette(&self) -> Vec<Mat4> {
        let local: Vec<Mat4> = self.bind.iter().map(BindTransform::matrix).collect();
        self.palette(&local)
    }

    /// `local[i]` is the local transform of bone `i`; returns the skinning
    /// palette `world[i] * inverse_world[i]`.
    fn palette(&self, local: &[Mat4]) -> Vec<Mat4> {
        let mut world = vec![Mat4::IDENTITY; self.parents.len()];
        for i in 0..self.parents.len() {
            let p = self.parents[i];
            world[i] = if p < 0 {
                self.root_placement * local[i]
            } else {
                world[p as usize] * local[i]
            };
        }
        world
            .iter()
            .zip(&self.inverse_world)
            .map(|(w, iw)| *w * *iw)
            .collect()
    }
}

/// An animation clip: per-bone transform curves matched to a skeleton by name.
pub struct AnimationClip {
    pub duration: f32,
    tracks: Vec<Track>,
}

struct Track {
    name: String,
    position: Gr2Curve,
    orientation: Gr2Curve,
    scale_shear: Gr2Curve,
}

impl AnimationClip {
    /// Build the clip for animation `anim_index`, gathering every track across
    /// its track groups.
    pub fn from_gr2(file: &Gr2File, anim_index: usize) -> Option<Self> {
        let anim = file.animations.get(anim_index)?;
        let mut tracks = Vec::new();
        for &tg in &anim.track_group_indices {
            let Some(group) = file.track_groups.get(tg) else {
                continue;
            };
            for t in &group.transform_tracks {
                tracks.push(Track {
                    name: t.name.clone(),
                    position: t.position.clone(),
                    orientation: t.orientation.clone(),
                    scale_shear: t.scale_shear.clone(),
                });
            }
        }
        Some(AnimationClip {
            duration: anim.duration,
            tracks,
        })
    }

    fn track(&self, name: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.name == name)
    }

    /// Sample the skinning palette for `skeleton` at time `t` (seconds). Bones
    /// with a matching track use the sampled transform; the rest keep their bind
    /// local transform.
    pub fn skinning_palette(&self, skeleton: &SkeletonPose, t: f32) -> Vec<Mat4> {
        let local: Vec<Mat4> = (0..skeleton.bone_count())
            .map(|i| {
                let bind = &skeleton.bind[i];
                match self.track(&skeleton.names[i]) {
                    Some(tr) => local_matrix(
                        eval_vec3(&tr.position, t, bind.position),
                        eval_quat(&tr.orientation, t, bind.rotation),
                        eval_mat3(&tr.scale_shear, t, bind.scale_shear),
                    ),
                    None => bind.matrix(),
                }
            })
            .collect();
        skeleton.palette(&local)
    }
}

/// The five GR2 entity animation states. `Stand` lives inside the model file;
/// the rest are separate files under `data/model/3dmob_bone/`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gr2Action {
    Stand,
    Move,
    Attack,
    Dead,
    Damage,
}

impl Gr2Action {
    pub const ALL: [Gr2Action; 5] = [
        Gr2Action::Stand,
        Gr2Action::Move,
        Gr2Action::Attack,
        Gr2Action::Dead,
        Gr2Action::Damage,
    ];

    pub fn index(self) -> usize {
        match self {
            Gr2Action::Stand => 0,
            Gr2Action::Move => 1,
            Gr2Action::Attack => 2,
            Gr2Action::Dead => 3,
            Gr2Action::Damage => 4,
        }
    }

    pub fn from_state(state: EntityState) -> Gr2Action {
        match state {
            EntityState::Moving => Gr2Action::Move,
            EntityState::Attacking | EntityState::SkillExec => Gr2Action::Attack,
            EntityState::Dead => Gr2Action::Dead,
            EntityState::Hurt => Gr2Action::Damage,
            _ => Gr2Action::Stand,
        }
    }

    fn file_suffix(self) -> Option<&'static str> {
        match self {
            Gr2Action::Stand => None,
            Gr2Action::Move => Some("move"),
            Gr2Action::Attack => Some("attack"),
            Gr2Action::Dead => Some("dead"),
            Gr2Action::Damage => Some("damage"),
        }
    }
}

/// GRF path of the animation file for `(bone_type, action)`, or `None` when the
/// animation lives in the model file itself (`Stand`). The caller must handle a
/// missing file: not every bone type ships every action.
pub fn animation_file_path(bone_type: u32, action: Gr2Action) -> Option<String> {
    let suffix = action.file_suffix()?;
    Some(ragnarok_resources::model::mob_animation(bone_type, suffix))
}

pub fn is_gr2_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".gr2")
}

/// GRF path of a name-table `.gr2` entry.
pub fn gr2_model_path(name: &str) -> String {
    ragnarok_resources::model::mob(&name.to_ascii_lowercase())
}

/// Bone type from a model filename's trailing digit (`..._{N}.gr2`).
pub fn bone_type_from_name(name: &str) -> Option<u32> {
    name.to_ascii_lowercase()
        .strip_suffix(".gr2")?
        .rsplit('_')
        .next()?
        .parse()
        .ok()
}

/// World Y-rotation for an entity's 8-direction facing, applied to a GR2 model
/// stood upright by `rotate_x(FRAC_PI_2)`. Direction 0 = North, 2 = West,
/// 4 = South, 6 = East; N/S map opposite the model's raw yaw, E/W do not.
pub fn model_facing_yaw(direction: u8) -> f32 {
    std::f32::consts::PI - direction as f32 * (std::f32::consts::TAU / 8.0)
}

/// The skeleton and clips of one `.gr2`, shared by every entity drawn with it.
pub struct Gr2Asset {
    pub pose: SkeletonPose,
    pub clips: [Option<AnimationClip>; 5],
}

/// Per-entity GR2 animation state: the shared model and the currently playing
/// action.
pub struct Gr2ModelInstance {
    asset: Rc<Gr2Asset>,
    action: usize,
    action_start: f32,
}

impl Gr2ModelInstance {
    pub fn new(asset: Rc<Gr2Asset>) -> Self {
        Gr2ModelInstance {
            asset,
            action: Gr2Action::Stand.index(),
            action_start: 0.0,
        }
    }

    /// Switch to `desired` when it changed, falling back to `Stand` when the
    /// model has no clip for it. Keeps the running clip's phase otherwise.
    pub fn set_action(&mut self, desired: Gr2Action, now: f32) {
        let idx = if self.asset.clips[desired.index()].is_some() {
            desired.index()
        } else {
            Gr2Action::Stand.index()
        };
        if idx != self.action {
            self.action = idx;
            self.action_start = now;
        }
    }

    pub fn action(&self) -> usize {
        self.action
    }

    /// Whether the current (non-looping) action has played through once.
    pub fn action_completed(&self, now: f32) -> bool {
        match &self.asset.clips[self.action] {
            Some(clip) => now - self.action_start >= clip.duration,
            None => true,
        }
    }

    /// Skinning palette at wall-clock time `now`. `Dead` holds its last frame;
    /// every other action loops.
    pub fn skinning_palette(&self, now: f32) -> Vec<Mat4> {
        match &self.asset.clips[self.action] {
            Some(clip) if clip.duration > 0.0 => {
                let mut t = now - self.action_start;
                if self.action == Gr2Action::Dead.index() {
                    t = t.min(clip.duration - 1e-4);
                } else {
                    t %= clip.duration;
                }
                clip.skinning_palette(&self.asset.pose, t)
            }
            Some(clip) => clip.skinning_palette(&self.asset.pose, 0.0),
            None => self.asset.pose.bind_palette(),
        }
    }
}

/// Compose a bone's local transform as translation × (rotation × scale-shear).
fn local_matrix(position: Vec3, rotation: Quat, scale_shear: Mat3) -> Mat4 {
    Mat4::from_translation(position) * Mat4::from_mat3(Mat3::from_quat(rotation) * scale_shear)
}

/// GR2 stores 3×3 matrices row-major; glam expects column-major, so transpose.
fn mat3_from_row_major(m: &[f32; 9]) -> Mat3 {
    Mat3::from_cols_array(m).transpose()
}

/// First `dim` control values of a constant curve, or `default` when empty.
fn eval_scalars(curve: &Gr2Curve, dim: usize, t: f32) -> Option<Vec<f32>> {
    if curve.controls.len() < dim {
        return None;
    }
    if curve.degree < 2 || curve.knots.len() < 3 {
        return Some(curve.controls[..dim].to_vec());
    }
    Some(deboor2(&curve.knots, &curve.controls, dim, t))
}

/// Quadratic (degree-2) B-spline de Boor evaluation over non-uniform knots.
/// See <https://en.wikipedia.org/wiki/B-spline>.
fn deboor2(knots: &[f32], controls: &[f32], dim: usize, t: f32) -> Vec<f32> {
    let n = knots.len() as isize;
    let span = (knots.partition_point(|&k| k <= t) as isize).clamp(2, n - 1);
    let idx = |i: isize| i.clamp(0, n - 1) as usize;
    let ka = knots[idx(span - 2)];
    let kb = knots[idx(span - 1)];
    let kc = knots[idx(span)];
    let kd = knots[idx(span + 1)];
    let a = safe_div(t - kb, kc - kb);
    let b = safe_div(t - ka, kc - ka);
    let c = safe_div(t - kb, kd - kb);
    let ctl = |point: usize, d: usize| controls[point * dim + d];
    let (i0, i1, i2) = (idx(span - 2), idx(span - 1), idx(span));
    (0..dim)
        .map(|d| {
            let e1 = (1.0 - b) * ctl(i0, d) + b * ctl(i1, d);
            let e2 = (1.0 - c) * ctl(i1, d) + c * ctl(i2, d);
            (1.0 - a) * e1 + a * e2
        })
        .collect()
}

fn safe_div(num: f32, den: f32) -> f32 {
    if den.abs() < 1e-12 { 0.0 } else { num / den }
}

fn eval_vec3(curve: &Gr2Curve, t: f32, default: Vec3) -> Vec3 {
    match eval_scalars(curve, 3, t) {
        Some(v) => Vec3::new(v[0], v[1], v[2]),
        None => default,
    }
}

fn eval_quat(curve: &Gr2Curve, t: f32, default: Quat) -> Quat {
    match eval_scalars(curve, 4, t) {
        Some(v) => Quat::from_xyzw(v[0], v[1], v[2], v[3]).normalize(),
        None => default,
    }
}

fn eval_mat3(curve: &Gr2Curve, t: f32, default: Mat3) -> Mat3 {
    match eval_scalars(curve, 9, t) {
        Some(v) => {
            let a: [f32; 9] = v.try_into().unwrap();
            mat3_from_row_major(&a)
        }
        None => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bone_type_and_paths_from_name_table_entries() {
        assert_eq!(bone_type_from_name("Empelium90_0.gr2"), Some(0));
        assert_eq!(bone_type_from_name("Kguardian90_7.gr2"), Some(7));
        assert_eq!(bone_type_from_name("TREASUREBOX_2.gr2"), Some(2));
        assert_eq!(bone_type_from_name("dragon_5.gr2"), Some(5));
        assert_eq!(bone_type_from_name("poring.spr"), None);
        assert!(is_gr2_name("Guildflag90_1.gr2"));
        assert!(!is_gr2_name("몬스터"));
        assert_eq!(
            gr2_model_path("Empelium90_0.gr2"),
            "data/model/3dmob/empelium90_0.gr2"
        );
        assert_eq!(
            animation_file_path(7, Gr2Action::Attack).as_deref(),
            Some("data/model/3dmob_bone/7_attack.gr2")
        );
        assert_eq!(animation_file_path(7, Gr2Action::Stand), None);
    }

    #[test]
    fn action_from_entity_state() {
        assert_eq!(Gr2Action::from_state(EntityState::Moving), Gr2Action::Move);
        assert_eq!(
            Gr2Action::from_state(EntityState::Attacking),
            Gr2Action::Attack
        );
        assert_eq!(
            Gr2Action::from_state(EntityState::SkillExec),
            Gr2Action::Attack
        );
        assert_eq!(Gr2Action::from_state(EntityState::Dead), Gr2Action::Dead);
        assert_eq!(Gr2Action::from_state(EntityState::Hurt), Gr2Action::Damage);
        assert_eq!(
            Gr2Action::from_state(EntityState::Standing),
            Gr2Action::Stand
        );
        assert_eq!(
            Gr2Action::from_state(EntityState::Casting),
            Gr2Action::Stand
        );
    }

    fn empty_pose() -> SkeletonPose {
        SkeletonPose::new(
            &ragnarok_formats::gr2::model::Gr2Skeleton {
                name: String::new(),
                bones: Vec::new(),
            },
            Mat4::IDENTITY,
        )
    }

    fn clip(duration: f32) -> AnimationClip {
        AnimationClip {
            duration,
            tracks: Vec::new(),
        }
    }

    #[test]
    fn facing_yaw_points_model_front_at_the_right_compass_direction() {
        // A model stood upright by rotate_x(90°) has its front along world -Z
        // (south); rotate_y(facing_yaw) must swing it to the entity's facing.
        let front =
            |dir: u8| Mat4::from_rotation_y(model_facing_yaw(dir)).transform_vector3(Vec3::NEG_Z);
        let north = Vec3::Z;
        let south = Vec3::NEG_Z;
        let east = Vec3::X;
        let west = Vec3::NEG_X;
        assert!((front(0) - north).length() < 1e-5);
        assert!((front(2) - west).length() < 1e-5);
        assert!((front(4) - south).length() < 1e-5);
        assert!((front(6) - east).length() < 1e-5);
    }

    #[test]
    fn instance_falls_back_to_stand_and_keeps_phase_on_repeat() {
        let mut inst = Gr2ModelInstance::new(Rc::new(Gr2Asset {
            pose: empty_pose(),
            clips: [Some(clip(2.0)), None, Some(clip(1.0)), None, None],
        }));
        // No move clip: stays on stand.
        inst.set_action(Gr2Action::Move, 5.0);
        assert_eq!(inst.action(), Gr2Action::Stand.index());
        // Switching to attack restarts the clock; repeating it does not.
        inst.set_action(Gr2Action::Attack, 5.0);
        assert_eq!(inst.action(), Gr2Action::Attack.index());
        assert!(!inst.action_completed(5.5));
        inst.set_action(Gr2Action::Attack, 5.9);
        assert!(inst.action_completed(6.0));
    }
}
