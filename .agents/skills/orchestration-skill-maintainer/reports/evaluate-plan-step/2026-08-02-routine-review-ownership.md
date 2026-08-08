# Routine review ownership

## Observation and theory

Two consecutive Plan Slices projected a standing independent review after implementation. In the later Slice, the owner already inspected the returned implementation, found material defects, routed corrections, and judged each return before launching the independent reviewer. `evaluate-plan-step` nevertheless said to represent independent review as another Plan Step whenever it was required evidence, without distinguishing that evidence from the evaluation the Slice owner already performs.

## Revision

`evaluate-plan-step` now assigns ordinary artifact inspection, evidence sufficiency, acceptance, and correction routing directly to the Slice conversation. Another review or verification Plan Step is reserved for explicitly required independent evidence or a distinct unresolved concern, and its specification must state what evidence it adds beyond rechecking the implementer's return.

## Evaluation

The revision preserves independent review as a legitimate work outcome while removing it as a routine confidence hedge. It aligns review findings with the owner that can immediately accept work, route corrections, and update the Slice Plan.

A fresh boundary test supplied high-risk committed work with complete cited evidence but no independent-review requirement or distinct unresolved concern. The reader kept inspection and acceptance in the Slice conversation, created no review Plan Step, and proceeded toward combined completion only after its own evaluation.

A complementary test supplied an explicit requirement for a separate adversarial deny-path assessment and threat-case matrix. The reader retained that as a bounded review Plan Step after evaluating the implementation return, demonstrating that independently required evidence remains supported.
