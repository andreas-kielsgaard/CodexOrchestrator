# Observation pass: uncommitted presentation and Harness evidence

## Evidence boundary

This pass records **uncommitted, moving evidence**. None of the three working states below is treated as integrated product capability.

- Inspection date: 2026-08-07.
- The two presentation worktrees are detached at `55cdd40e1237040a651a6b01d3b1d4b2615282ae` (`Clarify unsettled integration attention evidence`, 2026-08-04).
- The main checkout is on `main` at `b86a8ac8f3e7483214b13e75b47397ca4df35074` (`refactor(skills): write guidance for inferred readers`, 2026-08-02), five commits ahead of `origin/main` at inspection time.
- `b86a8ac` is an ancestor of `55cdd40`; `55cdd40` is an ancestor of the current research snapshot, `9240364`.
- The working trees were read but not edited. No test or build command was run in them. `git diff --check` reported no whitespace errors for the inspected tracked deltas.
- Historical validation statements in `design-qa.md` are reported as retained evidence, not repeated or independently confirmed by this pass.
- The large skill-catalogue migration in the main checkout is deliberately not analyzed semantically here. Its existence and the file-location consequences for the product Harness are recorded because they affect runtime reachability.

## Snapshot inventory

| Working state | Dirty presentation/Harness material | Immediate role | Current disposition |
| --- | --- | --- | --- |
| `C:\Users\user\.codex\worktrees\demo-operational-spine-55cdd40` | modified `src/main.tsx`; three untracked files under `src/dev/demonstration/` | deterministic walkthrough using the real Work Unit detail view | standalone, uncommitted presentation alternative |
| `C:\Users\user\.codex\worktrees\human-review-integration-019fcb70\Codex Orchestrator` | modified `ApplicationRoot.tsx` and `design-qa.md`; untracked integration-review component, CSS, test, and `.codex-demo/` | detailed human-reviewed integration-settlement fixture | development-only, uncommitted presentation alternative with retained proof residue |
| `C:\Users\user\Documents\Code Projects\Codex Orchestrator` | six small tracked non-skill diffs plus untracked `product/skills/`; 104 total status entries including the excluded skill migration | move product Harness guidance between catalogue locations | mixed cumulative precursor and unfinished alternative relocation |

Neither presentation component, its route, nor `.codex-demo/` is present in the current research checkout. No commit containing either component was found on the inspected refs.

## 1. Operational Spine checkpoint demonstration

### Intended presentation

`src/dev/demonstration/OperationalSpineCheckpointDemo.tsx` presents one accepted Work Unit through five selectable checkpoints:

1. Materialized.
2. Implementation ready.
3. Review ready.
4. Handler accepted.
5. Integrated and settled.

The component does not invent a second Work Unit renderer. It constructs `PresentedWorkUnit` fixtures and passes them into the product `WorkUnitDetailWorkspace`. This makes the surface useful for reviewing the accumulated Work Unit presentation against carefully selected state combinations.

The route supports two fixture controls:

- `stage=<id>` chooses the initial checkpoint, defaulting to `materialized` for absent or unknown values;
- `focus=integration` scrolls the Work Unit context region to its bottom when the stage changes.

The fixtures progressively add real presentation DTO shapes for execution readiness, Implementer outcome, Handler review and decision, integration result, settlement, and prerequisite contribution. The component supplies no lifecycle entries and no Agent Sessions. It exercises the Work Unit projection and view, not the live runtime or the Session view.

### Proof boundary encoded in the fixture

The page labels itself `Controlled fixture` and names checkpoint `55cdd40`. Its header expressly disclaims:

- a live provider run;
- dependent activation;
- Work Slice settlement;
- publication;
- user acceptance.

The data follows the same boundary. Application readiness is populated while `providerActivityObserved` stays false. Handler review pending and Handler acceptance are separate stages. The final fixture records integration and Work Unit settlement and contributes to two dependent Work Units, while the visible copy says that this does not activate dependent work.

`OperationalSpineCheckpointDemo.test.tsx` encodes this intended story as one walkthrough. It checks the readiness language, pending semantic judgment, accepted Handler decision, integration success, Work Unit settlement, two-dependent contribution, and the non-activation statement. The test source exists; this pass did not rerun it.

### Reachability

The modified `src/main.tsx` statically imports the demonstration and selects it whenever the page query contains `demo-operational-spine`:

```text
index.html
  -> src/main.tsx
  -> ?demo-operational-spine
  -> OperationalSpineCheckpointDemo
  -> WorkUnitDetailWorkspace
```

There is no `import.meta.env.DEV` check, feature flag, or Tauri-side guard. If this dirty change were built, the demonstration would be reachable from the normal entry page in both development and production frontend modes by typing the query parameter. It replaces `ApplicationRoot` for that page load. No normal application navigation to it was found.

The data is local and deterministic. The page does not call Tauri, SQLite, Codex, an MCP endpoint, or a product application client. Its production-mode URL reachability would therefore expose a demonstration surface, not live orchestration.

### Retained artifacts

| Artifact | State | Purpose |
| --- | --- | --- |
| `src/main.tsx` | tracked modification | normal-entry query switch |
| `src/dev/demonstration/OperationalSpineCheckpointDemo.tsx` | untracked, 290 lines / 10,021 bytes | stage controls and typed fixtures |
| `src/dev/demonstration/operationalSpineCheckpointDemo.css` | untracked, 1,819 bytes | demonstration shell and responsive presentation |
| `src/dev/demonstration/OperationalSpineCheckpointDemo.test.tsx` | untracked, 1,684 bytes | five-stage semantic walkthrough |

No built output, captured screenshot, server log, or dedicated design-QA record is retained with this worktree.

### Cumulative, alternative, or incomplete?

The component is cumulative in only one narrow sense: it reuses productive DTOs and the existing Work Unit detail view at checkpoint `55cdd40`. The demonstration itself is not cumulative product work. It is an uncommitted presentation overlay with a release-reachable boot seam.

It is incomplete as a retained product surface because it has no declared packaging or navigation policy and no development guard. That does not make the fixture content incomplete; it means the repository has not settled whether this is a development demonstration, a separately built proof page, or an intentionally shipped URL surface.

## 2. Integration Settlement Review Harness

### Intended presentation

`src/dev/orchestrationSection/IntegrationSettlementReviewHarness.tsx` is a much denser deterministic review surface. It presents one accepted integration as six chronological facts:

- Implementer claim — agent-owned, claimed;
- Application evidence — application-owned, collected;
- Handler review — agent-owned, accepted;
- Integration applied — application-owned, applied;
- Settlement confirmed — Handler-owned, confirmed;
- Prerequisite contribution — application-owned, recorded.

Only the three agent-owned facts are primary activity rows. Each application-owned fact is nested under its preceding agent fact:

```text
Implementer claim
  -> Application evidence
Handler review
  -> Integration applied
Settlement confirmed
  -> Prerequisite contribution
```

Application and MCP records are further nested within the owning application activity. The right pane's `Session stream` is filtered to Worker and Handler turns, so application summaries and MCP calls do not masquerade as Agent Session turns.

The default state selects Handler review and the Session stream. Selecting an agent passage expands its full read-only output and recorded steps inline; there is no composer. Selecting an application or MCP record expands it within the activity column. Previous/next controls traverse all six linked facts even though only three are primary rows.

The peer `Evidence` tab deliberately opens without a selected detail. It contains:

- an available, inspectable file-diff fixture;
- an available, inspectable focused-test fixture;
- an unavailable, disabled integration-manifest record.

Hover and focus connect evidence back to the Implementer activity. Activity and Session passages also cross-highlight. Dates and derived processing durations are shown, while the Session list avoids a separate raw timestamp column.

The visible data is hard-coded in the component. Names, timestamps, attempt and revision identifiers, diff lines, test command, four claimed test cases, MCP activity, and application events are fixtures. The `Back to Work Slice planning point` callback is a no-op. `Current work` and technical detail are local popovers. No product client, Tauri command, persistent record, actual file diff, test runner, or MCP call backs the surface.

### Reachability

The modified `ApplicationRoot.tsx` detects `integration-settlement-review` only when `viteDevelopmentMode()` sees `import.meta.env.DEV === true`. It then dynamically imports the Harness and returns it instead of the normal product application:

```text
development index.html
  -> ApplicationRoot
  -> import.meta.env.DEV && ?integration-settlement-review
  -> dynamic import
  -> IntegrationSettlementReviewHarness
```

The check occurs before `humanReviewInstance()`. In a Vite development process where both conditions are true, the integration review wins and the isolated Worktree Review application does not mount.

The query cannot activate the Harness in a Vite production frontend because the development predicate is false. There is no normal product navigation to it and no backend exposure. This is a development root replacement rather than an application feature.

### Test and design evidence retained with the worktree

`IntegrationSettlementReviewHarness.test.tsx` contains five interaction tests covering:

- agent-only Session passages and nested application/MCP records;
- inline, read-only full-turn and step expansion without a composer;
- blank initial Evidence detail followed by diff and test selection;
- evidence-to-activity hover linkage;
- dated duration metadata and explicit unavailable evidence.

The rewritten `design-qa.md` records a human-review progression and says that this review slice was approved after the final application-summary nesting correction. It records:

- route `http://127.0.0.1:43231/?integration-settlement-review`;
- viewport 1239 by 986 CSS pixels;
- five focused tests, scoped ESLint, a Vite bundle, semantic browser interactions, and `git diff --check` as passed;
- zero browser warnings/errors;
- the repository-wide `npm run build` as blocked by unrelated baseline TypeScript errors in `nativeQuery.ts` and `nativeQuery.test.ts`;
- an explicit final limitation: the fixture is demonstration support, not production implementation proof.

This pass confirmed that those statements and matching test sources remain in the worktree. It did not recreate their runtime environment, rerun the commands, or independently reproduce the human approval.

### Retained proof residue

The untracked `.codex-demo/` directory contains 404 files totaling 4,973,702 bytes:

- minimal Vite and Vitest fixture configurations;
- `test-deps/`, a local dependency copy;
- `dist/`, containing 11 HTML/CSS/JavaScript outputs for the main app and Agent Session Harness;
- `vite.out.log`, `vite.err.log`, and `vite.pid`.

The log records Vite 7.3.6 at port 43231, repeated Harness/CSS reloads, and reloads of generated `dist` pages. The PID file contains `25096`, but that process was not running at inspection time. These are retained development-session artifacts, not source inputs and not evidence of a currently live review server.

The approved visual referenced by `design-qa.md` remains outside the worktree at:

`C:\Users\user\.codex\generated_images\019fa398-6f66-76d2-b7b9-38bb2b4898c9\exec-49659c02-277b-450d-bdcf-a182ef3ed623.png`

It is 1,218,508 bytes and was last modified on 2026-08-04. The worktree retains the reference but not a repository-owned copy.

### Retained source artifacts

| Artifact | State | Purpose |
| --- | --- | --- |
| `src/app/ApplicationRoot.tsx` | tracked modification | development-only query switch and root replacement |
| `src/dev/orchestrationSection/IntegrationSettlementReviewHarness.tsx` | untracked, 1,013 lines | fixture model and interactive review surface |
| `src/dev/orchestrationSection/integrationSettlementReviewHarness.css` | untracked, 1,617 lines | desktop and narrow-width presentation |
| `src/dev/orchestrationSection/IntegrationSettlementReviewHarness.test.tsx` | untracked | five semantic interaction tests |
| `design-qa.md` | tracked replacement | visual requirements, review history, retained validation claims, and explicit proof limit |
| `.codex-demo/` | untracked generated material | local test dependencies, build output, configuration, logs, and stale PID |

### Cumulative, alternative, or incomplete?

The view refines ideas already visible in the Work Unit product: distinct semantic ownership, review before integration, settlement after integration, and prerequisite contribution without dependent activation. It does not, however, reuse the product `WorkUnitDetailWorkspace` or bind those ideas to real application data. It is a standalone visual and interaction proposal.

The recorded design review appears complete on its own stated fixture-review terms. The code remains incomplete as product implementation: all facts are local fixtures, navigation is inert, and no application or backend seam supplies the evidence. The accumulated proof material is also not curated for repository retention; it includes generated build output, copied dependencies, logs, and a stale PID.

## 3. Relationship between the two presentation worktrees

The two worktrees are siblings, not a chain:

- both start from the exact same detached commit, `55cdd40`;
- neither contains the other's uncommitted files;
- neither component exists in the current descendant research checkout;
- no integrating commit was found;
- they modify different boot files, but each independently replaces the normal root for its own query.

They are best treated as **alternative presentation/proof approaches** to overlapping operational-spine material:

| Dimension | Operational Spine demo | Integration Settlement review |
| --- | --- | --- |
| Primary question | How does one Work Unit look across five accumulated states? | How should one accepted integration be inspected across ownership, Session, system records, and evidence? |
| Product reuse | real `WorkUnitDetailWorkspace` and presentation DTOs | bespoke fixture component and CSS |
| Data source | typed local fixture | typed local fixture |
| Entry boundary | unconditional normal entry query | Vite-development query inside `ApplicationRoot` |
| Backend dependency | none | none |
| Proof retained | one test source | five tests, QA narrative, screenshot reference, build/test residue |
| Claimed completion | no acceptance record | human-approved fixture review, explicitly not production proof |

They could technically coexist because their query keys differ, but no working state combines them and no evidence establishes that both should be retained. The richer Harness should not be described as a cumulative successor to the smaller demo merely because it was reviewed later; it uses a different component boundary and answers a narrower inspection-design question.

## 4. Main checkout: moving product Harness catalogue

### Inspected non-skill delta

Ignoring the large skill-file migration, six tracked files contain only 11 insertions and 10 deletions:

- `docs/agent-session/plan-builder-managed-session-boundary.md`;
- `src-tauri/src/orchestration/application.rs`;
- `src-tauri/src/orchestration/conversation_harness.rs`;
- `src-tauri/src/orchestration/conversation_harness_catalog.json`;
- `src-tauri/src/orchestration/conversation_harness_working_copy.rs`;
- `src/infrastructure/conversationHarnesses/tauriConversationHarnessInspectorSource.test.ts`.

The operational change is concentrated in the embedded JSON catalogue. Its three Harness profiles change their canonical guidance locations from `.agents/skills/...` to `.agents/product-skills/...` for:

- Epic Plan Builder;
- Epic Bootstrap Generator;
- Epic Runner.

The Rust and TypeScript changes align assertions and fixtures with that path. The documentation also distinguishes ad-hoc `run-overall-plan` use from the product Harness-selected `epic-plan-builder` role.

### How the path participates in runtime behavior

`conversation_harness.rs` embeds `conversation_harness_catalog.json` with `include_str!`. Before a managed role launches, `role_discovery_root`:

1. loads and validates the selected profile;
2. finds the required canonical guidance entry;
3. joins its relative path to the repository root;
4. reads the file;
5. checks its `name` metadata;
6. returns the repository root as the child working directory.

This is not display-only configuration. The canonical path is both prompt content and a launch precondition.

The path check is used by:

- managed Epic Plan Builder message launch in `application.rs`;
- Epic Bootstrap Generator launch in `bootstrap_transition.rs`;
- Epic Runner launch in `bootstrap_transition.rs`.

### The working tree is internally mismatched

At inspection time:

- `.agents/product-skills/` does not exist in the main checkout;
- the old `.agents/skills/epic-plan-builder/` directory shell remains, but its tracked `SKILL.md` and metadata are deleted from the working tree;
- the new untracked catalogue exists at `product/skills/`, not `.agents/product-skills/`;
- no non-reporting source reference to `product/skills` was found;
- every changed Harness canonical path therefore resolves to a missing file in this working tree.

Static consequence: `role_discovery_root` would return an unavailable-canonical-guidance error for all three configured roles before returning the child working directory. The modified test in `conversation_harness.rs` also asserts that the missing `.agents/product-skills/epic-plan-builder/SKILL.md` exists. No runtime or test result is claimed; the failing precondition follows directly from the inspected path check and filesystem state.

### The untracked product-owned catalogue

`product/skills/` retains 16 files totaling 16,922 bytes: one `SKILL.md` and one `agents/openai.yaml` for each of eight role directories:

- `epic-bootstrap-generator`;
- `epic-plan-builder`;
- `epic-runner`;
- `route-epic-feedback`;
- `sprint-runner`;
- `work-slice-planner`;
- `work-unit-handler`;
- `work-unit-implementer`.

This location is outside the automatic `.agents/skills` catalogue and outside the path named by the dirty Harness profiles. In the inspected state it is retained configuration material with no application reachability.

Seven directories correspond to role assets later committed under `.agents/product-skills/` on the `55cdd40` lineage; several files are identical to that descendant material and several have subsequently diverged. `route-epic-feedback` is an additional product catalogue entry. This is evidence of a moving catalogue-ownership decision, not one clean rename.

### Cumulative, alternative, or incomplete?

This main-checkout cluster contains two different answers:

1. The tracked `.agents/product-skills` path change is a **cumulative precursor**. That location and its assets were committed on the descendant `55cdd40` lineage, where the Harness catalogue was also expanded beyond the main checkout's three profiles.
2. The untracked `product/skills` catalogue is an **alternative, incomplete relocation**. It is absent from the current research descendant and is not wired into the dirty main-checkout Harness.

Taken as one working state, the cluster is incomplete and non-runnable for the three managed Harness roles. It should not be centralized by blindly copying all dirty files: doing so would preserve both incompatible catalogue destinations and the missing-path launch failure.

The broader main-checkout skill migration contains 104 total status entries, including many deletions under `.agents/skills`, new ad-hoc role directories, and maintainer reports. Those semantics are outside this pass. Their volume reinforces that this is a moving catalogue experiment rather than an isolated six-line production fix.

## Reachability matrix

| Artifact or behavior | Normal product navigation | Typed URL | Development-only | Requires Tauri/backend | Current integrated descendant |
| --- | --- | --- | --- | --- | --- |
| Operational Spine checkpoint demo | no | `?demo-operational-spine` | no guard | no | absent |
| Integration Settlement Review Harness | no | `?integration-settlement-review` | yes | no | absent |
| `.codex-demo/dist` pages | no | only through a separately served directory | proof residue | no live server at inspection | absent |
| main-checkout `.agents/product-skills` Harness path | managed role launch, indirectly | no | no | yes | location exists on descendants, but not in dirty main checkout |
| main-checkout `product/skills` assets | no | no | no loader found | no current consumer | absent |

## What this evidence does and does not establish

Established:

- two separate uncommitted presentations were created on the same operational-spine checkpoint;
- both use deterministic data and neither proves a live application, provider, MCP, persistence, integration, or dependent-activation path;
- one route would be reachable through the normal production frontend entry, while the other is Vite-development-only;
- the richer review fixture accumulated meaningful human-review and proof artifacts, including explicit limits;
- the main checkout mixes an intermediate Harness path migration with a differently located untracked product catalogue;
- the dirty main-checkout path state cannot satisfy its own managed-role launch precondition.

Not established:

- that either presentation should ship, be merged, or be pruned;
- that the retained build output corresponds byte-for-byte to the final untracked Harness source;
- that the historical test, browser, lint, or bundle claims still pass now;
- that human approval of the fixture design was approval of product functionality;
- that `product/skills` was the final intended catalogue location;
- that any of the three dirty states has been superseded merely because newer descendant commits exist.

## Follow-up questions preserved for later architecture work

- Should deterministic presentation fixtures be separate HTML build inputs, Vite-development routes, test-only components, or normal-entry query overrides?
- Which proof artifacts deserve durable repository retention: source fixtures and tests, approved screenshots, browser records, generated bundles, or none of the generated residue?
- Should review presentations consume the same application read models as the product, or deliberately remain visual prototypes with explicit fixture schemas?
- Is application/MCP activity a nested part of an agent-owned stage, a peer chronological event, or both through separate projections?
- Which system owns the product role catalogue, and how is selective Harness exposure implemented without relying on automatic repository discovery?
- Should canonical guidance paths remain runtime launch preconditions, or should packaged/embedded product assets remove the repository-layout dependency?
- How should development routes be enumerated and prevented from accidentally becoming release-reachable surfaces?
