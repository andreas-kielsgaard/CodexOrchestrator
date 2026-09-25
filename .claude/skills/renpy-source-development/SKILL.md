---
name: renpy-source-development
description: Develop, refactor, diagnose, and validate source-owned Ren'Py projects using their repository-pinned SDK workflow. Use for authored `.rpy` source, screens, state, content pipelines, lint, tests, compilation, and source-cache problems. Do not use for externally built, installed, or compiled-only games; use `renpy-workbench` there.
---

# Ren'Py Source Development

Follow repository instructions and use its pinned SDK wrapper rather than an unrelated Ren'Py installation. Preserve existing work and distinguish authored source, generated artifacts, and test fixtures before editing.

## Workflow clues

- Keep playable narrative, engine integration, generated outputs, and executable tests discoverably separate. Document which external content fields affect runtime and which are descriptive.
- After moving or renaming `.rpy` files, check for source-less `.rpyc` files at the old paths. Ren'Py can still load them and report duplicate labels or testcases. In a confirmed source-owned project, use the repository's compile workflow without `--keep-orphan-rpyc`, or a scoped stale-cache checker. Never transfer this assumption to a compiled-only game.
- Run content generation or staleness checks, lint, and tests serially. Lint and tests can both touch compiled-script caches; concurrent runs can create misleading failures.
- Treat either a nonzero exit or `Full traceback:` output as failure evidence. Ren'Py lint can emit a traceback while returning zero.
- Bound engine subprocesses with stage timeouts. On a stall, terminate only the process tree launched for that check, then isolate the affected testcase before retrying.
- State test coverage precisely: exact UI routes, partial routes, and injected fixtures prove different things. Do not infer full progression from a related passing testcase.

Lint and focused tests do not prove packaged-build, restart, visual, accessibility, or user acceptance behavior. Report those boundaries when relevant.
