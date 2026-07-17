# Post-orchestration review notes

These are deferred review inputs, not current implementation instructions.
Current product terminology follows `terminology.md`; historical discovery references below retain
their original wording only where that wording identifies the reviewed artifact.

## Orientation discovery component growth

The accepted discovery accumulated overview, detail, Sprint plan, dialog, session, continuation, and
status behavior in one 481-line component and one 837-line stylesheet. The cleanup after discovery
separated those responsibilities under `src/features/orchestrations/components/` and colocated
styles under `src/features/orchestrations/styles/` without changing behavior.

Review later:

- whether Work Unit review should flag a component that acquires several independently testable
  responsibilities;
- whether discovery completion should include a small structural pass after the user accepts a
  direction;
- whether styles should remain colocated with the responsibility that owns their selectors.

Do not add automated thresholds or architecture enforcement until this is reviewed against more
than one completed Epic.

## Recorded versus product control

The discovery used real Agent Session presentation components with recorded transcript data and
non-executing local controls. This is appropriate for deterministic UI evaluation, but it does not
prove product integration.

Review later:

- recorded and product modes should share the component tree;
- mock data and behavior should be injected at application client/controller boundaries;
- product Agent Control commands need explicit authority and policy;
- test-only process observation and verification privileges must not leak into product controllers.

## Legacy Rust root module

`src-tauri/src/lib.rs` is approximately 6,000 lines because it retains the earlier task/run product
implementation: legacy migrations, database helpers, task and run commands, Git/repository work,
Codex launch paths, validation, DTOs, and their tests. The reset left this implementation compiled
for compatibility and isolated tests while active legacy commands fail closed.

Correcting that blob is outside the current UI and orchestration-discovery scope. Until a dedicated
retirement or extraction effort is authorized:

- do not add new orchestration behavior to `lib.rs`;
- do not call its legacy task/run handlers from new product flows;
- state new requirements independently and create focused ports/modules for them;
- revisit deletion or extraction after active replacements cover the capabilities that are still
  genuinely needed.

## Product tooling explorations after the active flow

These explorations may begin only after the active Sprint has finished and no further independent
work remains within its accepted flow. They are not part of Sprint 6. Disposable evaluation views
may use dedicated application tabs like the Agent Session view; they should still reuse intended
product components and injected test data rather than become separate UI products.

### Agent Session harness inspector

When an Agent Session has a product harness configuration, overlay a control on the session pane.
Opening it replaces that pane with an inspector for viewing and editing the harness components, with
a clear way back to the conversation. Explore how to present prompt/context, skills, MCP tools,
model and reasoning settings, sandbox/authority settings, and application hooks without exposing
internal configuration as an undifferentiated blob. Distinguish configuration that can affect a
future invocation from context already delivered to the existing session.

### In-app file and diff viewer

Explore a reusable product viewer for created files and diffs. It should at minimum render Markdown
and provide familiar Git diff behavior such as file navigation, additions/deletions, hunks, and
side-by-side or unified inspection where appropriate. Keep file access and write authority separate
from presentation.

### Worktree-aware application testing

Explore how multiple worktree-based agents can run and test their own changes concurrently while
reusing unchanged build material from the main checkout where safe. The design must preserve source,
build-output, process, database, port, and application-state isolation. Investigate prior art such as
OpenAI's in-app development harness, but do not assume its implementation or safety properties.

### Parallel orchestration and human control

Future orchestration should support more parallel work while improving human control rather than
increasing monitoring burden. Explore attention routing, approval points, pause/resume, intervention,
comparison of projected and actual work, and safe automatic continuation after the underlying serial
transitions and recovery behavior are proven.

### Agent-native application control and evidence

Explore stronger semantic application-control capabilities so agents can inspect and exercise the
Codex Orchestrator without depending on generic computer-use automation. Prefer product-owned,
testable actions and structured observations while retaining visual proof where it matters.

Explore application-owned demo recording so an agent can capture a concise video of the feature and
flow it implemented. Recordings should identify the tested build and avoid exposing unrelated user
or session data.

Explore an in-app annotation mode for detailed human feedback during testing. Annotations should be
anchored to the relevant view, element, state, or captured frame and remain available to the agent as
structured feedback without making annotations authoritative product events.
