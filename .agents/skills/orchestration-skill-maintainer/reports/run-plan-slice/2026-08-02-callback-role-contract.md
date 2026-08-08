# Plan Slice callback and yielding contract

## Observation and theory

The callback wording correctly made a returned Plan Step disposition actionable, but the waiting boundary remained framed mainly as a ban on polling tools. In the Windows cancellation Slice, the planner kept one turn active while ingesting and narrating intermediate worker compilation, validation, and correction progress. It treated streamed progress as new Slice work even though no evaluable disposition had returned.

The role needs two complementary boundaries: an evaluable Plan Step return starts a decision turn, while intermediate progress after a launch or correction does not keep that turn open.

## Revision

`run-plan-slice` now directs the Slice planner to end the turn after launching a Plan Step or sending a correction unless distinct Slice-owned work can proceed independently. Intermediate Plan Step progress is explicitly non-actionable: the planner does not inspect, summarize, relay, or wait on it. Evaluation resumes when the Plan Step proactively returns an evaluable disposition.

The existing behavior remains: a returned disposition begins evaluation and the planner continues through any resulting correction, plan revision, ready packet, or combined completion judgment until a genuine external gate remains.

## Evaluation

The change targets token-consuming progress supervision without weakening parallel Slice work, ordinary evaluation, corrections, or proactive callbacks. It does not require silence when the Slice planner has independent work ready now, and it does not defer action after a completed or otherwise evaluable return.

The saved catalogue and the three relevant Slice checkouts at `89a3`, `1dcd`, and `955e` contain the same complete skill definition. SHA-256 for all four copies is `9523ACBF19021E3A32B174D660BBF1A97304F457ABDCD30410F095441D800B93`.
