# Voice workflow: implementation shape

This pass implements the navigation and attention-order findings in `reports/voice-workflow-ux-audit-2026-10-13/`. The existing presentation is a starting point, not an approved design.

## Decisions made for this pass

- Replace the three arbitrary collapsed groups with an eight-stage overview and focused stage neighbourhoods. Use the supplied compact coordinates for the overview and ELK for focused views. Show all routes initially so Digest's inputs remain visible; disclose labels on selection and allow context/feedback links to be hidden. Preserve the overview camera when returning from a focused view. This was refined after the first live capture exposed excessively small automatic overview layout.
- Keep inspection, behavioural editing and execution evidence distinct, with a return path that retains selection and draft. Long prompts and JEV evaluations use the workspace width rather than the narrow inspector.
- Use one settings draft and one Save/Discard boundary across Workflow and Connection. Preview the direct-reaction draft in the map, identify what changed, and distinguish subsequent-turn prompt application from new-conversation model application.
- Put purpose and incoming/outgoing routes before settings. Put tools, shared models and routing preferences next to their consuming roles. Move notification preferences into secondary disclosure for agent turns.
- Show one recorded event and its actual delivery receipts beside the map; open the receipt's invocation when available. Do not infer request ancestry from neighbouring timestamps or equate accepted input with completed work or playback.
- Keep the voice workspace mounted after first entry, pause its polling when away, and restore its DOM/camera/draft on product Back. This is in-memory navigation continuity, not cross-restart persistence.

## Ownership and concrete changes

| Owner                                                                           | Retain / change                                                                                                                                                                          |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VoiceSessionsScreen.tsx`                                                       | Retain the real client, polling, draft and save boundary. Reduce to shell, connection controls and three persistent task views.                                                          |
| `VoiceWorkflowPanel.tsx`                                                        | Own selected stage/connection/event and transitions into behaviour or evaluation inspection. Remove duplicate runtime header, inline prompt editors and the below-canvas activity stack. |
| New `VoiceBehaviorEditor.tsx`                                                   | One full-width editor for role prompts, shared discovery instructions, model groups and routing preferences. Reuse actual `VoiceSettings`; no second configuration model.                |
| New `VoiceDraftBar.tsx`                                                         | Derive changed fields, show their scope and expose the shared Save/Discard actions.                                                                                                      |
| New `VoiceActivityPanel.tsx`, `VoiceDiagnostics.tsx`, `VoiceRouteInspector.tsx` | Separate conversation outcomes, transport diagnosis and event delivery evidence. Consume the existing DTOs.                                                                              |
| `VoiceSetupPanel.tsx`                                                           | Keep device connection, profile and working directory. Remove duplicate model/prompt/routing editors and its separate-looking save button.                                               |
| `ExtensionWorkflowCanvas.tsx`                                                   | Own Voice's presentation projection, stage focus and context-link disclosure. Remove hard-coded composite groups.                                                                        |
| `WorkflowGraphCanvas.tsx`                                                       | Retain React Flow and ELK for all three consumers. Add optional compact navigation and saved viewport input/output; authoring and instance defaults remain intact.                       |
| `JevQueryInspector.tsx`                                                         | Lead with the applied decision, disclose probability detail, identify the compared evaluation and distinguish missing questions from pending responses. Pause polling while hidden.      |
| `App.tsx`                                                                       | Retain visited Voice workspace and route optional invocation/profile navigation through existing product history.                                                                        |

No new library is needed: React Flow already owns graph navigation and ELK owns layout. Runtime routing, tools and topology remain owned by the Voice extension. Arbitrary node creation/connection needs a separate executable authoring contract; this pass will state that boundary rather than provide inert controls.

## User-prescribed direction

Evaluate complete consumer journeys and transitions, make the underlying prompts/tools/agents/connections understandable and editable where supported, use progressive disclosure, and make the changes on the voice branch. Preserve unrelated working-tree changes.

## Validation

Exercise overview → stage → prompt/tools/model → return; connection → draft preview → change review → save/discard; evaluation → applied decision → route → invocation → Back; Activity → diagnostics; Connection → Workflow. Test draft isolation, active-connection edit locks, retained selection and exact invocation navigation. Run focused shared-canvas/JEV/Voice tests, typecheck/build and scoped lint. Rebuild and inspect the isolated desktop instance through Orchid's CLI, with screenshots of actual transitions. Provider execution and microphone/playback are separate from this UI verification.

### Results — 13 October 2026

- 18 Voice/JEV/shared-graph tests and 15 product-navigation tests passed, along with TypeScript, the release build, scoped ESLint and whitespace checks.
- Native checks verified stage focus and exact overview camera restoration; editor return and keyboard focus; shared tools → Capability Profile → Back; draft retention across views; persisted route changes; and recorded event → exact delivered invocation → Back.
- Recorded-route emphasis now identifies only evidenced connections. The shared canvas waits for computed node positions before fitting a focused view.
- A 960 × 760 emulated Activity viewport had no document horizontal overflow. This was not a full accessibility audit.
- Evaluation → released route → same evaluation passed with a test fixture. Available native history has applied-decision evidence only for an unreleased evaluation, so that handoff remains unverified against native retained data. No fresh provider evaluation, microphone capture or phone playback was exercised.
- Original review-instance settings were restored and compared equal; its connection remains stopped. Unrelated working-tree changes were preserved.

The [implemented walkthrough](../../reports/voice-workflow-ux-audit-2026-10-13/refinement.html) contains native screenshots and transition evidence. Final release build: `c919bf0e-fe98-4323-9f2b-4826a6c50ac5`.
