## Bugfixes

A bugfix corrects incorrect behaviour. It does not add new features or refactor working code.

### Before starting

- Reproduce the bug first. Understand exactly when it occurs and why.
- Read the relevant code before changing it.

### Making the fix

- Change only what is necessary to fix the bug. Do not improve unrelated code.
- If the bug was caused by a misuse of a Bevy API, check `AGENTS.md` for relevant notes and add one if it is missing.

### Verifying the fix

- Run `cargo test`. All existing tests must still pass.
- If the bug was user-visible, run the relevant UAT binary and confirm it passes (F1, exit code 0).
- If no UAT covers the scenario, consider adding one so the bug cannot regress silently.
