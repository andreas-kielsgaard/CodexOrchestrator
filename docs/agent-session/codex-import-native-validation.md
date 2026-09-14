# Codex import: native application validation

Date: 2026-09-14. Branch: `feature/import-conversation-from-codex`.

Executed two imports through the real Tauri/WebView modal, followed by one normal composer submission per imported session. Both authenticated model calls completed with history-dependent answers. No fixture provider or mocked application client was used in this pass.

## Execution and correction

The first preview failed with `paginated_threads is not supported yet`: npm's CLI 0.144.0 cannot read these desktop-created paginated histories. A read-only probe using the current desktop CLI 0.153.0-alpha.5 read the full history successfully.

The shared Windows executable resolver was selecting an npm shim found anywhere on PATH before looking for native executables. It now follows PATH directory order, resolving an npm shim to its native binary at that position. Explicit `.cmd` paths retain their previous behavior. Three focused resolver tests passed, including both directory orderings and explicit-shim handling; the native binary was rebuilt successfully.

The validation process explicitly put `C:/Users/user/AppData/Local/OpenAI/Codex/bin/994e8469124a0d31` first on PATH. The unversioned desktop `bin/codex.exe` on this machine reports 0.130.0-alpha.5 and is also older than the source format. No global installation, PATH, or native configuration file was changed. These results require a compatible CLI; they do not establish import support in those older binaries.

The test instance used its own SQLite database and WebView profile under `.dev/native-import-validation/`, registered the existing `C:/Users/user/.codex` home, and selected a capability profile with `gpt-5.6-terra`, low reasoning, read-only sandbox, and no selected MCP tools or skills. Native commands prepared that profile; the tested imports and follow-ups used visible controls through the existing owned-WebView inspector.

## Cases

| Source task | Orchid session | Fork | Result |
| --- | --- | --- | --- |
| Review premature work (`01a0a149-469d-74d3-9924-e7415b61a4e9`) | `8e5316db-c7d1-4a56-91ce-b45cd43f88d3` | `01a0a19c-0c68-71c2-bfdb-2b235abfe173` | Imported one turn with 16 items. The follow-up recovered the exact previously unanswered question about skills/plugins and summarized the prior answer. |
| Compare demo skills with project (`01a0a0af-4a82-71b2-b146-51b75d75bded`) | `07dd89a6-3e2b-45e2-aff1-4461993c1c5a` | `01a0a19e-2faa-7a61-bbda-bea8d919a9c8` | Imported one turn with 47 items. After restarting Orchid, the follow-up recalled the absent shared verification workflow and gaps in durable inspection coverage, then repeated the earlier recommendation. |

The follow-up prompts asked for details from the preceding conversation without supplying the expected answers. Both prohibited tools, browsing, file changes, and implementation; neither new invocation recorded tool activity.

## Durable checks

- Sessions, invocations, and runtime event rows matched exactly across a full native-process restart before the second follow-up.
- Two Orchid sessions, two import receipts, two source-turn mappings, and two native-profile bindings were present.
- Four invocations were present: two imported historical turns and two completed live continuations. There were 321 runtime event rows.
- The two launch acceptances and two native launch-provenance rows belonged to the actual continuations. No Session Event delivery rows were created.
- Independent `thread/read` and `thread/turns/list` calls against both fork IDs returned two turns each after continuation.
- Both original rollout files retained identical SHA-256 hashes before and after the entire exercise.

## Evidence

Retained locally in `.dev/native-import-validation/`: `source-before.json`, `source-after.json`, `restart-before.json`, `results.json`, both fork-history reads, per-action ownership receipts, native logs, and rendered snapshots/screenshots. `completed-one.png` and `completed-two.png` show the actual follow-up answers. The directory is ignored because it contains private conversation history and local instance data.

This verifies the native development application with the compatible desktop CLI and a real authenticated provider. It does not verify an installer/release package, other CLI versions, another host, or every historical item type. Both source working folders existed; missing-folder allocation retains its separate service/allocator coverage.
