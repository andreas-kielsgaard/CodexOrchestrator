# Ready Plan Slice packets

## Observation and theory

The Overall Plan contract mentioned cross-slice dependencies and likely parallel movement but did not require an operational grouping of Slices that can start now. The planner could therefore present a linear forecast and select one next Slice without considering whether other independent movements were simultaneously eligible.

The historical Epoch planning model used a dependency map, parallel lanes, gates, a first eligible packet, and explicit convergence. In the PDCI example, one shared-seam unit ran first and then unlocked two parallel lanes.

The first revision made those packets visible, but task `019fc106-1222-7f52-a1ad-9189481658e8` still treated a revised ready packet as an informational plan. It needed another user prompt to start three newly ready Slices because the maintenance result had no required launch disposition and did not preserve prior execution authorization explicitly.

## Revision

Every active or forecast Slice records hard dependencies, preferred ordering, gates, shared integration surfaces, and compatibility of ownership and work routes. The plan groups all currently eligible independent Slices into a ready Slice packet and shows parallel rationale, held work, unlocks, and convergence. A single-Slice packet names the concrete reason no parallel movement is ready.

Each maintenance pass now finishes with a launch disposition for every ready member. Existing execution authorization continues across replanning unless scope or authority changed, so the owning role can act on the packet immediately.

## Evaluation

This makes parallelism an operational planning output without treating readiness as authority. Re-evaluation after accepted results or material changes keeps the packet temporal, and the explicit disposition prevents plan presentation from becoming an unintended confirmation gate.

A fresh test formed `S1 + S4` as the current ready packet, held S2 and S3 behind accepted S1, forecast S2 and S3 as the next parallel packet, and identified S5 as their convergence point. In the follow-up tests, standing authority launched every eligible member in the same turn while an unrelated member with a pending user decision remained gated without stalling the eligible work.
