# Runtime Coordinator first-slice evidence

## Worktree Review correction, 2026-10-04

The user's normal review launch exposed a missing runtime dependency. The main controller's older embedded build publisher retained only the desktop executable. The initial verification used the source CLI and did not exercise that older controller. The updated controller now invokes the selected revision's build recipe; its shipped historical fallback includes the complete module set and builds a coordinator only for revisions containing its crate.

Review seeding also copied coordinator connection files, credentials, local identity and live ready endpoints. New seeds exclude the coordinator directory and its desktop associations, for both matching-schema snapshots and compatible imports. Existing review data is not reset. Other seeded configuration remains intact.

The final updated host is retained build `0ad7f16f-6cae-4745-8e48-0d8598618b4e`, PID 32268, with data under `%LOCALAPPDATA%/CodexOrchestrator/reviews/runtime-coordinator-connection`. Its native Worktree Review flow captured the dirty source, created `build-74d2d14c-377c-4c48-8919-2f41be1990c2`, compiled it, published both executables under attempt `a2bc5459c8f12a2be2cf9f4a0392c1f9`, and launched it through `worktree_review_open_build`. No executable or missing dependency was manually inserted into an old output.

The launched copy was relaunched from that same retained output and private review data with an owned WebView debug endpoint for UI verification; normal review launch intentionally removes debugger arguments. Final PID: 27036, endpoint `127.0.0.1:9238`. Its initial coordinator directory contained no copied settings. Opening Runtime Coordinator started its sibling coordinator (PID 26568) with identity `46d712b7-a337-4bb4-bcd1-bd99c5dfd795`, distinct from the controller's local identity. Selecting External and pressing Connect to laptop Docker authenticated the real existing deployment and saved its connection without frontend token entry. The copy is left open on Add Linux device against Docker. Inspected screenshots: `.dev/coordinator/final-child-local-settings.png` and `final-child-hetzner-setup.png`; native and rendered receipts are stored alongside them.

Correction checks passed: 3 build-selection/module tests, 7 review-seeding tests, 3 coordinator bridge/Docker boundary tests, 27 affected frontend tests, 23 build-tool tests, focused lint, TypeScript, both desktop builds and whitespace checks. The usual main executable was not replaced; it still needs the controller correction incorporated before it can build this slice correctly. Real Hetzner credentials and power acceptance remain pending.

Verified on 2026-10-04 in `feature/Remote-Development`, rebased onto local main `a598b7aa`. This implements the approved existing-VM registration and coordinator-owned Hetzner credential choices.

## Automated verification

| Check | Result |
| --- | --- |
| Coordinator HTTP/lifecycle integration tests | 7 passed: discovery rejection, registration without power, explicit start/shutdown, restart receipts, duplicate suppression, uncertain submissions, stale state, concurrent observation, authentication and endpoint validation. |
| Desktop coordinator bridge | 2 passed: failed external endpoint does not start local hosting; associated active work blocks Stop. Replacing saved connection settings and retaining the token are exercised on Windows. |
| Existing device service | 8 passed, including explicit-control ownership and idle-shutdown exclusion. |
| Storage/migration selection | 41 passed. |
| Affected frontend/navigation/composition tests | 29 passed, including setup, tab draft restoration, navigation, disconnected state, and lost-acknowledgement refresh. |
| Existing app/session tests | 17 passed. |
| Build tooling | 23 passed, including retained coordinator publication. |
| Focused ESLint, TypeScript, frontend and desktop debug build, diff whitespace | Passed. Existing Rust dead-code warnings and frontend chunk-size warning remain. |

The controlled provider proves request and persistence behavior; it does not establish real Hetzner acceptance.

## Live local and Docker verification

The retained desktop build is `6cea6455-e70b-4c82-88bc-b08feb5337fc`, under the worktree's build owner `e2e81157e01ed1dd`. It includes `codex-orchestrator.exe` and its sibling `orchid-coordinator.exe`. Application identifier: `dev.orchid.runtime-coordinator-review`; isolated data: `.dev/coordinator/review-data`; final reviewed desktop PID: 24036; owned WebView endpoint: `127.0.0.1:9237`.

The Linux Compose deployment built successfully and runs as `orchid-coordinator-coordinator-1`, publishing only `127.0.0.1:41800`. Authenticated health returned contract version 1. Coordinator identity `0a251275-11c1-4190-bc34-d28d27bd388d` remained unchanged after stopping and starting the container and after image recreation. The data volume and access-token file are retained separately from the image.

Orchid connected to Docker through the real native bridge. Stopping Docker produced the disconnected view with Add disabled, without local takeover. Restarting Docker and using Retry restored Connected. Restarting the desktop retained its external connection. Saving that endpoint with a blank token retained its protected credential.

Selecting local hosting through the UI started the coordinator from the final build. Its persisted identity `e5c0f662-0a75-4ef3-937b-96d62b785ac4` survived replacement of the earlier review process. Switching back to Docker selected its separate catalogue. The final review window is open on Add Linux device against Docker.

Screenshots were inspected for the list, disconnected state, coordinator settings, and progressive setup page. Back/Forward and returning from another top-level tab restored the setup page. Machine-local evidence is in ignored `.dev/coordinator/`: `final-coordinator-list.png`, `coordinator-docker-offline-final.png`, `coordinator-docker-reconnected.png`, `final-settings-external.png`, `final-local-connected.png`, `final-device-setup.png`, and `final-ready-external-setup.png`, with corresponding rendered-state JSON.

## Remaining acceptance boundaries

No valid Hetzner Cloud token was supplied, so real discovery, registration and Start/Stop on the intended worker remain unverified. The application inspector does not type into password controls; external credentials were supplied through its supported native command. Password entry is covered by frontend tests, not native input verification. Packaged installer installation was not exercised; retained desktop sidecar publication was.

Power state is independent of agent readiness. This slice does not install the Linux host program or configure Codex/Claude accounts. The Stop guard covers associated work known to this desktop; multiple-controller authority and manual server work are outside the slice. Docker on this laptop does not provide laptop-off availability.
