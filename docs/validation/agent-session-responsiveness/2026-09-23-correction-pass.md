# Agent Session responsiveness and Capability Profile correction validation

Date: 2026-09-23  
Branch: `refinement/usability`

## Baseline

The observed slow session contained 6,711 persisted events over about 221 seconds and peaked at 80 events per second. Before this pass, every correlated event reloaded and reprojected the complete history, alongside a separate 1.5-second full-history poll. A read-only database query and JSON decode of that history took about 31.6 ms before IPC, React projection, Markdown rendering, and layout.

## Corrected behavior

- Capability Profile routes start collapsed and summarize their device, harness, and inference source.
- The model picker stays open while models are added or removed. Removed models retain their in-process settings so re-adding restores them.
- Native capability inventory is no longer shown or loaded through Capability Profiles. Harness-provided tool diagnostics remain with the selected Codex profile in Technical Settings.
- An unsent Agent Session selection follows a renamed Capability Profile revision when its execution route still exists. A removed route produces an actionable selection error instead of silently switching routes. Existing sessions remain pinned.
- Agent Session history is owned by one application-scoped source. It loads once, applies ordinary events in memory, batches non-terminal notifications to an animation frame, publishes terminal changes immediately, and performs one recovery load for a detected event gap.
- Transcript projection reuses completed invocation projections and incrementally folds appended events. The normal transcript omits raw payload, usage, transport, process, and configuration noise while retaining progress, tool activity, final outcomes, and actionable failures.
- Cached sessions paint immediately when selected; refreshes retain the stale snapshot until the new one is ready. Session controls and the selector are isolated from transcript rendering.
- Windows extended-length prefixes such as `\\?\C:\Users\user\.codex` are normalized for display without changing the stored path.

## Automated checks

Focused frontend suite:

```text
8 files passed
45 tests passed
```

The suite covered local route display, live history ownership, transcript projection and presentation, viewport behavior, Agent Session behavior, and Capability Profile editing.

Performance-oriented tests:

```text
10,000-event projection: 18 ms
80-event burst: 5 ms, one initial load, one frame publication
20 tests passed
```

The burst test verifies that event arrival does not trigger complete-history reloads. Terminal and gap-recovery cases also passed.

Additional checks:

```text
npm run build:frontend: passed
cargo check --manifest-path src-tauri/Cargo.toml: passed
draft_selection_follows_rename_but_never_switches_a_removed_route: passed
git diff --check: passed
```

Existing Rust warnings remain. The broad repository `npm test` command still includes unrelated baseline failures, including Node build scripts collected by Vitest and pre-existing navigation/workflow fixture mismatches; the focused correction suite is green.

## Running-app review

The application was launched from this checkout at `src-tauri/target/debug/codex-orchestrator.exe` with the local Vite frontend. App-inspector evidence is retained under `.dev/agent-session-responsiveness/`.

Verified in the running application:

- `profile-open.png`: saved profile opens with its route collapsed and without the native-inventory section.
- `model-picker.png`: the picker shows the full observed/cached model catalogue and marks selected models with Remove.
- `model-picker-after-add.png`: adding Astra leaves the modal open and changes its action to Remove.
- `agent-sessions.png`: a completed 24-update turn is rendered as compact progress rather than technical event output.
- `path-check-after.png`: route details display the friendly Codex profile path without the Windows extended-length prefix.
- Switching Capability Profiles to Agent Sessions and back retained the unsaved model edit, selected session, and transcript.

## Remaining validation boundary

A live provider-turn smoke test did not reach the provider: the submitted test invocation stopped at the selected target's pre-existing stale-worktree setup failure. The composer was cleared afterward. Responsiveness during provider event arrival is therefore proven by the bounded burst/history tests and existing-session running-app inspection, not by a newly completed provider turn in this pass.
