use bevy::ecs::message::MessageReader;
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
    active_slot: Res<ActiveRuneSlot>,
    mut slots: Query<&mut RuneSlot>,
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

    let Ok(mut slot) = slots.get_mut(active_entity) else {
        return;
    };

    slot.rune_index = Some(index);
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

    #[test]
    fn active_slot_receives_typed_rune_updates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        futhark::configure_futhark_keyboard(&mut app);
        configure_rune_slots(&mut app);
        app.add_systems(
            Update,
            (
                activate_rune_slot_on_click,
                update_active_rune_slot_from_typed_input,
            )
                .chain(),
        );

        let inactive_slot = app
            .world_mut()
            .spawn((
                Interaction::None,
                RuneSlot {
                    rune_index: None,
                    foreground_set: RuneSlotForegroundSet::Primary,
                },
            ))
            .id();
        let active_slot = app
            .world_mut()
            .spawn((
                Interaction::Pressed,
                RuneSlot {
                    rune_index: None,
                    foreground_set: RuneSlotForegroundSet::Primary,
                },
            ))
            .id();

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
            .expect("slot should exist")
            .rune_index;
        let inactive_rune = app
            .world()
            .entity(inactive_slot)
            .get::<RuneSlot>()
            .expect("slot should exist")
            .rune_index;

        assert_eq!(active_rune, Some(1));
        assert_eq!(inactive_rune, None);
    }
}
