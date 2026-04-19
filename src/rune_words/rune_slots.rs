use bevy::ecs::message::MessageReader;
use bevy::ecs::message::MessageWriter;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use std::collections::{HashSet, VecDeque};

use crate::{GameAssets, futhark};

const SPRITE_SLOT_BACKGROUND: usize = 255;
const SPRITE_PRIMARY_RUNE_OFFSET: usize = 32;
const SPRITE_ALTERNATE_RUNE_OFFSET: usize = 64;
const RUNES_PER_SET: usize = 25;
const ALTERNATE_SET_PAGES: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuneSlotForegroundSet {
    Primary,
    Alternate { page: usize },
}

impl Default for RuneSlotForegroundSet {
    fn default() -> Self {
        Self::Primary
    }
}

impl RuneSlotForegroundSet {
    fn sprite_index_for_rune(self, rune_index: usize) -> usize {
        let normalized_index = rune_index % RUNES_PER_SET;

        match self {
            Self::Primary => SPRITE_PRIMARY_RUNE_OFFSET + normalized_index,
            Self::Alternate { page } => {
                let clamped_page = page.min(ALTERNATE_SET_PAGES - 1);
                SPRITE_ALTERNATE_RUNE_OFFSET + clamped_page * RUNES_PER_SET + normalized_index
            }
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuneSlot {
    pub rune_index: Option<usize>,
    pub foreground_set: RuneSlotForegroundSet,
}

#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuneSlotLinks {
    pub prev: Option<Entity>,
    pub next: Option<Entity>,
}

#[derive(Component)]
pub struct RuneSlotBackground {
    pub base_color: Color,
}

#[derive(Component)]
pub struct RuneSlotForeground;

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveRuneSlot {
    pub entity: Option<Entity>,
}

#[derive(Message, Default)]
pub struct EnterActiveRuneWord;

#[derive(bevy::ecs::message::Message, Clone)]
pub struct PlayFutharkLetters(pub String);

pub struct RuneSlotConfig {
    pub left: Val,
    pub top: Val,
    pub size: f32,
    pub background_color: Color,
    pub foreground_set: RuneSlotForegroundSet,
    pub initial_rune: Option<char>,
}

impl Default for RuneSlotConfig {
    fn default() -> Self {
        Self {
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            size: 48.0,
            background_color: Color::WHITE,
            foreground_set: RuneSlotForegroundSet::Primary,
            initial_rune: None,
        }
    }
}

/// Queued word audio playback — the first handle is spawned immediately and subsequent
/// handles play after each clip's duration has elapsed.
#[derive(Resource, Default)]
pub struct WordAudioQueue {
    pending: VecDeque<(Handle<AudioSource>, f32)>,
    elapsed: f32,
    current_duration: f32,
}

#[derive(Resource, Default)]
pub struct QueuedTypedInput {
    queued: VecDeque<char>,
}

#[derive(bevy::ecs::message::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypedInputDuringGrading(pub char);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypedInputPolicy {
    AcceptNormally,
    Ignore,
    QueueForNextWord,
}

pub fn configure_rune_slots(app: &mut App) {
    app.init_resource::<ActiveRuneSlot>();
    app.init_resource::<WordAudioQueue>();
    app.init_resource::<QueuedTypedInput>();
    app.add_message::<EnterActiveRuneWord>();
    app.add_message::<PlayFutharkLetters>();
    app.add_message::<TypedInputDuringGrading>();
}

fn typed_input_during_grading_behavior(
    battle_state: Option<&crate::rune_words::battle::BattleState>,
    pending_grading: Option<&crate::rune_words::battle::PendingRowGrading>,
    acting_data: Option<&crate::rune_words::battle_states::acting::ActingData>,
) -> TypedInputPolicy {
    let Some(state) = battle_state else {
        return TypedInputPolicy::AcceptNormally;
    };
    let Some(pending_grading) = pending_grading else {
        return TypedInputPolicy::AcceptNormally;
    };

    let in_submit_animation_window =
        pending_grading.is_active() || state.active_row_slots.is_empty();
    if !in_submit_animation_window {
        return TypedInputPolicy::AcceptNormally;
    }

    match state.phase {
        crate::rune_words::battle::BattlePhase::Acting => {
            if acting_data
                .map(|data| data.pending_success)
                .unwrap_or(false)
            {
                TypedInputPolicy::Ignore
            } else {
                TypedInputPolicy::QueueForNextWord
            }
        }
        _ => TypedInputPolicy::AcceptNormally,
    }
}

fn type_into_active_slot(
    letter: char,
    active_slot: &mut ActiveRuneSlot,
    slots: &mut Query<(&mut RuneSlot, Option<&RuneSlotLinks>)>,
) -> bool {
    let Some(active_entity) = active_slot.entity else {
        return false;
    };

    let Some(index) = futhark::letter_to_index(letter) else {
        return false;
    };

    let Ok((mut slot, links)) = slots.get_mut(active_entity) else {
        return false;
    };

    slot.rune_index = Some(index);

    if let Some(next) = links.and_then(|l| l.next) {
        active_slot.entity = Some(next);
    }

    true
}

/// Advance the word audio sequence each frame and spawn the next handle when the
/// current clip has finished.
pub fn tick_word_audio_queue(
    mut queue: ResMut<WordAudioQueue>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if queue.pending.is_empty() {
        return;
    }
    queue.elapsed += time.delta_secs();
    if queue.elapsed >= queue.current_duration {
        if let Some((handle, duration)) = queue.pending.pop_front() {
            commands.spawn((
                AudioPlayer::<AudioSource>(handle),
                PlaybackSettings::DESPAWN,
            ));
            queue.current_duration = duration;
            queue.elapsed = 0.0;
        }
    }
}

/// Play the first handle immediately, queue the rest for sequential playback.
fn start_audio_sequence(
    handles_and_durations: impl IntoIterator<Item = (Handle<AudioSource>, f32)>,
    queue: &mut WordAudioQueue,
    commands: &mut Commands,
) {
    queue.pending.clear();
    queue.elapsed = 0.0;
    queue.current_duration = 0.0;

    let mut iter = handles_and_durations.into_iter();
    if let Some((first_handle, first_duration)) = iter.next() {
        commands.spawn((
            AudioPlayer::<AudioSource>(first_handle),
            PlaybackSettings::DESPAWN,
        ));
        queue.current_duration = first_duration;
    }
    queue.pending.extend(iter);
}

/// Duration in seconds of a pre-processed clip.
fn clip_duration(p: &crate::audio::ProcessedAudio) -> f32 {
    if p.channels == 0 || p.sample_rate == 0 {
        return 0.0;
    }
    p.samples.len() as f32 / (p.channels as f32 * p.sample_rate as f32)
}

pub fn spawn_rune_slot(
    commands: &mut Commands,
    game_assets: &GameAssets,
    config: RuneSlotConfig,
) -> Entity {
    let rune_index = config.initial_rune.and_then(futhark::letter_to_index);
    let foreground_index = rune_index
        .map(|index| config.foreground_set.sprite_index_for_rune(index))
        .unwrap_or(SPRITE_PRIMARY_RUNE_OFFSET);
    let foreground_visibility = if rune_index.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: config.left,
                top: config.top,
                width: Val::Px(config.size),
                height: Val::Px(config.size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            RuneSlot {
                rune_index,
                foreground_set: config.foreground_set,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(config.size),
                    height: Val::Px(config.size),
                    ..default()
                },
                ImageNode::from_atlas_image(
                    game_assets.futhark.clone(),
                    TextureAtlas {
                        layout: game_assets.futhark_layout.clone(),
                        index: SPRITE_SLOT_BACKGROUND,
                    },
                ),
                RuneSlotBackground {
                    base_color: config.background_color,
                },
            ));

            parent.spawn((
                Node {
                    width: Val::Px(config.size - 12.0),
                    height: Val::Px(config.size - 12.0),
                    ..default()
                },
                ImageNode::from_atlas_image(
                    game_assets.futhark.clone(),
                    TextureAtlas {
                        layout: game_assets.futhark_layout.clone(),
                        index: foreground_index,
                    },
                ),
                foreground_visibility,
                RuneSlotForeground,
            ));
        })
        .id()
}

/// Like `spawn_rune_slot` but uses flex layout (no absolute top/left positioning).
/// Use this when the slot will be a flex child of a `RuneSlotRow` container.
pub fn spawn_rune_slot_flex(
    commands: &mut Commands,
    game_assets: &GameAssets,
    config: RuneSlotConfig,
) -> Entity {
    let rune_index = config.initial_rune.and_then(futhark::letter_to_index);
    let foreground_index = rune_index
        .map(|index| config.foreground_set.sprite_index_for_rune(index))
        .unwrap_or(SPRITE_PRIMARY_RUNE_OFFSET);
    let foreground_visibility = if rune_index.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands
        .spawn((
            Button,
            Node {
                width: Val::Px(config.size),
                height: Val::Px(config.size),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            RuneSlot {
                rune_index,
                foreground_set: config.foreground_set,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(config.size),
                    height: Val::Px(config.size),
                    ..default()
                },
                ImageNode::from_atlas_image(
                    game_assets.futhark.clone(),
                    TextureAtlas {
                        layout: game_assets.futhark_layout.clone(),
                        index: SPRITE_SLOT_BACKGROUND,
                    },
                ),
                RuneSlotBackground {
                    base_color: config.background_color,
                },
            ));

            parent.spawn((
                Node {
                    width: Val::Px(config.size - 12.0),
                    height: Val::Px(config.size - 12.0),
                    ..default()
                },
                ImageNode::from_atlas_image(
                    game_assets.futhark.clone(),
                    TextureAtlas {
                        layout: game_assets.futhark_layout.clone(),
                        index: foreground_index,
                    },
                ),
                foreground_visibility,
                RuneSlotForeground,
            ));
        })
        .id()
}

pub fn spawn_rune_word(
    commands: &mut Commands,
    game_assets: &GameAssets,
    configs: Vec<RuneSlotConfig>,
) -> Vec<Entity> {
    let entities: Vec<Entity> = configs
        .into_iter()
        .map(|config| spawn_rune_slot(commands, game_assets, config))
        .collect();

    let len = entities.len();
    for i in 0..len {
        let prev = if i > 0 { Some(entities[i - 1]) } else { None };
        let next = if i + 1 < len {
            Some(entities[i + 1])
        } else {
            None
        };
        commands
            .entity(entities[i])
            .insert(RuneSlotLinks { prev, next });
    }

    entities
}

pub fn activate_rune_slot_on_click(
    slots: Query<(Entity, &Interaction), (Changed<Interaction>, With<RuneSlot>)>,
    mut active_slot: ResMut<ActiveRuneSlot>,
) {
    for (entity, interaction) in &slots {
        if *interaction == Interaction::Pressed {
            active_slot.entity = Some(entity);
        }
    }
}

pub fn update_active_rune_slot_from_typed_input(
    mut typed_futhark_input: MessageReader<futhark::TypedFutharkInput>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut queued_typed_input: ResMut<QueuedTypedInput>,
    mut typed_during_grading: MessageWriter<TypedInputDuringGrading>,
    mut slots: Query<(&mut RuneSlot, Option<&RuneSlotLinks>)>,
    battle_state: Option<Res<crate::rune_words::battle::BattleState>>,
    pending_grading: Option<Res<crate::rune_words::battle::PendingRowGrading>>,
    acting_data: Option<Res<crate::rune_words::battle_states::acting::ActingData>>,
) {
    let typed_letters: Vec<char> = typed_futhark_input.read().map(|event| event.0).collect();

    let behavior = typed_input_during_grading_behavior(
        battle_state.as_deref(),
        pending_grading.as_deref(),
        acting_data.as_deref(),
    );

    if behavior == TypedInputPolicy::Ignore {
        return;
    }

    if behavior == TypedInputPolicy::QueueForNextWord {
        queued_typed_input.queued.extend(typed_letters);
        return;
    }

    while let Some(letter) = queued_typed_input.queued.pop_front() {
        if type_into_active_slot(letter, &mut active_slot, &mut slots) {
            typed_during_grading.write(TypedInputDuringGrading(letter));
        }
    }

    for letter in typed_letters {
        if type_into_active_slot(letter, &mut active_slot, &mut slots) {
            typed_during_grading.write(TypedInputDuringGrading(letter));
        }
    }
}

pub fn handle_backspace_in_rune_slots(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut keyboard_commands: MessageReader<futhark::FutharkKeyboardCommand>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut slots: Query<(&mut RuneSlot, Option<&RuneSlotLinks>)>,
) {
    let backspace_pressed = keyboard_input.read().any(|ev| {
        ev.state == ButtonState::Pressed && ev.key_code == KeyCode::Backspace
            || ev.key_code == KeyCode::Delete
            || ev.key_code == KeyCode::Comma // Positional standin for OSK.
    }) || keyboard_commands
        .read()
        .any(|command| command.0 == futhark::FutharkKeyboardCommandType::Backspace);

    if !backspace_pressed {
        return;
    }

    let Some(active_entity) = active_slot.entity else {
        return;
    };

    let Ok((slot, links)) = slots.get(active_entity) else {
        return;
    };

    if slot.rune_index.is_some() {
        if let Ok((mut slot, _)) = slots.get_mut(active_entity) {
            slot.rune_index = None;
        }
    } else if let Some(prev_entity) = links.and_then(|l| l.prev) {
        if let Ok((mut prev_slot, _)) = slots.get_mut(prev_entity) {
            prev_slot.rune_index = None;
        }
        active_slot.entity = Some(prev_entity);
    }
}

pub fn emit_play_active_rune_word_audio_on_enter(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut play_events: MessageWriter<EnterActiveRuneWord>,
) {
    let enter_pressed = keyboard_input
        .read()
        .any(|ev| ev.state == ButtonState::Pressed && ev.key_code == KeyCode::Enter);

    if enter_pressed {
        play_events.write_default();
    }
}

pub fn play_active_rune_word_audio(
    mut play_events: MessageReader<EnterActiveRuneWord>,
    active_slot: Res<ActiveRuneSlot>,
    slots: Query<(&RuneSlot, Option<&RuneSlotLinks>)>,
    battle_state: Option<Res<crate::rune_words::battle::BattleState>>,
    prebaked_audio: Option<Res<crate::futhark::PrebakedFutharkConversationalAudio>>,
    baked_samples: Option<Res<crate::futhark::BakedAudioSamples>>,
    mut queue: ResMut<WordAudioQueue>,
    mut commands: Commands,
) {
    if play_events.is_empty() {
        return;
    }

    play_events.clear();

    // In battle phases, Enter is used for guess submission and battle systems
    // will queue any required confirmation playback explicitly.
    if battle_state
        .as_ref()
        .map(|s| s.phase != crate::rune_words::battle::BattlePhase::Idle)
        .unwrap_or(false)
    {
        return;
    }

    let Some(prebaked_audio) = prebaked_audio else {
        return;
    };
    let Some(baked_samples) = baked_samples else {
        return;
    };

    let Some(active_entity) = active_slot.entity else {
        return;
    };

    let entries: Vec<(Handle<AudioSource>, f32)> = collect_word_rune_indices(active_entity, &slots)
        .into_iter()
        .filter_map(|rune_index| {
            let handle = prebaked_audio
                .handles_by_index
                .get(rune_index)
                .and_then(|h| h.first())
                .cloned()?;
            let duration = baked_samples
                .conversational
                .get(rune_index)
                .and_then(|v| v.first())
                .map(clip_duration)
                .unwrap_or(0.0);
            Some((handle, duration))
        })
        .collect();

    if !entries.is_empty() {
        start_audio_sequence(entries, &mut queue, &mut commands);
    }
}

pub fn play_futhark_letters_audio(
    mut play_events: MessageReader<PlayFutharkLetters>,
    prebaked_audio: Option<Res<crate::futhark::PrebakedFutharkConversationalAudio>>,
    baked_samples: Option<Res<crate::futhark::BakedAudioSamples>>,
    mut queue: ResMut<WordAudioQueue>,
    mut commands: Commands,
) {
    let Some(PlayFutharkLetters(letters)) = play_events.read().last().cloned() else {
        return;
    };

    let Some(prebaked_audio) = prebaked_audio else {
        return;
    };
    let Some(baked_samples) = baked_samples else {
        return;
    };

    let entries: Vec<(Handle<AudioSource>, f32)> = letters
        .chars()
        .filter_map(crate::futhark::letter_to_index)
        .filter_map(|rune_index| {
            let handle = prebaked_audio
                .handles_by_index
                .get(rune_index)
                .and_then(|h| h.first())
                .cloned()?;
            let duration = baked_samples
                .conversational
                .get(rune_index)
                .and_then(|v| v.first())
                .map(clip_duration)
                .unwrap_or(0.0);
            Some((handle, duration))
        })
        .collect();

    if !entries.is_empty() {
        start_audio_sequence(entries, &mut queue, &mut commands);
    }
}

fn collect_word_rune_indices(
    active_entity: Entity,
    slots: &Query<(&RuneSlot, Option<&RuneSlotLinks>)>,
) -> Vec<usize> {
    let mut first = active_entity;
    let mut seen = HashSet::new();

    loop {
        if !seen.insert(first) {
            break;
        }

        let Ok((_, links)) = slots.get(first) else {
            return Vec::new();
        };

        let Some(prev) = links.and_then(|l| l.prev) else {
            break;
        };
        first = prev;
    }

    let mut indices = Vec::new();
    let mut current = Some(first);
    let mut forward_seen = HashSet::new();

    while let Some(entity) = current {
        if !forward_seen.insert(entity) {
            break;
        }

        let Ok((slot, links)) = slots.get(entity) else {
            break;
        };

        if let Some(index) = slot.rune_index {
            indices.push(index);
        }

        current = links.and_then(|l| l.next);
    }

    indices
}

pub fn sync_rune_slot_visuals(
    active_slot: Res<ActiveRuneSlot>,
    slots: Query<(Entity, &RuneSlot, &Children)>,
    mut backgrounds: Query<
        (&RuneSlotBackground, &mut ImageNode),
        (With<RuneSlotBackground>, Without<RuneSlotForeground>),
    >,
    mut foregrounds: Query<
        (&mut ImageNode, &mut Visibility),
        (With<RuneSlotForeground>, Without<RuneSlotBackground>),
    >,
) {
    for (entity, slot, children) in &slots {
        let is_active = active_slot.entity == Some(entity);

        for child in children.iter() {
            if let Ok((background, mut image)) = backgrounds.get_mut(child) {
                image.color = if is_active {
                    highlighted_color(background.base_color)
                } else {
                    background.base_color
                };
            }

            if let Ok((mut image, mut visibility)) = foregrounds.get_mut(child) {
                if let Some(rune_index) = slot.rune_index {
                    if let Some(texture_atlas) = &mut image.texture_atlas {
                        texture_atlas.index = slot.foreground_set.sprite_index_for_rune(rune_index);
                    }
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

fn highlighted_color(base: Color) -> Color {
    let srgb = base.to_srgba();

    Color::srgba(
        (srgb.red + 0.25).min(1.0),
        (srgb.green + 0.25).min(1.0),
        (srgb.blue + 0.25).min(1.0),
        srgb.alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_foreground_set_uses_sprites_32_to_56() {
        assert_eq!(RuneSlotForegroundSet::Primary.sprite_index_for_rune(0), 32);
        assert_eq!(RuneSlotForegroundSet::Primary.sprite_index_for_rune(24), 56);
    }

    #[test]
    fn alternate_foreground_set_uses_sprites_64_to_113() {
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 0 }.sprite_index_for_rune(0),
            64
        );
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 0 }.sprite_index_for_rune(24),
            88
        );
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 1 }.sprite_index_for_rune(0),
            89
        );
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 1 }.sprite_index_for_rune(24),
            113
        );
    }

    #[test]
    fn alternate_foreground_page_is_clamped_to_supported_range() {
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 99 }.sprite_index_for_rune(3),
            92
        );
    }

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        futhark::configure_futhark_keyboard(&mut app);
        configure_rune_slots(&mut app);
        app.add_message::<KeyboardInput>();
        app.add_systems(
            Update,
            (
                activate_rune_slot_on_click,
                update_active_rune_slot_from_typed_input,
                handle_backspace_in_rune_slots,
                emit_play_active_rune_word_audio_on_enter,
            )
                .chain(),
        );
        app
    }

    fn spawn_unlinked_slot(app: &mut App, interaction: Interaction) -> Entity {
        app.world_mut()
            .spawn((
                interaction,
                RuneSlot {
                    rune_index: None,
                    foreground_set: RuneSlotForegroundSet::Primary,
                },
            ))
            .id()
    }

    fn spawn_linked_slot(app: &mut App, interaction: Interaction, links: RuneSlotLinks) -> Entity {
        app.world_mut()
            .spawn((
                interaction,
                RuneSlot {
                    rune_index: None,
                    foreground_set: RuneSlotForegroundSet::Primary,
                },
                links,
            ))
            .id()
    }

    #[test]
    fn active_slot_receives_typed_rune_updates() {
        let mut app = make_test_app();

        let inactive_slot = spawn_unlinked_slot(&mut app, Interaction::None);
        let active_slot = spawn_unlinked_slot(&mut app, Interaction::Pressed);

        app.update();

        app.world_mut()
            .write_message(futhark::TypedFutharkInput('f'));
        app.world_mut()
            .write_message(futhark::TypedFutharkInput('u'));

        app.update();

        let active_rune = app
            .world()
            .entity(active_slot)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;
        let inactive_rune = app
            .world()
            .entity(inactive_slot)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;

        assert_eq!(active_rune, Some(1));
        assert_eq!(inactive_rune, None);
    }

    #[test]
    fn typing_into_linked_slot_advances_active_to_next() {
        let mut app = make_test_app();

        // Spawn slots; we'll wire links manually after knowing entity IDs.
        let slot_a = spawn_unlinked_slot(&mut app, Interaction::None);
        let slot_b = spawn_unlinked_slot(&mut app, Interaction::None);

        app.world_mut().entity_mut(slot_a).insert(RuneSlotLinks {
            prev: None,
            next: Some(slot_b),
        });
        app.world_mut().entity_mut(slot_b).insert(RuneSlotLinks {
            prev: Some(slot_a),
            next: None,
        });

        app.world_mut().resource_mut::<ActiveRuneSlot>().entity = Some(slot_a);

        app.world_mut()
            .write_message(futhark::TypedFutharkInput('f'));

        app.update();

        let active = app.world().resource::<ActiveRuneSlot>().entity;
        assert_eq!(
            active,
            Some(slot_b),
            "active should have advanced to slot_b"
        );

        let rune_a = app
            .world()
            .entity(slot_a)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;
        assert_eq!(rune_a, Some(0), "slot_a should contain 'f' (index 0)");
    }

    #[test]
    fn typing_into_last_slot_in_word_does_not_advance() {
        let mut app = make_test_app();

        let slot = spawn_linked_slot(&mut app, Interaction::None, RuneSlotLinks::default());
        app.world_mut().resource_mut::<ActiveRuneSlot>().entity = Some(slot);

        app.world_mut()
            .write_message(futhark::TypedFutharkInput('u'));

        app.update();

        let active = app.world().resource::<ActiveRuneSlot>().entity;
        assert_eq!(active, Some(slot));
    }

    #[test]
    fn backspace_clears_previous_slot_and_makes_it_active() {
        let mut app = make_test_app();

        let slot_a = spawn_unlinked_slot(&mut app, Interaction::None);
        let slot_b = spawn_unlinked_slot(&mut app, Interaction::None);

        app.world_mut().entity_mut(slot_a).insert(RuneSlotLinks {
            prev: None,
            next: Some(slot_b),
        });
        app.world_mut().entity_mut(slot_b).insert(RuneSlotLinks {
            prev: Some(slot_a),
            next: None,
        });

        // Pre-fill slot_a
        app.world_mut()
            .entity_mut(slot_a)
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = Some(0);

        app.world_mut().resource_mut::<ActiveRuneSlot>().entity = Some(slot_b);

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: bevy::input::keyboard::Key::Backspace,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            repeat: false,
            text: None,
        });

        app.update();

        let active = app.world().resource::<ActiveRuneSlot>().entity;
        assert_eq!(active, Some(slot_a), "active should move back to slot_a");

        let rune_a = app
            .world()
            .entity(slot_a)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;
        assert_eq!(rune_a, None, "slot_a should be cleared");
    }

    #[test]
    fn backspace_when_slot_has_rune_clears_current_and_stays() {
        let mut app = make_test_app();

        let slot_a = spawn_unlinked_slot(&mut app, Interaction::None);
        let slot_b = spawn_unlinked_slot(&mut app, Interaction::None);

        app.world_mut().entity_mut(slot_a).insert(RuneSlotLinks {
            prev: None,
            next: Some(slot_b),
        });
        app.world_mut().entity_mut(slot_b).insert(RuneSlotLinks {
            prev: Some(slot_a),
            next: None,
        });

        // slot_b is active and has a rune
        app.world_mut()
            .entity_mut(slot_b)
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = Some(3);
        // slot_a also has a rune (should not be touched)
        app.world_mut()
            .entity_mut(slot_a)
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = Some(0);

        app.world_mut().resource_mut::<ActiveRuneSlot>().entity = Some(slot_b);

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: bevy::input::keyboard::Key::Backspace,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            repeat: false,
            text: None,
        });

        app.update();

        let active = app.world().resource::<ActiveRuneSlot>().entity;
        assert_eq!(active, Some(slot_b), "active should remain on slot_b");

        let rune_b = app
            .world()
            .entity(slot_b)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;
        assert_eq!(rune_b, None, "slot_b rune should be cleared");

        let rune_a = app
            .world()
            .entity(slot_a)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;
        assert_eq!(rune_a, Some(0), "slot_a should not be touched");
    }

    #[test]
    fn backspace_when_slot_is_empty_moves_to_previous() {
        let mut app = make_test_app();

        let slot_a = spawn_unlinked_slot(&mut app, Interaction::None);
        let slot_b = spawn_unlinked_slot(&mut app, Interaction::None);

        app.world_mut().entity_mut(slot_a).insert(RuneSlotLinks {
            prev: None,
            next: Some(slot_b),
        });
        app.world_mut().entity_mut(slot_b).insert(RuneSlotLinks {
            prev: Some(slot_a),
            next: None,
        });

        // slot_b is active and empty; slot_a has a rune
        app.world_mut()
            .entity_mut(slot_a)
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = Some(0);

        app.world_mut().resource_mut::<ActiveRuneSlot>().entity = Some(slot_b);

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Backspace,
            logical_key: bevy::input::keyboard::Key::Backspace,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            repeat: false,
            text: None,
        });

        app.update();

        let active = app.world().resource::<ActiveRuneSlot>().entity;
        assert_eq!(active, Some(slot_a), "active should move to slot_a");

        let rune_a = app
            .world()
            .entity(slot_a)
            .get::<RuneSlot>()
            .unwrap()
            .rune_index;
        assert_eq!(rune_a, None, "slot_a should be cleared");
    }

    #[test]
    fn enter_emits_play_active_rune_word_audio_message() {
        let mut app = make_test_app();

        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Enter,
            logical_key: bevy::input::keyboard::Key::Enter,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
            repeat: false,
            text: None,
        });

        app.update();

        let reader = app
            .world_mut()
            .resource_mut::<Messages<EnterActiveRuneWord>>();
        let mut cursor = reader.get_cursor();
        assert!(
            cursor.read(&reader).next().is_some(),
            "expected a play message after Enter"
        );
    }
}
