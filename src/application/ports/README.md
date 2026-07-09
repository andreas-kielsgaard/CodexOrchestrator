# Application Ports

Ports define technical capabilities that application use cases need but do not
implement themselves. Infrastructure adapters implement these contracts.

Use this folder when application logic needs an external capability such as Git,
filesystem, runtime process control, Tauri commands, or another side-effecting
system.

Rules:

- Keep ports independent of React and feature controllers.
- Return application/domain-friendly data instead of leaking adapter internals
  where practical.
- Put concrete adapter code in `src/infrastructure`, not here.
- If a port is only a UI consumption boundary over existing use cases, it may
  belong in `src/capabilities` instead.
