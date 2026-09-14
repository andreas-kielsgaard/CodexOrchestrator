# Codex Orchestrator

A local desktop application for working with related agent conversations, configuring their execution, and coordinating work through Workflows. React presents the product; Rust and Tauri own native application operations, persistence and the Codex app-server connection.

The main surfaces are Agent Sessions, Workflow authoring and instances, execution configuration, repository selection and Worktree Review. Product Decisions supports reasoning about work. File Review provides the contextual viewer and scoped loader, with a [remaining native startup limitation](docs/file-review.md). The retained Epic/Sprint system is documented separately because its implementation and its current strategic priority are different facts.

## Start development

On Windows, install Node.js 24 or newer with npm, Rust with the MSVC toolchain, Visual Studio C++ build tools and WebView2. Native Session execution also needs a usable Codex installation and selected native profile.

From the repository root:

```powershell
.\launch-dev.bat
```

The launcher restores npm development dependencies when its local Vite/Tauri commands are missing and starts Tauri with Vite. See [development](docs/development.md) for direct commands, build outputs and optional validation.

Build a standalone application with `npm run build`, then launch the printed executable separately. Use `npm run build -- --debug` when debugging is needed. Native build/check/test commands accept `--cache=auto|local|shared`; auto reuses the worktree's local profile artifacts or uses shared caching for a fresh target. `npm run build:frontend` compiles only the frontend.

## Documentation

Start with the [documentation index](docs/README.md). It routes to current behavior and source ownership, important decisions, and dated validation evidence.

- [Architecture](docs/architecture.md)
- [Agent Sessions](docs/agent-session/README.md)
- [Workflows](docs/workflows.md) and [execution configuration](docs/execution-configuration.md)
- [Repository and Worktree Review](docs/worktree-review.md)

The guides were researched at `e2bfc6c`, reconciled with main `60c3798`, and updated for the integrated tooling cleanup. Changes implemented on separate branches are identified as separate work; historical screenshots and test results retain their original scope.
