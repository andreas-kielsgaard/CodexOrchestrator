# Operation trace: File Review production and display

## Product shape

File Review has three related but distinct uses:

1. a release-capable viewer for an already persisted opaque review artifact;
2. a contextual Sprint request whose producer is composed only in debug builds in the baseline;
3. an internal productive evidence producer used by Work Unit execution and accepted-candidate authority.

Treating “File Review” as one reachable feature hides these differences.

## Data contract and viewer

The frontend application contract in `applicationOwnedFileReview.ts` models a changed-files document plus a stored artifact. `FileReviewScreen.tsx` presents file navigation and content/diff evidence.

The Tauri adapter accepts one opaque reference only. It does not accept document IDs, artifact IDs, paths, refs or Git objects from the frontend. `load_scoped_file_review` resolves and reauthorizes the reference through durable Epic/Sprint/provenance relationships and returns the bounded document/artifact payload.

The TypeScript adapter loads once before reporting a contextual source as ready, then caches that first snapshot for the viewer.

## Stored artifact model

The active database keeps:

- `file_review_documents` for ownership, title, summary, opaque reference and fingerprint;
- `file_review_changed_files` for ordered normalized membership;
- `stored_file_review_artifacts` for the bounded serialized payload;
- `file_review_git_capture_authorizations` for exact Git source authority;
- `file_review_git_capture_documents` for authorization-to-document linkage.

The stored payload is capped at one megabyte. The Git producer limits changed-file count, list bytes, file bytes and text lines.

## Hardened Git production

`produce_file_review_from_git` accepts only a private capture-authorization ID. It reloads exact repository/worktree/baseline/current authority from SQLite and performs a hardened Git capture with inherited Git environment/configuration, hooks, credentials, replacement objects, optional locks and external diff disabled.

It creates stable document, artifact and opaque IDs from the authorization identity, validates the complete artifact and stores all facts idempotently. A repeated exact capture returns the same opaque reference; conflicting replay fails.

## Contextual Sprint request

The frontend calls `request_contextual_file_review` with only a Sprint ID.

When a producer is available, the backend:

1. loads the initiated Sprint Git authority;
2. reauthorizes it against a current Worktree Runtime Git comparison;
3. creates a private capture authorization;
4. produces and persists the bounded Git review;
5. reloads it through the scoped opaque-reference path;
6. returns `available` only after production and reauthorization both succeed.

The frontend then loads the same opaque source before presenting the viewer.

### Release/debug boundary

In debug composition, Human/Worktree Review supplies the verified runtime comparison service and the contextual producer is available. In release composition, `ContextualFileReviewTauriState` is explicitly unavailable, so the command returns `not_ready`.

The command remains registered in release, and `load_scoped_file_review` remains functional. Registered request, producer availability and stored-artifact viewing are therefore separate facts.

## Work Unit evidence path

Execution Support uses the same Git producer without going through the contextual Tauri command:

1. validate the exact authorized Implementer workspace and clean committed candidate;
2. create a stable capture authorization from attempt capability and current object;
3. produce the File Review artifact;
4. immediately reload it through the opaque scoped path;
5. convert its changed files and payload into the Implementer evidence package;
6. store the capture authorization on the implementation outcome;
7. require exact linkage again before accepted candidate pinning.

This path is productive release backend functionality even though the contextual viewer producer is debug-only. File Review is therefore both a user-facing evidence format and an internal application evidence protocol.

## Authority distinctions

| Boundary | Caller supplies | Application derives/validates |
| --- | --- | --- |
| contextual frontend request | Sprint ID | initiated authority, runtime comparison, paths and Git objects |
| scoped load | opaque reference | document/artifact/provenance linkage |
| internal Git producer | capture authorization ID | repository/worktree/object identities and artifact IDs |
| Work Unit evidence | attempt capability | capture authorization, manifest, comparison and content fingerprints |

## Product/architecture reading

- The opaque-reference contract is a strong reusable boundary and prevents the UI from becoming a Git authority surface.
- The same stored format supports user review and machine-governed acceptance evidence.
- Contextual production currently depends on debug review infrastructure, while execution capture does not. Those are separate composition choices rather than one missing producer implementation.
- The viewer can be product-ready even when the entry action is unavailable, but UI copy and navigation must not imply that a new review can always be produced.
- Future extraction should preserve the private capture-authorization boundary and hardened Git runner.

## Questions to carry forward

- Should a release-safe Git comparison provider replace the debug Worktree Review dependency for contextual production?
- Is one stored format appropriate for both human review and acceptance evidence long term?
- What are the retention and privacy expectations for stored file contents?
- Should File Review be entered from Sprint, Work Unit evidence, Agent Session activity, or all three with explicit provenance?
- Which review states should be preloaded, refreshed or treated as immutable snapshots in the UI?
