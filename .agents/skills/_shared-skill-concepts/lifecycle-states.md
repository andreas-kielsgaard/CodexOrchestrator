# Lifecycle States

Useful workflow states include:

- `ready`
- `launched`
- `waiting-on-worker`
- `waiting-on-review`
- `waiting-on-record`
- `waiting-on-intake`
- `waiting-on-human`
- `waiting-on-tool`
- `settled`
- `paused`

Use explicit states. When human input is required, ask for the concrete decision directly.

A workflow waiting state does not keep an agent turn active. After the responsible actor has sent its required notification, it ends the turn and is resumed by a callback or new message when action returns to it.
