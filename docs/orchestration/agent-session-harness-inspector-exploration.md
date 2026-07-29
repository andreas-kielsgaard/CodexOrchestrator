# Session-owned Harness Management architecture

Status: recorded product prototype and future production boundary on
`codex/explore-harness-inspector`.

## Decision

An Agent Session owns one durable relationship to one Harness identity and one applied Harness
revision. A product application service resolves that relationship by `session_id`. Epic planning,
Agent Sessions, Harness Management, and invocation adapters consume the same read model; no view
chooses a Harness, assembles individual settings, or infers a revision.

A Harness revision is an immutable, atomic effective configuration. It includes the prompt prefix,
skill policies, tool policies, allowed models and reasoning levels, sandbox and authority,
Application hook references, update policy, permitted agent-name pool, and visual identity.
Individual elements cannot advance independently.

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

- The Harness name is the user-facing role. The machine key remains an administrative property;
  there is no second role field.
- The toolbar selects the Session-applied version, any historical committed version, or the
  working draft. Session version, current pushed version, working/uncommitted draft, and viewed
  revision history are separate facts. A historically pushed revision remains labeled
  **Previously pushed** when a newer revision becomes current; a never-pushed revision is labeled
  **Committed**.
- **Edit harness** creates a cached draft from the viewed committed revision; **Edit draft** reopens
  it. Commit and Push appear only in edit mode. Commit, Push, single-Session changes, and bulk
  changes require plain-language confirmation.
- **Prompt prefix** has two editable views over one stored value. Markdown is a formatted
  content-editable surface with a formatting toolbar; Plain is raw Markdown without a toolbar. The
  rich surface is not reconstructed after its own keystrokes, preserving its caret. The prefix is
  intended for the first Session prompt and future Harness-aware context compression; that
  compression routine is deferred.
- Skills are summarized as **Always applicable**, **Initial ingestion only**, and **Available**.
  The first two are selected whitelists. Only Available discovery can use whitelist or blacklist
  policy. A fuzzy-search dialog reads the complete checked-in product skill catalog and owns
  selection, timing, and removal. Clicking a selected skill opens its purpose, use condition, path,
  complete checked-in `SKILL.md` text, and applicability control. Skill groups remain deferred.
- Tools use the same **Always applicable**, **Initial ingestion only**, and **Available** labels for
  exposure timing. A fuzzy-search dialog reads the recorded Epic Plan Builder tool catalog. These
  labels do not turn schemas into prompt text: provider-owned schemas remain runtime-owned and
  runtime reconfiguration remains unimplemented.
- Each recorded model has an allowed flag and an accessible minimum/maximum reasoning range, with
  optional defaults constrained to an allowed model and range. No application capability catalog
  exists, so the UI labels its two-model source as a recorded catalog rather than complete runtime
  discovery.
- Current sandbox and authority values are inspectable and editable in the recorded working copy.
  Expanded sandbox customization is deferred.
- The permitted-name summary opens the Harness-specific subset. Editing changes only the recorded
  Harness draft; the Session-owned assigned name remains stable.
- Application hook references remain visible and are labeled **Proposed**. They are not presented
  as connected or exposed. Initial prompt delivery is not modeled as a hook.
- The version table shows status, active Session count, the selected Session indicator, and bulk
  next-prompt actions. Push moves the recorded local active revision and queues relevant recorded
  Sessions only for their next prompt. The separate Harness Management update panel and all
  interrupt controls are removed.

The product-backed source remains read-only. It adapts the existing managed Plan Builder query to
the same view contract, reports the session binding as untracked, supplies no agent identity, and
does not expose edit commands. Free-form `completionCriteria` entries are labeled **Not connected**
because no typed registry result confirms them. It therefore does not claim Git history, durable
assignment, activation, update delivery, interruption, or compression.

## Production repository and commands

The future product-owned local Harness store is a local Git repository in product data. It is not a
remote collaboration repository. Canonical Harness configuration files are committed as immutable
revisions. A product-owned draft cache retains the latest valid or partially edited working copy
before commit, with an optimistic draft revision and dirty flag.

The command meanings are:

1. `SaveHarnessWorkingCopy` validates command shape, records the whole working copy in the local
   draft cache, and advances its optimistic revision. The product editor may coalesce keystrokes,
   but the repository persists whole atomic configurations.
2. `CommitHarnessRevision` validates the complete effective configuration, writes one Git commit,
   records its object id and configuration digest, and clears the matching dirty draft. It does not
   activate the revision.
3. `ActivateHarnessRevision` is presented as **Push**. One application transaction advances the
   local product ref `refs/orchestrator/harnesses/<harness-key>/active` and sets that revision as the
   desired next-prompt revision for every relevant existing Session. It performs no network
   operation. New Sessions resolve the active ref.
4. `RequestSessionHarnessUpdate` sets one Session's desired committed revision for next-prompt
   consumption. `RequestHarnessUpdateForRelevantSessions` performs the same checked transition for
   each relevant Session. Components never mutate applied or desired versions directly.

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

**Interrupt and update now** belongs in the Agent Session view only when that Session has a queued
revision and is still executing an invocation resolved with the previous revision. It requires a
supported runtime/Application path that identifies and stops the running invocation before
restarting or continuing with the desired revision. Changing a database row cannot interrupt a
process. The recorded fixture contains a completed invocation, so it exposes no interrupt action.
This path is not implemented and must remain unavailable in the connected product until the
runtime returns an observed interruption outcome.

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

### Application hook registry conclusion

The current repository has no application-owned typed hook registry that guarantees every
connectable hook is discoverable by Harness configuration. The compiled Harness profile supplies
free-form `lifecycle.completionCriteria` strings. The recorded prototype maps those strings as
**Proposed** references, while the connected product inspection maps them as **Not connected**.
Neither state is implementation wiring, availability discovery, exposure, or validation.

Production should define one typed Application hook catalog as the source for:

- implementation registration and routing;
- Harness catalog discovery and exposure choices;
- availability/capability reporting; and
- configuration and invocation-time validation.

A hook implementation must not be connectable unless registered there, and Harness configuration
must reference registry identifiers rather than unrelated strings. The Application composition
owns enforcement. Deeper hook configuration UI remains deferred.

## Prototype and test evidence

- Identity tests cover the 100-name pool, curated-pool validation, distinct Harness visual
  identities, deterministic assignment, uniqueness, exhausted-pool fallback, and invalid pools.
- Recorded-source tests cover committed version history, the complete checked-in skill catalog,
  recorded tool/model catalogs, full atomic configuration, stable identity across draft pool
  changes, historical-push status, local draft/commit/Push transitions, next-prompt queues,
  default-range validation, and ensure free-form completion criteria remain proposed rather than
  connected or exposed.
- Pane tests cover Session-version entry, state cues, cached edits and dirty state across remount,
  Markdown/Plain editing, permitted-name inspection/editing without Session renaming, full
  selected-skill details and applicability changes, fuzzy catalog search/add/change/remove,
  consistent skill/tool labels, policy counts and collapse state, accessible range controls,
  confirmation-gated commit/Push/single/bulk next-prompt changes, and truthful proposed hook
  presentation.
- Product-source tests ensure completion criteria remain not connected without a typed hook
  registry result.
- App and planning tests cover the same source and identity presentation across Harness Management,
  Epic planning, and independent Agent Sessions, including persisted Agent names on transcript
  responses and the agent-authored proposal heading.
- Product-source tests preserve the current read-only, untracked production boundary.
- The aggregate passes 93 files and 631 tests with four workers. TypeScript/Vite build, ESLint,
  touched-file Prettier, Rust formatting, and diff checks pass; the repository-wide Prettier
  baseline still contains unrelated pre-existing findings.
- In-app browser evidence at the reviewed 760 x 900 width, 1440 x 1000, and 390 x 844 verifies the
  Session entry, desktop and narrow management layouts, real Markdown editing and Plain mode, fuzzy
  skill search, version defaults and cues, the name-pool and selected-skill dialogs, accessible
  range controls, preserved Manage-then-Copy order, no entry-button overlap, and no document-level
  horizontal overflow. The browser console has no warnings or errors.

This evidence exercises only the recorded development composition and does not establish production
runtime behavior.
