## Refinements

A refinement updates an existing feature or adds sub-features to it. It is not a bugfix.

### Before starting

- Read the existing code for the feature being refined. Do not assume the current state matches earlier instructions.
- If the scope is ambiguous (e.g. "make it better"), ask the user to clarify exactly what should change.

### Making changes

- Prefer the smallest diff that satisfies the request. Do not refactor unrelated code.
- If moving code between files (e.g. splitting a module), verify that all public types and functions remain accessible to callers.
- If a module gains a `pub fn configure_<name>(app: &mut App)`, wire it into `lib.rs` and `main.rs`.

### UAT for refinements

- If the refinement is visually observable, run the relevant UAT binary and confirm it passes (F1, exit code 0).
- If an existing UAT no longer reflects the feature after the change, update it.
- Do not add test-only visual logic to the game library — it belongs in the UAT binary.
