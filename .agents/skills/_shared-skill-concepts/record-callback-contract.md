# Record Callback Contract

When an actor requests record maintenance, include the return route and the compact callback it expects. The requester waits for that callback rather than polling records.

The callback should state update status, records updated or skipped, sourced open items, whether the requesting batch can close, and any already-requested human or tool action.
