# Epic product decisions exploration

## Purpose

Product decisions are durable reasoning-level stances that shape product identity. They exclude incidental implementation facts and constraints better enforced by tests, types, formatters, or lint rules.

This exploration keeps five concepts separate:

- canonical Epic-scoped decisions;
- eligible evidence from human interaction or completed work;
- AI-generated change candidates;
- detected conflicts requiring human judgment;
- requests for a later codebase compliance review.

The source is application-owned and keyed by Epic. Agent Sessions may contribute evidence but do not own decisions.

## Trigger and event direction

Future eligible events are `human_interaction`, `agent_session_completed`, `work_unit_approved`, `sprint_completed`, and `epic_completed`. Each evidence record carries an immutable typed reference to an opaque identifier for its originating application record. This provenance identifies eligible context; it does not claim that the event caused a decision. A trigger should record that evidence reference and request compilation asynchronously. Human interaction should mean an application-recognized product stance or review outcome, not every click or message.

Compilation should read the bounded evidence plus current canonical decisions and produce candidates with provenance. Trigger delivery, model invocation, retry/idempotency, and persistence are not implemented here.

## Authority and reconciliation

AI output is never canonical on arrival. A candidate may propose an introduction, refinement, or combination and name the decisions it targets. Reconciliation compares candidates with the current decision graph.

- Compatible candidates may eventually be condensed into one proposed canonical revision while retaining evidence and supersession edges.
- Contrary candidates create a separate conflict. They do not rewrite policy until a human resolves them.
- Accepted supersession or a newly completed policy can request a manual compliance review. A request does not claim that an audit ran.

This exploration is read-only. It offers no accept, reject, compile, reconcile, or audit controls.

## Graph evolution

The current tree is a navigable projection, not the final storage shape. The exploratory validator requires the supplied decision graph to be internally complete enough to validate lineage targets and cycles; this does not prescribe a durable historical-storage schema. Durable storage should evolve toward nodes for decisions, evidence, candidates, conflicts, and compliance-review requests, with edges such as `associated_evidence`, `parent_of`, `targets`, `conflicts_with`, and `supersedes`. Agents should query the current canonical subgraph for an Epic and follow provenance or history only when needed.

Historical decisions should remain addressable after supersession. Concision comes from the current projection, not deletion of lineage.

## Demonstrated boundary and limits

`EpicProductDecisionSource` is the reusable Epic-scoped application read boundary. The existing Epic detail flow renders `EpicProductDecisionsPanel` in its contained primary region. Only internal tree, detail, and review regions may overflow-scroll; the outer detail workspace remains fixed.

The adapter and data are recorded development fixtures available through the existing `?recorded-plan-builder` development composition. Product boot neither imports nor injects them. The demo proves typed validation, read-state handling, canonical/candidate/conflict separation, neutral evidence presentation with opaque origin references, a recorded compliance-review request, and in-app placement. It does not prove AI extraction, durable persistence, automatic reconciliation, product invocation, conflict resolution, graph queries, or codebase audit execution.

## User review points

Before production work, confirm:

1. which human interactions count as explicit product-stance evidence;
2. whether compatible, non-conflicting candidates may ever become canonical without approval;
3. the minimum provenance visible in the normal view;
4. whether compliance-review requests are user-created, policy-created, or both;
5. whether decisions inherit across Epic, product, and organization scopes.

## Exact next product slice

Persist one `work_unit_approved` evidence event and one idempotent compilation request against its Epic. Store the resulting candidate separately from canonical decisions, expose it through the existing source, and add one human review command that either rejects the candidate or accepts a new canonical revision with provenance. If acceptance supersedes an earlier decision, record—but do not execute—a compliance-review request. Defer other triggers, automatic compatible reconciliation, graph queries, and audit execution.
