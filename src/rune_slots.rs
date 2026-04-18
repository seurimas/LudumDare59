use bevy::ecs::message::MessageReader;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::{GameAssets, futhark};

const SPRITE_SLOT_BACKGROUND: usize = 255;
const SPRITE_PRIMARY_RUNE_OFFSET: usize = 24;
const SPRITE_ALTERNATE_RUNE_OFFSET: usize = 48;
const RUNES_PER_SET: usize = 24;
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

pub fn configure_rune_slots(app: &mut App) {
    app.init_resource::<ActiveRuneSlot>();
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
    mut slots: Query<(&mut RuneSlot, Option<&RuneSlotLinks>)>,
) {
    let Some(last_typed) = futhark::last_typed_futhark_character(&mut typed_futhark_input) else {
        return;
    };

    let Some(active_entity) = active_slot.entity else {
        return;
    };

    let Some(index) = futhark::letter_to_index(last_typed) else {
        return;
    };

    let Ok((mut slot, links)) = slots.get_mut(active_entity) else {
        return;
    };

    slot.rune_index = Some(index);

    if let Some(next) = links.and_then(|l| l.next) {
        active_slot.entity = Some(next);
    }
}

pub fn handle_backspace_in_rune_slots(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut slots: Query<(&mut RuneSlot, Option<&RuneSlotLinks>)>,
) {
    let backspace_pressed = keyboard_input
        .read()
        .any(|ev| ev.state == ButtonState::Pressed && ev.key_code == KeyCode::Backspace);

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
    fn primary_foreground_set_uses_sprites_24_to_47() {
        assert_eq!(RuneSlotForegroundSet::Primary.sprite_index_for_rune(0), 24);
        assert_eq!(RuneSlotForegroundSet::Primary.sprite_index_for_rune(23), 47);
    }

    #[test]
    fn alternate_foreground_set_uses_sprites_48_to_95() {
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 0 }.sprite_index_for_rune(0),
            48
        );
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 0 }.sprite_index_for_rune(23),
            71
        );
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 1 }.sprite_index_for_rune(0),
            72
        );
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 1 }.sprite_index_for_rune(23),
            95
        );
    }

    #[test]
    fn alternate_foreground_page_is_clamped_to_supported_range() {
        assert_eq!(
            RuneSlotForegroundSet::Alternate { page: 99 }.sprite_index_for_rune(3),
            75
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
}
