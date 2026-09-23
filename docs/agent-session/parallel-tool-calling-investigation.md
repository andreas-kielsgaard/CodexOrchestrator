# Parallel tool calling investigation

Date: 2026-09-23
Branch base: `afb364426524f01317a859a0f6e77d735e229fa1` (`origin/main`)

## Conclusion

Orchid should publish accurate MCP annotations and preserve upstream annotations, but should not add a blanket developer instruction requiring `Promise.all`.

Codex 0.154 treats an MCP tool as eligible for parallel calls when its server opts in or the tool publishes `readOnlyHint: true`. Eligibility does not force the model to choose concurrency. The model may reasonably prefer one model turn, one shell process, or sequential calls when that is cheaper.

The [OpenAI programmatic tool calling guide](https://developers.openai.com/api/docs/guides/tools-programmatic-tool-calling) likewise recommends bounded stages and measuring tokens, turns, tool calls, latency, and correctness rather than relying on a generic efficiency instruction.

## MCP correction

The five Orchid-owned tools that only read application-bound, closed-world context now publish:

```json
{"readOnlyHint": true, "openWorldHint": false}
```

State-changing tools remain unannotated. No policy was added that attempts to infer or enforce annotations from tool names or behavior. The harness proxy already retained complete tool objects; its test now proves annotations survive selected-tool filtering.

## Live evidence

All successful comparisons used `gpt-5.6-luna` through the real Codex CLI runtime owned by Agent Sessions. `gpt-6-luna` was tried first and rejected by the ChatGPT-backed Codex API as unsupported. Astra was not used.

### Annotated MCP tools

The probe server exposes four independent 1.5-second read-only tools.

| Run | Codex thread | Model program | Tool wall behavior | Turn input tokens |
| --- | --- | --- | --- | ---: |
| Native Codex baseline | `01a0cf19-4105-7d21-9830-251dbe09d8ac` | discovery, then `Promise.all` | starts within 1.4 ms; 1.50 s total | 37,927 |
| Orchid baseline 1 | `01a0cf23-dcb3-74c0-b137-8d711fdeb895` | discovery, then `Promise.all` | starts within 4.0 ms; 1.51 s total | 64,253 |
| Orchid baseline 2 | `01a0cf26-d330-7f91-9452-b7bf3ff2f7ec` | one program with a sequential loop | about 6.11 s total | 35,487 |

This shows that Orchid does not serialize Codex tool execution. It also shows that annotations are eligibility metadata, not a scheduling command. The parallel programs required a separate discovery program in these samples, while the sequential sample discovered and called the tools in one program. That explains part of the token/latency tradeoff.

### Shell inspection control

Two Orchid sessions read the same four tiny local files.

| Policy | Codex thread | Execution choice | Task duration | Turn input tokens |
| --- | --- | --- | ---: | ---: |
| Stock Codex instructions | `01a0cf27-595f-7422-b071-6fe15f18a37a` | one PowerShell process reading four files | 16.3 s | 35,223 |
| Explicit `Promise.all` developer instruction | `01a0cf27-e313-7692-8683-7a708ab3f411` | four concurrent PowerShell processes | 24.9 s | 35,334 |

The explicit instruction produced parallel calls but made this workload slower because process startup dominated the tiny reads. The stock behavior was the efficient batching choice.

## Recommendation

1. Ship the MCP metadata correction.
2. Let Codex make the default batching decision; its standard instructions already prefer parallel work when useful.
3. Use a narrow session or workflow-stage developer instruction only when Orchid knows the operations are independent and individually high latency. Do not apply it globally to inspections.
4. Evaluate behavior with all four dimensions: model turns and tokens, tool-call count, wall time, and correctness. A lower wall time can cost an extra full-context model turn, while fewer calls can be faster for cheap local work.

## Reproduction

The ignored live test `agent_session_parallel_tool_calling_live_probe` launches the annotated MCP baseline, stock shell baseline, and instructed shell comparison. It requires `CODEX_AGENT_SESSION_LIVE_SMOKE=true`, the `live-tests` Cargo feature, and a working Codex login. The probe server is `scripts/parallel-tool-calling-mcp.py`.
