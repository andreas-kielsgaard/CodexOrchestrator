# Product-owner perspective

## Product that has been created

The implementation now describes a product lifecycle substantially broader than an “Epic planning workflow”:

1. a user discusses and builds an Epic proposal with a managed Plan Builder;
2. the user explicitly confirms initiation;
3. an application-owned bootstrap agent produces durable materials;
4. an Epic Runner selects and starts Sprints;
5. Sprint Runners request plans at current temporal decision points;
6. Work Units are materialized into dependency-aware execution;
7. Handler and Implementer agents work in isolated, authorized workspaces;
8. implementation claims, captured evidence and independent review are separated;
9. accepted work is integrated and dependencies advance;
10. handbacks, retries, escalations and later settlement are modeled as explicit outcomes.

Agent Sessions, Harnesses, File Review, Native Profiles, Human Review tooling and Product Decisions surround that lifecycle as reusable or governance-oriented capabilities.

## Capability portfolio

| Product area | Value already present | Current qualification |
| --- | --- | --- |
| Agent Sessions | durable agent conversations, continuation, cancellation, normalized transcript and reusable embedded workspace | clear productive platform |
| Epic Plan Builder | conversation-first proposal, durable revisions and explicit human initiation | clear productive entry point |
| Epic bootstrap and Runner activation | converts approved plan into bounded materials and managed ownership | productive but operational depth is not yet easy to see |
| Sprint/Work Slice planning | current-context planning, refinement, materialization and handback | productive agent-driven control plane |
| Work Unit execution | isolated attempts, evidence capture, review, retry, integration and dependency settlement | deep productive spine |
| Harnesses | role policy, prompts, skills, tools, sandbox and immutable variants | productive configuration; management product is only partially connected |
| File Review | safe opaque artifact viewing and rich review experience | stored artifacts productive; release-time contextual production unavailable in baseline |
| Native Profiles | application-owned Codex identity, readiness, sandbox and danger authority | release-visible control plane; selected home now gates all Agent Sessions, while mode and launch-policy convergence remain incomplete |
| Product Decisions | accepted version history and agent-assisted correction | substantial sibling-line capability, not integrated into baseline |
| Sprint/Epic final settlement | strict durable closure facts | substantial sibling-line capability without Tauri additions |
| Human/Worktree Review | isolated builds, launch, comparison and evidence | mature internal/debug product rather than release feature |
| legacy Tasks | earlier task/run product | deliberately quarantined compatibility |

## Product reality versus product visibility

The backend implements more governance and operational truth than the mounted user experience communicates. Examples:

- a visible Work Unit can represent original Handler work, Handler action continuation, original Implementer work, Implementer reporting, Handler review, Git integration and dependency settlement;
- a Harness inspection view often shows only a compiled Plan Builder profile while execution uses durable pinned variants;
- Native Profile settings now select and bind the home used by ordinary and managed Agent Sessions, but their execution-mode and danger controls do not govern the shared runtime path;
- File Review can render stored evidence in release, but producing it contextually is debug-composed;
- Product Decisions and final settlement exist on divergent sibling lines and are easy to overlook if “the product” is equated with one checkout.

This gap is not only documentation debt. It affects what value a user can discover, trust and control.

## Product strengths visible in the implementation

- Explicit user confirmation remains a real boundary for Epic initiation.
- Agent claims are not treated as application evidence.
- Launch, provider activity, semantic completion, review, integration and settlement are intentionally distinct.
- Work Unit execution is bounded by application-owned identity and workspace authority.
- Restart reconciliation and idempotency are designed into the core lifecycle.
- Incomplete work can become retry, wait, handback or attention rather than an undifferentiated failure.
- Reusable Agent Session and detail-workspace components support a coherent product shell.

## Product risks and scope questions

- The created capability surface may be too broad to reason about as one “Epic workflow” feature.
- Some release-visible actions are incomplete seams, which can create false expectations.
- Internal tooling and product functionality are adjacent enough to blur intended audience.
- The Harness configuration product is more ambitious in UI/data shape than its current connected backend surface.
- Divergent capability lines make portfolio decisions difficult without an intentional integration view.
- A large retained legacy product increases apparent scope and maintenance even though it is unreachable.

## Useful product views for later presentation

- a capability portfolio with rings for release, conditional, sibling-line, debug/operator and quarantined scope;
- a customer-value journey from idea through accepted integrated work;
- a control-and-trust ladder showing which outcomes require user, application, agent, Git or evidence authority;
- a “visible experience versus implemented depth” comparison;
- a roadmap-option map that treats keeping, connecting, simplifying, extracting and retiring as separate decisions.

## Questions to carry forward

- What is the smallest coherent promise of the product today?
- Which governance capabilities should a user directly see and control, and which should remain automatic safeguards?
- Is Harness Management an end-user capability, product-administration capability or internal design tool?
- Are Native Profiles part of Agent Session creation, global Technical Settings, or a future execution-policy system?
- Should Product Decisions and final settlement be integrated before further feature growth?
- Which internal review capabilities should become supported operator product, if any?
