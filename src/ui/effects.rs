use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

use crate::GameAssets;
use crate::GameState;
use crate::health::{Buff, NpcAttackState, NpcCombatState, PlayerCombatState, ShieldState};
use crate::rune_words::battle_states::acting::ActingSucceeded;
use crate::spellbook::{Book, SpellEffect};
use crate::ui::arena::NpcSprite;
use crate::ui::hud_root::ArenaPanel;
use crate::ui::palette::*;

// ─── Constants ────────────────────────────────────────────────────────────────

const EFFECT_DISPLAY_DURATION: f32 = 0.8;
const FLOAT_SPEED: f32 = 40.0;
const SHAKE_DURATION: f32 = 0.4;
const SHAKE_INTENSITY: f32 = 4.0;

// ─── Components & Resources ───────────────────────────────────────────────────

/// A queued spell effect waiting to be animated.
#[derive(Clone, Debug)]
struct QueuedEffect {
    effect: SpellEffect,
    buff_total: i32,
}

/// Resource holding the queue of effects to animate sequentially.
#[derive(Resource, Default)]
pub struct EffectsQueue {
    queue: Vec<QueuedEffect>,
    active: Option<ActiveEffect>,
    buff_total: i32,
}

impl EffectsQueue {
    pub fn is_busy(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
    }
}

#[derive(Clone, Debug)]
struct ActiveEffect {
    effect: SpellEffect,
    elapsed: f32,
    spawned_visual: bool,
}

/// Floating text entity (damage number / stun label).
#[derive(Component)]
struct FloatingText {
    elapsed: f32,
    duration: f32,
    start_top_pct: f32,
}

/// NPC shake animation driven by stun effects.
#[derive(Component)]
struct NpcShake {
    elapsed: f32,
    duration: f32,
    base_left_pct: f32,
}

/// Buff box shown at the bottom of the arena.
#[derive(Component)]
struct BuffBox {
    buff_id: u64,
}

/// Bounce animation on a newly spawned buff box.
#[derive(Component)]
struct BuffBoxBounce {
    elapsed: f32,
    base_bottom_pct: f32,
}

// ─── Configure ────────────────────────────────────────────────────────────────

pub fn configure_effects(app: &mut App) {
    app.init_resource::<EffectsQueue>();
    app.add_systems(
        Update,
        (
            enqueue_effects_on_success,
            process_effect_queue,
            animate_floating_text,
            animate_npc_shake,
            sync_buff_boxes,
            animate_buff_box_bounce,
        )
            .chain()
            .run_if(in_state(GameState::Adventure)),
    );
}

// ─── Enqueue effects when acting succeeds ─────────────────────────────────────

fn enqueue_effects_on_success(
    mut events: MessageReader<ActingSucceeded>,
    mut queue: ResMut<EffectsQueue>,
    player: Option<Res<PlayerCombatState>>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
) {
    for event in events.read() {
        let word = &event.matched.word;

        // Look up the spell definition to get its effects
        let effects = if let Some(ref ga) = game_assets {
            if let Some(book) = books.get(&ga.spellbook) {
                book.spells()
                    .iter()
                    .find(|s| s.word == *word)
                    .map(|s| s.effects.clone())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Compute buff total for damage calculation
        let buff_total: i32 = player
            .as_ref()
            .map(|p| p.attack_buffs.iter().map(|b| b.value).sum())
            .unwrap_or(0);

        for effect in effects {
            queue.queue.push(QueuedEffect { effect, buff_total });
        }
    }
}

// ─── Process one effect at a time ─────────────────────────────────────────────

fn process_effect_queue(
    mut commands: Commands,
    time: Res<Time>,
    mut queue: ResMut<EffectsQueue>,
    mut player: Option<ResMut<PlayerCombatState>>,
    mut npcs: Query<&mut NpcCombatState>,
    npc_sprite_query: Query<Entity, With<NpcSprite>>,
    arena_query: Query<Entity, With<ArenaPanel>>,
    game_assets: Option<Res<GameAssets>>,
) {
    let dt = time.delta_secs();

    // Capture before mutable borrow
    let buff_total = queue.buff_total;

    // Tick active effect
    if let Some(ref mut active) = queue.active {
        active.elapsed += dt;

        if !active.spawned_visual {
            active.spawned_visual = true;
            let Ok(arena_entity) = arena_query.single() else {
                queue.active = None;
                return;
            };
            let font = game_assets
                .as_ref()
                .map(|ga| ga.font_cormorant_unicase_bold.clone())
                .unwrap_or_default();

            match &active.effect {
                SpellEffect::Damage { amount } => {
                    let effective = (*amount as i32 + buff_total).max(0) as u32;
                    // Apply damage to NPC
                    for mut npc in &mut npcs {
                        npc.hp = npc.hp.saturating_sub(effective);
                    }
                    // Spawn floating damage number
                    spawn_floating_text(
                        &mut commands,
                        arena_entity,
                        &format!("{}", effective),
                        BLOOD_BRIGHT,
                        font,
                        24.0,
                    );
                }
                SpellEffect::Binding { amount } => {
                    // Apply binding to NPC
                    for mut npc in &mut npcs {
                        npc.bindings += *amount;
                    }
                    // Spawn floating "BOUND" text
                    spawn_floating_text(
                        &mut commands,
                        arena_entity,
                        &format!("+{} BIND", amount),
                        MANA_BRIGHT,
                        font,
                        18.0,
                    );
                }
                SpellEffect::Stun { amount } => {
                    // Apply stun to NPC
                    for mut npc in &mut npcs {
                        npc.attack_state = NpcAttackState::Stunned(*amount);
                    }
                    // Spawn floating "STUN" text
                    spawn_floating_text(
                        &mut commands,
                        arena_entity,
                        "STUN",
                        GOLD_LIGHT,
                        font,
                        18.0,
                    );
                    // Add shake to NPC sprite
                    for entity in &npc_sprite_query {
                        commands.entity(entity).insert(NpcShake {
                            elapsed: 0.0,
                            duration: SHAKE_DURATION,
                            base_left_pct: 39.0,
                        });
                    }
                }
                SpellEffect::Shield { amount, duration } => {
                    // Apply shield to player
                    if let Some(ref mut p) = player {
                        p.shields.push(ShieldState {
                            amount: *amount,
                            expires_in: *duration,
                        });
                    }
                    // Spawn floating shield indicator
                    spawn_floating_text(
                        &mut commands,
                        arena_entity,
                        &format!("+{} Shield", amount),
                        MANA_BRIGHT,
                        font,
                        16.0,
                    );
                }
                SpellEffect::Buff { amount, duration } => {
                    // Apply buff to player
                    let buff_id = if let Some(ref mut p) = player {
                        let id = p.next_buff_id;
                        p.next_buff_id += 1;
                        p.attack_buffs.push(Buff {
                            id,
                            name: format!("+{}", amount),
                            value: *amount,
                            expires_in: *duration,
                        });
                        id
                    } else {
                        0
                    };
                    // Spawn buff box at bottom of arena
                    spawn_buff_box(
                        &mut commands,
                        arena_entity,
                        *amount,
                        buff_id,
                        game_assets
                            .as_ref()
                            .map(|ga| ga.font_cormorant_unicase_semibold.clone())
                            .unwrap_or_default(),
                    );
                }
            }
        }

        if active.elapsed >= EFFECT_DISPLAY_DURATION {
            queue.active = None;
        }
        return;
    }

    // Pop next effect from queue
    if let Some(queued) = queue.queue.first().cloned() {
        queue.buff_total = queued.buff_total;
        queue.queue.remove(0);
        queue.active = Some(ActiveEffect {
            effect: queued.effect,
            elapsed: 0.0,
            spawned_visual: false,
        });
    } else {
        // Queue fully drained — restart acting if we were mid-combat
        // (the acting system handles this via its own events)
    }
}

// ─── Spawn helpers ────────────────────────────────────────────────────────────

fn spawn_floating_text(
    commands: &mut Commands,
    arena_entity: Entity,
    text: &str,
    color: Color,
    font: Handle<Font>,
    font_size: f32,
) {
    let start_top = 40.0;
    commands.entity(arena_entity).with_children(|arena| {
        arena.spawn((
            FloatingText {
                elapsed: 0.0,
                duration: EFFECT_DISPLAY_DURATION,
                start_top_pct: start_top,
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(start_top),
                left: Val::Percent(30.0),
                right: Val::Percent(30.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Text::new(text.to_string()),
            TextFont {
                font,
                font_size,
                ..default()
            },
            TextColor(color),
            ZIndex(10),
        ));
    });
}

fn spawn_buff_box(
    commands: &mut Commands,
    arena_entity: Entity,
    amount: i32,
    buff_id: u64,
    font: Handle<Font>,
) {
    let base_bottom = 5.0;
    commands.entity(arena_entity).with_children(|arena| {
        arena.spawn((
            BuffBox { buff_id },
            BuffBoxBounce {
                elapsed: 0.0,
                base_bottom_pct: base_bottom,
            },
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Percent(base_bottom),
                left: Val::Auto,
                right: Val::Auto,
                padding: UiRect {
                    left: Val::Px(6.0),
                    right: Val::Px(6.0),
                    top: Val::Px(3.0),
                    bottom: Val::Px(3.0),
                },
                margin: UiRect {
                    left: Val::Px(4.0),
                    right: Val::Px(4.0),
                    ..default()
                },
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.04, 0.9)),
            BorderColor {
                top: VERDANT,
                right: VERDANT,
                bottom: VERDANT,
                left: VERDANT,
            },
            Text::new(format!("+{}", amount)),
            TextFont {
                font,
                font_size: 12.0,
                ..default()
            },
            TextColor(VERDANT),
            ZIndex(5),
        ));
    });
}

// ─── Animations ───────────────────────────────────────────────────────────────

fn animate_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FloatingText, &mut Node, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut float, mut node, mut color) in &mut query {
        float.elapsed += dt;
        let progress = (float.elapsed / float.duration).min(1.0);

        // Float upward
        let offset = progress * FLOAT_SPEED * (float.duration / 100.0) * 100.0;
        node.top = Val::Percent(float.start_top_pct - offset / 4.0);

        // Fade out in the last 30%
        let alpha = if progress > 0.7 {
            1.0 - (progress - 0.7) / 0.3
        } else {
            1.0
        };
        color.0 = color.0.with_alpha(alpha);

        if float.elapsed >= float.duration {
            commands.entity(entity).despawn();
        }
    }
}

fn animate_npc_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut NpcShake, &mut Node)>,
) {
    let dt = time.delta_secs();
    for (entity, mut shake, mut node) in &mut query {
        shake.elapsed += dt;
        let progress = (shake.elapsed / shake.duration).min(1.0);

        // Horizontal shake that decays
        let decay = 1.0 - progress;
        let frequency = 30.0;
        let offset = (shake.elapsed * frequency).sin() * SHAKE_INTENSITY * decay;
        node.left = Val::Percent(shake.base_left_pct + offset * 0.2);

        if shake.elapsed >= shake.duration {
            node.left = Val::Percent(shake.base_left_pct);
            commands.entity(entity).remove::<NpcShake>();
        }
    }
}

fn sync_buff_boxes(
    mut commands: Commands,
    query: Query<(Entity, &BuffBox)>,
    player: Option<Res<PlayerCombatState>>,
) {
    let Some(player) = player else {
        return;
    };
    for (entity, buff_box) in &query {
        let still_active = player.attack_buffs.iter().any(|b| b.id == buff_box.buff_id);
        if !still_active {
            commands.entity(entity).despawn();
        }
    }
}

const BOUNCE_DURATION: f32 = 0.35;
const BOUNCE_HEIGHT: f32 = 4.0;

fn animate_buff_box_bounce(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut BuffBoxBounce, &mut Node)>,
) {
    let dt = time.delta_secs();
    for (entity, mut bounce, mut node) in &mut query {
        bounce.elapsed += dt;
        let progress = (bounce.elapsed / BOUNCE_DURATION).min(1.0);

        // Single bounce: sine half-wave
        let offset = (progress * std::f32::consts::PI).sin() * BOUNCE_HEIGHT;
        node.bottom = Val::Percent(bounce.base_bottom_pct + offset);

        if bounce.elapsed >= BOUNCE_DURATION {
            node.bottom = Val::Percent(bounce.base_bottom_pct);
            commands.entity(entity).remove::<BuffBoxBounce>();
        }
    }
}
