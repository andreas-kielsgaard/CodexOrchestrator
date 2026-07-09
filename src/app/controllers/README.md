# App Controllers

Controllers in this folder own app-shell or cross-feature UI state and translate app-level
events into application capability calls.

They may:

- Hold React state for app-level loading, busy flags, notices, and transient errors.
- Call application queries, commands, and ports through injected dependencies.
- Adapt application results into view models by using presenters.

They should not:

- Implement domain policies.
- Call infrastructure adapters directly.
- Format display strings inline when a presenter/view-model helper owns that concern.

Feature-specific flow controllers should live with their feature instead.
