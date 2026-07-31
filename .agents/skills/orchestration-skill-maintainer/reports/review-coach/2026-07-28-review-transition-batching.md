# Review Environment Ownership and Self-Contained Handoffs

## Observation

After the user completed the Harness Management review, the coach selected the File/diff viewer as the next independent area. It first made server teardown a standalone user turn. It then gave the user commands to recreate the viewer checkout, install dependencies, start the server, open the view, and report readiness, even though none of those operations exercised the reviewed material.

After the coach adopted environment ownership, it launched the viewer in a second browser tab but left the user's current tab on Plan Builder. Its next instruction began `In the open Codex browser` and named controls without identifying the viewer tab or URL. When the user read only that section, there was no way to know where the actions belonged.

Later, the coach routed Harness Management corrections and selected the technically accepted worktree launcher as the next independent review area. It had authority and tools to prepare that launcher, but asked the user to reply `open the worktree launcher` before it would begin setup. The reply supplied no decision, evidence, or new authority; it only triggered coach-owned work.

In a subsequent transition, the coach sent the user back to the existing Harness Management tab at port `43184` and instructed them to reload and review it. It did not verify the backing preview server first. The user reached a dead port and had to report `this site can't be reached`; only then did the coach restart the accepted checkpoint and verify the page.

## Theory

The skill initially framed teardown and setup as user instructions because it did not give the coaching role authority to operate a safe review environment. Later revisions established that ownership and carried it across review-area transitions, but `confirm that the target is ready` did not distinguish current readiness evidence from remembered readiness. The coach could therefore treat an existing tab and a previously running server as a prepared surface without rechecking the live route. The standalone-section wording also needed to identify the prepared surface among multiple tabs.

## Revision

Reformulated the coaching flow so the coach owns reversible review-environment setup and cleanup that leaves reviewed material and consequential product state unchanged. With available tools it performs safe process management, isolated checkout preparation, dependency installation, fixture selection, navigation, and similar surrounding work while preserving unrelated state.

The coach now completes mechanical preparation and verifies readiness from current evidence immediately before addressing the user. An existing tab, window, process, or similar surface carries locator history, not current readiness; the coach verifies that its backing environment is live and presents the intended target state. It carries an unambiguous transition through setup in the same turn and reserves a separate user turn for an operation the coach cannot perform safely or for action, judgment, evidence, or authority that must come from the user.

The prepared surface is brought forward when possible. Every next-step section is written as a standalone instruction card: it identifies the exact reachable surface and starting condition, then states the review action, scope, relevant stop boundary, and return request without depending on earlier narration.

## Evaluation

This should remove mechanical work, readiness acknowledgements, magic-phrase setup triggers, and dead-surface recovery from the human-in-the-loop path while preserving the user's judgment and authorization boundaries. Rechecking immediately before handoff is small and bounded; it does not authorize continuous monitoring. The coach still asks before actions that genuinely need user authority or evidence.

The target is the general Codex `review-coach` skill. No product code or product-owned Orchestrator role skill is changed.

## Validation

`quick_validate.py` passed. A fresh coach given an old browser tab and no current server evidence treated prior readiness as history, represented checking the route, restarting the accepted isolated preview if needed, confirming the rendered prototype identity, and bringing the tab forward before giving the user's first review action.
