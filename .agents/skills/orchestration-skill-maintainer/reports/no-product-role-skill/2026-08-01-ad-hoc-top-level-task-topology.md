# Ad-hoc Epic task topology

## Observation

Historical Sprint 5 task `019f669c-6251-70b2-9d96-8c6f2e4275ae` kept one Sprint-wide plan, launched and accepted successive Work Units, revised the launch register as evidence changed, and returned the Sprint boundary to the Epic owner.

The newer skills instead made one planner own one next batch and inserted a separate delegation owner between it and implementation. Sprint 20 task `019fbff3-3522-7751-91e9-273c1e3a0249` consequently behaved like a one-slice planner despite its Sprint title.

The first correction restored whole-Sprint ownership but included maintenance explanations and full workflow taxonomy inside role skills. The root skill also described plan building and orchestration instantiation even though its session can exist only after the orchestration home is established.

## Theory

The earlier wording defined lifecycle by "next batch," so role naming could not produce whole-Sprint behavior. The first correction then addressed the catalogue from the maintainer's perspective instead of limiting each skill to facts its reader needs to act.

## Evidence Recovered

The original `sprint-runner` at commit `5d03fa317a74a5106ae31acee8b38ea786f880c8` supplied the strongest Sprint-planning structure:

- orient from an evidence baseline;
- define the Sprint frame;
- build a concern-first problem map;
- assess definition, complexity, risk, reversibility, verification, and work mode;
- reduce consequential ambiguity before launch;
- define coherent Work Units;
- map concerns, dependencies, parallel lanes, gates, and convergence;
- maintain a launch register immediately before the operational summary; and
- preserve planning revisions and actual execution history as outcomes return.

Its obsolete separate Planner-responsibility tier and catalogue commentary were not carried forward.

## Revision

`orchestration-next-work-planner` now speaks directly to the Sprint Runner. It uses the recovered planning structure, launches separately addressable Work Units, reviews their returns, revises future work without erasing history, audits the combined Sprint, and returns one handback.

`orchestration-root` now assumes an established orchestration home. It contains only Epic direction, its direct Sprint Runner lifecycle, handback evaluation, records needed for Epic decisions, interruption handling, and unattended continuation. Pre-instantiation guidance and the full downstream hierarchy were removed.

Compatibility and catalogue rationale remain in this report. Skill descriptions state only the assignments that should trigger each role.

## Evaluation

The Sprint Runner receives enough structure to reproduce the strong historical plans without being asked to reason about the surrounding catalogue. The Epic Runner knows only what it needs to create and evaluate its direct child. This reduces role drift while preserving top-level task visibility, proactive callbacks, independent Sprint audit, and unattended recovery.

Sprint 20 was already at its closing audit when this correction was made, so the revised lifecycle applies at the next genuine Sprint boundary.
