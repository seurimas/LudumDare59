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
    /// Volume multiplier applied per repetition-length in the echo envelope. Default 0.5.
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
