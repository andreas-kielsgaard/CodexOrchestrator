# NCHP audit-readiness closure

Status: documentation-only packaging for the frozen product checkpoint `6fd276fc3ef733d315d8199c79b431b6d18abfab` (tree `2fb8c23b1d791f0ce3269ebee7c2a48bf2b08f61`). The retained source chain is tree-equivalent at `2ba9bed7c89f4dc6c3e5a5ac3b59322f778ad211`.

## User-facing capability

The product can register or create a native Codex home, select it, and review its separate facts: authentication; sandbox setup or external adoption; Workspace Write proof; execution mode and Danger authorization; Full Access proof; MCP/reporting readiness; and attention.

- Workspace Write is the safer option: it uses the assigned application root with restricted network access.
- Danger Full Access is a separate explicit choice and authorization. It permits commands to read and write outside the assigned worktree and affect the full machine under the launching user’s OS rights, with unrestricted network access. It does not grant administrator rights or suppress Windows UAC.
- Option C is deferred and outside this Slice.

These facts are not interchangeable: a request, launch acceptance, process outcome, provider activity, receipt, readiness result, authorization, and workflow completion each retain their own boundary.

## Durable component inventory

Grouped by responsibility:

- Profile identity, selection, filesystem continuity, lifecycle, and attention.
- Restart-safe login and sandbox setup attempts with safe provenance, launch/terminal facts, recovery, and browser/UAC separation.
- External sandbox observation and explicit product adoption confirmation.
- Execution mode plus versioned, profile-bound Danger authorization and revocation.
- Workspace Write and Full Access canary evidence: bounded roots, process activity, provider observation, owned receipt, cleanup, and readiness.
- MCP/reporting probe authority, correlation, expiry, and readiness.
- Migrations through v34, including historical-state preservation and v33 cold-open admission.
- Strict native query/client/consumer/Settings projection and malformed-state rejection.

## Evidence matrix

| Boundary | Evidence | Truth retained |
| --- | --- | --- |
| Deterministic source evidence | Frozen checkpoint `6fd276f`; retained chain tree `2fb8c23b`; native-profile Rust suite 69/69; v33 cold-open regression 1/1; focused TS/UI 17/17; lint and production build passed | Durable contracts, migration behavior, projection validation, and build integrity; no live-provider claim |
| Workspace Write controlled-live proof | Accepted bounded canary and cold reopen | Restricted-network Workspace proof and receipt/readiness facts were observed separately |
| Danger controlled-live proof | One accepted request under the current v1 full-machine/unrestricted-network authorization; launch accepted, process terminal observed, exit 0, owned receipt observed, cleanup removed, Full Access passed, then cold reopen | Full Access proof is accepted; provider activity and UAC interaction remain separate and unobserved where stated |
| Cold reopen | Accepted reopen evidence for Workspace and Danger facts | Durable facts survived restart without inferring new activity |
| Unobserved facts | Browser handoff, product UAC interaction, provider activity where not directly observed, MCP/reporting in the retained controlled-live profile, and downstream workflow semantics | Unobserved remains unobserved; no credential, URL, transcript, provider-private, or opaque sandbox material is exposed |

## Residual register

- Browser login child terminal succeeded; browser handoff unobserved.
- Externally elevated sandbox adopted/confirmed; product UAC unobserved.
- Retained CLI attention `codex_cli_workspace_semantic_policy_unsupported` despite accepted bounded Workspace canary/launch projection.
- MCP/reporting not assessed in retained controlled-live profile.
- Downstream workflow semantics not exercised and outside Slice.
- Settings exposes current/latest setup attempt, not a user-facing history.
- Current checkpoint packaging/installer UI proof is not newly claimed unless already directly evidenced at this exact checkpoint.

## Complexity and debt map

Profile identity, selection, lifecycle, authorization, filesystem continuity, and bounded roots protect real authority boundaries. Attempt records, correlations, process/receipt/cleanup facts, adoption confirmation, and migrations protect restart safety, privacy, and truthful chronology. Strict DTO and consumer validation protects the frontend from contradictory or private state.

The chronology-heavy attempt and evidence families primarily support auditability and recovery. Their separate schemas, migrations, query validators, client decoders, and Settings copy increase coupling and maintenance cost, especially when a new authority or execution mode is introduced.

## Later, non-authorizing simplification candidates

Only after an audit-approved refactor, the routine UI could center on current readiness plus one evidence summary, with detailed chronology behind a technical/audit disclosure. Fact families could be consolidated only if every authority, privacy, restart, and unobserved boundary remains explicit. No additional schema, DTO, or state family should be added without a concrete user decision.

## Freeze and truth statement

No refactor is implemented or authorized here. The listed residuals are acceptable for closure review within this Slice. Publication and canonical integration remain separate decisions and are not claimed by this document.
