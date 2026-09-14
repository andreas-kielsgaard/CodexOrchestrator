# Fresh Hetzner server setup for the remote session prototype

Status: server provisioned, laptop profiles configured, and remote execution verified through Orchid. See [implementation evidence](remote-worktree-session-evidence.md) for versions, paths, and acceptance results. The procedure below documents setup.
Date: 2026-09-14.
Feature scope and ownership: [remote worktree session plan](remote-worktree-session-plan.md).

## Starting point before provisioning

| Item | Evidence |
| --- | --- |
| Server | IPv4 `2.28.122.21`; interactive SSH succeeded as `root`. |
| Hostname | `ubuntu-8gb-nbg1-1`; Linux reported by the setup task. Exact distribution release and architecture remain to be inspected. |
| Laptop key | Existing passphrase-protected `C:\Users\user\.ssh\orchid_hetzner`. Host key is already in the laptop's known_hosts. |
| Unattended access | Windows OpenSSH agent was disabled/stopped; the later noninteractive SSH attempt failed. |
| Git, Codex, Rust, repository, Orchid host | Not established by the setup task. Treat the machine as fresh. |

Use the existing root connection for this initial experiment. A new user, firewall redesign, daemon installation, containers, and an orchestration control plane are not prerequisites for this scope.

## 1. Make SSH usable from the desktop process

Enable/start the Windows OpenSSH agent, load the existing key with `ssh-add`, and complete its passphrase prompt. Service configuration may require an elevated Windows shell; setup should handle that explicitly. Reuse the passphrase already supplied by the user through an interactive mechanism; never put it in a checked-in script or configuration.

Add or update the task's alias in `C:\Users\user\.ssh\config`, preserving unrelated entries:

```sshconfig
Host orchid-remote
    HostName 2.28.122.21
    User root
    IdentityFile ~/.ssh/orchid_hetzner
    IdentitiesOnly yes
```

Confirm `ssh -T -o BatchMode=yes orchid-remote hostname` works without a terminal prompt. Orchid's protocol connection uses `ssh -T`, not a PTY. Launch Orchid under the same Windows user that can use the agent.

## 2. Inspect and install the server prerequisites

Read `/etc/os-release`, `uname -m`, available disk space, and versions/paths for Git and Codex. Check HTTPS/Git connectivity to the intended source repository and provider. Report observed facts rather than assuming the image version from the hostname.

Install Git and CA certificates. Install the tooling needed for the chosen host build route: a C linker/build tools and a stable Rust toolchain when building on this server. Match any repository-pinned Rust toolchain if one is introduced. No desktop/WebKit packages are needed: the host must compile independently of Tauri.

Prefer building `orchid-host` on the Linux server from the implementation's engine source. Transfer the current engine source and its lockfile over SSH, including intentional implementation edits, into a build directory outside the demonstration repository. This avoids requiring a feature publication just to exercise the host. Alternatively, deploy an already-built binary that matches the observed Linux architecture.

Install the complete Codex CLI Linux package at a recorded version compatible with the adapter. For the tested 0.144.0 x86-64 release, use `codex-package-x86_64-unknown-linux-musl.tar.gz` and retain its `bin`, `codex-path`, `codex-resources`, and package manifest together. The standalone executable archive omits `codex-code-mode-host` and cannot execute this model's file tools. Verify `codex --version`, `codex app-server --help`, and an actual file operation. Use an absolute executable path in host configuration so noninteractive SSH does not depend on an interactive shell's PATH. Node is not required by the Rust Orchid host or this native Codex package. See [Codex CLI](https://learn.chatgpt.com/docs/codex/cli).

The first session needs Git, the host binary, and Codex. Repository-specific build/test dependencies are installed only if the demonstration prompt actually needs them.

## 3. Sign in to Codex on the server

Use the native Codex home for the remote login, initially `/root/.codex`. Run:

```sh
codex login --device-auth
codex login status
```

The user completes the browser sign-in for the supplied device code. Device-code login may need enabling in account/workspace settings. If unavailable, use the documented SSH forwarding method for the login callback and complete the browser flow. This is provider sign-in, not an Orchid credential-management feature. See [headless authentication](https://learn.chatgpt.com/docs/auth#login-on-headless-devices).

Keep provider configuration and credentials in that remote native home. Do not copy the laptop's complete Codex home or assume its plugins, local paths, or MCP servers exist on the server. Confirm the remote engine can read a real model catalog and execute a small prompt in the demonstration directory. An installed executable alone does not prove authenticated execution.

## 4. Build and install the Orchid host

Installed locations under the existing product directory convention:

```text
/root/.local/bin/orchid-host
/root/.codex-orchestrator/host/config.json
/root/.codex-orchestrator/host/sessions/
/root/.codex-orchestrator/build/orchid-engine/
/root/.codex-orchestrator/repositories/orchid/source/
/root/.codex-orchestrator/repositories/orchid/worktrees/remote-development-demo/
/root/.codex/
```

Build with `cargo build --manifest-path <engine-source>/Cargo.toml --release --bin orchid-host`, then install the binary at the recorded absolute path. Keep the engine Cargo.lock with the source used for the build.

The installed host configuration is:

```json
{
  "deviceId": "hetzner-orchid",
  "deviceName": "Hetzner server",
  "configurations": [
    {
      "id": "codex-default",
      "executable": "/root/.local/bin/codex",
      "home": "/root/.codex"
    }
  ]
}
```

The adjacent `sessions/` directory retains product session ID, configuration reference, working directory, and provider thread ID. Conversation/event history stays in the laptop database; provider credentials stay in the native Codex home.

Verify the protocol entry point:

```text
ssh -T orchid-remote /root/.local/bin/orchid-host connect
```

The client sends the agreed describe/list requests over stdin and reads JSON frames from stdout. Logs use stderr. No public application port or systemd service is required. The host starts on demand and supervises Codex locally.

## 5. Prepare the repository and one branch worktree manually

Use the selected laptop project's configured source repository. If private, establish a remote Git credential for fetching that repository during setup; SSH access to the VM does not grant access to the Git source. Complete any provider-owned sign-in interactively. No Git authentication UI is added to Orchid.

Clone the source into the proposed `source/` directory. Fetch the intended branch and resolve the exact published commit before creating the demo instance. Record the repository URL, selected branch ref, commit, and paths in the implementation evidence.

Create a branch-attached worktree with the same branch ref selected in the laptop browser. Avoid checking that branch out in the source checkout as well: the source can be detached at its current commit before adding the separate branch worktree. When creating a local branch from a remote-tracking branch, resolve the published SHA and create it at that SHA. Use existing Git worktree operations; do not add a product creation endpoint. See [Git worktree](https://git-scm.com/docs/git-worktree).

Inspect `git worktree list --porcelain`, the instance's symbolic branch, and `HEAD`. Initial tracked/untracked working state should be clean. Existing laptop worktrees and unrelated changes are preserved. If a new demonstration branch is needed, use a task-owned `codex/` name on both devices, based on the selected published commit.

There are two independent source states: the published commit used by the demonstration worktree, and the implementation source used to build the host. Do not confuse them or reset the demo worktree to a host build revision.

## 6. Configure the product through Technical Settings

Create the remote Capability Profile with:

- device ID matching the host configuration and display name `Hetzner server`;
- SSH target `orchid-remote`;
- host executable `/root/.local/bin/orchid-host`;
- execution configuration `codex-default`;
- native discovered capabilities and appropriate defaults;
- repository mapping from the selected laptop catalog repository to the remote `source/` path.

Configure the laptop profile using its existing local native Codex environment. Save both through the product's normal persistence path. Read the profiles back and discover their existing branch worktrees. Leave a device visible when it has no matching instance.

## 7. Prove the first session

Run the UI acceptance sequence in the feature plan. Confirm the selected remote directory and actual file changes on the server independently of the assistant's prose. Follow up through the same Orchid session/provider thread. Verify a laptop target still executes locally, and exercise a provider interaction plus cancel while connected.

Record actual OS/architecture, host build revision, Codex version, profile IDs, branch/commit/path, and visual evidence. Provider login, successful SSH, a host protocol smoke test, and a successful Orchid session are separate results. Stop at the agreed prototype; no workflow callback, file-history, synchronization, or unattended-execution claims follow from this test.
