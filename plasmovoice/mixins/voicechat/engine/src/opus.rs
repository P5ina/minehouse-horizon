//! Opus through rusty-opus (pure Rust, in Horizon's lock file): nothing to
//! install on the player's machine. 48 kHz mono, 20 ms frames, as Plasmo
//! Voice sends them.
use rusty_opus::{Application, OpusDecoder, OpusEncoder};

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME: usize = 960;

pub const APPLICATION_VOIP: i32 = 2048;
pub const APPLICATION_AUDIO: i32 = 2049;
pub const APPLICATION_RESTRICTED_LOWDELAY: i32 = 2051;

/// Always available: the codec is compiled in.
pub fn check() -> Result<(), String> {
    Ok(())
}

fn application(id: i32) -> Application {
    match id {
        APPLICATION_AUDIO => Application::Audio,
        APPLICATION_RESTRICTED_LOWDELAY => Application::RestrictedLowDelay,
        _ => Application::Voip,
    }
}

pub struct Encoder {
    inner: OpusEncoder,
    application: i32,
    bitrate: i32,
    max_bytes: usize,
}

impl Encoder {
    /// `bitrate` in bits a second; Plasmo's -1000 (auto) and -1 (max) leave the codec's choice.
    pub fn new(application_id: i32, max_bytes: usize) -> Result<Self, String> {
        Self::with_bitrate(application_id, max_bytes, -1000)
    }

    pub fn with_bitrate(application_id: i32, max_bytes: usize, bitrate: i32) -> Result<Self, String> {
        let mut inner = OpusEncoder::new(SAMPLE_RATE as i32, 1, application(application_id))
            .map_err(|e| format!("opus encoder: {e:?}"))?;
        if bitrate > 0 {
            inner.bitrate_bps = bitrate;
        }
        Ok(Self { inner, application: application_id, bitrate, max_bytes: max_bytes.max(2) })
    }

    pub fn encode(&mut self, frame: &[i16; FRAME]) -> Result<Vec<u8>, String> {
        let pcm: Vec<f32> = frame.iter().map(|&s| f32::from(s) / 32768.0).collect();
        let mut out = vec![0u8; self.max_bytes];
        let n = self
            .inner
            .encode(&pcm, FRAME, &mut out)
            .map_err(|e| format!("opus encode: {e:?}"))?;
        out.truncate(n);
        Ok(out)
    }

    /// A fresh encoder for the next phrase, as Plasmo resets its own.
    pub fn reset(&mut self) {
        if let Ok(e) = Self::with_bitrate(self.application, self.max_bytes, self.bitrate) {
            *self = e;
        }
    }
}

pub struct Decoder {
    inner: OpusDecoder,
}

impl Decoder {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            inner: OpusDecoder::new(SAMPLE_RATE as i32, 1).map_err(|e| format!("opus decoder: {e:?}"))?,
        })
    }

    /// One frame; `None` is a lost packet (concealed). With `fec`, `packet` is
    /// the next packet and its in-band copy of the lost one is used.
    pub fn decode(&mut self, packet: Option<&[u8]>, fec: bool) -> Vec<i16> {
        let mut out = vec![0f32; FRAME * 6];
        let data = packet.unwrap_or(&[]);
        let n = if fec {
            self.inner.decode_fec(data, FRAME, &mut out)
        } else {
            self.inner.decode(data, FRAME * 6, &mut out)
        }
        .unwrap_or(0);
        out[..n].iter().map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect()
    }

    pub fn reset(&mut self) {
        if let Ok(d) = Self::new() {
            *self = d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_survives_encode_and_decode() {
        let mut enc = Encoder::new(APPLICATION_VOIP, 1024).unwrap();
        let mut dec = Decoder::new().unwrap();
        let mut energy = 0f64;
        for f in 0..10 {
            let mut frame = [0i16; FRAME];
            for (i, s) in frame.iter_mut().enumerate() {
                let t = (f * FRAME + i) as f32 / SAMPLE_RATE as f32;
                *s = ((t * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16;
            }
            let packet = enc.encode(&frame).unwrap();
            assert!(!packet.is_empty() && packet.len() < 1024);
            let pcm = dec.decode(Some(&packet), false);
            assert_eq!(pcm.len(), FRAME);
            if f > 3 {
                energy += pcm.iter().map(|&s| f64::from(s).powi(2)).sum::<f64>();
            }
        }
        assert!(energy > 1e9, "decoded audio is near silent: {energy}");
    }
}
