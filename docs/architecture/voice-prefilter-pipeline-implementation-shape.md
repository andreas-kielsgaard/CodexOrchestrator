# Voice prefilter and reasoning pipeline

Implementation branch: `feature/reasoned-voice-workflow`, based on main `9d8c6eda`.

## Agent-made choices

Use the selected voice Capability Profile for four ordinary persistent reasoning sessions: input summarization, routing, digest, and output. Keep the existing main session as the router; retain all stages across stop/resume and phone reconnects. An explicit New conversation starts a new context. Record original accepted input, source, time, stage results, and delegation references in the conversation so history survives application restart.

Let the input summarizer publish its brief receipt acknowledgment before summarizing. This meets the immediate acknowledgment requirement without waiting for the digest/output chain. Record that publication for digest context. Other spoken material comes from the output session after a digest request.

Use existing Agent Session delivery, steering, terminal notifications, profiled sessions, MCP tools, and the typed JEV facade. JEV calls are independent: supply chronological speech text, timestamps, accepted speech, and task references. Keep queue flags and previous classification results out of evaluator context. Relevance and completion are separate Choice evaluations. Preserve completed evaluations through ordinary invocation storage.

## Approximate choices

Start with approximately five-second recording segments, retaining the current WebRTC VAD and local Whisper transcription. Segmentation does not end a request. Publish speech-activity boundaries separately so completion cannot dispatch during continued recording or while recorded segments await transcription/filtering. JEV decides semantic completion; do not add trigger-phrase matching.

## Prescribed behavior

Filter every transcribed block before any reasoning session receives it. Ignore unaddressed speech. Buffer accepted input and play one listening cue when the buffer opens. Release when JEV judges the current sentence window complete, with no outstanding JEV evaluation; play a processing cue, then deliver input to summarization and routing independently and to digest context.

The router retains context and dated task handoffs, chooses existing or new workers, and delegates research/execution rather than doing it itself. Summaries, routing feedback, worker results, and user input reach the ongoing digest, steering its active turn when supported. Digest decides whether now is a useful time to speak and forwards a brief reason and relevant references to output. Output can inspect retained context and sessions on demand. Its instructions favor conversational headlines and availability announcements, with more detail when the user asks. These are agent behaviors, not output templates or hardcoded completion messages.

## Ownership and change surface

- Retain `voice_sessions/application.rs` for lifecycle, persisted state, and explicit publication; remove its direct raw-fragment-to-main dispatch.
- Create `voice_sessions/prefilter.rs` for typed JEV questions, accepted-input assembly, and the completion gate. Keep transcription and JEV provider transport in their existing owners.
- Create `voice_sessions/pipeline.rs` for stage creation, role instructions, shared dispatch/steering, annotated activity, and digest-to-output requests. Use the existing delegation implementation for existing and new workers.
- Adapt `domain.rs` and `repository.rs` to persist stage identities, input decisions/buffer, dated delegations, and activity. Runtime capture/transcription counts remain transient. Existing conversations initialize stages on resume.
- Adapt `observations.rs` to route summary and worker outcomes to their proper recipients and digest; use active-turn steering rather than waiting for idle digest sessions.
- Extend the existing `voice_session` MCP package with context inspection, progress feedback, and output requests. Keep trusted caller validation and existing application session tools.
- Replace the combined input/conversation instructions with concise stage-specific resources. Inject them through existing product context rather than inventing another skill runtime.
- Persist editable copies of the stage prompts and shared tool discovery guidance in voice settings; expose a selector, editor, and reset action in desktop Voice Sessions. Keep shipped resources as defaults and typed JEV choices/publication permissions in application code.
- Adapt `gateway.rs` and Android `AudioCapture`/`ConversationService` for continuous PCM transport and activity boundaries. Add a small media-volume cue player; retain microphone routing, speech playback, and audio focus.
- Adapt desktop and phone session history to expose the reasoning stages, filter decisions, and input buffer in separate sections. Preserve existing authenticated history reads.

## Verification

Listening inactivity is owned by Android `ListeningInactivity`, using monotonic elapsed time and a configurable 60-second default. `prefilter.rs` sends `relevant_input` after each accepted relevance decision. `ConversationService` renews the window only for that event or explicit activation, releases capture on expiry, and retains reply playback. `ConnectionSettings` and phone Settings own the duration; Voice displays the countdown and stop reason. No reasoning prompt or completion rule changes are needed.

Test relevance/context/completion assembly, immediate dispatch during capture, no dispatch against unevaluated sentence revisions, single cues, stage publication authority, durable restart state, and observation/steering delivery. Run focused Rust and frontend checks, Android build/lint, then exercise real JEV and reasoning sessions in an isolated Orchid instance. Verify phone cues and recording boundaries where the connected device permits; distinguish simulated input from live microphone and hardware-button evidence.

## Continuous input revision (2026-10-08)

`continuous_audio.rs` owns the rolling PCM window; `transcript.rs` reconciles sentence identities and revisions. Remove segmented finalization and the separate preview transcriber. `query_schedule.rs` triggers JEV after the configurable sentence threshold or voice pause, using the current revisable text. `prefilter.rs` evaluates each sentence in the time window, preserves accepted relevance, and separately identifies continuation of recently delivered input. Deliveries retain request identity and continuation links. Android keeps listening for the configurable follow-up duration, resumes accumulation on relevant continuation, and requires explicit activation after timeout. Phone Settings owns the four numeric controls; desktop Voice prompts adds continuation instructions.
