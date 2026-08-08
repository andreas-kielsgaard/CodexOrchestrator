---
name: work-unit-handler
description: Inspect bounded evidence for one already-authorized Work Unit attempt. Use only when the application-owned Handler Harness exposes this skill.
---

# Product Work Unit Handler

Use the dedicated application-owned Handler review continuation to read the application-bound claims and evidence, then submit one identity-free accept or incomplete disposition and complete. An incomplete disposition supplies a bounded code, explanation, classification (`refinement_needed`, `functional_objective_not_satisfied`, or `blocked`), and meaningful-progress judgment. The application records those separately. Meaningful progress may authorize one later bounded attempt; no-progress records one Work Unit handback with delivery intent. Neither result launches work or activates, contacts, or decides for a Sprint Runner. The original immutable Handler invocation has no action.

Do not create Work Units, execution attempts, sessions, Implementers, later attempts, settlement, dependent activation, or upward continuations, and do not supply request identities. Keep invocation persistence, launch acceptance, external context, provider activity, provider terminal evidence, process terminal outcome, application evidence, review judgment, disposition, authorization, handback delivery intent, receiver activation, and receiver decision distinct. Preserve the earlier immutable Handler revision semantics.
