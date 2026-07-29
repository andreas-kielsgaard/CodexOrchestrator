# Product read-model composition

`composeProductOrchestrationReadModels` is the single exported product composition path. It takes
decoded Orchestration Events, Agent Control, artifact-access contracts, and a product reference
index. It imports neither compatibility DTOs nor recorded fixtures.

The reference index is the minimum narrative/display supplement: every entry keys an identity from
the Event root. An available entry names source fact or provenance references; pending,
unavailable, and unsupported entries carry an explicit reason. It does not create facts.

Each Epic overview carries separately sourced lifecycle state, movement items, ready work, and the
currently waiting human-input action. Movement and actions include typed, validated product
navigation targets. Pending, unavailable, or unsupported sources carry a reason without inventing a
value.

The event root is the composition root. Agent Control recipients and semantic targets must exist in
it. An `orchestration_event_recorded` result must point to an Event fact, and one command cannot
record competing Event outcomes. Artifact and Document contracts must match Event identities and
provenance. Documents remain explicit user-facing references; artifacts remain separate.

Artifact and Document ownership is an explicit, source-backed Sprint association in the reference
index. Every Event artifact and Document has exactly one owner; every Document's Event and
artifact-access artifact membership must agree, and its linked artifacts have the same owner. This
prevents cross-Sprint projection without guessing from ordering or provenance.

Sprint Plan revisions are ordered from the validated linear Event chain. The terminal revision is
current and selected by default. An optional selector may choose another revision of that same Sprint Plan;
it is presentation input only and is never written back as a fact.
