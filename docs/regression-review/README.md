# Regression and repair evidence package

This package retains the September 7, 2026 recorded-browser baseline and subsequent repair evidence. The original baseline at `4bded63` reproduced defects; repair work on `e77a725` produced `c1b89b9`, followed by Harness UX consolidation. Current behavior is explained in [Workflows](../workflows.md), [Sessions](../agent-session/README.md) and [configuration](../execution-configuration.md).

The [consolidated evidence record](../validation-evidence.md#session-event-regressions-and-repair) owns the findings, corrections and proof limits. These images and JSON files keep their original meaning: a baseline `reproduced: true` result is evidence of the old defect, not a passing acceptance test or a claim that the defect remains today.

## Replaying the retained probes

From a checkout of the intended source revision with npm dependencies installed:

```powershell
node docs/regression-review/probes/run-browser.mjs
node docs/regression-review/repairs/run-browser.mjs
```

The scripts use installed Microsoft Edge and Playwright supplied by the bundled Codex dependencies; `REVIEW_NODE_MODULES` can select a different directory containing Playwright. The baseline runner starts Vite at `127.0.0.1:2381`, opens its own headless browser, records observations, then stops both. Inspect the selected runner's paths before replay: outputs are local evidence files, and current product imports may no longer reproduce the original snapshot.

The real React screens use fake clients. Browser-instance storage is fixture state, not native durability. Joined Rust repair tests supplied separate SQLite reopen and normal notification-to-dispatch evidence with recording inference and local HTTP. None of these browser files establishes paid-provider, release-package, native-keyboard or user acceptance.

## Original baseline

The JSON records before/after values that a still image cannot show, including run selection and the blocked second message. Source entries and the runner are retained alongside the evidence.

| File                                                                                                | Role                          |
| --------------------------------------------------------------------------------------------------- | ----------------------------- |
| [browser-results.json](evidence/browser-results.json)                                               | Recorded browser observation  |
| [checkbox-label-wrap.png](evidence/checkbox-label-wrap.png)                                         | Original defect screenshot    |
| [collapsed-runtime-still-visible.png](evidence/collapsed-runtime-still-visible.png)                 | Original defect screenshot    |
| [node-forbidden-model.png](evidence/node-forbidden-model.png)                                       | Original defect screenshot    |
| [ordinary-session-second-message-blocked.png](evidence/ordinary-session-second-message-blocked.png) | Original defect screenshot    |
| [run-id-changed.png](evidence/run-id-changed.png)                                                   | Original defect screenshot    |
| [workflow-narrow.png](evidence/workflow-narrow.png)                                                 | Original defect screenshot    |
| [browser.html](probes/browser.html)                                                                 | Recorded fixture/probe source |
| [browser.tsx](probes/browser.tsx)                                                                   | Recorded fixture/probe source |
| [run-browser.mjs](probes/run-browser.mjs)                                                           | Recorded fixture/probe source |

## Repair evidence

The repair runner exercised four viewport widths, node/connection editing, graph layout, instance creation, opening a Session, Back and saved-instance reopening. `05-saved-instance.png` shows the restored graph; it is later evidence than the walkthrough's earlier list-style instance image. That pair was inspected during documentation research; this rewrite does not claim a fresh review of every image.

| File                                                                | Role                           |
| ------------------------------------------------------------------- | ------------------------------ |
| [browser.html](repairs/browser.html)                                | Recorded repair fixture/runner |
| [browser.tsx](repairs/browser.tsx)                                  | Recorded repair fixture/runner |
| [01-flow.png](repairs/evidence/01-flow.png)                         | Repair screenshot              |
| [02-connection.png](repairs/evidence/02-connection.png)             | Repair screenshot              |
| [03-node-1280.png](repairs/evidence/03-node-1280.png)               | Repair screenshot              |
| [03-node-640.png](repairs/evidence/03-node-640.png)                 | Repair screenshot              |
| [03-node-850.png](repairs/evidence/03-node-850.png)                 | Repair screenshot              |
| [03-node-958.png](repairs/evidence/03-node-958.png)                 | Repair screenshot              |
| [04-create-1280.png](repairs/evidence/04-create-1280.png)           | Repair screenshot              |
| [04-create-640.png](repairs/evidence/04-create-640.png)             | Repair screenshot              |
| [04-create-850.png](repairs/evidence/04-create-850.png)             | Repair screenshot              |
| [04-create-958.png](repairs/evidence/04-create-958.png)             | Repair screenshot              |
| [05-saved-instance.png](repairs/evidence/05-saved-instance.png)     | Repair screenshot              |
| [06-instance-session.png](repairs/evidence/06-instance-session.png) | Repair screenshot              |
| [results.json](repairs/evidence/results.json)                       | Recorded repair result         |
| [run-browser.mjs](repairs/run-browser.mjs)                          | Recorded repair fixture/runner |

## Provenance

The complete former narratives are available at `e2bfc6c:docs/regression-review/README.md`, `e2bfc6c:docs/regression-review/verification.md` and the corresponding `repairs/verification.md`. The repair full Rust run contained one timeout that passed in isolation; it was not silently changed to a full-suite pass. Later integration and native evidence have their own checkpoints in the [evidence record](../validation-evidence.md).

No retained image, fixture, script or JSON file was regenerated, relabeled or removed by the documentation rewrite.
