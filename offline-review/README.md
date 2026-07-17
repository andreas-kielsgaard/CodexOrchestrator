# Codex Orchestrator offline review

This package is for reviewing the current product direction without internet access or live Agent
Sessions.

Start with:

1. [Main application review](main-application/README.md)
2. [Offline UI walkthrough](main-application/UI-WALKTHROUGH.md)
3. [Architecture review](main-application/ARCHITECTURE-REVIEW.md)
4. [Decision worksheet](main-application/DECISION-WORKSHEET.md)
5. [Review notes](main-application/REVIEW-NOTES.md)

Exploration packages:

- [Harness Inspector](explorations/harness-inspector/README.md)
- [Agent-native application testing and feedback](explorations/agent-testing-feedback/README.md)
- [File and diff viewer](explorations/file-diff-viewer/README.md)
- [Worktree-aware application runtime](explorations/worktree-runtime/README.md)

## What is shared

The accepted product baseline is `main` at `f23f5fd`. It is already pushed to `origin/main`.
Explorations remain on separate local `codex/explore-*` branches and are not included in that
baseline.

## Suggested review order

- **15 minutes:** read the main application summary and current happy-flow map.
- **20-30 minutes:** inspect the recorded UI using the walkthrough or packaged screenshots.
- **20 minutes:** read the architectural boundaries and known liabilities.
- **10-20 minutes:** record opinions in the decision worksheet.
- **Optional:** review each adjacent exploration package.

The package distinguishes implemented, deterministically proven, manually observed, recorded, and
still-unproven behavior. Do not infer live agent behavior from recorded demonstrations.
