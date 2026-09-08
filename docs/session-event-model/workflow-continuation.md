# Workflow OTP and node file inputs

The product imports the locally compiled `workflow` package in `active_app.rs` through
`OtpRegistry::import(&["workflow"])`. The same declarations supply managed MCP exposure,
the designer catalogue, input validation and connection compilation.

## Package boundary

| Location under src-tauri/src | Responsibility |
| --- | --- |
| `otp_api/contract.rs` | Package/tool/output descriptors, invocation context, typed inputs and new/exact session requests. |
| `otp_api/handles.rs` | The curated product surface: bound node/connection definitions, node-session summaries and declared output emission. |
| `otp_host/` | Local imports, managed MCP transport, trusted context, scoped adapters and request execution through Session Events. |
| `otp_packages/workflow/` | The four Workflow behaviors; imports only the OTP API and libraries. |
| `workflows/` | Recipe compilation, instance state, centralized event observation, routing, input resolution and attempt records. |

Session requests are returned values, not another provider-launching API. A package requests
a new session for its bound node, or supplies that node and an exact session ID. The host
executes the request once using the shared Session Event dispatcher. Node initialization is
included only for a new session. Reads are scoped to the invocation's bound source/output
nodes and connection; session summaries contain selection facts, not transcripts.

| Tool | Entry | Output |
| --- | --- | --- |
| `trigger_workflow_continuation` | MCP, optional `outputFiles` array | `continuation`: `outputFiles`, `sourceNode` |
| `handoff_to_agent` | MCP, required `filePaths` array and `promptText` | `handoff`: `filePaths`, `promptText`, `sourceNode` |
| `on_invocation_completed` | Engine-observed terminal invocation | `completed`: final `output`, `sourceNode`; completed status only |
| `prompt_agent` | Routed action | Node-bound new/exact session requests |

The engine observes terminal facts and delivers them to configured package consumers. The
package decides whether to emit an output. MCP handlers explicitly emit their outputs;
neither path depends on observing arbitrary MCP calls or extracting tool response fields.

## Configure a connection

1. Enable `workflow / trigger_workflow_continuation` in the source node's
   Capability Profile and Node Profile. Existing Sessions retain their pinned profiles.
2. Bind the connection's source node and select **Workflow continuation** as its trigger.
   Select **Prompt agent** and the output node.
3. Add ordered prompt inputs:
   - **Output field**: `outputFiles` or `sourceNode`.
   - **Files associated with node**: select a node and Created, Edited, or Created or edited.
   - Explicit file content, plus the connection's fixed prompt text as needed.
4. Configure **Session mode**: `new` always creates; `select` applies ordering, cardinality,
   running/created-by filters and the create/fail/noop policy when no session matches.
5. Save and activate the recipe, then create an instance from that revision.

The node's initial prompt specifies when to call `trigger_workflow_continuation`.
The optional argument is `{ "outputFiles": ["docs/spec.md"] }`; `{}` is valid.
The call triggers only connections matching this tool and its trusted calling node,
within that Session's Workflow instance. Zero or multiple matches are supported.
It does not close the node or change its lifecycle.

## Offered data

The package declaration in `otp_packages/workflow/mod.rs` supplies the metadata. The
continuation output offers:

| Field         | JSON value                                          |
| ------------- | --------------------------------------------------- |
| `outputFiles` | Array of supplied path strings; empty when omitted. |
| `sourceNode`  | Object with the calling node's `id` and `name`.     |

The managed proxy supplies trusted Session/runtime/invocation headers separately from tool
arguments. The host checks the active invocation and pinned tool exposure, then derives
instance/node identity from the Session's stored logical address. Connections match the
source node and declared package/tool/output identity. Typed values stay JSON until the
package renders prompt contributions.

A node-file source resolves at delivery construction to:

```json
{
  "nodeId": "plan",
  "nodeName": "Plan",
  "association": "edited",
  "files": [
    {
      "path": "docs/plan.md",
      "exists": true,
      "created": {
        "nodeId": "overview",
        "nodeName": "Overview",
        "sessionId": "session-1",
        "invocationId": "invocation-1",
        "recordedAt": "2026-09-08T10:00:00Z"
      },
      "lastEdited": {
        "nodeId": "revision",
        "nodeName": "Revision",
        "sessionId": "session-3",
        "invocationId": "invocation-3",
        "recordedAt": "2026-09-08T10:05:00Z"
      }
    }
  ]
}
```

Plan remains associated with this file because its historical edits are retained.
Missing files remain listed with `exists: false`. Unknown attribution is `null`.
Paths are relative to the instance worktree; this input includes metadata, not file contents.

## Ownership and limits

- Codex JSONL normalizes successful `file_change` items into typed create/edit/delete records.
- Agent Sessions persist those records alongside their runtime event in one transaction.
  Historical queries include archived Sessions and stay within the requested logical scope.
- Workflow aggregates that history and resolves current existence before invoking the action.
  Prompt contributions and their input references remain visible in recorded deliveries.
- The generic Session Event kernel now has an explicit new-session target. Session selection
  policy lives in the package; existing generic exact/logical targeting remains available.
- Delivery retains existing Session rules: a target with an active invocation rejects a new
  delivery. This change adds no queue, interrupt, or revision-coordination behavior.
- Supplied output paths never establish authorship. Shell writes or older events without
  normalized file-change records have no inferred attribution. This implementation begins
  recording new file-change events; it does not backfill earlier runtime history.
- A reported delete affects existence when the file is absent, but does not count as an edit
  for selection. A later reported create at the same path replaces its creation attribution.

## Contract and evidence

Recipes use contract version 2. Older recipes/instances stay stored, are excluded from current
lists and return an explicit unsupported-contract error when loaded by ID. The retired
`workflow_handoff` server has no alias. Create new exploration recipes and profiles with the
`workflow` tools; existing Sessions retain their immutable profiles.

Imports are local compiled code. The current surface assumes product-owned runtime sessions;
it adds no permission framework, external loader, package manager, hot reload, extra tools,
approval enforcement or revision coordination. Configuration forms cover only this package's
text/enum fields; schema validation covers the object/array/string subset used here.

See [OTP validation](otp-exploration-validation.md) for automated, live-provider and visible
designer evidence and their separate limits.
