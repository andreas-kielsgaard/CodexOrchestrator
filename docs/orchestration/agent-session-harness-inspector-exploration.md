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
- **Prompt prefix** has two modes over one stored value. Rendered mode uses the accepted File Review
  `AgentMarkdown` -> `MarkdownContent` path and does not edit. Plain mode is a normal raw-Markdown
  textarea and is the only editing surface. Switching modes does not parse, serialize, or normalize
  the source value; the textarea retains its selection when returning from Rendered mode. GFM is
  enabled, raw HTML is skipped, and links open with safe external-link attributes. The prefix is
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
  optional defaults constrained to an allowed model and range. The fixed-width discrete controls
  show only their selected endpoints. A revision either owns that policy atomically or explicitly
  delegates to one shared per-revision policy. Revision-owned controls are editable only in Harness
  edit mode; a delegated shared policy can be adjusted in view mode and is shared by recorded
  Sessions using that revision. The current Session model/effort choice is a separate record shown
  only in the Agent Session view, where its options stay within the applied revision/shared-policy
  constraints. The user's global last-used preference remains a separate recorded record.
  Invalidating a Harness default uses a provisional valid fallback and remembers the prior choice
  only until it becomes valid again or the user makes a manual default/model choice, finishes
  editing, commits, navigates away, or crosses another ownership boundary. Automatic recorded
  working-copy and delegated-policy cache updates are not acceptance boundaries because clearing
  on every change would prevent untouched exclusion/reinclusion restoration. No application
  capability catalog exists, so the UI labels its two-model source as a recorded catalog rather
  than complete runtime discovery.
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

### Markdown implementation and bundle impact

The accepted File Review renderer is the shared implementation: `AgentMarkdown` delegates to the
product-level `MarkdownContent` wrapper around `react-markdown` and `remark-gfm`. Harness Management
does not keep a second parser or a rich-editor command layer. The previous `@mdxeditor/editor` and
Lexical dependency tree, deferred JavaScript chunk, and editor stylesheet are removed. The
remaining Markdown dependencies already serve Agent Session transcripts and the File Review
surface. The validated build emits the Agent Session JavaScript chunk at 427.88 kB (129.01 kB gzip)
and its stylesheet at 37.48 kB (6.97 kB gzip); it no longer emits a separate editor chunk or Vite's
greater-than-500 kB chunk warning. `npm audit --omit=dev` reports no production vulnerabilities.
The full development graph reports five high-severity advisories in the
ESLint/minimatch/brace-expansion toolchain; the suggested automated fix would require a breaking
ESLint upgrade and was not applied.

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
- `harness_revision_delegated_model_policies`: optional shared model/range policy keyed by Harness
  revision, with policy revision, local adjustment state, editor, and update time. This record
  exists only when the immutable Harness revision delegates instead of fixing the policy.
- `agent_session_harness_bindings`: one row per session with Harness key, applied revision, optional
  desired revision, update strategy and state, stable agent name, applied visual token and accent,
  name-pool provenance, assignment revision/time, and optimistic binding revision.
- `agent_session_model_overrides`: an optional model/effort choice per Session, stored independently
  from both the Harness revision and its delegated shared policy. Agent Session controls may select
  any model/effort within the effective allowed constraints but cannot widen them.
- `user_model_preferences`: the application-owned durable register of each user's globally
  last-used model/reasoning choice and update time. A human-created Session's **Caller choice**
  consults this register; the recorded adapter demonstrates the shape but does not implement
  production durability.
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

Model choice resolution is also application-owned and deterministic:

1. The applied Harness revision supplies immutable constraints when **Version specific** is on;
   otherwise its separately stored delegated shared policy supplies the constraints for every
   Session using that revision.
2. An explicit current-Session model/effort choice may select within those applicable constraints.
   It cannot enable a model or effort outside the effective Harness policy.
3. An explicit valid Session default wins, followed by an explicit valid Harness
   revision/delegated-policy default.
4. **Caller choice** uses the durable user last-used preference when it is within the effective
   allowed model/range.
5. Otherwise the application chooses a provisional valid fallback and records the owning source in
   the invocation resolution.

UI fallback memory is transient interaction state only. It never overwrites any of the four owned
records. `save_working_copy` and `save_delegated_model_policy` are automatic recorded cache writes,
not an explicit save action and not a fallback-memory boundary. Fallback memory is discarded by
manual default/model choice, **Finish editing**, commit, navigation, or another ownership boundary.
The prototype defines **Finish editing** as its explicit end-of-edit boundary and makes no claim
about a separate persistence-backed Save operation.

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
  default-range validation, separately owned delegated shared policies/Session overrides/user
  preference, Session-only identity changes, and ensure free-form completion criteria remain
  proposed rather than connected or exposed.
- Pane tests cover Session-version entry, state cues, cached edits and dirty state across remount,
  rendered/Plain source round trips, permitted-name inspection/editing without Session renaming, full
  selected-skill details and applicability changes, fuzzy catalog search/add/change/remove,
  consistent skill/tool labels, policy counts and collapse state, compact accessible range
  controls, revision-owned versus delegated view/edit boundaries, Agent Session-only model/effort
  controls, shared rendered-Markdown normalization, provisional default restore/discard behavior,
  confirmation-gated commit/Push/single/bulk next-prompt changes, and truthful proposed hook
  presentation.
- Product-source tests ensure completion criteria remain not connected without a typed hook
  registry result.
- App and planning tests cover the same source and identity presentation across Harness Management,
  Epic planning, and independent Agent Sessions, including persisted Agent names on transcript
  responses and the agent-authored proposal heading.
- Product-source tests preserve the current read-only, untracked production boundary.
- The aggregate passes 93 files and 643 tests with two workers. TypeScript/Vite build, ESLint,
  touched-file Prettier, Rust formatting, and diff checks pass; the repository-wide Prettier
  baseline still contains unrelated pre-existing findings.
- In-app browser evidence exercises 1440 x 1000, 1014 x 794, 870 x 794, 791 x 794, 390 x 844,
  1014 x 794, and 1440 x 1000 in sequence after opening the Session through Agent Sessions,
  including application tab movement. At every stop the complete title has equal client and scroll
  dimensions (233 x 19), the identity marker remains visible, and Manage harness then Copy entire
  session remain disjoint. The Harness Management view and edit toolbars pass the same resize
  sequence without item overlap, clipped action labels, or document-level horizontal overflow. At
  1014 x 794 the Agent-name input and Available-names search have matching 37 px visible control
  bounds. Every reasoning control retains a 267 px width with a fixed 112 px endpoint readout,
  delegated v3 controls are enabled in view mode, and revision-owned v4 controls are disabled until
  edit mode. Session model/effort controls appear only in the Agent Session view, and the Prompt
  prefix view has the shared `agent-markdown` renderer class. A focused Prompt-prefix pass at the
  same responsive widths verifies both Rendered and Plain modes without horizontal overflow,
  toolbar or `contenteditable` remnants. A GFM task item and safe external link render through
  `AgentMarkdown`, raw HTML is skipped, the exact 113-character source and end caret survive the
  Rendered/Plain round trip, the dirty draft survives conversation navigation, and Commit remains
  confirmation-gated. The browser console has no warnings or errors.

This evidence exercises only the recorded development composition and does not establish production
runtime behavior.
