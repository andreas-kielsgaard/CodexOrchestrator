# File Review

File Review presents one application-selected collection of files and changes. The viewer is read-only: it receives display-ready content, paths, counts, and hunks through `FileReviewSource.load()`. The originating context owns source selection and authorization. This guide describes main `60c3798`, checked on 2026-09-14.

The viewer, stored-artifact adapter, native scoped loader, and contextual request client are implemented. **Current native startup supplies an unavailable contextual producer**, so requesting a fresh Sprint-context review returns `not_ready`. The producer's implementation and recorded routes do not establish an available production flow.

## Review interaction

The changed-file list shows additions, deletions, and change kind. Selecting a file resets to **Changes**, where **Unified** and **Split** display the same supplied diff. **File** shows complete text or rendered Markdown. Unchanged context can be expanded; renamed files retain both current and previous display paths.

Binary and unsupported content have named states. A successful empty snapshot shows **No changed files**; a failed load shows **Review unavailable**. Markdown reuses the product renderer with raw HTML skipped. The viewer exposes no edit, stage, discard, direct path-open, or filesystem-read action.

The application can open the viewer for a scoped document or a particular changed-file evidence destination. Navigation retains the originating product location so Back can restore it. An asynchronous contextual request also checks that its originating navigation is still current before opening the result. These navigation capabilities do not imply that every production context supplies a source.

## Source contract and authorization

A `FileReviewSource` is already scoped when it reaches the viewer. Display paths are labels, not filesystem handles, and the viewer does not enumerate repositories or choose among unrelated sources.

For stored application material, the adapter resolves an authorized changed-files Document and its artifact. It rechecks the Document/artifact identities and exact changed-file set, validates UTF-8 and the versioned JSON contract, and applies a configurable artifact byte limit (default 1,000,000). Text and Markdown facts must be complete: line numbers, hunk ranges, addition/deletion counts, and full content must agree. The contract has no partial-hunk success state.

Authorization, missing artifacts, mismatched identities, size limits, and invalid content remain separate adapter errors. The presentation reports a load failure without inventing missing content. An unsupported file is different from an unreadable or unauthorized review artifact.

## Native contextual boundary

The contextual client sends only a Sprint ID to `request_contextual_file_review`. An available native service would resolve the stored Sprint authority, reauthorize its repository/worktree comparison, produce a durable review artifact, and verify a scoped load before returning an opaque reference. Presentation cannot supply a private authority, arbitrary path, ref, or Git object to that action.

Current boot in [active_app.rs](../src-tauri/src/active_app.rs) installs `ContextualFileReviewTauriState::unavailable(...)`. Consequently the command returns `not_ready` before production. [transport.rs](../src-tauri/src/orchestration/transport.rs) also defines the available-service path and bounded results for an unready source, conflicting authority, an unavailable produced source, or another failure. The frontend client checks the first scoped snapshot before declaring a request ready.

The independent `load_scoped_file_review` command resolves an existing opaque reference through the orchestration repository. Its presence does not fill the missing originating service at boot. The native Git producer, reauthorization service, and their focused tests are implemented components with a narrower integration boundary than an end-to-end live Sprint review.

## Implementation map

| Concern                                                      | Owner                                                                                                                                                                                          |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scoped viewer facts and completeness checks                  | [fileReview.ts](../src/application/fileReview.ts)                                                                                                                                              |
| Authorized Document/artifact resolution and decoding         | [applicationOwnedFileReview.ts](../src/application/applicationOwnedFileReview.ts)                                                                                                              |
| List, diff layouts, file content, and empty/error states     | [FileReviewScreen.tsx](../src/features/fileReview/FileReviewScreen.tsx)                                                                                                                        |
| Markdown rendering                                           | [MarkdownContent.tsx](../src/components/MarkdownContent.tsx)                                                                                                                                   |
| Native contextual client and scoped ports                    | [tauriContextualFileReview.ts](../src/infrastructure/fileReview/tauriContextualFileReview.ts), [tauriScopedFileReview.ts](../src/infrastructure/fileReview/tauriScopedFileReview.ts)           |
| Origin, exact file selection, stale-request guards, and Back | [App.tsx](../src/app/App.tsx), [productNavigation.ts](../src/application/productNavigation.ts)                                                                                                 |
| Native originating authorization and Git production          | [file_review_originating_entry.rs](../src-tauri/src/orchestration/file_review_originating_entry.rs), [file_review_git_producer.rs](../src-tauri/src/orchestration/file_review_git_producer.rs) |
| Scoped persistence/read and commands                         | [repository.rs](../src-tauri/src/orchestration/repository.rs), [transport.rs](../src-tauri/src/orchestration/transport.rs)                                                                     |
| Actual frontend and native composition                       | [productApplicationComposition.ts](../src/bootstrap/productApplicationComposition.ts), [active_app.rs](../src-tauri/src/active_app.rs)                                                         |

## Evolution and evidence

The July 17 exploration at `9de25c2` established the reusable viewer and safe display boundary using five recorded source classes. Its delegated producing task, `019f6e52-184d-7573-af77-479f9883223f`, records the original scope at raw line 9 and acceptance with an outstanding empty-source gap at line 778. This was exploration acceptance, not native-product validation.

Continuation `019f6ed6-ee68-79d1-b88e-80505d13a163` reports `ba130cf` at raw line 499: Document/artifact loading, Sprint Documents entry, and explicit identity, size, encoding, unavailable, and empty-source handling. Git history records the later scoped-source change at `d4eb0de`. The old source dropdown and endless-loading defect therefore no longer describe the current contract; native producer availability remains a separate question with the current limit stated above.

For recorded inspection, [ApplicationRoot](../src/app/ApplicationRoot.tsx) supports the development route `?file-diff-viewer`, with `file-review-fixture=staged`, `commit-range`, `generated`, or `application-owned`; the default is `working-tree`. Each name selects recorded facts and does not read live Git. See [development](development.md) for starting the current app and [the offline image index](../offline-review/README.md#file-review-exploration) for the dated captures.

The August 5 recorded Work Unit review exercised an available file destination, an unavailable destination that did not navigate, and exact Session/invocation return. It also distinguished available and unavailable typed test evidence. These are presentation and focused-test results, not live-provider or production-persistence proof. [Validation evidence](validation-evidence.md) retains their scope. The original records are recoverable at `e2bfc6c:docs/orchestration/file-diff-viewer-exploration.md` and `e2bfc6c:offline-review/main-application/WORK-UNIT-REVIEW-EVIDENCE.md`.
