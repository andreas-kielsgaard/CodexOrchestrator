# Ad-hoc role and operation catalogue

Status: revised.

## Observation

The ad-hoc catalogue had grown to 22 exposed skills. It represented coordination stages, recovery helpers, record maintenance, review, settlement, and merge handling as durable roles even though the intended workflow has three conversation types: Overall Plan, Slice Plan, and Plan Step. Reducing the catalogue to eight operations clarified the actions but left no single skill describing the standing responsibility and lifecycle of each conversation.

Early Overall Plan conversation evidence from task `019f48bb-85b0-7451-bf2c-5483a36a18ff` before 2026-07-16 used a provisional sequence of bounded movements. Each movement stated its objective, concerns or explorations, and an exit or evaluation condition. It kept projected and actual execution distinct and left detailed work to the bounded planning conversation.

The 2026-07-15 Sprint 5 plan from task `019f669c-6251-70b2-9d96-8c6f2e4275ae` provides the strongest observed model for the detailed Slice Plan. It used a frame, evidence baseline and revision, problem map, planning characteristics, decision packet, concern and dependency maps, sequence and parallel lanes, detailed work specifications, evidence map, risks, launch register, and concise summary.

## Theory

The catalogue treated every recurring procedure as a skill-bearing role. That obscured the stable three-conversation topology and encouraged extra conversations, handoffs, records, and review pipelines even when the owning conversation could perform the operation directly.

Renaming those roles without reducing them preserved the same premature structure. Conversely, operation skills alone repeat role-wide boundaries or leave the reader to reconstruct how the operations fit together. The earlier plans were clearer because they separated planning altitude and execution state without requiring a permanent role for every stage.

## Revision

The ad-hoc workflow now exposes three role skills:

- `run-overall-plan` owns Initiative direction and selects among Overall Plan operations.
- `run-plan-slice` owns one detailed slice through its Plan Steps and combined return.
- `run-plan-step` owns one fixed assigned outcome through execution and callback.

Each role skill names the operation skills available to its reader and holds persistent ownership, state, lifecycle, and parent-child boundaries. The eight operation skills remain:

- Overall Plan: `maintain-overall-plan`, `start-plan-slice`, `evaluate-plan-slice`.
- Slice Plan: `maintain-slice-plan`, `start-plan-steps`, `evaluate-plan-step`, `complete-plan-slice`.
- Plan Step: `execute-plan-step`.

`maintain-overall-plan` follows the early provisional movement map. `maintain-slice-plan` treats the old Sprint 5 considerations as required coverage while allowing flexible grouping instead of rigid headings. Review, exploration, integration, verification, documentation, or similar work can be expressed as Plan Steps when needed.

Every Overall Plan maintenance pass presents the complete current plan. Before first delegation, every Plan Slice presents all detailed planning considerations inherited from the historical Sprint plan; it may combine sections for clarity but does not replace them with an internal plan update or child prompt.

Standing role information was removed from the operation skills where the role skill now supplies it. Start operations create top-level conversations with the appropriate `run-*` skill; the receiving role skill then selects its current operation.

The creating role also selects the child profile. Overall Plan defaults Plan Slices to Sol with high reasoning. Slice Plan selects Luna or Terra from Plan Step ambiguity and independently selects low through high reasoning from context scope, definition, and potential blast radius. Start operations record the profile the harness actually applies.

The prior workflow entry points were removed. Their detailed instructions were not moved into reference files. Historical reports and non-discoverable evidence remain available, while `orchestration-skill-maintainer` remains a maintenance utility rather than part of the workflow catalogue.

Product role skills remain separately stored under repository-owned `product/skills` and are not referenced by these ad-hoc operations.

## Evaluation

The revision makes conversation ownership stable while keeping actions modular and progressively disclosed. It retains the observed strengths of the earlier plans while allowing simpler slices to omit irrelevant sections. The three role skills add little policy: they primarily identify responsibility, state, operation selection, and return behavior. This reduces reconstruction and duplication without reviving the former auxiliary-role catalogue.
