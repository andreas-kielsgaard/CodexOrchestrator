# Internal subagent routing

## Observation

Plan Slice `019fdb79-f997-7f90-82d4-b88d19f9b101` launched its S3 Plan Step as collaboration child `/root/s3_controlled_live_proof`, then ended its turn. The Slice could not be independently addressed while the child worked, and child completion did not itself evidence a new Slice evaluation turn. The Overall Plan watchdog repeatedly supplied receiver activation elsewhere in the same route.

## Theory

The Slice bypassed `start-plan-steps` after a long, repeatedly resumed execution history. This was not an absent definition: the current operation skill already requires a separate top-level host task and states that a collaboration subagent is not a Plan Step conversation. `run-plan-slice` also assigns all Plan Step instantiation to that operation skill.

Using an internal child collapsed execution and callback ownership into one transient agent tree. The flow then depended on host activation behavior that the ad-hoc contract explicitly does not infer from callback delivery.

## Correction

The existing Slice was reactivated with a bounded instruction to re-ingest `run-plan-slice` and `start-plan-steps`, inspect the existing S3 child once without duplicating it, disposition its result, and use separate top-level Codex tasks for every later Plan Step.

No skill wording change was made. Repeating the already-explicit rule would add weight without clarifying the reader's action. Future recurrence after confirmed re-ingestion would justify investigating launch-tool availability or a narrower placement change rather than another duplicate rule.

## Expected effect

The direct correction restores the current owner route without replacing active work. Top-level Plan Step tasks preserve independent addressability and a durable callback route; parent evaluation still begins only when receiver activation is evidenced.

## Observed result

The reactivated Slice found that S3 had already returned, evaluated its truthful expired-request residual, published checkpoint `d8d88ffa7a55881d76020d430454f49c95e0d459`, and returned it. The Overall Plan activated and authorized one fresh bounded proof. The Slice then created top-level Plan Step task `019fdc74-fcc0-7480-b0fa-7987008ff77c`; both delivery and activation were evidenced.
