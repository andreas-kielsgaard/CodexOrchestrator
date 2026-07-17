# Agent-native application testing exploration

## Direction

Use one application-owned Test Session with semantic view, element, action, and condition IDs. A
future invocation-scoped MCP server should expose only those typed operations. Test Session
authority is separate from production Agent Control authority.

The reversible proof is available in development at `/?agent-test-mode`. It uses the real Agent
Session screen with the recorded Agent Session adapter and synthetic data.

## Proof

The test-mode controller supports:

- navigation to an allowlisted view;
- semantic action dispatch;
- structured state and element observation;
- bounded waits on typed conditions;
- a screenshot request that fails closed while no app-window capture adapter exists;
- a demo-capture envelope tied to a build reference, Test Session, and one capture root;
- annotations anchored to a view, element, state, or timeline point;
- feedback delivery as `application_test_feedback/v1` with `feedback_only` authority.

The recorded feedback sink is in memory. It does not create Orchestration Events or Agent Control
commands.

## Extension and security boundaries

- Product boot does not receive a test-mode controller. The proof loads only when Vite development
  mode and the explicit query flag are both present.
- The proof allows synthetic data only.
- View, element, action, and condition IDs are allowlisted. There are no coordinates, selectors,
  arbitrary scripts, or generic filesystem/process operations.
- Screenshot and video operations must capture one app-owned root in an isolated Test Session.
  Broad desktop capture is not a fallback.
- Capture metadata must bind the executable build, Test Session, view, and evidence artifact.
- Feedback is advisory input to a named Agent Session. It is never product state authority.
- A future MCP adapter should follow the existing managed-invocation pattern: loopback-only
  listener, per-invocation bearer, child-only launch injection, explicit tool allowlist, Host and
  Origin checks, and owned terminal/shutdown cleanup.

## Prototype limits

- There is no Test Session MCP server or Tauri command bridge.
- The build reference is a recorded fixture identity, not executable attestation.
- Pixel screenshot and video capture are not implemented. The UI reports this explicitly.
- Feedback delivery is not connected to a product Agent Session and is not durable.
- Annotations are in memory and are not yet attached to captured frames.
- Worktree, port, database, process, and application-state isolation remain unproven.

## User-review points

- Whether a tester is a separate Agent Session or a scoped capability added to the implementing
  Agent Session.
- Which build identity is sufficient: commit plus dirty-tree digest, packaged binary digest, or
  both.
- Whether the user must approve each demo before delivery to the implementing agent.
- Annotation retention, deletion, and redaction policy.
- Whether capture may include transient Agent Session text even inside the isolated app root.

## Exact next product slice

Build **Test Session Host v1** as a debug-only focused Rust/Tauri module:

1. Open one isolated synthetic Test Session window bound to a commit and dirty-tree digest.
2. Start one invocation-scoped MCP server with six tools:
   `describe_test_session`, `navigate_test_view`, `perform_test_action`, `observe_test_view`,
   `wait_for_test_condition`, and `capture_test_view_png`.
3. Bridge those tools to a frontend semantic registry for the isolated window.
4. Capture only that window's declared root and persist a PNG plus a build/session/view manifest.
5. Prove forged view/action IDs, missing credentials, cross-session access, terminal cleanup, and
   application shutdown all fail closed.

Do not add video recording or product Agent Session feedback delivery in that slice. Establish the
isolated host and trustworthy still-image evidence first.
