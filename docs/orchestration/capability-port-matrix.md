# Capability and port matrix: Epic and Sprint state contracts

Status: current contract evidence. This matrix defines requirements independently of legacy task/run
code. It supplies no adapter, persistence, UI, filesystem, clipboard, provider, or runtime behavior.

| Accepted need                                | Contract boundary                                   | Classification               | Evidence and boundary                                                                                      |
| -------------------------------------------- | --------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Epic/Sprint/Plan events                      | `OrchestrationEventsV1` decoder                     | Existing capability boundary | Provider-neutral identities and Orchestration Events are decoded; read-model integration remains separate. |
| Agent Session association and prompt control | `AgentControlCommandV1` / future Agent Session port | Existing boundary to evolve  | Session identity stays provider-neutral; Orchestration participation is a separate association.            |
| Agent Control, policy, and continuation      | `AgentControlContractsV1` decoder                   | Existing boundary to evolve  | Command, eligibility, and resulting Orchestration Event remain distinct.                                   |
| Artifact and Document references             | `ArtifactAccessContractsV1` decoder                 | Focused port                 | Internal artifacts remain technical; user inspection requires an explicit Document link.                   |
| Resolve artifact for open                    | `ArtifactAccessPortV1.resolveForOpen`               | Focused port                 | A purpose-bound request may record resolution; it is not opening proof.                                    |
| Open with the system default                 | `ArtifactAccessPortV1.openWithSystemDefault`        | Later adapter                | Only an `observed_success` result proves opening. No adapter is implemented.                               |
| Copy/reveal raw path                         | `ArtifactAccessPortV1.copyPath`                     | Later adapter                | A raw path is allowed only in that explicit successful result.                                             |
| Runtime/system observation                   | Result `observedEffectReference`                    | Later adapter                | Unsupported, denied, failed, and observed success remain separate outcomes.                                |
| Recorded/product unsupported adapter         | `ArtifactAccessController`                          | Provisional                  | Opening stays unsupported and non-executing until a focused native port is supplied.                       |
| Presentation/product integration             | Application clients and controllers                 | Sprint 1 boundary            | Product and recorded adapters share the component tree; persistence and runtime remain deferred.           |

## Legacy quarantine

`src-tauri/src/lib.rs` is quarantined evidence. No current Orchestration capability requirement may
adopt, call, expand, or couple to its legacy task/run implementation. Any later adapter must satisfy
the focused contracts independently.

## Validation boundary

Decoder and projection tests can establish reference integrity, leakage rejection, operation/outcome
separation, composition coherence, and the copy-path exception. They cannot prove persistence,
filesystem resolution, system opening, clipboard use, provider behavior, prompt delivery, transition
execution, or live UI behavior.
