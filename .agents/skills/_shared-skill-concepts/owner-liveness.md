# Owner Liveness

A launch owner remains responsible until required downstream review, integration or sign-off, reporting, record update, and callback work has settled, or the work is explicitly waiting, stopped, or abandoned.

If a required continuation cannot be performed, mark the concrete waiting state and notify the next actor with the exact missing action.

A delivered notification and an activated receiving turn are separate facts. Report only the evidence returned by the harness. After one delivery attempt, end the sender's turn without polling or rereading the receiver merely to prove activation.

The actor responsible for unattended liveness may message an inactive task directly when no parent owns its continuation. For a child task, direct recovery is reserved for an evidenced technical interruption or explicitly released pause. Use a neutral resume prompt without reconstructing its work.

For other suspected child inactivity, notify the parent with the observed state and evidence suggesting unfinished responsibility. The parent decides whether and how the child continues. A static error status alone is inconclusive, and watchdog reasoning does not authorize a continuation command to the child.

When the owner's assigned lifecycle settles, its recovery authority expires. Return control to the named parent for work outside that settled lifecycle; do not reactivate a completed task to choose later work. If the parent, interruption cause, lifecycle stage, or continuation cannot be evidenced, preserve the route as `waiting-on-tool` for the next recovery check.
