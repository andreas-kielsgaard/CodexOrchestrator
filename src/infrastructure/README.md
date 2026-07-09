# Infrastructure

`src/infrastructure` owns concrete adapters for local runtime boundaries such as
Tauri commands, SQLite storage, Git inspection, Codex execution, validation
runtime, and development status checks.

Rules:

- Implement application or domain ports with concrete local adapters.
- Depend inward on `src/application` and `src/domain` contracts as needed.
- Keep adapter-specific parsing, schemas, command invocation, and persistence
  details here.
- Do not import React app code, feature workflows, reusable views, or controllers.
- Do not move domain policy into adapters; adapters translate external facts into
  application or domain shapes.

Adapter overlap and duplicated runtime mapping should be checked and resolved in a
later cleanup pass, separately from documentation-only architecture work.
