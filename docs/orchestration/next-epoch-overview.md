# Input overview: Orchestration and Epoch state contracts

> Historical bounded input: this resolved pre-Sprint-2 brief intentionally retains its original
> vocabulary. It is not current authority or an implementation plan; use `terminology.md` and
> `epic-sprint-state-contracts-final-handoff.md`. Retain it while the contract handoff needs source
> provenance, and remove it only in an authorized archive-retirement pass.

Status: historical bounded input, now resolved by the accepted WU-OESC1 through WU-OESC5 outcomes
and the final handoff. This document never authorized launch or implementation.

Translate the accepted recorded views into provider-neutral application read models and explicit
control contracts. Begin from the UI requirements consolidated in
`epoch-control-surface-discovery-final-handoff.md`; do not promote the discovery fixture or its
TypeScript shapes into durable schema by convenience.

Resolve only the contract questions needed by the accepted Orchestration -> Epoch -> Epoch Plan ->
Epoch Plan Revision -> Work Unit hierarchy:

- durable identity, revision, attempt, review, gate, decision, document, and provenance facts versus
  derived presentation;
- distinct Plan, Plan Revision, Planner Episode, and Agent Session Reference entities, connected by
  explicit relations even when a scenario currently maps them one to one;
- read models that link Concerns, Documents, and Agent Sessions without leaking provider or storage
  shapes;
- Agent Control commands with explicit authority, prompt provenance, requested-versus-event outcomes,
  and idempotency requirements;
- separate eligibility and execution contracts for Orchestration-level and Epoch-level automatic
  continuation;
- safe artifact references for later path resolution and system-default opening;
- current requirements that need focused new ports rather than legacy task/run reuse.

As these contracts are resolved, decompose the current discovery module along its existing change
boundaries: contracts, decoding and validation, derived state, relationship projection, and final
read-model assembly. Preserve one public application entry point without retaining one source-file
monolith.

Do not implement production clients/controllers, persistence, prompt delivery, artifact opening,
automatic continuation, transition execution, or legacy extraction in this contract-definition
Epoch unless a separately accepted plan explicitly assigns that work.
