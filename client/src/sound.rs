use ragnarok_audio::attenuate;
use ragnarok_game::sound::SoundSource;

use crate::App;

impl App {
    /// Cheap xorshift for sound randomization (hit-sound variants, gates).
    pub(crate) fn next_sfx_rand(&mut self) -> u32 {
        self.sfx_rng ^= self.sfx_rng << 13;
        self.sfx_rng ^= self.sfx_rng >> 17;
        self.sfx_rng ^= self.sfx_rng << 5;
        self.sfx_rng
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

    /// Resolve queued sound requests to positional gains and hand them to the
    /// mixer. Runs every frame in every scene.
    pub(crate) fn drain_sound_queue(&mut self, delta: f32) {
        // Fold effect-emitted sounds (feeds 1 & 2) into the queue as world sounds.
        for (name, pos) in self.effect_holder.drain_sfx() {
            self.sound_queue.world(name, pos);
        }

        let listener = self.listener_pos();
        self.game.schedulers.ambient_sounds.update(
            delta,
            listener.map(|l| [l[0], l[2]]),
            &mut self.sound_queue,
        );
        let requests: Vec<_> = self.sound_queue.pending.drain(..).collect();
        if let Some(grf) = self.grf.as_ref() {
            for req in requests {
                let gain = match req.source {
                    SoundSource::Ui { depth } => {
                        attenuate(0.0, depth, 0.0, req.min_dist, req.max_dist) * req.vfactor
                    }
                    SoundSource::World(pos) => {
                        let l = listener.unwrap_or(pos);
                        attenuate(
                            pos[0] - l[0],
                            0.0,
                            pos[2] - l[2],
                            req.min_dist,
                            req.max_dist,
                        ) * req.vfactor
                    }
                };
                let path = format!("data/wav/{}", req.name);
                let disk_rel = req.name.replace('\\', "/");
                self.sound.play_sfx(&path, gain, || {
                    grf.read_file(&path)
                        .ok()
                        .or_else(|| std::fs::read(format!("wav/{disk_rel}")).ok())
                        .or_else(|| std::fs::read(format!("data/wav/{disk_rel}")).ok())
                });
            }
        }
        self.sound.tick();
    }
}
