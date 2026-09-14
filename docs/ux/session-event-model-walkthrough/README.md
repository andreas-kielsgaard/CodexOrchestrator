# Session Event walkthrough assets

These are historical presentation and recorded-client assets from the Session Event redesign, including the September 7, 2026 merged capture set. They are supporting evidence, not the current product specification. Start with [Workflows](../../workflows.md) and [execution configuration](../../execution-configuration.md) for current behavior.

The former index called the v7 deck current and described a graph that did not match all its captures. The merged `10-saved-instance.png` shows list-style instances; the later [repair image](../../regression-review/repairs/evidence/05-saved-instance.png) shows the restored graph. The two images were inspected during baseline research. Neither deck nor the complete image collection was freshly visually reviewed during this rewrite.

## Presentations and their sources

Both decks remain at their existing paths. Their names and generated format alone do not establish whether their explanations are redundant. The `.mjs` files are generation/seeding sources, not ordinary application startup commands; they may encode the original local environment.

| File                                                                                            | Role                           |
| ----------------------------------------------------------------------------------------------- | ------------------------------ |
| [seed-demo-session.mjs](build/seed-demo-session.mjs)                                            | Presentation/fixture source    |
| [walkthrough-v7.mjs](build/walkthrough-v7.mjs)                                                  | Presentation/fixture source    |
| [walkthrough.mjs](build/walkthrough.mjs)                                                        | Presentation/fixture source    |
| [Session Event UI Demo Walkthrough v7.pptx](<output/Session Event UI Demo Walkthrough v7.pptx>) | Historical presentation output |
| [Session Event Model UI Walkthrough.pptx](<Session Event Model UI Walkthrough.pptx>)            | Historical presentation output |

## Early configuration studies

The first capture family records Capability Profiles, Workflow Node Profiles, prompt/target choices, Session settings and delivery inspection during the redesign. Captions below identify the supplied files; they do not assert current UI fidelity or a newly verified execution result. Original file-by-file producing conversations were not all recovered.

| File                                                                                                           |
| -------------------------------------------------------------------------------------------------------------- |
| [01-capability-profile-overview.jpg](screenshots/01-capability-profile-overview.jpg)                           |
| [02-capability-allowed-selections.jpg](screenshots/02-capability-allowed-selections.jpg)                       |
| [03-workflow-node-profile.jpg](screenshots/03-workflow-node-profile.jpg)                                       |
| [04-node-restrictions-and-defaults.jpg](screenshots/04-node-restrictions-and-defaults.jpg)                     |
| [05-connection-trigger-and-prompt.jpg](screenshots/05-connection-trigger-and-prompt.jpg)                       |
| [06-connection-prompt-and-target.jpg](screenshots/06-connection-prompt-and-target.jpg)                         |
| [07-compile-and-run.jpg](screenshots/07-compile-and-run.jpg)                                                   |
| [08-session-profile-and-message-controls.jpg](screenshots/08-session-profile-and-message-controls.jpg)         |
| [09-pinned-defaults-and-node-capabilities.jpg](screenshots/09-pinned-defaults-and-node-capabilities.jpg)       |
| [10-attached-runtime-and-delivery-provenance.jpg](screenshots/10-attached-runtime-and-delivery-provenance.jpg) |
| [11-copy-node-configuration.png](screenshots/11-copy-node-configuration.png)                                   |
| [12-capability-profile-save-and-delete.png](screenshots/12-capability-profile-save-and-delete.png)             |

## Merged September 7 capture set

These images use actual React screens with recorded clients. They include the old overlapping Harness screen and layout problems that later work addressed. They do not prove native persistence, provider execution, keyboard accessibility or user acceptance. Filenames such as `03-workflow-canvas.png` are historical labels rather than guarantees about the depicted implementation.

| File                                                                                                         |
| ------------------------------------------------------------------------------------------------------------ |
| [01-capability-profile.png](screenshots/merged-2026-09-07/01-capability-profile.png)                         |
| [02-allowed-capabilities.png](screenshots/merged-2026-09-07/02-allowed-capabilities.png)                     |
| [03-workflow-canvas.png](screenshots/merged-2026-09-07/03-workflow-canvas.png)                               |
| [04-node-profile.png](screenshots/merged-2026-09-07/04-node-profile.png)                                     |
| [05-copy-node-profile.png](screenshots/merged-2026-09-07/05-copy-node-profile.png)                           |
| [06-node-limits-defaults.png](screenshots/merged-2026-09-07/06-node-limits-defaults.png)                     |
| [07-connection-trigger.png](screenshots/merged-2026-09-07/07-connection-trigger.png)                         |
| [07-session-settings.png](screenshots/merged-2026-09-07/07-session-settings.png)                             |
| [08-connection-prompt-target.png](screenshots/merged-2026-09-07/08-connection-prompt-target.png)             |
| [09-create-instance.png](screenshots/merged-2026-09-07/09-create-instance.png)                               |
| [10-saved-instance.png](screenshots/merged-2026-09-07/10-saved-instance.png)                                 |
| [11-instance-session.png](screenshots/merged-2026-09-07/11-instance-session.png)                             |
| [12-agent-identity.png](screenshots/merged-2026-09-07/12-agent-identity.png)                                 |
| [13-session-runtime-and-deliveries.png](screenshots/merged-2026-09-07/13-session-runtime-and-deliveries.png) |
| [14-delivery-details.png](screenshots/merged-2026-09-07/14-delivery-details.png)                             |
| [15-harness-identity.png](screenshots/merged-2026-09-07/15-harness-identity.png)                             |
| [16-legacy-harness-management.png](screenshots/merged-2026-09-07/16-legacy-harness-management.png)           |
| [17-mcp-trigger.png](screenshots/merged-2026-09-07/17-mcp-trigger.png)                                       |
| [18-recorded-source-target.png](screenshots/merged-2026-09-07/18-recorded-source-target.png)                 |
| [19-prompt-sources.png](screenshots/merged-2026-09-07/19-prompt-sources.png)                                 |

## Provenance and related evidence

The original index and checkpoint context are available at `e2bfc6c:docs/ux/session-event-model-walkthrough/README.md` and `e2bfc6c:docs/regression-review/README.md`, which identifies `4bded63` as the reviewed product checkpoint. Keep artifact creation, the reviewed source and later repairs separate.

See [regression/repair assets](../../regression-review/README.md) and [validation evidence](../../validation-evidence.md#session-event-regressions-and-repair). No deck, screenshot or source generator was changed or removed by the documentation rewrite.
