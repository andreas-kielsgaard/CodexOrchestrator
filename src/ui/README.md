# UI

`src/ui` owns reusable UI primitives and design-system-level components.

Components here should be behavior-agnostic: buttons, panels, tabs, indicators,
and similar building blocks. They may expose props and callbacks, but they
should not know about application use cases, feature workflows, runtime clients,
domain policies, or concrete infrastructure.

Promote a component here only when it is reusable across multiple features or
app surfaces. Feature-specific composition belongs in that feature's `views`
folder.
