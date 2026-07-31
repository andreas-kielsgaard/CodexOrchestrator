# Objective-Grounded View Coaching

Status: general review-coach skill revised. Product code and harnesses unchanged.

## Observation

After re-ingesting the review-coach skill, coaching session `019fa398-6f66-76d2-b7b9-38bb2b4898c9` stated that the failed Plan Builder view did not explicitly explain recovery and then asked whether the user expected another message or a Retry control.

The source material discussed retry and recovery broadly but did not establish an explicit Sprint 6 UI objective requiring this view to present recovery instructions. The coach introduced its own design criterion and reduced the view review to a binary choice.

## Theory

The skill asked the coach to separate observations and design ideas, give concrete questions, and advance one step at a time. It did not establish who owned experiential judgment, require review concerns to come from explicit objectives, or define one view as an appropriate batch. “One focused question” encouraged serial exchanges, while “observation” left room for the coach to state an evaluative opinion as fact.

## Revision

The coach now separates factual description, sourced objectives, and user-owned evaluation. Explicit objectives produce neutral questions about whether the result fulfills them. Unsupported design preferences stay out of the foreground.

Each coherent view, flow, state, or technical area now receives one compact review frame containing all concerns supported by its objectives and evidence. If the scope is incomplete, the coach asks for a broader assessment and avoids implying completeness. Binary questions are reserved for genuinely bounded decisions.

Initial forward tests exposed two remaining loopholes: narrowing the frame to only one unresolved objective, and declaring that a view passed before asking for acceptance. The wording was tightened so the whole relevant objective set remains in the user's review frame and factual verification never becomes coach-owned acceptance.

A valid underspecified-view test then treated concerns absent from the recorded objectives as explicitly outside the gate. Scope guidance was tightened so known objectives are not assumed exhaustive unless their source establishes a comprehensive boundary; partial scope now produces a broader invitation without foregrounding speculative concerns.

The existing reviewer-fork, callback-routing, non-blocking delegation, and user-continuation boundaries remain intact.

## Evaluation

The revision should reduce anchoring and unnecessary back-and-forth while preserving useful orientation. It gives the user enough grounded concerns to review a view in one pass, yet avoids inventing criteria when the coach cannot scope the area accurately.

Final valid tests covered both cases: a comprehensively scoped view produced one frame containing every recorded objective, while an underspecified view stated its known objectives and invited a broader holistic assessment without introducing an unsupported concern or declaring acceptance.
