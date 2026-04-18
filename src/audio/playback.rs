use std::sync::Arc;

/// Fully pre-processed audio data (f32 PCM samples).
///
/// Not a Bevy asset — kept in `BakedAudioSamples` for mid-game concatenation.
/// Call `samples_to_wav` to obtain bytes suitable for `bevy::audio::AudioSource`.
pub struct ProcessedAudio {
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Encode f32 PCM samples as an in-memory IEEE-float WAV file (format code 3).
///
/// The returned bytes can be passed directly to
/// `bevy::audio::AudioSource { bytes: wav.into() }`.
pub fn samples_to_wav(samples: &[f32], channels: u16, sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 4) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);

    // RIFF header
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // fmt chunk — IEEE float (3), not PCM (1)
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * channels as u32 * 4).to_le_bytes()); // byte rate
    out.extend_from_slice(&(channels * 4).to_le_bytes()); // block align
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per sample

    // data chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }

    out
}
