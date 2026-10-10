# Voice coordination prototype verification

Implemented from the [change proposal](../architecture/voice-agent-coordination-change-proposal.md). No new scheduler, database tables, crash recovery, or phone-management tooling.

## Checks

- Voice backend: 21 focused tests passed, including retained failed JEV context, task revisions, parent-invocation revision ownership, and invocation-specific returns.
- Shared delivery: the focused Agent Session test passed for active steering and an undelivered correction at turn completion.
- Voice screen: two tests passed; frontend and Android builds passed. The updated APK was installed on S21 `R3CRA0BAYXD`.
- Desktop screenshot review showed named workers, pending delivery, and the current JEV error. S21 screenshots showed named task revisions and expandable task details; application context was collapsed in history.

## Live coordination

The existing conversation was preserved. Downstream input was submitted directly to its router through the application's normal Agent Session command because JEV credentials were unavailable in the newly launched process. This verifies downstream orchestration, not successful voice classification.

The router created separate Voice capture audit and Weekend picnic tasks with independent workers and assignment IDs. Both workers called `report_outcome`. A combined update preserved each task ID and worker: the audit narrowed to release ownership and a concurrency risk; the picnic became vegan for two. Conversational status was routed through digest/output while the audit continued. Intermediate worker messages reached digest with source references. Older outcomes remained identifiable by revision.

The test exposed a steering rejection, `no active turn to steer`, around turn completion. The router recovered the audit through another assignment. The implementation was then corrected to retain rejected/expired input for next-turn delivery once that invocation ends, and to avoid delivering an older rejected amendment after its replacement was delivered. This application correction requires a rebuilt executable for live verification.

The initial JEV request failed due to missing `TYPESAFE_API_KEY`. Diagnostics showed that error and retained the opening beyond subsequent input. Successful credential-backed reevaluation has not been verified in this pass. The key must be supplied by the application launcher; it is not stored in voice state.

Saved stage prompts matched previous shipped defaults and were updated. Saved shared preferences matched the initial shipped instructions and were replaced with the current routing preferences. Custom prompt matching was not assumed.

Evidence: `.dev/voice-coordination/` and `.dev/voice-activation-final/coordination-*.png`. These are local ignored review artifacts. Physical microphone quality, headset/watch activation, and spoken wake detection were not retested by this coordination change.
