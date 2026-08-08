---
name: complete-plan-slice
description: Complete and return one ad-hoc Plan Slice after its required Plan Steps are dispositioned. Use in the Plan Slice conversation to judge combined acceptance and report the result to the Overall Plan conversation.
---

# Complete Plan Slice

Evaluate the slice as a combined result, not merely as a collection of accepted Plan Steps.

Check the slice objective and exit condition, integration and coherence across results, required validation, regressions or standards relevant to the slice, residual risks, deferrals, and unproven boundaries. Confirm that every accepted Plan Step which changed repository state returned a committed checkpoint and that the resulting commit sequence represents the accepted slice state on one named Slice branch.

Reuse accepted Plan Step evidence for unchanged checkpoints. Use the Slice Plan's contract edges and shared surfaces to identify interactions no individual Step could prove. Choose combined validation for risks introduced by composition, integration, or an uncovered slice-level boundary, such as extensible variants, ordering and correlation, privacy or negative authority, reopen and recovery, or competing writers and race winners. These clues are non-exhaustive. Each added command should contribute distinct evidence; preserve any explicitly required independent or full-suite gate.

When repository state changed, finish combined acceptance with a clean worktree and an exact committed Slice checkpoint. Push the named Slice branch to its configured remote and verify that the remote ref resolves to that checkpoint. Publication preserves the accepted Slice result; it does not imply Overall Plan acceptance or canonical integration. Update a canonical branch only when the Slice handoff explicitly assigns that integration and publication boundary.

If required work remains, including unresolved commit or publication state, identify it and leave the slice open. When the slice is complete or requires an Overall Plan decision, prepare one compact report containing:

- the result and accepted movement;
- the evidence supporting acceptance;
- local and remote branch, exact checkpoint, and clean-worktree evidence where relevant;
- remaining gaps, explicit deferrals, and unproven claims; and
- implications for later slices or the Overall Plan.

Append one compact retirement record per distinct Slice or Plan Step route: absolute route, owning task, exact retained checkpoint, Git cleanliness, known generated state or owned processes, and any concrete reason to retain local state. Mark unknown facts as unknown. Do not reclaim or remove task routes from the Slice conversation, and do not repeat accepted implementation detail in this record.
