# Worker 023 Conversation Store Boundary

Date: 2026-07-02

## Summary

Added a narrow pure TypeScript Conversation create/update/query store boundary plus a SQLite
adapter on top of Worker 016's TaskRun/Conversation schema. This slice does not add Codex runtime
integration, Codex JSONL parsing, app-server/SDK work, transcript/message storage, ChatGPT export
import, event emission, automatic task-run link maintenance, runtime database opening,
Tauri/Rust commands, Git execution, React/UI work, package/dependency changes, or broad stores for
other record families.

## Behavior

- Added `ConversationStore` with deterministic `createConversation`, `updateConversation`, and
  `queryConversations` behavior.
- Creates require `provider` and `title`, use injected ID/time providers, and leave optional
  `taskId`, `taskRunId`, `externalThreadId`, and `summary` unset unless provided.
- Updates keep `id`, `provider`, and `createdAt` immutable, update `updatedAt` from the injected
  clock, leave omitted fields unchanged, and treat `null` as an explicit clear for optional fields.
- Missing updates throw the typed `ConversationNotFoundError`.
- Queries support optional filters by `provider`, `taskId`, `taskRunId`, and `externalThreadId`,
  order by `createdAt` plus stable `id` tie-breaker, and support a non-negative integer `limit`.
- Added an in-memory implementation for focused domain tests.
- Added a SQLite adapter behind an injected SQLite-like interface with no production import from
  `node:sqlite`.
- The SQLite adapter uses Worker 016's `conversationToRow` and `conversationFromRow` mappers and
  the app migration coordinator in tests.

## Changed Files

- `src/domain/conversationStore.ts`: Conversation store contract, create/update/query helpers,
  typed missing error, cloning, and in-memory implementation.
- `src/domain/conversationStore.test.ts`: domain coverage for deterministic create, update
  semantics, explicit optional clears, missing-conversation errors, query filtering, ordering,
  limits, empty results, non-integer limits, and clone/output isolation.
- `src/infrastructure/sqlite/conversationStore.ts`: SQLite adapter using injected database
  interfaces, Worker 016 mappers, and optional transactions.
- `src/infrastructure/sqlite/conversationStore.test.ts`: app-migrated SQLite coverage for create
  round-trip, SQL `NULL` persistence, update semantics, typed missing errors, query behavior, limit
  behavior, clone/output isolation, and transaction rollback.
- `docs/architecture.md`: documented the Conversation store boundary.
- `docs/task-logs/worker-023-conversation-store-boundary.md`: recorded this worker result.

## Verification

- `git diff --check main...worker/023-conversation-store-boundary` -> pass
- `npm run test -- src/domain/conversationStore.test.ts src/infrastructure/sqlite/conversationStore.test.ts`
  -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None known.

## Review Notes

- Review whether mutable `taskId` and `taskRunId` links are desirable long term. They follow the
  recent optional-field clear/update contract and the schema's `ON DELETE SET NULL` durability
  choice.
- The SQLite tests use `node:sqlite`, which emits Node's experimental feature warning during test
  runs.
