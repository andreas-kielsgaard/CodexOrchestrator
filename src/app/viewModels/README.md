# App View Models

View-model modules in this folder contain pure display shaping for the app shell or
feature-agnostic shared formatting.

They may:

- Map domain or application results into labels, summaries, option lists, and disabled-state
  friendly values.
- Format paths, dates, status labels, and compact previews.
- Be imported by controllers and rendering components.

They should not:

- Own React state.
- Perform async work.
- Call clients, stores, runtimes, Tauri commands, or filesystem/CLI adapters.
- Own feature-specific labels, forms, or workflow shaping when that feature has its own
  `viewModels` folder.
