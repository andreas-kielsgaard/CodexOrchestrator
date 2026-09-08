# Workflow CLI

The headless Workflow CLI uses the same active Workflow repository, Agent Session application,
native-profile authority, Harness engine, and Workflow MCP implementation as the desktop. It is an
authoring and demonstration surface, not a second Workflow engine.

Invoke it through the application binary:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml -- workflow-cli import `
  --app-data-dir "C:\absolute\app-data" `
  --file "C:\absolute\catalog.json"
```

Available commands are:

- `import`: create and activate every definition in a `workflow-cli-catalog/v1` JSON file. It
  refuses to overwrite an existing Workflow with the same name.
- `instantiate`: create an instance for an active Workflow and a worktree currently returned by
  the temporary discovered-worktree target source.
- `send`: send the first message through the Workflow node boundary, or continue the most recent
  Session already associated with that node, then wait for the instance to become idle.
- `show`: emit the instance target, Sessions, invocation outcomes, and connection activations as
  JSON.

Every command requires `--app-data-dir`. This keeps the target database explicit and prevents a
headless command from silently choosing a different desktop profile. `instantiate` requires an
absolute `--worktree` path, and resolves its repository and branch through the temporary target
boundary rather than synthesizing identity.

The sample catalogs are:

- `examples/workflows/capability-demonstrations.json` for fan-out, output-regex selection, and
  native MCP handoff;
- `examples/workflows/serial-session-policy-demonstration.json` for fresh versus continuing
  receiver Sessions without concurrent routes;
- `examples/database-hardening/managed-database-parallel-fanout.json` for parallel Workflow and
  cross-process managed-database demonstrations. Its runbook and concurrent runner are in the same
  directory.
