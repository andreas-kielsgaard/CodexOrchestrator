---
name: evaluate-worktree-cleanup
description: Evaluate which worktrees remain relevant and recommend cleanup based on their contents, development history, and current use. Use only when explicitly requested.
disable-model-invocation: true
---

# Evaluate Worktree Cleanup

Determine what work still matters and what can be retired. This is an evaluation; do not perform cleanup unless separately requested.

Inventory the relevant worktrees, their repository and branch or detached commit, dirty and untracked contents, publication state, and storage footprint. Use this evidence to guide investigation rather than treating dirty or unmerged work as automatically worth retaining.

Identify each ambiguous checkout's purpose and associated conversations. Follow the latest substantive handling, merges, and successor worktrees. Evaluate whether its intended functionality was integrated, manually reimplemented, superseded, rejected, or abandoned. Check relevant current code where necessary: textual differences do not establish missing functionality, and a historical merge claim alone does not establish redundancy.

Explain what remains unique and whether it still has value. Distinguish product work, research, presentation sources, generated output, and runtime state. Check ongoing use and dependencies on the checkout before recommending removal. Unpublished work may be intentionally disposable; merged work may still hold useful local material.

Separate reclaiming build caches from retiring a checkout or deleting refs. State what would be lost, what remains recoverable, and the practical tradeoff. Recommend preservation actions only for an identified reason, rather than automatically checkpointing or publishing everything.

Give concise recommendations grouped by disposition. For uncertain candidates, explain their purpose, where the work continued or was resolved, what remains unique, and the consequence of cleanup. Include named conversation links. Investigate enough to offer a judgment; where uncertainty remains, identify the specific unanswered question instead of handing the user a Git inventory to interpret.
