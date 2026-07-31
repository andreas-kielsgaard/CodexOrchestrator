# Work Unit Implementer Role Creation

## Trigger and gap

The clarified hierarchy names the actual implementation agent separately from the Work Unit Handler. Existing `work-unit` and `orchestration-worker` implement ad-hoc workflows and report to different owners.

## Reader and revision

The new skill addresses one Implementer created by a Work Unit Handler. It completes the fixed implementation scope, chooses proportionate validation, preserves unrelated work, and returns evidence to the Handler. It neither accepts nor integrates its own result.

## Evaluation

The narrow boundary should prevent planning and review authority from leaking into implementation. Specialized profiles for recurring UI, database, frontend, or similar Work Units remain deliberately deferred. The skill does not claim a current product harness.

## Validation

`quick_validate.py` passed. A fresh Implementer treated later convergence validation as compatible with full local delivery, rejected a request to replace implementation with advice, and returned the scope change to its Handler without claiming completion.
