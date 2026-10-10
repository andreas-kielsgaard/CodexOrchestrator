# Jev API sessions

Jev uses the existing Agent Session and invocation lifecycle. Backend callers create a session, submit a typed evaluation, and read its persisted result. Each invocation makes an independent HTTP request; previous invocations are not added to its state.

In **Technical Settings → JEV**, save the TypeSafe API key in Windows Credential Manager. It is scoped to the current Windows user under `dev.codex-orchestrator.app/jev/api-key`, independent of executable path, branch, and database. If no saved key exists, Orchid uses the launcher's `TYPESAFE_API_KEY`. The key is read at invocation time and is not stored in session records or returned by backend commands. The endpoint is `https://api.typesafe.ai/v1/systemone`; the default model is `jev-latest`. A model ID can be selected when creating a session.

## Backend commands

The commands are registered through Tauri and need no new UI. For example, from a frontend integration using `invoke` from `@tauri-apps/api/core`:

```ts
const session = await invoke<{ id: string }>('create_jev_api_session', {
  input: { title: 'Voice decisions', model: 'jev-latest' },
});

const invocationId = crypto.randomUUID();
const receipt = await invoke('invoke_jev_api_session', {
  input: {
    sessionId: session.id,
    invocationId,
    evaluation: {
      state: { transcript: 'Please investigate this', playback: 'idle' },
      questions: {
        needs_work: {
          type: 'noul',
          instructions: 'Does this fragment request work?',
        },
      },
    },
  },
});

const view = await invoke('load_jev_api_invocation', {
  sessionId: session.id,
  invocationId,
});
```

`receipt` contains `sessionId`, `invocationId`, and `launchAccepted`. Launch acceptance is separate from evaluation success. `view` contains the invocation, including its status and any runtime error, and an optional typed `result`. Read it after the existing `agent-session-update` terminal notification, or poll until the invocation is terminal. Accept a result when its invocation is `completed`.

Rust product consumers call `AgentSessionApplication::{create_jev_api_session, invoke_jev_api_session, load_jev_api_invocation}` directly. Input/output types live in `orchid_engine::providers::jev::contracts`. The voice feature owns its question definitions, explicit context in `state`, and interpretation of answers.

## Application CLI

`node review-tools/app-inspector/jev-query.mjs request.json result.json` submits an evaluation through the running executable's native commands and waits for its persisted terminal result. The request contains `exe`, `pid`, `debugUrl`, `targetUrl`, and `evaluation` (the state/questions object above). Optional `model` and `title` apply to a new session; `sessionId` reuses a Jev session, and `invocationId` permits explicit replay verification. A failed invocation exits with status 1 and retains its result record. The key stays in Windows Credential Manager or the executable's environment.

Use an explicitly identified instance launched with a loopback WebView2 debug port; see the [application inspector](../review-tools/app-inspector/README.md). The CLI verifies endpoint ownership and invokes the product's actual session commands. It does not call TypeSafe directly.

Reusing an invocation ID with the same evaluation returns its existing acknowledgement without another request. Reusing it with different input fails. Use a new ID for an intentional new evaluation.

Cancellation uses `cancel_agent_invocation`, and application shutdown cancels active Jev requests. A 30-second timeout and API failures become durable failed invocations. Calls are not automatically retried. Cancelling the local request cannot reverse vendor processing or charges.

The request accepts typed Noul, Choice, and Score questions, including multiple questions per evaluation. The result retains the model, answers, and usage returned by the API. Schema validation rejects incomplete or mismatched answers before publication. See the [TypeSafe API reference](https://docs.typesafe.ai/api) for question and answer shapes.

This integration supplies backend API sessions. Tools/skills, conversational context delivery, native resume, remote hosts, and OTP functionality are not exposed by this slice.

The provider adapter owns Credential Manager access in `src-tauri/src/runtime/providers/jev/credentials.rs` and injects its reader into the portable JEV runtime. Settings use `load_jev_credential_status`, `save_jev_api_key`, and `remove_jev_api_key`; status exposes only `saved` and `environmentAvailable` booleans. Saving or removing a key takes effect at the next invocation without restarting.
