use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::health::PlayerCombatState;
use crate::rune_words::battle::{
    BattlePhase, BattleRuneSlot, BattleSet, BattleState, LEGACY_ACTIVE_ROW_TOP, PendingRowGrading,
    RowResolved, RuneMatchState, collect_guess_submission, queue_row_grading_playback,
    reset_battle_state, score_guess_submission, spawn_battle_row, spawn_battle_row_in_container,
};
use crate::rune_words::battle_states::LastGradedWord;
use crate::rune_words::rune_slots::{ActiveRuneSlot, EnterActiveRuneWord, RuneSlot};
use crate::spellbook::SpellDef;
use crate::ui::inscribed::RuneSlotRow;
use crate::{GameAssets, dictionary};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct StartActing;

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct ActingSucceeded {
    pub matched: dictionary::Futharkation,
    pub results: Vec<RuneMatchState>,
}

#[derive(Resource, Default)]
pub struct ActingData {
    pub max_rune_count: usize,
    pub pending_success: bool,
    pub pending_matched: Option<dictionary::Futharkation>,
    pub grading_word: Option<String>,
}

pub fn configure_acting(app: &mut App) {
    app.init_resource::<ActingData>();
    app.add_message::<StartActing>();
    app.add_message::<ActingSucceeded>();
    app.add_systems(
        Update,
        (start_acting, score_acting_row_on_enter.run_if(is_acting)).chain(),
    );
    app.add_systems(
        Update,
        on_acting_row_resolved
            .run_if(is_acting)
            .in_set(BattleSet::PostAnimation),
    );
    app.add_systems(Update, refill_hand_on_acting_success);
}

fn refill_hand_on_acting_success(
    mut events: MessageReader<ActingSucceeded>,
    player: Option<ResMut<PlayerCombatState>>,
    tutorial: Option<Res<crate::tutorial::TutorialState>>,
) {
    let Some(mut player) = player else {
        return;
    };
    let in_tutorial = tutorial.map_or(false, |t| t.active);
    let mut rng = rand::rng();
    for event in events.read() {
        if player.cast_from_hand(&event.matched.word) {
            if !in_tutorial {
                player.draw(1, &mut rng);
            }
        }
    }
}

fn is_acting(state: Res<BattleState>) -> bool {
    matches!(state.phase, BattlePhase::Acting)
}

fn start_acting(
    mut commands: Commands,
    mut start_events: MessageReader<StartActing>,
    game_assets: Option<Res<GameAssets>>,
    player: Option<Res<PlayerCombatState>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    mut battle_state: ResMut<BattleState>,
    mut acting_data: ResMut<ActingData>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    row_slot_container: Query<Entity, With<RuneSlotRow>>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if start_events.read().last().is_none() {
        return;
    }

    let Some(player) = player else {
        return;
    };

    let targets = targets_from_hand(&player.hand);
    if targets.is_empty() {
        return;
    }

    let max_rune_count = targets
        .iter()
        .map(|t| t.letters.chars().count())
        .max()
        .unwrap_or(1);

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Acting;

    acting_data.max_rune_count = max_rune_count;
    acting_data.pending_success = false;
    acting_data.pending_matched = None;
    acting_data.grading_word = None;

    let row = if let Some(container) = row_slot_container.iter().next() {
        spawn_battle_row_in_container(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            max_rune_count,
            container,
        )
    } else {
        spawn_battle_row(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            max_rune_count,
            LEGACY_ACTIVE_ROW_TOP,
        )
    };

    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

fn score_acting_row_on_enter(
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut acting_data: ResMut<ActingData>,
    mut pending_grading: ResMut<PendingRowGrading>,
    player: Option<Res<PlayerCombatState>>,
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

    let Some(player) = player else {
        return;
    };
    let targets = targets_from_hand(&player.hand);

    let Some((best_target, best_results)) = find_best_match(&guess, &targets) else {
        return;
    };

    let correct = best_results
        .iter()
        .filter(|r| matches!(r, RuneMatchState::Correct))
        .count();

    active_slot.entity = None;

    queue_row_grading_playback(
        row_id,
        &battle_state.active_row_slots,
        &guess,
        &best_results,
        &mut pending_grading,
        prebaked_audio.as_deref(),
        baked_samples.as_deref(),
    );

    acting_data.grading_word = Some(best_target.word.clone());

    acting_data.pending_success = correct == best_target.letters.chars().count();
    acting_data.pending_matched = Some(best_target);

    battle_state.active_row_slots.clear();
}

fn on_acting_row_resolved(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut row_resolved: MessageReader<RowResolved>,
    mut acting_data: ResMut<ActingData>,
    mut succeeded: MessageWriter<ActingSucceeded>,
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

    // Record the resolved word for the ledger.
    last_graded_word.word = acting_data.grading_word.take();

    if acting_data.pending_success {
        let matched =
            acting_data
                .pending_matched
                .take()
                .unwrap_or_else(|| dictionary::Futharkation {
                    word: String::new(),
                    letters: String::new(),
                });
        acting_data.pending_success = false;
        battle_state.phase = BattlePhase::Idle;
        succeeded.write(ActingSucceeded {
            matched,
            results: Vec::new(),
        });
        active_slot.entity = None;
        return;
    }

    // Failure: despawn old slots, spawn a fresh row.
    for entity in existing_battle_slots.iter() {
        commands.entity(entity).despawn();
    }

    let row = if let Some(container) = row_slot_container.iter().next() {
        spawn_battle_row_in_container(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            acting_data.max_rune_count,
            container,
        )
    } else {
        spawn_battle_row(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            acting_data.max_rune_count,
            LEGACY_ACTIVE_ROW_TOP,
        )
    };
    battle_state.phase = BattlePhase::Acting;
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

pub(crate) fn find_best_match(
    guess: &[Option<char>],
    targets: &[dictionary::Futharkation],
) -> Option<(dictionary::Futharkation, Vec<RuneMatchState>)> {
    targets
        .iter()
        .map(|target| {
            let results = score_guess_submission(guess, &target.letters);
            let correct = results
                .iter()
                .filter(|r| matches!(r, RuneMatchState::Correct))
                .count();
            let present = results
                .iter()
                .filter(|r| matches!(r, RuneMatchState::Present))
                .count();
            (target.clone(), results, correct, present)
        })
        .max_by_key(|(_, _, correct, present)| (*correct, *present))
        .map(|(target, results, _, _)| (target, results))
}

fn targets_from_hand(hand: &[SpellDef]) -> Vec<dictionary::Futharkation> {
    hand.iter().map(SpellDef::as_futharkation).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futhark;
    use crate::health::PlayerCombatState;
    use crate::rune_words::battle::configure_battle;
    use crate::rune_words::rune_slots::{RuneSlot, configure_rune_slots};
    use crate::spellbook::SpellDef;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        futhark::configure_futhark_keyboard(&mut app);
        configure_rune_slots(&mut app);
        configure_battle(&mut app);
        app.add_systems(
            Update,
            crate::rune_words::rune_slots::update_active_rune_slot_from_typed_input,
        );
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
            spellbook: Handle::default(),
        });
        app.insert_resource(PlayerCombatState::default());
        app
    }

    fn fill_active_row(app: &mut App, guess: &str) {
        let slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        for (entity, letter) in slots.into_iter().zip(guess.chars()) {
            app.world_mut()
                .entity_mut(entity)
                .get_mut::<RuneSlot>()
                .expect("slot")
                .rune_index = futhark::letter_to_index(letter);
        }
    }

    fn active_row_letters(app: &mut App) -> Vec<Option<char>> {
        let slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        slots
            .into_iter()
            .map(|entity| {
                app.world()
                    .entity(entity)
                    .get::<RuneSlot>()
                    .and_then(|slot| slot.rune_index)
                    .and_then(futhark::index_to_letter)
            })
            .collect()
    }

    fn futha(word: &str, letters: &str) -> dictionary::Futharkation {
        dictionary::Futharkation {
            word: word.to_string(),
            letters: letters.to_string(),
        }
    }

    fn set_hand_from_targets(app: &mut App, targets: &[dictionary::Futharkation]) {
        let mut player = app.world_mut().resource_mut::<PlayerCombatState>();
        player.hand = targets
            .iter()
            .map(|target| SpellDef {
                word: target.word.clone(),
                effects: Vec::new(),
                futharkation: target.letters.clone(),
                starter: true,
            })
            .collect();
    }

    fn start_acting_with_targets(app: &mut App, targets: Vec<dictionary::Futharkation>) {
        set_hand_from_targets(app, &targets);
        app.world_mut().write_message(StartActing);
    }

    // --- unit tests for find_best_match ---

    #[test]
    fn find_best_match_returns_none_for_empty_targets() {
        let guess: Vec<Option<char>> = vec![Some('f'), Some('u')];
        assert!(find_best_match(&guess, &[]).is_none());
    }

    #[test]
    fn find_best_match_picks_target_with_most_correct() {
        let guess: Vec<Option<char>> = vec![Some('f'), Some('u'), Some('t')];
        let targets = vec![futha("a", "fxx"), futha("b", "fut")];
        let (matched, results) = find_best_match(&guess, &targets).unwrap();
        assert_eq!(matched.word, "b");
        assert!(results.iter().all(|r| matches!(r, RuneMatchState::Correct)));
    }

    #[test]
    fn find_best_match_breaks_ties_by_present_count() {
        // "fax" vs "fut": guess "fut" → "fax" gives 1 correct (f) + 0 present,
        // "fut" gives 3 correct. Let's test present tiebreaker instead:
        // guess "fab", targets "fax" (2 correct: f,a) vs "fbx" (1 correct: f, 1 present: b)
        let guess: Vec<Option<char>> = vec![Some('f'), Some('a'), Some('b')];
        let targets = vec![futha("fax", "fax"), futha("fbx", "fbx")];
        let (matched, _) = find_best_match(&guess, &targets).unwrap();
        assert_eq!(matched.word, "fax");
    }

    #[test]
    fn find_best_match_prefers_correct_over_present() {
        // guess "abc", target1 "abc" (3 correct), target2 "bca" (0 correct, 3 present)
        let guess: Vec<Option<char>> = vec![Some('a'), Some('b'), Some('c')];
        let targets = vec![futha("bca", "bca"), futha("abc", "abc")];
        let (matched, _) = find_best_match(&guess, &targets).unwrap();
        assert_eq!(matched.word, "abc");
    }

    // --- app tests ---

    #[test]
    fn start_acting_spawns_slots_for_longest_target() {
        let mut app = make_test_app();
        start_acting_with_targets(
            &mut app,
            vec![
                futha("aa", "fu"),
                futha("bbbbb", "futar"),
                futha("ccc", "fut"),
            ],
        );
        app.update();
        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.active_row_slots.len(),
            5,
            "slots = longest target length"
        );
        assert_eq!(state.phase, BattlePhase::Acting);
    }

    #[test]
    fn start_acting_ignores_empty_targets() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![]);
        app.update();
        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "empty targets should not start acting"
        );
    }

    #[test]
    fn acting_full_match_marks_pending_success() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        fill_active_row(&mut app, "futar");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(
            acting_data.pending_success,
            "full match should set pending_success"
        );
    }

    #[test]
    fn acting_success_keeps_accepting_book_words() {
        let mut app = make_test_app();
        let targets = vec![futha("fable", "futar"), futha("dune", "dune")];
        start_acting_with_targets(&mut app, targets.clone());
        app.update();

        fill_active_row(&mut app, "futar");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(acting_data.pending_success);
        assert_eq!(
            app.world().resource::<PlayerCombatState>().hand.len(),
            targets.len(),
            "success should not mutate the hand before acting succeeds"
        );
    }

    #[test]
    fn acting_partial_match_does_not_set_pending_success() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        // 3/5 correct should still fail under exact-match success.
        fill_active_row(&mut app, "futxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(!acting_data.pending_success);
    }

    #[test]
    fn acting_success_sends_succeeded_and_transitions_to_idle() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        fill_active_row(&mut app, "futar");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        // advance past animation
        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "phase should return to Idle after success"
        );
    }

    #[test]
    fn acting_failure_spawns_new_row_and_stays_acting() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        // only 1 correct → failure
        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Acting,
            "phase should stay Acting after failure"
        );
        assert_eq!(state.active_row_slots.len(), 5, "new row should be spawned");
    }

    #[test]
    fn acting_failure_increments_resolved_count() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(state.resolved_rows, 1, "one row should have been resolved");
        assert_eq!(state.active_row_slots.len(), 5, "new row should be spawned");
    }

    #[test]
    fn acting_failure_queues_typed_input_for_next_row_in_order() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        // Force failure (partial match).
        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        app.world_mut()
            .write_message(futhark::TypedFutharkInput('u'));
        app.world_mut()
            .write_message(futhark::TypedFutharkInput('t'));
        app.world_mut()
            .write_message(futhark::TypedFutharkInput('a'));

        for _ in 0..16 {
            app.update();
        }

        let row = active_row_letters(&mut app);
        assert_eq!(row[0], Some('u'));
        assert_eq!(row[1], Some('t'));
        assert_eq!(row[2], Some('a'));
    }

    #[test]
    fn acting_success_ignores_typed_input_during_grading_animation() {
        let mut app = make_test_app();
        start_acting_with_targets(&mut app, vec![futha("fable", "futar")]);
        app.update();

        fill_active_row(&mut app, "futar");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        app.world_mut()
            .write_message(futhark::TypedFutharkInput('u'));
        app.world_mut()
            .write_message(futhark::TypedFutharkInput('t'));

        for _ in 0..12 {
            app.update();
        }

        assert_eq!(
            app.world().resource::<BattleState>().phase,
            BattlePhase::Idle
        );

        // Start a fresh acting round and verify no buffered input leaked through.
        start_acting_with_targets(&mut app, vec![futha("again", "futar")]);
        app.update();

        let row = active_row_letters(&mut app);
        assert!(row.into_iter().all(|letter| letter.is_none()));
    }
}
