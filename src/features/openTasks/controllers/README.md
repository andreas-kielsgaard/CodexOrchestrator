# Open Tasks Controllers

Controllers in this folder own UI-serving state for the Open Tasks feature.

They coordinate feature flows such as dashboard loading, repo onboarding, task creation,
task editing, task runs, and task detail selection. They receive application clients through
props/options and translate feature events into application calls.

They should not:

- Call Tauri or other infrastructure adapters directly.
- Implement domain policies.
- Own app-shell concerns such as backend maintenance or runtime health monitoring.
