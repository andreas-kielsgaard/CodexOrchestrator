# 0002: MCP transport and access

Status: accepted basis for Sprint 5 implementation.

## Decisions

- Use the official Rust MCP SDK: `rmcp = { version = "=2.2.0", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }`. Add `tokio` and `tokio-util` only with the runtime/lifecycle features used by the adapter. Do not handwrite MCP or JSON-RPC.
- Run one application-owned, in-process Streamable HTTP server at `http://127.0.0.1:<ephemeral-port>/mcp`. Start it before the managed Codex invocation; cancel it, wait for it, and invalidate its credential when the invocation ends, is cancelled, or the app shuts down. It is not a general listener or a persisted endpoint.
- Configure the SDK's host and Origin allow lists explicitly. Bind only `127.0.0.1`; accept an absent Origin for the local CLI, reject every present Origin except the concrete active-app origins, and return HTTP 403 for invalid Origin. Do not rely on the SDK default because its Origin allow list is empty.
- Generate a high-entropy bearer credential per managed invocation. Pass only its environment-variable name in Codex MCP configuration; pass the value only in that child environment. Never persist, log, expose in UI/URLs, or reuse it. The server checks bearer authentication before protocol/tool dispatch.
- A capability profile is a durable application policy: allowed tool names, draft scope, operation scopes,
  output limits, and expiry. Client `enabled_tools`/approval settings reduce model context only; the
  server rechecks profile, draft access, revision preconditions, and idempotency for every call.

## Required adapter tests

Direct deterministic HTTP/MCP checks must cover: no/invalid bearer (401), wrong Origin (403), wrong
Host, expired profile, hidden/disallowed tool, invalid schema, stale revision, repeated idempotency
key, clean cancellation, and `initialize`/`tools/list`/both tool calls. These checks do not invoke a
model.

## Evidence and reversal point

On 2026-07-15, `cargo info rmcp@2.2.0` reported the official SDK and the selected features; a
disposable external `cargo check` compiled them with `tokio` and `tokio-util`. The SDK exposes
`StreamableHttpServerConfig` with cancellation plus host/Origin configuration. The MCP Streamable
HTTP specification requires Origin validation, localhost binding, and authentication. Reconsider
only if a supported Codex client cannot use Streamable HTTP with bearer authentication.

Sources: [RMCP SDK](https://github.com/modelcontextprotocol/rust-sdk),
[RMCP 2.2.0 docs](https://rust.sdk.modelcontextprotocol.io/), and
[MCP Streamable HTTP transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).
