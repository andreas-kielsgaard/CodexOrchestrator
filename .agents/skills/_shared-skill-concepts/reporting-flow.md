# Reporting Flow

Prefer proactive owner notification over polling.

A normal slice flows through worker result, review, integration or sign-off, reporting, and notification to the actor responsible for subsequent work. Completion is not merely the worker's final response.

If an actor cannot notify the required destination, emit an explicit owner-notification payload rather than leaving the continuation implicit.
