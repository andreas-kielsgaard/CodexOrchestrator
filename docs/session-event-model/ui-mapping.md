# UI mapping

UI implementation follows the functional happy-flow proof. The current UI may be broken while its
underlying contracts are replaced.

The mapping assigns product meaning and ownership. It does not prescribe the final visual language,
panel geometry or canonical run presentation. Reuse suggestions should be checked against the
actual component contract at implementation time.

## Existing surface to target ownership

| Existing UI                     | Target owner               | Result                                                                                   |
| ------------------------------- | -------------------------- | ---------------------------------------------------------------------------------------- |
| Harness Management page         | Split                      | Capability Profile editor, Workflow Node editor and read-only Session Profile inspector. |
| Harness name                    | Capability Profile         | User-facing reusable capability definition name.                                         |
| Machine key                     | Remove                     | Keep internal identifiers out of ordinary editing UI.                                    |
| Current Agent / Visual identity | Agent Identity             | Name or initials with selectable color and circle, square or hexagon shape.              |
| Permitted name pool             | Defer                      | No current functional foundation requires it.                                            |
| Prompt prefix                   | Node Profile               | Rename to initial prompt; use only in creation events.                                   |
| Models and reasoning            | Capability + Node Profiles | Runtime availability, capability allowance, node exposure and node defaults.             |
| Running Session model controls  | Message composer           | Per-message model/reasoning from the Session's attached runtime exposure.                |
| MCP tools/connections           | Capability + Node Profiles | Allowed versus exposed; provider-managed values are disabled and inherited.              |
| Skills                          | Capability + Node Profiles | Available, allowed and node-exposed layering.                                            |
| Sandbox                         | Capability + Node Profiles | Allowed modes and pinned selection; unsupported controls remain inherited.               |
| Approval policy                 | Runtime/Session inspector  | Read-only runtime fact until the application controls it.                                |
| Hooks                           | Session Event connection   | Trigger, prompt sources and target selection.                                            |
| Update policy                   | Remove                     | Session configuration is pinned at creation.                                             |
| Harness history                 | Separate lifecycles        | Capability revision, Workflow draft/activation and immutable Session resolution.         |
| Workflow Role/Harness selectors | Node editor                | Capability Profile selection, embedded Node Profile, identity and copy-node state.       |
| Connection runtime popup        | Session Event projection   | Event group, deliveries, targets, prompt sources and outcomes.                           |
| Session Harness editor          | Session Profile inspector  | Runtime truth only; no post-creation starting-prompt edits.                              |

## First UI compositions

### Capability Profile editor

- profile name and revision;
- Runtime Profile source summary;
- allowed models, reasoning, MCP exposure, skills and sandbox modes;
- disabled/inherited presentation for provider-owned values;
- validation and save.

### Workflow node editor

- node name;
- Agent Identity;
- Capability Profile selection;
- initial prompt;
- node-exposed capability subsets;
- pinned defaults;
- copy configuration from another node.

### Workflow connection editor

- trigger selector;
- ordered prompt-source list;
- target identity and selectors;
- missing-target/create behavior;
- compact compiled-event summary.

Up/down controls are enough for initial prompt-source ordering. Drag-and-drop is not required.

### Agent Session runtime view

- read-only Session Profile;
- event source/provenance presentation;
- per-message model/reasoning controls;
- navigation from a delivered message to its event and prompt sources.

The initial event inspector can be a simple group and delivery list. It does not require a graph or
custom projection framework.

## Reusable UI foundation

Reuse `CollapsibleSection`, Markdown rendering, Workflow drag behavior, Conversation viewport,
transcript projection and identity presentation. Extract only controls with immediate multiple
consumers:

- `CatalogSingleSelect`
- `CatalogMultiSelect`
- `ResolvedValueField`
- `ValidationSummary`
- generic editor panel/dialog shell
- catalogue loading, unavailable and inherited states

Do not build a schema-driven universal configuration editor. Domain editors share presentation
primitives but retain their own explicit contracts and lifecycle language.
