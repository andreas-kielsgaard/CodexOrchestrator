# Observation pass: packaged Agent Session verification surface

## Anchor

Is the recorded Agent Session browser harness merely test source, a development tool, or part of the built application distribution?

This pass followed one explicit verification surface from source entrypoint through the frontend build and Tauri bundle configuration.

## Observed path

1. `agent-session-harness.html` loads `src/dev/agentSessions/main.tsx`.
2. That entry mounts `AgentSessionHarness`, which wraps the reusable `AgentSessionScreen` with a recorded client and deterministic scenario controls.
3. `vite.config.ts` declares both `index.html` and `agent-session-harness.html` as production Rollup inputs.
4. `npm run build` runs TypeScript checking and Vite production build for both inputs.
5. `src-tauri/tauri.conf.json` uses the resulting `dist` directory as `frontendDist`.

The default Tauri window still opens the main application entry, and no product navigation to the recorded harness was found.

## Concrete observations

### The separate harness is intentionally emitted by production builds

The second build input is not conditioned on development mode or a feature flag. Repository documentation explicitly says that a production build must contain both:

- `dist/index.html`;
- `dist/agent-session-harness.html`.

The Sprint 3 convergence record reports verifying both emitted artifacts. This is therefore a deliberate build requirement, not merely a source file accidentally left under `src/dev`.

### It shares the real presentation but substitutes the application boundary

The harness renders the same `AgentSessionScreen` used by the product. It replaces Tauri IPC and persisted native data with `createRecordedAgentSessionClient` and recorded DTO scenarios.

The controls can:

- select scenarios;
- advance one or all recorded updates;
- reset the recorded store;
- remount the screen while retaining the store.

StrictMode is intentionally omitted so manual one-step inspection stays deterministic. Scenario selection can also be supplied through the page query string.

### The recorded client models behavior, not the Codex protocol

The client implements the application-facing Agent Session contract: create, list, load, reload, subscribe, send, cancel, disconnect, and correlated updates. It preserves durable-looking history across a second client or remount using its in-memory store.

Repository guidance explicitly limits the claim: raw payload is opaque fixture data, the harness does not reproduce Codex JSONL or Rust evidence records, and it does not prove provider behavior.

This surface is strongest for presentation, subscription, reload, remount, and error-state behavior.

### Built does not mean product-reachable

The normal entrypoint is `index.html` -> `src/main.tsx` -> `ApplicationRoot`. No application link, Tauri command, window definition, or route to `agent-session-harness.html` was found. The documented use is to run the development server and open the URL directly.

The build configuration nevertheless places the harness beside the normal application assets, and Tauri points at that distribution directory. This pass did not inspect a final installer, so it establishes build-output inclusion rather than final-package byte inclusion.

### Historical pruning was selective

Commit `4445a65` introduced this deterministic verification harness on 2026-07-12. Later orientation work explicitly retained it as “source-valued” while removing two superseded orientation harness HTML entries, their Vite inputs, and unreachable A/B or fixture scaffolding.

The repository has therefore already distinguished between a retained verification surface and disposable exploration surfaces. The retained item was chosen because it continued to exercise a shared product component and important reload behavior.

### Its location and lifecycle signals disagree

- Directory: development/test material (`src/dev`).
- Build role: unconditional production-build entry.
- Product navigation: absent.
- Component use: real product presentation.
- Data/runtime authority: deterministic recorded substitute.
- Historical disposition: explicitly retained.

No single label captures all of these facts.

## Unexpected connections

- A verification page can be excluded from normal application navigation while still being a required production-build artifact.
- The recorded client is also a durable-contract simulator for restart behavior, not merely a visual fixture.
- The same product component is verified through three different environments: React tests, the recorded browser harness, and Tauri-backed runtime use.
- Prior cleanup removed other secondary entries but deliberately kept this one, providing historical disposition evidence that source reachability alone would miss.

## Questions opened by the pass

- Is production-build inclusion still necessary, or would an explicit verification build preserve the intended value more clearly?
- Is direct access to the page possible or relevant in the packaged WebView environment?
- Which other development or recorded sources are deliberately part of production build outputs?
- What criteria distinguished the retained Agent Session harness from removed orientation harnesses, and should those criteria guide later cleanup?
- Would future maintainers understand its intended evidence boundary from its build configuration alone?

The pass does not call the harness a leftover. It establishes a deliberately retained verification asset whose source, build, product, and evidence roles are different.
