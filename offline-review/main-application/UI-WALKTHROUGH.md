# Offline UI walkthrough

The recorded development composition is the safest useful offline review surface. It uses the real
application shell and real presentation components with recorded adapters.

## Prerequisites

- Work from `C:\Users\user\Documents\Code Projects\Codex Orchestrator`.
- `node_modules` must already exist. Do not run an install while offline.
- No Agent Session or provider call is required.

Check locally:

```powershell
Set-Location "C:\Users\user\Documents\Code Projects\Codex Orchestrator"
Test-Path node_modules
git rev-parse --short HEAD
```

Expected baseline: `f23f5fd`.

## Start the recorded application

```powershell
npm run dev -- --host 127.0.0.1 --port 4173 --strictPort
```

Open:

`http://127.0.0.1:4173/?recorded-plan-builder`

For the focused Work Unit review fixture, open:

`http://127.0.0.1:4173/?recorded-work-unit-review`

This route is a recorded development presentation only. It does not prove live-provider behavior,
productive persistence, acceptance, MCP availability, or user acceptance.

Stop the server afterward with `Ctrl+C`.

If port 4173 is already occupied, choose another local port and replace it in the URL. If Vite asks
to install anything, stop; the offline package should use already-installed dependencies only.

## Review path

### 1. Orchestration overview

Look at:

- whether **Orchestration** is the right capability-level entry;
- whether planning drafts are distinguishable from initiated Epics;
- whether **Plan an Epic** is the appropriate primary action;
- whether the overview provides enough orientation without pretending execution exists.

### 2. Plan an Epic

Open **Plan an Epic**.

Review:

- conversation as the primary working surface;
- Epic name placement;
- the adjacent **Proposed Epic** hierarchy;
- predicted Sprint and concern density;
- collapse/scroll behavior;
- whether structured state looks authoritative without competing with the conversation;
- copy that distinguishes discussion from plan submission.

Do not treat recorded proposal content as live product state.

### 3. Agent Sessions

Open the **Agent Sessions** peer tab.

Review:

- session-list versus conversation proportions;
- collapsed processing beneath the final answer;
- status and cancellation placement;
- transcript readability;
- whether the interaction still feels useful outside orchestration.

### 4. Return to Orchestration

Confirm that switching capability surfaces preserves a coherent mental model: Agent Sessions are a
general interaction primitive, while Orchestration is a product workflow that uses them.

### 5. Recorded Epic and Sprint detail

From the overview, open **Codex Epic Runner workspace development**, then open **Sprint Control
Surface Discovery**.

This is recorded discovery material, not the current live Epic. Use it to review:

- whether Epic → Sprint navigation is understandable;
- whether the Sprint flow map communicates dependency and sequence;
- whether plan revisions are discoverable;
- whether planner and Work Unit nodes are visually distinct enough;
- whether **Flow**, **Concerns**, and **Documents** form the right detail hierarchy;
- whether the Agent Session entry belongs at the bottom of the workspace;
- whether the wide canvas needs fit-to-view, minimap, zoom, or stronger horizontal-navigation cues.

The packaged 1440-pixel capture visibly clips the continuation of the map to the right. Treat that as
a review prompt, not proof that the current navigation is sufficient.

### 6. Focused Work Unit Activity and Evidence review

Using `?recorded-work-unit-review`, open **Codex Epic Runner workspace development**, then **Sprint
Control Surface Discovery**, and open **WU-ECS2E — Plan and Work Unit detail surfaces**.

Exercise the recorded review flow:

- Confirm **Activity** and **Evidence** are peer tabs and no turn is selected initially.
- Select Handler and Implementer entries and verify the shared inspector shows the exact recorded
  Session/invocation pair, complete input/output, expandable recorded steps, and no composer.
- Expand the nested application summaries. Confirm MCP-call detail is explicitly unavailable and a
  missing related activity remains unavailable rather than becoming an inferred link.
- Move to **Evidence**, inspect the typed changed-file entries, use **View owning activity**, and
  confirm the activity is highlighted after navigation.
- Confirm test detail remains explicitly unavailable.
- Repeat tab navigation with ArrowLeft/ArrowRight and resize to an ordinary desktop width and a
  narrow mobile-like width. Text and controls should remain readable without horizontal overflow.

The focused automated evidence is
`src/features/orchestrations/components/WorkUnitDetailWorkspace.activityEvidence.test.tsx`; the
recorded composition test also verifies that the inspection payload is absent from canonical reads
and added only by the explicit recorded presentation route. The executed route, accessibility, and
viewport observations are recorded in `WORK-UNIT-REVIEW-EVIDENCE.md`.

## Optional native application review

If battery and local build time permit:

```powershell
npm run dev:tauri
```

This starts the actual application composition, not the recorded Plan Builder composition. Without
provider connectivity, do not use it to judge live agent behavior. It can still be used to inspect
native-window sizing, navigation, and unavailable/error states.

## Not possible offline

Do not attempt to complete the final Sprint 6 manual gate. It requires a real user-authored Plan
Builder query, a live proposal submission, the production confirmation modal, Bootstrap Generator
completion, and an Epic Runner launch.

## Packaged screenshots

The design can be reviewed even if the local server cannot start:

1. [Orchestration overview](assets/01-orchestration-overview.png)
2. [Plan an Epic](assets/02-plan-an-epic.png)
3. [Agent Sessions](assets/03-agent-sessions.png)
4. [Recorded Epic detail](assets/04-epic-detail.png)
5. [Recorded Sprint flow](assets/05-sprint-detail.png)

All five are recorded development presentation. They are not screenshots of a live orchestration.
