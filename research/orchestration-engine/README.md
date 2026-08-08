# Orchestration engine research

This repository area documents the current and nearby implementation of the capability currently known as the Epic workflow from product, architecture, implementation and experience perspectives.

## Start here

1. Read [Current orchestration product and system](current-state/README.md) for the shortest supported explanation.
2. Read [Near-future and moving work](current-state/near-future-and-moving-work.md) for descendant, sibling and uncommitted implementation that is locally present but not part of the stable snapshot.
3. Open the [interactive insight atlas](visualizations/product-map/README.md) to review the current explanation visually.
4. Continue through a role reading, catalogue or operation trace only when more depth is useful.

## Directory guide

| Directory | Contents | Use it when |
| --- | --- | --- |
| `current-state/` | Concise current product/system shape and locally present near-future work | Orienting yourself or handing the research to another agent |
| `visualizations/` | Interactive and rendered models derived from the current research | Reviewing the explanation and giving directional feedback |
| `catalogs/` | Capabilities, code locations, frontend views, Tauri operations, MCP, Harness, processes and durable state | Finding what exists and where it is implemented |
| `operation-traces/` | End-to-end paths through representative operations | Understanding how frontend, Rust, persistence, MCP and effects connect |
| `evidence-passes/` | Focused behavioral evidence, exceptions and snapshot-qualified findings | Checking a consequential behavior or qualification in depth |
| `discovery-sweeps/` | Completed broad trigger/effect sweeps that selected deeper investigations | Auditing coverage or returning to raw discovery evidence |
| `perspectives/` | Product-owner, architect, developer and designer readings | Interpreting the implementation for a particular role |
| `history/` | Capability chronology, decisions and materially different implementation lines | Understanding how the current and nearby states arose |
| `research-context/` | Evidence scope, checkout topology, coverage and presentation preferences | Checking research boundaries or source-state provenance |
| `questions/` | Open questions that could change current or near-future interpretation | Choosing a useful follow-up investigation |

The concise current-state reading is intentionally more legible than exhaustive. Detailed evidence preserves qualifications where they change the meaning of a capability. Code presence, build inclusion, UI reachability, deterministic tests, live observation, integration and product acceptance remain separate facts.

## Evidence library

### Current and nearby state

- [Current orchestration product and system](current-state/README.md)
- [Near-future and moving work](current-state/near-future-and-moving-work.md)
- [Visualization index](visualizations/README.md)
- [Interactive orchestration insight atlas](visualizations/product-map/README.md)

### Foundation and history

- [Research evidence universe](research-context/evidence-universe.md)
- [Source-state inspection register](research-context/inspection-register.md)
- [Research checkout and repository topology](research-context/repository-topology-and-checkout.md)
- [Presentation and representation lenses](research-context/representation-lenses.md)
- [Research coverage and evidence gaps](research-context/coverage-status.md)
- [Material implementation lines](history/implementation-lines.md)
- [Capability chronology](history/capability-chronology.md)
- [Decision-record evolution](history/decision-record-evolution.md)

### Capability and artifact catalogues

- [Capability landscape](catalogs/capability-landscape.md)
- [Code artifact map](catalogs/code-artifact-map.md)
- [Frontend experience map](catalogs/frontend-experience-map.md)
- [Rust backend and Tauri boundary](catalogs/backend-and-tauri.md)
- [Tauri operation catalogue](catalogs/tauri-operations.md)
- [MCP servers and tools](catalogs/mcp-servers-and-tools.md)
- [Conversation Harness and configuration authority](catalogs/harness-and-configuration.md)
- [CLI, process and environment surfaces](catalogs/cli-process-and-environment.md)
- [Durable state and artifact ownership](catalogs/durable-state.md)

### Representative operation traces

- [Agent Session message](operation-traces/agent-session-message.md)
- [Plan Builder proposal and initiation](operation-traces/plan-builder-proposal-and-initiation.md)
- [Orchestration native query](operation-traces/orchestration-native-query.md)
- [Work Unit execution, review and settlement](operation-traces/work-unit-execution-review-and-settlement.md)
- [Native Profile readiness and launch](operation-traces/native-profile-readiness-and-launch.md)
- [File Review production and display](operation-traces/file-review-production-and-display.md)

### Behavior-led evidence

- [Evidence pass index](evidence-passes/README.md)
- [Cross-cutting system findings](current-state/cross-cutting-system-findings.md)
- [Completed discovery sweeps](discovery-sweeps/README.md)

### Role-oriented readings

- [Product owner](perspectives/product-owner.md)
- [Product architect](perspectives/product-architect.md)
- [Expert developer](perspectives/expert-developer.md)
- [Expert designer](perspectives/expert-designer.md)

### Unresolved work

- [Open questions](questions/open-questions.md)

## Extending this snapshot

Future agents can add evidence where it naturally fits and revise the current/near-future reading when the implementation changes. Linking a new finding into the concise reading is usually more useful than creating another parallel summary. These are editorial preferences for this bounded research effort, not permanent rules for how the product must be documented.
