## New Features

When adding new features, be sure to ask questions eagerly about the behavior of any systems. Use the AskUserQuestion or similar tool for this.

As part of the feature, identify the points at which you can automate testing of that feature, including:

- unit tests: Architect the code such that small units of logic are easy to test, and then include unit tests for that.
- app tests: Create a test which creates the whole app and verify some part of the feature works
- snapshot tests: Use a utility to take a snapshot of some visible element. Ask the user to verify any changes present.

### Core functionality

The snapshots utility is not written yet. Write this as part of your first snapshot test. Here is the command to take a snapshot:
```
commands
.spawn(Screenshot::primary_window())
.observe(save_to_disk(path))
```

The snapshot utility should save to the checked snapshot location only when the `update` feature flag is set.