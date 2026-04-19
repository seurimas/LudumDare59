use bevy::prelude::*;

use crate::health::{NpcAttackState, NpcCombatState};
use crate::rune_words::battle::{BattlePhase, BattleState, NpcType};
use crate::ui::clock::{BattleUiClock, wave};
use crate::ui::hud_root::ArenaPanel;
use crate::ui::palette::*;
use crate::{GameAssets, GameState};

// ─── Components ───────────────────────────────────────────────────────────────

/// Marker for the two-bar GOLD corner bracket decorations.
#[derive(Component)]
struct CornerBracket;

/// The NPC sprite shown in the arena. Name kept stable to avoid UAT churn.
#[derive(Component)]
pub struct NpcSprite;

/// Ground shadow ellipse beneath the NPC sprite.
#[derive(Component)]
struct GroundShadow;

/// Pill label in the top-left of the arena showing the current phase.
#[derive(Component)]
struct PhaseMark;

/// The pulsing dot inside the PhaseMark pill.
#[derive(Component)]
struct PhaseMarkDot;

/// The text label inside the PhaseMark pill.
#[derive(Component)]
struct PhaseMarkText;

/// Added to an NPC sprite entity when it dies; drives the fade-out animation.
#[derive(Component, Default)]
struct NpcDeathFade {
    elapsed: f32,
}

const NPC_DEATH_FADE_DURATION: f32 = 0.8;

// ─── Configure ────────────────────────────────────────────────────────────────

pub fn configure_arena(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::Adventure),
        spawn_arena_ui.after(crate::ui::hud_root::spawn_battle_hud_root),
    );
    app.add_systems(
        Update,
        (
            sync_npc_sprite,
            sync_phase_mark,
            animate_arena,
            animate_npc_death_fade,
        )
            .run_if(in_state(GameState::Adventure)),
    );
}

// ─── Spawn ────────────────────────────────────────────────────────────────────

pub fn spawn_arena_ui(
    mut commands: Commands,
    panel_query: Query<Entity, With<ArenaPanel>>,
    game_assets: Res<GameAssets>,
) {
    let Ok(panel_entity) = panel_query.single() else {
        return;
    };

    let font = game_assets.font_cormorant_unicase_semibold.clone();

    // Upgrade the placeholder node: backdrop image + GOLD border colour
    commands.entity(panel_entity).insert((
        ImageNode::new(game_assets.backdrop.clone()),
        BorderColor {
            top: GOLD,
            right: GOLD,
            bottom: GOLD,
            left: GOLD,
        },
    ));

    commands.entity(panel_entity).with_children(|arena| {
        // ── Corner brackets: 2 bars (horizontal + vertical) per corner ───────
        let bracket_pct = 2.5_f32;
        for (t, r, b, l) in [
            (
                Val::Percent(bracket_pct),
                Val::Auto,
                Val::Auto,
                Val::Percent(bracket_pct),
            ), // TL
            (
                Val::Percent(bracket_pct),
                Val::Percent(bracket_pct),
                Val::Auto,
                Val::Auto,
            ), // TR
            (
                Val::Auto,
                Val::Auto,
                Val::Percent(bracket_pct),
                Val::Percent(bracket_pct),
            ), // BL
            (
                Val::Auto,
                Val::Percent(bracket_pct),
                Val::Percent(bracket_pct),
                Val::Auto,
            ), // BR
        ] {
            // Horizontal bar
            arena.spawn((
                CornerBracket,
                Node {
                    position_type: PositionType::Absolute,
                    top: t.clone(),
                    right: r.clone(),
                    bottom: b.clone(),
                    left: l.clone(),
                    width: Val::Percent(18.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(GOLD),
            ));
            // Vertical bar
            arena.spawn((
                CornerBracket,
                Node {
                    position_type: PositionType::Absolute,
                    top: t,
                    right: r,
                    bottom: b,
                    left: l,
                    width: Val::Px(2.0),
                    height: Val::Percent(18.0),
                    ..default()
                },
                BackgroundColor(GOLD),
            ));
        }

        // ── PhaseMark pill ───────────────────────────────────────────────────
        arena
            .spawn((
                PhaseMark,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(5.0),
                    left: Val::Percent(5.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(5.0),
                    padding: UiRect {
                        left: Val::Px(7.0),
                        right: Val::Px(7.0),
                        top: Val::Px(3.0),
                        bottom: Val::Px(3.0),
                    },
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.04, 0.02, 0.85)),
                BorderColor {
                    top: GOLD_DARK,
                    right: GOLD_DARK,
                    bottom: GOLD_DARK,
                    left: GOLD_DARK,
                },
            ))
            .with_children(|pill| {
                // Pulsing dot
                pill.spawn((
                    PhaseMarkDot,
                    Node {
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(BLOOD_BRIGHT),
                ));
                // Phase name text
                pill.spawn((
                    PhaseMarkText,
                    Text::new("Idle"),
                    TextFont {
                        font,
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(GOLD_LIGHT),
                ));
            });
    });
}

// ─── NPC sprite sync (moved from combat.rs) ───────────────────────────────────

fn npc_sprite_index(npc: &NpcCombatState, phase: BattlePhase) -> usize {
    if matches!(phase, BattlePhase::Binding)
        || matches!(npc.attack_state, NpcAttackState::Stunned(_))
    {
        return 3;
    }
    if npc.chosen_attack.is_none() {
        return 0;
    }
    match npc.attack_state {
        NpcAttackState::WaitingFor(_) => 2,
        NpcAttackState::AttackingIn(_) => 1,
        _ => 0,
    }
}

const FLICKER_INVISIBLE_SECONDS: f32 = 1.0 / 15.0;

fn npc_sprite_visible(npc: &NpcCombatState) -> bool {
    let NpcAttackState::AttackingIn(remaining) = npc.attack_state else {
        return true;
    };
    let Some(attack) = npc.chosen_attack else {
        return true;
    };
    if attack.flicker_rate <= 0.0 {
        return true;
    }
    let elapsed = (attack.attack_time - remaining).max(0.0);
    let period = 1.0 / attack.flicker_rate;
    let phase = elapsed.rem_euclid(period);
    phase >= FLICKER_INVISIBLE_SECONDS
}

fn npc_image(npc_type: NpcType, game_assets: &GameAssets) -> ImageNode {
    let (image, layout) = match npc_type {
        NpcType::Goblin => (
            game_assets.goblin.clone(),
            game_assets.goblin_layout.clone(),
        ),
        NpcType::Robed => (game_assets.robed.clone(), game_assets.robed_layout.clone()),
    };
    ImageNode::from_atlas_image(image, TextureAtlas { layout, index: 0 })
}

fn sync_npc_sprite(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    battle_state: Option<Res<BattleState>>,
    panel_query: Query<Entity, With<ArenaPanel>>,
    mut npc_query: Query<
        (Entity, &mut ImageNode, &NpcCombatState),
        (With<NpcSprite>, Without<NpcDeathFade>),
    >,
    shadow_query: Query<Entity, With<GroundShadow>>,
) {
    let Some(battle_state) = battle_state else {
        for (entity, _, _) in &npc_query {
            commands.entity(entity).despawn();
        }
        for entity in &shadow_query {
            commands.entity(entity).despawn();
        }
        return;
    };

    // Victory phase: begin death fade on the NPC sprite (only once — sprite
    // won't appear in npc_query again after NpcDeathFade is inserted).
    if matches!(battle_state.phase, BattlePhase::Victory) {
        for (entity, _, _) in &npc_query {
            commands.entity(entity).insert(NpcDeathFade::default());
        }
        return;
    }

    let should_show = battle_state.npc.is_some();

    if !should_show {
        for (entity, _, _) in &npc_query {
            commands.entity(entity).despawn();
        }
        for entity in &shadow_query {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Some(npc_spec) = battle_state.npc.as_ref() else {
        return;
    };
    let npc_type = npc_spec.npc_type;

    if npc_query.is_empty() {
        let Ok(panel_entity) = panel_query.single() else {
            return;
        };

        let mut combat_state = NpcCombatState::default();
        combat_state.max = npc_spec.max_health;
        combat_state.hp = npc_spec.max_health;
        combat_state.attacks = npc_spec.attacks.clone();

        let sprite_index = npc_sprite_index(&combat_state, battle_state.phase);
        let mut image_node = npc_image(npc_type, &game_assets);
        if let Some(atlas) = &mut image_node.texture_atlas {
            atlas.index = sprite_index;
        }

        commands.entity(panel_entity).with_children(|arena| {
            // Ground shadow ellipse (oval via narrow height)
            arena.spawn((
                GroundShadow,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Percent(22.0),
                    left: Val::Percent(35.0),
                    right: Val::Percent(35.0),
                    height: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            ));

            // NPC sprite: ~22% wide, centered horizontally, 28% from top
            arena.spawn((
                NpcSprite,
                combat_state,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(28.0),
                    left: Val::Percent(39.0),
                    right: Val::Percent(39.0),
                    aspect_ratio: Some(1.0),
                    ..default()
                },
                image_node,
                ZIndex(1),
            ));
        });
    } else {
        for (_, mut image_node, combat_state) in &mut npc_query {
            let sprite_index = npc_sprite_index(combat_state, battle_state.phase);
            if let Some(atlas) = &mut image_node.texture_atlas {
                atlas.index = sprite_index;
            }
            let alpha = if npc_sprite_visible(combat_state) {
                1.0
            } else {
                0.0
            };
            image_node.color = image_node.color.with_alpha(alpha);
        }
    }
}

// ─── PhaseMark sync ───────────────────────────────────────────────────────────

fn phase_display_name(phase: BattlePhase) -> &'static str {
    match phase {
        BattlePhase::Idle => "Idle",
        BattlePhase::Acting => "Combat",
        BattlePhase::Binding => "Binding",
        BattlePhase::Victory => "Victory",
    }
}

fn sync_phase_mark(
    battle_state: Option<Res<BattleState>>,
    mut text_query: Query<&mut Text, With<PhaseMarkText>>,
    mut dot_query: Query<&mut BackgroundColor, With<PhaseMarkDot>>,
) {
    let phase = battle_state
        .as_ref()
        .map(|s| s.phase)
        .unwrap_or(BattlePhase::Idle);

    for mut text in &mut text_query {
        **text = phase_display_name(phase).to_string();
    }

    let dot_color = match phase {
        BattlePhase::Idle => Color::srgba(0.4, 0.3, 0.2, 0.6),
        BattlePhase::Acting => BLOOD_BRIGHT.with_alpha(0.9),
        BattlePhase::Binding => GOLD.with_alpha(0.9),
        BattlePhase::Victory => Color::srgb(0.2, 0.8, 0.4).with_alpha(0.9),
    };
    for mut bg in &mut dot_query {
        bg.0 = dot_color;
    }
}

// ─── Animations ───────────────────────────────────────────────────────────────

fn animate_arena(
    clock: Res<BattleUiClock>,
    mut npc_query: Query<&mut Node, With<NpcSprite>>,
    mut shadow_query: Query<&mut BackgroundColor, With<GroundShadow>>,
    mut dot_query: Query<&mut BackgroundColor, (With<PhaseMarkDot>, Without<GroundShadow>)>,
) {
    // NPC bob: small percent-based vertical oscillation
    let bob = wave(clock.elapsed, 2.2, -1.5, 1.5);
    for mut node in &mut npc_query {
        node.top = Val::Percent(28.0 + bob * 0.4);
    }

    // Ground shadow breathe (opacity inversely tracks bob height)
    let shadow_alpha = wave(clock.elapsed, 2.2, 0.25, 0.55);
    for mut bg in &mut shadow_query {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, shadow_alpha);
    }

    // Phase mark dot pulse
    let pulse_alpha = wave(clock.elapsed, 1.4, 0.45, 1.0);
    for mut bg in &mut dot_query {
        let base = bg.0;
        bg.0 = base.with_alpha(pulse_alpha);
    }
}

// ─── NPC death fade ───────────────────────────────────────────────────────────

fn animate_npc_death_fade(
    mut commands: Commands,
    time: Res<Time>,
    mut npc_query: Query<(Entity, &mut ImageNode, &mut NpcDeathFade), With<NpcSprite>>,
    shadow_query: Query<Entity, With<GroundShadow>>,
    mut battle_state: Option<ResMut<BattleState>>,
) {
    for (entity, mut image_node, mut fade) in &mut npc_query {
        fade.elapsed += time.delta_secs();
        let alpha = (1.0 - fade.elapsed / NPC_DEATH_FADE_DURATION).max(0.0);
        image_node.color = image_node.color.with_alpha(alpha);

        if fade.elapsed >= NPC_DEATH_FADE_DURATION {
            commands.entity(entity).despawn();
            for shadow_entity in &shadow_query {
                commands.entity(shadow_entity).despawn();
            }
            if let Some(ref mut bs) = battle_state {
                bs.npc = None;
                bs.phase = BattlePhase::Idle;
            }
        }
    }
}
