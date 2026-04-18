use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    prelude::{
        App, Assets, Camera, Commands, Component, Handle, Image, On, PreUpdate, Query, Res, ResMut,
        Resource, Startup, Update, With,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Resource, Clone)]
struct SnapshotRenderTarget(pub Handle<Image>);

#[derive(Component)]
struct SnapshotRenderTargetApplied;

#[derive(Resource, Default)]
struct SnapshotRequests(VecDeque<&'static str>);

/// Configures an app to support `take` by preparing an off-screen render target
/// and auto-assigning it to cameras as they are spawned.
pub fn initialize_app(app: &mut App) {
    app.insert_resource(SnapshotRequests::default());
    app.add_systems(Startup, ensure_render_target_exists);
    app.add_systems(PreUpdate, apply_render_target_to_new_cameras);
    app.add_systems(Update, process_snapshot_requests);
}

fn ensure_render_target_exists(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d {
        width: 320,
        height: 180,
        depth_or_array_layers: 1,
    };

    let mut render_target = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // Screenshot capture requires copying from this texture on the GPU.
    render_target.texture_descriptor.usage |=
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let handle = images.add(render_target);
    commands.insert_resource(SnapshotRenderTarget(handle));
}

fn apply_render_target_to_new_cameras(
    target: Option<Res<SnapshotRenderTarget>>,
    mut commands: Commands,
    cameras: Query<
        bevy::prelude::Entity,
        (
            With<Camera>,
            bevy::prelude::Without<SnapshotRenderTargetApplied>,
        ),
    >,
) {
    let Some(target) = target else {
        return;
    };

    for entity in &cameras {
        commands.entity(entity).insert((
            RenderTarget::Image(target.0.clone().into()),
            SnapshotRenderTargetApplied,
        ));
    }
}

fn process_snapshot_requests(
    mut commands: Commands,
    target: Option<Res<SnapshotRenderTarget>>,
    mut requests: ResMut<SnapshotRequests>,
    cameras: Query<&RenderTarget, With<Camera>>,
) {
    if requests.0.is_empty() {
        return;
    }

    let Some(target) = target else {
        panic!("snapshot::initialize_app must create a snapshot render target before capture");
    };

    let has_targeted_camera = cameras.iter().any(|render_target| {
        if let RenderTarget::Image(image_target) = render_target {
            image_target.handle.id() == target.0.id()
        } else {
            false
        }
    });

    assert!(
        has_targeted_camera,
        "snapshot::take requires at least one camera with the snapshot render target; spawn a camera after calling snapshot::initialize_app"
    );

    while let Some(name) = requests.0.pop_front() {
        println!("[snapshot] spawning Screenshot entity for '{name}'");
        commands
            .spawn(Screenshot::image(target.0.clone()))
            .observe(snapshot_observer(name));
    }
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}.png"))
}

fn snapshot_last_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}_last.png"))
}

/// Spawns an off-screen camera, captures one frame, and attaches the snapshot observer.
///
/// When the `update` feature is set, saves the captured image as the new baseline.
/// Otherwise, compares against the saved baseline and panics if they differ or none exists.
/// The captured image is always written to `tests/snapshots/<name>_last.png`.
pub fn take(commands: &mut Commands, name: &'static str) {
    commands.queue(move |world: &mut bevy::prelude::World| {
        let mut requests = world
            .get_resource_mut::<SnapshotRequests>()
            .unwrap_or_else(|| {
                panic!("snapshot::initialize_app must be called before snapshot::take")
            });
        requests.0.push_back(name);
    });
}

fn snapshot_observer(
    name: &'static str,
) -> impl FnMut(On<ScreenshotCaptured>) + Send + Sync + 'static {
    move |trigger: On<ScreenshotCaptured>| {
        println!("[snapshot] observer fired for '{name}'");
        let img = trigger
            .image
            .clone()
            .try_into_dynamic()
            .unwrap_or_else(|_| panic!("Failed to convert screenshot '{name}' to image"));
        let actual = img.to_rgba8();

        assert!(
            actual
                .pixels()
                .any(|pixel| pixel.0[0] != 0 || pixel.0[1] != 0 || pixel.0[2] != 0),
            "Snapshot '{name}' captured a black frame (all zero RGB). Rendering output was not produced."
        );

        let last_path = snapshot_last_path(name);
        std::fs::create_dir_all(last_path.parent().unwrap())
            .expect("Failed to create snapshots directory");
        img.save(&last_path)
            .unwrap_or_else(|e| panic!("Failed to save _last snapshot '{name}': {e}"));

        let path = snapshot_path(name);

        #[cfg(feature = "update")]
        {
            img.save(&path)
                .unwrap_or_else(|e| panic!("Failed to save snapshot '{name}': {e}"));
            println!("Snapshot updated: {path:?}");
        }

        #[cfg(not(feature = "update"))]
        {
            assert!(
                path.exists(),
                "No snapshot found at {path:?}. Run with `--features update` to create one."
            );
            let expected = image::open(&path)
                .unwrap_or_else(|e| panic!("Failed to load snapshot '{name}': {e}"));
            assert_eq!(
                actual.into_raw(),
                expected.to_rgba8().into_raw(),
                "Screenshot does not match saved snapshot '{name}'"
            );
        }
    }
}
