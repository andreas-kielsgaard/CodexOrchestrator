# Cross-cutting system findings

This is the detailed explanatory layer behind the [current-state overview](README.md), not a final architecture or keep/tune/prune classification. Each structure below appeared in more than one behavior-led pass at the inspected snapshots.

## Capability is a footprint, not a yes/no label

Source presence, compilation, command registration, release composition, navigation, successful effect and accepted product outcome repeatedly disagree:

- legacy Task commands are release-registered but deliberately fail behind a quarantine guard;
- the Agent Session scenario Harness uses real presentation and is emitted by the production frontend build, but normal product navigation cannot reach it;
- Sprint auto-flow and Document actions are visible while release composition supplies unsupported mutation controllers;
- File Review rendering and stored-artifact loading are productive while contextual production depends on build-specific composition.

A later overview should show the dominant product status first, then reveal this footprint on demand. A single exhaustive certainty score would hide more than it clarifies.

## One Agent Session spine carries several products

Ordinary conversation, Plan Builder, bootstrap, Sprint roles, Handler and Implementer behavior share one `AgentSessionApplication`, Codex runtime, repository and notifier path. Their different semantics come from surrounding durable facts and launch envelopes:

- application provenance and fixed identities;
- immutable Harness revisions;
- role-specific prompt, sandbox and working-directory policy;
- invocation-scoped MCP endpoints;
- execution-support and Git authority;
- at `9240364`, a product-wide selected Native Profile identity gate.

The clearest future picture is therefore a common runtime spine with role envelopes, not a diagram of unrelated agent engines.

## Productive operations are assembled contracts

No single artifact defines an effective managed-agent operation. The Implementer reporting continuation is assembled from durable attempt identity, an immutable Harness revision, code-authored prompt material, an invocation-scoped MCP server, selected Native Profile continuity, runtime option resolution, inherited environment, Codex argument construction, child-process policy and later semantic/lifecycle receipts.

The Git-authority traversal has the same shape at a larger scale: one productive Work Unit crosses SQLite authority, compile-time source identity, Git objects and refs, app-data worktrees, process launches, evidence capture and frontend projection. A command, endpoint or configuration file names only one portion of the operation.

A later overview should therefore make representative operations the legible spine and attach component, transport, configuration and evidence detail as overlays. The exact artifact catalogue remains necessary drill-down, but should not be the primary explanation.

## Configuration often executes authority

The repository calls many things configuration that directly change runtime behavior: Harness prompt/skill/model/sandbox material, MCP arguments and bearer environment, selected Native Profile identity, worktree compatibility contracts, Vite build entries and generated Tauri launch configuration. The application source checkout captured at compile time goes further: its cleanliness and `HEAD^1`/`HEAD` facts decide whether Sprint authority can exist, and its branch can later become the integration target. These are not merely settings metadata.

Future views should place configuration on the operation it governs. A separate “configuration inventory” is still useful, but insufficient to explain behavior.

## Durable convergence is stronger than presentation convergence

Runtime facts are persisted before notifications. Backend transition observers can advance orchestration before the frontend event is emitted. Startup reconciliation can settle stale invocations, create Sessions and worktrees, open MCP servers, launch Codex and progress orchestration before the frontend mounts. The frontend then reloads Agent Session state but does not generally refresh the surrounding orchestration snapshot. Native query composition also joins separate reads without a shared snapshot token.

The review surfaces show a related presentation problem in the other direction: progress polling can replace a comparison source and repeatedly reset a user’s local viewing state. In both cases the database, application process and screen can be truthful at different moments.

A useful presentation pattern is a short lifecycle strip separating durable fact, application reaction and visible projection. It should not turn every screen into a warning about temporal uncertainty.

## Recovery is productive behavior, not passive loading

Application startup invokes the ordinary bootstrap and Sprint reconcilers after Agent Session recovery. Those paths can perform much of the same external work as live callbacks. Native Profile recovery follows a different rule and may not classify pending attempts until Technical Settings queries them. Exit explicitly drains managed MCP registries and the shared Agent runtime, while Native Profile and debug-review cleanup use different best-effort or operating-system ownership paths.

This makes startup, query-time recovery and exit part of the product's operating model rather than incidental Tauri glue. A future lifecycle view can show automatic recovery as one calm band, with external effects and user-visible catch-up disclosed beneath it.

## Most gaps occur at handoffs, not inside the dense implementations

Repeated examples include:

- backend-created Handler/Implementer Session identities are not projected into productive embedded Session references;
- the native MCP backend distinguishes request creation from reconciliation while the visible action reaches only reconciliation;
- selected Native Profile identity now reaches the shared runtime, but its execution-mode and danger policy do not;
- Artifact Access has a typed and tested controller, but no productive native adapter;
- direct worktree comparison emits provenance that the frontend contract discards.

This shifts later architecture analysis toward connectors, contract ownership and end-to-end outcomes. Module size alone will not identify the most important seams.

## Retained non-product code has several different jobs

The observed residue is not one “test leftovers” bucket:

- quarantined compatibility implementation;
- deterministic scenario and recorded-state verification;
- debug/operator tooling with real native effects;
- build-included but product-unreachable secondary entry points;
- unmounted predecessor UI;
- parallel operational CLI implementations;
- sibling-branch design and integration evidence.

Later disposition work should name the role before judging whether an artifact is waste. The legible default can still group these as supporting or retained surfaces, with the exact retention role behind disclosure.

## Names frequently understate or overstate scope

`ManagedPlanBuilderNotifier` now observes every Agent Session. `prepare_managed_agent_session_launch` is installed on the generic application. A registered Tauri command may only preserve an explicit failure for old callers. A development-folder Harness may be part of production build output.

Architecture and presentation should use observed reach and authority as the primary label. Current symbol or folder names remain valuable evidence, but not the final explanation.

## “Current product” is a lineage question

The inspected operational line, Product Decisions line, final-settlement line and uncommitted worktrees contain different late-stage capabilities. The dedicated research branch now names the newest clean operational descendant, but it is not local `main` and does not absorb the sibling lines.

A future overview should show one calm primary storyline with branch-local additions attached where they change the answer. It should avoid both a false single-tree picture and an unreadable graph of every branch.

## A dirty worktree is not automatically unfinished future work

The moving-state passes produced four different historical meanings:

- the large Work Slice Planner transition diff is an early precursor whose direction was committed, corrected and extended hundreds of commits later;
- the runtime-toolchain diff is a later experiment on a stale base, partly parallel to already committed acceleration tooling;
- two presentation worktrees are sibling visual/proof alternatives that never entered their descendants;
- the dirty `main` Harness relocation mixes a precursor destination with a second unwired destination and cannot satisfy its own launch precondition.

Working-state evidence therefore needs ancestry and behavioral comparison before it is called new, lost or pending. A later history view can summarize these roles with a small badge—precursor, parallel, alternative or unresolved—while keeping the exact diff in the evidence layer.

## Code and artifact volume can mislead

The 2,086-line raw Work Slice Planner diff reduces to two materially changed files after formatting normalization. The runtime-toolchain experiment's 400 deletions are mostly a dev-dependency lock contraction. A reviewed presentation retains nearly five megabytes of generated proof residue around a fixture-only surface.

File and line counts remain useful navigation signals, but not product-weight or cleanup-value measures. Later hotspot and disposition views should combine responsibility, reachability and behavioral delta before showing size.

## Presentation consequence

The findings currently favor a layered explanatory surface:

1. a small number of product journeys and shared platforms;
2. capability footprints showing how far each journey is connected;
3. authority/configuration overlays on the consequential steps;
4. lineage and exact artifact detail only when requested.

Uncertainty should appear where it changes interpretation—for example, branch-local or uncommitted behavior—not as equal visual weight on every fact. The underlying repository should remain more exact than the eventual default presentation.
