# Agent Session options correction pass

Status: proposed implementation shape, 2026-09-23. Planning only.

## Target and boundaries

Make direct-user Agent Session controls immediate and concrete. The model and reasoning controls always display an effective value, the branch control resolves the conversation context's default branch, and `/` menus retain their results while the query changes. Capability Profiles continue to choose the execution route and the MCP/skill groups compiled into a session, but do not restrict the model or reasoning choices presented to a manually addressed session.

This pass does not add broad workflow enforcement, alter automated-workflow policy, invent remote discovery, or persist a new copy of every skill catalogue. It reuses the existing durable model catalogue and adds bounded context caching for the live facts that are expensive or working-directory-specific.

## Proposed ownership

### 1. One manual-session capability catalogue

Replace the current split between `RuntimeProfileSnapshot`, `selectedTargetQuickFeatures`, toolbar arrays, and a later slash-menu discovery call with one `ManualSessionCapabilityCatalogue` projection owned by Agent Sessions. It should contain:

- the selected device and Codex configuration reference;
- the working-directory context, when known;
- ordered models with their model-specific reasoning modes;
- the provider's concrete default model and each model's concrete default reasoning mode;
- available quick actions and skills, plus observation limitations.

Create the pure effective-choice resolver beside this projection, for example in `src/features/agentSessions/effectiveSessionOptions.ts`. Resolve model in this order: a still-available per-message override, the provider/profile default, then the first available model. Resolve reasoning against that model: a still-supported override, the configured default when supported, the model's default, then its first supported reasoning level. Return concrete values for presentation while retaining nullable override provenance internally, so a route change can still adopt its new defaults.

Adapt `selectedTargetQuickFeatures.ts` or replace it with this projection. Delete its Capability Profile model/reasoning intersections and its lossy reconstruction of model metadata. `AgentSessionScreen.tsx`, `SessionComposerToolbar.tsx`, and `composerQuickActions.ts` should consume the same resolved catalogue rather than independently deriving options.

The toolbar selects use the effective model and reasoning IDs as their values. Remove blank `Model default`, `Reasoning default`, and `Use Session default` options. The inherited concrete item is simply marked selected. Selecting another item creates an override; selecting the inherited item may collapse back to `null` internally without exposing that implementation detail.

### 2. Context cache and stale-while-refresh menu behavior

Give native quick-feature discovery a stable cache key made from configuration reference, normalized cwd/folder context, and the pinned session identity where applicable. Put request coalescing and the session-lifetime cache at the execution-configuration/Agent Session application boundary, rather than inside the visual menu. Concurrent requests for the same key share one discovery operation.

Use the already-persisted `execution_model_catalogues` result as the immediate model source. Prefetch the combined catalogue when the selected device, Codex profile, folder, or worktree context changes; opening `/` must not initiate an otherwise avoidable full model probe. Keep cwd-sensitive skills in the bounded in-memory context cache and refresh them on context change or an explicit refresh. Do not add a new durable skill-cache table in this pass.

Split normal and forced skill discovery. Agent Session reads must not call `skills/list` with `forceReload: true`; reserve forced reload for the explicit Technical Settings refresh action. Avoid re-reading configuration, requirements, every model page, and skills merely because the user reopened `/`.

In `useAgentSession.ts`, make the catalogue loader stable for its cache key instead of depending on newly allocated `executionQuickFeatures` objects. In `useComposerQuickMenu.ts`, retain the current catalogue while a refresh is running; remove unconditional `setCatalog(null)` from the load path. Query changes only filter cached actions. A changed context may display its cached value immediately or a first-load state, but results from the previous context must never be relabeled as current. Superseded frontend work remains ignored, and backend coalescing prevents abandoned Codex app-server probes from accumulating.

### 3. Concrete branch selection for new drafts

Move initial branch resolution out of `SessionTargetDialog.tsx` into a reusable draft-target resolver consumed by both `useSessionTarget.ts` and the dialog. Resolve a repository/workflow draft to its owning repository, use the branch graph's `referenceTarget` as the default branch, then apply the existing worktree rules for the selected device and Capability Profile:

- one eligible matching worktree: select it automatically;
- no matching worktree: select the default branch as a create-on-send target at its published commit;
- multiple matching worktrees: keep the default branch selected and displayed, but require an explicit worktree choice before send;
- no repository context: select and display `Empty workspace`, rather than the placeholder `Select branch`.

Represent the displayed branch independently from a resolved physical worktree when necessary. This keeps the branch concrete without weakening the existing rule against silently choosing among multiple worktree instances. Cache the resolved draft target in the existing conversation-specific draft cache so tab changes and application restarts restore the same effective selection.

`SessionComposerToolbar.tsx` should receive a presentation-ready target value rather than deriving `Select branch` from `workspace.kind === "auxiliary"`. Its branch button always names the selected branch, the selected worktree, a create-on-send branch, or `Empty workspace`.

### 4. Remove duplicated policy and preserve runtime truth

Delete the manual model/reasoning filtering in `AgentSessionScreen.tsx` and the parallel filtering in `selectedTargetQuickFeatures.ts`. Replace the current manual-session validation error that rejects a model outside `CapabilityProfile.allowedCapabilities.models` with validation against the chosen route's live or last-known catalogue. Keep Capability Profile defaults as preference inputs and keep skill/MCP group compilation unchanged.

Do not expand `RuntimeProfileSnapshot` into another catalogue. It remains a runtime availability/lock snapshot. Rich models, defaults, reasoning compatibility, skills, caching metadata, and refresh state belong to the manual-session catalogue. This makes the distinction discoverable for future work and prevents the toolbar and slash menu from rebuilding incomplete versions of the same facts.

## Concrete change surface

- Adapt `src/features/agentSessions/{AgentSessionScreen,SessionComposerToolbar,useAgentSession,useComposerQuickMenu,composerQuickActions,useSessionTarget,SessionTargetDialog}.ts*`.
- Replace or narrow `selectedTargetQuickFeatures.ts`; create a focused effective-options/catalogue module rather than adding more projections to `AgentSessionScreen.tsx`.
- Adapt `src/application/agentSessions/quickFeatures.ts` and the execution-target contracts only where a concrete branch-with-unresolved-worktree presentation state is required.
- Add cache/coalescing ownership under `src-tauri/src/execution_configuration/` and consume the existing model-catalogue repository. Adapt `native_codex.rs`, `service.rs`, and quick-feature transport to distinguish cached reads from explicit refresh.
- Adapt `crates/orchid-engine/src/codex/app_server/{environment,skills}.rs` so ordinary menu discovery does not force a skill reload.
- Remove obsolete default-option rendering and duplicated Capability Profile filtering once all consumers use the shared effective catalogue.

## Verification

- With the current `Usability verification` profile still allowing only Astra, a manual draft offers Astra, Sol, Terra, Luna, and 5.5 from the selected Codex route. Automated/profile compilation behavior is unchanged.
- On first render, the toolbar displays concrete model and reasoning values. `/model` and `/reasoning` show the same lists and selected values, without any default placeholder. Changing model selects a supported concrete reasoning value.
- A repository-scoped new conversation immediately shows its default branch. One matching worktree is selected, no match becomes create-on-send, multiple matches retain the branch and require a worktree, and an unattached conversation displays `Empty workspace`.
- Opening `/`, typing `/s`, continuing to `/skills`, backing out, and reopening does not clear results or spawn another discovery for the same context. Change profile or cwd and confirm one new coalesced discovery occurs.
- Add a composition-level regression test in which `AgentSessionScreen` rerenders for every typed character; assert one loader call and continuously visible results. Existing isolated composer tests with a stable mock loader are insufficient.
- Cover cached model fallback, explicit refresh, discovery failure with retained stale results, context switching, restart-restored draft selection, and the branch ambiguity rules. Run focused frontend/Rust tests, the broad suites, build the app, and verify the flows in the native window with timings and screenshots.

## Explicit non-goals

No new workflow policy, no hard model/reasoning enforcement for direct-user sessions, no automatic choice between multiple matching worktrees, no persistent skill-manifest mirror, no remote Orchid Network work, and no changes to the per-session MCP/skill exposure contract.
