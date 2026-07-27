# Agent-review dependency audit

Snapshot: 2026-07-27. Audit results can change as advisories are published.

## Lockfile scope

- `package-lock.json`: 431 package entries added, zero removed, zero pre-existing versions changed.
- `src-tauri/Cargo.lock`: 17 package-version entries added, zero removed.
- `package.json`: development dependencies only; no production dependency was added.
- `Cargo.toml`: both WDIO plugins are optional and reachable only through `native-review`.

| Direct development dependency                                     | Reason retained                                                           |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `@playwright/test` 1.61.1                                         | Deterministic renderer and bounded WebView2/CDP adapters                  |
| `@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework` 9.27.1 | One executable native test with a local Mocha worker                      |
| `@wdio/tauri-service`, `@wdio/tauri-plugin` 1.2.0                 | Tauri process lifecycle, embedded provider, frontend/native bridge        |
| `get-port` 7.1.0                                                  | Select one available loopback port for the isolated native session        |
| `@wdio/native-utils` override 2.5.0                               | Service 1.2.0 imports an export absent from its declared 2.4.0 dependency |

## Current npm findings

`npm audit --omit=dev` reports one high package finding. `postcss` 8.5.16 is present at the same
version in the baseline lock; this review slice did not introduce or update it.

`npm audit` reports 28 package findings: one moderate and 27 high. Six affected packages already
exist at unchanged baseline versions (`@eslint/config-array`, `@eslint/eslintrc`, `brace-expansion`,
`eslint`, `minimatch`, and `postcss`). The other 22 are in the new WDIO dependency chain. Playwright
and the Tauri bridge package have no reported finding in this audit.

Npm reports no available top-level fix for the current WDIO CLI, local runner, or Tauri service. It
offers `@wdio/mocha-framework@7.7.3` as a semver-major downgrade, which would abandon the proven
9.27.1 stack. The baseline ESLint finding similarly offers a major upgrade. This checkpoint retains
the tested versions and reports the risk instead of applying an unrelated or compatibility-breaking
lock rewrite.

## Containment

The affected review stack is installed for development only. The native service and Rust plugins
are reachable only through the alternate `native-review` frontend, feature, Tauri config, and
capability. Normal production composition contains no Agent Review Lab, remote-debugging route,
WDIO permission, frontend bridge, or Rust plugin marker.

Treat dependency installation and native review execution as trusted-development operations. Do
not run them against authenticated profiles or production data. The launch adapters additionally
scrub `CODEX_HOME` and credential-shaped environment variables. Re-run both audit commands before
updating or publishing this checkpoint.
