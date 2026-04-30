use LudumDare59::{configure_app, configure_loading, rune_words::battle::configure_battle};
use bevy::prelude::*;

#[cfg(test)]
use bevy::time::TimeUpdateStrategy;
#[cfg(test)]
use std::time::Duration;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Runic Ascendancy".into(),
            resolution: bevy::window::WindowResolution::new(1280_u32, 960_u32),
            ..default()
        }),
        ..default()
    }));
    configure_app(&mut app);
    configure_battle(&mut app);
    configure_loading(&mut app);
    app.run();
}

#[cfg(test)]
mod tests {
    use bevy::window::WindowResized;

    use super::*;

    fn create_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        configure_app(&mut app);
        app.add_message::<WindowResized>();

        // Use a fixed timestep so elapsed time progression is deterministic.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )));

        app
    }

    #[test]
    fn time_advances_between_frames() {
        let mut app = create_test_app();

        app.update();
        let elapsed_after_first_update = app.world().resource::<Time>().elapsed();
        app.update();
        let elapsed_after_second_update = app.world().resource::<Time>().elapsed();
        assert!(elapsed_after_second_update > elapsed_after_first_update);
    }
}
