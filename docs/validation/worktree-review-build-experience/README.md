# Worktree Review build experience validation

Validated on Windows from a packaged release controller on 2026-09-24.

## Native flow

- Opened **Create a build** and verified the three prescribed source descriptions.
- Started a live-worktree build, navigated to another branch while it ran, and started a second build. Both used distinct Cargo targets and application identifiers.
- Observed terminal notification dots on the Worktree Review tab and the exact originating branch. Loading that branch cleared its dot.
- Verified initiated time, **Rebuild**, **Launch**, and available-output presentation.
- Launched two retained builds concurrently as distinct responsive native processes.
- Re-launched one build and observed **Focused existing build** with the same surviving PID.
- Verified one private AppData root and WebView2 root per build. The reviewed process did not inherit the controller's WebView debugging port.

## Compatibility correction found during validation

The first older-branch launch exposed a controller-schema 58 / reviewed-schema 55 mismatch. The final implementation detects the schema declared by the reviewed source. It copies controller databases only for an exact schema match; otherwise it mirrors safe non-database configuration and lets the reviewed application initialize its own database.

The corrected packaged flow recorded `controllerSchemaVersion: 58`, `targetSchemaVersion: 55`, and `databaseSeeded: false`; the older reviewed build then created schema 55, remained responsive, and focused on repeat launch. See `corrected-older-branch-native-observation.json` and `corrected-repeat-launch-observation.json`.

## Evidence index

- `create-build-modal.png`: modal placement and prescribed build-source copy.
- `first-build-running.png`, `second-branch-while-first-running.png`, `second-build-running.png`: navigation and concurrent build flow.
- `first-build-terminal.png`, `second-build-terminal.png`: application and exact-branch terminal dots.
- `first-build-available.png`: initiated time, simple availability, Rebuild and Launch.
- `corrected-older-branch-launch.png`: successful older-schema launch through the corrected controller.
- `corrected-repeat-launch.png`: repeat launch reports focus.
- `corrected-simultaneous-launches.png`: two retained reviewed builds launched together.
- `corrected-older-branch-native-observation.json`: seed manifest, process, window and debug-port evidence.
- `corrected-repeat-launch-observation.json`: same-PID focus and target database version.
- `corrected-simultaneous-instance-observation.json`: two distinct responsive reviewed processes.

Automated coverage: lint, frontend build, 45 Worktree Review frontend tests, 22 build-tool tests, 75 focused Rust tests, the 768-test repository suite, and packaged release builds.
