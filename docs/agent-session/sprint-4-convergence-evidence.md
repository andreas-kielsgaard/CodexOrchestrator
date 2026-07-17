# Sprint 4 convergence evidence - Epic Plan Builder foundation

Status: accepted and closed after final Epic review on 2026-07-15.

## Accepted UX and boundaries

- Orchestration overview opens **Plan an Epic** in the normal application shell.
- Opening the surface is local only: it creates no Agent Session, planning draft, association, or title record. The first successful managed discussion send creates the Agent Session; its acknowledged provider-neutral ID is then reconciled idempotently into the durable planning-draft binding.
- The shared `AgentSessionWorkspace` / `ConversationViewport` is the primary surface. An editable, ephemeral Epic name and a scrollable, collapsible proposed-Sprint hierarchy sit beside it.
- The proposal is read only from injected `EpicPlanProposalSource`; it is either neutral available content or an unavailable state. Agent Session transcript prose cannot become plan state. Product composition injects the unavailable source. Recorded content and its mutable adapter stay in the development composition, selected only by the `recorded-plan-builder` development opt-in.
- The managed configuration declares `epic_plan_builder` / `epic_planning` and lists skills, folder routing, MCP tools, and durable orchestration as unsupported. It authorizes no runtime behavior.
- First send goes through the existing provider-neutral `AgentSessionClient` boundary with title `Epic builder session for <Epic name>` (or `Epic builder session` when blank). After the client acknowledgement supplies `sessionId`, subsequent sends use that id and do not rename the session.

## Deterministic evidence

`App.orchestrations.test.tsx` exercises the user order with the injected recorded Agent Session client: overview -> Plan an Epic -> first send -> acknowledged-session resume -> source-owned proposal update -> overview. It asserts the exact first/resume commands, unchanged first title after a later name edit, and that the proposal does not change from conversation prose.

Focused validation: 5 files / 33 tests passed.

- `npx vitest run src/app/App.orchestrations.test.tsx src/application/orchestrations/managedPlanBuilderSession.test.ts src/bootstrap/productApplicationComposition.test.ts src/features/agentSessions/useAgentSessionController.test.tsx src/application/agentSessions/contracts.test.ts --maxWorkers=1`
- Full frontend suite: `npm test -- --maxWorkers=1` - 78 files / 535 tests passed. Serial execution was used proactively; no timing rerun was required.
- `npm run lint`, `npm run build`, `npm run format:check`, and `git diff --check` passed.

Static import/boundary scans found recorded proposal provenance only under `src/dev/orchestrationSection`; product startup uses `unavailableEpicPlanProposalSource`. The Plan Builder files contain no task/run, event, material, initiation, execution, or upload path. Rust was not changed for Sprint 4 Plan Builder work and was not rerun for this frontend-only convergence pass.

## Manual live gate - not run

Do not send a paid/live Codex prompt in this Sprint. A later authorized manual gate must run the actual application with an approved provider/runtime, enter Plan an Epic, send one first message, verify the provider acknowledgement and returned `sessionId`, send one resume message, and verify that the session title remains the first-send title and that no transcript text is treated as authoritative Sprint or Work Unit state.

## Explicitly unsupported / deferred

No live-provider proof, MCP/tools, capability-profile assembly, folder routing, durable Epic/Sprint/Work Unit identity, durable plan persistence, material generation, initiation, execution, automatic continuation, or file upload is implemented or evidenced. Recorded proposal content is development-only and is not product truth.

## Handoff

Sprint 4 is closed as the bounded Epic Plan Builder foundation. Final Epic review independently
confirmed the neutral proposal boundary, truthful intake guidance, production/dev import separation,
and validation above. The working tree was intentionally dirty before this convergence pass; no
pre-existing changes were reset, cleaned, staged, committed, or otherwise altered. No later Sprint
work started. The next eligible work requires a separately authorized Sprint with an explicit
contract and proof plan rather than an extension of this presentation surface.
