# Open questions

These questions could materially change the current or nearby product explanation. They are not presumed defects, final disposition proposals or a checklist that must be exhausted. Future investigation can select the questions relevant to the work at hand and leave the rest as recorded uncertainty.

## Product and scope

- Which capabilities are part of the intended product, operator tooling, development support, or retained historical evidence?
- Which implemented capabilities are valuable but not yet reachable through a coherent user experience?
- Where has implementation depth outgrown the current product value or visibility?
- Which sibling-line capabilities represent cumulative product work rather than alternatives?
- Which branch-line convergence target is authoritative across the inspected operational line, Product Decisions, final settlement and moving uncommitted transitions?

## Architecture

- What is the effective boundary between Tauri composition/transport and reusable Rust backend logic?
- Which modules combine domain policy, persistence, process control, transport, and configuration strongly enough to obscure ownership?
- What should be the single authority for Harness prompts, skills, tools, models, sandbox policy, and completion criteria?
- Which lifecycle facts and settlement boundaries are duplicated across services or projections?
- Should MCP hosting, security, lifecycle, and Codex injection become shared infrastructure?
- How should the current database and migration surface be segmented by product ownership, retention policy, and lifecycle?
- Where should newer execution/settlement architecture decisions be recorded now that implementation has outgrown the initial decision-record series?
- Is the compile-time application checkout intentionally the product's repository-selection and integration authority, or a convergence adapter for a more general prepared-runtime boundary?
- How should immutable Sprint origin authority and the mutable accepted-integration target relate after the target branch advances?
- Is startup reconciliation's ability to create Sessions, worktrees, MCP endpoints and Codex processes an explicit supported recovery contract?
- Which process families should participate in one application-wide shutdown and durable recovery protocol?

## Implementation

- Which Tauri commands and MCP tools have complete productive callers and consumers?
- Which compiled commands are unavailable, debug-only, test-only, stale, or unreferenced?
- Which compatibility projections and recorded fixtures remain required by productive code?
- Which database migrations protect real retained data, and which exist only for historical fixtures?
- Which Rust and JavaScript worktree-runtime implementation is authoritative?
- Is the native MCP probe initiation/reconciliation split complete and intentional?
- Should the current Agent Session binding consume only selected Native Profile identity, or also converge on Native Profile execution mode, danger authority and strict launch projection?
- What productive producer should create contextual File Review sessions outside debug builds?
- Which static Harness catalogue entries are descriptions, and which are intended to resolve to code-generated or durable revisions at runtime?
- Which inherited environment and Codex configuration values are part of supported runtime behavior versus incidental process inheritance?
- Should every managed invocation have a durable, secret-safe effective-launch record that explains Harness, prompt, profile, MCP, environment, arguments and ownership together?
- Is Implementer reporting allowed to start a fresh Codex context when the intended same-Session external context is absent?

## Experience and design

- Which views and components form a coherent reusable product system?
- Where does feature ownership create accidental cross-feature dependencies?
- Which status vocabularies describe the same concept differently?
- Which backend or test boundaries leak into visible product structure or copy?
- Should Harness Management, Human Review, Product Decisions, and File Review be primary product surfaces, contextual tools, or operator-only experiences?
- How should evidence, attention, retries and settlement states be normalized without hiding genuinely different lifecycle facts?
- Which screens promise live state, navigation-time state or mount-time state, and how should automatic startup work become legible without turning the product into an operations console?
