---
name: orchestration-skill-maintainer
description: Maintain skills used by agents in the Codex Orchestrator product. Use when observed agent behavior suggests that a categorized agent is behaving inappropriately, or that skill is missing, unclear, or producing unexpected behavior, or when prompted to make skill revisions by a user
---

# Orchestration Skill Maintainer

Maintain skills for agent roles in the Codex Orchestrator product.

`../_shared-skill-concepts/` may contain relevant context. This folder is for you to use and maintain. 
Do not reference that library directly in skills. If ideas in a shared concept are relevant to a skill, translate them role-local wording.
If you revise a shared skill, this should not automatically trigger changes in all skills that use that concept. Only if revision of that skill is part of the scope of your skill maintenance task.

Before revising a skill, understand how that skill is used in the context of Orchestration product. 

# Revision procedure

For each revision:

1. If possible, find and inspect conversations that have exhibited behavior relevant to the skill revision. 
2. Propose a theory for why the current skill, or lack of a skill, led to the behavior that the skill revision aims to change.
3. Propose a revision concept.
4. Reason about whether that concept is likely to correct the behavior.
5. Apply the revision.
6. Save the analysis and revi   sion details under `reports/[skill-name]` in this skill directory. If you rename a skill, also rename the corresponding reports folder.
7. Report changes

If user provides feedback on the revision, take it into consideration. If that results in further changes, rewrite the revision report instead of making another one, or appending the new information to the first one.

Do not prompt running agents to apply the revision concepts ad-hoc. You role is just to observe agent behavior and maintain the skill definitions. 

# Creating a new skill

If you investigation reveals that a new skill would be valuable, you are authorized to create it. It should be clearly scoped when this skill is relevant, and that scope should be clearly differentiable from current skill selection.

Write an initial report that explains
1: What triggered the creation of this skill
2: Why adjacent skills was not sufficient
3: How the wording of this skill satisfied the requirement

# Skill writing guidlines

- Prefer low-verbosity skill wording. 
- Prefer clues over rules until clues have proven ineffective. 
- If a skill's scope, or the revision's relationship to that scope, is unclear, prefer vague guidance over premature constraint.
- Prefer iterative low-risk revision that is likely to improve behavior somewhat over a potentially perfect revision that may make behavior worse or introduce new problems.
