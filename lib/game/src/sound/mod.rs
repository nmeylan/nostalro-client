use std::borrow::Cow;

pub mod ambient;
pub mod bgm_table;
pub mod repeat;
pub mod tables;

pub const DEFAULT_MAX_DIST: f32 = 250.0;
pub const DEFAULT_MIN_DIST: f32 = 40.0;

/// `depth` is the original's dy volume knob in both variants: it only ducks the
/// gain, and 0.0 leaves it alone.
#[derive(Debug, Clone, Copy)]
pub enum SoundSource {
    /// Non-positional: distance never attenuates it.
    Ui {
        depth: f32,
    },
    World {
        pos: [f32; 3],
        depth: f32,
    },
}

#[derive(Debug, Clone)]
pub struct SoundRequest {
    pub name: Cow<'static, str>,
    pub source: SoundSource,
    pub vfactor: f32,
    pub max_dist: f32,
    pub min_dist: f32,
}

#[derive(Default)]
pub struct SoundQueue {
    pub pending: Vec<SoundRequest>,
}

impl SoundQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ui(&mut self, name: impl Into<Cow<'static, str>>) {
        self.push(
            name,
            SoundSource::Ui { depth: 0.0 },
            1.0,
            DEFAULT_MAX_DIST,
            DEFAULT_MIN_DIST,
        );
    }

    pub fn ui_at_depth(&mut self, name: impl Into<Cow<'static, str>>, depth: f32) {
        self.push(
            name,
            SoundSource::Ui { depth },
            1.0,
            DEFAULT_MAX_DIST,
            DEFAULT_MIN_DIST,
        );
    }

    pub fn world(&mut self, name: impl Into<Cow<'static, str>>, pos: [f32; 3]) {
        self.world_at_depth(name, pos, 0.0);
    }

    pub fn world_at_depth(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        pos: [f32; 3],
        depth: f32,
    ) {
        self.push(
            name,
            SoundSource::World { pos, depth },
            1.0,
            DEFAULT_MAX_DIST,
            DEFAULT_MIN_DIST,
        );
    }

    pub fn world_ranged(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        pos: [f32; 3],
        max_dist: f32,
        min_dist: f32,
        vfactor: f32,
    ) {
        self.push(
            name,
            SoundSource::World { pos, depth: 0.0 },
            vfactor,
            max_dist,
            min_dist,
        );
    }

    fn push(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        source: SoundSource,
        vfactor: f32,
        max_dist: f32,
        min_dist: f32,
    ) {
        self.pending.push(SoundRequest {
            name: name.into(),
            source,
            vfactor,
            max_dist,
            min_dist,
        });
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, SoundRequest> {
        self.pending.drain(..)
    }

    /// Drain to `(name, gain)`, dropping silent requests and collapsing repeats
    /// of one wave to its loudest. Mixing N in-phase copies of a sample
    /// multiplies its amplitude by N, which a splash skill would otherwise do
    /// once per victim.
    pub fn drain_resolved(
        &mut self,
        gain_of: impl Fn(&SoundRequest) -> f32,
    ) -> Vec<(Cow<'static, str>, f32)> {
        let mut out: Vec<(Cow<'static, str>, f32)> = Vec::with_capacity(self.pending.len());
        for req in self.pending.drain(..) {
            let gain = gain_of(&req);
            if gain <= 0.0 {
                continue;
            }
            match out.iter_mut().find(|(name, _)| *name == req.name) {
                Some((_, g)) => *g = g.max(gain),
                None => out.push((req.name, gain)),
            }
        }
        out
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_wave_collapses_to_its_loudest_and_silence_is_dropped() {
        let mut q = SoundQueue::new();
        // One splash skill, three victims at different distances.
        q.world("_enemy_hit_normal1.wav", [200.0, 0.0, 0.0]);
        q.world("_enemy_hit_normal1.wav", [40.0, 0.0, 0.0]);
        q.world("_enemy_hit_normal1.wav", [120.0, 0.0, 0.0]);
        q.world("effect\\EF_FrostDiver.wav", [40.0, 0.0, 0.0]);
        q.world("out_of_range.wav", [10_000.0, 0.0, 0.0]);

        let listener = [0.0f32, 0.0, 0.0];
        let resolved = q.drain_resolved(|r| match r.source {
            SoundSource::Ui { .. } => 1.0,
            SoundSource::World { pos, .. } => {
                let d = ((pos[0] - listener[0]).powi(2) + (pos[2] - listener[2]).powi(2)).sqrt();
                if d >= r.max_dist {
                    0.0
                } else if d <= r.min_dist {
                    1.0
                } else {
                    r.min_dist / d
                }
            }
        });

        assert_eq!(resolved.len(), 2, "{resolved:?}");
        let hit = resolved
            .iter()
            .find(|(n, _)| n == "_enemy_hit_normal1.wav")
            .unwrap();
        assert_eq!(hit.1, 1.0, "the nearest victim sets the gain");
        assert!(q.pending.is_empty());
    }
}
