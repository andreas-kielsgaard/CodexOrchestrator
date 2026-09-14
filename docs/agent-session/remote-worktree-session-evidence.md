# Remote worktree session implementation evidence

Status: implemented and live-validated on both devices. Updated 2026-09-14.

## Run the configured prototype

1. Run `launch-remote-development.bat` from the feature worktree. It uses a separate prototype database and WebView directory.
2. Open **Agent Sessions**, choose **New session**, then **Target worktree**.
3. Choose **Codex Orchestrator**, branch **codex/remote-development-demo**, and the **Hetzner server** worktree using **Hetzner Codex**. Confirm **Use worktree**, then send a prompt.
4. Connection settings and the remote repository mapping are under **Technical Settings → Devices and Capability Profiles**. Profiles `laptop-codex` and `hetzner-codex` are saved; `laptop-codex` is the default for untargeted sessions.

The remote host starts through SSH on demand. There is no background synchronization or worktree creation flow in this slice.

## Source and setup

- Feature worktree: `C:\Users\user\.codex\worktrees\remote-development`, branch `feature/Remote-Development` from local `main` at `e2bfc6cb584a9ce7ada0d762f768c605a33b4160`. Git cannot represent the requested space in a branch name.
- The published `origin/main` SHA was independently confirmed with `git ls-remote`. Unrelated dirty plans in the original checkout were preserved.
- Windows OpenSSH agent enabled and started; the existing `orchid_hetzner` key loaded interactively. `ssh -T -o BatchMode=yes orchid-remote hostname` succeeds. SSH alias saved in the user's existing SSH configuration location.
- Destination: `root@2.28.122.21`, hostname `ubuntu-8gb-nbg1-1`, Ubuntu 26.04.1 LTS, x86-64. Git was already installed. Build tools and Rust 1.98.1 were installed.
- Complete Codex CLI 0.144.0 package installed at `/root/.local/lib/codex/0.144.0`, with `/root/.local/bin/codex` pointing to its entrypoint. Version matches the laptop CLI. The package includes the code-mode helper and native resources required for file tools. Native device-code login completed; `codex login status` reports authenticated with ChatGPT.
- Public repository cloned from `https://github.com/andreas-kielsgaard/CodexOrchestrator.git` into `/root/.codex-orchestrator/repositories/orchid/source`.
- Both demonstration instances start at the exact published SHA above, on `refs/heads/codex/remote-development-demo`:
  - Laptop: `C:\Users\user\.codex\worktrees\remote-development-demo`.
  - Server: `/root/.codex-orchestrator/repositories/orchid/worktrees/remote-development-demo`.
- Remote source checkout is detached so the demo branch is attached only to its separate worktree. No worktree creation feature has been introduced.

## Host build and checks

- Independently built and tested the engine on Windows and Linux. Shared engine tests: 34 passed on Windows, 33 passed on Linux (the extra Windows case is platform-specific).
- Linux release host installed at `/root/.local/bin/orchid-host`. Latest deployed source archive SHA256: `a35353ec7d041b217e81a90a56aca1132d7f32dd106a1ce73fac56de9d1d5c83`.
- SSH host describe, branch-filtered worktree listing, and capability discovery passed against the real server. Runtime discovery returns the remote model/skill catalog.
- Host config exposes `codex-default` for device `hetzner-orchid` / `Hetzner server`, using `/root/.codex`.
- Laptop product database backed up using SQLite backup to `C:\Users\user\.codex-orchestrator\backups\pre-remote-development-20260914.sqlite` before the feature app launch.

## Acceptance

- Frontend: production build, 145 affected tests, and touched-file ESLint passed.
- Desktop Rust: `cargo check`, 71 Agent Session tests, 20 execution configuration tests, 39 storage/migration tests, one native-home continuity test, 26 Codex runtime tests, nine branch review tests, and four repository catalog tests passed.
- Live host first prompt and continuation passed using `gpt-5.6-terra`. Session `host-smoke-797237dd-ee0c-44fc-8033-190ea54153c2`, native provider thread `01a0a082-4490-77b1-bc99-2b78a5ed79f6`.
- The first prompt read `README.md` and created `orchid-host-smoke-247c6995-8a05-4b69-a279-aa718fdf643d.txt` in the remote demo worktree. A separate SSH `cat` confirmed its marker. The follow-up read it through the same native thread and working directory.
- This check found and fixed the adapter's null reasoning-effort override on resume. A regression test covers absent native effort and preservation of explicit selections.
- Live remote provider approval passed: one-time approval for a scoped `/tmp` marker command, with independent SSH verification. Provider thread `01a0a091-20b1-72e2-9fdc-71cce2c58776`, request `7f9a8862-a1cb-486e-ad7f-551b1f82ec03`. Live cancellation interrupted a running `sleep 60`; native status was interrupted and normalized status was canceled, without a runtime error. Provider thread `01a0a091-4f2c-78a2-a635-b42a19859328`. Full frames and results: `.dev/remote-host-interactions-8241e2da-bfd3-4506-944b-c7f4ff5779db.jsonl` and `.json`.

## Native UI acceptance

Another development worktree is running concurrently, so this feature uses `launch-remote-development.bat`, port 1430, and `C:\Users\user\.codex-orchestrator\remote-development-laptop`. That app-data directory was seeded from the pre-feature SQLite backup; prototype settings and sessions are saved there. The launcher also isolates WebView2 data in `.dev/webview-remote-isolated` because sharing its default data directory prevented the second development window from appearing. Native UI checks attached to this executable's actual WebView2 over a temporary debugging port; no mocked Tauri transport was used. The repository's inspector verified the native executable PID and its descendant debugger listener in `.dev/remote-native-owner-receipt.json`.

- Created `hetzner-codex` and `laptop-codex`, revision 1, through Technical Settings. Both use `gpt-5.6-terra` and `workspace_write` defaults. Saved the remote repository mapping through the same tab and selected the laptop default. The settings survived a native app rebuild/restart.
- The target modal required a repository first, loaded the shared branch list and graph, and showed both existing demo worktrees with their paths and HEADs. Choosing `main` left the remote device visible with “No existing worktree for this branch.” The graph used the same open modal. A typed prompt survived target selection.
- Remote product Session `1e08c78d-ceae-4fb9-b71e-91663f75dae6` completed its first prompt and follow-up. Both reported the remote worktree. An independent SSH read verified `orchid-ui-remote-fccf676a-eb8a-4ab7-b0dc-6875dc4aeb6a.txt` there.
- All four remote invocations reported the same Codex thread, `01a0a0a1-641f-7012-ad0f-e37c41ce6f68`. Their persisted outcomes were completed, completed, canceled, completed. Cancel was clicked after the running `sleep 60` tool appeared. The fourth turn exposed a real provider approval; **Allow once** ran only the displayed command. SSH independently verified `/tmp/orchid-ui-approval-20260914.txt`.
- Laptop product Session `78e7feed-35c8-402b-9db7-a0e1e8e1df5f`, Codex thread `01a0a0a4-2543-7c82-8c1c-b4545b37f604`, completed a prompt reporting `C:\Users\user\.codex\worktrees\remote-development-demo` and the README first line. Its binding froze the selected native profile ID.
- Reloading the native UI and reopening the remote Session restored its transcript and disabled target control. Durable target/provider/invocation records were independently read from SQLite into `.dev/remote-ui-acceptance.json`.

The live checks found and fixed new-session drafts being replaced by existing history, and branch queries depending on Worktree Review's selected repository. Explicit repository branch queries now avoid changing review selection or recording review observations. The final navigation/backend regression suite passed 5 tests; the final frontend navigation, target, profile, and App suites passed 54 tests. The production frontend build and touched-file ESLint passed after those fixes.

One laptop profile save encountered a changing native Codex skill catalogue between discovery and save. Refreshing discovery and applying the current capabilities resolved it; no upstream discovery or synchronization machinery was added.

Visual artifacts are in `.dev/`: `remote-connection-settings.png`, `target-two-devices.png`, `target-empty-remote-instance.png`, `target-branch-graph.png`, `remote-first-prompt.png`, `remote-followup.png`, `remote-canceled.png`, `remote-approval-pending.png`, `remote-approval-complete.png`, `laptop-target-complete.png`, and `remote-restored-after-ui-reload.png`.

These checks cover the connected prototype and UI reload. They do not establish detached execution, recovery of interrupted active work, synchronization, workflow operation, or Claude support.

## Closed idle connection repair

The user's demo invocation `9812c9ea-11d1-4573-84fa-78ee132a448d` in Session `c940bb82-3f86-458c-b6cd-9efefcb26a76` failed with `runtime_preflight_failed: Remote connection has closed`. The cached execution endpoint retained an exited SSH process. The failure occurred before a provider thread was created or the prompt was delivered; the requested demo file did not exist. A fresh SSH connection and host discovery succeeded. The original failure record is retained in `.dev/closed-connection-before.json`.

The desktop runtime now replaces a closed idle connection before a new invocation. Active interactions retain their existing connection, and an uncertain invocation is never automatically replayed. Four focused regression tests cover idle replacement with resume identity, active interaction behavior, no replay after an uncertain failure, and healthy connection reuse. The native `test-fast` build passed; no server redeployment was required.

The rebuilt native app retried the original prompt in the same product Session. Invocation `e536bceb-0a65-4bbb-9af0-18079a0d6031` completed and created `orchid-remote-demo.txt`; an independent SSH read confirmed the hostname, remote worktree path, published HEAD, README heading, and requested phrase. With no active invocation or host child process, host PID 27771 was deliberately terminated. Without restarting Orchid, follow-up invocation `677d2218-1bfd-4b71-a42b-ea00e26b0686` completed through replacement host PID 29390. The Codex thread remained `01a0a135-5c59-7a22-b700-872338501edb`. SSH confirmed the appended line, and Git status showed only the new demo file alongside the existing smoke artifacts. The original failed invocation remains in history. Evidence: `.dev/closed-connection-after.json` and `.dev/closed-connection-recovered.png`.
