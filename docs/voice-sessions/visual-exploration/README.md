# Voice workflow visual exploration

2026-10-12. Branch: `codex/voice-workflow-visual-exploration`, based on main `3d9f7d09`.

These are built-in Image Gen concept mockups, shown inline in the conversation. They are layout explorations, not screenshots of implemented features or executable workflow definitions. Product code and the running voice configuration were not changed.

| Image | Focus |
| --- | --- |
| [01-workflow-canvas.png](01-workflow-canvas.png) | Typed stages, reaction shortcut, JEV inspector and separately scoped runtime |
| [02-live-trace.png](02-live-trace.png) | Request provenance, classification, delivery and current processing stage |
| [03-extension-workspace.png](03-extension-workspace.png) | Extension-owned runtime, microphone/output independence and ongoing work |
| [04-prototype-workflow.png](04-prototype-workflow.png) | Working extension graph, actual stage bindings and separate runtime controls |
| [05-prototype-node.png](05-prototype-node.png) | Real request-buffer inspector, reaction switch and silent node notification setting |
| [06-prototype-activity.png](06-prototype-activity.png) | Recorded routes and delivery receipts from real JEV/agent runs with simulated text |

These could be complementary views of the same extension. The existing application screenshot in [reference-current-workflow.png](reference-current-workflow.png) and `src/theme/tokens.css` grounded the palette and controls.

## Phone viewer

`apps/orchid-android/concept-viewer` packages these three desktop mockups into **Orchid Concepts**, a separate offline application with Previous/Next, zoom/pan, Fit and Wide/Portrait controls. It preserves the desktop compositions and displays the corrections below; the depicted workflow controls are illustrations.

Installed and verified on S21 `R3CRA0BAYXD` over Tailscale on 2026-10-12. Build and lint passed (0 errors; 5 informational warnings). The device probe passed navigation, rendered content, Zoom and Fit. Captured screens confirmed all three images, panning and both orientations; landscape retains the selected design and uses compact controls. Production Orchid Voice was not rebuilt or reconfigured. Local evidence: `.dev/concept-viewer-phone-tests.txt` and `.dev/concept-viewer-*.png`.

The implementation experiment is on `codex/voice-extension-workflow`. Viewer 0.2 adds screens 4–6 from that native desktop prototype; all six images, navigation, Zoom and Fit passed the S21 device probe. The original mockups remain screens 1–3. See [prototype shape and evidence](../../architecture/voice-extension-workflow-prototype.md).

## Visual corrections before implementation

- The first image incorrectly labels Digest as a Function. Digest is an Agent.
- The third image incorrectly depicts Digest distributing work. Router delegates; worker observations reach Digest, which requests Output. Quick receipt publishes independently of Output.
- Extra extension/workflow names in the first two images are illustrative filler, not proposed features.
- Runtime lifecycle controls, typed capability/function nodes and direct reaction routing in the canvas are proposed surfaces, not existing generic workflow capabilities.

## Generation brief

Common brief: one realistic 1440 × 1024 desktop concept per image; preserve Orchid's dark canvas, mint accents, readable typography and restrained separators. Present Voice as an extension rather than a permanent core feature. Distinguish agent, typed JEV capability, deterministic request-state operations and associated runtime. Use illustrative Lunch ideas and Release note draft tasks, with a natural request for more detail routing directly to Output while other work continues. Microphone paused does not stop the runtime, playback or delegated work. No debugging, recovery, deployment or app-store features.

Direction prompts: make the workflow graph primary with a JEV inspector; make a request's live trace primary with a route explanation; make the extension workspace primary with associated runtime ownership and independently continuing work. All three use sample activity, not live performance claims.
