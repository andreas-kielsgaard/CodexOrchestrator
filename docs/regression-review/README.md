# Session Event UI regression review

Reviewed on 7 September 2026. This is a review, not a repair pass.

The replacement UI is mounted, but several basic flows are missing or broken. The missing flow canvas and Workflow instance screens are only part of the problem. Starting a normal Session, sending its next message, handing work to the next node, and keeping the same run selected need attention before another user demo.

## Checkpoint and scope

- Existing walkthrough work committed as `4bded63` on `codex/session-event-model-overhaul`.
- Review branch: `codex/session-event-regression-review`, created from that commit.
- Review worktree: `C:/Users/user/.codex/worktrees/session-event-regression-review/Codex Orchestrator`.
- Comparison point: `9fc822f`, the combined Workflow/Harness work before this overhaul.
- No product source was changed. No live provider was called. The original demo and its data were left alone.
- PowerPoint's open-file lock was not committed or removed. The substantive deck, screenshots and source were committed; the deck remains an unfinished review artifact, not UI acceptance.
- After the checkpoint, two `event-result-fixture` files and `13-event-result-and-delivery-details.png` appeared in the original walkthrough folder. They were left there uncommitted and are not part of the reviewed checkpoint.

Role removal, embedded Node Profiles, pinned Session Profiles, per-message model/reasoning choices and deferred provider setup are accepted changes. They are not regressions. This review also does not require a new run graph or a redesign of the Epic workflow.

## Main findings

Browser evidence uses the actual React screens and CSS with fake clients. Backend findings trace the production code but were not exercised through a live native runtime. The distinction matters: passing editor tests does not prove a Workflow can run from end to end.

| Area                            | What is wrong                                                                                                                                                                                         | Evidence                                                                                                                                                                                                             |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Normal Sessions                 | The first message can create a Session with no pinned profile. Once selected, the UI blocks its second message because that profile is missing.                                                       | Browser reproduction plus [B1](backend-findings.md#b1--ordinary-sessions-cannot-receive-their-second-message-p1). [Screenshot](evidence/ordinary-session-second-message-blocked.png).                                |
| Workflow handoffs               | Connections can be saved and compiled, but their triggers are not wired to the new execution service. Completing A does not start B through that service.                                             | Production source trace, [B2](backend-findings.md#b2--new-workflow-connections-never-run-from-their-triggers-p1).                                                                                                    |
| Workflow instances and worktree | The instance list, creation form and worktree choice are gone from the mounted route. New event-created Sessions get no working directory. An editable ID is not a replacement for a stored instance. | [U2](ui-layout-findings.md#u2-workflow-instance-creation-and-selection-are-missing), [B3](backend-findings.md#b3--new-workflow-sessions-lose-their-chosen-worktree-p1).                                              |
| Run selection                   | Leaving the run panel and returning makes a new instance ID, while the old result remains visible.                                                                                                    | Browser reproduction, [F5](frontend-findings.md#f5-returning-to-the-run-page-changes-the-instance-but-keeps-old-results).                                                                                            |
| Saved and unsaved work          | Activation erases newer local edits. Switching recipes also loses edits. A late save response can overwrite text typed while saving.                                                                  | Browser reproductions for activation/switching; component reproduction for late save. [F1, F2](frontend-findings.md), [F6](frontend-findings.md#f6-changing-screens-or-recipes-drops-unsaved-drafts-without-notice). |
| Pinned Sessions                 | Workflow sends reload current shared profiles before finding the target Session. A later profile edit or deletion can block a message to an already pinned Session.                                   | Production source trace, [B4](backend-findings.md#b4--editing-shared-definitions-can-break-an-already-pinned-session-p1).                                                                                            |
| Node choices                    | The editor lets a node select capabilities its Capability Profile forbids. Removing a default from the allowed set leaves a hidden invalid default.                                                   | Browser reproductions, [F3, F4](frontend-findings.md).                                                                                                                                                               |
| Connection choices              | Activation accepts a prompt source that its selected trigger cannot supply. Delivery then fails.                                                                                                      | Compiler/materializer source trace, [B5](backend-findings.md#b5--activation-accepts-triggersource-pairs-that-cannot-deliver-p2).                                                                                     |
| Flow layout and screen size     | The canvas was replaced with a list. Below 900 px, even the recipe picker and New Workflow button disappear.                                                                                          | Source comparison and browser reproduction, [U1, U3](ui-layout-findings.md).                                                                                                                                         |
| Shared controls                 | Collapsed sections remain visible. Checkboxes take most of the row and squeeze their labels.                                                                                                          | Computed browser layout and screenshots, [U4, U5](ui-layout-findings.md).                                                                                                                                            |
| Session details                 | The old identity view/edit entry point is gone. Delivery history does not refresh while the same Session stays open.                                                                                  | Source trace, [F7, F8](frontend-findings.md).                                                                                                                                                                        |

## What the existing checks prove

- `npm run build`: passed.
- Full frontend suite: **980 tests passed across 165 files**.
- Saved browser probe: **nine issues reproduced**, with no page JavaScript errors.
- Further component checks and backend call-path review are documented in the linked reports.

Many passing tests still exercise the old Workflow screen or isolated new controls. They do not cover the new mounted screen's draft ownership, run selection or the runtime completion path. See [verification and replay](verification.md) for exact commands, fixtures and limits.

## Suggested next repair order

1. Give normal Session creation a valid path into the new profile model; prove first and second sends together with a fake runtime.
2. Restore a stored Workflow instance with its target worktree. Keep the instance selected across navigation and keep its results tied to that instance.
3. Connect one real completion path from node A to node B. Prove it through the normal notifier, not by calling dispatch directly in a test.
4. Keep existing pinned Sessions usable after shared definitions change. Reject impossible node and connection choices before activation.
5. Restore the flow canvas around the new node/connection editors. Protect unfinished edits, retain identity inspection, and fix the shared controls and narrow layout.

These are proposed repair groups, not a request to preserve the old Role model or to settle the future run UI. The useful old canvas, instance target picker and navigation behavior can be adapted without restoring deprecated concepts.

## Read by task

- Session creation, runtime handoffs, pinning: [backend findings](backend-findings.md).
- Editing state, node choices, run selection, Session details: [frontend findings](frontend-findings.md).
- Canvas, instance entry points, responsive layout and CSS: [UI/layout findings](ui-layout-findings.md).
- Replaying checks and judging their limits: [verification](verification.md).
