# Workflow continuation live validation — 2026-09-08

Passed against implementation commit `5eaf5e4d43d90c7a15fb57ea11ee6740e53b11b9`.
Six real Codex invocations ran across four Workflow nodes in a disposable Git workspace.
The completed exercise took 123.65 seconds.

## Runtime exercised

The opt-in driver uses production Workflow authoring, recipe compilation, instance
execution, Session Event delivery, Agent Session storage, Codex subprocesses, JSONL
normalization, Harness proxy handling and the Workflow-owned MCP server. Provider
responses and file-change events are real. The proxy listener runs in-process through
the existing test host; native child-sidecar startup and the Tauri window are not covered.

Successful runtime: desktop-bundled Codex CLI `0.153.0-alpha.5`, using the existing
configured model (`gpt-6-astra`). The first attempt with PATH CLI `0.144.0` failed before
task execution because that model requires a newer CLI. Global configuration and CLI
installation were left unchanged.

## Observations

| Exercise | Observed result |
| --- | --- |
| Initial Discussion prompt asks it to wait for approval | One completed invocation, no files, no downstream sessions. This is prompt behavior. |
| Approve Discussion; create `spec.md`; call continuation | Real `file_change` creation persisted before the MCP call. |
| Discussion has two matching connections | One MCP call returned two deliveries; Overview and Clarification both ran. |
| Deliver `sourceNode`, `outputFiles` and Discussion's files | Both destination prompts contained parseable JSON with the trusted node identity and `spec.md` creation attribution. |
| Supply nonexistent `claimed-only.md` in `outputFiles` | It appeared in that trigger field, but never in file history or node-file selection. |
| Overview creates an overview and edits the spec | Both changes were attributed to Overview through real Harness events. |
| Resume Clarification on a later user request | The same session changed `decisions.md` from `hello` to `welcome`; continuation delivered only to Revision. |
| Revision receives file inputs from three nodes | Its delivered prompt contained Discussion, Overview and Clarification metadata; it read the files and wrote a revision summary. |
| Revision calls continuation with `{}` and no outgoing connection | Successful MCP response with zero deliveries. |
| Query Overview's Edited files after Revision edits the spec | `spec.md` remained selected, with Discussion as creator and Revision as latest editor. This query used a reopened database connection. |

All six invocations completed. Three successful MCP calls returned delivery counts
`2`, `1`, and `0`. Seven create/edit records were persisted. Independent inspection
of the saved provider events and destination prompts confirmed these results. Runtime
shutdown left no exercise Codex processes running. No feature defect was found in
this scenario.

This run does not establish native designer interaction, packaged startup/restart,
cross-instance isolation, deleted-file behavior, repeated continuation from the same
node, or delivery to a busy implementation session. It uses a compact four-node
scenario rather than the complete proposed discussion/overview/plan/implementation flow.

## Reproduce

Use an authenticated Codex executable compatible with the configured model and a fresh
output directory. This ignored test launches real provider work and is excluded from
ordinary test runs.

```powershell
$env:WORKFLOW_LIVE_EXERCISE_ROOT = 'C:/path/to/fresh-exercise'
$env:WORKFLOW_LIVE_CODEX_PROGRAM = 'C:/path/to/codex.exe'
cargo test --manifest-path src-tauri/Cargo.toml --profile test-fast --features live-tests --lib agent_sessions::application::tests::repair_tests::live_continuation::real_codex_continuation_exercise -- --ignored --exact --nocapture
```

The driver writes the isolated `live.sqlite`, `recipe.json`, `sessions.json`,
`history.json`, `attempts.json`, `historical-editor-selection.json` and document workspace
under the chosen directory. `sessions.json` includes real prompts and provider events.

Evidence from this run is retained locally under
`C:/Users/user/.codex/tmp/workflow-continuation-live-20260908-02`.
`verified-results.json` contains the independently inspected MCP responses, delivered
JSON inputs and historical-editor selection. The failed older-CLI launch is retained
separately under `workflow-continuation-live-20260908-01`.
