use LudumDare59::{
    GameState, acceptance, acceptance::AcceptanceTest, configure_app, configure_loading,
};
use bevy::prelude::*;

const TEST_ID: u8 = 3;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    app.add_systems(OnEnter(GameState::Ready), spawn_ready_confirmation);
    acceptance::initialize_app(
        &mut app,
        AcceptanceTest::from(TEST_ID).with_grid(),
        "Processes audio one rune per frame with fade-in animation, then enters Ready",
    );
    app.run();
}

fn spawn_ready_confirmation(mut commands: Commands) {
    commands.spawn((
        Text::new("Processing complete — press F1 to pass, F2 to fail"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            ..default()
        },
    ));
}
