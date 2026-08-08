# Plan definition and decomposition

## Observation and theory

The observed Slice Plan delegated a carefully written but very broad implementation prompt after publishing only a short two-step summary. The skill's flexible presentation model allowed execution authorization to be interpreted as permission to omit detailed historical planning considerations and move planning detail into the child prompt.

The first revision required a complete presentation before launch but did not make an explicit later request for the plan equivalent to a full presentation. A later correction required the full plan on every maintenance pass, which unnecessarily repeated unaffected detail.

In task `019fc109-d697-7a70-8a6f-41585d58d9d9`, the original plan presentation was a short implementation/review summary. After the user requested the plan, the reader produced a seven-section retrospective covering the frame, concerns, actual tasks, revisions, evidence gates, and launch register. It did not reconstruct the planning-characteristic assessment, ambiguity and decision packet, concern-to-step and dependency reasoning, sequence and parallelization analysis, detailed Plan Step specifications, validation map, risks, or operational summary required by the complete plan contract.

The historical Sprint 5 revision in task `019f669c-6251-70b2-9d96-8c6f2e4275ae` exposed the full reasoning behind its plan: grounded evidence, concern and complexity maps, decision packet, ownership and sequencing rationale, detailed work specifications, validation claims, risks, gates, and current register. The weaker output was therefore not explained merely by execution having started.

The current flow also repeatedly selected Terra/high: Slice 1 used it for implementation and independent review, while Slice 2 used it for two read-only explorations and its implementation. The profile policy named ambiguity and context scope but did not require those assessments to be shown or compared with adjacent settings. Selecting the maximum profile was therefore an easy confidence hedge. The historical Sprint 5 plan instead exposed concern-level definition, complexity, risk, verification, and work mode, then used Luna/high for a well-defined TypeScript integration while reserving Terra/high for broader Rust/MCP work.

Both Slices also treated independent review as a standard post-implementation gate even though the Slice conversation itself inspected the returned work, discovered defects, routed corrections, and owned combined acceptance. The plan skill did not require a review step to resolve evidence beyond that ordinary evaluation.

## Revision

`maintain-slice-plan` keeps the complete current plan through every revision and presents it in full when first established, when the user requests it, and after context compaction. `run-plan-slice` carries the same presentation boundary. Other maintenance passes present the revision and its effects on the affected concerns, steps, dependencies, gates, evidence, risks, and launch register.

The reader retains the original planning rationale and updates it with actual execution evidence and superseded projections instead of replacing it with current statuses.

Every presentation covers the detailed planning considerations, gives every concern a disposition, and separates independently evaluable outcomes even when dependencies make them serial. Sections may be combined for clarity, but their considerations may not disappear. One implementation step remains valid when it is genuinely one evaluation boundary.

Each concern now receives an explicit characteristic assessment. Every projected Plan Step visibly states ambiguity, context scope and blast radius, model, reasoning, and why that profile is more appropriate than adjacent settings. Profiles derive from the step's owned work rather than overall Slice difficulty.

Ordinary returned-work evaluation remains with the Slice conversation. A projected review or verification step must name independently required evidence or a distinct unresolved concern beyond routine evaluation; it is not a standing gate after implementation.

## Evaluation

The change makes initial decomposition, explicit plan-review requests, and post-compaction recovery inspectable without forcing full-plan repetition during ordinary maintenance or unrelated evaluation and status responses.

The role skill makes post-compaction recovery explicit: reconstruct and present every full-plan coverage area before another operation. A status or launch-register summary is not the recovered plan.

The initial read-only test covered every detailed consideration, combined compatible sections, separated native contract, TypeScript projection, and independent convergence into three serial Plan Steps, and launched nothing.

A fresh unchanged-plan test supplied an active implementation step, a planned serial verification step, and no material new evidence, then asked to see the plan. The reader reproduced the complete planning breakdown: frame and baseline, concerns, characteristic assessment, ambiguity treatment, dependency and sequence reasoning, detailed step specifications, validation placement, risks, launch register, execution state, and operational summary. It distinguished unavailable profile evidence rather than inventing it and launched or monitored nothing.

A focused ordinary-maintenance test reopened one implementation step for a bounded correction, updated the downstream gate, and reported only the changed consequences rather than repeating the full plan.

A fresh mixed-profile test planned five steps from four supplied concerns without launching them. It selected Luna/low for a two-file rename, Luna/medium for a frozen DTO propagated across six TypeScript surfaces, Terra/high for unresolved cross-layer authorization analysis, Luna/high for implementation after the policy decision removes ambiguity, and Terra/high for an uncertain Rust race diagnosis. Each selection exposed the relevant ambiguity, context breadth or blast radius, and a comparison with adjacent settings rather than inheriting one profile from the Slice.
