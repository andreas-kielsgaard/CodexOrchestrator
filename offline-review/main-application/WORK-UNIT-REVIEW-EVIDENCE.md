# Recorded Work Unit review evidence

Captured 2026-08-05 from the local development route:

`http://127.0.0.1:4173/?recorded-work-unit-review`

This is recorded/demo evidence. It does not prove live-provider behavior, productive persistence,
MCP availability, acceptance, or user acceptance.

## Executed route flow

- The local browser opened Epic, **Sprint Control Surface Discovery**, and **WU-ECS2E**. At the
  ordinary local viewport (`754px` wide), Activity and Evidence were peer tabs and Activity had no
  initial selected turn.
- A correlated Lifecycle step opened only its exact Handler Activity. The selected Activity card
  contained the reusable read-only complete turn, including recorded steps and authoritative
  timing; the former full-session workspace and separate selected-turn panel were absent.
- The second Implementer reporting Activity showed its immediately preceding authoritative input.
  Its **Open in Agent Sessions** control opened the exact Session/invocation. The standalone view
  focused that invocation and displayed **Return to Work Unit Activity**; returning restored the
  Activity selection without a composer.
- Evidence exposed a typed available changed-file destination and a typed unavailable changed-file
  destination. The available item opened the isolated read-only File and diff review on
  `WorkUnitDetailWorkspace.tsx`; the unavailable item did not navigate. Explicit MCP detail
  unavailability remained nested beneath its owning Activity.
- The fixture displayed a typed available test run, command, environment, result, and individual
  case. Focused component and decoder evidence exercise the separate typed unavailable test path.

## Accessibility and viewport observations

- The focused interaction test exercises peer-tab ArrowRight focus movement, exact Lifecycle
  highlighting on Activity focus, disclosure interaction, unavailable states, and no composer.
- In the executed `640px x 900px` browser flow, exact standalone focus/return, Activity/Evidence,
  available file navigation, and nested content all completed with
  `document.documentElement.scrollWidth === clientWidth === 640`.
- The known `430px` top-navigation clipping/scrolling remains an unclaimed responsive residual.
  It is not presented as passing evidence.

## Executable evidence

- `src/dev/orchestrationSection/recordedWorkUnitReview.consumer.test.tsx` covers the explicitly
  recorded route payload, exact selection, disclosure, evidence ownership, and no composer.
- `src/features/orchestrations/components/WorkUnitDetailWorkspace.activityEvidence.test.tsx`
  covers focused tab keyboard behavior, Lifecycle focus highlighting, unavailable inspection, and
  Activity/Evidence navigation.
- `src/application/orchestrations/nativeQuery.test.ts` covers typed test detail and changed-file
  destination decoding, including foreign correlation rejection.
- `src/app/App.agentSessions.test.tsx` covers exact standalone Session focus, typed return
  restoration, and available recorded file-review navigation.
