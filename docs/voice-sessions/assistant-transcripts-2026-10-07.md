# Assistant transcript view

Keep the four existing activation buttons above a scrolling transcript view. Remove connection/capture diagnostics, explanations, and the listening-stop reason from this screen. Voice in the main app retains those details.

Android `DetectedInput` keeps the latest 20 transcript blocks, before JEV relevance filtering. `ConversationService` appends incoming `transcript` events and applies later `input_classification` events by input ID. `prefilter.rs` emits that event for every relevance decision, including unaddressed speech. `AssistantActivity` renders colored spans, follows new input at the bottom, and preserves manual scrolling. Classification-only changes repaint existing text.

Pending input uses the normal text color. Requests use the existing green accent, task context uses blue, and unaddressed input uses muted gray. No transcript is hidden based on its classification. The classification event does not renew inactivity; the existing accepted-input event retains that authority.

During speech, `AudioCapture` sends a growing WAV snapshot approximately once per second with the same ID as its eventual completed segment. The gateway keeps only the latest queued snapshot and transcribes drafts on a separate local worker, so drafts cannot delay final-input transcription. `transcript_preview` updates the visible neutral text; `transcript` finalizes it. Late drafts cannot overwrite finalized text or its classification. The view refreshes every 250 ms. Transcription and network latency still apply.

Drafts never enter JEV, reasoning sessions, or the inactivity clock. Completed blocks retain the existing relevance and completion gates. Connection hello restores recent classification colors after reconnection. Audio snapshots are not retained.

Empty final transcriptions clear any matching draft without adding blank rows or consuming the 20-block limit. A bounded set of completed IDs rejects late drafts for those empty segments. Scrolling follows layout changes after text replacement.

## Verification

Source `b25d26e0` was merged to main and built in release mode. Main executable SHA256: `F951ED76D6CF5DFFEB5A3958AD54BF2AAD55D851A5F9DFE3F9193362068F2BEA`. The running main instance retained the existing conversation and all reasoning-session identities.

A controlled WAV fixture through the rebuilt gateway produced its first draft 2.99 seconds after preview upload, before final audio was submitted at 4.26 seconds. Final transcript arrived at 7.24 seconds and JEV classification at 8.01 seconds. Three draft updates arrived. Preview-only input left the input-decision count unchanged and final audio queue empty. The completed fixture was classified unaddressed. This measures the gateway/recognizer, not a user's microphone-to-screen latency.

Eleven voice Rust tests, frontend typecheck/build, Android build and lint passed. Installed-phone instrumentation verified draft replacement, final text preservation against late drafts, all three classification colors, empty-segment cleanup, and the rolling limit. The actual assistant screen was inspected for its four controls, scrolling text, and classification colors. The phone's actual unsilenced built-in microphone remained selected. User-perceived live timing remains a separate check.

Installed Android APK SHA256: `B0CB68C21E8BBAF3061DCA9AF6D234DEE3272C9E10D3123741469E67A70B3D79`. Private timing, phone-state, and screenshot evidence is under `.dev/voice-pipeline-main/`.
