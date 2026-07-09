# Codex Infrastructure

`src/infrastructure/codex` owns concrete parsing and integration helpers for
Codex runtime output and app-server messages.

Keep raw Codex protocol parsing, JSONL event handling, and Codex-specific adapter
details here. Application and feature code should consume stable contracts rather
than depend on Codex wire formats directly.
