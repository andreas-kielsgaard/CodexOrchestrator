# Plan Slice role skill

## Observation and theory

Task `019fc109-d697-7a70-8a6f-41585d58d9d9` confirmed its implementation child and then made three `wait_threads` calls with ten yielded wait continuations. It published two no-change status updates while claiming not to ingest routine progress.

After the user told it that `run-plan-slice` had been updated, the task re-read the skill and deliberately began new waits. The recorded sequence after the reload includes at least two new `wait_threads` calls and five continuation waits, plus another unchanged-status message. It described an incomplete slice, a held gate, and its launch register as reasons to remain active.

The same task announced only one broad implementation step followed by review. Its child prompt was detailed, but seven distinct concerns spanning durable transition state, Harness authority, reconciliation, product projection, and validation were collapsed into the implementation step. Shared surfaces were treated as a reason not to decompose, although serial Plan Steps can still have independent outcomes and evaluation gates.

That first Slice and the succeeding Slice both projected a standing independent review after their broad implementation step. The second Slice's owner repeatedly inspected the implementation itself, found and routed several correction rounds, and then opened another read-only reviewer as a confidence gate. The role already owns step evaluation and combined acceptance, but its permission to represent additional review as a Plan Step did not distinguish independent evidence from ordinary acceptance work.

The prior wording said to avoid polling and to end the turn, but it left the reader to reconcile that with responsibility for an incomplete slice. The reader treated pending state as current work and interpreted "continue from the existing launch register" as authority to keep the turn alive. It also did not state that the Slice Plan must be presented in the conversation, so an internal plan update or detailed child prompt could replace the reviewable plan.

## Revision

`run-plan-slice` now requires a planning revision to be presented in the conversation before first launch, treats independently evaluable outcomes as separate Plan Steps even when serial, and identifies shared surfaces as sequencing concerns rather than automatic consolidation.

The role now states that a slice normally spans multiple turns and that ending an idle turn does not settle or abandon it. On every skill load and after every operation, the reader must identify work it can perform now without a child result. If none exists, it records the pending callback briefly and ends immediately. Waiting, holding a gate, maintaining a register, and remaining ready are explicitly states rather than actions.

After task route and activation or delivery are evidenced, task listing, reading, or waiting solely for activity or progress is outside the role. The reader treats later routed input as its next opportunity to act rather than polling to compensate for uncertain activation.

Plan Step model choice follows ambiguity: Luna for low ambiguity and Terra for high. Reasoning independently rises from low through high with context uncertainty, breadth, and potential blast radius. Each projected step now presents those assessments and explains why its profile fits better than adjacent settings. Terra/high is not a confidence default, and a shared slice-level difficulty cannot justify every child profile.

The Slice owner now performs ordinary artifact inspection, acceptance judgment, and correction discovery itself. A separate review or verification Plan Step requires explicit independent evidence or a distinct unresolved concern, which must be named together with the evidence it contributes. Other genuinely separate outcomes remain available as Plan Steps.

The full Slice Plan is presented at initial creation, on request, and as the first operation after context compaction. Ordinary plan revisions may show only their affected consequences. Whenever the runner routes work to another task, its final output ends with an `Action summary` naming the action, destination, evidenced routing state, and expected return.

## Evaluation

The revision addresses planning quality and the observed failure after a live skill reload without requiring a fixed number of Plan Steps or forbidding legitimate serial execution. A bounded launch confirmation remains available only when a usable task route or activation is not yet evidenced. The wording distinguishes an unresolved slice from actionable work in the current turn and does not claim that callback delivery itself guarantees receiver activation.

A fresh read-only test loaded the revised role with an incomplete slice, one evidenced active Plan Step, no returned result, and no other ready work. It chose to yield the turn immediately without polling, relaunching, replanning, or beginning evaluation.

A correction-routing test ended with the required `Action summary` after its disposition, naming the existing destination, evidenced delivery and activation, and expected correction return.

This boundary removes the observed default implementation-then-review pattern without preventing independent evidence where acceptance genuinely requires it. It also keeps review findings in the Slice owner's existing evaluation loop rather than introducing another routine handoff.
