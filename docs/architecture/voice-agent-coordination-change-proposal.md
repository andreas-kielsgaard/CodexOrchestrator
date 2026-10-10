# Voice agent coordination change proposal

Status: implemented for prototype evaluation. See [verification](../voice-sessions/coordination-2026-10-10.md) for live results and limits.

Revised 2026-10-10 after the user's prototype-scope correction. Source baseline: main `2fc467df`. Runtime evidence: `.dev/voice-activation-final/usage-review.md`.

## 1. Recommended implementation choices

Keep the existing persistent summary, router, digest, output, and worker sessions. Improve how they exchange input and results using ordinary Agent Session delivery, steering, history, and managed tools.

Proposed choices:

- Add small task and assignment records to the existing voice conversation state. Task identity, current request version, and worker invocation identify what work belongs together.
- Share the delivery decision used by stages and workers: new turn when idle, steering when active, and next-turn delivery when a queued correction was not delivered before the current turn ended.
- Use typed inputs/results for existing handoff, progress, context, and output-request tools. Add a small worker-outcome operation if needed to identify which assignment and request version the result addresses.
- Let the existing digest interpret selected agent messages as observations. A separate stream-interpreter agent remains an optional later experiment.

These choices improve the normal flow without introducing another execution scheduler, durable delivery subsystem, or general orchestration framework.

## 2. Open questions and prescribed scope

Still open: whether steering arrives early enough to improve evolving requests; how reliably agents identify the task being amended; whether ordinary intermediate messages provide useful progress; and whether smaller context makes replies sufficiently responsive. Evaluate these through actual use before adding more machinery.

The user's prescribed scope is a prototype. Handle observed issues and reasonable real-time overlap. Do not add crash recovery, phone restart recovery, command replay, exactly-once delivery, a new persistence architecture, or speculative failure-state machinery. Retain the persistence and lifecycle behavior already provided by Agent Sessions and voice state.

Keep the requested continuous transcription, revisable sentence blocks, JEV relevance/completion/continuation evaluation, sticky accepted relevance, buffering, acknowledgment, independent routing, delegated work, digest/output split, editable prompts, and phone controls. Tailscale remains the application's connection prerequisite; debugging access stays operator tooling.

## 3. Evidence and intended changes

The live review exercised unrelated tasks, amendments to two tasks in one utterance, older-task follow-ups, and explanations during work. It observed a rejected JEV evaluation losing its opening context, stale diagnostics, a worker amendment waiting about 64 seconds, a delayed status reply describing already-finished work as pending, and difficult-to-read histories.

Source inspection also found session-only worker-result routing and removal of observations once steering was merely queued. These source findings concern normal concurrent execution; their full user-facing effects were not reproduced.

| Change | Intended contribution | Limit or unknown |
| --- | --- | --- |
| Retain unresolved JEV input for reevaluation; distinguish current evaluation state from historical errors. | A failed evaluation does not silently lose the request's opening, and recovery becomes visible. | Valid output can still contain a mistaken classification. |
| Add task identity and current request version to handoffs and results. | Related amendments stay together; unrelated work and mixed requests remain distinguishable. | Agents still have to interpret references such as “that one.” |
| Use active steering for worker amendments and retain known-undelivered corrections until the next turn. | Reduces obsolete work and prevents normal turn completion from dropping pending input. | Runtime acceptance does not prove immediate incorporation. |
| Link returned results to the actual assignment/invocation. | Reusing a session does not send a result to every former caller. | Several amendments may occur within one invocation. |
| Use compact context and current task observations for output. | Reduces avoidable reply delay and stale status narration. | Model latency and changes during drafting still require observation. |
| Show named tasks and concise activity, with raw context expandable. | Makes recording, classification, pending input, work, and reply state understandable. | Useful summaries need to remain grounded in source records. |

## 4. Normal real-time operation

### Speech and classification

Display revisable transcription as it arrives. JEV evaluates the current sentence window while capture continues. Keep one evaluation active and schedule the latest changed window next, rather than build a queue of obsolete snapshots.

Apply classifications to the sentence revisions evaluated. Continue reconsidering the window as context develops; preserve accepted relevance as already requested. Relevant but unfinished input stays buffered.

Distinguish malformed/rejected JEV results from semantically mistaken classifications. On evaluation failure, retain the unresolved opening context and reevaluate with the next current window, with a bounded immediate retry where appropriate. Show the failed/current evaluation state and clear the active error on success.

An initially missed request can become relevant during subsequent window evaluation. A premature or incorrect interpretation after release should reach the router as a linked continuation/correction. Preserve the original accepted input so the agent can reason about the change. Do not add hardcoded phrase rules or silently retract sticky relevance.

When JEV judges the input ready, send it independently to summary and router, and supply it to digest. Summary acknowledges briefly; routing proceeds without waiting for that acknowledgment.

### Busy agents and changing tasks

Use the existing observation buffer and Agent Session controls. Idle recipients receive a new turn; active recipients receive a steer. Keep input pending until its existing interaction record establishes delivery. If a turn ends before a queued correction is delivered, send that correction as the next turn. Do not automatically resend a correction already accepted by the runtime.

Keep provider calls outside voice-state locks. Batch ordinary progress, but retain the individual accepted inputs and amendments. Agents finish their turns when waiting for delegated results, allowing later input or results to resume them.

The router decides whether input creates a task, amends one or several tasks, or requests conversation about existing material. Record those decisions through the handoff tools. Do not hardcode a task taxonomy or question-to-output route.

Register an assignment before launching its worker, then associate the returned invocation reference. If a result arrives before that association is available, keep it pending briefly until it can be matched. Use the existing persisted invocation history to obtain its result. This is a normal launch-ordering concern, not a new crash-recovery protocol.

A worker may receive several amendments in one invocation. Its outcome identifies the request version it reports addressing. Keep an earlier result as material if a later amendment remains pending; the router decides whether more work is needed. A completed turn can also report a question, partial findings, or a child delegation.

### Conversation while work continues

Digest receives accepted input, routing decisions, progress, and results while workers continue independently. It decides whether now is a useful time to speak and forwards a short reason plus references to output.

Output retrieves the relevant material and current task state. Refresh status context before drafting and feed relevant completions into an active output turn. First evaluate whether this fixes the observed stale reply before adding version-based publication rejection or draft-retry machinery.

Preserve agent-authored headlines and availability announcements. Detailed explanation follows the user's request. Summary retains acknowledgment-only publication; ordinary speech comes from output. Stopping speech and changing delegated work remain separate actions.

### Interpreting agent streams

First send selected completed intermediate messages and explicit progress reports from workers/observed sessions to the existing digest. Include source session, invocation, time, and message references. Do not forward every token or depend on hidden reasoning.

Digest can infer progress or a possible clarification from those messages. Keep that interpretation distinguishable from application lifecycle facts and explicit worker outcomes. Do not treat inferred prose as an automatic assignment, cancellation, or completion.

Observe worker sources rather than digest/output publications, avoiding self-generated feedback. Try a separate read-only interpreter only if ordinary use reveals that digest context or message noise needs a different owner.

## 5. Code shape and reuse

| Ownership | Retain, adapt, create, or remove |
| --- | --- |
| Continuous speech | Retain `voice_sessions/{continuous_audio,transcript,query_schedule}.rs`; adapt `prefilter.rs` for current evaluation status, unresolved context, and revision-aware reevaluation. |
| Task meaning | Add focused `voice_sessions/tasks.rs`; extend existing `domain.rs` and `repository.rs` records. Keep the current voice-state persistence. |
| Delivery mechanics | Extract the repeated stage/worker delivery decision into a focused Agent Session application helper; adapt `product_sessions.rs`, `steering_queue.rs`, and voice `pipeline.rs`/`delegation.rs`. Preserve the existing runtime queue and interaction records. |
| Observations | Adapt `observations.rs` for assignment/invocation correlation, tracked pending input, and selected intermediate messages. Remove session-only return routing and removal on merely queued steering. |
| Structured operations | Adapt `orchid_control/{service,mcp}.rs` and add focused voice tool contract definitions using existing Serde and managed caller context. Remove untyped observation JSON strings where typed records suffice. |
| Output and history | Adapt `application.rs`, `diagnostics.rs`, `phone_history.rs`, desktop `VoiceSessionsScreen.tsx`, and Android history views. Replace repeated full-context injection and generic worker labels with compact referenced context and named tasks. |
| Behavior | Update concise stage resources and the existing desktop prompt editor as needed. Preserve intentional custom prompts; show conflicting effective instructions rather than silently overwrite them. |

Use the already-used Agent Session infrastructure, Serde, Tokio, SQLite, and managed MCP host. Do not introduce new libraries, a generic event bus, a second scheduler, or new database tables solely for anticipated robustness.

Epic inspires distinct request/revision/outcome identities. Workflow inspires explicit source and destination relationships. OTP inspires declared structured interfaces and host-supplied provenance. Their broader execution models remain independent.

## 6. Implementation order and verification

1. Fix JEV reevaluation/current diagnostics and share the delivery path for stages and worker amendments.
2. Add task/assignment references and structured outcomes; verify normal concurrent routing.
3. Improve compact context, output freshness, and desktop/phone visibility.
4. Evaluate intermediate-message interpretation in digest before deciding whether a separate interpreter is worthwhile.

Exercise two unrelated tasks, evolving instructions to both, an explanation request while work continues, an older task revisited, a result arriving while an amendment is pending, and an agent asking for clarification. Also verify the observed JEV rejection/recovery and the ordinary case of a queued correction reaching a turn boundary.

Measure release-to-acknowledgment, amendment-to-runtime-acceptance, explanation-to-publication, and task-result-to-announcement. Inspect what was actually delivered and what the agent reported incorporating. Avoid claiming that delivery acceptance guarantees understanding.

Run relevant focused checks and launch the application to review the normal desktop/phone flow. Crash/restart recovery and speculative fault-injection scenarios are outside this prototype proposal.

## References

- [Voice pipeline shape](voice-prefilter-pipeline-implementation-shape.md).
- [Voice Sessions shape](voice-sessions-implementation-shape.md).
- [Epic orchestration concepts](../orchestration/README.md).
- [Workflow routing concepts](../workflows.md).
- OTP interface inspiration: `src-tauri/src/otp_api/contract.rs`.
