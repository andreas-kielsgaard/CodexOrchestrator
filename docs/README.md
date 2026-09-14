# Documentation

The guides describe the product at `60c3798`, with current behavior, responsibility boundaries and the decisions that explain them. Follow the subject you need; historical task logs are not prerequisites.

## Use or change a capability

| Subject                                                            | Guide                                                 |
| ------------------------------------------------------------------ | ----------------------------------------------------- |
| Start, continue and inspect an agent conversation                  | [Agent Sessions](agent-session/README.md)             |
| Understand native homes, identities, profiles and Harness delivery | [Execution configuration](execution-configuration.md) |
| Author a Workflow and inspect its instances and deliveries         | [Workflows](workflows.md)                             |
| Register repositories, select source, build and open a review      | [Worktree Review](worktree-review.md)                 |
| Read contextual documents and diffs                                | [File Review](file-review.md)                         |
| Understand retained Epic/Sprint planning and execution             | [Epic/Sprint orchestration](orchestration/README.md)  |
| Propose and accept product decisions                               | [Product Decisions](product-decisions.md)             |

## Understand the project

- [Architecture](architecture.md) maps composition and source ownership; [ActiveDatabase](architecture/active-database.md) owns the persistence access rules.
- [Development](development.md) explains setup, command scope and optional tools.
- [Project evolution](project-evolution.md) explains changes of direction and lessons from retired approaches.
- [Validation evidence](validation-evidence.md) separates historical proof, current source observations and remaining uncertainty.

## Work outside this checkpoint

These are explicit status boundaries, not a second task queue:

| Work                       | Relationship to this checkout                                                                                                                                                                                                                                                   |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Session navigation         | `99912ce` on `codex/agent-sessions-navigation-refinement` is not integrated into `60c3798`. The [existing navigation plan](agent-session/repository-session-navigation-plan.md) is preserved under its owner's control; its status heading is not current integration evidence. |
| Remote development         | `ab220ff` on `feature/Remote-Development` is separate. Its plan, server setup and evidence are available from that revision under `docs/agent-session/remote-worktree-*`; they are not files added by this documentation change.                                                |
| Orchestration Tool Package | `8ef084b` on `codex/workflow-continuation-files` is separate; see the [Workflow boundary](workflows.md).                                                                                                                                                                        |
| Remaining tooling cleanup  | A separate task owns command changes and recorded Session simulator retirement. The disconnected UI/status-startup cleanup and legacy Task retirement are integrated through `60c3798`.                                                                                         |

The completed legacy cleanup retains its separately owned [retirement record](architecture/legacy-task-retirement-plan.md) and [real-agent verification](architecture/legacy-task-retirement-verification.md).

Inspect a branch document without changing the checkout, for example:

```powershell
git show ab220ff:docs/agent-session/remote-worktree-session-plan.md
```

The [documentation cleanup record](cleanup/documentation-rewrite-plan.md) and [file map](cleanup/documentation-rewrite-map.tsv) account for the consolidated text. Retained visual packages have their own dated indexes: [offline review](../offline-review/README.md), [regression evidence](regression-review/README.md), and [Session Event walkthrough](ux/session-event-model-walkthrough/README.md).
