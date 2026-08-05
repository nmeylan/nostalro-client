use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct Pcm {
    pub samples: Vec<f32>,
    pub channels: u16,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }
}

#[derive(Debug)]
pub struct DecodeError(pub String);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn decode(bytes: &[u8], out_rate: u32) -> Result<Pcm, DecodeError> {
    match decode_probe(bytes.to_vec(), out_rate) {
        Ok(pcm) => Ok(pcm),
        Err(e) => {
            // Some RO wavs carry an mp3 payload the wav demuxer rejects;
            // retry probing the raw data chunk.
            if let Some(payload) = riff_data_chunk(bytes) {
                decode_probe(payload.to_vec(), out_rate).map_err(|_| e)
            } else {
                Err(e)
            }
        }
    }
}

fn decode_probe(bytes: Vec<u8>, out_rate: u32) -> Result<Pcm, DecodeError> {
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| DecodeError(format!("probe: {e}")))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| DecodeError("no track".into()))?;
    let track_id = track.id;
    let in_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| DecodeError("no sample rate".into()))?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| DecodeError(format!("codec: {e}")))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut channels: u16 = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                channels = spec.channels.count() as u16;
                let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                buf.copy_interleaved_ref(decoded);
                samples.extend_from_slice(buf.samples());
            }
            Err(SymError::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    if samples.is_empty() || channels == 0 {
        return Err(DecodeError("no samples decoded".into()));
    }
    if in_rate != out_rate {
        samples = resample(&samples, channels as usize, in_rate, out_rate);
    }
    Ok(Pcm { samples, channels })
}

fn resample(samples: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32> {
    let frames = samples.len() / channels;
    if frames == 0 {
        return Vec::new();
    }
    let out_frames = (frames as u64 * to as u64 / from as u64) as usize;
    let ratio = from as f64 / to as f64;
    let mut out = Vec::with_capacity(out_frames * channels);
    for i in 0..out_frames {
        let src = i as f64 * ratio;
        let i0 = (src as usize).min(frames - 1);
        let i1 = (i0 + 1).min(frames - 1);
        let frac = (src - i0 as f64) as f32;
        for c in 0..channels {
            let a = samples[i0 * channels + c];
            let b = samples[i1 * channels + c];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

fn riff_data_chunk(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let start = pos + 8;
        if id == b"data" {
            return bytes.get(start..(start + size).min(bytes.len()));
        }
        pos = start + size + (size & 1);
    }
    None
}

#[cfg(test)]
pub(crate) fn wav_pcm16(rate: u32, channels: u16, frames: usize) -> Vec<u8> {
    let data_len = frames * channels as usize * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for i in 0..frames * channels as usize {
        out.extend_from_slice(&((i % 128) as i16 * 256).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_and_resamples_wav() {
        let bytes = wav_pcm16(22050, 1, 2205);
        let pcm = decode(&bytes, 44100).unwrap();
        assert_eq!(pcm.channels, 1);
        assert!((pcm.frames() as i64 - 4410).unsigned_abs() < 10);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode(&[0u8; 64], 44100).is_err());
    }
}
