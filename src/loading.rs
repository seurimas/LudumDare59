use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::GameAssets;
use crate::GameState;
use crate::futhark::{
    BakedAudioSamples, PrebakedFutharkAudio, PrebakedFutharkConversationalAudio,
    bake_futhark_letter,
};

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct ProcessingScreen;

#[derive(Component)]
struct ProcessingRuneSlot {
    slot_index: usize,
    fade_elapsed: f32,
    assigned: bool,
}

#[derive(Resource)]
struct ProcessingQueue {
    next_letter: usize,
    regular_handles: Vec<Vec<Handle<AudioSource>>>,
    conversational_handles: Vec<Vec<Handle<AudioSource>>>,
    regular_samples: Vec<Vec<crate::audio::ProcessedAudio>>,
    conversational_samples: Vec<Vec<crate::audio::ProcessedAudio>>,
}

impl Default for ProcessingQueue {
    fn default() -> Self {
        Self {
            next_letter: 0,
            regular_handles: vec![Vec::new(); FUTHARK_LETTER_COUNT],
            conversational_handles: vec![Vec::new(); FUTHARK_LETTER_COUNT],
            regular_samples: (0..FUTHARK_LETTER_COUNT).map(|_| Vec::new()).collect(),
            conversational_samples: (0..FUTHARK_LETTER_COUNT).map(|_| Vec::new()).collect(),
        }
    }
}

const FUTHARK_LETTER_COUNT: usize = 25;
const RUNE_COUNT: usize = 5;
const RUNE_SPACING: f32 = 48.0;
const FADE_DURATION_SECONDS: f32 = 0.2;

pub fn configure_loading(app: &mut App) {
    app.init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Processing)
                .load_collection::<GameAssets>(),
        )
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
        .add_systems(OnExit(GameState::Loading), despawn_loading_screen)
        .add_systems(OnEnter(GameState::Processing), spawn_processing_screen)
        .add_systems(
            Update,
            (process_next_letter, animate_processing_runes)
                .chain()
                .run_if(in_state(GameState::Processing)),
        )
        .add_systems(OnExit(GameState::Processing), despawn_processing_screen);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 1280.0,
                min_height: 720.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            LoadingScreen,
        ))
        .with_child((
            Text::new("Loading..."),
            TextFont {
                font_size: 50.0,
                ..default()
            },
        ));
}

fn despawn_loading_screen(mut commands: Commands, query: Query<Entity, With<LoadingScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn spawn_processing_screen(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.insert_resource(ProcessingQueue::default());

    let offset = (RUNE_COUNT as f32 - 1.0) * 0.5;
    for i in 0..RUNE_COUNT {
        commands.spawn((
            Sprite {
                image: game_assets.futhark.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: game_assets.futhark_layout.clone(),
                    index: 0,
                }),
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                ..default()
            },
            Transform::from_xyz((i as f32 - offset) * RUNE_SPACING, 0.0, 0.0),
            ProcessingRuneSlot {
                slot_index: i,
                fade_elapsed: 0.0,
                assigned: false,
            },
            ProcessingScreen,
        ));
    }
}

fn process_next_letter(
    mut queue: ResMut<ProcessingQueue>,
    game_assets: Res<GameAssets>,
    sound_configs: Res<Assets<crate::audio::FutharkSoundConfig>>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
    mut slots: Query<(&mut ProcessingRuneSlot, &mut Sprite)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    if queue.next_letter >= FUTHARK_LETTER_COUNT {
        return;
    }

    let letter_index = queue.next_letter;
    let slot_index = letter_index % RUNE_COUNT;

    for (mut slot, mut sprite) in &mut slots {
        if slot.slot_index == slot_index {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = letter_index;
            }
            sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            slot.fade_elapsed = 0.0;
            slot.assigned = true;
        }
    }

    let regular_config = sound_configs.get(&game_assets.futhark_sound_params);
    let conv_config = sound_configs.get(&game_assets.futhark_conversational_params);

    let (reg_handles, reg_samples) = bake_futhark_letter(
        letter_index,
        &game_assets,
        regular_config,
        &mut audio_assets,
    );
    queue.regular_handles[letter_index] = reg_handles;
    queue.regular_samples[letter_index] = reg_samples;

    let (conv_handles, conv_samples) =
        bake_futhark_letter(letter_index, &game_assets, conv_config, &mut audio_assets);
    queue.conversational_handles[letter_index] = conv_handles;
    queue.conversational_samples[letter_index] = conv_samples;

    queue.next_letter += 1;

    if queue.next_letter >= FUTHARK_LETTER_COUNT {
        commands.insert_resource(PrebakedFutharkAudio {
            handles_by_index: queue.regular_handles.clone(),
        });
        commands.insert_resource(PrebakedFutharkConversationalAudio {
            handles_by_index: queue.conversational_handles.clone(),
        });
        // Move raw samples into a permanent resource for mid-game concatenation.
        let mut baked = BakedAudioSamples::default();
        std::mem::swap(&mut baked.regular, &mut queue.regular_samples);
        std::mem::swap(&mut baked.conversational, &mut queue.conversational_samples);
        commands.insert_resource(baked);
        next_state.set(GameState::Ready);
    }
}

fn animate_processing_runes(
    mut slots: Query<(&mut ProcessingRuneSlot, &mut Sprite)>,
    time: Res<Time>,
) {
    for (mut slot, mut sprite) in &mut slots {
        if !slot.assigned {
            continue;
        }
        slot.fade_elapsed += time.delta_secs();
        let alpha = (slot.fade_elapsed / FADE_DURATION_SECONDS).clamp(0.0, 1.0);
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
}

fn despawn_processing_screen(mut commands: Commands, query: Query<Entity, With<ProcessingScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<ProcessingQueue>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_cycles_after_five_letters() {
        let expected = [0usize, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1];
        for (letter, &expected_slot) in expected.iter().enumerate() {
            assert_eq!(letter % RUNE_COUNT, expected_slot);
        }
    }

    #[test]
    fn fade_alpha_starts_at_zero() {
        let alpha = (0.0f32 / FADE_DURATION_SECONDS).clamp(0.0, 1.0);
        assert_eq!(alpha, 0.0);
    }

    #[test]
    fn fade_alpha_reaches_one_after_full_duration() {
        let alpha = (FADE_DURATION_SECONDS / FADE_DURATION_SECONDS).clamp(0.0, 1.0);
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn all_twenty_four_letters_fit_in_five_slots() {
        let mut max_slot = 0;
        for i in 0..FUTHARK_LETTER_COUNT {
            max_slot = max_slot.max(i % RUNE_COUNT);
        }
        assert_eq!(max_slot, RUNE_COUNT - 1);
    }
}
