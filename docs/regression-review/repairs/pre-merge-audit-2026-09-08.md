# Pre-merge audit — 8 September 2026

This audit covers `codex/harness-ux-workflow-convergence` at `7d794e4`, plus the browser-check updates recorded with this note. It does not merge the branch.

## Functional checks

The replacement path passed these focused checks:

- 16 Session Event tests.
- The Workflow user-entry test. It creates a Session and records prompt sources in order.
- 9 application repair tests. They cover pinned Session Profiles, second messages, instance isolation, recipe pinning, file prompt sources, normal completion, application events, and the MCP handoff path.
- 14 focused frontend tests for the shared graph, Workflow authoring, Workflow instances, Session Event editors, per-message controls, collapsible sections, and Tauri clients.
- The TypeScript and Vite production build.
- The Epic confirmation modal passed its five tests when run on its own.

The first Rust command reused stale Cargo output from a removed worktree and could not build. The checks above used a new `CARGO_TARGET_DIR` and passed. A normal validation run should either use an isolated target directory or clear the stale build output when no other test run is using it.

One repository-wide test remains flaky: `native_profiles::tests::concurrent_reporting_dispatches_adopt_one_pending_request`. It failed once and passed on the next isolated run. The feature diff does not change this test or the reporting flow it exercises. This still prevents an honest all-green claim.

## Browser flow

The browser check uses the real React screens with fake clients. It now follows the current node-based instance view.

1. Move nodes, connect them, and save the draft.
2. Open and collapse node sections at 1280, 958, 850, and 640 pixels.
3. Open the instance dialog at each width.
4. Create an instance without starting a Session.
5. Open the start node and send its first message.
6. Open the created Session in the node panel.
7. Leave and reopen the instance.
8. Reload the page, reopen the instance, and reopen its Session.

The run passed with no page errors. See [recorded results](evidence/results.json), [the saved instance](evidence/05-saved-instance.png), and [the Session in the node view](evidence/06-instance-session.png).

## Visual review

- At 1280 and 958 pixels, the Workflow canvas, node editor, connection editor, instance dialog, instance graph, and Session panel are clear and aligned.
- The instance view now uses the same graph parts as the definition view. Nodes and edges keep the same visual language.
- The Session panel clearly separates the Session identity, its pinned settings, the transcript, and the composer.
- The identity editor clearly shows the color and circle, square, and hexagon choices.
- At 850 and 640 pixels, the important controls remain available. The editor covers most of the canvas, which is a reasonable trade-off for now. The top product navigation scrolls sideways; its last item is only partly visible at 640 pixels until scrolled.
- The old Harness Management preview is still available only when its development surface is mounted. It looks finished, but it describes the old concept and overlaps with Capability Profiles and Session Profiles.
- The main Agent Sessions list still projects Sessions into Epic or Independent groups. In the browser fixture, the Workflow-created Session therefore appears as independent. The Workflow instance view itself shows the correct node and Session relation.

## Evidence limits

- The browser run does not call a provider and does not use the native desktop window.
- Browser persistence is fixture storage. SQLite persistence and pinned runtime truth are covered by the Rust tests.
- The saved walkthrough deck still contains older list-style Workflow instance images. The current regression images above are the reliable visual evidence for this branch.

## Merge choices still open

Before merging, it would be reasonable to decide these points explicitly:

1. Whether the development-only Harness Management preview should stay for reference or be removed with the old concept.
2. Whether the old Epic/Independent Agent Sessions projection may remain until the later runtime-projection work, or whether main should wait for that work.
3. Whether to fix the native-profile concurrency flake now or accept it as a known repository-wide gate issue.
4. Whether the old walkthrough decks and screenshots belong in main. The current regression evidence is smaller and matches the present UI.
