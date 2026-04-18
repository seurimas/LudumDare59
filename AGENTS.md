# Types of work

When you are asked to do something, it should fall under a certain umbrella. Based on that umbrella, read a markdown file for more instructions.

- tasks/TASK_FEATURE.md
  - This is a brand new feature. It is not updating an old feature. It is not fixing a bug.
- tasks/TASK_REFINE.md
  - This is an update to a feature, which may include new features as well. It is not fixing a bug.
- tasks/TASK_BUGFIX.md
  - This is a bugfix.

## Running UAT binaries

Always run UAT binaries with `cargo run --bin <name>` from the project root. This ensures the `assets/` directory is found relative to the working directory. Never launch the compiled `.exe` directly (e.g. via `Start-Process` or by path), as it will not find assets.

Example:
```
cargo run --bin uat_shows_loading_rune_reveal
```

## Pre-work checklist

Do `git log -3` before beginning work. This will highlight the most recent work and where progress is being made. This may help inform your current task, but do not assume it should if the link is not clear.

## Post-work checklist

After completing ANY work, always run `cargo test`. Additionally, identify any new or changed UAT tests. Run them with `cargo run --bin <name>` and expect a zero output. YOU must run the UAT when it is appropriate to do so. If you get a non-zero output, use AskUserQuestion or similar tool to ask what went wrong and iterate on their feedback.

Verify that your changes have not introduced new problems. When that is complete, go ahead and do all of the following:

```
cargo fmt ; cargo build
git add .
git commit -m "<A meaningful commit message>"
```

Then, report on the results to the user.

## Important bevy 0.18 things

- Old bevy had `add_startup_system`. Now, you `add_systems(Startup, <system>)`.
- State lifecycle hooks: `add_systems(OnEnter(State::Variant), ...)` and `add_systems(OnExit(State::Variant), ...)`.
- Register states with `app.init_state::<MyState>()`.
- `TextureAtlasLayout::from_grid` takes `UVec2`, not `Vec2` (e.g. `UVec2::splat(32)`).
- `despawn()` in bevy 0.18 despawns the entity and its children by default.
- `TextFont` is the component for font settings on `Text` nodes (not `TextStyle`).

## Asset loading preferences

- Always use `bevy_asset_loader` (crate version 0.26 for bevy 0.18) for loading assets. Do not manually poll `AssetServer::is_loaded_with_dependencies`.
- Derive `AssetCollection, Resource` on the assets struct and annotate fields with `#[asset(...)]` attributes.
- Use `#[asset(texture_atlas_layout(tile_size_x = N, tile_size_y = N, columns = C, rows = R))]` for atlas layouts — no separate manual construction needed.

## Code organisation preferences

- Keep asset loading in its own module: `src/loading.rs` with a `pub fn configure_loading(app: &mut App)` entry point.
- Separate `configure_app` (global settings like `ClearColor`) from `configure_loading` (state machine + asset pipeline).
- UAT-specific setup (e.g. spawning test sprites) must live inside the UAT binary, not in the game library. The library's `OnEnter(GameState::Ready)` should be left empty unless it is real game logic.

## Commit messages

Keep commit messages short (one line). Do not sign with a co-author tag or your name.

## Memory

Do not write memories to the external memory system. Keep all persistent notes in checked-in files like this one.