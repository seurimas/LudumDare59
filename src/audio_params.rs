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
use rodio::source::{Amplify, Delay, FadeIn, SamplesConverter, SkipDuration, Speed, TakeDuration};
use serde::Deserialize;

/// Parameters for a single reverb preset.
/// All fields have sane defaults.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ReverbParams {
    /// Delay length in seconds (controls perceived room size). Default 0.1.
    pub room_size: f32,
    /// Feedback decay per reflection (0.0 = no echo, <1.0 required for stability). Default 0.5.
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

/// A rodio Source that applies a feedback comb-filter reverb to another source.
pub struct ReverbSource<S: Source<Item = f32>> {
    inner: S,
    buffer: Vec<f32>,
    pos: usize,
    decay: f32,
    wet: f32,
}

impl<S: Source<Item = f32>> ReverbSource<S> {
    pub fn new(inner: S, params: &ReverbParams) -> Self {
        let sample_rate = inner.sample_rate();
        let channels = inner.channels() as usize;
        // Align delay length to a whole number of frames so L/R channels stay paired.
        let delay_frames = ((params.room_size * sample_rate as f32) as usize).max(1);
        let buffer_len = delay_frames * channels;
        Self {
            inner,
            buffer: vec![0.0; buffer_len],
            pos: 0,
            decay: params.decay,
            wet: params.wet,
        }
    }
}

impl<S: Source<Item = f32>> Iterator for ReverbSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let input = self.inner.next()?;
        let delayed = self.buffer[self.pos];
        // Feedback comb filter: y[n] = x[n] + decay * y[n-delay]
        let feedback = input + self.decay * delayed;
        self.buffer[self.pos] = feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        // Mix dry signal with the reverb tail
        Some(input + self.decay * delayed * self.wet)
    }
}

impl<S: Source<Item = f32>> Source for ReverbSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        // Reverb adds a tail, so total duration is unbounded.
        None
    }
}

/// Parameters for a single playback of a futhark sound.
/// All fields have sane defaults so partial JSON entries are valid.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SoundParams {
    /// Playback speed multiplier. Also changes pitch. Default 1.0.
    pub speed: f32,
    /// Volume multiplier. Default 1.0.
    pub volume: f32,
    /// Fade-in duration in milliseconds. Default 0.
    pub fade_in_ms: u64,
    /// Silence before the sound starts, in milliseconds. Default 0.
    pub delay_ms: u64,
    /// Skip this many milliseconds from the start of the audio data. Default 0.
    pub skip_ms: u64,
    /// Truncate playback after this many milliseconds. Omit for no limit.
    pub take_ms: Option<u64>,
    /// Array of reverb presets to choose from at random. Empty means no reverb.
    pub reverb: Vec<ReverbParams>,
    /// Resolved reverb chosen by pick_params. Not deserialized from JSON.
    #[serde(skip)]
    pub selected_reverb: Option<ReverbParams>,
}

impl Default for SoundParams {
    fn default() -> Self {
        Self {
            speed: 1.0,
            volume: 1.0,
            fade_in_ms: 0,
            delay_ms: 0,
            skip_ms: 0,
            take_ms: None,
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

/// A single playback-ready audio asset: raw bytes plus resolved parameters.
/// Created at play-time by combining a loaded AudioSource with a SoundParams.
#[derive(Asset, TypePath)]
pub struct ProcessedAudio {
    pub bytes: Arc<[u8]>,
    pub params: SoundParams,
}

type InnerDecoder = rodio::Decoder<Cursor<Arc<[u8]>>>;
type ProcessedDecoder = ReverbSource<
    Delay<FadeIn<TakeDuration<SkipDuration<Speed<Amplify<SamplesConverter<InnerDecoder, f32>>>>>>>,
>;

impl Decodable for ProcessedAudio {
    type DecoderItem = f32;
    type Decoder = ProcessedDecoder;

    fn decoder(&self) -> Self::Decoder {
        let p = &self.params;
        // None means "play to end"; use a duration that exceeds any sound file.
        let take = p
            .take_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(86400));

        // rodio's FadeIn asserts duration > 0; use 1ns floor so zero means "instant".
        let fade_in = Duration::from_millis(p.fade_in_ms).max(Duration::from_nanos(1));

        let chain = rodio::Decoder::new(Cursor::new(self.bytes.clone()))
            .expect("valid ogg bytes")
            .convert_samples::<f32>()
            .amplify(p.volume)
            .speed(p.speed)
            .skip_duration(Duration::from_millis(p.skip_ms))
            .take_duration(take)
            .fade_in(fade_in)
            .delay(Duration::from_millis(p.delay_ms));

        let reverb = p.selected_reverb.as_ref().cloned().unwrap_or(ReverbParams {
            wet: 0.0,
            ..Default::default()
        });
        ReverbSource::new(chain, &reverb)
    }
}

/// Pick a variant from the config for the given rune index.
/// Chooses uniformly at random when multiple variants are present.
/// Also resolves a single reverb preset from the variant's reverb array.
pub fn pick_params(config: Option<&FutharkSoundConfig>, index: usize) -> SoundParams {
    let variants = config
        .and_then(|c| c.0.get(index))
        .filter(|v| !v.is_empty());

    let mut params = match variants {
        None => SoundParams::default(),
        Some(v) if v.len() == 1 => v[0].clone(),
        Some(v) => v[rand::thread_rng().gen_range(0..v.len())].clone(),
    };

    params.selected_reverb = if params.reverb.is_empty() {
        None
    } else if params.reverb.len() == 1 {
        Some(params.reverb[0].clone())
    } else {
        Some(params.reverb[rand::thread_rng().gen_range(0..params.reverb.len())].clone())
    };

    params
}

pub fn configure_audio_params(app: &mut App) {
    app.init_asset::<FutharkSoundConfig>()
        .register_asset_loader(FutharkSoundConfigLoader)
        .add_audio_source::<ProcessedAudio>();
}
