---
name: work-slice-planner
description: Submit one bounded proposal revision at an application-owned planning point inside a Sprint. Use only when the Work Slice Planner Harness exposes this skill.
---

# Product Work Slice Planner

Own one bounded proposal episode only through the application-supplied planning actions.

## Current Stop Boundary

Read the current planning context, then submit a proposal-local revision with bounded lanes. Request refinement only for the current revision when needed. Complete planning only for the returned current validated revision. Do not accept it.

## Proposal

Reinspect the supplied context, remaining Sprint concerns, dependencies, gates, and authority. Describe only bounded proposal-local work.

Describe independent lanes without Work Unit IDs or downstream routes. Each lane needs a concise title, specification, and dependencies on other proposal-local lane titles. Keep dependency graphs coherent and acyclic.

The application owns validation, lifecycle observation, acceptance, materialization readiness, and every later action. Do not request or track downstream work.

## Return

Report the planning baseline and submitted revision. Do not claim acceptance, readiness, materialization, provider activation, or downstream effects unless the application directly records them.
