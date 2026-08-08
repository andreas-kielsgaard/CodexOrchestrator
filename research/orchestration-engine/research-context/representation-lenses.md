# Presentation and representation lenses

This note records how the current research might become understandable to different roles. It informs what relationships and metadata are useful to preserve; it is not a presentation specification or a standing rule for future documentation.

## Shared visual vocabulary

The same capability can be shown through different lenses if the underlying records preserve:

- capability or responsibility;
- owning artifact and implementation layer;
- caller and consumer;
- transport or interface;
- state/evidence authority;
- runtime reachability;
- configuration source;
- lifecycle stage;
- branch/history scope;
- unresolved question or contradiction.

Those dimensions are more reusable than prose organized only by source directory.

## Product-owner views

Useful forms:

- capability portfolio: what value exists and for whom;
- value journey: idea to accepted integrated outcome;
- release/conditional/debug/branch-only/quarantined rings;
- implemented depth versus visible product value;
- decision map for keep, connect, tune, extract, simplify or retire.

The research should therefore preserve product purpose, complete user outcome and qualification—not only code presence.

## Product-architect views

Useful forms:

- bounded-context and dependency map;
- transport/control-plane map for Tauri, events, MCP and external processes;
- authority graph for user, application, agent, database, filesystem and Git;
- durable-state domain map;
- configuration provenance graph;
- cross-store consistency boundaries;
- branch capability topology.

The research should preserve direction, authority and lifecycle boundaries rather than flattening everything into “backend.”

## Expert-developer views

Useful forms:

- code ownership and artifact map;
- operation matrix linking frontend caller, Tauri command, service, repository, process and result;
- hotspot map with responsibilities and test density;
- productive/debug/test/legacy reachability map;
- duplication and drift map;
- representative sequence traces.

The research should keep exact file paths, entry points, schemas, tool names and evidence for reachability.

## Expert-designer views

Useful forms:

- information architecture and contextual-navigation map;
- user-visible journey above hidden agent/evidence stages;
- surface/reachability map;
- reusable component ownership map;
- status-language matrix;
- progressive-disclosure model for evidence and oversight;
- role/session constellation around one Epic, Sprint or Work Unit.

The research should preserve what a user can see, initiate, understand and return to, plus where visible language could overstate underlying facts.

## Cross-role artifacts

Some visualizations can bridge all four perspectives:

| Artifact | Product owner | Architect | Developer | Designer |
| --- | --- | --- | --- | --- |
| capability topology | portfolio scope | context boundaries | implementation owners | surface presence |
| operation matrix | available actions | authority/transport | exact call path | user initiation/feedback |
| lifecycle swimlane | value movement | state machines | handlers/effects | progress language |
| configuration provenance | governance | source of truth | concrete artifacts | inspectability/control |
| branch timeline | created investment | integration topology | commits/diffs | experience evolution |
| evidence ladder | trust promise | authority stages | durable facts | progressive disclosure |

## Guidance, not constraint

Findings should not be discarded because they do not fit these shapes. If the investigation reveals a better organizing relationship, the repository structure and eventual visuals should change. The immediate purpose is to collect enough typed evidence that multiple honest representations remain possible.

The behavior-led passes shift the preference toward a calm dominant explanation with evidence depth behind it. Branch scope, reachability and incomplete handoffs should be prominent when they change the meaning of a capability; routine uncertainty should not compete equally with the product story. Legibility is the presentation objective, while the research repository preserves the denser truth.

The evidence-selected traversals sharpen that preference toward an operation-centered surface. A reader should first see a few consequential journeys or operating loops, then reveal authority, configuration, lifecycle and code-artifact overlays for the selected step. This better represents how real behavior is assembled across modules without asking every role to decode the full implementation graph.

## Lessons from the first visual review

The first prototype made four roles reinterpret the same journey and platform tiles. That supported comparison but imposed one ontology on every reader and produced more inventory than insight. The revised preference is:

- give each perspective a distinct question and visual structure;
- use the overview as an atlas rather than a long report;
- let a selected concept open interpretation and relationships before exact evidence;
- describe emerging work by product meaning rather than repository topology;
- keep the prototype lightweight instead of fleshing every component to equal depth.

These are choices for the current research presentation, not requirements for a permanent product or documentation system.

A fresh-context visible audit then found that the page spent too much space repeating framing while its strongest interpretations remained subordinate. The current pass compresses the framing, promotes one conclusion per canvas, combines conditional seams, and gives the Implementation canvas one real vertical path—Epic initiation—with concept and evidence depth behind each boundary.
