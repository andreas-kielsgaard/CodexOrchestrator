# Sprint Runner Five-Role Boundary

## Observation and theory

The previous `sprint-runner` defined the current ad-hoc development procedure and directly designed and launched Work Units. It explicitly disclaimed the product role. Under the clarified hierarchy this collapses Sprint ownership, temporal planning, Handler ownership, and implementation launch.

## Revision

The skill was reformulated as the product Sprint Runner. Before start it exposes only a low-resolution concern forecast and possible execution shapes. At start it rechecks current branch reality and produces a higher-resolution forecast. During execution it requests bounded Work Slice Planners, reconciles their settled slices, manages Sprint convergence, and returns the Sprint outcome to the Epic Runner.

Each Work Slice Planner is one planning-and-settlement episode. A Sprint may request fresh planners at later temporal points.

Forward testing showed that the initial wording allowed a pre-start Runner to return only a start prerequisite, omit its forecast, and hand control back to the Epic Runner. The current output contract keeps the forecast in the Sprint context, continues from a later application-delivered start state, and reserves Epic handback for a terminal Sprint boundary.

## Evaluation

The boundary prevents forecasted Work Units from becoming product objects prematurely and prevents the Sprint Runner from creating Handlers or Implementers directly. No product harness for this role is currently claimed; the skill is ready for later harness integration.

## Validation

`quick_validate.py` passed. After one wording correction, a fresh pre-start Runner retained the inherited Work Units as predictions, returned the required low-resolution forecast and start prerequisite in Sprint context, and created no lower sessions or Epic handback.
