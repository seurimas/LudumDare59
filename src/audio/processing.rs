use std::io::Cursor;
use std::sync::Arc;

use rodio::Source;

use super::params::{ReverbParams, SoundParams};
use super::playback::ProcessedAudio;

/// Decode raw audio bytes and apply all effects specified by `params`.
/// Returns a `ProcessedAudio` ready to be added to the bevy asset store.
pub fn process_audio(bytes: &Arc<[u8]>, params: &SoundParams) -> ProcessedAudio {
    let decoder = rodio::Decoder::new(Cursor::new(bytes.clone())).expect("valid audio bytes");
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();

    let all: Vec<f32> = decoder.convert_samples().collect();

    // Skip + volume
    let skip = ms_to_sample_count(params.skip_ms, sample_rate, channels);
    let mut samples: Vec<f32> = all[skip.min(all.len())..]
        .iter()
        .map(|&s| s * params.volume)
        .collect();

    // Fade-in
    if params.fade_in_ms > 0 {
        let n = ms_to_sample_count(params.fade_in_ms, sample_rate, channels);
        for i in 0..n.min(samples.len()) {
            samples[i] *= i as f32 / n as f32;
        }
    }

    // Duration normalisation
    if params.duration_ms > 0 {
        let target = ms_to_sample_count(params.duration_ms, sample_rate, channels);
        let fade_len = ms_to_sample_count(100, sample_rate, channels); // 100ms fade-out

        if samples.len() >= target {
            // Longer sample: truncate with 100ms fade-out at the cut point.
            samples.truncate(target);
            let fade_start = target.saturating_sub(fade_len);
            for i in fade_start..target {
                let t = (i - fade_start) as f32 / fade_len as f32;
                samples[i] *= 1.0 - t;
            }
        } else if !params.conversational {
            // Shorter sample: echo (repeat) until duration is satisfied.
            // Spillover beyond `target` on the last echo is allowed.
            let original = samples.clone();
            while samples.len() < target {
                samples.extend_from_slice(&original);
            }

            // Continuous exponential decay envelope so volume always trends downward,
            // regardless of the sample's internal amplitude shape.
            // After each repetition-length period, amplitude is multiplied by echo_decay once.
            let period = original.len() as f32;
            let decay_per_sample = params.echo_decay.powf(1.0 / period);
            let mut env = 1.0f32;
            for s in samples.iter_mut() {
                *s *= env;
                env *= decay_per_sample;
            }
        }
        // conversational + shorter than target: play as-is (no echo, no padding)
    }

    // Reverb
    if let Some(reverb) = &params.selected_reverb {
        apply_reverb(&mut samples, reverb, sample_rate, channels);
    }

    // Delay (silence before the sound)
    if params.delay_ms > 0 {
        let n = ms_to_sample_count(params.delay_ms, sample_rate, channels);
        let mut delayed = vec![0.0f32; n];
        delayed.extend_from_slice(&samples);
        samples = delayed;
    }

    ProcessedAudio {
        samples: samples.into(),
        channels,
        sample_rate,
    }
}

fn ms_to_sample_count(ms: u64, sample_rate: u32, channels: u16) -> usize {
    (ms as f32 / 1000.0 * sample_rate as f32) as usize * channels as usize
}

/// Apply a feedback comb-filter reverb in-place.
fn apply_reverb(samples: &mut [f32], params: &ReverbParams, sample_rate: u32, channels: u16) {
    let delay_frames = ((params.room_size * sample_rate as f32) as usize).max(1);
    let buffer_len = delay_frames * channels as usize;
    let mut buffer = vec![0.0f32; buffer_len];
    let mut pos = 0usize;

    for s in samples.iter_mut() {
        let input = *s;
        let delayed = buffer[pos];
        let feedback = input + params.decay * delayed;
        buffer[pos] = feedback;
        pos = (pos + 1) % buffer_len;
        *s = input + params.decay * delayed * params.wet;
    }
}
