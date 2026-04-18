use std::sync::Arc;
use std::time::Duration;

use bevy::audio::Decodable;
use rodio::Source;

/// A fully pre-processed audio asset ready for playback.
/// All effects (duration normalisation, echo, fade, reverb) are baked into `samples`.
#[derive(bevy::prelude::Asset, bevy::prelude::TypePath)]
pub struct ProcessedAudio {
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub sample_rate: u32,
}

/// A simple iterator that plays back a pre-processed sample buffer.
pub struct SamplesDecoder {
    pub(super) samples: Arc<[f32]>,
    pub(super) pos: usize,
    pub(super) channels: u16,
    pub(super) sample_rate: u32,
}

impl Iterator for SamplesDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos < self.samples.len() {
            let s = self.samples[self.pos];
            self.pos += 1;
            Some(s)
        } else {
            None
        }
    }
}

impl Source for SamplesDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len().saturating_sub(self.pos))
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let frames = self.samples.len() / self.channels as usize;
        Some(Duration::from_secs_f32(
            frames as f32 / self.sample_rate as f32,
        ))
    }
}

impl Decodable for ProcessedAudio {
    type DecoderItem = f32;
    type Decoder = SamplesDecoder;

    fn decoder(&self) -> Self::Decoder {
        SamplesDecoder {
            samples: self.samples.clone(),
            pos: 0,
            channels: self.channels,
            sample_rate: self.sample_rate,
        }
    }
}
