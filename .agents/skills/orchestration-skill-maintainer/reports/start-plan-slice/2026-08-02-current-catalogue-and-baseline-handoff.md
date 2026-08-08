# Current catalogue and baseline handoff

## Observation and theory

An earlier Slice missed the full plan because its handoff loaded `run-plan-slice` from an old change target. The first correction replaced that explicit path with host skill-name resolution.

Three later Slice tasks showed that host name resolution was still insufficient:

- `019fc25c-7470-7013-a9d9-cee1edd8fa91`
- `019fc25c-b6cc-77f1-83e4-6bfe6f18e1ef`
- `019fc25d-51c0-7830-83a1-945bb82203fc`

Each task received `run-plan-slice` from its isolated baseline worktree. That file had SHA-256 `23D95A36B7C35B59D737EADCB3FBE4A484195A8F26DC9340A3F05500AC47EC95`; the current ad-hoc catalogue file was `72F444AC61CC04A07CDA149875BA4BB5AB24D1999E328EF013F3BB534FBAD22A`. The injected copy lacked the complete-plan presentation and launch-boundary wording. None of the three assistant histories presented the current full-plan coverage before Plan Step activity.

The failure came from treating host resolution as evidence of catalogue identity. In a task whose change route is an older worktree, the host can legitimately discover that worktree's repository-owned skill copy.

## Revision

`start-plan-slice` now uses the catalogue that supplied the operation as its instruction source. It resolves the sibling `run-plan-slice/SKILL.md`, verifies that it is readable, and begins the child prompt with an `AD-HOC TASK ROUTING` header containing the absolute role skill, catalogue, task route, and callback task. The child is told to retain those values and re-read the exact role skill after compaction. The repository or worktree remains a separate change target.

The existing self-contained baseline handoff remains unchanged: accepted prior work is expressed as present facts, authoritative artifacts supply evidence, and conversation ids serve only as routes.

## Evaluation

The revision addresses prompt-carried delivery rather than duplicating the planning contract in the parent skill. Current Codex has no typed instruction-source field, so this is not host-enforced immutability. It nevertheless makes source identity explicit, transitive, and recoverable after compaction while exposing a visible routing gate if the catalogue is unavailable.

In a forward test with the older `20a2` worktree as the product route, the operation selected the saved catalogue's absolute `run-plan-slice/SKILL.md` path as the instruction source and kept the worktree separate. It produced a self-contained Slice handoff without reading or selecting the worktree's stale skill copy.

It does not alter the three active Slice tasks retroactively.
