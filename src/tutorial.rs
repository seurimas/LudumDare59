use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::combat::BattleStart;
use crate::health::{NpcAttackState, NpcCombatState, PlayerCombatState};
use crate::npcs::NpcSpec;
use crate::rune_words::battle::{BattlePhase, BattleState};
use crate::rune_words::battle_states::acting::{ActingSucceeded, StartActing};
use crate::rune_words::battle_states::binding::{BindingSucceeded, StartBinding};
use crate::spellbook::Book;
use crate::ui::arena::NpcSprite;
use crate::ui::hud_root::{BindingPanel, BookPanel, InscribedPanel};
use crate::ui::keyboard::KeyboardPanel;
use crate::{GameAssets, GameState};

/// The binding word used during the tutorial.
pub const TUTORIAL_BINDING_WORD: &str = "ash";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialStep {
    Welcome,
    ExplainSpellbook,
    ExplainKeyboard,
    DefendPhase,
    AttackPhase,
    BindingPhase,
    Done,
}

#[derive(Resource)]
pub struct TutorialState {
    pub active: bool,
    pub step: TutorialStep,
    /// Set to true when the current step's action has been completed and we
    /// should advance on the next frame.
    pub advance_pending: bool,
}

impl Default for TutorialState {
    fn default() -> Self {
        Self {
            active: false,
            step: TutorialStep::Welcome,
            advance_pending: false,
        }
    }
}

impl TutorialState {
    pub fn start() -> Self {
        Self {
            active: true,
            step: TutorialStep::Welcome,
            advance_pending: false,
        }
    }
}

// ─── UI Components ────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct TutorialOverlay;

#[derive(Component)]
struct TutorialTextBox;

#[derive(Component)]
struct TutorialContinueButton;

#[derive(Component)]
struct TutorialHighlight;

// ─── Configure ────────────────────────────────────────────────────────────────

pub fn configure_tutorial(app: &mut App) {
    app.init_resource::<TutorialState>();
    app.add_systems(
        Update,
        (
            handle_tutorial_continue_input,
            advance_tutorial_step,
            sync_tutorial_overlay,
            block_npc_attack_during_tutorial,
            handle_tutorial_shield_success,
            handle_tutorial_bop_success,
            handle_tutorial_binding_success,
        )
            .chain()
            .run_if(in_state(GameState::Adventure))
            .run_if(tutorial_active),
    );
    app.add_systems(OnExit(GameState::Adventure), cleanup_tutorial);
}

fn tutorial_active(state: Res<TutorialState>) -> bool {
    state.active
}

// ─── Input handling ───────────────────────────────────────────────────────────

fn handle_tutorial_continue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<TutorialContinueButton>)>,
    mut tutorial: ResMut<TutorialState>,
) {
    let step = tutorial.step;
    // Only text-and-continue steps respond to Enter / click
    if !matches!(
        step,
        TutorialStep::Welcome
            | TutorialStep::ExplainSpellbook
            | TutorialStep::ExplainKeyboard
            | TutorialStep::Done
    ) {
        return;
    }

    let enter_pressed =
        keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter);
    let button_clicked = interactions.iter().any(|i| *i == Interaction::Pressed);

    if enter_pressed || button_clicked {
        tutorial.advance_pending = true;
    }
}

// ─── Step advancement ─────────────────────────────────────────────────────────

fn advance_tutorial_step(
    _commands: Commands,
    mut tutorial: ResMut<TutorialState>,
    mut battle_state: ResMut<BattleState>,
    mut player: ResMut<PlayerCombatState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut battle_start: MessageWriter<BattleStart>,
    mut start_acting: MessageWriter<StartActing>,
    game_assets: Option<Res<GameAssets>>,
    specs: Res<Assets<NpcSpec>>,
    books: Res<Assets<Book>>,
    _npcs: Query<&mut NpcCombatState, With<NpcSprite>>,
) {
    if !tutorial.advance_pending {
        return;
    }
    tutorial.advance_pending = false;

    match tutorial.step {
        TutorialStep::Welcome => {
            tutorial.step = TutorialStep::ExplainSpellbook;
        }
        TutorialStep::ExplainSpellbook => {
            tutorial.step = TutorialStep::ExplainKeyboard;
        }
        TutorialStep::ExplainKeyboard => {
            // Spawn a goblin with 1 HP
            let Some(ref ga) = game_assets else { return };
            let Some(spec) = specs.get(&ga.goblin_spec) else {
                return;
            };

            battle_state.npc = Some(spec.clone());
            battle_start.write(BattleStart);

            // Set player hand to just "shield"
            set_tutorial_hand(&mut player, &["shield"], &ga, &books);

            // After the NPC spawns, we'll configure it in block_npc_attack_during_tutorial
            tutorial.step = TutorialStep::DefendPhase;
            start_acting.write(StartActing);
        }
        TutorialStep::DefendPhase => {
            // Handled by handle_tutorial_shield_success
        }
        TutorialStep::AttackPhase => {
            // Handled by handle_tutorial_bop_success
        }
        TutorialStep::BindingPhase => {
            // Handled by handle_tutorial_binding_success
        }
        TutorialStep::Done => {
            tutorial.active = false;
            battle_state.phase = BattlePhase::Idle;
            next_state.set(GameState::MainMenu);
        }
    }
}

// ─── Block NPC attacks during tutorial ────────────────────────────────────────

fn block_npc_attack_during_tutorial(
    tutorial: Res<TutorialState>,
    game_assets: Option<Res<GameAssets>>,
    specs: Res<Assets<NpcSpec>>,
    mut npcs: Query<&mut NpcCombatState, With<NpcSprite>>,
) {
    if !tutorial.active {
        return;
    }

    for mut npc in &mut npcs {
        match tutorial.step {
            TutorialStep::DefendPhase => {
                // Set NPC to 1 HP, set attack state to AttackingIn with first attack
                // but never let it reach 0 (freeze at a small value)
                if npc.hp != 1 {
                    npc.hp = 1;
                    npc.max = 1;
                }
                if let Some(ref ga) = game_assets {
                    if let Some(spec) = specs.get(&ga.goblin_spec) {
                        if npc.chosen_attack.is_none() && !spec.attacks.is_empty() {
                            npc.chosen_attack = Some(spec.attacks[0]);
                            npc.attack_state =
                                NpcAttackState::AttackingIn(spec.attacks[0].attack_time);
                        }
                    }
                }
                // Freeze the attack timer: never let it actually fire
                match npc.attack_state {
                    NpcAttackState::AttackingIn(t) if t <= 0.5 => {
                        npc.attack_state = NpcAttackState::AttackingIn(0.5);
                    }
                    _ => {}
                }
            }
            TutorialStep::AttackPhase => {
                // Keep NPC at 1 HP, stunned so they don't attack
                if npc.hp != 1 {
                    npc.hp = 1;
                }
                npc.attack_state = NpcAttackState::Stunned(999.0);
            }
            TutorialStep::BindingPhase => {
                // NPC is dead (hp=0), binding phase handles this
                npc.attack_state = NpcAttackState::Stunned(999.0);
            }
            _ => {}
        }
    }
}

// ─── Handle acting successes during tutorial ──────────────────────────────────

fn handle_tutorial_shield_success(
    mut events: MessageReader<ActingSucceeded>,
    mut tutorial: ResMut<TutorialState>,
    mut player: ResMut<PlayerCombatState>,
    mut start_acting: MessageWriter<StartActing>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
) {
    if tutorial.step != TutorialStep::DefendPhase {
        events.clear();
        return;
    }

    let Some(event) = events.read().last() else {
        return;
    };

    if event.matched.word.to_lowercase() == "shield" {
        // Shield cast! Now give the player "bop" and move to attack phase
        let Some(ref ga) = game_assets else { return };
        set_tutorial_hand(&mut player, &["bop"], &ga, &books);
        tutorial.step = TutorialStep::AttackPhase;
        start_acting.write(StartActing);
    }
}

fn handle_tutorial_bop_success(
    mut events: MessageReader<ActingSucceeded>,
    mut tutorial: ResMut<TutorialState>,
    mut npcs: Query<&mut NpcCombatState, With<NpcSprite>>,
    mut start_binding: MessageWriter<StartBinding>,
) {
    if tutorial.step != TutorialStep::AttackPhase {
        events.clear();
        return;
    }

    let Some(event) = events.read().last() else {
        return;
    };

    if event.matched.word.to_lowercase() == "bop" {
        // Kill the NPC and directly start binding (phase is already Idle
        // after acting success, so trigger_binding_on_npc_death won't fire).
        for mut npc in &mut npcs {
            npc.hp = 0;
        }
        tutorial.step = TutorialStep::BindingPhase;
        start_binding.write(StartBinding(None));
    }
}

fn handle_tutorial_binding_success(
    mut events: MessageReader<BindingSucceeded>,
    mut tutorial: ResMut<TutorialState>,
    mut battle_state: ResMut<BattleState>,
) {
    if tutorial.step != TutorialStep::BindingPhase {
        events.clear();
        return;
    }

    if events.read().last().is_none() {
        return;
    }

    // Show the final message (advance_tutorial_step handles the actual exit)
    tutorial.step = TutorialStep::Done;
    battle_state.phase = BattlePhase::Idle;
}

// ─── Tutorial overlay UI ──────────────────────────────────────────────────────

fn sync_tutorial_overlay(
    mut commands: Commands,
    tutorial: Res<TutorialState>,
    game_assets: Option<Res<GameAssets>>,
    existing_overlay: Query<Entity, With<TutorialOverlay>>,
    keyboard_panels: Query<Entity, With<KeyboardPanel>>,
    book_panels: Query<Entity, With<BookPanel>>,
    inscribed_panels: Query<Entity, With<InscribedPanel>>,
    binding_panels: Query<Entity, With<BindingPanel>>,
) {
    if !tutorial.is_changed() {
        return;
    }

    // Despawn old overlay
    for entity in &existing_overlay {
        commands.entity(entity).despawn();
    }

    if !tutorial.active {
        return;
    }

    let font = game_assets
        .as_ref()
        .map(|ga| ga.font_cormorant_garamond_italic.clone())
        .unwrap_or_default();
    let bold_font = game_assets
        .as_ref()
        .map(|ga| ga.font_cormorant_unicase_semibold.clone())
        .unwrap_or_default();

    let (message, show_continue, highlight_target) = tutorial_step_config(tutorial.step);

    // Spawn the overlay
    let mut overlay = commands.spawn((
        TutorialOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ZIndex(100),
    ));

    overlay.with_children(|parent| {
        // Semi-transparent backdrop for text steps
        if matches!(
            tutorial.step,
            TutorialStep::Welcome | TutorialStep::ExplainSpellbook | TutorialStep::ExplainKeyboard
        ) {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ));
        }

        // Text box
        parent
            .spawn((
                TutorialTextBox,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    max_width: Val::Px(600.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.95)),
                BorderColor {
                    top: crate::ui::palette::GOLD_DARK,
                    right: crate::ui::palette::GOLD_DARK,
                    bottom: crate::ui::palette::GOLD_DARK,
                    left: crate::ui::palette::GOLD_DARK,
                },
            ))
            .with_children(|text_box| {
                text_box.spawn((
                    Text::new(message),
                    TextFont {
                        font: font.clone(),
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(crate::ui::palette::PARCHMENT_WARM),
                    Node {
                        max_width: Val::Px(520.0),
                        ..default()
                    },
                ));

                if show_continue {
                    text_box
                        .spawn((
                            TutorialContinueButton,
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(24.0), Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                margin: UiRect::top(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(crate::ui::palette::INK),
                            BorderColor::from(crate::ui::palette::GOLD_DARK),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Press Enter to continue"),
                                TextFont {
                                    font: bold_font.clone(),
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(crate::ui::palette::PARCHMENT),
                            ));
                        });
                }
            });
    });

    // Add highlight borders to target panels
    match highlight_target {
        HighlightTarget::None => {}
        HighlightTarget::Book => {
            for entity in &book_panels {
                commands.entity(entity).insert((
                    TutorialHighlight,
                    BorderColor {
                        top: crate::ui::palette::GOLD,
                        right: crate::ui::palette::GOLD,
                        bottom: crate::ui::palette::GOLD,
                        left: crate::ui::palette::GOLD,
                    },
                    Outline::new(Val::Px(3.0), Val::ZERO, crate::ui::palette::GOLD),
                ));
            }
        }
        HighlightTarget::Keyboard => {
            for entity in &keyboard_panels {
                commands.entity(entity).insert((
                    TutorialHighlight,
                    Outline::new(Val::Px(3.0), Val::ZERO, crate::ui::palette::GOLD),
                ));
            }
        }
        HighlightTarget::RuneWordAndBinding => {
            for entity in &inscribed_panels {
                commands.entity(entity).insert((
                    TutorialHighlight,
                    Outline::new(Val::Px(3.0), Val::ZERO, crate::ui::palette::GOLD),
                ));
            }
            for entity in &binding_panels {
                commands.entity(entity).insert((
                    TutorialHighlight,
                    Outline::new(Val::Px(3.0), Val::ZERO, crate::ui::palette::GOLD),
                ));
            }
        }
    }
}

enum HighlightTarget {
    None,
    Book,
    Keyboard,
    RuneWordAndBinding,
}

fn tutorial_step_config(step: TutorialStep) -> (&'static str, bool, HighlightTarget) {
    match step {
        TutorialStep::Welcome => (
            "Welcome, you are a wizard battling your way through a cursed forest.",
            true,
            HighlightTarget::None,
        ),
        TutorialStep::ExplainSpellbook => (
            "You have in your spellbook various spells which you must cast from runes.",
            true,
            HighlightTarget::Book,
        ),
        TutorialStep::ExplainKeyboard => (
            "Click or type these runes to spell words.",
            true,
            HighlightTarget::Keyboard,
        ),
        TutorialStep::DefendPhase => (
            "The enemy prepares to strike! Type the runes you see here to cast a shield!",
            false,
            HighlightTarget::None,
        ),
        TutorialStep::AttackPhase => ("Finish your foe!", false, HighlightTarget::None),
        TutorialStep::BindingPhase => (
            "You must bind this spirit away by divining their binding word. Use your runes to sound it out. If you fail, the creature will receive a burst of energy and rise to fight again.",
            false,
            HighlightTarget::RuneWordAndBinding,
        ),
        TutorialStep::Done => (
            "You have bound the spirit, but more lurk deeper. Are you ready?",
            true,
            HighlightTarget::None,
        ),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn set_tutorial_hand(
    player: &mut PlayerCombatState,
    words: &[&str],
    game_assets: &GameAssets,
    books: &Assets<Book>,
) {
    let Some(book) = books.get(&game_assets.spellbook) else {
        return;
    };

    player.hand.clear();
    player.deck.clear();
    player.discard.clear();

    for &word in words {
        if let Some(spell) = book.spells().iter().find(|s| s.word == word) {
            player.hand.push(spell.clone());
        }
    }
}

fn cleanup_tutorial(
    mut commands: Commands,
    overlay: Query<Entity, With<TutorialOverlay>>,
    highlights: Query<Entity, With<TutorialHighlight>>,
) {
    for entity in &overlay {
        commands.entity(entity).despawn();
    }
    for entity in &highlights {
        commands
            .entity(entity)
            .remove::<(TutorialHighlight, Outline)>();
    }
}
