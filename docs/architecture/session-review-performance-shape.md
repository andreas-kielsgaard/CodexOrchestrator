# Session review corrections

## Evidence and choices

Native baseline: navigation IPC 50.7 ms, a 682 KB history load 95 ms;
80 typed characters had frame latency median 14.7 ms, p95 16.2 ms, max 95.7 ms.
CPU samples identify `useComposerAutosize`'s zero-height/scrollHeight layout as
the largest sampled application function. This trace did not reproduce sustained
stuttering or establish draft storage as its primary cause.

Use native CSS field sizing where available. Keep an older-WebView fallback.
Keep React, existing ModalDialog, branch picker/graph and windows-sys; a rich
editor dependency is not needed for recognized skill-reference highlighting.
The official Tauri clipboard plugin does not supply Windows file-list paste;
use the existing Windows binding for the small native clipboard adapter.

Assumptions: long paste exceeds 4,000 characters or 50 lines; file contents are
portable and materialized on the execution device; skill metadata is shown only
when supplied by its source. No inferred creation dates.

## Ownership

- Extract draft editing/persistence into `composerDraftStore.ts`. The composer
  alone subscribes; session orchestration reads the latest draft on submission.
  Persist after 500 ms idle, at most 15 seconds without a checkpoint, and flush
  on navigation/unmount. Remove keystroke-driven screen state/storage effects.
- Adapt `useComposerAutosize.ts` to observe viewport size once, not recreate
  observers or force a transcript-wide layout for each character.
- Retain the app-scoped history cache. Cache collection snapshots across view
  mounts, and render cached history while auxiliary profile/queue data loads.
  Batch repository history reads where profiling proves useful; keep decoding
  and query code in focused repository modules, not the transport.
- The large conversation contains 226,287 events and 154 MB of payloads, including
  205,959 output deltas. Add a display-history read omitting known unnormalized
  wire telemetry before deserialization. Keep raw history intact. Carry its
  durable sequence cursor; ignored telemetry advances it without a React update.
  Bound oversized action previews and fetch exact durable event details on
  expansion or full-detail export. Keep raw records unchanged and scope reads
  to the owning session. Sidebar request counts read only request-control facts.
- Add repository-owned attention state and navigation commands. A notification
  refers to an unread fully completed turn. Hollow means subsequent queued work
  remains, not intermediate text. Viewing acknowledges; menus can mark unread.
- Retain SessionContextMenu/BranchContextMenu, add compact menu groups and
  overlaid lateral submenus. Reuse SessionTargetDialog/BranchGraphBrowser for
  the Sessions menu's explicit Create worktree action.
- Extract attachment conversion and clipboard access from the composer. Extend
  existing portable attachment records and provider materialization for files;
  keep image delivery native. Preview newest, including pastes. Text-to-file
  conversion and explicit paste share the same attachment entry point.
- Add catalogue-backed skill-reference presentation and a lazy information
  modal. Keep plain text as the submission/storage format; skill content is
  loaded on demand, never per keystroke.
- Keep `review-tools/app-inspector/session-performance.mjs` as a repeatable
  owned-native baseline/after trace. Store raw measurements outside the repo.
- Group copy utilities in a lateral submenu. Export four cumulative Markdown
  levels: messages, reasoning, action summaries with filenames, full available
  action details. Omit unsent steering; retain chronology and credential redaction.

## Verification

Repeat the same native input/history scenario before and after. Include long
drafts and a large history, separate IPC cost from input/layout cost, and report
limits. Test durable read state, full-turn continuation, draft checkpoints,
clipboard/file transport, menu geometry and skill metadata. No commit, merge or
exe publication is part of this follow-up until requested.

Native after trace (isolated review database, rebuilt worktree app):

- Same 80-character input: frame latency median 15.9 ms, p95/max 16.2 ms;
  no long tasks in that trace. Baseline p95 was already 16.2 ms; the observed
  95.7 ms maximum did not recur. This is a single scenario, not a broad benchmark.
- Large-history display: 5,993 events, 16.7 MB, 1.68 seconds. Deferring large
  action bodies reduced the intermediate 43.3 MB / 3.0-second read; canonical
  raw history remains intact. Cold opening is still not instantaneous.
- Returning to the cached large conversation: 117 ms. Long-draft input handling
  was 2 ms median / 5 ms p95 in the final run, with height capped at two-thirds
  of the workspace.
- Native flows verified: four Copy levels, lateral flyout geometry, skill
  metadata/content modal, branch-graph worktree selection/cancel, text/file
  clipboard paste, long-paste attachment, newest image preview, unread/read dots.
  No real provider send or worktree creation was performed in this pass.
