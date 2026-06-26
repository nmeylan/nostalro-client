use crate::App;
use crate::sprite::cart::direction_offset;
use ragnarok_formats::act::{MotionType, SpriteAnimationState};
use ragnarok_game::entity::EntityState;
use ragnarok_game::movement::direction_from_delta;
use ragnarok_game::sprite_loader;
use ragnarok_game::sprite_path::falcon_sprite_path;
use ragnarok_renderer::{EntitySprite, build_entity_sprite};
use std::rc::Rc;

/// Falcon sprite action groups (`매`, 2 actions × 8 directions). The bird is
/// always in flight, so action 0 is the flapping hover/travel loop; action 1 is
/// the dart/dive played when it attacks a skill target.
const FALCON_ACTION_HOVER: usize = 0;
const FALCON_ACTION_FLY: usize = 0;
const FALCON_ACTION_ATTACK: usize = 1;

/// How far behind the owner (in cells, along the opposite of its facing) the
/// falcon hovers at rest — the original game parks the bird off the back of the
/// character.
const FALCON_ORBIT_DISTANCE: f32 = 1.5;

/// Full wing-flap cycle duration, in ms. The falcon's ACT authors a *different*
/// delay per directional slot, so relying on the ACT timing makes the flap speed
/// change with the camera angle; pinning the whole cycle to a fixed duration
/// keeps it steady (and brisk) from every angle.
const FALCON_FLAP_MS: f32 = 500.0;

/// How high above the ground the falcon hovers, in world units. Native RO
/// coordinates put up at −Y, so this is subtracted from the terrain height.
/// Matches the original game's bird (`GetHeight - 25`).
const FALCON_HOVER_HEIGHT: f32 = 25.0;

/// Exponential follow rate (1/s) used to ease the falcon toward its orbit point.
/// Chosen so the bird lags then catches up like the original's proportional
/// per-frame step.
const FALCON_FOLLOW_RATE: f32 = 3.0;

/// Out-flight duration when darting to a skill target, in seconds (the original
/// runs ~25 frames at its fixed step). When it elapses the bird reverts to follow
/// mode, which eases it back home. Also used to delay the skill's hit so the
/// damage lands when the falcon reaches the target, not before.
pub(crate) const FALCON_FLIGHT_OUT_SECS: f32 = 0.42;

/// Extra height the falcon gains at the apex of its attack dart, in world units.
const FALCON_FLIGHT_ARC: f32 = 14.0;

/// Planar distance (world units) beyond which the falcon snaps to its orbit
/// instead of easing — handles the owner teleporting or a map change.
const FALCON_SNAP_DISTANCE: f32 = 60.0;

/// An in-progress dart to a skill target (Blitz Beat / Falcon Assault / Detecting).
/// The bird flies from where it was to `target` over [`FALCON_FLIGHT_OUT_SECS`]
/// with a vertical arc, then the flight clears and follow easing returns it.
pub struct FalconFlight {
    start: [f32; 3],
    target: [f32; 3],
    elapsed: f32,
}

/// The falcon's world position, facing, and flight state — the pure motion law,
/// independent of any GPU sprite so it can be advanced and tested on its own.
pub struct FalconMotion {
    pub pos: [f32; 3],
    pub direction: u8,
    pub flight: Option<FalconFlight>,
}

impl FalconMotion {
    pub fn new(pos: [f32; 3], direction: u8) -> Self {
        Self { pos, direction, flight: None }
    }

    /// Begins a dart toward `target` from the current position.
    pub fn start_flight(&mut self, target: [f32; 3]) {
        self.flight = Some(FalconFlight { start: self.pos, target, elapsed: 0.0 });
    }

    /// Advances the falcon one frame and returns the sprite (action group, motion)
    /// to play. In follow mode it eases toward `orbit` (snapping when the owner
    /// teleported); while darting it flies out on an arc, then reverts to follow.
    pub fn advance(
        &mut self,
        delta: f32,
        orbit: Option<[f32; 3]>,
        owner_moving: bool,
    ) -> (usize, MotionType) {
        let prev = self.pos;
        let result = match &mut self.flight {
            Some(flight) => {
                flight.elapsed += delta;
                let t = (flight.elapsed / FALCON_FLIGHT_OUT_SECS).clamp(0.0, 1.0);
                let lerp = |a: f32, b: f32| a + (b - a) * t;
                self.pos = [
                    lerp(flight.start[0], flight.target[0]),
                    lerp(flight.start[1], flight.target[1])
                        - FALCON_FLIGHT_ARC * (std::f32::consts::PI * t).sin(),
                    lerp(flight.start[2], flight.target[2]),
                ];
                if flight.elapsed >= FALCON_FLIGHT_OUT_SECS {
                    self.flight = None;
                }
                (FALCON_ACTION_ATTACK, MotionType::Loop)
            }
            None => {
                if let Some(target) = orbit {
                    let far = (target[0] - self.pos[0]).powi(2)
                        + (target[2] - self.pos[2]).powi(2)
                        > FALCON_SNAP_DISTANCE * FALCON_SNAP_DISTANCE;
                    if far {
                        // Owner teleported / changed map: snap home rather than
                        // streaking the bird across the world.
                        self.pos = target;
                    } else {
                        let k = 1.0 - (-FALCON_FOLLOW_RATE * delta).exp();
                        for i in 0..3 {
                            self.pos[i] += (target[i] - self.pos[i]) * k;
                        }
                    }
                }
                if owner_moving {
                    (FALCON_ACTION_FLY, MotionType::Loop)
                } else {
                    (FALCON_ACTION_HOVER, MotionType::Loop)
                }
            }
        };
        if let Some(dir) = direction_from_delta(self.pos[0] - prev[0], self.pos[2] - prev[2]) {
            self.direction = dir;
        }
        result
    }
}

/// The hovering falcon companion of a hunter/sniper. Holds its own sprite and
/// animation alongside the independent [`FalconMotion`].
pub struct FalconVisual {
    pub sprite: Rc<EntitySprite>,
    pub animation: SpriteAnimationState,
    pub motion: FalconMotion,
}

impl App {
    /// World position of the falcon's resting orbit point for `owner_gid`: just
    /// behind the owner's facing, hovering above the terrain. `None` until the
    /// map coordinates are known.
    fn falcon_orbit_target(&self, owner_gid: u32) -> Option<[f32; 3]> {
        let coords = self.game.map_coords.as_ref()?;
        let entity = self.game.entities.get(owner_gid)?;
        let (px, py) = entity.movement.position();
        let (ox, oy) = direction_offset(entity.direction);
        let (cx, cy) = (px - ox * FALCON_ORBIT_DISTANCE, py - oy * FALCON_ORBIT_DISTANCE);
        let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
        let ground = self
            .game
            .gat
            .as_ref()
            .map_or(0.0, |gat| gat.get_height(cx + 0.5, cy + 0.5));
        Some([wx, ground - FALCON_HOVER_HEIGHT, wz])
    }

    /// Loads (or replaces) the falcon companion for `owner_gid`. A failed sprite
    /// load is non-fatal — the falcon simply renders nothing until the resource
    /// is available.
    pub(crate) fn spawn_falcon_visual(&mut self, owner_gid: u32) {
        if self.game.falcons.contains_key(&owner_gid) {
            return;
        }
        let job = self.game.entities.get(owner_gid).map(|e| e.job).unwrap_or(0);
        let (grf, renderer) = match (&self.grf, &self.renderer) {
            (Some(g), Some(r)) => (g, r),
            _ => return,
        };
        let base = falcon_sprite_path(job);
        let body = match sprite_loader::load_sprite_data(
            grf,
            &format!("{base}.spr"),
            &format!("{base}.act"),
        ) {
            Some(d) => d,
            None => {
                tracing::warn!("falcon sprite not found: {base}");
                return;
            }
        };
        let sprite = Rc::new(build_entity_sprite(
            &renderer.device.device,
            &renderer.device.queue,
            &renderer.texture_cache.bind_group_layout,
            body,
            None, None, None, None, None, None, None, None,
        ));
        let direction = self
            .game
            .entities
            .get(owner_gid)
            .map(|e| e.direction)
            .unwrap_or(0);
        let pos = self
            .falcon_orbit_target(owner_gid)
            .or_else(|| self.entity_world_pos(owner_gid))
            .unwrap_or([0.0; 3]);
        self.game.falcons.insert(
            owner_gid,
            FalconVisual {
                sprite,
                animation: SpriteAnimationState::new(direction),
                motion: FalconMotion::new(pos, direction),
            },
        );
    }

    pub(crate) fn despawn_falcon_visual(&mut self, owner_gid: u32) {
        self.game.falcons.remove(&owner_gid);
    }

    /// Sends `owner_gid`'s falcon darting toward `target` (a target actor's or
    /// ground cell's world position). Ignored when the owner has no falcon.
    pub(crate) fn start_falcon_flight(&mut self, owner_gid: u32, target: [f32; 3]) {
        if let Some(falcon) = self.game.falcons.get_mut(&owner_gid) {
            falcon.motion.start_flight(target);
        }
    }

    /// Advances every falcon's position and animation.
    pub(crate) fn update_falcon_visuals(&mut self, delta: f32) {
        let camera_dir = self
            .renderer
            .as_ref()
            .map(|r| r.camera.direction_index())
            .unwrap_or(0);
        let owners: Vec<u32> = self.game.falcons.keys().copied().collect();
        for gid in owners {
            if self.game.entities.get(gid).is_none() {
                self.game.falcons.remove(&gid);
                continue;
            }
            let orbit = self.falcon_orbit_target(gid);
            let owner_moving = self
                .game
                .entities
                .get(gid)
                .is_some_and(|e| e.state == EntityState::Moving);
            let Some(falcon) = self.game.falcons.get_mut(&gid) else {
                continue;
            };
            let (action, motion) = falcon.motion.advance(delta, orbit, owner_moving);
            let direction = falcon.motion.direction;
            falcon
                .animation
                .set_action_clamped(action, motion, &falcon.sprite.body_act);
            falcon.animation.set_direction(direction);
            // Pin the flap to a fixed cycle time so it doesn't speed up/slow down
            // as the camera rotates through the sprite's uneven per-direction delays.
            falcon
                .animation
                .set_motion_speed_override(Some(FALCON_FLAP_MS));
            falcon
                .animation
                .update(delta, &falcon.sprite.body_act, camera_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eases_toward_orbit_then_snaps_when_owner_teleports() {
        let orbit = [100.0, 0.0, 100.0];
        let mut m = FalconMotion::new([90.0, 0.0, 100.0], 0);

        // A normal follow step eases partway toward the orbit (not all the way).
        let (action, _) = m.advance(0.1, Some(orbit), false);
        assert_eq!(action, FALCON_ACTION_HOVER);
        assert!(m.pos[0] > 90.0 && m.pos[0] < 100.0, "eased partway: {}", m.pos[0]);

        // Walking owner plays the flapping flight action.
        let (action, _) = m.advance(0.1, Some(orbit), true);
        assert_eq!(action, FALCON_ACTION_FLY);

        // A huge jump (owner teleported) snaps the bird home in one frame.
        let far_orbit = [1000.0, 0.0, 1000.0];
        m.advance(0.016, Some(far_orbit), false);
        assert_eq!(m.pos, far_orbit);
    }

    #[test]
    fn flight_darts_to_target_with_arc_then_reverts_to_follow() {
        let mut m = FalconMotion::new([0.0, 0.0, 0.0], 0);
        let target = [40.0, 0.0, 0.0];
        m.start_flight(target);

        // Mid-flight: heading toward the target and lifted above the flight line.
        let (action, _) = m.advance(FALCON_FLIGHT_OUT_SECS * 0.5, None, false);
        assert_eq!(action, FALCON_ACTION_ATTACK);
        assert!(m.pos[0] > 0.0 && m.pos[0] < 40.0, "advancing: {}", m.pos[0]);
        assert!(m.pos[1] < 0.0, "arced upward (−Y is up): {}", m.pos[1]);
        assert!(m.flight.is_some());

        // The frame that finishes the out-flight reaches the target and clears
        // the flight; the next frame is back in follow/hover.
        m.advance(FALCON_FLIGHT_OUT_SECS, None, false);
        assert!(m.flight.is_none());
        assert!((m.pos[0] - 40.0).abs() < 0.01, "landed on target: {}", m.pos[0]);
        let (action, _) = m.advance(0.016, None, false);
        assert_eq!(action, FALCON_ACTION_HOVER);
    }
}
