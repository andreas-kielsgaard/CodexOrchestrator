---
name: orchestration-skill-maintainer
description: Maintain agent skills from observed behavior and user feedback. Use when a skill is missing, unclear, over-scoped, under-specified, written for the wrong reader context, or appears to have contributed to unintended agent behavior.
---

# Skill Maintainer

Maintain skill definitions from evidence about how their readers behave.

Treat possession of a skill as evidence of the reader's role. Give that reader only the information needed to perform the role inferred by the skill.

Keep maintenance rationale, comparisons, migration notes, compatibility explanations, and edit history in reports rather than target skills.

## Maintain Continuously

Assess observed behavior and user feedback against the relevant skill and reader context.

Begin revision when:

- the user identifies behavior as inappropriate, unintended, or undesirable, and evidence indicates that an in-scope skill contributed; or
- investigation independently reaches that conclusion.

When evidence is uncertain or the skill did not contribute, continue investigation or explain the assessment. Keep revisions proportional to the evidence and edit only clearly owned targets.

A confirmed behavioral defect establishes that a revision is warranted; current mutation authority determines whether to apply it. Under a report-only, read-only, or no-edit boundary, provide the revision concept and affected targets as a proposal. A later request to reflect, elaborate, or consider alternatives preserves that boundary unless it clearly authorizes edits.

## Establish The Reader

Before revising, determine:

- the role inferred by receiving the skill;
- the prompt, state, tools, sandbox, workspace, model, reasoning, and lifecycle context supplied to that reader;
- what the reader can observe, decide, change, and report;
- the authority and boundaries that affect its actions; and
- the output or return route needed to complete its responsibility.

Inspect relevant sessions, harness configuration, source material, and observed outputs where useful.

Treat storage and discovery as part of the reader context. Identify the harness that owns exposure and its automatic discovery roots before choosing a skill location. Store the definition in the owning system's catalogue; nesting it beneath another harness's namespace does not transfer ownership. When exposure must be selective, keep the source outside automatic discovery roots and let the owning harness supply it.

Assume prerequisites already established before the reader receives the skill. Exclude instructions about creating or preparing that prerequisite environment unless the reader can encounter and act on that state.

Exclude wider structure, adjacent roles, alternate workflows, implementation history, and maintenance rationale unless one of those facts changes what this reader must do.

Translate every retained fact into an action, boundary, decision clue, or evidence requirement meaningful from the reader's perspective. Assign only actions the reader can perform.

## Revise

For each revision:

1. Inspect the behavior, skill, and reader context.
2. State why the wording, omission, or reader mismatch likely produced the behavior.
3. Propose a small revision concept.
4. Evaluate likely improvement, ambiguity, and constraint risk.
5. Reconsider the complete skill and reformulate it coherently rather than appending a scenario note.
6. Apply the revision within the authorized scope.
7. Save the current analysis and revision under `reports/[skill-name]`.
8. Report the result and validation.

When later feedback changes the revision, rewrite its report as the current account.

Keep running sessions unchanged. Maintain definitions; do not use a skill revision as authority to steer active work.

## Create When Needed

Create a skill when evidence identifies a distinct reader role or reusable behavior that adjacent skills do not cover. Record what triggered creation, why adjacent skills were insufficient, who reads it, what context that reader receives, and why the wording belongs in that reader's instructions.

## Write For The Reader

- State desired actions, evidence, and boundaries directly.
- Include shared material only when its operational content helps this reader act.
- Prefer concise clues over broad explanation.
- Use prohibitions for evidenced risks that affirmative guidance would leave ambiguous.
- Use closed lists only for genuinely exhaustive requirements; otherwise preserve room for analogous cases.
- Keep examples broad and label them non-exhaustive when appropriate.
- Remove sentences that merely explain why the skill, role, name, or surrounding structure exists.
- Produce a proposal when ownership or reader context is not clear enough for a safe edit.
