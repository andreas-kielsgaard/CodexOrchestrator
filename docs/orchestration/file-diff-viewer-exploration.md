# In-app file and diff viewer exploration

Status: development-only product exploration. Open with `?file-diff-viewer` in Vite development.

## Direction

One optional application tab uses the existing shell and Markdown renderer. An injected read-only
client supplies changed files for working-tree, staged, commit-range, generated-material, and
application-owned sources. The same screen handles navigation, counts, hunks, context expansion,
unified and split layouts, text or Markdown inspection, and explicit binary or unsupported states.

## Neutral contracts

`src/application/fileReview.ts` defines display-ready source, file, content, and hunk facts plus a
read-only client. Source IDs are opaque. Display paths are relative labels, not filesystem handles.
The presentation does not import Git, repository, worktree, artifact, or Tauri adapters.

## Security boundaries

- The adapter owns repository and worktree identity, source collection, and path/read authorization.
- Markdown skips raw HTML through the existing `AgentMarkdown` component.
- No edit, stage, discard, open-path, copy-path, or write operation exists in the viewer contract.
- Binary and unsupported content fail closed to named empty states.

## Prototype limits

- Recorded fixtures exercise the presentation; they do not prove live Git or artifact integration.
- Hunks arrive parsed and display-ready. Parsing, size limits, encoding detection, and truncation are
  unimplemented.
- Split pairing is line-oriented and does not perform word-level alignment.
- Syntax highlighting, search, comments, media playback, and virtualized large files are excluded.

## Visual verification

Local browser captures exercised the working-tree, staged, commit-range, generated-material, and
application-owned sources at 1440×900. They also covered changed-file selection, context expansion,
unified and split diffs, Markdown and text files, rename, binary, unsupported, and deleted states.
A fresh 780×900 capture exposed unnecessary horizontal overflow in unified mode; lowering only that
layout's minimum width removed it. The final narrow capture has no page, workspace, inspector, or
inspector-body horizontal overflow. The captured flow reported no browser console errors.

## User review points

- Changed-file density and the placement of additions/deletions.
- Unified versus split as the default.
- Provenance wording, especially generated versus persisted application-owned material.
- Whether renamed, deleted, binary, and unsupported states give enough context.
- Minimum useful behavior at narrow desktop widths.

## Exact next product slice

Add one authorized application adapter that resolves an existing application-owned changed-files
Document and stored diff artifact into `FileReviewSnapshot`, then open this same viewer from the
Sprint Documents surface. Include controller tests for identity, authorization, size limits,
encoding, binary detection, and unavailable artifacts. Keep live working-tree collection and every
write action out of that slice.
