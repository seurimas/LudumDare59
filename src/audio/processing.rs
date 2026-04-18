use std::io::Cursor;
use std::sync::Arc;

use pitch_shift::{Shifter, TOTAL_F32};
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

    // Pitch scaling (phase-vocoder time-stretch + resample).
    if (params.pitch_scale - 1.0).abs() > f32::EPSILON {
        samples = pitch_scale_interleaved(&samples, channels, sample_rate, params.pitch_scale);
        normalize_peak(&mut samples, 0.9);
    }

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
        normalize_peak(&mut samples, 0.9);
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

fn pitch_scale_interleaved(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    pitch_scale: f32,
) -> Vec<f32> {
    if samples.is_empty() || channels == 0 || pitch_scale <= 0.0 {
        return samples.to_vec();
    }

    if (pitch_scale - 1.0).abs() < 1e-6 {
        return samples.to_vec();
    }

    let channel_count = channels as usize;
    let frame_count = samples.len() / channel_count;
    if frame_count == 0 {
        return samples.to_vec();
    }

    let semitones = 12.0 * pitch_scale.log2();
    let mut channels_out: Vec<Vec<f32>> = Vec::with_capacity(channel_count);

    for ch in 0..channel_count {
        let mut mono = Vec::with_capacity(frame_count);
        for i in 0..frame_count {
            mono.push(samples[i * channel_count + ch]);
        }

        let shifted = shift_mono_streaming(&mono, semitones, sample_rate as f32);
        channels_out.push(shifted);
    }

    let mut out = Vec::with_capacity(frame_count * channel_count);
    for i in 0..frame_count {
        for channel in channels_out.iter().take(channel_count) {
            out.push(channel[i]);
        }
    }

    // Preserve any trailing partial frame, unchanged.
    out.extend_from_slice(&samples[frame_count * channel_count..]);
    out
}

fn shift_mono_streaming(input: &[f32], semitones: f32, sample_rate: f32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    const HOP: usize = 128;
    const ALGO_DELAY: usize = 1024 - HOP;

    let state_vec = vec![0.0f32; TOTAL_F32];
    let state_box: Box<[f32; TOTAL_F32]> = state_vec
        .into_boxed_slice()
        .try_into()
        .expect("state length matches TOTAL_F32");
    let mut shifter = Shifter::new(state_box);
    let mut shifted_stream = Vec::<f32>::with_capacity(input.len() + ALGO_DELAY + HOP * 8);

    let mut i = 0usize;
    while i < input.len() {
        let mut chunk = [0.0f32; HOP];
        let remaining = input.len() - i;
        let take = remaining.min(HOP);
        chunk[..take].copy_from_slice(&input[i..i + take]);
        let out = shifter.shift(&chunk, semitones, sample_rate);
        shifted_stream.extend_from_slice(out);
        i += take;
    }

    // Flush tail so we can compensate the algorithmic delay cleanly.
    let zeros = [0.0f32; HOP];
    for _ in 0..10 {
        let out = shifter.shift(&zeros, semitones, sample_rate);
        shifted_stream.extend_from_slice(out);
    }

    if shifted_stream.len() <= ALGO_DELAY {
        return vec![0.0; input.len()];
    }

    let available = &shifted_stream[ALGO_DELAY..];
    let mut out = available[..available.len().min(input.len())].to_vec();
    if out.len() < input.len() {
        out.resize(input.len(), 0.0);
    }
    out
}

fn normalize_peak(samples: &mut [f32], max_peak: f32) {
    let peak = samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    if peak > max_peak && peak > 1e-9 {
        let gain = max_peak / peak;
        for s in samples {
            *s *= gain;
        }
    }
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

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;
    use std::sync::Arc;

    use super::*;
    use crate::audio::SoundParams;

    #[test]
    fn pitch_scale_default_is_no_op() {
        let sample_rate = 8_000u32;
        let input = sine_wave(220.0, 1.0, sample_rate);
        let wav = wav_bytes_mono_16(sample_rate, &input);

        let out = process_audio(&Arc::from(wav), &SoundParams::default());

        assert_eq!(out.channels, 1);
        assert_eq!(out.sample_rate, sample_rate);
        assert_eq!(out.samples.len(), input.len());
    }

    #[test]
    fn pitch_scale_up_raises_frequency() {
        let sample_rate = 8_000u32;
        let input = sine_wave(220.0, 1.0, sample_rate);
        let wav = wav_bytes_mono_16(sample_rate, &input);

        let params = SoundParams {
            pitch_scale: 2.0,
            ..SoundParams::default()
        };
        let out = process_audio(&Arc::from(wav), &params);
        let measured = estimate_frequency_zero_crossing(&out.samples, sample_rate);

        assert!((measured - 440.0).abs() < 20.0, "measured={measured}");
        assert_eq!(out.samples.len(), input.len());
    }

    #[test]
    fn pitch_scale_down_lowers_frequency() {
        let sample_rate = 8_000u32;
        let input = sine_wave(220.0, 1.0, sample_rate);
        let wav = wav_bytes_mono_16(sample_rate, &input);

        let params = SoundParams {
            pitch_scale: 0.5,
            ..SoundParams::default()
        };
        let out = process_audio(&Arc::from(wav), &params);
        let measured = estimate_frequency_zero_crossing(&out.samples, sample_rate);

        assert!((measured - 110.0).abs() < 12.0, "measured={measured}");
        assert_eq!(out.samples.len(), input.len());
    }

    #[test]
    fn pitch_scaling_respects_duration_normalisation() {
        let sample_rate = 8_000u32;
        let input = sine_wave(220.0, 1.0, sample_rate);
        let wav = wav_bytes_mono_16(sample_rate, &input);

        let params = SoundParams {
            pitch_scale: 1.5,
            duration_ms: 500,
            conversational: true,
            ..SoundParams::default()
        };

        let out = process_audio(&Arc::from(wav), &params);
        assert_eq!(out.samples.len(), (sample_rate as usize) / 2);
    }

    #[test]
    fn pitch_scaling_preserves_stereo_layout() {
        let sample_rate = 8_000u32;
        let left = sine_wave(220.0, 0.6, sample_rate);
        let right = sine_wave(330.0, 0.6, sample_rate);

        let mut interleaved = Vec::with_capacity(left.len() * 2);
        for i in 0..left.len() {
            interleaved.push(left[i]);
            interleaved.push(right[i]);
        }

        let wav = wav_bytes_stereo_16(sample_rate, &interleaved);
        let params = SoundParams {
            pitch_scale: 2.0,
            ..SoundParams::default()
        };
        let out = process_audio(&Arc::from(wav), &params);

        assert_eq!(out.channels, 2);
        assert_eq!(out.samples.len(), interleaved.len());

        let (out_left, out_right) = deinterleave_stereo(&out.samples);
        let f_left = estimate_frequency_zero_crossing(&out_left, sample_rate);
        let f_right = estimate_frequency_zero_crossing(&out_right, sample_rate);

        assert!((f_left - 440.0).abs() < 25.0, "f_left={f_left}");
        assert!((f_right - 660.0).abs() < 35.0, "f_right={f_right}");
    }

    #[test]
    fn pitch_scaling_keeps_peak_under_one() {
        let sample_rate = 8_000u32;
        let input = sine_wave(440.0, 1.0, sample_rate);
        let wav = wav_bytes_mono_16(sample_rate, &input);

        let params = SoundParams {
            volume: 2.0,
            pitch_scale: 0.5,
            ..SoundParams::default()
        };
        let out = process_audio(&Arc::from(wav), &params);
        let peak = out.samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));

        assert!(peak <= 1.0, "peak={peak}");
    }

    fn sine_wave(freq_hz: f32, duration_s: f32, sample_rate: u32) -> Vec<f32> {
        let len = (duration_s * sample_rate as f32) as usize;
        (0..len)
            .map(|n| {
                let t = n as f32 / sample_rate as f32;
                (2.0 * PI * freq_hz * t).sin() * 0.8
            })
            .collect()
    }

    fn estimate_frequency_zero_crossing(samples: &[f32], sample_rate: u32) -> f32 {
        let mut crossings = 0usize;
        for i in 1..samples.len() {
            if samples[i - 1] <= 0.0 && samples[i] > 0.0 {
                crossings += 1;
            }
        }

        crossings as f32 * sample_rate as f32 / samples.len() as f32
    }

    fn wav_bytes_mono_16(sample_rate: u32, samples: &[f32]) -> Vec<u8> {
        let pcm: Vec<i16> = samples
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();
        wav_bytes_from_pcm_i16(sample_rate, 1, &pcm)
    }

    fn wav_bytes_stereo_16(sample_rate: u32, interleaved_samples: &[f32]) -> Vec<u8> {
        let pcm: Vec<i16> = interleaved_samples
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();
        wav_bytes_from_pcm_i16(sample_rate, 2, &pcm)
    }

    fn wav_bytes_from_pcm_i16(sample_rate: u32, channels: u16, pcm: &[i16]) -> Vec<u8> {
        let bytes_per_sample = 2u16;
        let block_align = channels * bytes_per_sample;
        let byte_rate = sample_rate * block_align as u32;
        let data_len = (pcm.len() * 2) as u32;
        let riff_len = 36 + data_len;

        let mut out = Vec::with_capacity((44 + data_len) as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&riff_len.to_le_bytes());
        out.extend_from_slice(b"WAVE");

        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&(16u16).to_le_bytes());

        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for &sample in pcm {
            out.extend_from_slice(&sample.to_le_bytes());
        }

        out
    }

    fn deinterleave_stereo(samples: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let mut left = Vec::with_capacity(samples.len() / 2);
        let mut right = Vec::with_capacity(samples.len() / 2);
        for frame in samples.chunks_exact(2) {
            left.push(frame[0]);
            right.push(frame[1]);
        }
        (left, right)
    }
}
