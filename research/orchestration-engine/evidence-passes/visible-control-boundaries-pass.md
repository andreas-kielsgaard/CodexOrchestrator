# Observation pass: visible controls and unavailable product mutations

## Anchor

What actually happens when a user operates product-shaped controls already present in the orchestration workspace?

This pass started with two visible interactions: Sprint Auto-flow and document Open/Resolve/Copy path. It followed their release composition rather than assuming that a rendered control implies backend support.

## Observed paths

### Sprint Auto-flow

1. `nativeQueryProductCompositionInputV2` constructs an `agentControl` contract with empty policies, eligibility evaluations, commands, and results.
2. `productReadModelComposer.ts::composeContinuation` still produces a Sprint continuation read model, but its policy is `null`.
3. `SprintWorkspace.tsx` always renders `SprintContinuationControl`, using `false` when no policy exists and omitting the policy-update intent.
4. Product boot supplies `unsupportedProductSprintAutomaticContinuationPolicyController`.
5. Operating the switch does not change the checked state. The component reports that no durable policy store is connected.

The corresponding Epic presentation omits `ContinuationControl` entirely when there is no policy, so the two levels expose absence differently.

### Document actions

1. The native orchestration query can project durable File Review documents and artifacts into the product composition input.
2. `SprintDocumentsPanel` renders each document as an openable card plus Resolve, Open, and Copy path actions.
3. Product boot supplies `unsupportedArtifactAccessController`.
4. Each operation produces explicit unsupported feedback: artifact access is not connected to a native implementation in product mode.

The artifact controller has a detailed productive-shaped contract, including idempotency, purpose, effect references, path-leak checks, and separate observed-success handling. No Tauri adapter or Rust command implementing this port was found.

## Concrete observations

### Product composition is the decisive reachability boundary

`src/bootstrap/productApplicationComposition.ts` explicitly connects working Agent Session, managed Plan Builder, native query, confirmation, Harness inspection, contextual File Review, native profile, bootstrap, and Sprint-transition clients. In the same object it deliberately supplies unsupported controllers for:

- artifact access;
- Sprint automatic-continuation policy updates;
- Epic automatic-continuation policy updates.

The `App` and lower-level components also default to absent or unsupported implementations, so a test or development caller cannot accidentally turn a missing port into success merely by omitting a prop.

### A rendered interaction can be an honest product boundary, not an implemented action

The Auto-flow controls do not optimistically mutate their state. They ask an injected controller and display its result. The document actions likewise display structured feedback rather than treating a click as success.

This is truthful interaction behavior, but it also means the visible workspace contains affordances whose normal product outcome is “unsupported.” Reachability must therefore be evaluated past component rendering and through release composition.

### The read side and mutation side evolved separately

The native query now projects substantial durable orchestration and File Review data. It intentionally leaves `agentControl` policy and command arrays empty while populating artifact descriptions from File Review documents.

The read-model composition can therefore display information governed by contracts whose associated mutation ports have no product adapter. Data availability and operation availability are independent.

### Some unused controller code represents an earlier interaction model

`agentControlController.ts` defines Sprint and Epic `requestContinuation` controllers, including unsupported product implementations and a recorded implementation. Current searches found callers only in its tests; current visible continuation controls use the newer, narrower automatic-policy controller instead.

The continuation-control test still supplies extra `requestContinuation` functions to confirm that changing policy never requests continuation. Structurally valid TypeScript allows those additional methods, but the production component sees only `updatePolicy`.

### These boundaries predate most operational orchestration depth

The artifact and automatic-policy controllers were introduced in `2cf81be` on 2026-07-17. Their visible controls followed in `e54430e` the same day. Later commits added the operational bootstrap, Sprint Runner, Work Unit, review, integration, escalation, native-profile, and settlement machinery without connecting these ports.

The sampled Product Decisions (`82d9351`) and final-settlement (`8965191`) product compositions still inject the same unsupported controllers. No corresponding artifact-open/copy or automatic-policy Tauri commands were found on those lines.

### Recorded development does not make these operations real

`createRecordedDevelopmentApplicationComposition` also injects the unsupported artifact and automatic-policy controllers. The rich recorded read model demonstrates the controls and their states, but the development composition intentionally keeps the mutations unavailable.

## Unexpected connections

- A sophisticated application contract can be both well tested and absent from the runtime product.
- The native query has caught up with the document read model but not with the document-operation port.
- Sprint and Epic absence are presented inconsistently: a disabled-in-practice Sprint switch versus no Epic control without a policy.
- Early UI hypotheses remain visible after the backend developed a different, MCP- and reconciliation-led continuation mechanism.
- “Unsupported” here is not an exception path. It is the configured release behavior.

## Questions opened by the pass

- Are these controls retained previews of intended product authority, or should unavailable operations be represented without interactive affordances?
- Do current backend continuation mechanisms correspond to the earlier Agent Control contracts, or do they express a materially different product concept?
- Should File Review navigation replace generic artifact actions, coexist with them, or eventually implement their native port?
- Which other rendered controls resolve to unsupported release controllers despite having productive-looking contracts and tests?
- When assessing leftovers, should a strong unimplemented port be treated as reusable design work, deferred scope, or historical evidence?

The pass does not answer those questions. It establishes that presentation components, read models, application ports, and release adapters represent four distinct levels of implementation.
