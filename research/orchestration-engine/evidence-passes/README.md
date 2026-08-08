# Behavior-led evidence passes

These passes begin with concrete behavior and follow it across presentation, application, transport, persistence, configuration and external effects. They are evidence records, not a settled product taxonomy.

Use the [current-state overview](../current-state/README.md) first. Return here when its concise explanation needs exact behavior, exceptions, branch scope or source references. Snapshot labels identify which passes remain historical evidence rather than silently current descriptions.

## Stable snapshots

| Pass | Snapshot | Starting question |
| --- | --- | --- |
| [Agent runtime comparison](agent-runtime-comparative-pass.md) | `b28137b` | How do ordinary, Plan Builder and Handler/Implementer Sessions differ if they share one runtime? |
| [Native and managed runtime authority](native-and-managed-runtime-pass.md) | compares `b28137b` and `9240364` | Where do Native Profile, Harness, MCP and common runtime authority actually meet? |
| [Persistence, reconciliation and freshness](reconciliation-and-freshness-pass.md) | `b28137b` | What happens after a runtime fact is persisted, and which projections become fresh? |
| [Visible controls and unavailable mutations](visible-control-boundaries-pass.md) | `b28137b` | What does the product render when its read side exists but its effect adapter does not? |
| [Legacy Task quarantine](legacy-task-quarantine-pass.md) | `b28137b` | What remains of the pre-Agent-Session Task product, and what can still execute? |
| [Packaged Agent Session verification](packaged-verification-surface-pass.md) | `b28137b` | What does it mean for a real-component, recorded-data Harness to be included in a production build? |
| [File Review and Human/Worktree Review](review-surfaces-pass.md) | `b28137b` | How do release, debug, recorded and isolated-review forms of one visual capability differ? |

## Evidence-selected deep traversals

These passes were selected after four independent signal sweeps exposed the same high-information seams. They follow assembled operations rather than predefined component groups.

| Pass | Snapshot | Starting question |
| --- | --- | --- |
| [Application source checkout as Git authority](source-checkout-git-authority-pass.md) | `9240364` | How does the checkout used to compile the backend become planning context, attempt ancestry and accepted-integration target? |
| [Effective Implementer reporting launch](effective-implementer-reporting-launch-pass.md) | `9240364` | What exact executable contract emerges from Harness, MCP, profile, environment, process and receipt layers? |
| [Startup, shutdown and user observability](startup-shutdown-observability-pass.md) | `9240364` | What external work can occur around application lifecycle, and which parts become visible or controllable? |

Several passes intentionally retain the original `b28137b` evidence boundary. Current research source is `9240364`; later changes are called out rather than silently rewriting the earlier observation.

## Moving working-state passes

Uncommitted passes are snapshots of source worktrees, not integrated capability claims.

| Pass | Source state | Main interpretation |
| --- | --- | --- |
| [Work Slice Planner prelaunch transition](uncommitted-operational-transition-pass.md) | dirty `b964509` worktree | early precursor absorbed, corrected and greatly extended by committed descendants |
| [Runtime toolchain experiment](uncommitted-runtime-toolchain-pass.md) | dirty detached `b86a8ac` worktree | developer/test-tier alternative, partly superseded and partly unique |
| [Presentation and Harness evidence](uncommitted-presentation-and-harness-pass.md) | two dirty `55cdd40` worktrees plus dirty `main` | alternative deterministic presentations and a mixed, internally mismatched Harness relocation |

## Provisional synthesis

[Cross-cutting system findings](../current-state/cross-cutting-system-findings.md) records only structures repeated across multiple passes and incorporated into the present explanatory layer.
