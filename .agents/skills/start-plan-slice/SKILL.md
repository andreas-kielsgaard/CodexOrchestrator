---
name: start-plan-slice
description: Start one separately addressable Plan Slice conversation for an ad-hoc Codex Initiative. Use from the Overall Plan conversation after the next bounded slice is selected and task creation is authorized.
---

# Start Plan Slice

Create one new Plan Slice conversation for one bounded movement in the Overall Plan. Begin it with its handoff and harness context, without another Plan Slice's conversation history.

## Prepare the handoff

Supply the slice objective, why it is next, current evidence, accepted decisions, constraints, authority, completion or re-evaluation condition, relevant sources, repository or worktree route, and the Overall Plan conversation's callback id.

Express prior accepted work as the present-tense baseline facts and evidence this Slice needs. Omit earlier Plan Slice identity and history. The new conversation owns only its stated movement; route unfinished earlier work to its existing owner.

Make the handoff self-contained. Supply authoritative artifact locations rather than directing the new conversation to read another conversation. Use conversation ids only for ownership and callback routing.

State only the broad validation boundary the Slice outcome must cover. Leave commands, tests, checks, and method to the receiving skill. A later integration check does not defer the slice's implementation, deliverables, or local acceptance.

Keep the handoff to task-specific content. State the callback id as an address only. Invoke `run-plan-slice` for planning, Plan Step handling, profile selection, validation method, evaluation, reporting, callback action, waiting, tool use, and similar role mechanics. Omit those instructions from the brief.

Translate relevant parent reasoning into a few neutral clues about concerns, evidence, or surfaces to inspect. Omit unaccepted solution candidates and command or check suggestions rather than presenting them as clues. Reserve exact instructions for genuine hard boundaries in the assigned task. Leave solution discovery and ordinary judgment to the Plan Slice.

State task facts directly. Silently omit parent-only reasoning instead of explaining what was withheld, what the child may choose, or how it should perform or return its role.

## Start the conversation

When this Slice can create substantial local build, dependency, runtime, or validation state, confirm that available storage provides meaningful headroom for the route before creating it. Return a storage gate rather than launching into plainly insufficient capacity.

Use the host harness to create a new top-level task with the model and reasoning profile selected by the Overall Plan role. Apply both the selected model and reasoning level in the creation call and compare the host-evidenced profile with the intended profile before delivering the handoff. A missing or mismatched field is a routing boundary rather than an implicit default.

Use the catalogue that supplied this operation as the instruction source: resolve its sibling `run-plan-slice/SKILL.md`, verify that file is readable, and include its absolute path in the task prompt. Keep that instruction source separate from the repository or worktree change route. Host name resolution or a skill copy inside the child's worktree does not establish catalogue identity. A collaboration subagent is not the Plan Slice conversation.

Begin the task prompt with this routing header, populated with absolute paths and the actual callback id:

```text
AD-HOC TASK ROUTING
Role skill: <absolute run-plan-slice/SKILL.md path>
Instruction catalogue: <absolute catalogue path>
Repository/worktree: <absolute task route>
Callback task: <Overall Plan task id>
```

Immediately after the header, tell the receiving conversation to read the exact role skill before acting, retain these routing values as distinct state, resolve operation skills only from that catalogue, and re-read the exact role skill after context compaction. These are prompt-carried routing facts; claim only the delivery evidence the host exposes.

Create the task rather than forking, retitling, or repurposing an existing conversation. Reusing a repository state or worktree route does not justify inheriting another slice's transcript. If the required baseline is available only in an existing task directory and the harness cannot create a clean conversation there, report the routing boundary before starting the slice.

Confirm that no active conversation already owns the same slice. Record the task id and applied profile evidenced by the harness, and send the complete handoff once. After successful delivery, let the new conversation plan and coordinate the slice; do not poll or repeatedly ingest routine progress.

If the current instruction source, task creation, profile application, or message delivery is not evidenced, report that boundary without claiming it occurred.
