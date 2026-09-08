# Delivery sequence

The overhaul proceeds as a functional replacement, then a UI recomposition, then legacy
retirement. Adjacent branch integration follows only after the branch works in isolation.

The sequence expresses dependency and review gates. Agents may split steps further or reorder work
that has no real dependency, while preserving the functional-before-UI and replacement-before-
retirement direction.

## 1. Canonical vocabulary

- make Runtime, Capability, Node and Session Profiles the only replacement configuration terms;
- make Session Event definition, occurrence, group and delivery explicit;
- retain typed Workflow identity translation;
- mark mixed Harness and generic Role contracts as legacy.

**Gate:** every target concept has one owning module and one obvious public route.

## 2. Functional services

- add Capability Profile repository and service;
- add Runtime and Session Profile queries;
- add Workflow authoring V2 persistence;
- add Workflow compile/validation application service;
- add Session Event ingress and query services;
- add concrete referenced-content and identity-assignment adapters.

**Gate:** replacement domains operate without a frontend.

## 3. Backend happy-flow proof

Execute the path in [functional-happy-flow.md](functional-happy-flow.md): create a Capability
Profile, compile a Workflow, create and address a Session, pin its Session Profile, deliver a later
event, inspect records and send a direct-user message with invocation-local choices.

**Gate:** focused tests prove the required invariants and one integrated happy flow.

## 4. Replace persistent product integrations

### Workflow

- replace Role, Harness and override payloads with Capability reference, embedded Node Profile,
  identity reference and event-oriented connections;
- compile at activation;
- execute compiled Session Event definitions.

### Agent Sessions

- preserve lifecycle, repository, runtime, transcript and observation behavior;
- replace Harness attachment and Session model overrides with pinned Session Profile and event
  provenance;
- support direct-user invocation-local model/reasoning.

**Gate:** active functional paths no longer require generic Role or legacy Harness configuration.

## 5. Frontend application contracts

- expose browser-safe profile, Workflow authoring, compilation, event-query and direct-invocation
  contracts;
- expose value origin, availability, inherited and locked states in read models;
- keep persistence records out of feature components.

**Gate:** every target editor and inspector has an application contract independent of its visual
composition.

## 6. UI foundation and composition

- extract reusable selectors, resolved fields, validation and editor shells;
- build Capability Profile, Node Profile and connection editors;
- build read-only Session Profile and event projections;
- add per-message runtime controls to the Session composer;
- let Workflow compose the editors rather than own their internal forms.

**Gate:** the target happy flow is operable and legible through the product UI.

## 7. Retire legacy surfaces

- remove generic Workflow Roles;
- remove mixed Harness editors, DTOs, commands and recorded sources;
- remove legacy adapters, update policies and Session-level model overrides;
- consolidate duplicated presentation components.

**Gate:** no active import or persisted product path depends on the deprecated concepts.

## 8. Reconcile adjacent development

- integrate parallel Workflow and Agent Session changes at the new application and adapter
  boundaries;
- preserve surviving behavior without restoring deprecated concepts;
- run broader functional and UI validation after reconciliation.

This sequence is directional rather than a demand for large atomic commits. Each step should remain
reviewable and may be divided into smaller changes that preserve the same gates.
