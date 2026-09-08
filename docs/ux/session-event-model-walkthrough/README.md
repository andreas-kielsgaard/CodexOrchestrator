# Session Event model UI walkthrough

The current walkthrough is `output/Session Event UI Demo Walkthrough v7.pptx`.
It covers the merged UI screen by screen, using captures from the local browser fixture.

The merged branch now shows:

1. Capability Profile list, details, and allowed capabilities.
2. The Workflow flow canvas with node and connection editing.
3. Node copy, prompt, capability, and pinned default sections.
4. Connection triggers, ordered prompt sources, and Session addressing.
5. Workflow instance creation, saved instances, and Sessions inside an instance.
6. Per-message model and reasoning controls.
7. Session Profile details, delivery records, and Agent identity editing.
8. The older Harness Management screen that still remains in the product.

The deck also marks the main visible problems:

- Some connection checkboxes are too large and sit too far from their labels.
- Open Session settings are covered by the fixed conversation and composer layers.
- Long delivery IDs wrap badly in the narrow instance column.
- Harness Management still overlaps in purpose with the new profile screens.

Evidence limits:

- The screenshots use the real React screens with local fake clients.
- No live provider call was made.
- The desktop-native window and real persistence were not tested in this walkthrough.

The source captures are in `screenshots/merged-2026-09-07`. The presentation speaker notes
give a short description for each slide.
