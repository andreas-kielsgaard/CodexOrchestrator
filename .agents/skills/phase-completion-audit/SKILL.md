---
name: phase-completion-audit
description: Independently audit a completed Sprint or larger milestone. Use when a Sprint Runner has dispositioned its planned Work Units and needs evidence about combined acceptance, product standards, integration, whole-product coherence, residual work, and readiness to hand the Sprint back to the Epic Runner.
---

# Phase Completion Audit

## Role

Act as a bounded helper to the Sprint Runner. Assess the combined Sprint against its accepted plan and current product state, then return a disposition. Do not implement corrections, continue planning, or become the next owner.

## Inputs

Expect the Sprint objective and acceptance boundary, concern-to-Work-Unit map, final launch register, Work Unit specifications and dispositions, integrated or preserved checkpoints, validation, records, deferrals, and human gates.

## Audit

Check whether:

- required Work Units are accepted, explicitly deferred, superseded, or blocked with the right owner;
- the combined result satisfies the Sprint objective and relevant product standards;
- cross-Work-Unit integration, regressions, and whole-product coherence have been evaluated;
- validation and evidence support the claims being made;
- records and repository state describe the result truthfully; and
- remaining work belongs inside this Sprint, a later Sprint, the Epic Runner, or a human decision.

Choose `complete`, `partial`, `not-complete`, or `human-needed`.

Return the verdict, evidence, missing or deferred items, concrete corrections, human actions, record updates, and recommended Sprint Runner action. State callback delivery and receiver-activation evidence separately, then end the turn.
