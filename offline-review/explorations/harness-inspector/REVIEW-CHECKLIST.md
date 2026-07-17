# Harness Inspector product-owner checklist

Date reviewed:

Reviewer:

## Interaction

- [ ] The **Inspect harness** control is discoverable and does not crowd Agent Session actions.
- [ ] Replacing the pane feels appropriately focused.
- [ ] **Back to conversation** is easy to find and preserves orientation.
- [ ] The inspector remains usable at a narrow window size.
- [ ] A single scrolling view is preferable to tabs for this information density.

Notes:

## Truth and terminology

- [ ] Configured context is not presented as proof of delivery.
- [ ] **Delivery not evidenced** is understandable without implementation knowledge.
- [ ] Read-only and unsupported states are clear.
- [ ] Invalid validation is meaningfully distinct from unverified validation.
- [ ] **Profile configuration**, **Future invocation**, and **Application owned** are useful scopes.
- [ ] Provenance explains where the displayed facts came from.

Notes:

## Future behavior

- [ ] Keep prompt/context expanded by default.
- [ ] Consider a collapsed prompt/context summary.
- [ ] Keep all fields read-only in the first product slice.
- [ ] Revisit editing only after durable reads and authority rules are proven.
- [ ] Require complete-profile validation, stale-revision rejection, and a new version for future
      invocations before any apply command.
- [ ] Record configuration provenance separately from activation/delivery evidence.

Fields that should eventually be editable, if any:

Fields that must remain product-owned or unsupported:

## Decision

- [ ] Accept the interaction direction for read-only product integration.
- [ ] Request a bounded revision before planning integration.
- [ ] Defer the concept.

Required changes or rationale:

The proposed read-only Epic Plan Builder query is a candidate for later planning only. Checking this
list does not authorize implementation, merge, or push.
