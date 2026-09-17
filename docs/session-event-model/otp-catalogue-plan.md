# OTP catalogue and package details

Planned against `codex/OTP-Job-Agent` at `9e11e45` (`feat: add Job Agent OTP integration`). The worktree was clean before this plan was added. This is a plan only; it does not claim validation or implementation.

## Objective

Make imported Orchestration Tool Packages understandable from Technical Settings while retaining Orchid's existing visual language. The flow has three levels:

1. An OTP directory lists imported packages and a compact inventory of their offerings.
2. Selecting **View details** shows a package-local detail page with its description and offered elements.
3. Selecting an element opens one reusable details dialog, with sections appropriate to that element's role.

The third visual exploration defines the information hierarchy, not the visual style. The implementation uses the established Technical Settings palette, typography, panel treatment, buttons, and native dialog patterns.

## Accepted model and scope

Use these terms in the descriptor and UI:

- **Package / OTP**: an imported package such as Workflow or Job Agent.
- **Workflow tool**: a package offering with an MCP, session-event, or action entrypoint.
- **MCP service**: an MCP server delivered to configured agent sessions.
- **Capability group**: a coherent group within an MCP service. Job Agent's existing `source_discovery`, `recipe_authoring`, and related capability identifiers become these groups.
- **MCP endpoint**: one tool in a capability group.
- **Grant**: a separately configured authority for a guarded operation. It is neither an endpoint nor a capability group.

The viewer derives a Workflow tool's role from its existing entrypoint and output contract:

- MCP and session-event tools with data outputs appear as **Trigger and event sources**.
- action tools that request session work appear as **Destinations**.
- agent MCP endpoints appear under their MCP service and capability group, with the integration statement **Available to configured agent sessions**.

Do not add a generic compatibility flag, a second workflow-entry category, a dynamic package marketplace, arbitrary cross-OTP calls, or new routing semantics. A future OTP contribution can become a direct Workflow trigger or destination only by supplying the existing Workflow tool contract. The Workflow engine retains routing, session delivery, and durable state.

`src-tauri/src/otp_packages/job_agent/mod.rs` remains the source of truth for the locally imported Job Agent OTP catalogue in this slice. The Job Agent Python bridge remains the source of executable endpoint behavior and is verified for operational compatibility. This avoids making installation state a competing descriptor source or requiring a dynamic external-package loader before it is needed.

## Current findings

- `PackageDescriptor` currently has an identifier, protocol version, requested handles, Workflow tools, and agent MCP servers. It has no package presentation metadata, capability-group descriptions, endpoint schemas, behavior, usage guidance, grants, or explicit endpoint-to-grant relationship.
- Workflow tools already expose entrypoints, input schemas, outputs, and configuration. The Workflow OTP supplies five tools through `src-tauri/src/otp_packages/workflow/mod.rs`.
- The Job Agent package currently generates a flat list of 25 endpoint names and six capability strings. Its endpoint descriptions are placeholders, and its grant records are separate tuples.
- `OtpRegistry::catalogue()` is already the product-owned source of catalogue data. Its Tauri command is currently named `list_workflow_capabilities` and owned by the Workflow Authoring transport despite also serving Technical Settings and capability profiles.
- `src/features/otp/otpElements.tsx` and `src/components/otp/OtpElementPicker.tsx` already provide a shared, accessible workflow-tool detail renderer and native dialog foundation. They should be extended rather than duplicated.
- `OtpConfigurationPanel` currently combines the directory, the Job Agent installation form, and shallow package rows. The App has no OTP-specific global destination; adding one would be unnecessary for a settings-local page and breadcrumb.

## Work packages

### P1 — Enrich the OTP catalogue contract

**Outcome:** every locally imported OTP can describe its purpose and its offered elements without UI-owned hardcoding.

- Extend `src-tauri/src/otp_api/contract.rs` and `src/application/otp/contracts.ts` with matching serializable catalogue fields:
  - package display name, summary, and full description;
  - capability-group identifier, name, and description on an agent MCP server;
  - endpoint input schema, output schema, expected behavior, recommended usage, and required grant identifiers;
  - server grant descriptors with stable IDs, labels, descriptions, capability, and transition.
- Retain `id`, `contractVersion`, `Entrypoint`, `OutputDescriptor`, and `ConfigurationField` as execution contracts. The new fields are catalogue metadata except for stable capability/group/grant identity; neither the renderer nor prose controls execution.
- Keep endpoint membership normalized: an endpoint retains one `capability` group ID, and the UI groups it by that ID. Groups do not duplicate endpoint arrays.
- Use the existing entrypoint/output fields for Workflow integration. Add a small presentation helper that translates them into the current supported integration statements rather than inventing a broad inter-package protocol.
- Add `Deserialize` only where needed for existing product validation. Do not widen the JSON validator or begin validating arbitrary remote MCP argument/response shapes; Job Agent continues to validate its own MCP calls.
- Update descriptor serialization and fixture tests so the TypeScript fixture contains Workflow and Job Agent catalogue metadata. Assert stable IDs, valid group references, and endpoint grant references.

### P2 — Give each built-in package a complete, coherent catalogue

**Outcome:** the two imported packages use the same contract but describe their own offerings accurately.

- Refactor `src-tauri/src/otp_packages/job_agent/mod.rs` away from flat `TOOLS` and `GRANTS` tuples. Define named capability-group and grant descriptor data once, then derive its agent MCP server descriptor and configuration fields from it.
- Replace generated endpoint labels and placeholder descriptions with the purpose already embodied in Job Agent's MCP tool definitions. Declare concise documentation schemas for endpoint arguments and result envelopes, exposing useful named inputs and key result fields while leaving domain-specific nested objects as objects. Do not model every Job Agent persistence shape in Orchid.
- Link guarded endpoints to the existing registry, calibration, recipe-authoring, and execution grant IDs. Read-only endpoints have no required grants. Preserve the current default-deny configuration and the bridge's capability/transition enforcement.
- Extend `src-tauri/src/otp_packages/workflow/mod.rs` with package name, summary, description, and concise expected behavior/recommended use for its five tools. Preserve the current `trigger_workflow_continuation` scope exactly: it starts connections configured for the exact calling source node and trigger, with no lifecycle semantics.
- Improve `src-tauri/src/otp_host/installations.rs` verification to compare the Job Agent bridge's endpoint names and capability-group identifiers with the imported descriptor. Keep the bridge manifest narrow; it remains an executable compatibility probe, not a second copy of endpoint documentation.
- Add focused Rust tests beside the Job Agent and Workflow packages for catalogue completeness, group/grant linkage, and bridge-manifest parity.

### P3 — Move catalogue querying to the OTP boundary

**Outcome:** all UI consumers read one product-owned OTP catalogue through a correctly named, reusable query boundary.

- Create a small Tauri catalogue query state/command under `src-tauri/src/otp_host/`, backed directly by `Arc<OtpRegistry>`. Name the command `list_otp_catalogue`.
- Register it in `src-tauri/src/active_app.rs`. Remove `list_workflow_capabilities` from `src-tauri/src/workflows/authoring_transport.rs`; workflow authoring retains recipe commands only.
- Add `src/infrastructure/otp/tauriOtpCatalogueReader.ts` implementing the existing `OtpCatalogueReader`. Export it with the OTP infrastructure and pass it from `src/bootstrap/productApplicationComposition.ts` through `src/app/App.tsx`.
- Remove `listCapabilities` from `WorkflowAuthoringClient` and its Tauri client. Supply the reader explicitly to Workflow Authoring, Capability Profiles, and Technical Settings. Update test clients and fixtures together so no feature owns a shadow catalogue.
- Keep `OtpRegistry` as the sole package import/runtime authority. This work does not change package import configuration, Job Agent provisioning, session profile storage, or Workflow compilation behavior.

### P4 — Build the settings-local OTP directory and package page

**Outcome:** Technical Settings presents a browseable package catalogue and an OTP detail page without a new global app route.

- Replace the package-row portion of `src/features/technicalSettings/OtpConfigurationPanel.tsx` with a coordinator in a new `src/features/otpCatalogue/` feature area. It owns loaded catalogue state and a local `selectedPackageId` page state.
- Create focused components such as `OtpPackageDirectory.tsx`, `OtpPackageDetailPage.tsx`, `OtpElementDetailsDialog.tsx`, and `otpCataloguePresentation.ts`. The presentation helper maps raw descriptors into package counts, Workflow source/destination sections, MCP services, capability groups, and stable element identities. It does not fetch or mutate product state.
- Directory cards use the supplied package name, summary, import status, and counts by offered element type. They contain **View details**. No search, sort, package import, enable/disable, or install controls are part of this slice.
- The detail page uses a breadcrumb/back control to return to the directory. It shows the package description and its offering sections. Workflow shows sources and destinations. Job Agent shows its service, capability groups, and nested endpoint rows.
- Move the Job Agent root/Python installation form into the Job Agent package detail page as a package-specific child supplied by the Technical Settings coordinator. Keep it out of generic catalogue components and preserve its save-and-verify behavior, status, loading state, and error handling.
- Reuse `technicalSettings.css` conventions and extend it only with OTP catalogue selectors. Reuse the repository's native `<dialog>` accessibility/focus behavior. Do not import generated imagery or copy the exploration board's unrelated visual language.

### P5 — Generalize element details and reuse them in selectors

**Outcome:** details are consistent in the package page, modal, and existing OTP pickers.

- Refactor `src/features/otp/otpElements.tsx` into a typed element-detail renderer that accepts Workflow tools/outputs, MCP services, capability groups, and MCP endpoints. Keep it presentation-only and derive labels from the P1 contract.
- The package-page dialog wraps that renderer and provides accessible heading, Escape/backdrop-close behavior, focus return, and a visible element-type label.
- Render common fields first: purpose, expected behavior, recommended use, and integration statement. Then render type-specific sections:
  - Workflow source: entrypoint/event, invocation input where applicable, offered output fields, and connection-routing behavior.
  - Workflow destination: accepted configuration, session effect, and declared request result.
  - MCP capability group: intent, endpoint list, and agent-session availability.
  - MCP endpoint: input schema, result schema, required grants, and agent-session availability.
- Update `WorkflowTriggerPicker.tsx`, `WorkflowDestinationActionPicker.tsx`, and `OtpMcpToolsPicker.tsx` to consume this renderer where they already show details. Preserve their selection and eligibility behavior; this work improves explanatory content and removes duplicate formatting only.
- Do not make capability groups independently selectable in profiles or Workflow connections. They remain a catalogue/navigation layer while individual MCP endpoints remain selected by existing profile tooling.

## Validation

1. Rust contract/package tests verify every rendered reference resolves: endpoint-to-group, endpoint-to-grant, Workflow tool-to-output, and Job Agent bridge tool/capability parity.
2. Frontend tests cover directory loading/error/empty states, package counts, opening and returning from details, Job Agent installation persistence in its new location, nested capability groups, and each element-detail type.
3. Extend the existing picker tests to show enriched details without changing selection outcomes. Cover dialog focus return and Escape using the native dialog path already exercised by the picker.
4. Run affected Vitest suites, focused Rust tests, the frontend build, the Rust build, and `git diff --check`.
5. Launch the native Orchid instance and inspect the real Technical Settings flow: directory, Workflow details, Job Agent details after verification, a Workflow source modal, an MCP endpoint modal, back navigation, and narrow-window layout. Record native UI evidence separately from automated checks.

## Deliberate deferrals

No package installer, remote registry, dynamically loaded external OTP descriptors, general OTP permissions model, new deterministic Workflow destination, package documentation URLs, changelog/history view, endpoint search, sortable directory, dynamic schema forms, or expanded Workflow routing behavior. The details view documents currently declared capabilities; it does not make additional capabilities executable.
