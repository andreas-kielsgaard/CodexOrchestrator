# 0008: Sprint 6 confirmation, context delivery, and transition UI

Status: accepted implementation direction for WU-S6-03. Extends 0006 and 0007.

## Decision

- One application-level controller owns the typed confirmation event subscription, button request,
  request queue, and resolution. Button and agent requests render through one accessible in-app
  modal; feature components never invoke Tauri or listen to events directly.
- A confirmed button request records one server-derived pending context delivery against the
  associated managed Plan Builder session. The next managed query keeps its submitted text and
  transcript provenance unchanged and adds concise application-provenance runtime context. Before
  launch, the claim is durably bound to a preallocated Agent Invocation ID and sent through the
  provider-neutral idempotent application boundary. Agent-origin requests do not schedule it.
- The strict `epic-bootstrap-transition-query/v2` decoder is a focused adjunct. Its correlated
  projection joins the native-v2 Epic only in `productReadModelComposer`, which remains the
  canonical product-read convergence boundary.
- The application shows one compact transition status. Labels preserve confirmation, preparation,
  Bootstrap attempt/lifecycle, retry/block, material acceptance, and Runner creation/launch as
  distinct facts and never imply a Sprint started.

## Recovery and errors

Malformed confirmation or transition payloads fail closed. Failed refreshes replace prior success
with unavailable state. Duplicate request receipts are idempotent, distinct requests are serialized,
and stale, timed-out, or unavailable resolutions remain visible without inventing completion.
Confirmation resolution and the later application refresh are separate: a refresh outage cannot
reclassify or repeat a successful durable confirmation.

An interrupted context claim is reconciled before another claim is selected. Launch acceptance is a
separate provider-neutral durable fact recorded only after runtime start or resume returns success;
`started_at`, running, and interrupted state are not acceptance. That fact consumes the claim for its
bound invocation without redelivery. Missing or persisted-but-unaccepted evidence releases it, while
conflicting identity or provenance fails closed. Reconciliation is idempotent. Runtime success followed
by acceptance-marker persistence failure remains conservatively unaccepted and retryable, so atomic
external provider processing is not claimed.
