---
name: resolve-epic-ambiguity
description: Resolve and durably record a consequential Sprint ambiguity on behalf of the user during an explicitly automatic Epic run. Use when a Sprint Runner routes a decision to the Epic Runner because waiting for human input would interrupt auto execution, while the choice must remain visible, opinionated, bounded, reversible where practical, and open to later human criticism.
---

# Resolve Epic Ambiguity

Make one explicit Epic-level choice without disguising judgment as fact.

## 1. Confirm authority

Verify:

- automatic Epic execution is enabled;
- the decision was routed to the Epic Runner;
- the choice is within delegated product, risk, and repository authority;
- the latest safe decision point has been reached.

Do not use this skill to override an explicitly human-reserved decision or authorize destructive,
external, paid, security-sensitive, or otherwise ungranted action.

## 2. Frame the ambiguity

Inspect the decision packet, current evidence, affected plan, and repository state. State:

- the exact question;
- why a choice is required now;
- viable alternatives and meaningful tradeoffs;
- assumptions and missing evidence;
- consequences of postponement.

Do not manufacture false balance. Keep rejected options visible when they were credible.

## 3. Choose opinionatedly

Select one option and label it as a decision. Explain why it best advances the Epic under current
evidence and constraints. State confidence and what evidence could overturn it.

Prefer a choice that:

- satisfies the current need without speculative expansion;
- localizes the assumption behind a focused boundary;
- avoids spreading the choice through unrelated contracts;
- can be replaced or revised without rewriting the whole feature;
- has observable acceptance evidence.

Do not describe the choice as inevitable, self-evident, or the only reasonable design.

## 4. Record before implementation

Write or update the project-designated durable decision record before affected work launches. Use an
authoritative product command when available; otherwise write an explicit decision artifact without
claiming an Orchestration Event exists. Record:

- decision ID and title;
- status: `auto_decided`;
- trigger, timestamp, and Epic/Sprint context;
- delegated authority source;
- ambiguity and latest safe decision point;
- alternatives considered;
- chosen option and rationale;
- confidence, assumptions, and overturning evidence;
- affected scope and explicit non-goals;
- containment and reversal path;
- acceptance evidence;
- later human-review questions;
- source references.

Keep the record concise and link it from the Sprint ambiguity register. Preserve superseded
decisions rather than rewriting their history.

## 5. Return the decision

Send the Sprint Runner:

- decision ID and chosen option;
- decision-record path;
- implementation constraints and reversal boundary;
- required validation;
- whether affected Work Units are now eligible.

Automatic execution may continue without human interruption when the decision is within authority.
When a human later criticizes it, revise through a new decision record and re-plan affected work
rather than pretending the original choice was never made.
