# Worker 026 Codex JSONL Event Parser Boundary

Date: 2026-07-02

## Summary

Added a small pure TypeScript parser/normalizer boundary for captured `codex exec --json` JSONL
streams. This slice does not execute Codex, read or manage Codex credentials, launch processes,
open databases, write stores, create artifacts/events/task runs/conversations, add Tauri/Rust
commands, touch React/UI, change package dependencies, or compose lifecycle recorder behavior.

## Behavior

- Added `parseCodexJsonlEvents` to parse newline-delimited JSON text into typed event envelopes.
- Supports documented top-level event kinds: `thread.started`, `turn.started`, `turn.completed`,
  `turn.failed`, `item.*`, and `error`.
- Supports documented item categories: `agent_message`, `reasoning`, `command_execution`,
  `file_change`, `mcp_tool_call`, `web_search`, and `plan_update`.
- Preserves raw JSON objects on events and items, including unknown fields.
- Ignores blank lines so trailing newlines parse cleanly.
- Throws `CodexJsonlParseError` with stable line numbers for invalid JSON, non-object lines,
  missing or non-string event types, and malformed known event or item envelopes.
- Preserves unknown event and item types as typed unknown records when their basic envelopes are
  valid.
- Added `summarizeCodexJsonlEvents` to extract the Codex thread ID, final completed
  `agent_message` text, terminal status, `turn.completed` token usage, and item counts by item
  type.

## Changed Files

- `src/infrastructure/codex/jsonlEvents.ts`: parser, event/item types, parse error, and summary
  helper.
- `src/infrastructure/codex/jsonlEvents.test.ts`: documented-like stream coverage and parser edge
  cases.
- `docs/architecture.md`: documented the Codex JSONL parser boundary.
- `docs/task-logs/worker-026-codex-jsonl-event-parser.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/codex/jsonlEvents.test.ts` -> pass
- Full required verification is recorded in the worker completion report after final run.

## Blockers

None known.

## Review Notes

- `itemCountsByType` counts every valid `item.*` event by nested `item.type`, so a future stream
  with both started and completed records for the same logical item will count both observations.
- Known event validation is intentionally minimal and documented-envelope focused. Unknown event
  types pass through with raw preservation rather than failing the stream.

## Orchestrator Review Addendum

- Hardened `summarizeCodexJsonlEvents` so it does not treat structurally valid unknown
  `CodexUnknownEvent` records with an `item.*` type as item events unless they actually carry the
  parser-produced item envelope.
- Added a focused regression test for that exported-union edge case.
