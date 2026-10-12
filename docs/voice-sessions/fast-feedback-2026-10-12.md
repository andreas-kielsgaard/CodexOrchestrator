# Fast voice feedback experiment

## Runtime shape

Accepted work reaches the summary, router and digest through the existing observation dispatcher. Summary publishes the immediate receipt; routing starts independently and delegates work. Response stages default to GPT-6 Luna low with Fast mode; the router and new workers default to GPT-6.1 Sol medium with standard service. Existing external chats retain their configuration. Desktop Voice models controls apply to newly created conversation agents.

JEV separately classifies relevance, completion, continuation and reaction. The recent spoken output, pending topics, tasks and annotated activity are supplied as context. A completed release containing only reaction requests goes directly to output, avoiding summary/router/digest hops. Mixed input retains the parallel work path. Output can forward a misclassified reaction's original accepted requestId to the stronger router using route_input, without reconstructing user speech.

Raw microphone activity only updates speaking telemetry. A current addressed JEV answer plus a completed sentence may interrupt playback, once per sentence revision. Sticky acceptance still preserves input, but does not authorize interruption. Reconnection classification history is remembered without interruption. During playback, the first completed sentence triggers JEV instead of waiting for the normal sentence threshold.

## Ownership and changes

- Voice models and stage creation: voice_sessions/models.rs, application.rs, pipeline.rs and delegation.rs; configured session creation stays in agent_sessions/application/product_sessions.rs.
- Native Fast mode: portable Codex options and app-server adapter, preserving absent-field serialization for existing profiles.
- JEV classification and direct dispatch: prefilter.rs and reaction-instructions.md; fallback tool is bound to the output session in orchid_control.
- Phone interruption: AddressedSpeechInterruption.kt and ConversationService.kt; remove VAD interruption and speaking-based rejection of output.
- Desktop: Voice Sessions model controls and editable reaction prompt.

No crash/reboot recovery or extra orchestration layer is introduced. Remaining uncertainties are classification quality with ambiguous/mixed input and end-to-end latency under concurrent reasoning. Runtime tests must distinguish JEV latency, publication latency and audible playback.

## Validation

Baseline acknowledgement: 22.837 seconds from submitted text. The existing digest failed with an active-writer conflict, so this is a single observed sample, not a clean model comparison. New-build scenario timings and playback evidence will be recorded below.
