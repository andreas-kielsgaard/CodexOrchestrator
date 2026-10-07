# Voice Sessions: implementation shape

Status: implemented on `feature/reasoned-voice-workflow`; live validation in progress. See `docs/voice-sessions/validation.md` for verified outcomes.

## Functional target

The user starts a conversation on the S21 before leaving, locks the phone, and engages handsfree with Orchid on the laptop over mobile data. Speech can start work, inspect existing work, ask specific questions, and change direction. The system briefly acknowledges commands, announces useful information when its agent judges it ready, and explains it when invited. Speaking during an explanation pauses playback and gives the reasoning agent the new input and playback context. The agent can investigate, answer, change work, and offer to continue relevant unfinished material.

The initial version uses distributed agents: the main conversation hands relevant work to independent Agent Sessions so tasks continue while it reasons about new input. Success means a deployed, tested laptop application and Android APK supporting that flow with concurrent delegated work. A local text demo, microphone recorder, or native-voice feasibility result alone does not satisfy the slice.

## Proposed choices

- Add a dedicated **Voice Sessions** tab and a focused backend service. One active phone conversation has an ordinary persistent main session and independently running delegated sessions.
- Use a native Kotlin Android companion in this repository. Its foreground service owns recording, playback, connection, and notification. The phone contains no work orchestration or intent classifier.
- Connect phone and laptop with Tailscale. Use an authenticated WebSocket endpoint bound to the configured laptop Tailscale address; keep the existing debug interfaces separate.
- The main reasoning agent interprets input and delegates relevant execution, investigation, orchestration, or report preparation. Delegated agents can hand off further work through the same application tools when appropriate. The exact role split remains prompt-defined; a fixed three-agent topology is not required.
- Provide product-owned `orchid_control` and `voice_session` managed MCP tools through the existing Harness. The former exposes existing application operations; the latter lets the conversation agent publish announcements and explanations. Avoid making the slice depend on a new external OTP loader.
- Native Codex audio required API-key authentication in the probe. The user declined OpenAI API use. Use local faster-whisper transcription and offline Android text-to-speech; reasoning retains the existing selected Codex or Claude provider.

## Pending decisions and assumptions

- The next departure time is unknown. The delivery order below brings remote connection and voice feasibility forward so problems are discovered before desktop UI polish.
- Claude Code's documented dictation uses a local microphone and does not expose the required phone audio bridge. Desktop/web sign-in does not establish Claude Code authentication; the existing Orchid provider checks its own setup.
- Tailscale is signed in on both devices. The actual S21 app has connected over mobile data; outdoor continuity remains to be tested.
- The first walking test should use the user's actual audio setup. No headset or Bluetooth route has been assumed; the S21 microphone/speaker is the initial test baseline.
- One manually started conversation, one laptop, and one phone are sufficient. Laptop remains awake on its current Wi-Fi with Orchid running. Basic reconnect restores conversation context; disconnected microphone audio is discarded rather than silently queued for later execution.

## Prescribed behaviour

- The feature has its own tab; reuse workflow capabilities only when they fit.
- Distributed agents and asynchronous handoffs are required in the initial slice. Delegated work continues while the main conversation processes new input.
- No required three-agent split or workflow-instance observation restriction.
- Every completed spoken input reaches reasoning. No keyword routing between clarification, investigation, steering, and cancellation.
- Agents determine when there is useful information and what to announce. Prompts can tune this behaviour; completion events do not automatically become spoken reports.
- Announcements precede full reports; user invitation permits the relevant explanation.
- Commands receive accurate brief acknowledgements. Interruptions can require lookup work; the agent should offer to continue relevant unfinished output.
- Preserve normal Agent Session visibility. The laptop is the application and history owner; the phone is a companion.

## Source findings

| Current responsibility | Evidence and implication |
| --- | --- |
| Product navigation and composition | `src/application/productNavigation.ts`, `src/app/App.tsx`, `src/bootstrap/productApplicationComposition.ts`: add a typed destination and feature client; keep tab-specific state out of the application shell. |
| Ordinary profiled sessions and continuation | `src-tauri/src/agent_sessions/application/{mod,direct_user,creation,queue,interactions,lifecycle}.rs`: reuse creation, queued prompt delivery, steering, cancellation, runtime requests, and durable history. |
| Trusted managed tools | `src-tauri/src/harness_engine/{service,exposure,proxy}.rs`: existing session-scoped upstream registration, tool exposure, and injected session/invocation identity can serve product tools without UI automation. |
| Tool catalogue composition | `src-tauri/src/active_app/{sessions,execution_configuration}.rs` accepts product-tool maps independently of OTP. Add this feature's tool declarations to that existing composition. |
| OTP workflow coupling | `src-tauri/src/otp_api/handles.rs` provides node definitions, node sessions, and outputs; `src-tauri/src/otp_host/mcp.rs` holds a `WorkflowExecutionService`. Calling those tools from a standalone voice session is not currently a generic application-control interface. |
| Notifications | `src-tauri/src/active_app/session_notifications.rs` fans durable notifications out to product observers. Adapt it to feed voice observations without running inference inside the callback. |
| Build notifications | `src/application/completionNotifications.ts` combines turn/build completion for a frontend sound. Reuse backend worktree-build observations for the new service rather than making a mounted React hook its event source. Trace the build publisher when implementing that adapter. |
| Codex runtime owner | `src-tauri/src/runtime/providers/codex/app_server/mod.rs` re-exports `crates/orchid-engine/src/providers/codex/app_server/`. Its connection and process lifecycle are invocation-scoped; a long-lived audio session needs an explicit owner instead of extending an invocation indefinitely. |
| Storage | `src-tauri/src/storage.rs` composes focused schemas. Add feature state there; keep agent histories in their existing repository. |
| Device/tooling | Wireless ADB shell and transfer were verified without USB on S21 `R3CRA0BAYXD`, Android 15/API 35. Java 21, Android SDK/build-tools 36, Android Studio, and a Gradle cache are present. No Android project or voice integration currently exists in this checkout. |

## Ownership and concrete change surface

### Desktop feature

Create `src/application/voiceSessions/{contracts.ts,index.ts}` for typed settings, conversation state, utterances, and client operations. Create `src/infrastructure/voiceSessions/tauriVoiceSessionClient.ts` for native commands and updates. Create `src/features/voiceSessions/{VoiceSessionsScreen.tsx,voiceSessions.css,index.ts}`.

The tab shows connection/listening state, the main session, delegated work with links to its ordinary sessions, pending topics, recent spoken exchanges, and concise activity. Setup chooses an existing Capability Profile, shows the laptop endpoint and access token, and offers start/stop and editable conversation instructions. Reuse profile selectors and the embedded Agent Session view for deeper inspection; do not build another transcript renderer.

Adapt `productNavigation.ts`, `App.tsx`, and `productApplicationComposition.ts` to add the destination, tab, and injected client. The tab does not own audio or background orchestration, so changing tabs cannot end the phone conversation.

### Conversation and application access

Create `src-tauri/src/voice_sessions/{mod.rs,domain.rs,application.rs,delegation.rs,repository.rs,transport.rs,observations.rs,gateway.rs,speech.rs,mcp.rs}`. Split only when the responsibility exists; these are ownership targets, not a requirement for empty scaffolding.

- `VoiceSessionApplication`: active conversation, reasoning deliveries, publication of spoken material, observation wakeups, and phone state. Coordinates existing services rather than executing worker commands itself. No parent invocation waits synchronously for delegated agents to finish.
- `VoiceDelegationService` in `delegation.rs`: accepts an agent-chosen handoff, creates or addresses an ordinary recipient session, delivers the task/context, and returns the accepted session/invocation references. Records the conversation, originating request, delegating session, recipient, task brief, and intended return session. Results return as observations through normal session notifications; runtime status and transcripts retain their existing owners. This is handoff coordination, not another scheduler.
- `VoiceSessionRepository`: conversation ID, main session ID, selected profile, instructions, delegation relationships, pending topics, published utterances, source references, and acknowledged playback progress. Agent histories and worker status remain in Agent Sessions.
- `VoiceGateway`: authenticated phone connection and audio/control framing. Uses Axum's WebSocket support and existing Tokio dependencies. Runs only while enabled; the application retains its lifetime owner and shuts it down explicitly.
- `SpeechSession`: the selected speech implementation, kept independent of orchestration and UI. Start with the operations actually required: input audio, transcript events, speak supplied material, interrupt, and close.
- `VoiceSessionMcp`: trusted conversation-agent tools to announce availability and speak an explanation. Persist publication before sending audio; a generated or sent utterance is not considered played.
- `VoiceObservationAdapter`: converts existing session/build facts into compact referenced observations. Maintains a pending batch while the reasoning session is busy and uses existing queued delivery after completion. Ignore coordinator-generated activity as an automatic wake source to avoid self-triggering loops. Human input reaches the running reasoning invocation through steering when available, rather than waiting behind worker observations.

Expose an asynchronous `delegate_work` tool through `VoiceSessionMcp`, alongside announcement and explanation tools. It accepts a task brief, relevant context references, and a new or existing recipient; creation uses the existing profile/target contracts. It returns delivery acceptance immediately, not a completed answer. Main and delegated sessions can use it, with trusted caller identity establishing the return relationship. Agents decide when to delegate, how to divide work, whether more delegation is useful, and how to respond to steering. Application code owns actual session dispatch and result delivery.

The main agent can issue a handoff and finish its turn, leaving the worker active and itself available for new input. A specific follow-up question can start another investigation while earlier tasks continue. Editing prompts or ending phone playback does not implicitly cancel worker invocations; changes to work use the existing explicit steering/cancellation operations.

Create `src-tauri/src/orchid_control/{mod.rs,service.rs,mcp.rs}` for reusable application operations. Tools should cover session discovery/history, available workspace/profile discovery, ordinary session creation, sending work, steering, cancellation, and answering a pinned runtime request. Reuse `AgentSessionApplication`, repository catalog, profile services, and existing interaction receipts. Observation is available across the application, without requiring workflow membership. Original user speech and application observations retain distinct provenance.

Register product tools in the existing product-tool catalogue and managed MCP registry. Require the Harness-supplied caller context, not session IDs claimed as caller identity in tool arguments. The conversation agent receives these tools in its selected profile; other consumers can select the reusable application tools normally. Do not add another hard-coded OTP package or general external-package framework for this slice.

Create a focused `src-tauri/src/active_app/voice_sessions.rs` composition helper. Adapt `active_app.rs`, `active_app/sessions.rs`, `active_app/session_notifications.rs`, and `storage.rs` for startup, tool registration, notification attachment, native transport, and schema composition. Keep `lib.rs` to module declaration changes. Feed build-completion facts from the backend publisher directly.

### Local speech

`voice_sessions/speech.rs` owns one lazy Python transcription worker. `resources/voice-sessions/transcribe.py` decodes completed 16 kHz mono WAV input and runs faster-whisper on CPU. `scripts/android/setup-voice.ps1` installs its environment and model. Android owns speech detection, interruption, and offline text-to-speech. No speech API key is required.

### Android companion

Create `apps/orchid-android/` as a standalone Gradle project with wrapper, manifest, and a small Kotlin app. Use OkHttp for WebSocket framing; Android's `AudioRecord`/`AudioTrack` and available acoustic echo cancellation for audio. Pin compatible dependencies during setup rather than assuming the cached Gradle version fits the Android plugin.

Suggested classes: `MainActivity`, `ConversationService`, `OrchidConnection`, `AudioCapture`, `SpeechPlayer`, and `ConnectionSettings`. The activity handles endpoint/token setup, permission requests, start/end, status, and a text fallback. The service owns microphone/media playback foreground notification, active connection, audio routing, and a bounded wake lock while the conversation runs.

Start recording while visible after microphone permission is granted; support screen-off operation during the active session. Audio focus and the actual S21 playback route belong here. Use the selected speech service's speech-start event for interruption if it responds promptly; otherwise use a maintained speech activity detector, not a custom intent classifier. Avoid collecting speaker playback as new user input by testing acoustic echo cancellation and the actual audio route.

Create `docs/voice-sessions/protocol.md` for the small shared wire contract: versioned control messages, ordered audio frames, final/partial transcripts, utterance IDs, playback progress, interrupt, and connection state. The phone pauses playback and flushes queued audio immediately on detected user speech, then reports its position. This is an audio mechanism; only reasoning decides what that input means. Discard obsolete output after interruption rather than allowing it to play after a clarification.

Create `resources/voice-sessions/instructions.md` as the bundled default conversation prompt, with an editable override owned by the tab. Deliver current instructions as application context on reasoning invocations, preserving the user's speech separately. Keep it limited to the agreed behaviour and available tools. No report-readiness rule engine or phrase-matching command vocabulary.

Add `scripts/android/build.ps1` and `scripts/android/deploy-s21.ps1` plus discoverable root commands. Use the existing phone helpers to discover and verify the S21, not a remembered IP/port. Generated Gradle/build outputs and APKs are ignored; source, wrapper, and build instructions are committed. The unrelated repaired phone helpers remain outside this feature's Git changes.

## End-to-end behaviour

1. The user enables Voice Sessions on the laptop and starts the Android conversation. The desktop creates or reuses its ordinary coordinator session; the phone connects using the configured private endpoint/token.
2. Captured speech produces a transcript. The service supplies it to the coordinator with relevant playback and pending-topic context. Partial transcription can be displayed; actionable delivery uses completed speech input.
3. The main agent acknowledges through an explicit voice tool and hands relevant work to other sessions. Handoff returns accepted delivery references so the conversation remains available while recipients work independently. Starting work and confirming its result remain different facts.
4. Delegated agents can inspect application state, execute tasks, or delegate further work. Their activity/results return to the intended reasoning session as referenced observations. Reasoning may read additional history, delegate report preparation, remain quiet, or publish an availability announcement and pending material. Existing sessions outside the delegation relationships remain discoverable through application tools.
5. A spoken invitation reaches reasoning like any other input. It selects the relevant material, investigates further if needed, and publishes a concise explanation. Native voice must not automatically read worker final messages.
6. New speech during playback pauses audio locally. Reasoning receives the question and last confirmed playback position. It decides what to investigate or change and whether to offer continuation. Track played segments or milliseconds, without claiming word-perfect knowledge of what was heard.
7. Reconnect preserves the ordinary conversation, delegation relationships, and pending material. Audio is not automatically replayed. End stops recording/playback and closes the speech session while retaining conversation history; delegated work has its own session lifetime.

Approvals already required by an agent's runtime remain requests that the reasoning agent can present and the user can answer in speech. Reuse request IDs and response encoding; do not bypass approval settings. If a selected runtime requires a desktop-only interaction, report that concrete limitation rather than claiming handsfree success.

## Delivery order and verification

1. **Speech feasibility:** one focused local probe of Codex 0.159.0 with recorded input and captured output. Record the actual transport/version. Native access required API-key authentication; the selected path is local transcription and offline Android speech. Neither schema presence nor synthetic tests prove live microphone or playback behavior.
2. **Remote skeleton:** add a minimal desktop tab/service and Android APK. Connect over Tailscale with phone Wi-Fi disabled and laptop Wi-Fi unchanged. Round-trip text and events; authenticate the intended Orchid instance. Confirm the phone endpoint works without ADB.
3. **Distributed reasoning and tools:** attach the main session, expose managed application/voice tools, and delegate real work to two independently running sessions. While a task remains active, process new user input and hand off a separate investigation. Verify result delivery, steering/cancellation, and runtime-request responses through existing durable records. Exercise a delegated agent handing further work to another session, and observe an already-existing unrelated session too.
4. **Handsfree integration:** record/play audio with phone locked. Exercise command acknowledgement, agent-chosen availability announcement, invited explanation, a specific question requiring lookup, and a change of direction. Verify interrupted output does not resume unsolicited. Test the user's actual audio setup.
5. **Walking acceptance:** run at least a 15-minute outdoor mobile-data session with the laptop staying on its Wi-Fi. Start delegated work, then ask a separate question while it continues; verify both results return to the conversation. Include an idle period followed by proactive availability, interrupted reporting, and a brief network reconnect. Require no phone touches after start apart from deliberate ending. Observe delays and record remaining limitations honestly.

Run focused backend tests for asynchronous handoff/return correlation, concurrent worker lifetimes while the main session handles input, publication/playback state, busy observation batching versus human steering, trusted tool context, and endpoint authentication. Use engine fixtures for audio protocol changes if native voice is selected. Add navigation/client checks for the new tab and Android lifecycle/audio checks where they verify actual behaviour. Run frontend build/lint and repository-approved Rust checks; use Gradle to build the APK. Native/provider and real-phone evidence remains mandatory after tests pass.

Create `docs/voice-sessions/README.md` for setup/start/end/build/deploy and `docs/voice-sessions/validation.md` for actual versions, deployed binaries/APK, exact app instance/storage, phone identity, mobile-data checks, observations, and unverified limits. Launch and inspect the dedicated tab with the Orchid application CLI; use the phone skill for APK deployment, UI/log inspection, and screenshots. User speech is needed for the final audible walking test.

## Scope kept small

Retain Workflow recipes, engine, and UI. Reuse Agent Sessions, Harness mediation, process supervision, existing HTTP dependencies, and notification producers directly. Use generic Session Events only if a concrete delivery needs their addressing/provenance; no artificial workflow recipe or node membership is required.

No broad application-shell rewrite, generic observer framework, external OTP platform, additional cloud backend, public tunnel, phone-side work execution, offline command queue, multiple-phone support, automatic boot listening, provider-switching UI, or precise word-level playback alignment. No existing product files are slated for deletion. If RPC code is extracted, remove the moved implementation from its former owners and migrate every existing caller in that same change.

## External references

- [Codex app-server](https://learn.chatgpt.com/docs/app-server): stable protocol and experimental opt-in. The local 0.159.0 generated schema exposes `thread/realtime/start`, `appendAudio`, `appendText`, `appendSpeech`, transcript/audio notifications, and client-managed handoffs; these remain untested here.
- [OpenAI live transcription](https://developers.openai.com/api/docs/guides/realtime-transcription) and [text-to-speech](https://developers.openai.com/api/docs/guides/text-to-speech): independent speech layer if native Codex is unsuitable.
- [Axum WebSockets](https://docs.rs/axum/latest/axum/extract/ws/index.html): enable the existing dependency's `ws` feature rather than implementing WebSocket framing.
- [Tailscale Android](https://tailscale.com/docs/install/android) and [Windows](https://tailscale.com/docs/install/windows): private device connectivity.
- [Android foreground-service types](https://developer.android.com/develop/background-work/services/fgs/service-types): microphone and media playback operation while backgrounded.
- [Android AcousticEchoCanceler](https://developer.android.com/reference/android/media/audiofx/AcousticEchoCanceler): availability and attachment to recording sessions; actual S21 behaviour needs testing.

Android capture uses maintained WebRTC voice activity detection. Completed utterances are transcribed locally; partial transcription is outside this slice.
