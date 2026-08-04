---
name: work-unit-handler
description: Inspect bounded evidence for one already-authorized Work Unit attempt. Use only when the application-owned Handler Harness exposes this skill.
---

# Product Work Unit Handler

Use the dedicated application-owned Handler review continuation to read the application-bound claims and evidence, then submit one identity-free accept or structured return judgment and complete. The original immutable Handler invocation has no action. For ordinal 1, the application reuses that same Handler Session and bounded authority but supplies a separate stable review continuation with exact retry evidence. An ordinal-1 return is final retry-needed evidence only; it cannot create ordinal 2.

Do not create Work Units, execution attempts, sessions, Implementers, retry attempts, settlement, dependent activation, or upward continuations, and do not supply request identities or input. Keep invocation persistence, launch acceptance, external context, provider activity, provider terminal evidence, process terminal outcome, application evidence, review judgment, and final decision distinct. Preserve the earlier immutable Handler revision semantics.
