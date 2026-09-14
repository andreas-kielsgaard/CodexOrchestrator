# Product Decisions

Product Decisions preserve explicit product stances and their reasoning within an Epic. A decision records what was accepted, its intent, its evidence and the context of acceptance. These records make reasoning inspectable without treating every agent statement as product policy.

This guide describes source at `60c3798`; historical research used `e2bfc6cb584a9ce7ada0d762f768c605a33b4160`. The production composition includes durable decision versions and correction flows. Some richer relationship and review examples belong to the recorded development composition and have a narrower status.

## Current, accepted and applied are different

Explicit human acceptance creates an immutable official version and makes that version current. The previous version remains in history. An accepted current decision has application state `not_applied`; acceptance does not apply it to running work, publish it, reconcile plans or certify compliance.

The acceptance request names the decision and Epic, the expected current version and an idempotency key. A stale version or conflicting replay is rejected rather than overwriting intervening work. A safe retry can return the original accepted result. These checks protect the meaning of “I accepted this correction to this version.”

Manual acceptance records its human interaction origin. Agent-assisted acceptance additionally records the proposal passage from the associated Agent Session and invocation. Provenance identifies the context and evidence of the decision; it does not claim that every contextual event caused or endorsed the decision.

## Correcting a decision

The productive Epic view presents the current version's title, statement, intent and evidence, with **Not applied** status and access to version history.

For a direct correction, **Edit** opens tentative fields. **Accept correction** creates a new version; cancelling leaves the current version intact.

**Discuss correction with agent** starts a conversation bound to the decision and its base version. The conversation can produce a retained proposal, but the agent does not replace the official decision. **Accept proposed correction** is the explicit human action that creates the next version. If the base version has become stale, the proposal remains available while the UI asks the user to reload or start a correction from the new current version.

This distinction also applies to an apparently complete final response: generating a proposal, persisting it and accepting it are separate events. The guide does not assume that the older proposed automatic extraction workflow exists merely because productive correction persistence is implemented.

## Evidence and relationships

An actionable evidence reference identifies an exact destination. Session passage navigation carries the Session and invocation identity, with the particular passage or runtime event where required. The navigation layer validates the source and target rather than finding a similar-looking title or nearby transcript text. Application Back navigation retains the return context.

Some retained evidence is historical and cannot resolve to a supported current destination. It remains labeled evidence with an unavailable reason; it does not become a fabricated link. Missing, stale, malformed or wrong-Epic targets must not be substituted with a convenient current Session.

The recorded exploration also models explicit `derives_from`, `expands` and `contradicts` relationships, as well as introduction, refinement and combination lineage. These are different relations: chronology or provenance alone does not create a hierarchy. Candidate, conflict and compliance-request examples describe recorded review material; a request is not evidence that a compliance audit ran.

The recorded view was deliberately made a separate Epic view with a simple decision list and secondary review machinery. Its collapsed intent/evidence presentation is a recorded UX choice, not a claim that every productive card uses the same disclosure state. Shared components and typed navigation can be reused without promoting every recorded capability to production authority.

## Publishing and other unfinished capabilities

**Publish** opens a typed destination for the exact decision version. The current screen says publishing is unavailable and performs no publish or apply effect. It does not invalidate work, change orchestration state, infer applicability or settle a review.

Automatic event-triggered compilation, policy reconciliation, applicability propagation, inheritance and executed compliance auditing remain exploration proposals. The earlier question of automatic acceptance has a concrete answer for the current productive flow: official versions require explicit human acceptance. The other proposals are not an adopted implementation sequence.

Decision evidence navigation is also distinct from [File Review](file-review.md). A typed Session passage reference does not establish availability of a fresh native file-review source.

## Source map

| Responsibility                                             | Implementation                                                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Current versions, acceptance and correction contracts      | [productiveProductDecisions.ts](../src/application/productDecisions/productiveProductDecisions.ts)                                                                                                                                                                                                |
| Durable versions, retained proposals and acceptance guards | [product_decisions.rs](../src-tauri/src/product_decisions.rs)                                                                                                                                                                                                                                     |
| Productive interaction and transport                       | [ProductiveProductDecisionsPanel.tsx](../src/features/productDecisions/ProductiveProductDecisionsPanel.tsx), [tauriProductDecisionClient.ts](../src/infrastructure/productDecisions/tauriProductDecisionClient.ts)                                                                                |
| Typed evidence and return navigation                       | [productNavigation.ts](../src/application/productNavigation.ts), [App.tsx](../src/app/App.tsx)                                                                                                                                                                                                    |
| Publish boundary                                           | [ProductDecisionPublishPlaceholder.tsx](../src/features/productDecisions/ProductDecisionPublishPlaceholder.tsx)                                                                                                                                                                                   |
| Recorded relationships and exploratory UI                  | [epicProductDecisions.ts](../src/application/productDecisions/epicProductDecisions.ts), [EpicProductDecisionsPanel.tsx](../src/features/productDecisions/EpicProductDecisionsPanel.tsx), [recordedEpicProductDecisionSource.ts](../src/dev/productDecisions/recordedEpicProductDecisionSource.ts) |
| Production wiring and fixture exclusion                    | [productApplicationComposition.ts](../src/bootstrap/productApplicationComposition.ts), [productDecisionProductionExclusion.test.ts](../src/bootstrap/productDecisionProductionExclusion.test.ts)                                                                                                  |

## Decision history and provenance

The exploration's enduring purpose and open proposals are preserved at `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/epic-product-decisions-exploration.md`. Its old absence-of-persistence statement is superseded by the productive version and correction implementation above.

The routed, user-reviewed correction in task `019fbe99-1f06-7433-ae70-73a805bd9a01`, raw JSONL lines 71 and 80, establishes the separate Epic view, explicit relationship hierarchy, secondary review material and exact Session citations. Lines 171 and 215 report implementation and review at `5ed3b496f5016b39e6dd0b5b938f05b5aac02820`; this was recorded-composition evidence, not a live production acceptance claim. The associated navigation review is retained in Git at `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/product-decision-evidence-navigation-review.md`.

The later routed authority assignment in task `019fd739-8a4f-7ce3-88cd-5f85ee6ff468`, raw line 20, specifies immutable official/current versions, explicit acceptance, retained agent proposals, `not_applied` status and a navigation-only Publish placeholder at checkpoint `82d9351f1437aa64dd07147d294e8c0110fea418`. Current source corroborates these semantics. The independent audit closeout and the original authority-owner task's relevant turns were not recovered; the assignment is not a passed-audit result. See [Validation evidence](validation-evidence.md) for the scoped evidence and remaining gaps.
