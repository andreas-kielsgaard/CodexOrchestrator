# Navigation and composer integration

14 September 2026. Combines navigation `6488967` with main's cleanup and composer quick features at `8965039`.

## Result

- Repository and workflow navigation, ordering, pins, disclosure, workflow jumps, deeplinks and agent commands remain intact.
- Composer discovery uses the repository main working tree for a new folder draft. Switching drafts refreshes discovery; saved sessions retain their execution directory regardless of visual placement.
- Discovery and creation share one folder-directory resolver. The conversation hook retains main's quick-menu integration while collection state stays separate.
- Main's retired simulator, legacy command handlers and tooling remain deleted. The minimal crate entry point and development dependency placement are preserved.

## Validation

- Frontend: 127 files / 700 tests passed, including combined folder discovery, message selections and conversation lifecycle coverage.
- TypeScript/Vite build, targeted ESLint and native `test-fast` build passed.
- Rust folder creation/discovery and saved-session context checks passed. The broader run's work-slice planning test failed under concurrent execution, then passed in isolation without code changes.
- The rebuilt native app opened the existing isolated demo. Agent commands exercised pinned ordering, pinned selection without expanding folders, independent group collapse, exact workflow/node/session navigation and an unsent workflow-folder draft. Original order and disclosure were restored; the demo still contains 22 sessions.
- Native visual inspection confirmed the merged navigation and folder-draft composer. The demo has no configured provider profile, so this integration run did not validate a live provider send or live model/skill discovery.

Integration targets local main only. Unrelated modified and untracked documentation remains outside the merge commit.
