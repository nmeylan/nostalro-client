use std::sync::Arc;

use crate::decode::Pcm;

pub const NUM_SFX_VOICES: usize = 30;
pub const BGM_FADE_SECS: f32 = 1.27;

#[derive(Default)]
struct Voice {
    pcm: Option<Arc<Pcm>>,
    cursor: usize,
    gain: f32,
    /// -1 hard left, 0 centre, 1 hard right.
    pan: f32,
    looping: bool,
}

impl Voice {
    fn finished(&self) -> bool {
        match &self.pcm {
            None => true,
            Some(pcm) => !self.looping && self.cursor >= pcm.frames(),
        }
    }
}

pub struct MixerCore {
    voices: [Voice; NUM_SFX_VOICES],
    bgm: Voice,
    // Some(Some(pcm)) = fade out then swap, Some(None) = fade out then stop
    bgm_next: Option<Option<Arc<Pcm>>>,
    bgm_fade: f32,
    fade_step: f32,
    sfx_master: f32,
    bgm_master: f32,
    paused: bool,
    stereo: bool,
}

/// Left/right multipliers for `pan`. Centre stays at unity on both channels so
/// panning never changes how loud an unpanned sound was.
fn channel_gains(pan: f32) -> (f32, f32) {
    let p = pan.clamp(-1.0, 1.0);
    (1.0 - p.max(0.0), 1.0 + p.min(0.0))
}

impl MixerCore {
    pub fn new(out_rate: u32, bgm_master: f32, sfx_master: f32) -> Self {
        Self {
            voices: Default::default(),
            bgm: Voice::default(),
            bgm_next: None,
            bgm_fade: 1.0,
            fade_step: 1.0 / (BGM_FADE_SECS * out_rate.max(1) as f32),
            sfx_master,
            bgm_master,
            paused: false,
            stereo: true,
        }
    }

    pub fn play(&mut self, pcm: Arc<Pcm>, gain: f32, pan: f32) -> bool {
        let Some(voice) = self.voices.iter_mut().find(|v| v.finished()) else {
            return false;
        };
        *voice = Voice {
            pcm: Some(pcm),
            cursor: 0,
            gain,
            pan,
            looping: false,
        };
        true
    }

    pub fn play_bgm(&mut self, pcm: Arc<Pcm>) {
        if self.bgm.pcm.is_none() && self.bgm_next.is_none() {
            self.bgm = Voice {
                pcm: Some(pcm),
                cursor: 0,
                gain: 1.0,
                pan: 0.0,
                looping: true,
            };
            self.bgm_fade = 1.0;
        } else {
            self.bgm_next = Some(Some(pcm));
        }
    }

    pub fn stop_bgm(&mut self) {
        if self.bgm.pcm.is_some() {
            self.bgm_next = Some(None);
        }
    }

    pub fn stop_all_sfx(&mut self) {
        for v in &mut self.voices {
            *v = Voice::default();
        }
    }

    pub fn set_masters(&mut self, bgm: f32, sfx: f32) {
        self.bgm_master = bgm;
        self.sfx_master = sfx;
    }

    /// Off collapses every voice to centre, keeping distance attenuation.
    pub fn set_stereo(&mut self, stereo: bool) {
        self.stereo = stereo;
    }

    /// Silences output and freezes the BGM cursor. One-shot SFX are dropped
    /// rather than resumed — a hit sound restarting minutes later is wrong.
    pub fn set_paused(&mut self, paused: bool) {
        if paused && !self.paused {
            self.stop_all_sfx();
        }
        self.paused = paused;
    }

    pub fn render(&mut self, out: &mut [f32], out_channels: usize) {
        out.fill(0.0);
        if out_channels == 0 || self.paused {
            return;
        }
        let frames = out.len() / out_channels;
        for f in 0..frames {
            let base = f * out_channels;
            for v in &mut self.voices {
                let Some(pcm) = &v.pcm else { continue };
                if v.cursor >= pcm.frames() {
                    v.pcm = None;
                    continue;
                }
                let ch = pcm.channels as usize;
                let g = v.gain * self.sfx_master;
                let (lg, rg) = if self.stereo && out_channels >= 2 {
                    channel_gains(v.pan)
                } else {
                    (1.0, 1.0)
                };
                for c in 0..out_channels {
                    let s = pcm.samples[v.cursor * ch + c.min(ch - 1)] * g;
                    out[base + c] += match c {
                        0 => s * lg,
                        1 => s * rg,
                        _ => s,
                    };
                }
                v.cursor += 1;
            }
            self.render_bgm_frame(&mut out[base..base + out_channels]);
        }
        for s in out.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
    }

    fn render_bgm_frame(&mut self, out: &mut [f32]) {
        if self.bgm_next.is_some() {
            self.bgm_fade -= self.fade_step;
            if self.bgm_fade <= 0.0 || self.bgm.pcm.is_none() {
                let next = self.bgm_next.take().unwrap();
                self.bgm = Voice {
                    pcm: next,
                    cursor: 0,
                    gain: 1.0,
                    pan: 0.0,
                    looping: true,
                };
                self.bgm_fade = 1.0;
            }
        }
        let Some(pcm) = &self.bgm.pcm else { return };
        let frames = pcm.frames();
        if frames == 0 {
            return;
        }
        let ch = pcm.channels as usize;
        let g = self.bgm_master * self.bgm_fade;
        for (c, s) in out.iter_mut().enumerate() {
            *s += pcm.samples[self.bgm.cursor * ch + c.min(ch - 1)] * g;
        }
        self.bgm.cursor = (self.bgm.cursor + 1) % frames;
    }

    #[cfg(test)]
    pub(crate) fn active_sfx_voices(&self) -> usize {
        self.voices.iter().filter(|v| !v.finished()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(frames: usize) -> Arc<Pcm> {
        Arc::new(Pcm {
            samples: vec![0.5; frames],
            channels: 1,
        })
    }

    #[test]
    fn voice_pool_drops_beyond_capacity() {
        let mut m = MixerCore::new(1000, 1.0, 1.0);
        for _ in 0..NUM_SFX_VOICES {
            assert!(m.play(pcm(100), 1.0, 0.0));
        }
        assert!(!m.play(pcm(100), 1.0, 0.0));
        assert_eq!(m.active_sfx_voices(), NUM_SFX_VOICES);
    }

    #[test]
    fn stop_all_sfx_clears_voices() {
        let mut m = MixerCore::new(1000, 1.0, 1.0);
        for _ in 0..NUM_SFX_VOICES {
            assert!(m.play(pcm(1000), 1.0, 0.0));
        }
        assert_eq!(m.active_sfx_voices(), NUM_SFX_VOICES);
        m.stop_all_sfx();
        assert_eq!(m.active_sfx_voices(), 0);
    }

    #[test]
    fn finished_voice_is_reused() {
        let mut m = MixerCore::new(1000, 1.0, 1.0);
        for _ in 0..NUM_SFX_VOICES - 1 {
            assert!(m.play(pcm(1000), 1.0, 0.0));
        }
        assert!(m.play(pcm(10), 1.0, 0.0));
        assert!(!m.play(pcm(100), 1.0, 0.0));
        let mut out = vec![0.0; 20 * 2];
        m.render(&mut out, 2);
        assert!(m.play(pcm(100), 1.0, 0.0));
    }

    #[test]
    fn bgm_fade_swaps_track() {
        let rate = 1000;
        let mut m = MixerCore::new(rate, 1.0, 1.0);
        let a = pcm(50);
        let b = Arc::new(Pcm {
            samples: vec![-0.5; 50],
            channels: 1,
        });
        m.play_bgm(a);
        let mut out = vec![0.0; 10];
        m.render(&mut out, 1);
        assert!(out[0] > 0.0);
        m.play_bgm(b);
        let fade_frames = (BGM_FADE_SECS * rate as f32) as usize + 10;
        let mut out = vec![0.0; fade_frames];
        m.render(&mut out, 1);
        let mut out = vec![0.0; 10];
        m.render(&mut out, 1);
        assert!(out[0] < 0.0);
    }

    #[test]
    fn stop_bgm_fades_to_silence() {
        let rate = 1000;
        let mut m = MixerCore::new(rate, 1.0, 1.0);
        m.play_bgm(pcm(50));
        m.stop_bgm();
        let fade_frames = (BGM_FADE_SECS * rate as f32) as usize + 10;
        let mut out = vec![0.0; fade_frames];
        m.render(&mut out, 1);
        let mut out = vec![1.0; 10];
        m.render(&mut out, 1);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn bgm_loops() {
        let mut m = MixerCore::new(1000, 1.0, 1.0);
        m.play_bgm(pcm(10));
        let mut out = vec![0.0; 35];
        m.render(&mut out, 1);
        assert!(out[34] > 0.0);
    }

    #[test]
    fn mono_sfx_mixed_into_both_channels() {
        let mut m = MixerCore::new(1000, 1.0, 1.0);
        m.play(pcm(10), 1.0, 0.0);
        let mut out = vec![0.0; 8];
        m.render(&mut out, 2);
        assert_eq!(out[0], 0.5);
        assert_eq!(out[1], 0.5);
    }
}
