# Job Agent OTP curation validation

Date: 2026-09-30. Branch: `codex/OTP-Job-Agent`. Base: `917c07f8`.

The old branch's OTP work was already hand-integrated into main. Its original tip is preserved as `codex/OTP-Job-Agent-before-curation-rebase`; the new worktree starts at current main without replaying those superseded commits.

## Automated checks

- Frontend typecheck and production build passed.
- Rust check passed. The subsequent complete debug app build includes the transport and cleanup corrections and passed.
- Focused OTP Rust suite: 22 passed, including the opt-in real Job Agent host test.
- Harness proxy suite: 11 passed, including descriptor metadata filtering and resource-link/result forwarding.
- Frontend OTP detail, MCP picker, and Workflow selection tests: 7 passed.
- Catalogue fixture matches the actual package registry; whitespace checks passed.

The real-host test uses one synthetic job and a synthetic profile in `.dev/curation-proof/job-agent`. It exports the dataset, reads the local record, observes an invalid-result response, writes the fixed result, saves and retrieves the curation, stops the host, and verifies temporary export deletion while the saved curation remains.

It exposed two adapter problems: the current Job Agent SDK defaults to SSE responses, and immediate process termination bypassed scratch cleanup. The adapter now accepts JSON or SSE with complete JSON-RPC results preserved, and closes stdin before a bounded three-second graceful-exit window.

## Isolated application proof

The app uses its own application identifier (`com.orchid.otpcuration.proof`) and database under `.dev/curation-proof/app-data`. Product commands configure the synthetic Job Agent installation, register the existing native Codex home, create the curation-only Capability Profile, and register the synthetic workspace. No Job Agent source or real job records are changed.

Current main explicitly exposes only `danger-full-access` for native Codex sessions. Therefore the application proof uses that baseline policy. It cannot establish workspace-write sandbox isolation; the earlier plan assumed that policy was available. No sandbox-policy expansion belongs to this OTP slice.

Agent-session outcome: passed. Session `d7784ee4-bb15-4f3d-99e7-014c4b756bb8`, invocation `022c6b68-121d-4e04-b3f8-5e260eb91359` completed with `OTP_CURATION_PASS`. All three curation MCP calls completed through the Harness. Export results retained resource links, `com.orchid/resultHandling`, and `com.orchid/resultStats`. The agent processed the local JSONL, wrote the fixed result, and persisted `curation-20260930133800Z-ddbeaa97` containing one synthetic role.

The app was closed gracefully through its owned Windows process after verifying the executable path. The exported scratch directory was removed; the saved curation and compact proof receipt remain. The frontend window-close command is not permitted by the current Tauri capability configuration; no permission expansion was needed.

Initial attempts were corrected using existing product configuration: the persisted execution binding normalizes the device display name to `This device`; branch target discovery requires `refs/heads/main`; the baseline rejects workspace-write because it exposes only full access. These observations did not require OTP code changes. Raw local evidence is retained under `.dev/curation-proof/` and is intentionally untracked.

## Limits

This slice provides local artifact-backed curation and metadata. It adds no artifact viewer/store/transfer, remote filesystem access, Workflow routing semantics, or temporary export sharing across nodes. Descriptor tests and product-command proofs do not constitute a visual UI audit.
