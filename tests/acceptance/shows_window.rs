use LudumDare59::acceptance;
use bevy::prelude::*;

const TEST_ID: u8 = 1;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    acceptance::initialize_app(&mut app, TEST_ID, "Shows window");
    app.run();
}
