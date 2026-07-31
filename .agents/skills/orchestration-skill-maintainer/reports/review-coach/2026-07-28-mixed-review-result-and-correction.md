# Feedback Routing and Response Separation

## Observation

Four observed responses exposed related failures. When the user reported both a successful Plan Builder action and a failed watcher hook, the coach eventually preserved the proposal result but first spent several rounds diagnosing the watcher in the coaching session. After later receiving layout feedback, it summarized the change request and then foregrounded worktree and reviewer identifiers before giving the next review step. The next step was labeled, but the explanation of how the feedback was understood remained mixed with delegation and implementation bookkeeping. When the watcher correction returned, the coach titled the user-relevant result `Tooling disposition`; this exposed an internal work classification even though the practical message was simply that the user must return manually after each test.

Most recently, the coach routed a 27-point Harness Management correction, confirmed its worker and reviewer routes, and ended by telling the user to stop reviewing the invalid preview. It supplied neither another executable review frame nor an audited no-action handoff, so correction status became the endpoint of the coaching response.

## Theory

The skill separately described actionable-feedback routing and review continuation, but it did not tell the reader to decompose a mixed user message before acting. The coach therefore treated the tooling failure as the dominant task even though the completed product action supplied usable review evidence.

The first revision made the next step visually distinct but treated all preceding material as one supporting passage. It did not establish a presentation boundary between the user-relevant synthesis of feedback and lower-value operational routing details. The delegation procedure then pulled task mechanics into the foreground. The second revision established visual hierarchy but did not require headings to use the user's review vocabulary, leaving room for technically accurate yet context-poor labels. The conditional `Whenever review work can be done now` also left no required response ending when the coach assumed the current area was blocked, allowing it to skip both the complete queue audit and a clear account of what happens next.

## Revision

Reformulated the complete review flow to:

- process a completed review result and a correction request as independent tracks;
- bound the defect's effect on review evidence while delegating diagnosis, implementation, and acceptance;
- keep only necessary immediate containment in the coaching session;
- make correction completion a prerequisite only when it concretely invalidates evidence or blocks suitable review work; and
- present feedback understanding, work routing, and the next review step as visibly distinct regions or clear equivalents.

The feedback synthesis leads in user-facing terms and remains free of task mechanics, while operational information uses a lower heading level or disclosure block and contains only useful detail. Every user-facing response now ends with a prominent next-movement section. It contains an executable review frame when one exists; otherwise, only after the complete queue audit, it states that the user has no current action, the exact readiness condition and return route, and the frame the coach will prepare next. Headings use user-recognizable review language, and feedback routing identifies ownership without performing root-cause analysis unless needed for safe routing.

## Evaluation

This is likely to keep the main session focused on coaching while preserving truthful evidence boundaries. It does not invent user work when none exists; instead it makes the absence of action accountable to the same queue-liveness audit. Stable semantic regions and a mandatory final movement prevent implementation status from replacing coaching.

The target is the general Codex `review-coach` skill, not a product-owned Orchestrator role skill. The Orchestration Skill Maintainer supplied the behavior analysis and report location; the general skill maintenance path owns the edit.

## Validation

`quick_validate.py` passed. With one invalid area and another ready prototype, a fresh coach prepared the independent target and ended with its first review action. With a genuinely exhausted audited queue, a second fresh coach still ended under `Next step`, stated that no user review was executable, named the active return condition, and identified the corrected frame it would prepare next.
