# Session-owned Harness Management architecture

Status: recorded product prototype and future production boundary on
`codex/explore-harness-inspector`.

## Decision

An Agent Session owns one durable relationship to one Harness identity and one applied Harness
revision. A product application service resolves that relationship by `session_id`. Epic planning,
Agent Sessions, Harness Management, and invocation adapters consume the same read model; no view
chooses a Harness, assembles individual settings, or infers a revision.

A Harness revision is an immutable, atomic effective configuration. It includes the prompt prefix,
skill policies, tool policies, allowed models and reasoning levels, sandbox and authority, exposed
Application hooks, update policy, permitted agent-name pool, and visual identity. Individual
elements cannot advance independently.

The session also owns its assigned agent name and the visual identity copied from its applied
Harness revision. Assignment happens once during session creation. Reopening the session through
any route returns the same values; later name-pool or visual changes do not rename or restyle an
existing session. Presentation uses `[Agent name]: [Harness role]` and the same marker in session
lists, workspaces, planning, Harness Management, and authored-result headings.

## Recorded prototype

The recorded development composition exposes **Harness Management** over an Agent Session. It uses
one application-source instance for Epic planning, standalone Agent Sessions, and the management
preview. Its in-memory repository is outside React component state, so a working edit and its dirty
state survive view navigation and component remount. They do not survive an application restart.

The prototype records these product concepts without claiming production effects:

- Harness details show its explicit name, machine key, role, working version, local active version,
  session-applied version, and session-desired version.
- **Prompt prefix** has View and Edit modes. View renders Markdown; Edit uses the reusable product
  Markdown editor. The prefix is intended for a new session's initial prompt and for future
  Harness-aware context compression. That compression routine is deferred.
- Skill policy distinguishes **Always applicable**, **Initial ingestion only**, and
  **Available for discovery** with whitelist or blacklist discovery. Skill groups are deferred.
- Tool policy treats schema exposure as a provider/runtime capability, not prompt text. Exposure is
  configured separately from optional initial or recurring human-readable guidance. Runtime tool
  reconfiguration remains unimplemented.
- Model and reasoning controls accept multiple allowed values and an inherited/default choice.
- Current sandbox and authority values are inspectable and editable in the recorded working copy.
  Expanded sandbox customization is deferred.
- Application hooks remain visible. Initial prompt delivery is not modeled as a hook.
- Commit, Push, and session-update choices operate only on the recorded repository. Labels and
  notices identify this as a preview rather than a connected product lifecycle.

The product-backed source remains read-only. It adapts the existing managed Plan Builder query to
the same view contract, reports the session binding as untracked, supplies no agent identity, and
does not expose edit commands. It therefore does not claim Git history, durable assignment,
activation, update delivery, interruption, or compression.

## Production repository and commands

The future product-owned local Harness store is a local Git repository in product data. It is not a
remote collaboration repository. Canonical Harness configuration files are committed as immutable
revisions. A product-owned draft cache retains the latest valid or partially edited working copy
before commit, with an optimistic draft revision and dirty flag.

The command meanings are:

1. `SaveHarnessWorkingCopy` validates command shape, records the whole working copy in the local
   draft cache, and advances its optimistic revision.
2. `CommitHarnessRevision` validates the complete effective configuration, writes one Git commit,
   records its object id and configuration digest, and clears the matching dirty draft. It does not
   activate the revision.
3. `ActivateHarnessRevision` is presented as **Push**. It atomically advances the local product ref
   `refs/orchestrator/harnesses/<harness-key>/active`. It performs no network operation. New sessions
   resolve this active revision.
4. `RequestSessionHarnessUpdate` sets a session's desired revision and strategy through the binding
   repository. `RequestHarnessUpdateForRelevantSessions` performs the same checked transition for
   each relevant session. Components never mutate applied or desired versions directly.

Every command carries expected draft, binding, or active-ref revisions and is authorized at the
Application boundary. A successful database write alone is not evidence that a running agent
received an update.

## Durable records

The proposed product-owned schema is intentionally revision-oriented:

- `harness_working_copies`: Harness key, complete configuration, draft revision, dirty state, editor,
  and saved time.
- `harness_revisions`: revision id, Harness key, Git commit object id, canonical configuration
  digest, complete configuration, creator, and commit time.
- `harness_active_refs`: one local active revision and optimistic revision per Harness key.
- `agent_session_harness_bindings`: one row per session with Harness key, applied revision, optional
  desired revision, update strategy and state, stable agent name, applied visual token and accent,
  name-pool provenance, assignment revision/time, and optimistic binding revision.
- `agent_invocation_harness_resolutions`: invocation id, session id, Harness key, applied revision,
  configuration digest, agent identity, resolution time, launch outcome, and update-consumption
  facts.

The `AgentSessionHarnessService` owns creation-time assignment, `resolveSessionHarness(sessionId)`,
and all binding commands. Name assignment uses a validated Harness subset when configured and the
100-name product pool otherwise. It is deterministic for a creation request, avoids names already
assigned within the chosen uniqueness scope, and uses a stable numeric suffix when the pool is
exhausted. The current Epic Plan Builder, Epic Bootstrap Generator, and Epic Runner Harnesses have
curated role-referential pools and distinct visual identities.

The read result contains the complete resolved revision plus the session-owned name, visual
identity, applied revision, desired revision, update state, and assignment provenance. Both Epic
planning and independent Agent Sessions query this result. Every invocation path must resolve the
same binding immediately before preparation, regardless of which view created or reopened the
session, and record the resolution before launch.

## Update delivery boundaries

**Wait until next prompt** is consumed at the Application send boundary. Before accepting the next
user or Application-authored prompt, that boundary resolves the desired revision, computes the
delivery transition, durably advances the applied revision, and records which revision the new
invocation used. All send entry points must converge there.

**Interrupt now** requires a supported runtime/Application path that can identify and stop the
running invocation before restarting or continuing with the desired revision. Changing a database
row cannot interrupt a process. This path is not implemented and must remain unavailable in the
connected product until the runtime returns an observed interruption outcome.

An update transition compares complete revisions. It does not re-prefix skills or tools already
present. It appends changed guidance, tells the agent which removed guidance or resources no longer
apply, and updates runtime tool exposure only where the provider adapter supports it. Tool schemas
continue to travel through the provider tool interface, never as fabricated prompt text. Full
prompt reconstruction and the Harness-aware compression routine are deferred.

## Current repository gaps

The checked-in Harness catalog is compiled into Rust with `include_str!` and exposes a profile
version. Plan Builder currently associates a session through
`planning_draft_agent_session_associations` and assigns `capability_profile_id`; that profile id is
not an applied Harness revision. The general `agent_sessions` table has no Harness key, applied or
desired revision, stable agent identity, or visual identity. The Plan Builder association records
association time, not update state.

Consequently, this branch does not implement production Git persistence, draft caching, active
refs, durable session binding, identity assignment, invocation provenance, next-prompt consumption,
interrupt delivery, apply-to-all, runtime tool changes, or context compression. Relabeling
`capability_profile_id` would not close those gaps. Product-owned validation also remains with the
future implementations that own each setting; the management view has no Validation or provenance
section.

## Prototype and test evidence

- Identity tests cover the 100-name pool, curated-pool validation, distinct Harness visual
  identities, deterministic assignment, uniqueness, exhausted-pool fallback, and invalid pools.
- Recorded-source tests cover full atomic configuration, stable identity across working-copy pool
  changes, local draft/commit/activation transitions, and per-session update recording.
- Pane tests cover the return path, cached edits and dirty state across remount, and distinct
  commit/Push/session-update transitions.
- App and planning tests cover the same source and identity presentation across Harness Management,
  Epic planning, and independent Agent Sessions, including the agent-authored proposal heading.
- Product-source tests preserve the current read-only, untracked production boundary.
- The serial aggregate passes 93 files and 624 tests. ESLint, TypeScript and the production Vite
  build, Rust formatting, touched-file Prettier, and `git diff --check` pass. The repository-wide
  Prettier check still reports 39 pre-existing files outside this change.
- In-app browser evidence at 1440 x 1000 and 760 x 900 covers View/Edit Markdown, immediate dirty
  state, return and remount persistence, Commit then local Push then next-prompt queueing, and the
  same Agent identity and Harness state when reopened from independent Agent Sessions. The narrow
  viewport has no document-level horizontal overflow.

This evidence exercises only the recorded development composition and does not establish production
runtime behavior.
