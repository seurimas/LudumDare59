use bevy::prelude::*;

pub mod acceptance;

pub fn configure_app(app: &mut App) {
    let _ = app
        // Set the clear color to blue
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 1.0)));
}
