# Overall Plan role skill

## Observation and theory

The three Overall Plan operations described their individual actions but did not give the owning conversation one standing contract for direction, retained state, child ownership, operation selection, the normal Plan Slice profile, or plan presentation. Task `019fc106-1222-7f52-a1ad-9189481658e8` initially returned a sequence summary and later presented a plan whose slices contained only intended movement and exit condition. The user identified that presentation as suspiciously simple.

A Plan Slice cannot select its own creation settings, so Sol with high reasoning belongs with the Overall Plan reader that creates it. The role also needs explicit plan-presentation boundaries and a consistent user-facing conclusion when it routes work.

## Revision

`run-overall-plan` now holds the conversation's persistent ownership, defaults Plan Slice creation to Sol with high reasoning, and routes establishment or revision, slice creation, and slice evaluation to the three operation skills. It presents the full plan at initial creation, on request, and as the first operation after compaction; other maintenance may present only the revision and its consequences.

Whenever it routes work to another task, its final output ends with an `Action summary` naming the action, destination, evidenced routing state, and expected return. `start-plan-slice` applies and records the selected profile.

## Evaluation

The role skill gives the reader enough context to choose an operation and configure its direct Plan Slice without introducing Slice Plan responsibilities. A read-only forward test correctly selected `start-plan-slice` after an accepted design slice. Focused maintenance avoids repeated full-plan output, while the explicit compaction operation prevents a short status reconstruction from substituting for recovery.

A corrected post-compaction test restored every required Overall Plan area and the rationale, dependencies, concerns, and exit condition for each slice.

Task `019fc106-1222-7f52-a1ad-9189481658e8` created Slice 2 by calling `fork_thread` on completed Slice 1 task `019fc109-d697-7a70-8a6f-41585d58d9d9` with the same-directory environment. The resulting Slice 2 task `019fc14c-463c-7771-a9c9-9e4a387d7bb0` retained Slice 1's transcript and original handoff. The runner now treats repository-state continuity and conversation continuity separately and requires every slice to own a newly created conversation without inherited slice history.
