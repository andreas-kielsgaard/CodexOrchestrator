# Source Layout

`src` contains the React/TypeScript desktop application. It is organized by
technical responsibility rather than by visible screen area.

Start with `AGENTS.md` when deciding where a new file belongs. The short version:
shell code goes in `app`, workflow code goes in `features`, reusable UI-facing
contracts go in `capabilities`, use cases and ports go in `application`, pure
business rules go in `domain`, concrete external adapters go in `infrastructure`,
design primitives go in `ui`, and reusable render-only app views go in `views`.

CSS restructuring is intentionally outside the current folder-boundary work.
