# Historical Codex app-server 0.144.0 contract

This is a parser fixture for CLI 0.144.0, not the active production protocol. The active CLI
0.154 protocol rejects `skills/extraRoots/set`; Orchid no longer sends it. Skill discovery comes
from the selected native Codex configuration until a compatible discovery-root contract exists.

Verified with the installed Windows `codex-cli 0.144.0` on 2026-09-09. The executable contract is `scripts/codex-app-server-contract.node-test.mjs`; it is opt-in through `CODEX_APP_SERVER_CONTRACT_PROGRAM` and uses a disposable home and local Responses provider. Production-adapter peer fixtures are in `runtime/codex/app_server/tests.rs`. Neither fixture is a captured user conversation.

The historical execution sequence was `initialize` / `initialized`, `skills/extraRoots/set`, `thread/start` or `thread/read` + `thread/resume`, `turn/start`, confirmed `turn/started`, then `turn/steer` with `expectedTurnId` or `turn/interrupt`. JSONL request IDs correlate replies independently of notifications and native request IDs.

`turn/steer` includes `clientUserMessageId`. An acknowledged turn ID must match the expected active turn. Turn completion precedes process cleanup; process exit alone is not success.

Native resume retains some prior model/effort values. The adapter resolves current values with an ephemeral `thread/start` before resuming the durable thread. `skills/extraRoots/set` supplies product skill roots without editing native configuration. MCP additions are individual thread-configuration keys and preserve native servers.

Request mapping and validation live in `app_server/requests.rs`; unknown requests receive an explicit protocol error and durable limitation event. CLI 0.144.0 has no app-server equivalent of exec's `--ignore-rules`; this limitation is checked before launch.

The sibling exec JSONL recordings remain historical parser fixtures. They do not describe the production transport after this cutover.

On Windows, native 0.144.0 can resolve an explicit workspace-write request to read-only when Windows sandbox support is disabled. The production adapter checks the resolved policy before `turn/start`. The final application fixture verified rejection of that mismatch and success with unelevated sandbox support configured in its disposable home.
