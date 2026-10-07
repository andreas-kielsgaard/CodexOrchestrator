# Assistant transcript view

Keep the four existing activation buttons above a scrolling transcript view. Remove connection/capture diagnostics, explanations, and the listening-stop reason from this screen. Voice in the main app retains those details.

Android `DetectedInput` keeps the latest 20 transcript blocks, before JEV relevance filtering. `ConversationService` appends incoming `transcript` events and applies later `input_classification` events by input ID. `prefilter.rs` emits that event for every relevance decision, including unaddressed speech. `AssistantActivity` renders colored spans, follows new input at the bottom, and preserves manual scrolling. Classification-only changes repaint existing text.

Pending input uses the normal text color. Requests use the existing green accent, task context uses blue, and unaddressed input uses muted gray. No transcript is hidden based on its classification. The classification event does not renew inactivity; the existing accepted-input event retains that authority.
