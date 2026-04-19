use LudumDare59::{
    GameAssets, GameState, acceptance, configure_app, configure_loading,
    futhark::{self, FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    ui::hud_root::spawn_battle_hud_root,
};
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

const TEST_ID: u8 = 4;
const SPEED_MIN: f32 = 30.0;
const SPEED_MAX: f32 = 60.0;
const SPEED_STEP: f32 = 5.0;

#[derive(Component)]
struct TypedRuneDisplay;

#[derive(Component)]
struct SpeedLabel;

#[derive(Component)]
struct SpeedButton {
    delta: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    app.add_systems(
        OnEnter(GameState::Adventure),
        spawn_futhark_keyboard.after(spawn_battle_hud_root),
    );
    app.add_systems(OnEnter(GameState::Adventure), spawn_typed_rune_display);
    app.add_systems(OnEnter(GameState::Adventure), spawn_speed_controls);
    app.add_systems(
        Update,
        (update_typed_rune, handle_speed_buttons, sync_speed_label)
            .chain()
            .run_if(in_state(GameState::Adventure)),
    );
    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Displays only the rune that matches the most recently typed character",
    );
    app.run();
}

fn spawn_typed_rune_display(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        Sprite {
            image: game_assets.futhark.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: game_assets.futhark_layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 120.0, 0.0),
        Visibility::Hidden,
        TypedRuneDisplay,
    ));
}

fn spawn_speed_controls(mut commands: Commands, speed: Res<FutharkKeyboardAnimationSpeed>) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            right: Val::Px(16.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    SpeedButton { delta: -SPEED_STEP },
                ))
                .with_child((
                    Text::new("-"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

            parent.spawn((
                Text::new(format!("{:.0} °/s", speed.hue_degrees_per_second)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                SpeedLabel,
            ));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    SpeedButton { delta: SPEED_STEP },
                ))
                .with_child((
                    Text::new("+"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
        });
}

fn handle_speed_buttons(
    buttons: Query<(&Interaction, &SpeedButton), (Changed<Interaction>, With<Button>)>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    for (interaction, btn) in &buttons {
        if *interaction == Interaction::Pressed {
            speed.hue_degrees_per_second =
                (speed.hue_degrees_per_second + btn.delta).clamp(SPEED_MIN, SPEED_MAX);
        }
    }
}

fn sync_speed_label(
    speed: Res<FutharkKeyboardAnimationSpeed>,
    mut labels: Query<&mut Text, With<SpeedLabel>>,
) {
    if !speed.is_changed() {
        return;
    }
    for mut text in &mut labels {
        *text = Text::new(format!("{:.0} °/s", speed.hue_degrees_per_second));
    }
}

fn update_typed_rune(
    mut typed_rune_input: MessageReader<futhark::TypedFutharkInput>,
    mut display: Query<(&mut Sprite, &mut Visibility), With<TypedRuneDisplay>>,
) {
    let Some(last_typed) = futhark::last_typed_futhark_character(&mut typed_rune_input) else {
        return;
    };

    let Ok((mut sprite, mut visibility)) = display.single_mut() else {
        return;
    };

    if let Some(index) = futhark::letter_to_index(last_typed) {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = index;
            *visibility = Visibility::Visible;
        }
    } else {
        *visibility = Visibility::Hidden;
    }
}
