use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::rune_words::battle::{
    BattlePhase, BattleRuneSlot, BattleSet, BattleState, LEGACY_ACTIVE_ROW_TOP, PendingRowGrading,
    RowLetterGraded, RowResolved, RuneMatchState, collect_guess_submission,
    queue_row_grading_playback, reset_battle_state, score_guess_submission, spawn_battle_row,
    spawn_battle_row_in_container,
};
use crate::rune_words::battle_states::LastGradedWord;
use crate::rune_words::rune_slots::{ActiveRuneSlot, EnterActiveRuneWord, RuneSlot};
use crate::ui::inscribed::RuneSlotRow;
use crate::{GameAssets, dictionary};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct StartBinding(pub dictionary::Futharkation);

#[derive(bevy::ecs::message::Message, Clone, Debug, Default)]
pub struct BindingSucceeded;

#[derive(Resource, Default)]
pub struct BindingData {
    pub target: Option<dictionary::Futharkation>,
    pub pending_success: bool,
    pending_eliminations_by_row: HashMap<u32, HashSet<char>>,
}

pub fn configure_binding(app: &mut App) {
    app.init_resource::<BindingData>();
    app.add_message::<StartBinding>();
    app.add_message::<BindingSucceeded>();
    app.add_systems(
        Update,
        (start_binding, score_binding_row_on_enter.run_if(is_binding)).chain(),
    );
    app.add_systems(
        Update,
        (
            apply_binding_key_eliminations_from_grading,
            on_binding_row_resolved,
        )
            .chain()
            .run_if(is_binding)
            .in_set(BattleSet::PostAnimation),
    );
}

fn letters_eliminated_in_row(guess: &[Option<char>], results: &[RuneMatchState]) -> HashSet<char> {
    let mut missing_letters = HashSet::new();
    let mut non_missing_letters = HashSet::new();

    for (guess_letter, result) in guess.iter().copied().zip(results.iter().copied()) {
        let Some(guess_letter) = guess_letter else {
            continue;
        };

        match result {
            RuneMatchState::Missing => {
                missing_letters.insert(guess_letter);
            }
            RuneMatchState::Present | RuneMatchState::Correct => {
                non_missing_letters.insert(guess_letter);
            }
        }
    }

    missing_letters
        .difference(&non_missing_letters)
        .copied()
        .collect()
}

fn is_binding(state: Res<BattleState>) -> bool {
    matches!(state.phase, BattlePhase::Binding)
}

fn start_binding(
    mut commands: Commands,
    mut start_events: MessageReader<StartBinding>,
    game_assets: Option<Res<GameAssets>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    mut battle_state: ResMut<BattleState>,
    mut binding_data: ResMut<BindingData>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    eliminated_keys: Option<ResMut<crate::futhark::EliminatedFutharkKeys>>,
    row_slot_container: Query<Entity, With<RuneSlotRow>>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(StartBinding(target)) = start_events.read().last().cloned() else {
        return;
    };

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Binding;
    binding_data.target = Some(target.clone());
    binding_data.pending_eliminations_by_row.clear();
    if let Some(mut eliminated_keys) = eliminated_keys {
        eliminated_keys.clear();
    }

    let rune_count = target.letters.chars().count();
    let row = if let Some(container) = row_slot_container.iter().next() {
        spawn_battle_row_in_container(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            container,
        )
    } else {
        spawn_battle_row(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            LEGACY_ACTIVE_ROW_TOP,
        )
    };
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

fn score_binding_row_on_enter(
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut binding_data: ResMut<BindingData>,
    mut pending_grading: ResMut<PendingRowGrading>,
    rune_slots: Query<&RuneSlot>,
    battle_slots: Query<&BattleRuneSlot>,
    prebaked_audio: Option<Res<crate::futhark::PrebakedFutharkConversationalAudio>>,
    baked_samples: Option<Res<crate::futhark::BakedAudioSamples>>,
) {
    if enter_events.is_empty() {
        return;
    }
    enter_events.clear();

    if battle_state.pending_resolved_row.is_some() || pending_grading.is_active() {
        return;
    }

    let Some(target) = binding_data.target.clone() else {
        return;
    };
    let Some(guess) = collect_guess_submission(&battle_state.active_row_slots, &rune_slots) else {
        return;
    };

    let Some(first_entity) = battle_state.active_row_slots.first().copied() else {
        return;
    };
    let Ok(battle_slot) = battle_slots.get(first_entity) else {
        return;
    };
    let row_id = battle_slot.row_id;

    let results = score_guess_submission(&guess, &target.letters);
    let letters_to_eliminate = letters_eliminated_in_row(&guess, &results);
    let all_correct = results.iter().all(|r| matches!(r, RuneMatchState::Correct));

    queue_row_grading_playback(
        row_id,
        &battle_state.active_row_slots,
        &guess,
        &results,
        &mut pending_grading,
        prebaked_audio.as_deref(),
        baked_samples.as_deref(),
    );

    if !letters_to_eliminate.is_empty() {
        binding_data
            .pending_eliminations_by_row
            .insert(row_id, letters_to_eliminate);
    }

    active_slot.entity = None;

    binding_data.pending_success = all_correct;
    battle_state.active_row_slots.clear();
}

fn apply_binding_key_eliminations_from_grading(
    mut graded_events: MessageReader<RowLetterGraded>,
    mut binding_data: ResMut<BindingData>,
    eliminated_keys: Option<ResMut<crate::futhark::EliminatedFutharkKeys>>,
) {
    let Some(mut eliminated_keys) = eliminated_keys else {
        return;
    };

    for graded in graded_events.read() {
        if graded.match_state != RuneMatchState::Missing {
            continue;
        }

        let mut remove_row_entry = false;
        if let Some(letters) = binding_data
            .pending_eliminations_by_row
            .get_mut(&graded.row_id)
        {
            if letters.remove(&graded.letter) {
                eliminated_keys.insert(graded.letter);
            }
            remove_row_entry = letters.is_empty();
        }

        if remove_row_entry {
            binding_data
                .pending_eliminations_by_row
                .remove(&graded.row_id);
        }
    }
}

fn on_binding_row_resolved(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut row_resolved: MessageReader<RowResolved>,
    mut binding_data: ResMut<BindingData>,
    mut succeeded: MessageWriter<BindingSucceeded>,
    existing_battle_slots: Query<Entity, With<BattleRuneSlot>>,
    row_slot_container: Query<Entity, With<RuneSlotRow>>,
    mut last_graded_word: ResMut<LastGradedWord>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if row_resolved.read().count() == 0 {
        return;
    }

    let Some(target) = binding_data.target.clone() else {
        return;
    };

    // Record word for the ledger.
    last_graded_word.word = Some(target.word.clone());

    if binding_data.pending_success {
        binding_data.pending_success = false;
        battle_state.phase = BattlePhase::Idle;
        active_slot.entity = None;
        succeeded.write(BindingSucceeded);
        return;
    }

    // Failure: despawn old slots, spawn fresh row.
    for entity in existing_battle_slots.iter() {
        commands.entity(entity).despawn();
    }

    let rune_count = target.letters.chars().count();
    let row = if let Some(container) = row_slot_container.iter().next() {
        spawn_battle_row_in_container(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            container,
        )
    } else {
        spawn_battle_row(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            LEGACY_ACTIVE_ROW_TOP,
        )
    };
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futhark;
    use crate::rune_words::battle::{PendingRowGrading, RuneMatchState, configure_battle};
    use crate::rune_words::rune_slots::{RuneSlotBackground, configure_rune_slots};
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    #[test]
    fn duplicate_letters_are_not_eliminated_if_any_copy_is_non_missing() {
        let guess = vec![Some('r'), Some('r')];
        let results = vec![RuneMatchState::Missing, RuneMatchState::Correct];

        let eliminated = letters_eliminated_in_row(&guess, &results);
        assert!(eliminated.is_empty());
    }

    #[test]
    fn missing_letters_without_non_missing_copy_are_eliminated() {
        let guess = vec![Some('k'), Some('r'), Some('r')];
        let results = vec![
            RuneMatchState::Missing,
            RuneMatchState::Missing,
            RuneMatchState::Correct,
        ];

        let eliminated = letters_eliminated_in_row(&guess, &results);
        assert_eq!(eliminated, HashSet::from(['k']));
    }

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        configure_rune_slots(&mut app);
        configure_battle(&mut app);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.insert_resource(crate::GameAssets {
            futhark: Handle::default(),
            futhark_layout: Handle::default(),
            futhark_sounds: Vec::new(),
            futhark_sound_params: Handle::default(),
            futhark_conversational_params: Handle::default(),
            backdrop: Handle::default(),
            parchment_tile: Handle::default(),
            corner_bracket: Handle::default(),
            vignette: Handle::default(),
            sigils: Handle::default(),
            sigils_layout: Handle::default(),
            goblin: Handle::default(),
            goblin_layout: Handle::default(),
            robed: Handle::default(),
            robed_layout: Handle::default(),
            font_cormorant_unicase_semibold: Handle::default(),
            font_cormorant_unicase_bold: Handle::default(),
            font_cormorant_garamond_italic: Handle::default(),
            font_im_fell_sc: Handle::default(),
            font_unifraktur: Handle::default(),
            goblin_spec: Handle::default(),
            robed_spec: Handle::default(),
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
    fn start_binding_spawns_one_slot_per_target_rune() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartBinding(dictionary::Futharkation {
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
        app.world_mut().write_message(StartBinding(target));
        app.update();

        let submitted_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        fill_active_row(&mut app, "fukkk");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let mut saw_animation = false;
        for _ in 0..24 {
            let is_animating = app
                .world()
                .resource::<BattleState>()
                .pending_resolved_row
                .is_some();
            if is_animating {
                saw_animation = true;
                break;
            }
            app.update();
        }

        assert!(
            saw_animation,
            "row should begin animating after typed letters are graded"
        );

        let first_bg_color = {
            let world = app.world();
            let children = world
                .entity(*submitted_row_slots.first().unwrap())
                .get::<Children>()
                .unwrap();
            let mut color = None;
            for child in children.iter() {
                if let Some(bg) = world.entity(child).get::<RuneSlotBackground>() {
                    color = Some(bg.base_color);
                    break;
                }
            }
            color.unwrap()
        };
        assert_eq!(first_bg_color, RuneMatchState::Correct.background_color());

        for _ in 0..24 {
            if app
                .world()
                .resource::<BattleState>()
                .pending_resolved_row
                .is_none()
                && !app.world().resource::<PendingRowGrading>().is_active()
                && !app
                    .world()
                    .resource::<BattleState>()
                    .active_row_slots
                    .is_empty()
            {
                break;
            }
            app.update();
        }
        let battle_state = app.world().resource::<BattleState>();
        assert!(battle_state.pending_resolved_row.is_none());
        assert_eq!(battle_state.active_row_slots.len(), 5);
    }

    #[test]
    fn multiple_failed_submissions_increment_resolved_count() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartBinding(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));
        app.update();

        // Submit first row (partial)
        let first_slot = app.world().resource::<BattleState>().active_row_slots[0];
        app.world_mut()
            .entity_mut(first_slot)
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = futhark::letter_to_index('f');

        app.world_mut().write_message(EnterActiveRuneWord);
        for _ in 0..30 {
            app.update();
            if !app
                .world()
                .resource::<BattleState>()
                .active_row_slots
                .is_empty()
            {
                break;
            }
        }

        // Submit second row (partial)
        let second_slot = app.world().resource::<BattleState>().active_row_slots[0];
        app.world_mut()
            .entity_mut(second_slot)
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = futhark::letter_to_index('u');

        app.world_mut().write_message(EnterActiveRuneWord);
        for _ in 0..30 {
            app.update();
            if app
                .world()
                .resource::<BattleState>()
                .pending_resolved_row
                .is_none()
                && !app.world().resource::<PendingRowGrading>().is_active()
            {
                break;
            }
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(state.resolved_rows, 2, "two rows should have been resolved");
        assert_eq!(state.active_row_slots.len(), 5, "fresh row should be ready");
    }

    #[test]
    fn binding_exact_match_sets_pending_success() {
        let mut app = make_test_app();
        let target = dictionary::Futharkation {
            word: "fable".to_string(),
            letters: "futar".to_string(),
        };
        app.world_mut().write_message(StartBinding(target));
        app.update();

        fill_active_row(&mut app, "futar");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        assert!(
            app.world().resource::<BindingData>().pending_success,
            "exact match should set pending_success"
        );
    }

    #[test]
    fn binding_partial_match_does_not_set_pending_success() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartBinding(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));
        app.update();

        fill_active_row(&mut app, "futxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        assert!(
            !app.world().resource::<BindingData>().pending_success,
            "partial match should not set pending_success"
        );
    }

    #[test]
    fn binding_success_transitions_to_idle_and_spawns_no_new_row() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartBinding(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));
        app.update();

        fill_active_row(&mut app, "futar");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        for _ in 0..30 {
            if app.world().resource::<BattleState>().phase == BattlePhase::Idle {
                break;
            }
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "phase should be Idle after binding success"
        );
        assert!(
            state.active_row_slots.is_empty(),
            "no new row should be spawned after success"
        );
    }
}
