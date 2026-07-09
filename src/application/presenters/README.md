# Application Presenters

Presenters contain pure output shaping that belongs above the domain layer but
below React features.

Use this folder for reusable mapping from application/domain results into stable
DTOs, summaries, status descriptions, or report-friendly structures when that
mapping is not specific to one React feature.

Presenters should not own React state, perform async work, call clients, mutate
stores, or import infrastructure adapters.
