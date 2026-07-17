# Work Integration State

Possible repository outcomes after review:

- `merged-clean`: accepted work merged and validation completed with a clean target state.
- `accepted-in-place`: accepted work already lives in the target route and no merge was needed.
- `committed`: accepted work was committed during integration.
- `left-dirty-for-followup`: accepted work remains modified or untracked for later work.
- `report-only`: accepted evidence or analysis produced no repository change.
- `planner-needed`: repository state requires planning judgment.

Use integration or reconciliation for ordinary mechanics. Reserve `planner-needed` for state that affects sequencing or strategy.
