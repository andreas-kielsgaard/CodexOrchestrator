# In-app file and diff viewer exploration

Status: development-only product exploration. Open with `?file-diff-viewer` in Vite development.
The development harness can bind a deterministic fixture with `file-review-fixture=working-tree`,
`staged`, `commit-range`, `generated`, or `application-owned`. The default is `working-tree`.

## Direction

One optional application tab uses the existing shell and Markdown renderer. An injected read-only,
already-scoped source supplies one normalized file-review snapshot. The screen handles navigation,
counts, hunks, context expansion, unified and split layouts, text or Markdown inspection, and
explicit binary or unsupported states. It does not select or explain technical origins.

## Neutral contracts

`src/application/fileReview.ts` defines display-ready file, content, and hunk facts plus one scoped
`FileReviewSource`. Display paths are relative labels, not filesystem handles. The presentation does
not import Git, repository, worktree, artifact, or Tauri adapters.

`src/application/applicationOwnedFileReview.ts` resolves one changed-files Document and stored diff
artifact through separate read-only ports. It rechecks Document authorization, matches Document,
artifact, and changed-file identities, bounds artifact bytes, requires UTF-8 JSON, and names binary
or unsupported file content without exposing storage locators.

## Security boundaries

- The originating context and adapter own selection, authorization, retrieval, and normalization.
- Markdown skips raw HTML through the existing `AgentMarkdown` component.
- No edit, stage, discard, open-path, copy-path, or write operation exists in the viewer contract.
- Binary and unsupported content fail closed to named empty states.

## Prototype limits

- Five deterministic fixtures exercise the same scoped presentation without adding product origin
  controls. They do not prove live Git or a native artifact backend. One recorded Document and
  artifact fixture exercise the application-owned ports.
- Linking working changes, staged changes, commit ranges, generated material, or Documents to this
  view is deferred.
- Stored hunks remain parsed and display-ready. The adapter validates their shape and derives counts;
  unified-diff parsing and truncation remain unimplemented.
- Split pairing is line-oriented and does not perform word-level alignment.
- Syntax highlighting, search, comments, media playback, and virtualized large files are excluded.

## Visual verification

Browser measurements at 1440×900 and 780×900 switched Changes to File without moving the
inspection-mode group or its reserved 152 px layout slot on either axis. In File mode the layout
group was absent and the slot contained no focusable controls. Both sizes had zero horizontal page,
workspace, inspector, or inspector-body overflow, no origin selector, and no console warnings or
errors.

## User review points

- Changed-file density and the placement of additions/deletions.
- Unified versus split as the default.
- Whether renamed, deleted, binary, and unsupported states give enough context.
- Minimum useful behavior at narrow desktop widths.

## Application-owned adapter proof

Tests cover identity, authorization, size bounds, UTF-8 and unsupported encoding, binary detection,
unavailable artifacts, and the empty scoped-review state. Product boot still supplies no file-review
source. Entry-point integration and a focused native read adapter remain separate future work.
