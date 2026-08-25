# Route Epic Feedback

## Observation and theory

A future product-owned coaching or human-loop interface needs a narrow way to return human input to a running Epic. The existing execution-role skills define downward ownership but do not define external feedback ingress, and ad-hoc task messaging is not a product authority boundary.

## Revision concept

Create a hidden product skill that routes durable feedback through an application action to the Handler, Work Slice Planner, Sprint Runner, or Epic Runner whose maintained state must change. Keep the Implementer behind its Handler and preserve an application-owned return route.

## Evaluation

The skill lives under repository-owned `product/skills`, outside Codex discovery. It describes role selection and evidence boundaries without claiming the feedback action is implemented or adding Harness integration. Its rules remain intentionally provisional until product sessions expose real feedback-routing behavior.
