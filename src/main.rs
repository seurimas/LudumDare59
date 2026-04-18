use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    app.run();
}

fn configure_app(app: &mut App) {
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        configure_app(&mut app);

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
