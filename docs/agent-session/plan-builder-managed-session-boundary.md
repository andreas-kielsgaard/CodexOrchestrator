# Plan Builder managed Agent Session boundary

`managedPlanBuilderSessionConfiguration` declares the Plan Builder identity, title derivation, role,
purpose, durable pre-initiation proposal support, and exactly one MCP tool. It captures the current
Epic name for the first Agent Session title and falls back to `Epic builder session`; it does not
rename later sessions or claim that a declarative frontend field activates a skill.

The Plan Builder uses the narrow `send_managed_plan_builder_message` command: first send
creates/acknowledges an Agent Session and later sends resume the acknowledged session ID. The
orchestration service sets the Codex child working directory to the repository root after validating
`product/skills/epic-plan-builder/SKILL.md` metadata. Ordinary Agent Session launches keep
their own working-directory behavior.

The Epic Plan Builder product context owns a versioned Conversation Harness configuration. In this implementation, a harness does one thing: it supplies a lightweight application-provenance prefix immediately before the first user query. Generic Agent Session code accepts only a neutral optional initial-prefix value and knows no Plan Builder role or harness schema. The user's submitted text is persisted unchanged, and durable invocation history prevents prefix reinjection after restart.

The product-owned catalog is `src-tauri/src/orchestration/conversation_harness_catalog.json`; its
adapter validates the schema and fails when the requested configuration is missing or invalid.
Skill guidance records canonical names, repository-relative sources, purposes, and activation stages
without embedding `SKILL.md` or claiming that guidance loads a skill. Repository-local metadata and
`codex debug prompt-input` can deterministically prove the catalog input. Only an authorized live
managed child can prove model discovery or selection. Discussion may explore goals, scope,
ambiguity, risks, and alternatives without structured state. `run-overall-plan` applies
when an ad-hoc Codex conversation owns planning outside the application. The Harness-selected `epic-plan-builder`
applies to product proposal construction or revision. Instantiation and root startup remain later,
unavailable product stages.

MCP exposure and authorization, durable draft/session association, and invocation lifecycle stay in
their existing orchestration services; they are not Conversation Harness behavior.
`submit_epic_plan_proposal` accepts only
`{ suggestedEpicName?, sprints: [{ title, intendedMovement, concernSummaries }] }`.
`concernSummaries` is required for every Sprint and may be `[]`. At managed invocation start, the
application captures the authorized current revision and derives replay protection from the Agent
Invocation plus canonical payload. The server independently rechecks authorization and the captured
precondition, so callers cannot select another draft or overwrite a concurrent proposal. Only a
validated semantic submission changes the structured proposal; prose does not.

Deferred: Agent Session UI should allow inspection and potential editing of configuration supplied by its product context. It must distinguish the currently configured prefix from the immutable prefix already delivered to an existing session. This Sprint does not implement an editor, runtime mutation, or durable user customization.

The G3 transcript is an observation from before this correction, not proof of live success: it shows
one logical skill read as two lifecycle rows, then two logical MCP operations as four lifecycle rows
(`get_epic_planning_context` and `save_epic_plan_proposal`). A later separately authorized live
rerun is required after deterministic correction checks; no model invocation is part of this
correction evidence.
