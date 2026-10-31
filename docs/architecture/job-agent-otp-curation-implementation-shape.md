# Job Agent OTP artifact-backed curation

Status: implemented and locally validated. See `job-agent-otp-curation-validation.md` for evidence and the baseline sandbox-policy limitation.

## Target

Expose Job Agent's `export_curation_dataset`, `create_curation`, and `get_curation` through the existing Job Agent OTP. Preserve the declared MCP result-handling metadata through discovery and calls, explain it in endpoint details, and prove the local export-to-curation flow through an Orchid-owned session.

The inspected Job Agent contract is commit `c4fcc11` on `feature/mcp-local-harness`. Read the implementation when starting work; the earlier paged-context proposal was superseded.

## Ownership

- Job Agent owns structural dataset selection, export files, integrity checks, curation validation, persistence, and scratch cleanup.
- The Job Agent OTP describes the available endpoints and their usage. Tool selection continues to determine which capability groups the session's managed MCP host receives.
- Orchid's OTP contract carries endpoint metadata. The Job Agent facade exposes the configured catalogue and forwards calls. Harness filtering preserves MCP descriptors and results.
- Existing endpoint detail components explain recognized result-handling metadata. Workflow routing remains unchanged.

## Current gaps

`src-tauri/src/otp_packages/job_agent/mod.rs` declares six source-related groups and no curation endpoints. Its schema helper treats arrays as arrays of objects. Its annotation helper infers read-only behavior from endpoint names; export needs explicit classification.

`AgentMcpToolDescriptor` carries input/output schemas and annotations but no custom metadata. The frontend DTO omits both metadata and annotations. The facade synthesizes `tools/list` without `outputSchema` or `_meta`; call results are already forwarded as JSON. The Harness filters selected tools while preserving their other fields.

Job Agent returns compact structured metadata and `resource_link` blocks. The export descriptor and successful result carry `com.orchid/resultHandling`; results also include `com.orchid/resultStats`. Full job records and profile contents are already kept out of inline responses.

Job Agent's `orchid_host` announces `artifactRoot`; Orchid currently does not retain it. Export URIs refer to the Job Agent machine's filesystem. Discovery is configured information and must remain available without starting or contacting that server.

## Proposed changes

### 1. Extend the shared endpoint descriptor

Edit `src-tauri/src/otp_api/contract.rs`:

- Add an optional, omitted-when-empty namespaced metadata map to `AgentMcpToolDescriptor`, serialized as `meta` in the OTP catalogue and `_meta` when constructing an MCP tool.
- Keep `ToolAnnotations` as the existing standard annotation representation. Do not put result handling into standard annotations.
- Keep output schemas truthful: success fields and domain-error fields must match the actual endpoint responses. Publishing an output schema creates a runtime expectation, so do not blindly publish the current coarse success-only schema for every source endpoint.

Edit `src/application/otp/contracts.ts` to carry metadata and the existing optional MCP annotation fields. Update descriptor constructors and fixtures affected by the Rust field addition. Preserve existing packages with empty metadata.

Use existing `serde_json`, `rmcp` annotations, and MCP content representations. The real-host check required a direct dependency on the already-transitive `sse-stream` parser: Job Agent's current HTTP transport returns SSE. No alternate artifact protocol is needed.

### 2. Add the curation package definitions

Create `src-tauri/src/otp_packages/job_agent/curation.rs` for the three endpoint descriptors, explicit schemas, annotations, and usage guidance. Keep source endpoint definitions in `mod.rs`; no broad catalogue rewrite.

Edit `src-tauri/src/otp_packages/job_agent/mod.rs`:

- Add the `curation` capability group and compose its descriptors into the existing server catalogue.
- Include these endpoint IDs in selected-tool validation and capability resolution. They require no source mutation grants.
- Update package/server summaries to cover source setup and job curation.
- Keep existing source endpoints and their grants unchanged.

Use the implemented signatures:

| Endpoint | Inputs | Result and behavior |
| --- | --- | --- |
| `export_curation_dataset` | Optional string arrays `job_refs`, `application_statuses`, `posting_statuses`, `source_ids`, `evidence_levels`; optional strings `observed_from`, `observed_to` | Creates a temporary export; compact result includes `export_id`, counts, filters, lifetime, `result_uri`, and file metadata/resource links. |
| `create_curation` | Required `export_id` | Validates the fixed result file and persists an advisory curation; returns identity, counts, summary, and view URL, or a domain validation error. |
| `get_curation` | Required `curation_id` | Returns the saved advisory curation, or a domain error. |

Do not include speculative filters from the earlier discussion. Export and create are non-read-only, non-destructive, non-idempotent, and closed-world; get is read-only and idempotent.

Copy the implemented export metadata:

```json
{
  "com.orchid/resultHandling": {
    "disposition": "artifact-required",
    "modelIngestion": "avoid",
    "contentTypes": ["application/x-ndjson", "application/json"],
    "cardinality": "many"
  }
}
```

Usage guidance should explain local streaming analysis, bounded derived output, the fixed `curation-result.json`, retrying validation with the same export ID, and host-lifetime exports. Avoid instructions to ingest the complete export.

### 3. Preserve the descriptor and results through the facade

Edit `src-tauri/src/otp_host/job_agent.rs`:

- Centralize descriptor-to-MCP JSON construction in a small helper used by `tools/list`.
- Publish `_meta` where present and accurate curation output schemas. Audit the existing source schemas before exposing them; leave them catalogue-only if they cannot truthfully describe runtime results in this slice.
- Preserve unknown metadata keys and all returned content blocks, structured content, result metadata, and error information.
- Retain configured discovery and lazy backend startup on the first call. An unavailable installation must remain a tool-call error, not a design-time or session-launch blocker.
- Continue using one managed backend for the related export and create calls. Do not restart it between those calls.

Retain `src-tauri/src/harness_engine/proxy.rs` behavior unless tests reveal a preservation failure. Extend its existing preservation tests with `_meta` and resource-link results rather than adding a competing proxy.

### 4. Explain result handling in the existing endpoint details

Edit `src/features/otp/otpElements.tsx`; create a focused sibling metadata presentation helper only if needed:

- Show a Result handling section for the recognized `com.orchid/resultHandling` key.
- Explain disposition, model-ingestion guidance, content types, and cardinality in plain language.
- State in export usage text that the files are temporary and local to the MCP host.
- Reuse the same endpoint detail renderer in OTP details and endpoint selection. Keep the existing group/endpoint hierarchy.
- Preserve unfamiliar metadata without assigning it invented behavior. Do not add a generic metadata editor.

Update `src/features/workflowAuthoring/otpCatalogue.fixture.json` from the actual catalogue and adapt relevant OTP selection/detail tests. No new Workflow trigger or destination is required for these agent MCP endpoints.

### 5. Prove local access and lifecycle

Run a disposable local curation session through Orchid with the three tools selected. Use a disposable Job Agent root containing minimal synthetic records, configured as both the Job Agent installation root and the session workspace, under workspace-write policy. This puts host-created exports inside the existing writable workspace without widening sandbox access.

Through the actual agent/runtime path:

1. Discover the configured tools and export metadata.
2. Export a small dataset and observe resource links plus compact metadata.
3. Process the export using local scripts with bounded output.
4. Write the fixed result file and call `create_curation`.
5. Read it with `get_curation` and verify normal persisted curation data.
6. Release the owned MCP host and verify scratch removal while the saved curation remains.

Capture cleanup failures before deciding on changes: `JobAgentHostOwner::stop` currently closes stdin and may immediately kill the child. If this prevents Job Agent's normal cleanup, adjust this owner to allow bounded graceful termination before forced shutdown, using the product's existing process-lifecycle utilities.

The real-host check reproduced scratch surviving immediate termination. The owner now closes stdin and allows three seconds for natural exit before killing. Existing supervisor utilities own supervised processes rather than this adapter's `Child`; the bounded wait stays in this owner without introducing a separate supervisor.

The proof establishes the local same-workspace flow. Arbitrary session workspaces and remote execution require a separate artifact-access design. Keep this limitation explicit in endpoint usage; do not silently widen access or start the backend at launch just to obtain `artifactRoot`.

Current main exposes only `danger-full-access` for native Codex sessions as a temporary product policy. The isolated app proof therefore uses that existing policy and a registered synthetic workspace. It proves the local artifact flow, but not workspace-write isolation. Sandbox policy remains outside this slice.

## Validation

- Rust package tests: all three endpoints belong to `curation`; string-array schemas and annotations match the implementation; selected endpoints produce the capability without source grants.
- Facade tests: discovery works with an unavailable Job Agent; metadata/output schemas serialize correctly; calls preserve resource links, result statistics, and domain errors.
- Harness tests: selection filtering preserves endpoint `_meta` and forwarded artifact results.
- Frontend tests: Curation appears in package browsing and MCP selection; endpoint details explain result handling; source-only endpoints render normally.
- Run the relevant focused Rust tests through `scripts/build-tools.mjs`, focused Vitest tests, `npm run build:frontend`, and `npm run check:rust`. Build the debug app for the live proof using `npm run build -- --debug`.
- Keep live evidence separate from descriptor/unit-test evidence. Use only disposable Job Agent data; avoid real saved jobs, profiles, curations, and source transitions.

## Exclusions and decisions

Agent-made choices: use one optional metadata map; keep curation schemas in a focused package file; prove access with a shared local workspace; condition cleanup changes on observed failure.

Unresolved follow-up: artifact presentation/interception, access from arbitrary workspaces, remote transfer, and reuse of temporary exports across workflow nodes. None is needed for the defined local proof.

Approved direction: add the three implemented endpoints and Curation group, preserve namespaced metadata through discovery/results, explain it in existing endpoint details, and keep configured design access independent of server availability.

Do not add legacy curation aliases, new OTP categories, general filesystem tools, an artifact store, artifact transport, or implicit workflow routing. No Job Agent source changes are planned; its current contract is sufficient for this slice.

Existing dirty agent-session target-operation work and its plan documents are unrelated and must be preserved.
