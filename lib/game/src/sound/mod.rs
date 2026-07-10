use std::borrow::Cow;

pub mod ambient;
pub mod bgm_table;
pub mod repeat;
pub mod tables;

pub const DEFAULT_MAX_DIST: f32 = 250.0;
pub const DEFAULT_MIN_DIST: f32 = 40.0;

#[derive(Debug, Clone, Copy)]
pub enum SoundSource {
    /// Non-positional; `depth` is the original's dy volume knob (0.0 = full volume).
    Ui { depth: f32 },
    World([f32; 3]),
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
        self.push(name, SoundSource::Ui { depth: 0.0 }, 1.0, DEFAULT_MAX_DIST, DEFAULT_MIN_DIST);
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
        self.push(
            name,
            SoundSource::World(pos),
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
        self.push(name, SoundSource::World(pos), vfactor, max_dist, min_dist);
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
}
