# Voice workflow visual exploration

2026-10-12. Branch: `codex/voice-workflow-visual-exploration`, based on main `3d9f7d09`.

These are built-in Image Gen concept mockups, shown inline in the conversation. They are layout explorations, not screenshots of implemented features or executable workflow definitions. Product code and the running voice configuration were not changed.

| Image | Focus |
| --- | --- |
| [01-workflow-canvas.png](01-workflow-canvas.png) | Typed stages, reaction shortcut, JEV inspector and separately scoped runtime |
| [02-live-trace.png](02-live-trace.png) | Request provenance, classification, delivery and current processing stage |
| [03-extension-workspace.png](03-extension-workspace.png) | Extension-owned runtime, microphone/output independence and ongoing work |

These could be complementary views of the same extension. The existing application screenshot in [reference-current-workflow.png](reference-current-workflow.png) and `src/theme/tokens.css` grounded the palette and controls.

## Visual corrections before implementation

- The first image incorrectly labels Digest as a Function. Digest is an Agent.
- The third image incorrectly depicts Digest distributing work. Router delegates; worker observations reach Digest, which requests Output. Quick receipt publishes independently of Output.
- Extra extension/workflow names in the first two images are illustrative filler, not proposed features.
- Runtime lifecycle controls, typed capability/function nodes and direct reaction routing in the canvas are proposed surfaces, not existing generic workflow capabilities.

## Generation brief

Common brief: one realistic 1440 × 1024 desktop concept per image; preserve Orchid's dark canvas, mint accents, readable typography and restrained separators. Present Voice as an extension rather than a permanent core feature. Distinguish agent, typed JEV capability, deterministic request-state operations and associated runtime. Use illustrative Lunch ideas and Release note draft tasks, with a natural request for more detail routing directly to Output while other work continues. Microphone paused does not stop the runtime, playback or delegated work. No debugging, recovery, deployment or app-store features.

Direction prompts: make the workflow graph primary with a JEV inspector; make a request's live trace primary with a route explanation; make the extension workspace primary with associated runtime ownership and independently continuing work. All three use sample activity, not live performance claims.
