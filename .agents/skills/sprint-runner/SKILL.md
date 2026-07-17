---
name: sprint-runner
description: Plan, run, or revise one project Sprint by identifying problems, reducing consequential ambiguity before execution, allocating Planner responsibilities, defining launch-ready Work Units, and mapping dependencies, sequencing, parallelism, evaluation gates, models, and reasoning levels. Use for a Sprint handoff, dependency-aware work initiation, upfront acceptance decisions, Work Unit launch and review coordination, or replanning after outcomes and blockers. The skill coordinates the Sprint; it does not implement Work Units itself.
---

# Sprint Runner

This skill coordinates the current Codex development workflow. It is not the contract for the product's future Sprint Runner or Planner roles.

Produce a versioned, evidence-grounded plan that separates the Sprint’s problem structure from its projected executions, then coordinate accepted work.

Sprint Runners should run on the current Sol model with `high` reasoning by default. Do not lower Runner reasoning merely because some resulting Work Units are simple.

## Core distinctions

Keep these separate throughout the plan:

- **Sprint Concern:** a problem or sub-problem that must be resolved.
- **Planner responsibility:** durable decision authority for a concern or coherent concern group.
- **Planner Decision:** a point where evidence changes or confirms projected work.
- **Projected Work Unit:** an anticipated execution; it may later change or never be initiated.
- **Actual Work Unit:** an execution that has genuinely been launched.
- **Attempt/review cycle:** implementation, review, correction, and acceptance within one Work Unit.
- **Accepted outcome:** reviewed evidence returned to the Sprint.

Do not present a file list or backlog as a problem map. Do not treat agent-idle as responsibility-complete.

## 1. Orient from evidence

Inspect the handoff’s primary sources, repository state, relevant code, tests, plans, and recent accepted outcomes. Verify material claims.

Record:

- planning revision number;
- evidence baseline or commit;
- assumptions accepted for this revision;
- material uncertainty;
- changes since any previous revision.

Do not implement project work while planning.

## 2. Define the Sprint frame

State:

- objective and intended movement;
- completion or re-evaluation condition;
- authority boundaries;
- non-goals;
- invariants that all Work Units must preserve.

Use the Sprint boundary to prevent attractive but unrelated work from entering the plan.

## 3. Build the problem map

Decompose reality into concerns before proposing execution.

For every concern capture:

- stable ID and title;
- purpose and why it matters to the Sprint;
- evidence that the problem exists;
- expected resolution evidence;
- parent concern if any;
- coupling to other concerns;
- current uncertainty;
- suggested Planner responsibility.

Prefer a small hierarchy of meaningful concerns over a deep taxonomy.

## 4. Assess planning characteristics

For each concern assess:

| Dimension | Guidance |
|---|---|
| Definition | High: outcome and constraints are known. Medium: bounded choices remain. Low: product or technical meaning needs exploration. |
| Complexity | Narrow: local/mechanical. Medium: several collaborating boundaries. Broad: cross-system or high coordination. |
| Risk/blast radius | Data loss, process ownership, concurrency, migrations, security, public contracts, or broad UX assumptions raise risk. |
| Reversibility | Prefer earlier experiments when choices are expensive to unwind. |
| Verification | Identify deterministic evidence available before launch. |
| Work mode | Executable, executable-with-design-reasoning, exploration-led, or integration/review. |

Use these assessments as the basis for model, reasoning, sequencing, and evaluation gates.

## 5. Reduce ambiguity before launch

Identify choices that could materially change product behavior, scope, architecture, sequencing,
acceptance, or expensive rework. Do not elevate ordinary local implementation judgment.

For each material ambiguity record:

- stable decision ID and question;
- why it is consequential;
- known options and tradeoffs;
- evidence available now and evidence that must be produced first;
- latest safe decision point;
- owner in manual mode and owner in automatic Epic mode;
- status and durable decision-record location.

Classify each as:

- **upfront**: resolve before affected Work Units launch;
- **evidence-gated**: decide after a named exploration or validation result;
- **experiential**: requires evaluating produced behavior, such as a UI review;
- **local**: delegated implementation judgment within stated invariants.

Batch upfront user decisions before execution so an accepted plan can run with minimal interruption.
Do not pretend experiential decisions can be resolved without seeing the result.

Forecast every planned human-attention gate. For each, state whether it can be resolved upfront,
requires new evidence, or genuinely requires experiencing produced behavior. Consolidate compatible
questions into one review. Optimize for few meaningful interruptions, not for eliminating necessary
human judgment.

When the Epic is manual, unresolved user-owned decisions gate affected work. When automatic mode
explicitly delegates the choice, route it to the Epic Runner using `$resolve-epic-ambiguity`; keep it
non-blocking within that authority and require a durable, criticizable decision record. Choices
outside delegated authority still require the user.

## 6. Allocate Planner responsibilities

Assign persistent Planner responsibility when a concern needs several decisions over time. A Planner may create multiple Work Units and revise its local projection as outcomes return.

- Keep Sprint convergence under Sprint Runner authority.
- Let a concern Planner revise projected work when evidence shows that a presumed-ready Work Unit is
  not ready. It may propose changed scope or prerequisite units, but must return the proposal to the
  Sprint Runner rather than launching it unilaterally.

## 7. Design Work Units

Bundle work by coherent outcome and review boundary, not by file or profession.

Each Work Unit must include:

- stable ID and title;
- concerns addressed;
- objective and rationale;
- expected outcome;
- scope and likely modules;
- context the executing task needs;
- behavior/invariants to preserve;
- deliverables;
- broad validation-scope clues;
- explicit non-goals;
- risks and decision authority;
- material decision IDs that constrain the work and the record each implementation must follow;
- dependencies and evaluation gates;
- planned model and reasoning;
- result/report destination.

Separate exploratory questions from production implementation. An exploration unit should end in evidence, alternatives, or a user decision—not silently choose and ship a product behavior.

### Scope validation across this development Sprint

Use the shared validation-scope concept in `../_orchestration-common/concepts.md`.

Indicate only whether validation is expected around the Work Unit itself or after later integration or convergence. Let the executing agent decide how to validate its work. Do not name tests, checks, commands, or methods unless an external requirement fixes them.

Validation placement does not revise a Work Unit. Deferring validation never defers its implementation, deliverables, or local acceptance criteria. Record the owner of any deferred validation.

## 8. Select model and reasoning

Reflect on the clarity of the task context. 
If the requirements, problem and solution model is clearly defined use the Luna model.
If on-the-fly reasoning, evidence collection and more complex decisionmaking need to be made, use the Terra model.
Depending on the scope of the implementation slice, how much cross-application work is and how much evidence needs to be gathered, use reasoning level low - high. 

## 9. Map execution

Create both:

1. a concern-to-Work-Unit map explaining what execution is intended to resolve;
2. a dependency map showing ordering, parallel lanes, and convergence.

For every dependency state whether it is:

- hard (`requires`);
- preferred sequencings (`should_follow`);
- an evaluation gate requiring user or Planner judgment.

Do not infer safety from the absence of a dependency. Explain why parallel units are sufficiently independent and identify shared integration surfaces.

Show the first eligible units and final convergence unit explicitly.

## 10. Maintain the launch register

End every planning revision with a launch register immediately before the concise operational summary. Treat it as the current operational index of projected and actual Work Units, not as a replacement for their detailed specifications.

Use this table shape:

| Unit / gate | Expected work | Status | Reason |
|---|---|---|---|

- Include one row for every projected or actual Work Unit and every named evaluation gate, in execution order.
- In **Expected work**, summarize a Work Unit's intended action and outcome in one brief sentence; use `—` for a gate unless brief gate work is genuinely useful.
- In **Reason**, briefly explain the current status, unmet dependency, gate condition, or acceptance basis.
- Keep model, reasoning, detailed dependencies, and task/thread IDs in the sequence overview, Work Unit specification, and launch reports—not in the register.
- Preserve superseded rows. Never erase execution history to make the current projection look cleaner.
- When detail differs from the register, repair the register; the detailed Work Unit specification and recorded Planner Decisions remain authoritative.

## 11. Produce the detailed plan

Use this output order:

1. **Sprint frame**
2. **Evidence baseline and planning revision**
3. **Problem map**
4. **Complexity and definition assessment**
5. **Ambiguity register and upfront decision packet**
6. **Planner responsibilities**
7. **Sequence overview table** with unit, concern, model, reasoning, dependencies, and work mode
8. **Parallel lanes and gates**
9. **Detailed Work Unit specifications**
10. **Evidence and validation map**
11. **Risks and unresolved decisions**
12. **Launch register** using the required columns and update rules above
13. **Concise operational summary**

The final summary should state the Sprint objective, concern groups, execution lanes, first eligible
units, evaluation gates, resolved upfront choices, and remaining decision points in a compact form.

## 12. Launch from the accepted plan

When later directed to initiate work, unless specific instructions on what to launch is given, evaluate which work units are ready for initiation, and start all ready and parallel units in their own thread.

- verify its dependencies, gate state, repository baseline, and current launch status;
- avoid re-planning unless material reality changed;
- create a fresh project task with the planned model and reasoning;
- provide objective, authority, scoped context, constraints, acceptance evidence, validation, and report route;
- record the task/thread ID and mark the unit active;
- verify every required ambiguity is resolved or properly delegated before launch;
- do not actively poll running tasks; wait for task completion or delivered updates.

Worker output returns to its responsible Planner for review. Only reviewed and accepted outcomes should be promoted to the Sprint.

Launch work as new conversation threads with the appropriate model and reasoning levels.

## 13. Revise after outcomes

When blockers, surprises, or accepted outcomes change reality:

- preserve the prior projection;
- create a new planning revision;
- record the triggering evidence and Planner Decision;
- add newly discovered material ambiguities to the register and assign their decision path;
- distinguish projected changes from already executed work;
- supersede rather than erase obsolete units;
- update eligibility, dependencies, models, and gates;
- return a revised concise summary.

If a completion of the work produces work that could be shown as a user demo or requires user verification, provide instructions for the user to check it out instead of continuing work even if asked to - unless the ask is specifically overruling this rule.

The actual execution map is historical evidence. Never rewrite it to make the original projection appear correct.
