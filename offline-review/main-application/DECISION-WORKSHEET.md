# Decision worksheet

Use this file as a scratchpad. Nothing here changes product state.

## Plan Builder experience

- [ ] Conversation clearly feels primary.
- [ ] Structured proposal clearly feels authoritative but subordinate.
- [ ] Epic name belongs where it is currently shown.
- [ ] Proposed Sprint hierarchy is understandable.
- [ ] Concern summaries have the right information density.
- [ ] Plan/Rebuild and ordinary discussion are distinguishable.

Notes:

---

## Agent Session experience

- [ ] Session list and conversation proportions feel right.
- [ ] Collapsing intermediate agent activity beneath the final answer is right.
- [ ] Processing can still be inspected when desired.
- [ ] Status, cancel, refresh, and copy controls are discoverable.
- [ ] Agent Session still feels provider- and workflow-neutral.

Notes:

---

## Initiation and transition language

Which terms are clear without documentation?

- [ ] Proposal submitted
- [ ] Initiation requested
- [ ] Awaiting confirmation
- [ ] Initiated
- [ ] Preparing Epic
- [ ] Bootstrap running
- [ ] Material accepted
- [ ] Epic Runner launched
- [ ] No Sprint started

Terms to rename or explain:

---

## Epic and Sprint detail

- [ ] The Epic overview gives enough orientation.
- [ ] The active Sprint is obvious.
- [ ] The Sprint flow map expresses dependency and sequence well.
- [ ] Plan revisions are understandable.
- [ ] Planner steps and Work Units are visually distinct.
- [ ] Flow, Concerns, and Documents are the right sibling views.
- [ ] The wide canvas is navigable without losing orientation.

Would fit-to-view, zoom, a minimap, or a different relational representation help?

Notes:

---

## Product control

- Should initiation always require confirmation, or may an explicit automatic policy authorize it?

  Notes:

- When an agent requests initiation, is the same modal sufficient context for the user?

  Notes:

- Which transition failures deserve immediate attention versus quiet retry?

  Notes:

- Should button-origin initiation always inform the Plan Builder on its next query?

  Notes:

## Harness policy

- Should model and reasoning remain inherited for Plan Builder, Bootstrap Generator, and Epic
  Runner?

  Notes:

- Which harness settings should eventually be user-editable?

  Notes:

- Should editing create a new profile version for future invocations only?

  Notes:

## Next product movement

Rank:

1. Finish Epic Runner → one Sprint → reviewed result.
2. Improve Plan Builder UX.
3. Refactor large orchestration modules before expansion.
4. Integrate Harness Inspector.
5. Integrate File and Diff Viewer with product Documents.
6. Build Test Session Host.
7. Build worktree instance registry/process ownership.

Preferred order and reasoning:

---

## Structural concerns

List anything that feels like a new monolith, duplicated truth, misplaced responsibility, or
premature abstraction:

---

## Decisions safe to defer

Record questions that are interesting but should not shape the next Sprint yet:

---
