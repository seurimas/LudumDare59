use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading,
    rune_words::{battle::configure_battle, battle_states::binding::BindingSucceeded},
    spellbook::LearnedSpells,
    ui::spell_selection::SpellSelection,
};
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

const TEST_ID: u8 = 12;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    configure_battle(&mut app);

    app.add_systems(OnEnter(GameState::Adventure), spawn_instructions);
    app.add_systems(
        Update,
        (fire_binding_success_on_f3, update_status_label).run_if(in_state(GameState::Adventure)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Spell selection: F3 triggers BindingSucceeded. A modal shows two un-learned spells; clicking one learns it.",
    );

    app.run();
}

#[derive(Component)]
struct StatusLabel;

fn spawn_instructions(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(48.0),
                top: Val::Px(40.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            ZIndex(200),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(
                    "Press F3 to trigger a binding success and open the spell selection window.",
                ),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            panel.spawn((
                Text::new("F1 = pass, F2 = fail"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            panel.spawn((
                StatusLabel,
                Text::new("Learned: -"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.85, 0.95)),
            ));
        });
}

fn fire_binding_success_on_f3(
    input: Res<ButtonInput<KeyCode>>,
    mut succeeded: MessageWriter<BindingSucceeded>,
) {
    if input.just_pressed(KeyCode::F3) {
        succeeded.write(BindingSucceeded);
    }
}

fn update_status_label(
    learned: Res<LearnedSpells>,
    selection: Res<SpellSelection>,
    mut labels: Query<&mut Text, With<StatusLabel>>,
) {
    let open = selection.is_open();
    let text = format!(
        "Learned [{}]: {}   |   Selection open: {}",
        learned.words.len(),
        learned.words.join(", "),
        open,
    );
    for mut label in &mut labels {
        if label.0 != text {
            label.0 = text.clone();
        }
    }
}
