# Workflow graph and JEV inspection

Implement on `codex/voice-extension-workflow`. Desktop only; retain the isolated prototype and leave main unchanged.

- Replace the shared straight-line canvas with a React Flow presentation component and an ELK layout adapter. Authoring, instance inspection and extensions consume the same surface. Feature owners supply node content, selection, editing and execution evidence.
- Organize explicitly; status polling never triggers layout. Authoring saves organized positions as one draft edit. Read-only graphs keep presentation positions locally. Preserve individual connection identities and directions. Add pan/zoom, fit selection, search, contextual labels and a textual connection list.
- Voice supplies three collapsible visual groups and connection roles. Groups do not change execution. Recorded activity highlights actual destinations; no simulated sequencing.
- JEV query history reuses agent invocations and structured results. Add a typed query projection for list/detail loading. The provider owns request/result fields; Voice retains invocation links and applied sentence decisions in its existing conversation state. Configuration remains separate from query history.
- Show exact submitted context, named questions and criteria, typed answers, probabilities and confidence separately, usage and timing. Repeated evaluations remain independent. Keep selection stable as new queries arrive; compare a question with its previous evaluation. Raw JSON is diagnostic detail.

Verify layout and independent edge selection, authoring edits/undo, saved positions, stable updates and grouping. Verify JEV projection and retained versus returned relevance, revisions and unapplied stale results. Run focused tests, compile the executable and inspect the native desktop prototype with existing query history and a real provider evaluation.

## Result — 2026-10-12

Implemented the shared React Flow canvas, worker-based ELK layout and independent directed edges. Removed the former renderer, drag implementation and grouped-connection dialog. Voice starts with three collapsed groups; all nodes and selected connections remain inspectable. Status polling preserves the viewport and layout.

JEV list/detail commands project stored evaluations and structured answers. Voice retains applied decisions and release references in its existing state. Query selection stays stable as new evaluations arrive. Comparisons match exact question IDs; questions absent from the previous evaluation have no corresponding answer.

37 focused frontend tests, 28 existing Voice tests, 2 new decision-evidence tests and 8 JEV provider tests passed. Scoped ESLint, TypeScript and the release build passed. The broader Rust suite was not rerun; its earlier five failures remain unresolved.

Native verification used isolated storage and the loopback gateway on port 4826. Organize and Save changed an overlapping three-node draft into persisted positions (40,40), (40,160), (40,280). The diagram and existing four JEV evaluations were inspected in the desktop application.

A simulated ambient sentence was entered through the desktop form and evaluated by the real provider in 0.55 seconds. Its stored response was unaddressed / continue / none. The applied decision recorded no accumulation and no release; task and assignment counts stayed at two and three. This verifies text evaluation and inspection, not microphone capture or playback. The isolated connection was stopped afterward; the normal phone-connected instance remained running.

Opening the bound JEV session also showed the five-query list with no message composer. The final desktop build is retained under attempt `86651290-ed48-4565-beca-f462021727fa`; its exact owner and review captures are in `.dev/voice-extension/`. The prototype remains open for review on Voice Sessions → Workflow.
