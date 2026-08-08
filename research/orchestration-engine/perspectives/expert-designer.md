# Expert-designer perspective

## Current experience architecture

The normal product shell presents three primary destinations:

- Orchestration;
- Agent Sessions;
- Technical Settings.

File Review appears contextually. Harness inspection is available contextually and through development routes; a full mutable Harness Management experience exists visually but is not connected to productive authoring. Human/Worktree Review is a debug experience. Product Decisions and more mature origin/return navigation exist on a sibling line.

## Reusable experience system

### Agent conversation

- `AgentSessionWorkspace` provides reusable conversation layout and input.
- `ConversationViewport` and transcript projection separate durable events from displayed turns.
- `SharedAgentSessionPanel` embeds the same live client inside orchestration views.
- `AgentIdentityBadge`, Markdown rendering/editing and invocation controls create a recognizable interaction language.

### Detail workspaces

- `DetailWorkspace` provides list/detail structure.
- `ResizableSplitSurface` supports transcript/evidence or navigation/detail layouts.
- `ProductViewHeader` supplies shared product framing.
- Epic, Sprint, Work Slice and Work Unit workspaces progressively reveal increasingly specific execution evidence.

### Review

File Review establishes a focused changed-files/navigation/content pattern. Worktree Review extends review into environment preparation, building, launching, comparison and proof, but currently for internal/debug audiences.

## Experience strengths

- Conversation is reused rather than reimplemented for each role.
- Contextual navigation can connect operational state to the exact Agent Session or review artifact.
- Opaque File Review references keep unsafe repository details out of the frontend contract.
- The implementation has enough durable state to present honest distinctions between waiting, blocked, running, accepted and settled.
- Recorded surfaces provide a broad visual test corpus for states that are difficult to reproduce live.

## Experience-model tension

### One product object, many hidden agents

A Work Unit shown as one item can involve several role continuations and evidence stages. The current experience must balance two needs:

- calm progress and outcome language for most users;
- exact claims, evidence and authority for diagnosis or oversight.

This argues for progressive disclosure rather than exposing the backend state machine directly or collapsing it into a misleading single status.

### Generic surfaces with narrow data

Harness Management visually implies broad authoring, versions, models, skills and tool control, but the mounted data source supplies a read-only compiled Plan Builder profile. The experience promise currently exceeds the connected capability.

### Settings with only partial policy consumption

Native Profile settings present selection, readiness and execution-mode control. Ordinary and managed Agent Sessions now consume the selected home identity, but not the selected execution mode or its strict launch policy. The experience needs to distinguish identity/readiness from the policy that actually governs a Session.

### Contextual tools with build-dependent availability

File Review’s viewer is product-grade, but producing the review contextually depends on debug-composed Human Review in the baseline. The visible request path and runtime availability can diverge.

## Information architecture questions

- Is “Orchestration” one destination containing planning, execution and oversight, or should those become distinct modes?
- Are Agent Sessions primary user objects or supporting evidence reachable from product work?
- Does Technical Settings own Native Profiles because they are global, or should execution policy be contextual to agents/workflows?
- Is Harness inspection contextual metadata, while Harness authoring belongs in administration?
- Should Product Decisions become a persistent Epic-level facet rather than a top-level destination?
- Where should review live: alongside the Work Unit, in an activity/evidence drawer, or as a separate task surface?

## Visual language and status semantics

The backend distinguishes many facts that should map into a smaller consistent vocabulary:

- requested;
- prepared/bound;
- launch accepted;
- active/observed;
- semantically completed;
- evidence ready;
- under review;
- accepted/returned;
- integrated;
- settled;
- waiting for dependency;
- attention required.

The design task is to group these without implying unsupported facts. For example, “Agent finished” should not stand in for evidence capture, review acceptance or integration.

## Design-system ownership issues

- reusable layout lives inside feature folders;
- global Markdown components depend on Agent Session wrappers;
- legacy dashboard CSS remains globally shipped;
- recorded and productive surfaces share models whose labels still call them disposable/fixture-driven;
- the Agent Session verification harness is emitted as a build entry even though it has no normal navigation.

These issues complicate visual consistency and make it harder to know which component variants are supported product primitives.

## Useful future visualizations

- a journey map with user-visible steps above hidden agents/evidence below;
- an “evidence ladder” component prototype for Work Units;
- a role/session constellation showing which conversations belong to one Epic/Sprint/Work Unit;
- a surface/reachability map for product, contextual, settings, debug and recorded experiences;
- a component ownership map exposing reusable primitives and feature cross-imports;
- a status-language matrix mapping durable backend facts to concise user language and diagnostic detail.

## Questions to carry forward

- Which backend facts are necessary for trust, and which belong only in diagnostics?
- What is the correct default level of workflow detail for a product owner versus an expert operator?
- How should origin/return navigation preserve context across Epic, Sprint, Work Unit, Session and File Review?
- Should internal review tooling influence the product’s review language and patterns, or remain visually distinct?
- Which recorded UI states represent intended design direction versus historical experiments?
