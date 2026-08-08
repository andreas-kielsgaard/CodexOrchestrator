# Completed discovery signal sweeps

These ledgers preserve the broad trigger, effect, feedback, reachability and artifact sweeps that selected the deeper observation passes. They are supporting discovery evidence, not an ongoing investigation queue or the preferred introduction to the product.

## Independent sweeps

| Direction | Evidence captured |
| --- | --- |
| [Frontend signals](frontend-signal-sweep.md) | 69 entry, interaction, implicit-refresh and adapter-only signals; 10 contradiction/orphan flags |
| [Native lifecycle signals](native-lifecycle-signal-sweep.md) | all 66 debug-build Tauri registrations, 45 release registrations, two event routes, startup/reconciliation/shutdown and background ownership |
| [Agent, process and executable-configuration signals](agent-process-signal-sweep.md) | 50 MCP, CLI, process, environment, Harness, Git, operator, quarantine and test signals |
| [Durable and external effects](durable-and-external-effect-sweep.md) | reverse sweep from cross-domain effects, including a focused immutable Harness publication traversal |

All four sweeps use research tip `9240364`. They record source behavior and reachability, not packaged or controlled-live product proof unless explicitly stated.

## Cross-sweep collision points

These are places where independently discovered signals meet or disagree. They are candidates for deeper traversal, not predefined product buckets.

### Native MCP action

- Rust exposes one command that creates the pending request and a second that reconciles/dispatches it.
- The visible action and frontend adapter call only reconciliation.
- Reconciliation does not create a missing request.

The backend lifecycle is substantial and the visible effect remains predictably inert without a separately created pending fact.

### Harness authority

- The frontend presents a generic management component, but the productive source supplies Plan Builder inspection without mutation dispatch.
- Work Unit progression automatically saves working copies, publishes immutable role revisions across SQLite and filesystem evidence, and pins them into launch facts.
- Static JSON, code-authored variants, revision content, transition prompts and dynamic MCP injection all participate in one effective launch.

Visible management, durable authoring and runtime authority are three different seams using overlapping vocabulary.

### Contextual File Review

- The control is visible and the Tauri command is release-registered.
- Release composition supplies an unavailable producer.
- Debug composition connects the Git-backed producer and persistence path.
- Stored opaque review facts can still be loaded through the productive read seam.

The viewer, producer, command registration and build composition each answer a different reachability question.

### Native Profile consumption

- Technical Settings offers extensive profile/readiness/mode operations.
- A “load” query performs reconciliation and can mutate durable state.
- Every shared Agent Session launch now consumes selected ready profile identity and `CODEX_HOME`.
- The stricter general WorkspaceWrite projection and selected execution-mode authority are not used by the shared runtime path.
- Native children use an allowlisted environment; ordinary Agent Sessions inherit the desktop environment around the selected home overlay.

Profile identity is centralized while launch policy and completion observation remain split.

### Product source checkout as runtime authority

- Sprint transition derives repository identity from compile-time `CARGO_MANIFEST_DIR`.
- It requires the application source checkout completely clean, including untracked files.
- Durable Sprint Git authority begins from that checkout's `HEAD^1` and `HEAD`.
- Work Unit worktrees, candidate commits, File Review evidence and accepted integration descend from this authority.

Build/source state is therefore productive configuration with downstream mutation consequences.

### Runtime feedback and freshness

- Agent events are persisted before synchronous orchestration callbacks and frontend notification.
- Transition progress has no dedicated Tauri event.
- The orchestration frontend primarily mount-loads three separate snapshots and does not generally poll them.
- Native Profile child completion is observed only through later query/action reconciliation.
- Human Review uses several rapid polls and can keep a hidden product application mounted beneath its own shell.

There is no single application-wide definition of “current on screen.”

### Startup and shutdown ownership

- Startup recovery can create Sessions, launch Codex children and open MCP servers before the frontend has requested a refresh.
- Exit explicitly drains managed Plan Builder, bootstrap/Sprint MCP registries and the shared Agent runtime.
- Shared Agent child shutdown can prevent application exit on failure.
- Native Profile children are cleaned up later through best-effort `Drop` and cannot prevent exit.
- Debug review uses a stronger Windows Job Object ownership model than ordinary Agent Sessions.

Process ownership and lifecycle truth vary by adapter.

### Secondary and retained surfaces

- The Agent Session recorded Harness is a separate production build input without normal navigation.
- Worktree Review is frontend-development-gated and backend-debug-gated through different mechanisms.
- The legacy Task UI, adapters and backend remain substantial, while release commands stop at one quarantine guard.
- A predecessor worktree runtime view is unmounted; Node and Rust operational runtimes coexist through different entry points.

Source, build, registration, route and effect reachability continue to disagree in repeatable ways.

## Traversal handles selected by evidence

The independent sweeps selected three high-information traces:

1. [the application source checkout becoming Sprint and Work Unit Git authority](../evidence-passes/source-checkout-git-authority-pass.md);
2. [one exact Work Unit launch assembled across Harness revision, MCP, Native Profile, environment and process policy](../evidence-passes/effective-implementer-reporting-launch-pass.md);
3. [startup and shutdown behavior that performs external work without a unified visible lifecycle](../evidence-passes/startup-shutdown-observability-pass.md).

All three traversals are now captured. They did not settle a taxonomy. They strengthened three cross-cutting explanations: build/source context can grant product authority; an effective agent is an assembled operation rather than one configuration artifact; and backend lifecycle ownership is more unified than frontend temporal visibility.
