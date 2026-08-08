# Plan Step role skill

## Observation and theory

The Plan Step domain has one execution operation, but the conversation still needs a standing contract for fixed ownership, correction reuse, callback behavior, and its relationship to Slice Plan judgment.

## Revision

`run-plan-step` now holds those lifecycle boundaries and directs initial work and bounded corrections through `execute-plan-step`. The operation skill retains work and result-content guidance.

## Evaluation

Separating role lifecycle from execution keeps the operation focused and gives every Plan Step conversation the same callback and scope behavior. A read-only forward test selected `execute-plan-step`, investigated only the assigned question, reported qualifications, and ended after one callback. The small overlap cost is justified by consistent role initialization.
