---
name: orchestration-worker
description: Complete an already-running orchestration-worker assignment. Use when a worker was launched by a work-slice-delegation route and must finish or correct that fixed slice, validate it, and return a review payload to the same delegation owner.
---

# Orchestration Worker

## Role

Finish the fixed assignment supplied by an existing `work-slice-delegation` route. Use only the launch prompt and referenced sources; preserve its repository/worktree boundary and unrelated work.

## Execute

- Reinspect the relevant sources and implement the complete assigned scope.
- Make ordinary local choices inside delegated authority.
- Choose proportionate validation and report unproven boundaries truthfully.
- Load `$agent-interface-first` when implementation or validation could involve visible UI control.
- Return a concrete blocker when the route, authority, or required evidence makes safe completion impossible.

## Return

Send the delegation owner one compact review payload containing status, result, changed files, repository state, validation, risks, decisions, and requested review disposition. State notification delivery and receiver-activation evidence separately, then end the turn without polling.

Handle a later correction or clarification only when it remains inside this assignment.
