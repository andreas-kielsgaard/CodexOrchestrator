# 0001: durable state and native query

Status: accepted basis for Sprint 5 implementation.

## Decisions

- Sprint 5 creates one fresh unified active-v3 database file with newly initialized Agent Session tables
  and the focused orchestration schema. The existing Agent Session v2 test database, incompatible
  active-v2 baseline, and legacy database stay untouched and unreferenced. No migration or import is implemented. A new schema
  version starts at `1`; later incompatible changes require an explicit migration decision.
- `EpicPlanningDraftId` is the pre-initiation identity. It can own proposal revisions and provenance,
  but never implies an Epic, Sprint, Work Unit, launch, or user/reviewer acceptance.
- Rust owns domain/application validation, authorization, idempotency, event recording, persistence,
  and the native query projection. TypeScript decodes the versioned native query and composes
  application/presentation reads. SQL rows and presentation DTOs are never a query authority.
- Native query contract starts as `orchestration-native-query/v1`: `generatedAt`, planning drafts,
  proposal revisions, recorded proposal events, and provenance links. Each collection is explicit,
  references use product IDs, and unavailable content is a semantic status rather than `null`-filled
  presentation data. It contains no display labels, derived progress, UI selection, or SQL fields.
- Rust serializes canonical golden fixtures for valid and rejected boundary cases. TypeScript decoder
  tests consume those byte-for-byte fixtures; Rust tests validate construction and JSON Schema. A
  version change adds `v2` and a decoder/fixture pair; never silently reinterpret a field.

## Current evidence and reversal point

`src-tauri/src/storage.rs` initializes the fresh unified active-v3 file and leaves older filenames
untouched.
`src/application/orchestrations/productReadModels.ts` labels its contracts as presentation inputs,
and `orchestrationClient.ts` has no durable connector. Revisit this record only if a future accepted
product requirement requires preserving real pre-Sprint-5 orchestration data.
