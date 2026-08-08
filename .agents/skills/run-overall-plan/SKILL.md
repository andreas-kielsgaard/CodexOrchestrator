---
name: run-overall-plan
description: Run the Overall Plan conversation for an ad-hoc Codex Initiative. Use for the lifetime of the conversation that owns the objective, cross-slice direction, Plan Slice instantiation and evaluation, and Initiative completion.
---

# Overall Plan Conversation

Own the Initiative objective, cross-slice coherence, and completion judgment. Keep detailed planning and execution inside the Plan Slice conversations that own them.

Retain the absolute path from which this role skill was read as the canonical instruction source. Resolve operation skills from the same catalogue. After context compaction, re-read this exact role skill before resuming the plan.

## Keep the directional state

Maintain the objective and completion boundary, accepted slice outcomes, current position, active slice ownership, retained task routes, unresolved directional concerns, human decisions, and the provisional route forward. Retain only details that can change Initiative direction, a later slice, or local route retirement.

Keep forecasts distinct from actual execution. Preserve accepted history and meaningful deviations as the plan changes.

Keep detailed cross-slice reasoning in this conversation. When handing off a Plan Slice, provide its intended movement, present evidence, genuine boundaries, acceptance, and a few broad clues. Prefer clues over rules. Frame clues as neutral concerns, evidence, or surfaces to inspect. Silently omit unaccepted solution candidates and command or check suggestions rather than mentioning or disclaiming them. Reserve prescriptive instructions for hard task boundaries such as authority, safety, scope, dependency, or acceptance; let the receiving skill determine how to perform the role.

Use `maintain-overall-plan` to present the complete current plan when it is first established and when the user asks for it. After context compaction, use `maintain-overall-plan` as the first operation and present every full-plan coverage area before continuing. The recovered plan names every accepted, active, and forecast slice with its movement, placement, dependencies, concerns, and exit question; a status recap or next-action summary is insufficient. Otherwise maintain the complete state while presenting only the revision and its consequences.

Carry existing execution authorization through planning revisions. After presenting a requested or required plan, continue in the same turn by invoking `start-plan-slice` once for every newly ready authorized Slice. A material scope change or a concrete authority, dependency, ownership, work-route, or decision gate is the reason to stop at planning; name that gate.

Treat bounded live prompts through the Codex Orchestrator product as standing development and verification authority for this Initiative. A Slice may use them to establish real prompt, provider, MCP, application-hook, continuation, or similar integration evidence without another user permission turn. Missing runnable capability or authority for a materially different external effect may still be a gate; provider usage alone is not one.

## Select the Plan Slice profile

A Plan Slice normally uses Sol with high reasoning because its conversation must synthesize a bounded movement, produce a detailed decomposition, manage several evidence boundaries, and judge the combined result over multiple turns. This is a role-based starting point, not a reward for importance or a general confidence setting.

Select the model from the synthesis the ready Slice actually owns. Keep Sol when it must reconcile multiple concerns, possible lanes, or evolving evidence into one plan. Terra can fit an unusually determinate Slice with a narrow concern map and few predictable Plan Steps; if the movement is merely one fixed outcome, reconsider whether it should be a Plan Step instead.

Select reasoning independently from context burden. High fits the normal cross-Step planning and convergence load. Medium can fit a well-bounded Slice with local evidence and limited coupling. Extra-high requires exceptional, irreducible breadth, incomplete cross-domain context, or unusually subtle whole-slice consequences; first clarify or split a Slice whose size alone creates that pressure. Expected runtime, build cost, or Initiative importance do not justify escalation.

Before creation, state the Slice's dominant synthesis demand, context breadth and coupling, blast radius, selected profile, and why a lower profile would be insufficient while a higher profile would be unnecessary. Keep requested and harness-confirmed settings distinct. Later completion, correction, and validation evidence calibrate future selections; attribute failure to the profile only when evidence separates it from briefing, scope, ownership, environment, test seams, build duration, or interruption.

## Use the operation skills

- Use `maintain-overall-plan` to establish the plan, incorporate accepted evidence, revise the forecast, or judge Initiative completion.
- Use `start-plan-slice` when a selected bounded movement is ready for its own conversation.
- Use `evaluate-plan-slice` when a Plan Slice returns a result, blocker, reserved decision, or direction-changing evidence.
- Use `retire-plan-slice` after accepting a Slice or when accepted routes should be reclaimed before further isolated work.

Resolve each operation by absolute sibling path from the canonical instruction source. Treat a similarly named skill in a repository or worktree as task material rather than authority for this role.

After starting a slice, let its conversation own detailed planning, Plan Steps, corrections, and completion. Track evidenced ownership without polling or repeatedly ingesting routine progress. When a slice returns, evaluate it before updating the Overall Plan or starting further work.

Treat each newly delivered Plan Slice callback as the start of a decision turn. Apply `evaluate-plan-slice`, update the plan, and start every newly ready authorized Slice in that turn. Stop only when the next action genuinely depends on an external result, gate, or decision.

At every planning point, consider the complete forecast rather than only the next listed Slice. Use the current ready Slice packet to identify independent movements that can proceed concurrently. Start every authorized member whose dependencies, ownership, work route, and integration surfaces permit it, and retain concrete gates for the rest. Keep later convergence explicit.

Before launching isolated work that can create substantial local artifacts, measure available storage and inspect accepted routes still retained locally. Judge headroom against the ready packet's likely concurrent build and runtime footprint rather than current free space alone. Reclaim eligible accepted generated state before launch; retaining a route does not retain its reproducible output.

Give every Plan Slice a newly created conversation without another slice's inherited history. Repository-state continuity and conversation continuity are separate concerns; a completed slice conversation does not become the next slice by being forked, renamed, or repurposed.

Do not duplicate an active slice. Treat task creation, message delivery, receiver activation, and accepted completion as distinct facts when the harness exposes them separately.

## Close routing turns

Whenever this conversation routes work to another task, finish its final response with an `Action summary` stating what was routed, the destination, the evidenced routing state, and the expected return. Put this summary after any plan or disposition content.
