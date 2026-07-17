# Offline review checklist

Exploration anchor: branch `codex/explore-agent-testing-feedback`, commit `8c7ccd1`.

## Before viewing

- [ ] The accepted worktree exists at
      `C:\Users\user\.codex\worktrees\0e46\Codex Orchestrator`.
- [ ] `git rev-parse --short HEAD` reports `8c7ccd1`.
- [ ] Node.js, npm, and the existing local `node_modules` directory are available.
- [ ] No provider prompt, Agent Session, network access, installation, merge, or push is needed.

## In-app review

- [ ] `/?agent-test-mode` opens the normal App shell.
- [ ] Test mode is a selected peer tab beside Orchestration and Agent Sessions.
- [ ] The header clearly says development-only, synthetic data, and feedback-only authority.
- [ ] Navigation, actions, observations, and waits use named semantic IDs rather than coordinates.
- [ ] Advancing the fixture changes structured state in the recorded Agent Session presentation.
- [ ] Screenshot capture fails closed because no app-window adapter exists.
- [ ] The demo boundary identifies a build, Test Session, app-owned root, and excluded data.
- [ ] An annotation can target a view, status state, transcript element, or timeline point.
- [ ] Delivered feedback is `application_test_feedback/v1` with `feedback_only` authority.
- [ ] The feedback goes only to the recorded in-memory sink and is not an Orchestration Event.

## Product decisions

Write a choice and any condition beside each item.

- Tester ownership - separate Agent Session or capability of the implementing session:

  Decision:

- Build identity - commit and dirty-tree digest, packaged binary digest, or both:

  Decision:

- Demo approval - approval for every delivery, approval by policy, or another gate:

  Decision:

- Annotation lifecycle - retention, deletion, and redaction:

  Decision:

- Capture privacy - whether transient Agent Session text may appear inside the isolated root:

  Decision:

## Next-slice gate

- [ ] Approve or revise **Test Session Host v1**.
- [ ] Keep the first slice debug-only and synthetic.
- [ ] Require one isolated window and one declared capture root.
- [ ] Require invocation-scoped credentials and a fixed six-tool semantic allowlist.
- [ ] Require PNG evidence plus build/session/view manifest.
- [ ] Require fail-closed tests for invalid IDs, credentials, cross-session access, and cleanup.
- [ ] Keep video and product feedback delivery out of this slice.

Notes:
