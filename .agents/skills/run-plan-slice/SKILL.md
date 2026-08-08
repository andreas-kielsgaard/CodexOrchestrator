---
name: run-plan-slice
description: Run one Plan Slice conversation in an ad-hoc Codex Initiative. Use for the lifetime of the conversation that owns detailed slice planning, Plan Step instantiation and evaluation, combined acceptance, and return to the Overall Plan conversation.
---

# Plan Slice Conversation

Own one bounded movement from its supplied objective through a combined, published result. Plan, launch, and evaluate Plan Steps; leave each step's assigned work to its own conversation.

Read the launch routing header before acting. Retain its absolute role-skill path, instruction catalogue, repository or worktree route, and callback task as distinct state. Resolve operation skills only from that catalogue. After context compaction, re-read the exact role-skill path before maintaining the plan or routing work.

## Keep the slice state

Maintain the slice frame, evidence baseline, planning revisions, concern map, decisions, projected and actual Plan Steps, their task routes, dependencies, gates, accepted outcomes, retention reasons, and remaining completion evidence.

Distinguish a concern, a projected step, an evidenced task, a returned result, and an accepted outcome. Preserve superseded projections and actual execution history when evidence changes the plan.

Ground planning in the supplied handoff, current repository or worktree, and authoritative artifacts. Discover the resolution independently. Treat conversation ids as routing addresses rather than evidence sources; do not read other conversations to reconstruct context or borrow their planning. Surface a material missing fact as an evidence gap, blocker, or reserved decision.

## Maintain and present the plan

Use `maintain-slice-plan` to maintain the complete current Slice Plan as evidence changes. Present the full planning breakdown before starting the first Plan Step and when the user asks to see, review, recap, or restate it. After context compaction, use `maintain-slice-plan` as the first operation and present every full-plan coverage area before continuing. The recovered plan includes its planning reasoning and detailed Plan Step specifications as well as execution history; a status recap or launch-register summary is insufficient. At other times, present the revision and its consequences without repeating unaffected plan detail.

Give distinct, independently evaluable outcomes their own Plan Steps even when dependencies require serial execution or the steps touch shared surfaces. Treat overlap as a sequencing and integration concern. Use one implementation step only when the slice is genuinely one coherent evaluation boundary.

Keep detailed decomposition reasoning in this conversation. Give each Plan Step its objective, success evidence, genuine boundaries, and a few broad clues. Prefer clues over rules. Frame clues as neutral concerns, evidence, or surfaces to inspect. Silently omit unaccepted solution candidates and command or check suggestions rather than mentioning or disclaiming them. Reserve prescriptive instructions for hard task boundaries such as authority, safety, scope, dependency, or acceptance; let `run-plan-step` govern execution and ordinary judgment.

Carry standing authority for bounded live prompts through the Codex Orchestrator product into any Step whose outcome needs real prompt-driven integration evidence. Do not create a user decision gate solely for writing or submitting those prompts. Let the Step choose the prompts, scenarios, and validation method; identify only the integration facts the Slice must eventually judge.

At every planning revision, consider every projected Plan Step for current eligibility rather than defaulting to one next task. Present the next ready packet, why its lanes can proceed concurrently, the shared surfaces and convergence point, what remains gated, and what the packet unlocks. Give a concrete dependency or overlap reason when only one step is ready.

## Select Plan Step profiles

Choose the model from solution ambiguity and the reasoning level independently from context burden.

Use Luna when the outcome, governing contracts, and evidence make the solution shape largely determinate. Typical, non-exhaustive fits include bounded implementation, local correction, documentation, established validation, and mechanical convergence. Use Terra when the Step must discover among plausible solutions or boundaries, reconcile uncertain behavior, or reason through novel integration, migration, concurrency, recovery, or adversarial findings. Plan Steps use Luna or Terra; an apparent need for Sol is a clue to reconsider whether the outcome should be decomposed or owned at the Plan Slice level.

Use low reasoning when the relevant context is local, explicit, reversible, and strongly verified. Use medium when several surfaces or constraints must be reconciled inside an understood boundary. Use high when the necessary context is broad or poorly defined, overlooked constraints could have a wide blast radius, or correctness depends on subtle invariants such as privacy, negative authority, recovery, concurrency, or historical compatibility. Luna/high and Terra/low are valid because the two axes answer different questions.

Task importance, expected duration, build cost, or a general desire for confidence do not raise either axis. Missing authority, environment, test seams, or evidence remain planning gates; a stronger profile does not repair them. When a Step appears to need the strongest setting mainly because it is large, clarify or decompose it before escalating.

For every projected Plan Step, present the dominant solution uncertainty, owned context and coupling, blast radius, selected model and reasoning, and why the adjacent lower setting is insufficient while the adjacent higher setting is unnecessary. Assess each Step independently; repeated profiles require repeated task-specific justification rather than a uniform default. Keep requested and harness-confirmed settings distinct, and use evaluation outcomes as later calibration evidence without blaming a profile for failures better explained by scope, environment, validation, or interruption.

## Use the operation skills

- Use `maintain-slice-plan` for the initial detailed plan and material replanning.
- Use `start-plan-steps` to instantiate each currently ready packet.
- Use `evaluate-plan-step` when a Plan Step returns or needs a slice-owned disposition.
- Use `complete-plan-slice` when all required outcomes appear dispositioned and the combined slice is ready for judgment.

Resolve each operation by absolute sibling path from the instruction catalogue in the launch header. A similarly named skill in the task worktree is not this conversation's instruction source.

Treat each newly delivered Plan Step callback as the start of an evaluation turn. Apply `evaluate-plan-step`, then continue in that turn through any resulting correction, plan revision, ready-packet launch, or `complete-plan-slice` judgment. Stop only when the next action genuinely depends on an external result, gate, or decision.

Let ready independent Plan Steps proceed in parallel. Route bounded corrections back to the owning Plan Step conversation. Represent additional exploration, integration, documentation, or similar outcomes as Plan Steps when genuinely required.

Evaluate returned Plan Steps in this conversation. Ordinary artifact inspection, acceptance judgment, and correction discovery belong to `evaluate-plan-step` and `complete-plan-slice`. Create a separate review or verification Plan Step only when Slice acceptance explicitly needs independent evidence or the plan identifies a distinct unresolved concern beyond evaluating the implementer's return. State that concern and the evidence the step contributes.

For repository-changing work, keep the accepted Step commits attributable, converge them onto the named Slice branch, and use `complete-plan-slice` to verify the clean exact checkpoint and publish that branch. Keep canonical-branch publication outside this Slice unless its handoff explicitly assigns that boundary.

## End turns while Plan Steps work

A Plan Slice normally spans multiple turns. After launching a Plan Step or sending it a correction, end the turn unless distinct Slice work can proceed independently now. Intermediate Plan Step progress is not an actionable return: do not inspect, summarize, relay, or wait on it. Resume evaluation when the Plan Step proactively returns an evaluable disposition.

If no action can be performed without a Plan Step result, state the pending callback in a short final response and end immediately. Ending the turn while Plan Steps are active does not complete, block, abandon, or settle the Slice.

Waiting for a result, holding a gate, maintaining a launch register, or remaining ready are states rather than actions. They do not justify commentary, task-status inspection, or an active turn.

Once a Plan Step's task route and activation or delivery are evidenced, do not call task listing, reading, or waiting tools merely to observe activity or progress. Continue only independent Slice work that is ready now. Treat later routed input as the next opportunity to evaluate a result; do not compensate for uncertain activation by polling.

Return to the Overall Plan conversation only after combined slice evaluation or when the slice needs a decision held there.

At that boundary, make one proactive return to the supplied callback route with the result, supporting evidence, relevant repository state, residuals, and unproven claims, then end the turn. Use a host continuation action when available. Record message delivery and receiver activation separately; a delivered but unactivated callback is not an evidenced return turn. Do not poll or repeat the callback.

## Close routing turns

Whenever this conversation routes work to another task, finish its final response with an `Action summary` stating what was routed, the destination, the evidenced routing state, and the expected return. Put this summary after any plan or disposition content.
