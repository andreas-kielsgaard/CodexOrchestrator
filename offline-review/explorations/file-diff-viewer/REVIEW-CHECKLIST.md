# File and Diff Viewer review checklist

Use this checklist after following the offline launch steps in [README.md](README.md).

## Start

- [ ] Branch is `codex/explore-file-diff-viewer`.
- [ ] Commit is `9de25c2`.
- [ ] `node_modules\.bin\vite.cmd` exists; no installation or network access is needed.
- [ ] The URL includes `?file-diff-viewer`.
- [ ] **Files & diffs** is an in-app peer tab and opens without Agent Sessions.

## Sources and files

- [ ] Open each source: Working tree, Staged changes, Commit range, Generated material, and
      Application-owned record.
- [ ] Confirm source labels and details distinguish recorded provenance without claiming live data.
- [ ] In Working tree, select each changed file and inspect additions/deletions.
- [ ] Confirm the renamed file shows its previous relative path.
- [ ] Confirm the deleted unsupported file remains in navigation.

## Inspection modes

- [ ] Expand and collapse unchanged context above and below the first hunk.
- [ ] Compare **Unified** and **Split**.
- [ ] Select **File** on the Markdown example and inspect headings, table, quote, and code block.
- [ ] Select **File** on `FileReviewScreen.tsx` and confirm plain text is read-only.
- [ ] Open the binary video and confirm the explicit binary state.
- [ ] Open the deleted Sketch file and confirm the explicit unsupported state.

## Authority and truthfulness

- [ ] No edit, save, stage, discard, open-path, copy-path, or write control is present.
- [ ] Displayed paths read as relative labels, not authorized filesystem paths.
- [ ] Working-tree, staged, and commit-range examples are understood as recorded fixtures.
- [ ] Generated material says it is not persisted.
- [ ] Application-owned material says it is a durable product record.

## Product review

- [ ] File-list density and additions/deletions are understandable.
- [ ] Choose a preferred default: Unified or Split.
- [ ] Provenance wording is clear across all five source classes.
- [ ] Rename, delete, binary, and unsupported states provide enough context.
- [ ] Narrow desktop behavior remains useful.

## Integration gate

- [ ] Record the known gap: an empty `listSources()` result currently appears to load forever.
- [ ] Require an explicit no-sources state and proof in the first real integration.
- [ ] Treat the Document plus stored-diff integration as a candidate, not authorization.
- [ ] Keep live Git collection and all write actions outside that candidate slice.

## Stop

- [ ] Stop Vite with `Ctrl+C`.
- [ ] Leave branch `codex/explore-file-diff-viewer` at `9de25c2`.
- [ ] Do not commit, merge, or push from this review.
