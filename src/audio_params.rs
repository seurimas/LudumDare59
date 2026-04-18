use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::audio::AddAudioSource;
use bevy::audio::Decodable;
use bevy::prelude::*;
use rand::Rng;
use rodio::Source;
use serde::Deserialize;

/// Parameters for a single reverb preset.
/// All fields have sane defaults.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ReverbParams {
    /// Delay length in seconds (controls perceived room size). Default 0.1.
    pub room_size: f32,
    /// Feedback decay per reflection (0.0 = instant decay, <1.0 required for stability). Default 0.5.
    pub decay: f32,
    /// Wet/dry mix (0.0 = dry, 1.0 = full reverb). Default 0.3.
    pub wet: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.1,
            decay: 0.5,
            wet: 0.3,
        }
    }
}

/// Parameters for a single playback of a futhark sound.
/// All fields have sane defaults so partial JSON entries are valid.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SoundParams {
    /// Volume multiplier. Default 1.0.
    pub volume: f32,
    /// Fade-in duration in milliseconds. Default 0.
    pub fade_in_ms: u64,
    /// Silence before the sound starts, in milliseconds. Default 0.
    pub delay_ms: u64,
    /// Skip this many milliseconds from the start of the audio data. Default 0.
    pub skip_ms: u64,
    /// Target playback duration in milliseconds. 0 means play as-is.
    /// - Samples longer than this are truncated with a 100ms fade-out at the cut point.
    /// - Samples shorter than this are echoed (repeated at decaying volume) to fill the duration,
    ///   unless `conversational` is true.
    pub duration_ms: u64,
    /// Volume multiplier applied to each successive echo repeat. Default 0.5.
    pub echo_decay: f32,
    /// If true, shorter samples play as-is (no echo) and longer samples still fade at cut. Default false.
    pub conversational: bool,
    /// Array of reverb presets to choose from at random. Empty means no reverb.
    pub reverb: Vec<ReverbParams>,
    /// Resolved reverb chosen by pick_params. Not deserialized from JSON.
    #[serde(skip)]
    pub selected_reverb: Option<ReverbParams>,
}

impl Default for SoundParams {
    fn default() -> Self {
        Self {
            volume: 1.0,
            fade_in_ms: 0,
            delay_ms: 0,
            skip_ms: 0,
            duration_ms: 0,
            echo_decay: 0.5,
            conversational: false,
            reverb: Vec::new(),
            selected_reverb: None,
        }
    }
}

/// JSON asset: a 24-element array (one per futhark letter in LETTERS order),
/// each element an array of one or more SoundParams variants.
/// When a key is pressed, one variant is chosen at random.
#[derive(Asset, TypePath, Deserialize)]
pub struct FutharkSoundConfig(pub Vec<Vec<SoundParams>>);

#[derive(Default, TypePath)]
pub struct FutharkSoundConfigLoader;

impl AssetLoader for FutharkSoundConfigLoader {
    type Asset = FutharkSoundConfig;
    type Settings = ();
    type Error = FutharkSoundConfigError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<FutharkSoundConfig, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[derive(Debug)]
pub enum FutharkSoundConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for FutharkSoundConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
        }
    }
}

impl std::error::Error for FutharkSoundConfigError {}

impl From<std::io::Error> for FutharkSoundConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for FutharkSoundConfigError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// A fully pre-processed audio asset ready for playback.
/// All effects (duration normalisation, echo, fade, reverb) are baked into `samples`.
#[derive(Asset, TypePath)]
pub struct ProcessedAudio {
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub sample_rate: u32,
}

/// A simple iterator that plays back a pre-processed sample buffer.
pub struct SamplesDecoder {
    samples: Arc<[f32]>,
    pos: usize,
    channels: u16,
    sample_rate: u32,
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

// ── helpers ─────────────────────────────────────────────────────────────────

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

// ── public processing entry point ────────────────────────────────────────────

/// Decode raw audio bytes and apply all effects specified by `params`.
/// Returns a `ProcessedAudio` ready to be added to the bevy asset store.
pub fn process_audio(bytes: &Arc<[u8]>, params: &SoundParams) -> ProcessedAudio {
    let decoder = rodio::Decoder::new(Cursor::new(bytes.clone())).expect("valid audio bytes");
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();

    // Collect all raw samples as f32
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
            // Longer sample: truncate with 100ms fade-out at the cut point
            samples.truncate(target);
            let fade_start = target.saturating_sub(fade_len);
            for i in fade_start..target {
                let t = (i - fade_start) as f32 / fade_len as f32;
                samples[i] *= 1.0 - t;
            }
        } else if !params.conversational {
            // Shorter sample: echo (repeat at decaying volume) until duration is satisfied.
            // Spillover beyond `target` on the last echo is allowed.
            let original = samples.clone();
            let mut vol = params.echo_decay;
            while samples.len() < target {
                let echo: Vec<f32> = original.iter().map(|&s| s * vol).collect();
                samples.extend_from_slice(&echo);
                vol *= params.echo_decay;
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

// ── variant selection ────────────────────────────────────────────────────────

/// Pick a random variant from the config for the given rune index and resolve
/// the reverb selection. Returns a fully-resolved `SoundParams`.
pub fn pick_params(config: Option<&FutharkSoundConfig>, index: usize) -> SoundParams {
    let variants = config
        .and_then(|c| c.0.get(index))
        .filter(|v| !v.is_empty());

    let mut params = match variants {
        None => SoundParams::default(),
        Some(v) if v.len() == 1 => v[0].clone(),
        Some(v) => v[rand::thread_rng().gen_range(0..v.len())].clone(),
    };

    params.selected_reverb = match params.reverb.len() {
        0 => None,
        1 => Some(params.reverb[0].clone()),
        n => Some(params.reverb[rand::thread_rng().gen_range(0..n)].clone()),
    };

    params
}

pub fn configure_audio_params(app: &mut App) {
    app.init_asset::<FutharkSoundConfig>()
        .register_asset_loader(FutharkSoundConfigLoader)
        .add_audio_source::<ProcessedAudio>();
}
