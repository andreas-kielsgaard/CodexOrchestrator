# Workflow continuation and node file inputs

## Configure a connection

1. Enable `workflow_handoff / trigger_workflow_continuation` in the source node's
   Capability Profile and Node Profile. Existing Sessions retain their pinned profiles.
2. Select **Workflow continuation** under the connection's **Workflow action**.
3. Add ordered prompt sources:
   - **Trigger field**: `outputFiles` or `sourceNode`.
   - **Files associated with node**: select a node and Created, Edited, or Created or edited.
   - Existing invocation output, application fields and explicit file-content inputs remain
     available for their corresponding triggers.
4. Save and activate the recipe, then create an instance from that revision.

The node's initial prompt specifies when to call `trigger_workflow_continuation`.
The optional argument is `{ "outputFiles": ["docs/spec.md"] }`; `{}` is valid.
The call triggers only connections matching this tool and its trusted calling node,
within that Session's Workflow instance. Zero or multiple matches are supported.
It does not close the node or change its lifecycle.

## Offered data

The Workflow-owned capability declaration in `workflows/trigger_capabilities.rs` supplies
the MCP metadata, authoring catalogue and field validation. Version 1 offers:

| Field         | JSON value                                          |
| ------------- | --------------------------------------------------- |
| `outputFiles` | Array of supplied path strings; empty when omitted. |
| `sourceNode`  | Object with the calling node's `id` and `name`.     |

The Harness injects trusted Session/invocation context; Workflow derives the instance
and node from the Session's persisted logical address. The tool implementation explicitly
dispatches the trigger. Raw MCP observation and response-field extraction are not involved.

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
- Workflow aggregates that history and resolves current existence. It compiles both new input
  types into existing Session Event referenced-content inputs; the generic kernel is unchanged.
- The Harness proxy supports multiple Workflow-participating tools on one managed server.
- Delivery retains existing Session rules: a target with an active invocation rejects a new
  delivery. This change adds no queue, interrupt, or revision-coordination behavior.
- Supplied output paths never establish authorship. Shell writes or older events without
  normalized file-change records have no inferred attribution. This implementation begins
  recording new file-change events; it does not backfill earlier runtime history.
- A reported delete affects existence when the file is absent, but does not count as an edit
  for selection. A later reported create at the same path replaces its creation attribution.

Focused coverage includes proxy-to-dispatch routing, fan-out, trusted-context rejection,
zero matches, historical editor selection after another editor, archival/reopen, instance
isolation, existence, unknown attribution, event transaction rollback, and designer save/reload.
