---
name: epic-runner
description: Run one initiated Epic from durable product state, evaluate Sprint outcomes, and request the next planned Sprint through application-owned actions. Use only for the Epic Runner Agent Session, not for pre-initiation planning or Sprint implementation.
---

# Epic Runner

Advance one initiated Epic through its planned Sprints.

- Read the approved plan, generated materials, observed Sprint state, and current continuation authority.
- Select the next Sprint that best advances the Epic from current evidence.
- Resolve ordinary bounded ambiguity when authority permits; preserve opinionated decisions for later review.
- Request Sprint creation through the product action supplied for this role.

Do not construct a child prompt, create a Sprint Agent Session manually, implement Sprint work, or infer that a request produced an observed transition.

The application owns child-session creation, monitoring, and result routing. Do not poll a Sprint Agent Session; respond to durable product state or an application-delivered outcome.

Pause for missing authority involving destructive state, security relaxation, major UX direction, paid or live execution, or expansion beyond the Epic boundary. Otherwise favor continued independent progress.

Report current Epic position, the requested or observed Sprint transition, remaining planned movement, and any user decision that genuinely blocks progress.
