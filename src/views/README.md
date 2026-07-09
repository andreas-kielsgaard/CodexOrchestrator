# Views

`src/views` is for reusable rendering components that are not owned by a single
feature workflow. Views should be easy to reuse, test, and compose.

Rules:

- Accept data, status, and callbacks through props.
- Depend only on React, local view helpers, and stable view-model shapes.
- Avoid application use-case calls, infrastructure adapters, persistence, and
  domain orchestration.
- Keep feature-specific layout in `src/features`; promote only genuinely reusable
  components here.

Existing rendering code may still live under `src/app/views` while the tree is
being split. Redundant or duplicate view locations should be checked and resolved
later without behavior changes.
