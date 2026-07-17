# Architecture and evidence

Source worktree:
`C:\Users\user\.codex\worktrees\37c4\Codex Orchestrator`

Accepted branch/commit: `codex/explore-harness-inspector` at `f3e332a`

## Ownership flow

```text
development route ?harness-inspector
  -> recorded development composition
    -> HarnessInspectorDevelopmentSurface
      -> HarnessAwareAgentSessionPane
        -> neutral AgentSessionWorkspace
        -> ConversationHarnessInspector
             ^
             |
       ConversationHarnessInspectorSource
             ^
             |
       recorded catalog adapter
```

- `ApplicationRoot` activates the recorded composition only in Vite development mode.
- `App` receives the development surface through composition; production boot does not supply it.
- `HarnessAwareAgentSessionPane` owns the conditional control and pane replacement.
- Agent Session contracts and components do not learn about harnesses.
- `ConversationHarnessInspectorSource` is the application boundary for available/unavailable reads.
- The inspector renders scope, editability, delivery, validation, and provenance from the read model.
- The recorded adapter parses the checked-in v2 catalog and binds only to one fixture session.

## Source map

| Concern                                  | Source                                                                      |
| ---------------------------------------- | --------------------------------------------------------------------------- |
| Read contract                            | `src/application/conversationHarnesses/harnessInspector.ts`                 |
| Conditional control and pane replacement | `src/features/conversationHarnesses/HarnessAwareAgentSessionPane.tsx`       |
| Inspector presentation                   | `src/features/conversationHarnesses/ConversationHarnessInspector.tsx`       |
| Development surface                      | `src/features/conversationHarnesses/HarnessInspectorDevelopmentSurface.tsx` |
| Recorded adapter                         | `src/dev/conversationHarnesses/recordedHarnessInspectorSource.ts`           |
| Injected recorded composition            | `src/dev/orchestrationSection/recordedOrchestrationClient.ts`               |
| Development-only route                   | `src/app/ApplicationRoot.tsx`                                               |
| Exploration record                       | `docs/orchestration/agent-session-harness-inspector-exploration.md`         |

## Accepted evidence

- Initial exploration commit: `35526ef`
- Truthfulness correction: `f3e332a`
- Corrected focused tests: 3 files / 7 tests passed.
- Serial frontend aggregate: 90 files / 609 tests passed.
- TypeScript, production Vite build, touched-file ESLint, Prettier, and `git diff --check` passed.
- Headless Edge review covered 1440 × 1000 and 760 × 900.
- Final Epic review independently reran the corrected 3 files / 7 tests and production build; both
  passed, and the worktree remained clean.

The accepted correction keeps four facts distinct:

1. configured profile context;
2. durable delivery evidence;
3. editability;
4. validation status.

The recorded adapter reports delivery as `not_evidenced`; `invalid` and `unverified` validation
remain separate states.
