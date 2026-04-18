## New Features

When adding new features, be sure to ask questions eagerly about the behavior of any systems. Use the AskUserQuestion or similar tool for this.

As part of the feature, identify the points at which you can automate testing of that feature, including:

- unit tests: Architect the code such that small units of logic are easy to test, and then include unit tests for that.
- app tests: Create a test which creates the whole app and verify some part of the feature works
- user acceptance tests: Create a binary that launches the app in a specific state so the user can visually verify behavior. See below.

### Core functionality

User acceptance test utility:

- Implemented in `src/acceptance.rs`.
- Public API includes:
	- `initialize_app(app: &mut App, test_id: u8, description: &str)`: adds keybinding systems and sets the window title to include the test description (e.g. "UAT #3: Can see flames on the screen"). Adds:
		- F1 keybinding: exits the process with status code 0 (pass).
		- F2 keybinding: exits the process with status code equal to the `test_id` (fail).
	- Each acceptance test must have a unique, hardcoded `test_id` (a nonzero `u8`).

Expected acceptance test flow:

- Create a new binary target under `tests/acceptance/` (e.g. `tests/acceptance/test_flames.rs`).
- Build the full app with `DefaultPlugins`, call `configure_app`, then call `acceptance::initialize_app(app, TEST_ID, "Can see flames on the screen")`.
- Set up whatever game state is needed to demonstrate the feature.
- The user runs the binary, visually inspects the result, and presses F1 (pass) or F2 (fail).
- The process exit code communicates the result; a CI wrapper or the user can check `$LASTEXITCODE` / `$?`.
- After the process exits, if the exit code is nonzero (F2 / fail), **immediately use the AskUserQuestion tool** to ask the user what went wrong. Iterate on their feedback until the UAT passes.