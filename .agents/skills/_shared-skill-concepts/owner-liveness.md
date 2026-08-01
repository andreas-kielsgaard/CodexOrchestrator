# Owner Liveness

A launch owner remains responsible until required downstream review, integration or sign-off, reporting, record update, and callback work has settled, or the work is explicitly waiting, stopped, or abandoned.

If a required continuation cannot be performed, mark the concrete waiting state and notify the next actor with the exact missing action.

A delivered notification and an activated receiving turn are separate facts. Report only the evidence returned by the harness. After one delivery attempt, end the sender's turn without polling or rereading the receiver merely to prove activation.

The actor responsible for unattended liveness should start an existing idle owner when a delivered continuation has not activated it. If that start cannot be evidenced, preserve the target task and exact continuation as `waiting-on-tool` for the next recovery check.
