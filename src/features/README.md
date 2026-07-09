# Features

Feature folders own user-facing workflows. A feature may compose feature
controllers, view compositions, reusable views, and application capabilities to
deliver one coherent screen or flow.

Rules:

- Depend on `src/views`, `src/capabilities`, `src/application`, and `src/domain`
  types as needed.
- Do not depend on the app root. `src/app` wires features together, not the other
  way around.
- Keep workflow coordination here when it spans multiple reusable views or
  capabilities.
- Do not define domain policy, persistence behavior, runtime adapters, or storage
  schemas.
- Do not make reusable UI primitives feature-private when they are shared by more
  than one workflow.
- Split large feature pages by technical concern: controllers for state and
  workflow, view models for pure projection, and views for rendering.

Known overlap between feature workflows, app-local views/controllers, and
application clients should be checked and resolved in a later cleanup pass. This
contract only documents the intended ownership.
