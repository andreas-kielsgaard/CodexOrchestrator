# Plan Step partial disposition

## Observation and theory

The execution operation originally gave no usable return shape when a work activation ended after meaningful progress with scoped work remaining. The initial partial-return revision was insufficient: HI-01 later ended while claiming a compile was still running and left dirty work without a callback. The reader needed an operational boundary for started commands as well as a result shape.

## Revision

Execution continues while safe scoped work remains actionable and settles commands or checks started in the activation. A partial return occurs only at a meaningful retained checkpoint or evidence boundary and includes retained state, exact remaining work, and the next execution entry point.

## Evaluation

The partial route preserves progress without pretending completion or forcing ordinary remaining work into a blocker. Settling started operations prevents a status update from masquerading as a return. The Slice receives enough information to reactivate the same owner deliberately.
