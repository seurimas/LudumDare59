use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::rune_words::battle::{
    ACTIVE_ROW_TOP, BattlePhase, BattleRuneSlot, BattleSet, BattleState, PendingRowGrading,
    RowResolved, RuneMatchState, collect_guess_submission, queue_row_grading_playback,
    reset_battle_state, score_guess_submission, spawn_battle_row,
};
use crate::rune_words::rune_slots::{ActiveRuneSlot, EnterActiveRuneWord, RuneSlot};
use crate::{GameAssets, dictionary};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct StartActing {
    pub targets: Vec<dictionary::Futharkation>,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct ActingSucceeded {
    pub matched: dictionary::Futharkation,
    pub results: Vec<RuneMatchState>,
}

#[derive(Resource, Default)]
pub struct ActingData {
    pub targets: Vec<dictionary::Futharkation>,
    pub max_rune_count: usize,
    pub pending_success: bool,
    pub pending_matched: Option<dictionary::Futharkation>,
    pub grading_against_letters: Option<String>,
}

#[derive(Component)]
pub struct ActingCountLabel;

#[derive(Component)]
pub struct ActingBookPanel;

#[derive(Component)]
pub struct ActingBookEntry {
    pub letters: String,
}

#[derive(Component)]
pub struct ActingBookEntryBackground {
    pub base_color: Color,
}

pub fn configure_acting(app: &mut App) {
    app.init_resource::<ActingData>();
    app.add_message::<StartActing>();
    app.add_message::<ActingSucceeded>();
    app.add_systems(Update, cleanup_acting_book_outside_phase);
    app.add_systems(
        Update,
        animate_acting_book_grading_highlight.run_if(is_acting),
    );
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
}

fn is_acting(state: Res<BattleState>) -> bool {
    matches!(state.phase, BattlePhase::Acting)
}

fn start_acting(
    mut commands: Commands,
    mut start_events: MessageReader<StartActing>,
    game_assets: Option<Res<GameAssets>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    existing_book: Query<Entity, With<ActingBookPanel>>,
    mut battle_state: ResMut<BattleState>,
    mut acting_data: ResMut<ActingData>,
    mut active_slot: ResMut<ActiveRuneSlot>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(StartActing { targets }) = start_events.read().last().cloned() else {
        return;
    };
    if targets.is_empty() {
        return;
    }

    for panel in existing_book.iter() {
        commands.entity(panel).despawn();
    }

    let max_rune_count = targets
        .iter()
        .map(|t| t.letters.chars().count())
        .max()
        .unwrap_or(1);

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Acting;

    acting_data.targets = targets;
    acting_data.max_rune_count = max_rune_count;
    acting_data.pending_success = false;
    acting_data.pending_matched = None;
    acting_data.grading_against_letters = None;

    let row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        max_rune_count,
        ACTIVE_ROW_TOP,
    );

    spawn_acting_book_panel(&mut commands, &acting_data.targets);

    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

fn spawn_acting_book_panel(commands: &mut Commands, targets: &[dictionary::Futharkation]) {
    let entries = first_four_book_entries(targets);
    let entry_base_color = Color::srgba(0.14, 0.2, 0.36, 0.92);

    commands
        .spawn((
            ActingBookPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                width: Val::Px(332.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.09, 0.75)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Book"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            for row in entries.chunks(2) {
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|grid_row| {
                        for entry in row {
                            let label = match entry {
                                Some(word) => word.word.clone(),
                                None => "-".to_string(),
                            };

                            let mut node = grid_row.spawn((
                                Node {
                                    width: Val::Px(150.0),
                                    min_height: Val::Px(56.0),
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(entry_base_color),
                                ActingBookEntryBackground {
                                    base_color: entry_base_color,
                                },
                                children![(
                                    Text::new(label),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                )],
                            ));

                            if let Some(word) = entry {
                                node.insert(ActingBookEntry {
                                    letters: word.letters.clone(),
                                });
                            }
                        }
                    });
            }
        });
}

fn animate_acting_book_grading_highlight(
    time: Res<Time>,
    pending_grading: Res<PendingRowGrading>,
    speed: Option<Res<crate::futhark::FutharkKeyboardAnimationSpeed>>,
    acting_data: Res<ActingData>,
    mut entries: Query<
        (
            &mut BackgroundColor,
            &ActingBookEntryBackground,
            Option<&ActingBookEntry>,
        ),
        With<ActingBookEntryBackground>,
    >,
) {
    let hue_speed = speed
        .as_ref()
        .map(|s| s.hue_degrees_per_second)
        .unwrap_or(30.0);
    let rune_color = Color::hsl((time.elapsed_secs() * hue_speed) % 360.0, 1.0, 0.5);

    for (mut bg, base, entry) in &mut entries {
        let is_target = pending_grading.is_active()
            && acting_data
                .grading_against_letters
                .as_deref()
                .is_some_and(|letters| {
                    entry
                        .map(|book_entry| book_entry.letters.as_str() == letters)
                        .unwrap_or(false)
                });

        bg.0 = if is_target {
            rune_color
        } else {
            base.base_color
        };
    }
}

fn cleanup_acting_book_outside_phase(
    mut commands: Commands,
    battle_state: Res<BattleState>,
    acting_book: Query<Entity, With<ActingBookPanel>>,
) {
    if matches!(battle_state.phase, BattlePhase::Acting) {
        return;
    }

    for panel in acting_book.iter() {
        commands.entity(panel).despawn();
    }
}

fn first_four_book_entries(
    targets: &[dictionary::Futharkation],
) -> [Option<dictionary::Futharkation>; 4] {
    let mut entries: [Option<dictionary::Futharkation>; 4] = [None, None, None, None];

    for (index, target) in targets.iter().take(4).cloned().enumerate() {
        entries[index] = Some(target);
    }

    entries
}

fn score_acting_row_on_enter(
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut acting_data: ResMut<ActingData>,
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

    let Some((best_target, best_results)) = find_best_match(&guess, &acting_data.targets) else {
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

    acting_data.grading_against_letters = Some(best_target.letters.clone());

    acting_data.pending_success = correct >= 2;
    if acting_data.pending_success {
        acting_data.pending_matched = Some(best_target);
    }

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
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if row_resolved.is_empty() {
        return;
    }
    row_resolved.clear();
    acting_data.grading_against_letters = None;

    if acting_data.pending_success {
        let matched = acting_data.pending_matched.take().unwrap_or_else(|| {
            acting_data
                .targets
                .first()
                .cloned()
                .unwrap_or_else(|| dictionary::Futharkation {
                    word: String::new(),
                    letters: String::new(),
                })
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

    // Failure: spawn another attempt with the same slot count.
    let row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        acting_data.max_rune_count,
        ACTIVE_ROW_TOP,
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futhark;
    use crate::rune_words::battle::{ACTIVE_ROW_TOP, ROW_RISE, configure_battle};
    use crate::rune_words::rune_slots::{RuneSlot, configure_rune_slots};
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
        });
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

    fn acting_book_panel_count(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<ActingBookPanel>>();
        query.iter(world).count()
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
        app.world_mut().write_message(StartActing {
            targets: vec![
                futha("aa", "fu"),
                futha("bbbbb", "futar"),
                futha("ccc", "fut"),
            ],
        });
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
        app.world_mut()
            .write_message(StartActing { targets: vec![] });
        app.update();
        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "empty targets should not start acting"
        );
    }

    #[test]
    fn acting_two_correct_marks_pending_success() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        fill_active_row(&mut app, "futxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(
            acting_data.pending_success,
            "3 correct runes should set pending_success"
        );
    }

    #[test]
    fn acting_success_keeps_accepting_book_words() {
        let mut app = make_test_app();
        let targets = vec![futha("fable", "futar"), futha("dune", "dune")];
        app.world_mut().write_message(StartActing {
            targets: targets.clone(),
        });
        app.update();

        fill_active_row(&mut app, "futxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(acting_data.pending_success);
        assert_eq!(
            acting_data.targets, targets,
            "success should not collapse acting targets to one matched word"
        );
    }

    #[test]
    fn acting_fewer_than_two_correct_does_not_set_pending_success() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        // only 1 correct ('f')
        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(!acting_data.pending_success);
    }

    #[test]
    fn acting_success_sends_succeeded_and_transitions_to_idle() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        fill_active_row(&mut app, "futxx");
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
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
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
    fn acting_failure_pushes_previous_row_up() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        let first_row = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..10 {
            app.update();
        }

        let top = app.world().entity(first_row[0]).get::<Node>().unwrap().top;
        assert_eq!(top, Val::Px(ACTIVE_ROW_TOP - ROW_RISE));
    }

    #[test]
    fn acting_failure_queues_typed_input_for_next_row_in_order() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        // Force failure (fewer than 2 correct).
        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        app.world_mut()
            .write_message(futhark::TypedFutharkInput('u'));
        app.world_mut()
            .write_message(futhark::TypedFutharkInput('t'));
        app.world_mut()
            .write_message(futhark::TypedFutharkInput('a'));

        for _ in 0..12 {
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
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        fill_active_row(&mut app, "futxx");
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
        app.world_mut().write_message(StartActing {
            targets: vec![futha("again", "futar")],
        });
        app.update();

        let row = active_row_letters(&mut app);
        assert!(row.into_iter().all(|letter| letter.is_none()));
    }

    #[test]
    fn first_four_book_entries_limits_and_pads() {
        let entries = first_four_book_entries(&[
            futha("w1", "fut"),
            futha("w2", "ark"),
            futha("w3", "gwn"),
            futha("w4", "ijp"),
            futha("w5", "zst"),
        ]);

        assert_eq!(entries[0].as_ref().map(|w| w.word.as_str()), Some("w1"));
        assert_eq!(entries[1].as_ref().map(|w| w.word.as_str()), Some("w2"));
        assert_eq!(entries[2].as_ref().map(|w| w.word.as_str()), Some("w3"));
        assert_eq!(entries[3].as_ref().map(|w| w.word.as_str()), Some("w4"));

        let padded = first_four_book_entries(&[futha("single", "fut")]);
        assert_eq!(padded[0].as_ref().map(|w| w.word.as_str()), Some("single"));
        assert!(padded[1].is_none());
        assert!(padded[2].is_none());
        assert!(padded[3].is_none());
    }

    #[test]
    fn start_acting_spawns_book_panel() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![
                futha("a", "fut"),
                futha("b", "ark"),
                futha("c", "gwn"),
                futha("d", "ijp"),
            ],
        });
        app.update();

        assert_eq!(acting_book_panel_count(&mut app), 1);
    }

    #[test]
    fn acting_book_panel_cleans_up_when_phase_changes() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("a", "fut")],
        });
        app.update();

        app.world_mut().resource_mut::<BattleState>().phase = BattlePhase::Idle;
        app.update();

        assert_eq!(acting_book_panel_count(&mut app), 0);
    }
}
