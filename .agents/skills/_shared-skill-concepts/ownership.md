# Ownership

- Root orchestrator: direction, current objective, loop timing, and high-level state.
- Intake refresh: root-owned state sensor. Return changed assumptions and source-backed deltas, then stop.
- Planner fork: next-work decisions, open-item evaluation, delegation routing, launched-slice tracking, and batch settlement.
- Work-slice delegation: planner-owned path for one slice through worker startup, review, integration, reporting, and planner notification.
- Worker root: implementation or investigation for one slice from its launch prompt.
- Record root: maintained orchestration memory. Record-maintainer children update records and notify owners when record work settles.
