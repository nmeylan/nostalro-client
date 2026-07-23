use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::rsw::{RswFile, RswObject};

use super::SoundQueue;

const CHEAP_CULL: f32 = 130.0; // 5 * 26
const RANGE_CULL: f32 = 75.0; // 5 * 15

struct Emitter {
    file: String,
    center: [f32; 3],
    width: f32,
    height: f32,
    range: f32,
    vfactor: f32,
    cycle_s: f32,
    timer_s: f32,
}

#[derive(Default)]
pub struct AmbientSoundScheduler {
    emitters: Vec<Emitter>,
}

impl AmbientSoundScheduler {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_rsw(rsw: &RswFile, gnd: &GndFile) -> Self {
        let scale_factor = gnd.zoom / 10.0;
        let center_x = gnd.width as f32 * gnd.zoom / 2.0;
        let center_z = gnd.height as f32 * gnd.zoom / 2.0;

        let mut emitters = Vec::new();
        for obj in &rsw.objects {
            let RswObject::Sound(s) = obj else { continue };
            if s.file_name.is_empty() {
                continue;
            }
            let cycle_s = if s.cycle > 0.0 { s.cycle } else { 4.0 };
            emitters.push(Emitter {
                file: s.file_name.clone(),
                center: [
                    s.position[0] * scale_factor + center_x,
                    0.0,
                    s.position[2] * scale_factor + center_z,
                ],
                width: s.width as f32,
                height: s.height as f32,
                range: s.range,
                vfactor: s.volume,
                cycle_s,
                // Fire immediately on entering range.
                timer_s: cycle_s,
            });
        }
        AmbientSoundScheduler { emitters }
    }

    pub fn update(&mut self, dt: f32, player_pos: Option<[f32; 2]>, out: &mut SoundQueue) {
        let Some([px, pz]) = player_pos else { return };
        for e in &mut self.emitters {
            e.timer_s += dt;

            let dcx = px - e.center[0];
            let dcz = pz - e.center[2];
            let diagonal = (e.width * e.width + e.height * e.height).sqrt();
            let center_dist = (dcx * dcx + dcz * dcz).sqrt();
            if center_dist - (diagonal + e.range) > CHEAP_CULL {
                continue;
            }

            let (ex, ez) = edge_point(dcx, dcz, e.width, e.height);
            let cx = ex + e.center[0];
            let cz = ez + e.center[2];
            let ddx = px - cx;
            let ddz = pz - cz;
            let edge_dist = (ddx * ddx + ddz * ddz).sqrt();
            if edge_dist - e.range > RANGE_CULL {
                continue;
            }

            if e.timer_s >= e.cycle_s {
                e.timer_s = 0.0;
                out.world_ranged(
                    e.file.clone(),
                    [cx, 0.0, cz],
                    e.range,
                    e.range / 6.0,
                    e.vfactor,
                );
            }
        }
    }
}

/// The point on the sound rectangle's boundary in the direction of the player.
/// `(dx, dz)` = player − center.
fn edge_point(dx: f32, dz: f32, width: f32, height: f32) -> (f32, f32) {
    if dx == 0.0 && dz == 0.0 {
        return (0.0, height / 2.0);
    }
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    let rect_slope = if width != 0.0 {
        height / width
    } else {
        f32::INFINITY
    };
    let p_slope = dz / dx;

    if p_slope.abs() <= rect_slope {
        // Vertical edge.
        let cx = half_w.copysign(dx);
        (cx, p_slope * cx)
    } else {
        // Horizontal edge.
        let cz = half_h.copysign(dz);
        (cz / p_slope, cz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_point_hits_expected_face() {
        // Player far to the +x side of a wide-ish rect → vertical (+x) edge.
        let (x, _z) = edge_point(100.0, 10.0, 40.0, 40.0);
        assert_eq!(x, 20.0);
        // Player far to the +z side → horizontal (+z) edge.
        let (_x, z) = edge_point(10.0, 100.0, 40.0, 40.0);
        assert_eq!(z, 20.0);
        // Player exactly at center → deterministic horizontal edge.
        assert_eq!(edge_point(0.0, 0.0, 40.0, 40.0), (0.0, 20.0));
    }

    #[test]
    fn fires_immediately_then_respects_cycle() {
        let mut sched = AmbientSoundScheduler {
            emitters: vec![Emitter {
                file: "birds.wav".into(),
                center: [0.0, 0.0, 0.0],
                width: 20.0,
                height: 20.0,
                range: 50.0,
                vfactor: 1.0,
                cycle_s: 4.0,
                timer_s: 4.0,
            }],
        };
        let mut q = SoundQueue::new();
        sched.update(0.016, Some([10.0, 10.0]), &mut q);
        assert_eq!(q.pending.len(), 1);
        sched.update(0.016, Some([10.0, 10.0]), &mut q);
        assert_eq!(q.pending.len(), 1); // still within cycle
        sched.update(4.0, Some([10.0, 10.0]), &mut q);
        assert_eq!(q.pending.len(), 2);
    }

    #[test]
    fn culls_when_far_beyond_range() {
        let mut sched = AmbientSoundScheduler {
            emitters: vec![Emitter {
                file: "birds.wav".into(),
                center: [0.0, 0.0, 0.0],
                width: 20.0,
                height: 20.0,
                range: 30.0,
                vfactor: 1.0,
                cycle_s: 4.0,
                timer_s: 4.0,
            }],
        };
        let mut q = SoundQueue::new();
        sched.update(0.016, Some([1000.0, 1000.0]), &mut q);
        assert_eq!(q.pending.len(), 0);
    }
}
