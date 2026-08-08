# Clean Plan Slice conversation boundary

## Observation and theory

Overall Plan task `019fc106-1222-7f52-a1ad-9189481658e8` needed Slice 2 to continue from Slice 1's accepted but uncommitted worktree state. It called `fork_thread` on Slice 1 task `019fc109-d697-7a70-8a6f-41585d58d9d9` with a same-directory environment, retitled the result, and sent the Slice 2 handoff. The resulting task `019fc14c-463c-7771-a9c9-9e4a387d7bb0` retained Slice 1's transcript and original handoff.

`start-plan-slice` required a separate top-level task but did not distinguish a new conversation from a forked conversation or separate repository-state continuity from conversation-history continuity. The reader therefore treated a same-directory fork as a valid way to preserve accepted state.

## Revision

Each Plan Slice now starts in a newly created top-level conversation containing its handoff and harness context, without another slice's conversation history. Forking, retitling, or repurposing an existing conversation is outside the operation.

Reusing a repository state or worktree route does not permit transcript inheritance. When accepted baseline state exists only in an existing task directory and the harness cannot create a clean conversation there, the reader reports that routing boundary before starting the slice.

## Evaluation

This keeps one conversation aligned with one Plan Slice while allowing the plan to depend on earlier accepted repository state. It does not prescribe Git integration or invent a harness feature. The current harness's inability to create a clean task in an arbitrary existing worktree remains a truthful routing limitation rather than a reason to copy the former owner's history.

A fresh read-only forward test supplied exactly that limitation. The reader kept Slice 2 pending, preserved Slice 1's state, made no creation or delivery claim, and reported the need for a clean top-level conversation with access to the accepted baseline instead of forking.
