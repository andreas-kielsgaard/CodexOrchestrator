# Worktree Review product correction

Status: implemented and validated. Baseline: `c998c34645847b618ff08354988a219fd00d2d9e` from
`codex/worktree-review-runtime`. Correction branch:
`codex/worktree-review-product-correction`.

## Objective and completion boundary

Worktree Review is a product capability and must be usable in development, release, and packaged
applications. Only recorded fixtures, proof navigation, and diagnostic controllers are
development support.

The correction is complete when:

- Rust composition, product commands, state, and the product UI are present in debug and release;
- an installed or relocated executable does not derive its repository from `CARGO_MANIFEST_DIR`;
- missing repository, Git, toolchain, or storage prerequisites produce typed readiness rather than
  preventing application startup;
- retained-build isolation, exact reuse identity, and conservative cleanup remain intact;
- product application code is independent of proof-only commands and navigation;
- release compilation and a release Tauri build are exercised, with focused lifecycle tests; and
- the final diff has an explicit changed-file manifest and no unrelated baseline changes.

## Evidence baseline

- `cargo check --manifest-path src-tauri/Cargo.toml --release` fails because
  `worktree_review` is removed by `#[cfg(debug_assertions)]` while two handlers remain registered.
- `active_app.rs` also composes Worktree Review only in debug and installs an unavailable contextual
  review service in release.
- `ApplicationRoot.tsx` injects the launcher only under `import.meta.env.DEV`; `App.tsx` labels it
  `Worktree Review Dev` and also owns proof-navigation polling.
- composition derives the repository from the build-time Cargo manifest path and eagerly discovers
  toolchains, so release relocation and missing prerequisites are not represented as product state.
- the central launcher service combines source discovery, Git, persistence, lifecycle operations,
  settings, comparison/detail queries, and proof support.
- adjacent Worktree Review and Epic Origin modules implement overlapping Git/process discovery in
  separate places.

The baseline branch is intentionally broad. This correction preserves its history and evaluates
only the diff from the exact commit above.

## Architectural decision

Product composition owns an unconditional `WorktreeReviewClient`, product screen, and settings
surface. Tauri owns an unconditional `WorktreeReviewState` with explicit readiness variants and
unconditional product commands. The Worktree Review application depends on focused ports for
repository inspection, retained-instance runtime, and storage. Development proof composition adds
proof commands and controllers without changing product availability.

The child executable marker `VITE_HUMAN_REVIEW_INSTANCE` remains a runtime mode discriminator so a
launched review instance does not recursively show the launcher. It is not an availability gate.

## Outcomes and gates

### 1. Separate product and proof boundaries

Result: product modules and contracts compile unconditionally; proof controller, recorded fixtures,
proof commands, and polling live behind one explicit development boundary. Product evidence types
remain product types even if they originated in proof work.

Gate: product command inventory is explicit and tests no longer assert development-only product
availability.

### 2. Establish cohesive application boundaries

Result: SQLite persistence, retained-instance runtime translation, proof support, readiness, and
transport have focused owners. A Worktree Review-owned runtime port hides `WorktreeTest*`
vocabulary and the transport layer only translates commands and structured errors. Source and
lifecycle orchestration remain together in the product service; this is an explicit residual.

Gate: the application service does not execute SQL or expose the adjacent runtime facade directly.

### 3. Share repository infrastructure without merging domains

Result: one repository-context boundary owns Git execution, executable discovery, canonical
repository/worktree identity, porcelain parsing, refs, and ancestry. Worktree Review and Epic Origin
retain separate domain policies and consume those primitives.

Gate: low-level read operations in the touched paths use the shared runner. Mutating worktree
attachment and the native folder picker remain explicit exceptions.

### 4. Add product repository selection and readiness

Result: one persisted active repository is identified by stable opaque identity. Historical
instances remain associated with their repositories when selection changes. Missing repository,
toolchain, Git, or storage is a typed recoverable state; composition is lazy relative to application
startup, though toolchain discovery within composition remains all-or-nothing.

Gate: no production path depends on the build-time manifest directory, and an unmatched retained
record fails closed instead of being silently reassigned.

### 5. Promote Rust and Tauri composition to release

Result: Worktree Review module, runtime adapter, state, product handlers, and contextual file review
composition are unconditional. Only proof handlers are conditional, with structured errors at the
transport boundary.

Gate: debug and release Cargo checks pass and release command registration is covered by tests.

### 6. Promote the frontend through normal product composition

Result: product composition supplies a typed client; the main application shows Worktree Review in
all builds without a `Dev` label. The child-instance branch stays isolated. Technical settings owns
review settings explicitly rather than accepting an arbitrary injected React node. The launcher is
retains its existing source, instance, operation, and dialog components and adds a dedicated
readiness screen.

Gate: no `import.meta.env.DEV` controls product availability and no product component polls proof
navigation.

### 7. Harden release evidence

Result: repository scripts distinguish debug Rust, release Rust, product validation, and release
validation. Evidence covers frontend build/tests, Rust tests, Cargo release, Tauri release build,
release product-command availability, proof-command absence, missing prerequisites, retained
lifecycle/cleanup, contextual file review, and Windows process ownership. Path, ref, attachment,
receipt, deletion, and WebView inputs receive a focused security audit.

Gate: failures are classified as candidate defects or explicit environmental prerequisites; green
compilation is not treated as functional proof.

### 8. Converge and hand off

Result: latest `main` is assessed deliberately, the changed-file manifest and diff hygiene are
clean, and the exact final commit and validation evidence are recorded.

Gate: remain on the correction branch. Do not merge, push, retire worktrees, or claim acceptance
without separate authority and evidence.

## Dependency and parallelism

The product/proof decision precedes contract extraction. Application boundaries precede shared
repository and readiness composition. Once Rust/frontend DTOs are frozen, backend release
composition and frontend promotion can proceed in parallel. All paths converge before release and
lifecycle evidence.

`1 -> 2 -> 3 -> 4 -> (5 || 6) -> 7 -> 8`

## Preserved safety contracts

- detached identity comes from Git porcelain and opaque refs, never ambient branch assumptions;
- build reuse requires exact source, toolchain, and artifact identity;
- mutable retained output stays under a private instance root; shared locations are caches only;
- detached-build cleanup defaults off and is limited to Prepared, Stopped, or Recovered instances;
- cleanup removes only the private instance root and authorized runtime record, then releases ports;
- repository switching cannot rewrite historical instance ownership.

## Meaningful deviations and residual work

- Existing `human_review_*` Tauri command strings remain as a compatibility protocol; product code
  and types use Worktree Review terminology.
- The product service is materially smaller and proof/SQL/runtime-free, but still coordinates
  catalog, lifecycle, and query operations in one large file.
- `repository_context/mod.rs` still groups commit, ref, and change reads; the runner has output
  bounds but no wall-clock timeout.
- Mutating `git worktree add` and the PowerShell repository picker have not yet moved behind the
  hardened repository runner.
- Toolchain discovery is deferred until Worktree Review readiness is requested, but missing
  Node/Cargo/Rust still prevents read-only browsing.
- Historical source-ref hashing is isolated by per-repository storage, but repository identity is
  not encoded in the source ref itself.
- The launcher remains large after proof polling and settings ownership were removed. Further UI
  decomposition should follow behavior seams, not this release correction.

## Current next-ready work

No correction outcome remains open. The residuals above are independently actionable and are not
required to restore Worktree Review as a release product capability.

## Progress log

- 2026-08-25: exact baseline verified clean; correction branch and worktree created; plan recorded.
- 2026-08-25: product/proof boundary inverted; product state, repository selection, structured
  readiness, and release handlers implemented.
- 2026-08-25: storage, runtime translation, proof support, and shared read-only repository context
  extracted; frontend promoted through normal product composition.
- 2026-08-25: focused Rust/frontend/runtime suites, debug/release checks, packaged Tauri build, and
  relocated release startup passed. Two unrelated frontend baseline assertions remain red.
