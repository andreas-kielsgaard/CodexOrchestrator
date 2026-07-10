# Agent Session Architecture and Decisions

Status: implemented recovery baseline

Created: 2026-07-10

## Product Definition

An Agent Session is a durable interaction context for doing work through a text interface.

Today, the runtime is Codex CLI and the presentation resembles a ChatGPT or Codex conversation.
In the future, an Agent Session may be backed by an API agent, a local model, a cloud agent, or a
deterministic tool. That future does not require a generic implementation now, but it does require
the product concept to remain independent from a specific process or provider thread ID.

The visible transcript is the presented reality. It is not the complete technical reality.

## Three Views of the Same Session

### Presented conversation

What the user normally sees:

- user inputs
- live processing while the invocation is active
- the final agent response
- completed processing collapsed behind an expandable disclosure

The collapsing behavior is intentional. Codex may emit reasoning, tool activity, intermediate
messages, and other work during one invocation. That activity should be inspectable while it is
happening and after completion, but the final comprehensive response should dominate the completed
view.

### Interaction history

The durable record needed to explain what happened:

- user inputs
- invocation lifecycle
- provider events
- tool and command activity
- intermediate and final agent messages
- errors, usage, and terminal results

This record is richer than the default transcript.

### Runtime context

Provider-specific state needed to continue correctly:

- Codex thread ID for CLI resume
- runtime kind and version information
- working directory and effective configuration
- future API context/cache lineage or local-agent state

Runtime context can be available without being visible. A future branched planner session may
inherit context from a root session while presenting only post-branch history.

## Core Records

The implementation uses separate records for the durable context, individual runtime work, and
technical output.

### `AgentSession`

Owns the stable product identity and runtime binding.

Minimum responsibilities:

- stable local `id`
- runtime kind, initially `codex_cli`
- nullable external context ID, initially the Codex thread ID
- optional working directory
- title and creation/update timestamps
- effective or requested runtime configuration where known
- availability/archive state, separate from invocation completion

The local ID is created once and never renamed when a provider identity becomes known.

### `AgentInvocation`

Represents one submitted input and one runtime attempt.

Minimum responsibilities:

- stable invocation ID and owning session ID
- submitted text
- lifecycle status
- requested/effective runtime options
- start and completion timestamps
- exit code, signal, and runtime error when present
- persistence or transport diagnostics without rewriting the runtime outcome

Initial invocation statuses:

- `pending`
- `running`
- `completed`
- `failed`
- `canceled`
- `interrupted`

Only one invocation may be active per session in the first slice. The process supervisor may own
processes for several different sessions concurrently.

### `AgentRuntimeEvent`

Represents ordered technical output for an invocation.

Minimum responsibilities:

- stable event ID
- invocation ID
- monotonic sequence number within the invocation
- source/stream
- raw provider payload
- normalized kind and selected normalized fields when understood
- recorded timestamp

Raw payload must be retained. Normalization should tolerate unknown event and item types.

### Transcript projection

The transcript is derived from the records above. It is not the authoritative persistence model.

The projection should distinguish:

- submitted user input
- currently active processing
- intermediate technical activity
- intermediate agent messages
- final agent response
- errors and interrupted/canceled outcomes

On completion, intermediate work is grouped into an expandable processing disclosure and the final
agent response remains visible. The projection must preserve the grouped detail even though it is
collapsed by default.

## Responsibility Flow

```text
Agent Session views
  -> Agent Session feature controller
    -> Agent Session application client
      -> Tauri Agent Session commands and queries
        -> Agent Session repository
        -> Codex CLI runtime adapter
          -> process supervisor
```

Runtime updates travel back through a persisted stream:

```text
child process output
  -> persist ordered runtime event
  -> emit correlated frontend update
  -> update transcript projection
```

Persistence precedes frontend notification. Missing a transient UI event must be repairable by
reloading the invocation or session.

## Layer Ownership

### Domain/application concepts

- Agent Session identity and invariants
- invocation lifecycle rules
- runtime binding as an abstract concept
- session/invocation repository contracts
- send, load, list, cancel, and startup-reconcile use cases
- provider-neutral event and transcript projection inputs

These concepts must not import React, Tauri, SQLite, process APIs, or Codex JSONL types.

### Codex CLI adapter

The first runtime adapter is deliberately Codex-specific. It owns:

- constructing supported `codex exec --json` and `codex exec resume` arguments
- mapping semantic runtime options to the installed CLI's supported syntax
- identifying the Codex thread from `thread.started`
- parsing Codex JSONL into normalized runtime events
- classifying Codex completion and failure
- preserving unknown provider data

The application layer should request semantic options such as model or sandbox. React must not
construct raw CLI argument arrays.

A future generic LLM/agent runtime port may have several implementations. The generic port should
be extracted from the proven needs of the Codex adapter, not invented in advance.

### Process supervisor

`CLISessionMaster` is replaced by a real backend process supervisor.

The supervisor owns actual child-process handles and is responsible for:

- starting a process for an invocation
- correlating the process with its invocation ID
- holding several active processes for different sessions
- preventing a second active invocation for the same session
- streaming stdout and stderr
- reporting process exit
- canceling one invocation
- terminating or reconciling owned processes during app shutdown
- removing completed processes from the active registry

The first supervisor does not need scheduling policy, worker queues, quotas, priorities, or generic
resource distribution. It should make concurrent process management possible without pretending to
solve orchestration.

### Tauri boundary

The Tauri boundary owns serializable commands, queries, acknowledgements, and update events. It does
not own transcript presentation.

Implemented surface:

- create or lazily establish a session
- list session summaries
- load one session with invocations/events
- send a message
- cancel an active invocation

Start/send acknowledgements must include both session and invocation IDs. Stream messages must be
correlated by invocation ID. A completion event is a notification, not the only way to learn the
terminal result.

### Feature/UI layer

The feature layer owns:

- selected/open session state
- draft input
- send/cancel interactions
- stream subscription and query reconciliation
- expansion state for completed processing details
- transcript and composer rendering

It does not own runtime identity, process state, durable history, raw CLI arguments, or SQLite
records.

## Key Decisions

### D-01: Local and provider identities remain separate

Reasoning: one product session may be backed by different runtime mechanisms over time, and a
provider identity may not exist until the first runtime event. Renaming the local primary key
breaks references and confuses continuation.

### D-02: The session outlives every process

Reasoning: Codex CLI uses a new `codex exec` process for each submitted input. The session is the
durable context across those invocations.

### D-03: Invocation status is not session status

Reasoning: a completed response does not complete the conversation. Terminal process state belongs
to the invocation; the session remains available for another input.

### D-04: Persistence is authoritative; streaming is an optimization

Reasoning: event delivery can be interrupted by WebView reload, listener failure, or app shutdown.
The session must recover by query after any of those failures.

### D-05: Store raw provider output before relying on projections

Reasoning: provider event shapes evolve, normalized parsing can be corrected, and technical detail
may later be needed for context management or diagnostics.

### D-06: Completed presentation is final-first

Reasoning: the comprehensive final response is normally the useful result. Intermediate work is
valuable for inspection and live feedback but should not overwhelm the completed conversation.

### D-07: The first runtime adapter is Codex-specific

Reasoning: current behavior and identity rules are provider-specific. A prematurely generic
adapter would move ambiguity into abstract names rather than removing it.

### D-08: Agent Sessions are independent from tasks and orchestrations

Reasoning: task, goal, repo, and orchestration relationships are optional future edges. They are
not ownership relationships and must not gate session startup or persistence.

### D-09: Runtime settings are capability-driven

Reasoning: the CLI surface changes. The app should not always emit hardcoded flags or fallback model
names. Unsupported settings remain absent or use a version-aware mapping.

### D-10: Prototype migration versions are never silently reused

Reasoning: local databases may contain migrations from the archived overlay. Migration identifiers
and positions must remain immutable even when their feature code is discarded. The rebuild must
audit/reset prototype data explicitly or move forward with a new non-colliding migration version.

## Existing Concepts Not Used as Foundations

### Task-oriented `Conversation`

The current `Conversation` record is task/run catalog metadata. It lacks invocation and event
history and includes task-specific relationships. It remains quarantined with the task-run system
until a later reconciliation decision.

### Orchestration `AgentConversation`

The archived orchestration conversation model is a UI truth/provenance model. It is not the Agent
Session domain and is excluded from this rebuild.

### TypeScript CLI pooling

The archived master/distributor classes pool handler objects rather than owning operating-system
processes. Their current implementations are discarded. General distribution policy is deferred
until real consumers and constraints exist.

## Structural Direction

The implemented ownership map is:

```text
src/
  application/agentSessions/     serializable frontend client contract and DTOs
  infrastructure/agentSessions/  Tauri Agent Session client
  features/agentSessions/        controller, transcript projection, and views

src-tauri/src/
  agent_sessions/domain.rs       durable records and invariants
  agent_sessions/ports.rs        repository, runtime, and notification ports
  agent_sessions/repository/     SQLite schema, mapping, and repository coordination
  agent_sessions/application/    lifecycle and persist-first update sink
  agent_sessions/transport/      Tauri DTOs, commands, and notifications
  runtime/processes/             real process supervisor
  runtime/codex/                 Codex CLI process and protocol adapter
```

The frontend and backend do not need mechanically identical folders. Responsibilities matter more
than symmetry.

## Implementation Notes and Known Limits

- A session is established lazily when its first message is sent. The acknowledgement returns both
  the stable local session ID and invocation ID.
- Tauri events provide low-latency updates; durable reload remains authoritative. While an
  invocation is active, the frontend also reconciles on a short interval so a missed event cannot
  strand the visible state.
- Expansion preference is transient UI state. The underlying processing and technical record is
  durable and can be expanded again after restart.
- The default process factory owns and reaps the direct Codex child. Portable Rust process APIs do
  not guarantee descendant-tree termination on Windows; full tree ownership would require a Job
  Object or another platform-specific factory.
- Several repository/supervisor inspection methods remain unused by the first product path and
  currently produce compile warnings. They are not exposed as inert UI controls or represented as
  completed scheduling behavior.
- The task-oriented conversation/run system remains separate. No task, goal, repo, or orchestration
  relationship was introduced into Agent Sessions during the reset.
