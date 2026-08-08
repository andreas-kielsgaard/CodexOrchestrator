# Operation trace: load and present durable orchestration state

## Frontend request

`createTauriOrchestrationNativeQueryClient.load` invokes `load_orchestration_native_query` and immediately decodes the unknown response with `decodeOrchestrationNativeQueryV2`.

`createNativeQueryOrchestrationClient` also loads the bootstrap-transition and Sprint-transition queries when those clients are supplied. It then:

- returns `empty` when no initiated Epic exists;
- otherwise converts native facts into composition input;
- builds canonical product read models with `composeProductOrchestrationReadModels`;
- returns an explicit unavailable result when any required load or decode fails.

No accepted Epic root is invented from a draft or proposal.

## Tauri and application boundary

`orchestration/transport.rs::load_orchestration_native_query` delegates to `OrchestrationApplication::native_query`, which delegates to `SqliteOrchestrationRepository::native_query`.

The command is a snapshot boundary. Notifications or refresh timing are not authority by themselves.

## Repository projection

`orchestration/repository.rs` reads and validates a broad durable projection that includes planning drafts, proposal revisions and events, associations and provenance, initiated Epics, execution graph and activity facts, Harness state, file-review references, and related lifecycle evidence present in the selected implementation line.

The separate bootstrap and Sprint transition queries expose lifecycle state owned by their respective services. The frontend composes the three sources rather than pretending the primary native query owns every transition fact.

## Presentation path

- `nativeQuery.ts` performs strict transport decoding.
- `productReadModelComposer.ts` establishes product relationships and derived views.
- `sprintWorkspacePresentation.ts` and related application modules shape view-specific presentation.
- `app/orchestrationPresentation.ts` adapts product read models into feature presentation.
- `OrchestrationSection` and its detail workspaces render the result.

## Architectural reading

This is the main read-side spine of the product. Its explicit decoding and empty/unavailable behavior are strong boundaries. Its scale also reveals accumulated responsibility: transport schema, validation, compatibility, cross-lifecycle assembly, product relationships, and presentation derivation span several very large TypeScript modules and three backend snapshot sources.
