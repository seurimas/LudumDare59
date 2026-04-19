use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use rand::Rng;

use crate::GameAssets;

pub use crate::ui::keyboard::{
    EliminatedFutharkKeys, EliminatedKeyPressed, FutharkActionButton, FutharkKeyBackground,
    FutharkKeyButton, FutharkKeyFade, FutharkKeyLabel, FutharkKeyLetterVisual,
    FutharkKeyRuneVisual, FutharkKeyboard, FutharkKeyboardAliases, FutharkKeyboardAnimationSpeed,
    FutharkKeyboardButton, FutharkKeyboardCommand, FutharkKeyboardCommandType,
    FutharkKeyboardLegendMode, FutharkTabActionVisual, KeyboardPanel, SPRITE_RUNE_OFFSET,
    TypedFutharkInput, animate_futhark_keyboard_colors, configure_futhark_keyboard,
    emit_futhark_keyboard_command_from_clicks, emit_typed_futhark_input_from_keyboard,
    emit_typed_futhark_input_from_keyboard_clicks, keyboard_rows_by_index,
    last_typed_futhark_character, spawn_futhark_keyboard, sync_eliminated_futhark_keys,
    sync_futhark_key_hover, sync_futhark_keyboard_labels, toggle_futhark_keyboard_legend_mode,
};

pub const LETTERS: [char; 25] = [
    'f', // 0
    'u', // 1
    'T', // 2
    'a', // 3
    'r', // 4
    'k', // 5
    'g', // 6
    'w', // 7
    'h', // 8
    'n', // 9
    'i', // 10
    'j', // 11
    'A', // 12
    'p', // 13
    'z', // 14
    's', // 15
    't', // 16
    'b', // 17
    'e', // 18
    'm', // 19
    'l', // 20
    'N', // 21
    'd', // 22
    'o', // 23
    'S', // 24
];

#[derive(Resource, Default)]
pub struct PrebakedFutharkAudio {
    pub handles_by_index: Vec<Vec<Handle<AudioSource>>>,
}

#[derive(Resource, Default)]
pub struct PrebakedFutharkConversationalAudio {
    pub handles_by_index: Vec<Vec<Handle<AudioSource>>>,
}

/// Raw f32 sample buffers for every baked letter variant.
/// Used mid-game when multiple letters need to be concatenated before playback.
#[derive(Resource, Default)]
pub struct BakedAudioSamples {
    pub regular: Vec<Vec<crate::audio::ProcessedAudio>>,
    pub conversational: Vec<Vec<crate::audio::ProcessedAudio>>,
}

/// Bake all parameter variants for one futhark letter.
///
/// Returns:
/// - `Vec<Handle<AudioSource>>` — WAV-encoded handles ready for `AudioPlayer`.
/// - `Vec<ProcessedAudio>`     — raw f32 samples kept for mid-game concatenation.
pub fn bake_futhark_letter(
    letter_index: usize,
    game_assets: &GameAssets,
    config: Option<&crate::audio::FutharkSoundConfig>,
    audio_assets: &mut Assets<AudioSource>,
) -> (Vec<Handle<AudioSource>>, Vec<crate::audio::ProcessedAudio>) {
    let Some(raw) = game_assets.futhark_sounds.get(letter_index) else {
        panic!("invalid futhark letter index");
    };
    let Some(source) = audio_assets.get(&raw.clone().typed::<AudioSource>()) else {
        return (Vec::new(), Vec::new());
    };

    let processed_list: Vec<crate::audio::ProcessedAudio> =
        params_to_bake_for_index(config, letter_index)
            .into_iter()
            .map(|params| crate::audio::process_audio(&source.bytes, &params))
            .collect();

    let handles: Vec<Handle<AudioSource>> = processed_list
        .iter()
        .map(|p| {
            let wav = crate::audio::samples_to_wav(&p.samples, p.channels, p.sample_rate);
            audio_assets.add(AudioSource { bytes: wav.into() })
        })
        .collect();

    (handles, processed_list)
}

fn params_to_bake_for_index(
    config: Option<&crate::audio::FutharkSoundConfig>,
    index: usize,
) -> Vec<crate::audio::SoundParams> {
    let variants = config
        .and_then(|c| c.0.get(index))
        .filter(|v| !v.is_empty());

    let base_variants: Vec<crate::audio::SoundParams> = match variants {
        None => vec![crate::audio::SoundParams::default()],
        Some(v) => v.clone(),
    };

    base_variants
}

pub fn play_futhark_key_sound(
    mut typed_futhark_input: MessageReader<crate::rune_words::rune_slots::TypedInputDuringGrading>,
    prebaked_audio: Option<Res<PrebakedFutharkAudio>>,
    mut commands: Commands,
) {
    let Some(prebaked_audio) = prebaked_audio else {
        return;
    };

    for event in typed_futhark_input.read() {
        let Some(index) = letter_to_index(event.0) else {
            continue;
        };
        let Some(handles) = prebaked_audio
            .handles_by_index
            .get(index)
            .filter(|h| !h.is_empty())
        else {
            continue;
        };

        let handle = if handles.len() == 1 {
            handles[0].clone()
        } else {
            let i = rand::thread_rng().gen_range(0..handles.len());
            handles[i].clone()
        };
        commands.spawn((
            AudioPlayer::<AudioSource>(handle),
            PlaybackSettings::DESPAWN,
        ));
    }
}

pub fn index_to_letter(index: usize) -> Option<char> {
    LETTERS.get(index).copied()
}

pub fn letter_to_index(letter: char) -> Option<usize> {
    LETTERS
        .iter()
        .position(|mapped_letter| *mapped_letter == letter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_letter_maps_all_futhark_entries() {
        for (index, letter) in LETTERS.iter().enumerate() {
            assert_eq!(index_to_letter(index), Some(*letter));
        }
    }

    #[test]
    fn letter_to_index_maps_all_futhark_entries() {
        for (index, letter) in LETTERS.iter().enumerate() {
            assert_eq!(letter_to_index(*letter), Some(index));
        }
    }

    #[test]
    fn unknown_values_are_rejected() {
        assert_eq!(index_to_letter(25), None);
        assert_eq!(letter_to_index('x'), None);
    }
}
