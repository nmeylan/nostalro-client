use ragnarok_audio::{attenuate, pan};
use ragnarok_game::sound::SoundSource;
use ragnarok_renderer::SfxPos;

use crate::App;

/// Cheap xorshift for sound randomization (hit-sound variants, gates).
pub(crate) fn next_rand(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

impl App {
    pub(crate) fn next_sfx_rand(&mut self) -> u32 {
        next_rand(&mut self.sfx_rng)
    }

    /// Handle a server `ZC_SOUND`, dispatched on the `act` field: `Play` plays
    /// once, `Repeat` plays once and re-fires every `term_ms`, `Stop` cancels a
    /// repeat by name. Positional at the actor `gid` when resolvable.
    pub(crate) fn handle_sound_effect(&mut self, name: String, act: u8, term_ms: u32, gid: u32) {
        const SOUND_ACT_REPEAT: u8 = 1;
        const SOUND_ACT_STOP: u8 = 2;
        if act == SOUND_ACT_STOP {
            self.game.schedulers.repeat_sounds.stop(&name);
            return;
        }
        match self.entity_world_pos(gid) {
            Some(pos) => self.sound_queue.world(name.clone(), pos),
            None => self.sound_queue.ui(name.clone()),
        }
        if act == SOUND_ACT_REPEAT && term_ms > 0 {
            self.game.schedulers.repeat_sounds.start(name, gid, term_ms);
        }
    }

    /// Play a BGM track (bare filename, e.g. `01.mp3`) from the loose-file BGM
    /// folder. The cache key is the track name, so re-entering a map with the
    /// same track is a no-op (deliberate deviation from the original).
    pub(crate) fn play_bgm_track(&mut self, track: &str) {
        let disk_path = format!("{}/{}", self.config.bgm_path, track);
        let grf = self.grf.as_ref();
        let grf_names = [format!("bgm\\{track}"), format!("data/wav/bgm/{track}")];
        self.sound.play_bgm(track, || {
            if let Ok(bytes) = std::fs::read(&disk_path) {
                return Some(bytes);
            }
            if let Some(g) = grf {
                for name in &grf_names {
                    if let Ok(bytes) = g.read_file(name) {
                        return Some(bytes);
                    }
                }
            }
            None
        });
    }

    /// Player world position — the audio listener (not the camera).
    fn listener_pos(&self) -> Option<[f32; 3]> {
        let gat = self.game.session.gat.as_ref()?;
        let coords = self.game.session.map_coords.as_ref()?;
        let pid = self.game.world.entities.player_id()?;
        let (cx, cy) = self.game.world.entities.get(pid)?.movement.position();
        let (wx, _, wz) = coords.cell_to_world(cx + 0.5, cy + 0.5);
        Some([wx, gat.get_height(cx + 0.5, cy + 0.5), wz])
    }

    /// The original game always pauses on focus loss; `custom.sound.play_when_unfocused`
    /// opts out of that.
    pub(crate) fn apply_sound_pause(&mut self) {
        let paused = !self.window_focused && !self.config.custom.sound.play_when_unfocused;
        self.sound.set_paused(paused);
    }

    /// Screen-right axis in world XZ. Every world source is panned by its
    /// projection onto it, so rotating the camera re-pans without moving any
    /// source. Falls back to +X before the renderer exists.
    fn listener_right(&self) -> [f32; 2] {
        match self.renderer.as_ref() {
            Some(renderer) => {
                let right = renderer.camera.right_vector();
                [right.x, right.z]
            }
            None => [1.0, 0.0],
        }
    }

    /// Resolve queued sound requests to positional gains and hand them to the
    /// mixer. Runs every frame in every scene.
    ///
    /// Requests that name the same wave collapse to the loudest one: a splash
    /// skill queues one identical hit wave per victim in the same frame, and
    /// mixing N copies of one sample in phase multiplies its amplitude by N.
    pub(crate) fn drain_sound_queue(&mut self, delta: f32) {
        // Fold effect-emitted sounds (feeds 1 & 2) into the queue.
        for e in self.effect_holder.drain_sfx() {
            match e.pos {
                SfxPos::World => self.sound_queue.world(e.name, e.world_pos),
                SfxPos::WorldAtDepth(depth) => {
                    self.sound_queue.world_at_depth(e.name, e.world_pos, depth)
                }
                SfxPos::Ui(depth) => self.sound_queue.ui_at_depth(e.name, depth),
            }
        }

        let listener = self.listener_pos();
        self.game.schedulers.ambient_sounds.update(
            delta,
            listener.map(|l| [l[0], l[2]]),
            &mut self.sound_queue,
        );
        let right = self.listener_right();
        let resolved = self.sound_queue.drain_resolved(|req| match req.source {
            SoundSource::Ui { depth } => (
                attenuate(0.0, depth, 0.0, req.min_dist, req.max_dist) * req.vfactor,
                0.0,
            ),
            SoundSource::World { pos, depth } => {
                let l = listener.unwrap_or(pos);
                let (dx, dz) = (pos[0] - l[0], pos[2] - l[2]);
                (
                    attenuate(dx, depth, dz, req.min_dist, req.max_dist) * req.vfactor,
                    pan(dx, dz, right[0], right[1]),
                )
            }
        });
        if let Some(grf) = self.grf.as_ref() {
            for r in resolved {
                let path = format!("data/wav/{}", r.name);
                let disk_rel = r.name.replace('\\', "/");
                self.sound.play_sfx(&path, r.gain, r.pan, || {
                    grf.read_file(&path)
                        .ok()
                        .or_else(|| std::fs::read(format!("wav/{disk_rel}")).ok())
                        .or_else(|| std::fs::read(format!("data/wav/{disk_rel}")).ok())
                });
            }
        }
        if let Some(track) = self.sound.take_bgm_retry() {
            self.play_bgm_track(&track);
        }
        self.sound.tick();
    }
}
