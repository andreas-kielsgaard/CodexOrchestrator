# Capability chronology

This chronology uses commit dates and subjects to reconstruct how the product expanded. It is a capability narrative, not a claim that every commit landed once in a single linear branch. Several changes were cherry-picked or reconstructed on parallel lines.

## July 10–12: Agent Session reset foundation

Representative commits:

- `4e171e9` — establish Agent Session migration baseline;
- `d717a28` — define provider-neutral contracts;
- `64c26ab` / `0f7da0f` — durable repository and separation;
- `a9c716b` — recoverable lifecycle;
- `0ebe65b` — durable transcript UI;
- `4445a65` — deterministic verification Harness;
- `43a65ed` — Agent Session reset baseline.

Product movement:

- replaced the earlier task/run interaction model with a reusable durable conversation platform;
- separated requested/effective runtime options, persistence and process lifecycle;
- created the frontend transcript/workspace foundation later reused throughout orchestration;
- established a deterministic secondary UI entry for scenario evidence.

## July 17: managed orchestration product begins

Representative commits:

- `2c6fd28` — durable managed orchestration backend;
- `2cf81be` — orchestration contracts and adapters;
- `e54430e` — Agent Session and orchestration workspaces.

Product movement:

- introduced the Rust orchestration module, managed Plan Builder and durable proposal/initiation concepts;
- connected frontend application contracts and initial Epic/Sprint workspaces;
- began the pattern of building orchestration on top of Agent Sessions rather than embedding provider behavior directly in UI.

## July 29–31: product hierarchy, navigation and role model

Representative commits:

- product-aware Agent Session navigation and Sprint detail review;
- Handler review kept on its originating Session;
- five-role Sprint forecasting;
- Epic Plan Builder hierarchy alignment;
- application-owned planning-point relationships;
- Plan Builder Harness Management convergence.

Product movement:

- moved from a set of screens toward an Epic/Sprint/Work Unit information hierarchy;
- made agent conversations contextually reachable from product work;
- established early role vocabulary and Harness inspection/management ambitions;
- started treating navigation origin and Session ownership as durable product semantics.

## August 1: experience expansion, File Review and authored Harness state

Representative changes:

- resizable Epic/Sprint detail panels and converged Agent Session navigation;
- explicit Handler/Implementer role vocabulary;
- scoped File Review facts, Git capture authorization and hardened production;
- initiated Sprint Git authority;
- contextual File Review invocation and Sprint navigation;
- durable Harness working copies.

Product movement:

- created the reusable split/detail workspace patterns used by the current product;
- made Git/file evidence a bounded application artifact rather than a raw repository view;
- introduced private Sprint Git authority;
- crossed from compiled Harness profiles into durable authoring concepts.

## August 2: planning materialization and execution authority

Representative changes:

- authorized Work Slice planning request and prelaunch state;
- prepared and launched the Work Slice Planner;
- accepted and materialized Work Slice proposals;
- added typed Work Units and relationships to the native query;
- created execution-support authorization and isolated attempt worktrees;
- added runtime-observation evidence;
- established immutable Harness revisions;
- activated durable Work Unit Handlers.

Product movement:

- transformed orchestration from planning/presentation into an executable workflow;
- introduced the application-owned boundary between a Work Unit and a real isolated workspace;
- made Harness revisions durable and pinned rather than relying only on current configuration;
- began projecting operational activity rather than inferred agent state.

## August 3: Handler-to-Implementer delegation

Representative changes:

- execution grants keyed by role;
- separate Handler action continuation;
- Implementer activation schema, failure facts, drain and read model;
- terminal-gated Handler continuation;
- role-scoped recovery, concurrency and authentication proofs.

Product movement:

- decomposed one Work Unit into controlled Handler and Implementer responsibilities;
- introduced a least-authority continuation exposing only the application-owned Implementer request;
- established restart-safe delegation and explicit launch failure states.

## August 4: outcome, evidence, review, retry and integration

The largest single-day capability expansion added:

- separate Implementer reporting Harness and same-Session continuation;
- bounded outcome claims and semantic completion;
- application-captured evidence readiness;
- independent Handler review and structured return;
- meaningful-progress retries and no-progress handbacks;
- accepted candidate authority tied to exact File Review capture;
- application-owned Git integration and restart convergence;
- dependency-wave activation and graph relations;
- multi-attempt history and strict frontend projection.

Product movement:

- completed the core implementation-to-review-to-integration spine;
- separated agent claims, application evidence and Handler judgment;
- made retries and dependencies first-class workflow facts;
- changed Git integration from an implied downstream action into an owned, durable state machine.

## August 5: graph completion, handback and Epic escalation

Representative changes:

- Work Slice graph drain and factual execution states;
- Sprint handback projection and typed frontend status;
- Epic escalation receiver, delivery, reassessment, dependency ownership and attention;
- combined graph settlement and Epic reassessment;
- corrections making original Epic Runner/Implementer paths productive.

Product movement:

- linked completed Work Units into Work Slice-level settlement;
- created an explicit upward route when local Sprint movement is exhausted;
- expanded from execution mechanics into workflow oversight and recovery.

This is also the shared divergence period. The latest operational, Product Decisions and final-settlement lines all descend from `e3bde2c` and then evolve separately.

## August 5–6 sibling line: Sprint continuation and final Epic settlement

Representative commits:

- `c9cf601` — durable Sprint continuation settlement;
- route-bound dependency waits and conflicting-result guards;
- `5900296` — durable Epic receipt for Sprint results;
- direct disposition realization;
- `13f8d44` — exact Epic settlement;
- strict eligibility and frontend projection corrections.

Product movement:

- modeled Sprint outcome delivery, Epic receipt, continuation/attention/settled decisions and successor realization;
- added a strict final Epic closure boundary requiring all lower-level evidence to agree;
- integrated through reconciliation/native query rather than new Tauri commands.

## August 6 sibling line: Product Decisions and navigation oversight

Representative commits:

- Product Decision evidence navigation and disclosure reset;
- durable immutable versions and passage anchors;
- correction conversation and proposal flow;
- explicit acceptance and live async guards.

Product movement:

- created durable product-level decision evidence attached to an Epic;
- allowed an Agent Session to discuss corrections without applying them automatically;
- kept human or agent-assisted acceptance provenance explicit;
- expanded cross-product navigation and inspection quality.

## August 5–6 operational correction line

After the shared point, the operational line concentrated on making the execution spine usable and safe:

- exact Sprint Git authority before planning;
- Handler workspace reopen and shorter Windows paths;
- productive original Implementer route;
- bounded candidate/reporting runtime;
- WorkspaceWrite MCP scoping;
- reentrant notifier/transition locks;
- evidence gating and project-trust hardening;
- private sandbox diagnostics.

This history is a reminder that “implemented” stages needed repeated integration correction after deterministic state-machine construction.

## August 6–7: Native Codex Profiles

The Native Profile line rapidly added and hardened:

- durable existing/dedicated Codex homes;
- profile selection and consumer gate;
- browser-login attempt evidence;
- Windows sandbox setup and setup-attempt evidence;
- external sandbox adoption plus explicit product confirmation;
- WorkspaceWrite canary receipts;
- execution modes and exact danger authorization;
- full-access canary and cleanup evidence;
- strict launch projection;
- native MCP reporting request, dispatch claim and receipt settlement.

The initial research anchor ends at `b28137b`. Three later commits through `dbe321d` serialize MCP probe creation with profile selection and strengthen race proof. `9240364` then makes selected, ready profile identity a shared Agent Session preflight requirement and persists Session/profile continuity. It does not adopt the stricter Native Profile launch projection or execution-mode authority.

## What the chronology suggests

- Agent Sessions are the platform foundation; orchestration began a week later and continually reused them.
- The product expanded vertically: planning, then execution authority, then delegation, then evidence/review, then integration/dependencies, then oversight/settlement.
- UI, state, configuration and proof generally evolved together rather than as separate workstreams.
- Many later commits are convergence and fail-closed corrections, indicating that lifecycle boundaries—not basic feature presence—were the dominant implementation challenge.
- Native Profiles, Product Decisions and final settlement are three different late-stage governance/control expansions on divergent lines.
- A future architecture should learn from the sequence: each new authority layer introduced new request/acceptance/observation/settlement distinctions and new cross-module coupling.

## Visual opportunity

This chronology can later become a layered timeline with:

- horizontal time;
- vertical lanes for experience, Agent platform, orchestration control, evidence/Git, Harness/configuration and governance;
- branch splits at `e3bde2c`;
- markers for feature introduction versus later convergence/correction.
