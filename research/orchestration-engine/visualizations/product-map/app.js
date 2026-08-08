const statusNames = {
  current: "Current product",
  emerging: "Emerging capability",
  conditional: "Conditional connection",
  retained: "Retained implementation",
};

const research = {
  current: ["Current product reading", "../../current-state/README.md"],
  nearby: ["Emerging capabilities", "../../current-state/near-future-and-moving-work.md"],
  frontend: ["Frontend experience map", "../../catalogs/frontend-experience-map.md"],
  backend: ["Rust backend and Tauri boundary", "../../catalogs/backend-and-tauri.md"],
  tauri: ["Tauri operation catalogue", "../../catalogs/tauri-operations.md"],
  harness: ["Harness and configuration authority", "../../catalogs/harness-and-configuration.md"],
  mcp: ["MCP servers and tools", "../../catalogs/mcp-servers-and-tools.md"],
  durable: ["Durable state ownership", "../../catalogs/durable-state.md"],
  code: ["Code artifact map", "../../catalogs/code-artifact-map.md"],
  workUnit: ["Work Unit delivery trace", "../../operation-traces/work-unit-execution-review-and-settlement.md"],
  planBuilder: ["Plan Builder trace", "../../operation-traces/plan-builder-proposal-and-initiation.md"],
  nativeProfile: ["Native Profile launch trace", "../../operation-traces/native-profile-readiness-and-launch.md"],
  implementationLines: ["Material implementation lines", "../../history/implementation-lines.md"],
  findings: ["Cross-cutting findings", "../../current-state/cross-cutting-system-findings.md"],
};

function concept(status, title, thesis, insight, relationships, artifacts, evidence) {
  return { status, title, thesis, insight, relationships, artifacts, evidence };
}

const concepts = {
  "shape-epic": concept(
    "current",
    "Shape the Epic",
    "Conversation produces a durable proposal; initiation remains a separate human action.",
    "This is the clearest product pattern in the current engine: an agent helps shape intent, but application-owned proposal state makes the result inspectable and the user still owns the consequential transition.",
    [
      ["Agent Session", "The discussion runs on the shared conversation platform rather than a bespoke planning runtime."],
      ["Durable proposal", "Proposal versions and initiation state live outside the transcript."],
      ["Scoped MCP", "The agent receives proposal operations, not general orchestration authority."],
    ],
    [
      "src/features/orchestrations/EpicPlanBuilder.tsx",
      "src-tauri/src/orchestration/application.rs",
      "src-tauri/src/orchestration/mcp.rs",
    ],
    [research.planBuilder, research.current],
  ),
  bootstrap: concept(
    "current",
    "Initiate and bootstrap",
    "One explicit confirmation opens a largely automatic transition into durable operating context.",
    "The bootstrap is not merely agent startup. It reconciles proposal authority, prepared files, role identity, Harness policy, MCP scope, Native Profile readiness, Session creation, and launch observations.",
    [
      ["Human boundary", "Initiation is explicit and distinct from proposal save."],
      ["Application authority", "Rust owns preparation, reconciliation, and launch coordination."],
      ["Shared runtime", "The resulting Epic Runner uses the common Agent Session platform."],
    ],
    [
      "src-tauri/src/orchestration/bootstrap_transition.rs",
      "src-tauri/src/active_app.rs",
      "src-tauri/src/agent_sessions/",
    ],
    [research.current, research.backend],
  ),
  "start-sprint": concept(
    "current",
    "Start a Sprint",
    "Epic-level intent becomes one bounded operational chapter with its own durable ownership.",
    "The useful distinction is not another agent title. The Epic remains strategic while a selected Sprint establishes a smaller authority and lifecycle boundary that can be recovered after restart.",
    [
      ["Epic Runner", "Selects the next bounded objective."],
      ["Sprint Runner", "Receives durable identity and managed execution context."],
      ["Native query", "Projects the transfer without inferring it from conversation text."],
    ],
    [
      "src-tauri/src/orchestration/sprint_runner_transition.rs",
      "src/application/orchestrations/nativeQuery.ts",
      "src/features/orchestrations/components/SprintWorkspace.tsx",
    ],
    [research.current, research.code],
  ),
  "plan-work": concept(
    "current",
    "Plan current work",
    "Planning happens at the current decision point rather than expanding the entire Epic in advance.",
    "The Work Slice and planning-point model is a temporal control: it constrains what must be decided now, records dependencies, and leaves later planning responsive to what execution reveals.",
    [
      ["Sprint objective", "Provides the bounded chapter being advanced."],
      ["Work Slice Planner", "Owns the present planning episode and its accepted revision."],
      ["Executable graph", "Accepted planning becomes ordered Work Units and explicit relationships."],
    ],
    [
      "src-tauri/src/orchestration/sprint_runner_transition.rs",
      "src/features/orchestrations/components/WorkSlicePlanningPointDetailWorkspace.tsx",
      "src/features/orchestrations/components/SprintWorkspace.tsx",
    ],
    [research.current, research.workUnit],
  ),
  "deliver-work": concept(
    "current",
    "Deliver Work Units",
    "One visible Work Unit is implemented through several separately authorized and observed stages.",
    "The important insight is separation: Handler activation, Implementer work, reporting claims, captured evidence, Handler judgment, candidate pinning, and integration do not become true at the same moment.",
    [
      ["Handler", "Owns the bounded responsibility and requests an Implementer through a scoped continuation."],
      ["Implementer", "Changes code in the exact application-authorized workspace, then reports separately."],
      ["Evidence and review", "Application evidence and an independent Handler judgment stand between claims and acceptance."],
      ["Settlement", "Git integration and durable Work Unit settlement remain separate coordinated facts."],
    ],
    [
      "src-tauri/src/orchestration/work_unit_execution_harness.rs",
      "src-tauri/src/orchestration/execution_support.rs",
      "src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx",
    ],
    [research.workUnit, research.findings],
  ),
  "review-integrate": concept(
    "current",
    "Review and integrate",
    "Accepted work crosses an application-owned evidence and Git settlement boundary.",
    "Review is not a decorative final step. The engine pins an exact candidate, revalidates repository identity, advances Git with compare-and-swap semantics, records immutable evidence, and only then settles dependencies.",
    [
      ["Review judgment", "An accept call remains pending until the exact review invocation completes."],
      ["Candidate authority", "The accepted commit and tree are pinned under private application authority."],
      ["Integration", "Git advancement and durable settlement are coordinated but not conflated."],
    ],
    [
      "src-tauri/src/orchestration/accepted_candidate_authority.rs",
      "src-tauri/src/orchestration/accepted_integration.rs",
      "src-tauri/src/orchestration/work_unit_dependency_wave.rs",
    ],
    [research.workUnit, research.durable],
  ),
  "epic-initiate-action": concept(
    "current",
    "Initiate action",
    "The visible action can request confirmation only for the current initiation-ready proposal.",
    "The React workspace does not initiate an Epic directly. It requires a ready capability, passes the exact application-composed request into the confirmation route, and refreshes durable proposal state after failure.",
    [
      ["Proposal authority", "The action is disabled unless the current proposal produces a ready initiation capability."],
      ["Confirmation boundary", "Opening the confirmation route is still only a request; it is not the user decision."],
      ["Failure recovery", "The workspace re-queries proposal state instead of treating a rejected command as authoritative state."],
    ],
    [
      "src/features/orchestrations/EpicPlanBuilder.tsx",
      "src/app/EpicInitiationConfirmationModal.tsx",
      "src/application/orchestrations/epicInitiationConfirmation.ts",
    ],
    [research.planBuilder, research.frontend],
  ),
  "epic-confirmation-client": concept(
    "current",
    "Capability and confirmation client",
    "A typed frontend adapter turns the proposal-specific request into an application-owned confirmation exchange.",
    "The client keeps request, resolution, and emitted state separate. It decodes every response and maps malformed or unavailable transport behavior into an explicit confirmation failure rather than guessing from UI state.",
    [
      ["Request", "Creates the pending application confirmation for the exact draft, revision token, and idempotency key."],
      ["Resolution", "Sends the request ID and the user's confirmed or rejected decision separately."],
      ["Observation", "A subscription projects confirmation progress without making the transcript authoritative."],
    ],
    [
      "src/infrastructure/orchestrations/tauriEpicInitiationConfirmation.ts",
      "src/application/orchestrations/epicInitiationConfirmation.ts",
    ],
    [research.planBuilder, research.tauri],
  ),
  "epic-confirmation-transport": concept(
    "current",
    "Tauri request and resolution",
    "Two Tauri commands carry pending confirmation and explicit resolution across the desktop boundary.",
    "The command handlers are deliberately narrow: they construct the application-user initiation command, identify the source as the visible button, translate the decision, and delegate to the coordinator. Confirmation semantics remain beyond transport.",
    [
      ["request_epic_initiation_confirmation", "Registers a pending request after validating the typed draft identity."],
      ["resolve_epic_initiation_confirmation", "Correlates the decision through the request ID."],
      ["Coordinator", "Owns idempotency, timeout, notification, and the transition from confirmed to applied."],
    ],
    [
      "src-tauri/src/orchestration/transport.rs",
      "src-tauri/src/orchestration/confirmation.rs",
      "src-tauri/src/lib.rs",
    ],
    [research.tauri, research.planBuilder],
  ),
  "epic-confirmed-initiation": concept(
    "current",
    "Confirmed initiation",
    "Only a confirmed request reaches the durable Epic initiation operation.",
    "The coordinator applies the stored command after confirmation. The Rust application delegates to repository authority, where the expected proposal revision is revalidated and initiation is recorded before downstream bootstrap work is allowed to matter.",
    [
      ["User decision", "Confirmed and rejected are explicit terminal choices for the pending request."],
      ["Revision check", "The expected proposal revision prevents a stale confirmation from initiating changed material."],
      ["Durable fact", "The initiation result exists before post-confirmation callbacks and projections are attempted."],
    ],
    [
      "src-tauri/src/orchestration/confirmation.rs",
      "src-tauri/src/orchestration/application.rs",
      "src-tauri/src/orchestration/repository.rs",
    ],
    [research.planBuilder, research.durable],
  ),
  "epic-bootstrap-reconciliation": concept(
    "current",
    "Bootstrap reconciliation",
    "Persisted initiation becomes prepared material and managed Sessions through a restart-safe reconciler.",
    "The post-confirmation service derives application-owned paths from the durable snapshot, ensures a transition record, and repeatedly reconciles the next missing stage. Startup reconciliation and invocation observations keep file creation, Session creation, launch, semantic completion, and Runner activation distinct.",
    [
      ["Prepared material", "Approved plan input and transition files derive from the persisted proposal snapshot."],
      ["Managed Sessions", "Bootstrap generation and Epic Runner launch are separate Session and invocation stages."],
      ["Recovery", "Startup reads persisted snapshots and transition stages rather than replaying the button action."],
    ],
    [
      "src-tauri/src/orchestration/bootstrap_transition.rs",
      "src-tauri/src/agent_sessions/application/",
      "src-tauri/src/runtime/codex/",
    ],
    [research.planBuilder, research.backend, research.durable],
  ),
  resilience: concept(
    "emerging",
    "More resilient Work Unit execution",
    "Locally present implementation strengthens Handler recovery and makes pre-readiness activation failure explicit.",
    "This improves an existing product capability rather than introducing a new product area. Its value is more truthful recovery and failure projection across durable attempt and workspace boundaries.",
    [
      ["Existing delivery spine", "The change extends Handler activation and execution support already present in the stable product."],
      ["Visible truth", "Durable failure becomes distinguishable from waiting, launch acceptance, or readiness."],
      ["Product horizon", "It is implemented nearby but not included in the stable research snapshot."],
    ],
    [
      "src-tauri/src/orchestration/execution_support.rs",
      "src-tauri/src/orchestration/bootstrap_transition.rs",
      "src/application/orchestrations/nativeQuery.ts",
    ],
    [research.nearby, research.workUnit],
  ),
  governance: concept(
    "emerging",
    "Product governance and correction",
    "A substantial implementation adds evidence-linked Product Decisions, version history, and correction flows.",
    "This changes the product from a workflow executor toward a governed product record. It matters as a possible capability direction, not because of where its source happens to be checked out.",
    [
      ["Evidence", "Decisions can be tied to inspectable product history."],
      ["Correction", "Conversation becomes a path to a versioned correction proposal."],
      ["Product shell", "Navigation and exact return behavior expand alongside the decision capability."],
    ],
    ["Product Decisions views and durable version services", "Typed product navigation and correction flows"],
    [research.implementationLines, research.frontend],
  ),
  closure: concept(
    "emerging",
    "Explicit continuation and completion",
    "Another substantial implementation closes the gap between settled work and an explicitly settled Sprint or Epic.",
    "The stable engine is strongest through Work Unit and planning-point settlement. This capability matters because product completion should be an explicit durable outcome, not an optimistic reading of downstream silence.",
    [
      ["Work Unit settlement", "Provides the accepted contributions consumed by later closure."],
      ["Sprint continuation", "Turns a completed chapter into a deliberate successor or terminal result."],
      ["Epic outcome", "Requires exact final settlement rather than inference from constituent work."],
    ],
    ["Orchestration persistence and transition services", "Epic and Sprint native-query projection"],
    [research.nearby, research.implementationLines],
  ),
  "architecture-surfaces": concept(
    "current",
    "Product surfaces",
    "Several visible products consume one shared orchestration and Agent Session foundation.",
    "The surface layer is broader than the Epic screen: orchestration, standalone Agent Sessions, technical settings, contextual File Review, and conditional management/review views all expose different portions of the same operating system.",
    [
      ["Primary", "Orchestration, Agent Sessions, and Native Settings are release-composed surfaces."],
      ["Contextual", "File Review is valuable but has split reachability."],
      ["Conditional", "Harness and Worktree Review expose deeper machinery to narrower audiences."],
    ],
    ["src/app/App.tsx", "src/features/orchestrations/", "src/features/agentSessions/"],
    [research.frontend, research.current],
  ),
  "tauri-boundary": concept(
    "current",
    "Tauri transport boundary",
    "Tauri registers desktop operations and adapts payloads; most orchestration semantics live beyond it.",
    "The useful split is not TypeScript versus Rust. Tauri is primarily the desktop transport and composition edge, while Rust application services own validation, lifecycle, persistence, and effects.",
    [
      ["Frontend clients", "Invoke named operations through typed adapters."],
      ["Command handlers", "Deserialize, delegate, and convert failures into transport responses."],
      ["Rust applications", "Own the consequential behavior after the transport crossing."],
    ],
    ["src-tauri/src/lib.rs", "src-tauri/src/agent_sessions/transport/", "src-tauri/src/orchestration/transport.rs"],
    [research.tauri, research.backend],
  ),
  "application-authority": concept(
    "current",
    "Rust application authority",
    "Application services and transition machines own the orchestration lifecycle rather than the UI or agent transcript.",
    "This is where product authority concentrates: state is validated, exact identities are correlated, recovery is reconciled, and external effects are authorized. Several vertically large modules preserve this end-to-end truth.",
    [
      ["Domain state", "Typed identities and phase constraints define valid progress."],
      ["Transition services", "Coordinate durable stages and effects across restart boundaries."],
      ["Native projection", "Returns current product truth to the frontend without transcript inference."],
    ],
    [
      "src-tauri/src/orchestration/bootstrap_transition.rs",
      "src-tauri/src/orchestration/sprint_runner_transition.rs",
      "src-tauri/src/agent_sessions/application/",
    ],
    [research.backend, research.findings],
  ),
  "policy-plane": concept(
    "current",
    "Executable policy",
    "Harness revisions, scoped MCP capabilities, Native Profile identity, and source authority directly shape execution.",
    "These elements look like configuration but behave as application functionality. They decide what an agent is told, which tools exist, which environment is selected, and where authorized effects may occur.",
    [
      ["Harness", "Immutable role revisions bind instructions, model settings, skills, and tool declarations."],
      ["MCP", "Invocation-scoped tool sets expose bounded semantic actions."],
      ["Native Profile", "Selected filesystem identity and readiness gate the shared launch path."],
      ["Source authority", "The application checkout becomes planning context and Git target authority."],
    ],
    [
      "src-tauri/src/orchestration/conversation_harness.rs",
      "src-tauri/src/orchestration/mcp.rs",
      "src-tauri/src/native_profiles.rs",
    ],
    [research.harness, research.mcp, research.nativeProfile],
  ),
  "durable-effects": concept(
    "current",
    "Durable state and external effects",
    "SQLite, files, Git refs, worktrees, and child processes form one operating model with distinct authorities.",
    "The architecture repeatedly coordinates a durable intent with an external effect and later reconciliation. Correctness depends on preserving those boundaries rather than pretending one database transaction controls every system.",
    [
      ["SQLite", "Owns canonical orchestration state, correlations, attention, and settlement records."],
      ["Filesystem", "Holds prepared materials, evidence objects, Harness sources, and runtime homes."],
      ["Git", "Owns repository identity, candidates, comparisons, and accepted integration refs."],
      ["Processes", "Launch acceptance and provider activity remain observations, not semantic completion."],
    ],
    ["src-tauri/src/storage.rs", "src-tauri/src/orchestration/repository.rs", "src-tauri/src/runtime/codex/"],
    [research.durable, research.workUnit],
  ),
  "frontend-entry": concept(
    "current",
    "React product entry",
    "Visible actions begin in feature-owned workspaces, not in a generic orchestration dashboard.",
    "The frontend is already organized around meaningful work contexts—Epic planning, Sprint progress, Work Unit delivery, Agent Sessions, and technical readiness—but large native projections make some boundaries harder to see in code.",
    [
      ["Feature workspace", "Owns user intent, selection, and progressive disclosure."],
      ["Application contracts", "Typed read models sit between views and native transport."],
      ["Shared components", "Agent conversation, detail layout, Markdown, identity, and evidence are reused unevenly."],
    ],
    ["src/features/orchestrations/", "src/features/agentSessions/", "src/application/orchestrations/"],
    [research.frontend, research.code],
  ),
  "frontend-adapters": concept(
    "current",
    "Frontend application and adapter layer",
    "TypeScript clients translate product actions into native operations and parse large durable projections.",
    "This boundary contains more than plumbing: strict parsers defend frontend truth, composers derive product presentation, and infrastructure adapters decide whether the application is using recorded or Tauri-backed behavior.",
    [
      ["Contracts", "Describe product-owned operations independently of Tauri invocation details."],
      ["Parsers", "Reject malformed or incoherent native state before it reaches the UI."],
      ["Composition", "Selects release, recorded-development, or review-specific implementations."],
    ],
    [
      "src/application/orchestrations/nativeQuery.ts",
      "src/infrastructure/orchestrations/tauriOrchestrationNativeQuery.ts",
      "src/bootstrap/productApplicationComposition.ts",
    ],
    [research.code, research.tauri],
  ),
  "command-entry": concept(
    "current",
    "Desktop command entry",
    "A named Tauri operation crosses into Rust, then delegates to an application-owned service.",
    "The command list is useful as an API catalogue, but it should not be mistaken for the backend architecture. Commands are transport entry points; the consequential operation often spans services, repositories, policy builders, and external adapters.",
    [
      ["Invoke registration", "Defines what the WebView can call."],
      ["Transport DTO", "Converts between serialization and domain-shaped inputs."],
      ["Application service", "Performs authorization, transition, persistence, and effects."],
    ],
    ["src-tauri/src/lib.rs", "src-tauri/src/orchestration/transport.rs", "src-tauri/src/agent_sessions/transport/"],
    [research.tauri, research.backend],
  ),
  "transition-services": concept(
    "current",
    "Rust transition services",
    "Long-lived lifecycle operations are implemented as restart-aware state machines, not single request handlers.",
    "The largest files are vertically complete because they coordinate many facts that must not be inferred from one another. This preserves truth, but it also concentrates policy, effects, recovery, projection, and proof in a few implementation regions.",
    [
      ["Prepare", "Persist exact intent and identity before effectful launch."],
      ["Observe", "Record launch, provider, lifecycle, and semantic facts separately."],
      ["Reconcile", "Adopt or complete partially performed stages after restart."],
      ["Project", "Expose only coherently supported state to the frontend."],
    ],
    [
      "src-tauri/src/orchestration/bootstrap_transition.rs",
      "src-tauri/src/orchestration/sprint_runner_transition.rs",
      "src-tauri/src/agent_sessions/application/",
    ],
    [research.backend, research.findings],
  ),
  "repository-effects": concept(
    "current",
    "Repositories and external effects",
    "Durable rows establish authority; filesystem, Git, MCP hosts, and processes perform effects under that authority.",
    "The backend cannot treat every successful system call as product completion. It records intent, performs or observes the effect, then reconciles the exact evidence required for the next semantic transition.",
    [
      ["Repository", "Stores canonical state and correlations in SQLite."],
      ["Git and workspaces", "Provide physical isolation, candidate identity, and accepted integration."],
      ["Runtime", "Starts Codex with an effective environment and invocation-scoped capabilities."],
      ["MCP host", "Turns bounded tool calls into semantic application requests."],
    ],
    [
      "src-tauri/src/orchestration/repository.rs",
      "src-tauri/src/orchestration/execution_support.rs",
      "src-tauri/src/runtime/codex/",
    ],
    [research.durable, research.mcp, research.workUnit],
  ),
  "executable-configuration": concept(
    "current",
    "Configuration that executes policy",
    "The effective launch contract is assembled from persisted, compiled, selected, and invocation-specific inputs.",
    "Harness text, tool schema, skill material, selected profile home, sandbox request, source checkout, environment overlays, and runtime arguments are not passive settings. Together they determine what the launched agent can understand and do.",
    [
      ["Compiled defaults", "Role catalogue and application composition supply product-owned policy."],
      ["Persisted revisions", "Harness changes become immutable invocation bindings."],
      ["Selected identity", "Native Profile and source checkout bind filesystem and repository context."],
      ["Invocation scope", "MCP bearer and capability set are minted for one managed action."],
    ],
    [
      "src-tauri/src/orchestration/conversation_harness_catalog.json",
      "src-tauri/src/orchestration/work_unit_execution_harness.rs",
      "src-tauri/src/runtime/codex/arguments.rs",
    ],
    [research.harness, research.mcp, research.nativeProfile],
  ),
  "experience-plan": concept(
    "current",
    "Planning experience",
    "The strongest experience combines conversation, visible proposal state, and an explicit initiation boundary.",
    "It gives strategic discussion somewhere durable to land. The user can inspect the current proposal and retains control over the transition from thinking to managed execution.",
    [
      ["Frontstage", "Conversation and proposal state appear together."],
      ["Feedback", "Save, revision, and initiation are visibly different states."],
      ["Backstage", "A scoped MCP tool writes trusted proposal data outside the transcript."],
    ],
    ["src/features/orchestrations/EpicPlanBuilder.tsx", "src/features/agentSessions/AgentSessionWorkspace.tsx"],
    [research.planBuilder, research.frontend],
  ),
  "experience-orient": concept(
    "current",
    "Orientation and progress",
    "Epic, Sprint, planning-point, Work Unit, and Agent Session views project one nested body of work.",
    "The product has a recognizable orientation grammar, but the depth of the lifecycle means navigation and status can become backend-shaped. Exact contextual return and calmer summaries are especially important.",
    [
      ["Frontstage", "Overview and detail workspaces expose nested product state."],
      ["Feedback", "Activity, attention, and related Sessions explain what is happening."],
      ["Backstage", "A large native query composes durable state from many subsystems."],
    ],
    ["src/features/orchestrations/OrchestrationSection.tsx", "src/features/orchestrations/components/DetailWorkspace.tsx"],
    [research.frontend, research.current],
  ),
  "experience-deliver": concept(
    "current",
    "Work Unit delivery experience",
    "The user sees one responsibility while the product coordinates several agent and evidence stages behind it.",
    "The design challenge is truthful compression: distinguish waiting, launch, work, reporting, review, retry, integration, and settlement without forcing the user to understand every invocation and database phase.",
    [
      ["Frontstage", "Work Unit detail, related Handler/Implementer Sessions, activity, and evidence."],
      ["Feedback", "Progress and attention should describe the next meaningful product state."],
      ["Backstage", "Multiple immutable Harness continuations and reconciliation loops preserve authority."],
    ],
    ["src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx", "src/features/orchestrations/components/SprintWorkspace.tsx"],
    [research.workUnit, research.frontend],
  ),
  "experience-review": concept(
    "current",
    "Review and completion experience",
    "Evidence inspection and disposition exist, but the route from accepted work to final product completion is uneven.",
    "The experience should explain what was reviewed, what was accepted, what was integrated, and what remains unsettled. Those are different product outcomes even when they occur close together.",
    [
      ["Frontstage", "File evidence, Handler disposition, retry or handback, and integration state."],
      ["Feedback", "Accepted claims, captured evidence, judgment, and settlement must not collapse into one badge."],
      ["Backstage", "Candidate pinning and Git/durable reconciliation protect the exact accepted contribution."],
    ],
    ["src/features/fileReview/", "src-tauri/src/orchestration/accepted_integration.rs"],
    [research.workUnit, research.frontend],
  ),
  "experience-seams": concept(
    "conditional",
    "Conditional experience seams",
    "Visible product value and productive reach do not always align.",
    "Three surrounding experiences deserve subordinate treatment until their audience and productive path are clearer: File Review has split reachability, Harness Management exposes more ambition than productive mutation, and substantial review tooling remains development-oriented.",
    [
      ["File Review", "A useful evidence viewer exists, but release viewing and evidence production are not one complete path."],
      ["Harness Management", "Productive inspection and recorded-development editing currently share one ambitious interface."],
      ["Review tooling", "Human and Worktree Review contain meaningful mechanics for narrower development contexts."],
    ],
    [
      "src/features/fileReview/",
      "src/features/conversationHarnesses/ConversationHarnessInspector.tsx",
      "src-tauri/src/worktree_review/",
    ],
    [research.frontend, research.code, research.harness],
  ),
  "file-review": concept(
    "conditional",
    "File Review reachability",
    "A valuable evidence viewer exists, but release-time viewing and development-time production are not one connected path.",
    "The experience is real and useful, yet its product boundary is split. That makes it easy to overstate either absence or completeness unless producer, source, navigation, and audience are considered separately.",
    [
      ["Release viewer", "Contextual evidence can be inspected from supported product surfaces."],
      ["Producer", "Some richer production paths remain development-composed."],
      ["Design implication", "The user-facing value is evidence understanding, not the implementation seam."],
    ],
    ["src/features/fileReview/", "src-tauri/src/orchestration/file_review_git_producer.rs"],
    [research.frontend, research.code],
  ),
  "harness-management": concept(
    "conditional",
    "Harness Management ambition",
    "The interface is substantially more capable than the productive mutation path currently connected to it.",
    "This is a useful design hypothesis for managing agent behavior, but productive inspection and recorded-development editing should not look like one fully available release capability.",
    [
      ["Inspection", "Product composition can expose effective Harness configuration read-only."],
      ["Management", "Editing, publishing, and queueing are connected in recorded development."],
      ["Design question", "Operator policy management may deserve a different product boundary from everyday agent use."],
    ],
    ["src/features/conversationHarnesses/ConversationHarnessInspector.tsx", "src/infrastructure/conversationHarnesses/tauriConversationHarnessInspectorSource.ts"],
    [research.frontend, research.harness],
  ),
  "internal-review": concept(
    "conditional",
    "Internal review tooling",
    "Human and Worktree Review capabilities contain meaningful review mechanics but target development and verification contexts.",
    "Their product insight is the evidence and control model they explore. Their current audience and composition should remain subordinate rather than being presented as ordinary release functionality.",
    [
      ["Human Review", "Supports evidence-led inspection of prepared review material."],
      ["Worktree Review", "Coordinates isolated review instances and runtime interaction."],
      ["Boundary", "Development composition and debug backend availability constrain reachability."],
    ],
    ["src/features/humanReviewLauncher/", "src-tauri/src/worktree_review/", "src-tauri/src/worktree_runtime/"],
    [research.frontend, research.code],
  ),
};

const views = {
  product: {
    label: "Product",
    title: "What product lifecycle has actually been built?",
    heading: "A managed progression from intent to accepted contribution",
  },
  architecture: {
    label: "Architecture",
    title: "Where does authority move, and who owns the effect?",
    heading: "A visible request narrows through durable authority before it becomes an effect",
  },
  implementation: {
    label: "Implementation",
    title: "How does “Initiate Epic” become durable execution?",
    heading: "One click crosses confirmation, authority, persistence, and launch boundaries",
  },
  experience: {
    label: "Experience",
    title: "What does the user see—and what remains backstage?",
    heading: "Visible work, product feedback, and automatic operation are different responsibilities",
  },
};

const productJourney = [
  ["shape-epic", "01", "A proposal becomes inspectable before work begins."],
  ["bootstrap", "02", "Explicit confirmation creates durable operating context."],
  ["start-sprint", "03", "A bounded Sprint receives managed ownership."],
  ["plan-work", "04", "Current planning becomes an executable graph."],
  ["deliver-work", "05", "Implementation, evidence, and review stay separate."],
  ["review-integrate", "06", "Accepted work crosses Git and durable settlement."],
];

const productHorizon = [
  ["resilience", "Strengthen", "Execution resilience", "Recover and project failures more truthfully."],
  ["governance", "Extend", "Governance and correction", "Make product decisions versioned and evidence-linked."],
  ["closure", "Complete", "Continuation and completion", "Make final Sprint and Epic outcomes explicit."],
];

const architectureLayers = [
  ["architecture-surfaces", "01", "Experience", "Product surfaces", "Orchestration · Sessions · Settings · Review"],
  ["tauri-boundary", "02", "Transport", "Tauri boundary", "Commands · DTOs · application composition"],
  ["application-authority", "03", "Application", "Rust authority", "Services · transitions · native projection"],
  ["policy-plane", "04", "Execution policy", "Capabilities and identity", "Harness · MCP · Native Profile · source"],
  ["durable-effects", "05", "State and effects", "Cross-store operating model", "SQLite · files · Git · worktrees · processes"],
];

const implementationPath = [
  ["epic-initiate-action", "01", "Initiate action", "The button requires a current proposal; opening confirmation is not confirmation."],
  ["epic-confirmation-client", "02", "Capability and confirmation client", "Describe the exact proposal and request application-owned confirmation."],
  ["epic-confirmation-transport", "03", "Tauri request and resolution", "Transport the pending request and the user’s explicit decision."],
  ["epic-confirmed-initiation", "04", "Confirmed initiation", "Revalidate authority, snapshot material, and record the durable initiation."],
  ["epic-bootstrap-reconciliation", "05", "Bootstrap reconciliation", "Prepare files and managed Sessions only after initiation is durable."],
];

const experienceStages = [
  ["experience-plan", "Plan", "Discuss and confirm"],
  ["experience-orient", "Orient", "Find state and context"],
  ["experience-deliver", "Deliver", "Follow bounded work"],
  ["experience-review", "Review", "Understand evidence and outcome"],
];

const experienceRows = [
  ["User sees", ["Conversation + proposal", "Epic, Sprint, Work Unit", "Work Unit + Agent Sessions", "Evidence + disposition"]],
  ["Product explains", ["Saved vs initiated", "Progress + attention", "Waiting, active, returned", "Reviewed, integrated, settled"]],
  ["Application does", ["Preserves proposal and confirmation", "Composes current durable state", "Coordinates bounded roles and evidence", "Pins accepted work and settles outcome"]],
];

const state = {
  view: "product",
  focus: null,
};

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function renderProduct() {
  const journey = productJourney
    .map(([id, number, copy]) => {
      const item = concepts[id];
      return `<button class="concept-button journey-step status-${item.status}" data-concept="${id}" type="button">
        <span class="step-number">${number}</span>
        <h3>${escapeHtml(item.title)}</h3>
        <small>${escapeHtml(copy)}</small>
      </button>`;
    })
    .join("");
  const horizon = productHorizon
    .map(([id, label, title, copy]) => `<button class="concept-button horizon-card status-emerging" data-concept="${id}" type="button">
      <span>${escapeHtml(label)}</span>
      <h3>${escapeHtml(title)}</h3>
      <p>${escapeHtml(copy)}</p>
    </button>`)
    .join("");
  return `<div class="product-layout">
    <div class="product-journey">${journey}</div>
    <div class="horizon-strip">
      <div class="horizon-intro">
        <span>Emerging</span>
        <strong>Product directions</strong>
      </div>
      ${horizon}
    </div>
  </div>`;
}

function renderArchitecture() {
  const layers = architectureLayers
    .map(([id, number, type, title, contents]) => `<div class="architecture-layer">
      <div class="layer-name"><span>${number} · ${escapeHtml(type)}</span><strong>${escapeHtml(title)}</strong></div>
      <div class="layer-concepts">
        <button class="concept-button layer-concept status-current" data-concept="${id}" type="button">
          <strong>${escapeHtml(contents)}</strong>
        </button>
      </div>
    </div>`)
    .join("");
  return `<div class="architecture-layout">
    <div class="architecture-stack">${layers}</div>
    <aside class="authority-reading">
      <h3>Authority narrows as effects become more concrete.</h3>
      <p>A visible request does not directly authorize a process, file, Git ref, or semantic completion.</p>
      <div class="authority-chain">
        <div><i>1</i><span><strong>User intent and gates</strong><small>Experience layer</small></span></div>
        <div><i>2</i><span><strong>Durable authority</strong><small>Application layer</small></span></div>
        <div><i>3</i><span><strong>Scoped capability</strong><small>Execution-policy layer</small></span></div>
        <div><i>4</i><span><strong>Effect and evidence</strong><small>State-and-effects layer</small></span></div>
      </div>
    </aside>
  </div>`;
}

function renderImplementation() {
  const path = implementationPath
    .map(([id, number, title, copy]) => `<button class="concept-button call-step status-current" data-concept="${id}" type="button">
      <span>${escapeHtml(number)}</span>
      <h3>${escapeHtml(title)}</h3>
      <p>${escapeHtml(copy)}</p>
    </button>`)
    .join("");
  return `<div class="implementation-layout">
    <div class="call-path">${path}</div>
    <aside class="policy-panel">
      <span>After confirmation</span>
      <h3>Execution policy joins the path.</h3>
      <p>These inputs shape later managed Sessions; they do not decide whether the user confirmed initiation.</p>
      <button class="policy-button policy-concept" data-concept="executable-configuration" type="button">
        <strong>Configuration becomes executable policy</strong>
        <span>Harness · scoped MCP · profile · environment · source</span>
      </button>
    </aside>
  </div>`;
}

function renderExperience() {
  const headers = experienceStages
    .map(([id, title, copy]) => `<button class="concept-button blueprint-stage status-current" data-concept="${id}" type="button">
      <strong>${escapeHtml(title)}</strong><small>${escapeHtml(copy)}</small>
    </button>`)
    .join("");
  const rows = experienceRows
    .map(([title, cells], rowIndex) => `<div class="blueprint-label"><strong>${escapeHtml(title)}</strong></div>${cells
      .map((copy) => `<div class="blueprint-cell ${rowIndex === 1 ? "feedback" : rowIndex === 2 ? "backstage" : ""}">${escapeHtml(copy)}</div>`)
      .join("")}`)
    .join("");
  return `<div class="experience-layout">
    <div class="blueprint">
      <div class="blueprint-corner">Journey moment</div>${headers}${rows}
    </div>
    <div class="experience-gap-strip">
      <button class="concept-button gap-summary status-conditional" data-concept="experience-seams" type="button">
        <span>3 conditional seams</span>
        <strong>Visible value and productive reach do not always align</strong>
        <small>File Review · Harness Management · internal review tooling</small>
      </button>
    </div>
  </div>`;
}

const renderers = {
  product: renderProduct,
  architecture: renderArchitecture,
  implementation: renderImplementation,
  experience: renderExperience,
};

function updateUrl(focus = state.focus) {
  const params = new URLSearchParams({ view: state.view });
  if (focus) params.set("focus", focus);
  history.replaceState(null, "", `${location.pathname}?${params.toString()}`);
}

function render() {
  const view = views[state.view];
  document.body.dataset.view = state.view;
  document.querySelector("#view-title").textContent = view.title;
  document.querySelector("#canvas-heading").textContent = view.heading;
  document.querySelector("#atlas-canvas").innerHTML = renderers[state.view]();
  document.querySelectorAll("[data-view]").forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.view === state.view));
  });
  document.querySelectorAll("[data-concept]").forEach((button) => {
    button.addEventListener("click", () => openConcept(button.dataset.concept));
  });
}

function openConcept(id, update = true) {
  const item = concepts[id];
  if (!item) return;
  state.focus = id;
  const dialog = document.querySelector("#concept-dialog");
  const status = document.querySelector("#detail-status");
  status.className = `detail-status status-${item.status}`;
  status.textContent = statusNames[item.status];
  document.querySelector("#detail-perspective").textContent = `${views[state.view].label} perspective`;
  document.querySelector("#detail-title").textContent = item.title;
  document.querySelector("#detail-insight").textContent = item.insight;
  document.querySelector("#relationship-list").innerHTML = item.relationships
    .map(([title, copy]) => `<article class="relationship-card"><strong>${escapeHtml(title)}</strong><p>${escapeHtml(copy)}</p></article>`)
    .join("");
  document.querySelector("#artifact-list").innerHTML = item.artifacts
    .map((artifact) => `<li><code title="${escapeHtml(artifact)}">${escapeHtml(artifact)}</code></li>`)
    .join("");
  document.querySelector("#evidence-list").innerHTML = item.evidence
    .map(([label, href]) => `<li><a href="${href}">${escapeHtml(label)} ↗</a></li>`)
    .join("");
  if (update) updateUrl(id);
  if (!dialog.open) dialog.showModal();
}

function closeConcept() {
  const dialog = document.querySelector("#concept-dialog");
  if (dialog.open) dialog.close();
  state.focus = null;
  updateUrl(null);
}

document.querySelectorAll("[data-view]").forEach((button) => {
  button.addEventListener("click", () => {
    if (!views[button.dataset.view] || state.view === button.dataset.view) return;
    const dialog = document.querySelector("#concept-dialog");
    if (dialog.open) dialog.close();
    state.view = button.dataset.view;
    state.focus = null;
    updateUrl(null);
    render();
  });
});

document.querySelector("#close-detail").addEventListener("click", closeConcept);
document.querySelector("#concept-dialog").addEventListener("click", (event) => {
  if (event.target === event.currentTarget) closeConcept();
});
document.querySelector("#concept-dialog").addEventListener("cancel", (event) => {
  event.preventDefault();
  closeConcept();
});

const note = document.querySelector("#atlas-note");
document.querySelector("#about-atlas").addEventListener("click", () => note.showModal());
document.querySelector("#close-note").addEventListener("click", () => note.close());
note.addEventListener("click", (event) => {
  if (event.target === note) note.close();
});

const params = new URLSearchParams(location.search);
if (views[params.get("view")]) state.view = params.get("view");
const focusAliases = {
  "agent-surface": "experience-orient",
  "agent-sessions": "experience-orient",
  "plan-builder": "shape-epic",
  "work-unit": "deliver-work",
  "git-workspace": "durable-effects",
};
const requestedFocus = focusAliases[params.get("focus")] ?? params.get("focus");
if (concepts[requestedFocus]) state.focus = requestedFocus;

render();
if (state.focus) openConcept(state.focus, false);
if (params.get("evidence") === "1" && state.focus) {
  document.querySelector(".evidence-layer").open = true;
}
if (params.get("note") === "1" && !note.open) note.showModal();
