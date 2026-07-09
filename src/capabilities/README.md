# Capabilities

`src/capabilities` is for reusable frontend-facing capability contracts that
describe what the UI can ask the system to do. Capabilities should be narrower
than a full screen and stable enough for multiple workflows to share.

Rules:

- Express user-meaningful actions such as loading dashboard data, starting a run,
  checking runtime health, or registering a repo.
- Depend on application contracts and DTOs, not concrete infrastructure adapters.
- Keep React state, rendering, and feature workflow sequencing out of capability
  definitions.
- Do not duplicate domain services or infrastructure ports; capabilities are a UI
  consumption boundary over application use cases.

Some current broad clients may overlap with future capabilities. Record and
resolve those redundancies in a later cleanup pass rather than changing behavior
as part of documentation work.

Current capability groups:

- `openTaskDashboard`: Open Tasks dashboard reads and task mutations.
- `repoOnboarding`: repo discovery and registration.
- `taskRunLaunch`: starting a Codex-backed task run.
- `taskRunDetail`: task/run detail reads.
- `runtimeHealth`: app/runtime stale-status checks.
- `backendMaintenance`: backend freshness checks and reopen requests.
