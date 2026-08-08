# Decision-record evolution

The eight records under `docs/orchestration/decisions/` explain the architectural intent from durable Plan Builder state through post-confirmation bootstrap. They are valuable historical authority, but they do not cover the later Work Slice/Work Unit execution, integration, escalation, Native Profile, Product Decision or final-settlement architecture.

## Record map

| Record | Accepted/superseded intent | Current reading |
| --- | --- | --- |
| 0001 durable state/native query | fresh unified active-v3 database; Rust owns durable authority; TypeScript strictly decodes and composes presentation | foundational shape remains; schema advanced from v1 to v35 and native query grew substantially |
| 0002 MCP transport/access | official rmcp, in-process loopback HTTP, ephemeral server, bearer-only child environment, Host/Origin guard, semantic authorization per call | productive pattern remains; expanded from Plan Builder to 13 orchestration server variants, with duplicated hosting and native-profile divergence |
| 0003 managed invocation/proof | keep Agent Runtime role-neutral; orchestration assembles child-scoped MCP extension; distinguish deterministic from live/provider proof | provider-neutral Agent Session boundary remains one of the strongest architectural choices |
| 0004 Plan Builder tool catalogue | originally one proposal tool | explicitly superseded by 0006; stale one-tool language survives in some frontend/config artifacts |
| 0005 initiation scope | exact proposal snapshot creates Epic and ordered Sprints but no Work Units or execution; `initiated` has narrow meaning | narrow initiation meaning remains; later services add separately named transition facts |
| 0006 Harness and confirmation | Plan Builder gets two tools; one explicit human confirmation coordinator; base Harness roles stay outside Agent Session identity | current Plan Builder and modal flow follow this; Harness variants later became much richer |
| 0007 post-confirmation bootstrap | preparation, Bootstrap attempts, semantic material completion and Epic Runner launch are separate, restart-safe facts | current bootstrap path follows this separation and has since connected into Sprint execution |
| 0008 context delivery/transition UI | one frontend confirmation controller; button-only one-shot context delivery; compact truthful transition status | productive frontend architecture; later product surfaces extend far beyond this recorded scope |

## Durable principles that survived expansion

- pre-initiation planning identity does not imply an Epic;
- initiation, preparation, Session creation, launch, semantic completion and acceptance are distinct;
- Rust owns durable validation and authorization;
- frontend reads decode versioned native contracts rather than querying SQL;
- MCP servers are transient and invocation-scoped;
- Agent Sessions and runtimes remain provider/role-neutral;
- bearer possession is supplemented by semantic durable checks;
- explicit user confirmation is required for initiation;
- deterministic protocol proof does not prove model/provider behavior;
- startup reconciliation uses durable identities instead of transcript inference.

## Areas where implementation outgrew the records

### MCP scope

The early records focus on Plan Builder and bootstrap. The implementation now has 22 orchestration tools across selection, start, planning, Handler/Implementer work, review, handback and escalation. There is no later decision record defining the shared endpoint taxonomy or stage-specific Harness variants.

### Persistence scope

The original focused database evolved into roughly 95 product table names plus external artifacts and Git authority. Schema-version increments and feature-local migrations accumulated without a later record revisiting whether one shared database remained the intended bounded-context architecture.

### Execution and integration

Record 0007 explicitly stopped after Epic Runner launch. Current code continues through Sprint Runner, Work Slice planning, Work Unit execution, evidence, review, retry, accepted integration, dependency waves and escalation. Those boundaries are well tested but not summarized by an equivalent architecture-decision series.

### Harness authoring

Records describe versioned profiles and runtime selection. Current code also has durable working copies, content-addressed immutable revisions and application-authored continuation variants, while the mounted UI exposes only a compiled Plan Builder inspection.

### Native execution policy

Native Profiles introduce a second, stricter Codex configuration and environment model with filesystem-identity authority, UAC/setup/canary evidence and danger-mode policy. This does not appear in the original Agent Session/managed-invocation decision set.

## Documentation status implications

- “Accepted” in these records should be read as the accepted boundary for its Sprint, not as a complete description of the current engine.
- Explicitly superseded statements should remain historical evidence rather than being used as current contracts.
- Some current code comments still repeat earlier scope or dormancy language after the capability became productive.
- The absence of later ADRs is itself a finding: implementation evidence currently carries much of the architecture that decision records once made explicit.

## Follow-up opportunity

A future decision review could create retrospective records for:

- execution/Harness continuation model;
- candidate/integration and dependency settlement authority;
- shared active-database ownership;
- Harness effective-configuration provenance;
- Native Profile relationship to Agent Runtime;
- internal review-tooling boundary;
- branch convergence for Product Decisions and final settlement.

Those records should describe current truth and future choices without rewriting the historical decisions.
