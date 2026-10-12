# Voice extension workflow prototype

Branch: `codex/voice-extension-workflow`, from the visual exploration.

## First slice

Make the existing voice flow inspectable and editable as an extension-owned workflow. Reuse the desktop graph presentation. Selecting JEV or an agent edits its real prompts; selecting a connection explains its trigger. A direct-reaction switch allows comparison with work routing. Activity shows actual recorded destinations and delivery state, rather than animating an assumed execution order. Runtime connection controls remain separate from the graph and delegated work.

The topology is supplied by Voice in this slice. Arbitrary node creation, a plugin registry, a second execution engine and runtime recovery are outside this experiment. Desktop compositions remain desktop views; the phone viewer is only a review surface.

Node notification editing adds one switch: sound when an agent turn finishes. Ordinary recipe nodes retain an enabled default, stored with their instance recipe. Voice nodes default to disabled, including JEV and delegated workers. The extension/workflow owner resolves the preference for a bound session; the Tauri session update carries it to the existing completion-sound listener. Terminal updates, routing and deliberate voice publication remain active. No notification framework or separate preference store is introduced.

## Ownership

- Add `workflows/extensions.rs` and `application/extensionWorkflows/contracts.ts` for small typed presentation contracts: function, capability, agent and external-session nodes; event connections; recorded routes. No scheduler or persistence owner belongs here.
- Add `voice_sessions/workflow.rs` for the Voice definition and session bindings. Its connection table also resolves release, summary, routing, progress and output-request destinations. Keep JEV semantics in `prefilter.rs`, dispatch/steering in `pipeline.rs`, provider observation in `observations.rs`, and audio lifetime in the existing gateway/speech owners.
- Extend existing voice settings with the direct-reaction choice. Existing voice persistence and configuration commands own saving. Prompts and model groups remain their existing values, with no competing workflow copy. Model changes still apply to new conversations.
- Add route evidence to recorded activity at enqueue time. Destination session IDs record intended recipients; observation receipts retain accepted/queued delivery facts. A diagram edge does not imply completion or playback.
- Add a reusable `features/extensionWorkflows` canvas using `features/workflowGraph`. Keep Voice-specific inspectors and activity projection in `features/voiceSessions`.
- Split the existing voice setup form from `VoiceSessionsScreen`. Retain conversation/status behavior in an Activity view and introduce Workflow / Activity / Setup sections. The screen owns reads and draft settings; the workflow inspector consumes those same drafts and the same save operation.

Ordinary Workflow recipes, Session Events, OTP and agent-session execution keep their current ownership. A future heterogeneous workflow compiler can consume the extension contract once execution requirements are established; this slice does not advertise those nodes as executable ordinary recipe nodes.

## Real-time behavior

Continuous transcript revisions and JEV reevaluation remain active. The request buffer releases completed accepted input using the selected reaction route. Work fans out to receipt, router and digest; reaction-only input goes to output when enabled. Summary and worker observations can steer existing sessions. Digest requests output with a reason and references. Output can reroute a misclassified reaction to the router. The UI shows concurrent stage status, pending input and assignment revisions from the existing runtime.

Changing prompts or the route switch requires stopping and saving the connection, matching the existing configuration boundary. Pausing/stopping the connection is not cancellation of delegated agent work. No automatic replay of user input is introduced.

## Verification

Test that the displayed connection table resolves actual destinations, including reaction-only, mixed input and router fallback. Check recorded route/session bindings and old activity without route evidence. UI tests cover node editing, stop/save boundaries, route inspection and queued versus accepted activity. Compile frontend and Rust, launch an isolated native instance and inspect actual controls and saved configuration. Compare real JEV/routing in that instance using ambiguous work, a follow-up reaction and a mixed request; label simulated text separately from microphone proof.

Review the prototype in the desktop application. The user has returned to the laptop; stop updating the phone viewer. Retain screenshots and concise evidence alongside the visual exploration. Do not merge to main during this exploratory slice.

## Observed results — 2026-10-12

- Frontend: 26 relevant tests passed; scoped ESLint and TypeScript/frontend compilation passed. Rust: 28 voice, 20 workflow and 5 transport tests passed. Release application built and launched with isolated storage, a separate gateway on loopback port 4826 and WebView inspector on 9290. Normal phone/voice configuration was preserved.
- Real JEV and Codex agents processed simulated natural text. A lunch request delegated one task. “What makes the first one easier…” routed directly to Output without another assignment. A vegetarian amendment and an unrelated board-game invitation revised the existing task to revision 2 and created a separate task; both worker outcomes completed.
- Disabling the reaction shortcut through the UI, saving and resuming sent a JEV-classified reaction through receipt/router/digest. Restored the shortcut afterward. The Output notification checkbox persisted both enabled and disabled values; the final definition shows all eight nodes silent. Transport and sound-listener tests verify that silent terminal updates remain observable without producing the completion sound. No physical speaker-audibility claim is made.
- Text-to-accepted timings were 0.46–0.76 seconds. Accepted-to-next-publication was 11.27 seconds for the initial receipt, 7.28 for the direct explanation, 4.82 for the mixed-request receipt and 6.74 for the routed explanation's receipt. These are single observations using persisted timestamps, not microphone-to-audio latency or a controlled performance comparison; the isolated instance had no phone playback receiver.
- Native captures verify the fitted graph, node inspector, disabled sound toggle, saved route choice and actual activity. Viewer 0.2 was installed on the S21; its six-screen navigation/Zoom/Fit probe passed. Evidence is retained in `.dev/voice-extension/`; phone-review images are in the visual-exploration directory.

The broader non-orchestration Rust run had 585 passes, 5 failures and 1 ignored test. Failures were unchanged inventory JSON expectations, unchanged schema table expectations and three Codex app-server timing/steering tests. Those failures are recorded in `.dev/voice-extension-rust-suite.log`; the broader suite is not claimed green.

The trace also exposed redundant digest output requests after publication, including a reason saying nothing further needed saying. Output did not publish another utterance for that particular request, but extra reasoning turns occurred. The next iteration should evaluate the digest/output prompts and observation policy using this evidence. The graph's fixed topology and crowded connection labels are also deliberate limits of this first slice; arbitrary heterogeneous recipe execution remains unimplemented.
