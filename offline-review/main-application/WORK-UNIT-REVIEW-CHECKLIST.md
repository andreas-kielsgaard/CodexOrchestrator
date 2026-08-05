# Recorded Work Unit review checklist

Route: `http://127.0.0.1:4173/?recorded-work-unit-review`

This is a reusable recorded development presentation. It is not live-provider proof, productive
persistence proof, acceptance, or user acceptance.

- [ ] Open the recorded route and reach `WU-ECS2E` through Epic → Sprint → Work Unit detail.
- [ ] Confirm Activity and Evidence are peer views.
- [ ] Confirm no activity is selected on first open.
- [ ] Select Handler and Implementer entries and verify exact Session/invocation correlation.
- [ ] Inspect complete input/output and expand recorded steps; confirm there is no composer.
- [ ] Confirm application summaries are nested beneath their owning activity.
- [ ] Confirm MCP-call detail and test detail are explicitly unavailable.
- [ ] Confirm the missing related activity stays unavailable and does not create a relationship.
- [ ] Open Evidence, inspect typed changed files, and navigate to the owning activity.
- [ ] Confirm the owning activity is selected/highlighted after navigation.
- [ ] Repeat the flow with keyboard tab navigation and at a narrow viewport.

Automated interaction and keyboard evidence: `WorkUnitDetailWorkspace.activityEvidence.test.tsx`.
Fixture-boundary evidence: `recordedOrchestrationClient.test.ts`.
