# Agent Sessions UI commands

The running desktop app exposes a local command endpoint. Commands activate Agent Sessions and use the mounted view's selection, folder disclosure, and organization actions. Replies contain the rendered selection, folders, session rows, visible rows, loading state, and errors.

The app writes `agent-session-navigation.json` in its app-data directory (`CODEX_ORCHESTRATOR_APP_DATA_DIR` when set). It contains the process ID, loopback endpoint, and per-process bearer token. Read this file again after restarting the app. Do not copy its token into logs or checked-in files.

On Windows, the default connection file is `$env:APPDATA\dev.codex-orchestrator.app\agent-session-navigation.json`. Use the override directory when launching an isolated development instance.

From this checkout:

```powershell
node scripts/agent-session-ui.mjs 'C:\path\to\app-data\agent-session-navigation.json'
node scripts/agent-session-ui.mjs 'C:\path\to\app-data\agent-session-navigation.json' '{"kind":"open_session","sessionId":"SESSION_ID"}'
```

The first command inspects the view. Use IDs from its reply in subsequent commands:

| Command JSON                                                                                                           | Effect                                                                                                                              |
| ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `{"kind":"inspect"}`                                                                                                   | Read current navigation state.                                                                                                      |
| `{"kind":"open_session","sessionId":"SESSION_ID"}`                                                                     | Select and reveal a session.                                                                                                        |
| `{"kind":"new_session","folderTarget":null}`                                                                           | Open a new Unfiled draft.                                                                                                           |
| `{"kind":"new_session","folderTarget":{"kind":"repository","repositoryId":"REPO_ID"}}`                                 | Open a repository draft.                                                                                                            |
| `{"kind":"new_session","folderTarget":{"kind":"workflow_instance","instanceId":"INSTANCE_ID"}}`                        | Open an ordinary draft in a workflow folder.                                                                                        |
| `{"kind":"set_folder_expanded","folderId":"FOLDER_ID","expanded":true}`                                                | Expand a folder; use `false` to collapse it.                                                                                        |
| `{"kind":"show_more","folderId":"FOLDER_ID"}`                                                                          | Reveal the remaining sessions; `unfiled` is also a valid ID.                                                                        |
| `{"kind":"move_session","sessionId":"SESSION_ID","placement":{"kind":"workflow_instance","instanceId":"INSTANCE_ID"}}` | Move display placement through the same action as drag/drop. Placement also accepts `repository` with `repositoryId`, or `unfiled`. |
| `{"kind":"pin_session","sessionId":"SESSION_ID","pinned":true}`                                                        | Pin a shortcut; use `false` to unpin.                                                                                               |
| `{"kind":"get_deeplink","sessionId":"SESSION_ID"}`                                                                     | Return the session URL without using the clipboard.                                                                                 |

Folder creation opens an unsent draft. It does not send a message. Moving changes organization only; it never changes the session's workspace or workflow ownership. The endpoint accepts only these typed commands, binds to loopback, and requires the descriptor's bearer token. It does not expose arbitrary JavaScript or Tauri invocation.

For direct HTTP use, POST the command JSON to `endpoint` with `Content-Type: application/json` and `Authorization: Bearer <token>`. Commands are serialized and return after the view has applied them. `loading: true` means conversation details are still loading; inspect again when those details matter. An unavailable UI produces a timeout error instead of a success acknowledgement.
