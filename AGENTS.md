# Types of work

When you are asked to do something, it should fall under a certain umbrella. Based on that umbrella, read a markdown file for more instructions.

- tasks/TASK_FEATURE.md
  - This is a brand new feature. It is not updating an old feature. It is not fixing a bug.
- tasks/TASK_REFINE.md
  - This is an update to a feature, which may include new features as well. It is not fixing a bug.
- tasks/TASK_BUGFIX.md
  - This is a bugfix.

## Post-work checklist

After completing ANY work, always run `cargo test`. Additionally, identify any new or changed UAT tests. Run them and expect a zero output. If you get a non-zero output, use AskUserQuestion or similar tool to ask what went wrong and iterate on the feedback.

Verify that your changes have not introduced new problems. When that is complete, go ahead and do all of the following:

```
cargo fmt
cargo build
git add .
git commit -m "<A meaningful commit message>"
```

Then, report on the results to the user.

## Important bevy 0.18 things

Old bevy had `add_startup_system`. Now, you `add_system(Startup, <system>)`.