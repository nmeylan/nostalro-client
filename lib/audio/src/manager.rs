use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::bgm_retry::BgmRetry;
use crate::decode::{self, Pcm};
use crate::mixer::MixerCore;

const CACHE_EXPIRE: Duration = Duration::from_secs(15);
const SWEEP_INTERVAL: Duration = Duration::from_secs(1);

struct CacheEntry {
    pcm: Arc<Pcm>,
    last_used: Instant,
}

pub struct SoundManager {
    core: Option<Arc<Mutex<MixerCore>>>,
    _stream: Option<cpal::Stream>,
    cache: HashMap<String, CacheEntry>,
    failed: HashSet<String>,
    bgm_retry: BgmRetry,
    current_bgm: Option<String>,
    out_rate: u32,
    last_sweep: Instant,
    dropped_voices: u64,
}

impl SoundManager {
    pub fn new(bgm_volume: f32, sfx_volume: f32) -> Self {
        match init_stream(bgm_volume, sfx_volume) {
            Ok((core, stream, out_rate)) => Self {
                core: Some(core),
                _stream: Some(stream),
                cache: HashMap::new(),
                failed: HashSet::new(),
                bgm_retry: BgmRetry::default(),
                current_bgm: None,
                out_rate,
                last_sweep: Instant::now(),
                dropped_voices: 0,
            },
            Err(e) => {
                tracing::warn!("audio disabled: {e}");
                Self::disabled()
            }
        }
    }

    pub fn disabled() -> Self {
        Self {
            core: None,
            _stream: None,
            cache: HashMap::new(),
            failed: HashSet::new(),
            bgm_retry: BgmRetry::default(),
            current_bgm: None,
            out_rate: 44100,
            last_sweep: Instant::now(),
            dropped_voices: 0,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.core.is_some()
    }

    pub fn play_sfx(
        &mut self,
        key: &str,
        gain: f32,
        pan: f32,
        load: impl FnOnce() -> Option<Vec<u8>>,
    ) {
        let Some(core) = &self.core else { return };
        if self.failed.contains(key) {
            return;
        }
        let now = Instant::now();
        let pcm = if let Some(entry) = self.cache.get_mut(key) {
            entry.last_used = now;
            entry.pcm.clone()
        } else {
            let Some(bytes) = load() else {
                tracing::warn!("sound not found: {key}");
                self.failed.insert(key.to_string());
                return;
            };
            match decode::decode(&bytes, self.out_rate) {
                Ok(pcm) => {
                    let pcm = Arc::new(pcm);
                    self.cache.insert(
                        key.to_string(),
                        CacheEntry {
                            pcm: pcm.clone(),
                            last_used: now,
                        },
                    );
                    pcm
                }
                Err(e) => {
                    tracing::warn!("failed to decode sound {key}: {e}");
                    self.failed.insert(key.to_string());
                    return;
                }
            }
        };
        if !core.lock().unwrap().play(pcm, gain, pan) {
            self.dropped_voices += 1;
            tracing::trace!(
                "sfx voice pool full, dropped {key} ({} total)",
                self.dropped_voices
            );
        }
    }

    pub fn play_bgm(&mut self, key: &str, load: impl FnOnce() -> Option<Vec<u8>>) {
        let Some(core) = &self.core else { return };
        if self.current_bgm.as_deref() == Some(key) {
            return;
        }
        let Some(bytes) = load() else {
            tracing::warn!("bgm not found: {key}");
            self.bgm_retry.schedule(key, Instant::now());
            return;
        };
        match decode::decode(&bytes, self.out_rate) {
            Ok(pcm) => {
                core.lock().unwrap().play_bgm(Arc::new(pcm));
                self.current_bgm = Some(key.to_string());
                self.bgm_retry.clear();
            }
            Err(e) => {
                tracing::warn!("failed to decode bgm {key}: {e}");
                self.bgm_retry.schedule(key, Instant::now());
            }
        }
    }

    /// The track to re-attempt, once its retry interval has elapsed.
    pub fn take_bgm_retry(&mut self) -> Option<String> {
        self.bgm_retry.take_due(Instant::now())
    }

    pub fn stop_bgm(&mut self) {
        if let Some(core) = &self.core {
            core.lock().unwrap().stop_bgm();
        }
        self.current_bgm = None;
        self.bgm_retry.clear();
    }

    pub fn stop_all_sfx(&mut self) {
        if let Some(core) = &self.core {
            core.lock().unwrap().stop_all_sfx();
        }
    }

    pub fn set_stereo(&mut self, stereo: bool) {
        if let Some(core) = &self.core {
            core.lock().unwrap().set_stereo(stereo);
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if let Some(core) = &self.core {
            core.lock().unwrap().set_paused(paused);
        }
    }

    pub fn set_volumes(&mut self, bgm: f32, sfx: f32) {
        if let Some(core) = &self.core {
            core.lock().unwrap().set_masters(bgm, sfx);
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_sweep) >= SWEEP_INTERVAL {
            self.last_sweep = now;
            self.sweep(now);
        }
    }

    fn sweep(&mut self, now: Instant) {
        self.cache
            .retain(|_, entry| now.duration_since(entry.last_used) < CACHE_EXPIRE);
    }
}

fn init_stream(
    bgm_volume: f32,
    sfx_volume: f32,
) -> Result<(Arc<Mutex<MixerCore>>, cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no output device".to_string())?;
    let default = device.default_output_config().map_err(|e| e.to_string())?;
    let config = if default.sample_format() == cpal::SampleFormat::F32 {
        default
    } else {
        device
            .supported_output_configs()
            .map_err(|e| e.to_string())?
            .find(|c| c.sample_format() == cpal::SampleFormat::F32)
            .map(|c| c.with_max_sample_rate())
            .ok_or("no f32 output config".to_string())?
    };
    let out_rate = config.sample_rate().0;
    let out_channels = config.channels() as usize;
    let core = Arc::new(Mutex::new(MixerCore::new(out_rate, bgm_volume, sfx_volume)));
    let cb_core = core.clone();
    let stream = device
        .build_output_stream(
            &config.config(),
            move |data: &mut [f32], _| match cb_core.try_lock() {
                Ok(mut mixer) => mixer.render(data, out_channels),
                Err(_) => data.fill(0.0),
            },
            |e| tracing::warn!("audio stream error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((core, stream, out_rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_sweep_expires_stale_entries_but_arc_stays_alive() {
        let mut m = SoundManager::disabled();
        let pcm = Arc::new(Pcm {
            samples: vec![0.0; 10],
            channels: 1,
        });
        let playing = pcm.clone();
        let now = Instant::now();
        m.cache.insert(
            "old.wav".into(),
            CacheEntry {
                pcm,
                last_used: now - Duration::from_secs(20),
            },
        );
        m.cache.insert(
            "fresh.wav".into(),
            CacheEntry {
                pcm: Arc::new(Pcm {
                    samples: vec![0.0; 10],
                    channels: 1,
                }),
                last_used: now,
            },
        );
        m.sweep(now);
        assert!(!m.cache.contains_key("old.wav"));
        assert!(m.cache.contains_key("fresh.wav"));
        assert_eq!(playing.frames(), 10);
    }

    #[test]
    fn disabled_manager_is_noop() {
        let mut m = SoundManager::disabled();
        assert!(!m.is_enabled());
        m.play_sfx("x.wav", 1.0, 0.0, || Some(vec![0u8; 4]));
        m.play_bgm("01.mp3", || None);
        m.stop_bgm();
        m.set_volumes(0.5, 0.5);
        m.tick();
        assert!(m.cache.is_empty());
    }
}
