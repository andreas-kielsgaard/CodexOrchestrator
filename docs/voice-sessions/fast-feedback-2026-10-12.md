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
## Measured scenarios

These are synthetic text inputs through the real application/JEV/model pipeline. Timings start before the native text submission and end at the persisted publication; they include inspector submission overhead, but exclude speech capture/transcription and TTS. Samples were not a controlled benchmark, and some ran while compiling.

| Scenario | JEV/release | First relevant feedback | Observed handling |
| --- | ---: | ---: | --- |
| Existing baseline lunch request | — | 22.84 s | Summary acknowledgement; old digest had a writer conflict |
| Same request, Luna low + Fast | 2.11 s | 11.24 s | Summary, router and digest ran in parallel |
| Explain an ingredient substitution | 2.00 s | 13.40 s | Direct output; no new assignment |
| Clarify the salad-bowl reference | 2.10 s | 13.25 s | Direct output; consistent with reported dairy-free outcome |
| Status while the walk task continued | 4.81 s | 17.62 s | Direct output while worker remained active |
| Explain lunches and amend walking requirements together | 3.52 s | 15.00 s | Work path; reused walking task at revision 2 |
| Ambiguous “the other one” | 3.41 s | 16.52 s | Conditional interpretation; no new task. Excluded an unrelated earlier packing publication |
| Vague follow-up clarification | 2.13 s | 82.73 s | Long gap inside native turn before first tool; later input steered the same turn |
| Forced JEV reaction misclassification of a work amendment | 1.92 s | 9.88 s reroute; 12.60 s acknowledgement | Output used route_input; same lunch worker/task advanced to revision 3 |
| Speech to Alex / bare “Okay” after prompt correction | — | No publication | Unaddressed; no release or work dispatch |

The original no-stove amendment reused the lunch task at revision 2. An unrelated rainy-walk request created a separate worker/task. The late dairy-free amendment retained that lunch identity and did not create a duplicate task. The deliberately overridden reaction prompt was restored afterward.

## Verification and limits

- Runtime evidence confirms GPT-6 Luna, low reasoning, priority service on response stages, and GPT-6.1 Sol, medium reasoning, default service on routing.
- 24 voice backend tests, 31 portable Codex adapter/options tests, the product delivery reasoning-default regression test, and two desktop screen tests passed. Android application/test APK builds and lint passed; installed interruption probe passed.
- The published desktop was rebuilt on main, launched, and exercised with direct reactions and the fault-injection scenario. Google handoff was rebound after instrumentation; the system assistant shortcut opened Orchid AssistantActivity and connected over Tailscale.
- Live phone capture exposed spurious responses to a bare acknowledgement. Relevance/reaction/completion prompts now distinguish it from a request or an answer to an outstanding question. The corrected live “Okay” test produced no release/publication. This does not prove all ambient input will classify correctly.
- Ambiguous references can still be assumed instead of clarified. Direct routing removes model hops, but native model latency remains variable; the 82.73 s outlier is unresolved. Raw microphone noise no longer stops playback, but acoustic interruption timing has not been independently measured with deliberate user speech.

Evidence: ignored .dev/voice-fast-*.json and build/test logs in this checkout. Source commits: 31382104, dbef1947, e924dfab, a5c97ff8. Final executable hash and completion playback will be appended after publication.

Final publication: desktop SHA256 07C77C0465201AE98F5DD089673D75A409C6CF69CB299BC7A0E59ECF4B601AF9; Android APK SHA256 407DF3D3CD70D1CD30243EA3FCEFE83479332FF9EA34CF6E7E853CA2019F8735. The release build was produced from main a5c97ff8 and launched as PID 31000 with inspector port 9282. An additional 23 preparation/queue tests passed, including explicit reasoning selections and target handling.

Completion announcement: output session 073a20f4-7de4-4671-bd98-14bbe99d0441 published utterance 9f235d58-7e99-4c5b-81ab-ab229dd319ae through voice_session.publish. The connected phone reported completed playback, 11,761 ms. The microphone had reached its normal inactivity timeout and stayed off while the connection remained available for playback. Final live desktop controls showed Luna/low/Fast enabled for response and GPT-6.1 Sol/medium/Fast disabled for routing.
