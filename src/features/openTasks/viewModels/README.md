# Open Tasks View Models

View-model modules in this folder contain pure display and form-shaping helpers for the
Open Tasks feature.

They may:

- Map Open Tasks application results into labels, summaries, option lists, and compact previews.
- Normalize feature form drafts before controllers call application capabilities.
- Be imported by Open Tasks controllers and rendering components.

They should not:

- Own React state.
- Perform async work.
- Call clients, stores, runtimes, Tauri commands, or filesystem/CLI adapters.
- Own app-shell status formatting that belongs in `src/app/viewModels`.
