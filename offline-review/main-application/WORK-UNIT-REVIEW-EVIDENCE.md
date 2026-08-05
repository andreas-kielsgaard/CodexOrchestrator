# Recorded Work Unit review evidence

Captured 2026-08-05 from the local development route:

`http://127.0.0.1:4173/?recorded-work-unit-review`

This is recorded/demo evidence. It does not prove live-provider behavior, productive persistence,
MCP availability, acceptance, or user acceptance.

## Executed route flow

- Epic → **Sprint Control Surface Discovery** → **WU-ECS2E** was opened successfully.
- Activity and Evidence appeared as peer tabs; Activity was selected initially and no turn inspector
  was present.
- The Handler action selected the exact recorded Session/invocation pair
  `recorded-session-WU-ECS2E` / `recorded-handler-WU-ECS2E-first-review`.
- The shared inspector displayed complete recorded input/output, recorded start time and duration,
  and the **Recorded steps** disclosure expanded to show one processing update. No textbox or
  composer was present.
- Evidence showed two typed changed-file entries and explicit unavailable test detail.
- **View owning activity** returned to Activity and selected/highlighted the exact Implementer
  reporting activity for the same attempt:
  `work-unit-inspection:WU-ECS2E:WU-ECS2E-attempt-2:implementer-reporting:recorded-implementer-WU-ECS2E-second-return`.

## Accessibility and viewport observations

- The browser DOM exposed `main`, `tablist`, `tab`, `tabpanel`, named regions, and selected-tab
  state. The focused consumer test executes ArrowRight tab movement and verifies focus/selection.
- At `640×900`, the browser reported `document.documentElement.scrollWidth === 640` and
  `clientWidth === 640`; no document-level horizontal overflow was observed. The Activity list,
  nested summaries, disclosure, and selected-turn content remained readable in the captured view.
- A `430×900` exploratory capture exposed clipped/scrollable top application navigation. That width
  is retained as a responsive residual, not claimed as passing evidence; the focused representative
  narrow evidence is `640×900`.

Executable evidence:

- `src/dev/orchestrationSection/recordedWorkUnitReview.consumer.test.tsx` — recorded route payload
  through the Work Unit consumer, exact selection, disclosure, navigation/highlighting, keyboard,
  accessibility roles, and no composer.
- `src/dev/orchestrationSection/recordedOrchestrationClient.test.ts` — canonical-read isolation and
  accepted activity-summary/peer/file correlation shape.
- `src/features/orchestrations/components/WorkUnitDetailWorkspace.activityEvidence.test.tsx` —
  focused component interaction and fail-closed inspector coverage.
