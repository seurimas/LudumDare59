use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{GameAssets, dictionary, futhark};

use super::rune_slots::{
    ActiveRuneSlot, PlayActiveRuneWordAudio, RuneSlot, RuneSlotBackground, RuneSlotConfig,
    RuneSlotForegroundSet, spawn_rune_word,
};

const ACTIVE_ROW_TOP: f32 = 220.0;
const ROW_RISE: f32 = 72.0;
const ROW_CENTER_LEFT: f32 = 240.0;
const SLOT_SPACING: f32 = 68.0;
const SLOT_SIZE: f32 = 48.0;
const ROW_RISE_DURATION_SECONDS: f32 = 0.5;

#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct StartBattle(pub dictionary::Futharkation);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuneMatchState {
    Missing,
    Present,
    Correct,
}

impl RuneMatchState {
    fn background_color(self) -> Color {
        match self {
            Self::Missing => Color::srgb(0.78, 0.2, 0.2),
            Self::Present => Color::srgb(0.85, 0.72, 0.16),
            Self::Correct => Color::srgb(0.24, 0.68, 0.32),
        }
    }
}

#[derive(Resource, Default)]
struct BattleState {
    target: Option<dictionary::Futharkation>,
    active_row_slots: Vec<Entity>,
    pending_resolved_row: Option<u32>,
    pending_settle_frames: u8,
    next_row_id: u32,
    resolved_rows: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct BattleRuneSlot {
    row_id: u32,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct BattleRowMotion {
    start_top: f32,
    end_top: f32,
    elapsed_seconds: f32,
}

pub fn configure_battle(app: &mut App) {
    app.init_resource::<BattleState>();
    app.add_message::<StartBattle>();
    app.add_systems(
        Update,
        (
            start_battle,
            score_active_row_on_enter,
            animate_resolved_rows,
            finish_resolved_rows,
        )
            .chain(),
    );
}

fn start_battle(
    mut commands: Commands,
    mut start_battle: MessageReader<StartBattle>,
    game_assets: Option<Res<GameAssets>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };

    let Some(StartBattle(target)) = start_battle.read().last().cloned() else {
        return;
    };

    for entity in &existing_rows {
        commands.entity(entity).despawn();
    }

    battle_state.target = Some(target.clone());
    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = None;
    battle_state.pending_settle_frames = 0;
    battle_state.next_row_id = 0;
    battle_state.resolved_rows = 0;

    let active_row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        target.letters.chars().count(),
        ACTIVE_ROW_TOP,
    );
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = active_row.clone();
    active_slot.entity = active_row.first().copied();
}

fn score_active_row_on_enter(
    mut commands: Commands,
    mut play_events: MessageReader<PlayActiveRuneWordAudio>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut slots: Query<(Entity, &BattleRuneSlot, &Children, &mut Node), With<RuneSlot>>,
    mut backgrounds: Query<&mut RuneSlotBackground>,
    rune_slots: Query<&RuneSlot>,
) {
    if play_events.is_empty() {
        return;
    }
    play_events.clear();

    if battle_state.pending_resolved_row.is_some() {
        return;
    }

    let Some(target) = battle_state.target.clone() else {
        return;
    };

    let Some(guess) = collect_guess_submission(&battle_state.active_row_slots, &rune_slots) else {
        return;
    };

    let Some(first_slot) = battle_state.active_row_slots.first().copied() else {
        return;
    };
    let Ok(battle_slot) = slots.get(first_slot) else {
        return;
    };
    let row_id = battle_slot.1.row_id;
    let active_slot_entities: HashSet<Entity> =
        battle_state.active_row_slots.iter().copied().collect();
    let results = score_guess_submission(&guess, &target.letters);

    active_slot.entity = None;

    for (entity, result) in battle_state
        .active_row_slots
        .iter()
        .copied()
        .zip(results.into_iter())
    {
        let Ok((_, _, children, node)) = slots.get_mut(entity) else {
            continue;
        };

        for child in children.iter() {
            if let Ok(mut background) = backgrounds.get_mut(child) {
                background.base_color = result.background_color();
            }
        }

        let start_top = match node.top {
            Val::Px(top) => top,
            _ => ACTIVE_ROW_TOP,
        };
        commands.entity(entity).insert(BattleRowMotion {
            start_top,
            end_top: start_top - ROW_RISE,
            elapsed_seconds: 0.0,
        });
    }

    for (entity, _, _, node) in &mut slots {
        if active_slot_entities.contains(&entity) {
            continue;
        }

        let start_top = match node.top {
            Val::Px(top) => top,
            _ => ACTIVE_ROW_TOP,
        };
        commands.entity(entity).insert(BattleRowMotion {
            start_top,
            end_top: start_top - ROW_RISE,
            elapsed_seconds: 0.0,
        });
    }

    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = Some(row_id);
    battle_state.pending_settle_frames = 1;
}

fn animate_resolved_rows(
    mut commands: Commands,
    time: Res<Time>,
    mut moving_rows: Query<(Entity, &mut Node, &mut BattleRowMotion)>,
) {
    for (entity, mut node, mut motion) in &mut moving_rows {
        motion.elapsed_seconds += time.delta_secs();
        let progress = (motion.elapsed_seconds / ROW_RISE_DURATION_SECONDS).clamp(0.0, 1.0);
        let top = motion.start_top + (motion.end_top - motion.start_top) * progress;
        node.top = Val::Px(top);

        if progress >= 1.0 {
            commands.entity(entity).remove::<BattleRowMotion>();
        }
    }
}

fn finish_resolved_rows(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    row_slots: Query<(Entity, &BattleRuneSlot)>,
    moving_rows: Query<(&BattleRuneSlot, Option<&BattleRowMotion>)>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };

    let Some(row_id) = battle_state.pending_resolved_row else {
        return;
    };

    if battle_state.pending_settle_frames > 0 {
        battle_state.pending_settle_frames -= 1;
        return;
    }

    let still_animating = moving_rows
        .iter()
        .any(|(slot, motion)| slot.row_id == row_id && motion.is_some());
    if still_animating {
        return;
    }

    for (entity, slot) in &row_slots {
        if slot.row_id == row_id {
            commands.entity(entity).remove::<(Button, Interaction)>();
        }
    }

    let Some(target) = battle_state.target.clone() else {
        battle_state.pending_resolved_row = None;
        return;
    };

    let next_row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        target.letters.chars().count(),
        ACTIVE_ROW_TOP,
    );

    battle_state.next_row_id += 1;
    battle_state.resolved_rows += 1;
    battle_state.pending_resolved_row = None;
    battle_state.pending_settle_frames = 0;
    battle_state.active_row_slots = next_row.clone();
    active_slot.entity = next_row.first().copied();
}

fn spawn_battle_row(
    commands: &mut Commands,
    game_assets: &GameAssets,
    row_id: u32,
    rune_count: usize,
    top: f32,
) -> Vec<Entity> {
    let start_left = ROW_CENTER_LEFT - (rune_count.saturating_sub(1) as f32 * SLOT_SPACING * 0.5);
    let configs = (0..rune_count)
        .map(|index| RuneSlotConfig {
            left: Val::Px(start_left + index as f32 * SLOT_SPACING),
            top: Val::Px(top),
            size: SLOT_SIZE,
            background_color: idle_row_color(),
            foreground_set: RuneSlotForegroundSet::Primary,
            initial_rune: None,
        })
        .collect();

    let slots = spawn_rune_word(commands, game_assets, configs);
    for entity in &slots {
        commands.entity(*entity).insert(BattleRuneSlot { row_id });
    }
    slots
}

fn collect_guess_submission(
    slot_entities: &[Entity],
    slots: &Query<&RuneSlot>,
) -> Option<Vec<Option<char>>> {
    let mut guess = Vec::with_capacity(slot_entities.len());
    let mut has_any_rune = false;

    for entity in slot_entities {
        let slot = slots.get(*entity).ok()?;
        let guess_letter = slot
            .rune_index
            .and_then(|rune_index| futhark::LETTERS.get(rune_index).copied());
        has_any_rune |= guess_letter.is_some();
        guess.push(guess_letter);
    }

    has_any_rune.then_some(guess)
}

fn idle_row_color() -> Color {
    Color::srgb(0.16, 0.28, 0.76)
}

pub fn score_guess(guess: &str, target: &str) -> Vec<RuneMatchState> {
    let guess_chars: Vec<Option<char>> = guess.chars().map(Some).collect();
    score_guess_submission(&guess_chars, target)
}

fn score_guess_submission(guess: &[Option<char>], target: &str) -> Vec<RuneMatchState> {
    let target_chars: Vec<char> = target.chars().collect();
    let mut results = vec![RuneMatchState::Missing; guess.len()];
    let mut remaining_target_counts = HashMap::<char, usize>::new();

    for (index, target_char) in target_chars.iter().copied().enumerate() {
        if guess.get(index) == Some(&Some(target_char)) {
            results[index] = RuneMatchState::Correct;
        } else {
            *remaining_target_counts.entry(target_char).or_default() += 1;
        }
    }

    for (index, guess_char) in guess.iter().copied().enumerate() {
        if results[index] == RuneMatchState::Correct {
            continue;
        }

        let Some(guess_char) = guess_char else {
            continue;
        };

        let Some(count) = remaining_target_counts.get_mut(&guess_char) else {
            continue;
        };
        if *count == 0 {
            continue;
        }

        results[index] = RuneMatchState::Present;
        *count -= 1;
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        super::super::rune_slots::configure_rune_slots(&mut app);
        configure_battle(&mut app);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.insert_resource(GameAssets {
            futhark: Handle::default(),
            futhark_layout: Handle::default(),
            futhark_sounds: Vec::new(),
            futhark_sound_params: Handle::default(),
            futhark_conversational_params: Handle::default(),
        });
        app
    }

    fn fill_active_row(app: &mut App, guess: &str) {
        let active_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        for (entity, letter) in active_row_slots.into_iter().zip(guess.chars()) {
            app.world_mut()
                .entity_mut(entity)
                .get_mut::<RuneSlot>()
                .expect("slot")
                .rune_index = futhark::letter_to_index(letter);
        }
    }

    #[test]
    fn score_guess_handles_duplicate_letters() {
        assert_eq!(
            score_guess("aaccc", "abaca"),
            vec![
                RuneMatchState::Correct,
                RuneMatchState::Present,
                RuneMatchState::Missing,
                RuneMatchState::Correct,
                RuneMatchState::Missing,
            ]
        );
    }

    #[test]
    fn start_battle_spawns_one_slot_per_target_rune() {
        let mut app = make_test_app();

        app.world_mut()
            .write_message(StartBattle(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));

        app.update();

        let battle_state = app.world().resource::<BattleState>();
        assert_eq!(battle_state.active_row_slots.len(), 5);
        assert!(battle_state.pending_resolved_row.is_none());
    }

    #[test]
    fn submit_guess_colors_row_and_spawns_fresh_row_after_animation() {
        let mut app = make_test_app();

        let target = dictionary::Futharkation {
            word: "fable".to_string(),
            letters: "futar".to_string(),
        };
        app.world_mut().write_message(StartBattle(target));
        app.update();

        let submitted_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        fill_active_row(&mut app, "fukkk");
        app.world_mut().write_message(PlayActiveRuneWordAudio);
        app.update();

        let resolved_row = app.world().resource::<BattleState>().pending_resolved_row;
        assert!(
            resolved_row.is_some(),
            "row should be animating after Enter"
        );

        let first_background_color = {
            let children = app
                .world()
                .entity(*submitted_row_slots.first().expect("first row slot"))
                .get::<Children>()
                .expect("children");

            let mut color = None;
            for child in children.iter() {
                if let Some(background) = app.world().entity(child).get::<RuneSlotBackground>() {
                    color = Some(background.base_color);
                    break;
                }
            }
            color.expect("background color")
        };
        assert_eq!(
            first_background_color,
            RuneMatchState::Correct.background_color()
        );

        for _ in 0..5 {
            app.update();
        }

        let battle_state = app.world().resource::<BattleState>();
        assert!(battle_state.pending_resolved_row.is_none());
        assert_eq!(battle_state.active_row_slots.len(), 5);
    }

    #[test]
    fn incomplete_submission_pushes_previous_rows_up() {
        let mut app = make_test_app();

        app.world_mut()
            .write_message(StartBattle(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));
        app.update();

        let first_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        app.world_mut()
            .entity_mut(first_row_slots[0])
            .get_mut::<RuneSlot>()
            .expect("slot")
            .rune_index = futhark::letter_to_index('f');

        app.world_mut().write_message(PlayActiveRuneWordAudio);
        app.update();
        for _ in 0..5 {
            app.update();
        }

        let second_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        app.world_mut()
            .entity_mut(second_row_slots[0])
            .get_mut::<RuneSlot>()
            .expect("slot")
            .rune_index = futhark::letter_to_index('u');

        app.world_mut().write_message(PlayActiveRuneWordAudio);
        app.update();
        for _ in 0..5 {
            app.update();
        }

        let first_row_top = app
            .world()
            .entity(first_row_slots[0])
            .get::<Node>()
            .expect("node")
            .top;
        let second_row_top = app
            .world()
            .entity(second_row_slots[0])
            .get::<Node>()
            .expect("node")
            .top;

        assert_eq!(first_row_top, Val::Px(ACTIVE_ROW_TOP - ROW_RISE * 2.0));
        assert_eq!(second_row_top, Val::Px(ACTIVE_ROW_TOP - ROW_RISE));
    }
}
