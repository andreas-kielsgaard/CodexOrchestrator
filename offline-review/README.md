# Offline review evidence index

This is the retained review packet prepared for offline travel on **2026-07-17**, plus a later **2026-08-05** textual Work Unit review. The original packet used product checkpoint `f23f5fd` and four separate exploration branches; it was published at `55780da`. Those checkpoints describe the package's creation, not current main.

This index was consolidated on 2026-09-14. All existing images, static HTML, evidence JSON, and the empty-directory placeholder are retained unchanged. Current behavior belongs in [the documentation index](../docs/README.md), especially [Worktree Review](../docs/worktree-review.md), [File Review](../docs/file-review.md), [execution configuration](../docs/execution-configuration.md), and [project evolution](../docs/project-evolution.md).

Images and the static HTML can be opened without running the application. Captions below preserve the original capture reports; this consolidation did not repeat visual or live-provider validation. Current recorded development routes are documented in [development](../docs/development.md). Old private-worktree paths and removed prototype commands are not current launch instructions.

## Main application captures

These five images show the July product presentation with recorded adapters in the real application shell. Recorded proposals and transcripts are sample material, not live orchestration state.

| Asset                                                                           | Caption and evidence limit                                                                                                                                  |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Orchestration overview](main-application/assets/01-orchestration-overview.png) | Planning drafts and initiated Epic navigation at the recorded checkpoint. It does not establish that the displayed execution occurred.                      |
| [Plan an Epic](main-application/assets/02-plan-an-epic.png)                     | Conversation is the primary surface, with the structured proposed Epic beside it. The proposal is recorded, not a live submission.                          |
| [Agent Sessions](main-application/assets/03-agent-sessions.png)                 | The general Session surface, transcript, processing detail, status, and cancellation placement. No live provider interaction is established by the image.   |
| [Epic detail](main-application/assets/04-epic-detail.png)                       | Recorded Epic-to-Sprint navigation and detail structure.                                                                                                    |
| [Sprint detail](main-application/assets/05-sprint-detail.png)                   | Recorded flow, concerns, and documents. The 1440-pixel capture clips the flow-map continuation at the right; it does not establish responsive completeness. |
| [Directory placeholder](main-application/assets/.gitkeep)                       | Retained empty-directory marker; it carries no review evidence.                                                                                             |

The packet's architectural lesson was to separate general Session interaction, product orchestration policy, native runtime adapters, and UI projection. A Session finishing does not by itself establish a product semantic result. Current owners and the later Workflow direction are explained in [architecture](../docs/architecture.md) and [project evolution](../docs/project-evolution.md).

The original decision worksheet, review notes, and checklists were unfilled prompts. They are not adopted user decisions, a current roadmap, or an unresolved mandatory review program.

### Later Work Unit review

The August 5 textual evidence describes `?recorded-work-unit-review`. Activity and Evidence were peer tabs; a Lifecycle item opened its exact Handler Activity; the read-only complete turn retained its authoritative input and timing. Opening the exact Session/invocation and returning restored the selected Activity without adding a composer.

The recorded flow exercised both an available file destination and an unavailable destination that stayed in place. Its updated evidence includes an available typed test run; focused tests also cover unavailable test detail. This supersedes the older checklist's blanket expectation that test detail is unavailable.

The browser report observed no document overflow at 640 × 900 and retained a known 430-pixel top-navigation residual. These are dated recorded-route and focused-test observations. No additional screenshot in this packet is claimed to depict that later run. See [validation evidence](../docs/validation-evidence.md) for the consolidated proof record; the original text is recoverable at `e2bfc6c:offline-review/main-application/WORK-UNIT-REVIEW-EVIDENCE.md`.

## Harness Inspector exploration

| Asset                                                                                     | Caption and evidence limit                                                                                                                                                                                    |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Recorded Harness control](explorations/harness-inspector/harness-inspector-recorded.png) | The recorded Session **before** opening the inspector, showing the injected application tab and over-pane **Inspect harness** control. This image does not show the full inspector or prove pane replacement. |

The July exploration used `codex/explore-harness-inspector`, initially `35526ef` and corrected at `f3e332a`. Its recorded adapter displayed the Conversation Harness v2 Epic Plan Builder profile. Opening the inspector replaced the conversation pane; Back restored it. Reported headless interaction checks covered that flow separately from this static image.

The correction preserves four distinct facts: configured context, durable delivery evidence, editability, and validation. The adapter reported delivery as `not_evidenced`; invalid and unverified validation were separate states. A catalog path or configured skill did not prove discovery, delivery, or use. Apply was disabled, and the exploration did not implement persistence or mutation. Its proposed integration steps were candidates, not authorization. Current configuration behavior belongs in [execution configuration](../docs/execution-configuration.md).

## File Review exploration

The July exploration at `9de25c2` on `codex/explore-file-diff-viewer` supplied working-tree, staged, commit-range, generated, and application-owned **recorded** fixtures to the same viewer. These labels describe sample provenance classes, not live Git or artifact-store collection.

| Asset                                                                                                 | Caption and evidence limit                                                                                                         |
| ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| [Desktop unified review](explorations/file-diff-viewer/assets/01-files-and-diffs-desktop-unified.png) | File navigation, counts, a hunk, and expandable unchanged context in the default fixture.                                          |
| [Split diff](explorations/file-diff-viewer/assets/02-split-diff.png)                                  | The same hunk shown side by side without changing file or source.                                                                  |
| [Markdown preview](explorations/file-diff-viewer/assets/03-markdown-preview.png)                      | Rendered Markdown; raw HTML is not interpreted as an interactive control.                                                          |
| [Renamed file](explorations/file-diff-viewer/assets/04-renamed-file.png)                              | Current and previous display paths, rename badge, and paired changes.                                                              |
| [Unsupported deleted file](explorations/file-diff-viewer/assets/05-unsupported-deleted.png)           | The deleted item remains in the list while unavailable content receives an explicit unsupported state.                             |
| [Narrow desktop](explorations/file-diff-viewer/assets/06-narrow-desktop.png)                          | The original report observed no horizontal overflow in unified mode at 780 pixels. Split mode retained a wider inspection surface. |

The viewer received display-ready facts and had no edit, stage, discard, or filesystem authority. The source dropdown, empty-source loading defect, and missing stored-artifact checks described in the old wrapper were later superseded. The [current File Review guide](../docs/file-review.md) explains the scoped API, artifact validation, and the native producer's present availability limit. These images establish neither those later changes nor live-provider behavior.

## Agent testing and feedback exploration

| Asset                                                                                   | Caption and evidence limit                                                                                                                       |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| [Test mode peer tab](explorations/agent-testing-feedback/assets/test-mode-peer-tab.png) | A manually captured 1440 × 1000 image of the normal shell with Orchestration, Agent Sessions, and Test mode, using a synthetic recorded Session. |

Source: `codex/explore-agent-testing-feedback` at `8c7ccd15d09454e826af213298046e0a662be09d`, inspected 2026-07-17. The historical route was `?agent-test-mode`; current `ApplicationRoot` does not expose it. The recorded image SHA-256 is `79915D13F81D00069B2ED8B59B7C6AD22DC85C8B41F97D1F9A4E20B2DEE24B72`.

The prototype demonstrated named semantic navigation/actions, structured observation, bounded waits, in-memory annotations, and an `application_test_feedback/v1` envelope sent to a recorded `feedback_only` sink. It did not provide a production MCP/native bridge or delivery to a real Agent Session.

The image was captured manually. The prototype's screenshot request reported an unavailable pixel adapter; the image therefore proves neither in-app capture nor video/audio recording. Executable attestation, durable annotations/evidence, and worktree/process/application-state isolation were not established. The proposed Test Session Host was a candidate design, not an adopted current capability.

## Worktree Runtime exploration

The old Worktree Runtime implementation and live scripts were removed when durable Worktree Review replaced that architecture. These assets remain readable historical evidence; the operating model is now described in [Worktree Review](../docs/worktree-review.md).

| Asset                                                                                   | Caption and evidence limit                                                                                                                                                                  |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Static review page](explorations/worktree-runtime/worktree-runtime-static-review.html) | Self-contained recorded HTML, with no scripts or external assets. Open directly in a browser; it does not report current process health.                                                    |
| [Static page preview](explorations/worktree-runtime/worktree-runtime-static-review.png) | A 1280-pixel-wide rendering of the static page, not a direct native application capture.                                                                                                    |
| [Evidence snapshot](explorations/worktree-runtime/evidence-snapshot.json)               | Machine-readable July 17 branch/checkpoint, historical observations, corrections, and limits. Its `currentLocalManifest` field means current at the original inspection, not current today. |

The package checkpoint was `4e5027cc1fe48dee830ad5a3f7e61d84ef664f48`. The documented two-worktree run used code `673ddf321d230f4ff497b5603efef42208d30dc4`: both builds and focused tests passed, each process/status identity matched, stopping one left the other healthy, final teardown cleared the recorded processes, and a stale-recovery drill completed. Mutable runtime directories were separate. Shared Rust compilation was not proved.

The JSON separately retains a freshly prepared dirty manifest at `35587bb` with **no observed build, test, or launch**. It is not the source of the earlier successful run. The final correction at `4e5027c` rejected re-prepare over live or stale instances and fingerprinted nested untracked file content; the snapshot records seven passing harness tests at that checkpoint.

The exploration's indirect 1280 × 820 render and observed titled Tauri windows were accepted as bounded visual evidence; direct native-window capture was not observed because Windows was locked. Process-ownership lookup/teardown and detached launch/manifest persistence were not atomic, ports were explicit slots rather than durable leases, provider credentials were not provisioned, and capture was unsupported. Pause meant stop and explicit restart. Installer and other OS-global behavior were untested. These are limits of that experiment, not a request to rebuild its proposed instance registry.

## Provenance and interpretation

The original user task `019f48bb-85b0-7451-bf2c-5483a36a18ff` requested adjacent features (raw rollout line 9779), actual in-app demonstrations (10600), and a packet for a flight without connectivity (10737). Its delivery at line 11089 records publication at `55780da`. The Harness correction and its review are recorded at lines 10425 and 10594. These references identify original messages, not a claim to have reread the whole task.

The consolidated captions and limitations come from the 17 former Markdown wrappers under this directory, recoverable through `git show e2bfc6c:offline-review/<path>`. Their unique material is now incorporated here and in the linked current subject guides, [project evolution](../docs/project-evolution.md), and [validation evidence](../docs/validation-evidence.md).

For this packet, configured or projected means a requested setting/action; observed means a check reported during the original run; recorded means retained historical material; unsupported means the prototype lacked the capability. Exploration acceptance applies to its named scope. It does not make a proposed next slice, live provider behavior, product acceptance, or user acceptance implicit.
