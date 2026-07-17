# Product Data and Controller Integration final handoff

Status: Sprint 1 is accepted and closed after final Epic review on 2026-07-15. Sprint 2 is the
authorized clean terminology break described in `terminology.md`.

## Established

- Provider-neutral product reads and canonical composition feed the accepted Orchestration
  capability surface and its Epic and Sprint component tree.
- Product and same-tree recorded adapters remain separate authorities; product boot has no development
  fixture authority.
- Embedded Agent Session injection, Agent Control seams, and ArtifactAccess seams are composed at the
  application boundary.
- Sprint-level and Epic-level automatic-continuation policy updates have separate level-specific
  controller seams. UI switches do not request continuation, and checked/progress state remains a
  canonical read.
- A recorded Agent Control result must reference the exact submitted command before canonical event
  coherence can be accepted.
- Product presentation does not claim an Epic ID is a task ID.

## Unproven and unsupported

No persistence, MCP handling, transition or automatic-continuation execution, prompt delivery,
provider/process launch, native artifact effect, or live call is implemented or proven. Product
policy updates remain explicitly unsupported until durable storage and canonical refresh exist.

## Sprint 2 boundary

Sprint 2 changes terminology only. It requires no data migration or compatibility aliases because
there is no production orchestration data; disposable recorded fixtures may be rebuilt. Runtime
behavior remains separately authorized.
