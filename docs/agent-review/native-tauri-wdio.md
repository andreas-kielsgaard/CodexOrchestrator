# Native Tauri WebdriverIO proof

## Reproduce

Run from the repository root:

```powershell
npm install
npm run review:native
```

The command builds a release Tauri binary with the `native-review` Cargo feature and
`src-tauri/tauri.native-review.conf.json`, then runs one embedded-provider WebdriverIO test. The
latest evidence is retained under `test-results/native-tauri-wdio/latest/`.

The test launches the real Windows shell, checks the rendered root, invokes
`load_orchestration_native_query` through `browser.tauri.execute`, asserts the v2 contract against a
fresh isolated database, and retains a screenshot. See `manifest.json`, `assertions.json`,
`build.log`, and `wdio-run.log` in the evidence directory.

The retained checkpoint copies the two generated logs as `build.txt` and `wdio-run.txt`, plus the
latest forwarded worker log as `wdio-service.txt`, so the evidence is not hidden by the repository's
global `*.log` ignore rule.

`@wdio/tauri-service` 1.2.0 currently declares `@wdio/native-utils` 2.4.0 but imports
`installMockSyncOverride`, which that version does not export. The repository pins the compatible
2.5.0 utility through an npm override. The pre-override failure is retained under
`test-results/native-tauri-wdio/dependency-mismatch-2.4.0/`.

## Production exclusion

- `native-review` is an opt-in Cargo feature. Both Rust WDIO plugins are optional dependencies and
  are registered only under that feature.
- The normal `src-tauri/tauri.conf.json` does not enable `withGlobalTauri` or WDIO permissions.
- The normal `src/main.tsx` does not import the WDIO frontend bridge.
- The alternate Vite build is the only build that imports `src/nativeReview.ts`.
- The alternate Tauri config disables bundling and grants `wdio:default` only to the `main` window.

`wdio:default` includes script execution and log-forwarding commands, while
`wdio-webdriver:default` enables the in-process WebDriver server. Treat this build as trusted
development/test code. Do not ship it, expose its port, or run it against authenticated profiles or
production data.

## Ownership and isolation

The runner allocates one available loopback port from 4445-4495, creates fresh app data and WebView2
profile directories inside the evidence directory, and lets `@wdio/tauri-service` own application
launch and termination. After assertions and log capture, it verifies that the port closed and
removes both isolated state directories. No credential, application database, or browser profile is
retained. The runner removes `CODEX_HOME` and credential-shaped environment variables before build
and launch, and the manifest records only the number removed.

Frontend and backend log forwarding is requested. The manifest records whether the behavioral test
passed and whether both channels were observed. The retained WDIO worker log is under `wdio-output/`.
The accepted Windows run forwarded the isolated database initialization from the backend and the
native-query action from the frontend.

The frontend bridge also warned that its `defineProperty` invoke interception could not be
installed. That does not invalidate the passed `browser.tauri.execute` and real IPC assertion, but
command mocking remains unproven. The forwarded frontend channel also emitted non-fatal JSON
deserialization warnings; the manifest records their count rather than treating them as proof
failure because the real command result and all assertions passed.

## Focused validation

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml --features native-review
cargo tree --manifest-path src-tauri/Cargo.toml -e normal
cargo tree --manifest-path src-tauri/Cargo.toml -e normal --features native-review
```

The normal dependency tree must omit both `tauri-plugin-wdio` packages. The feature tree must include
both at 1.2.0.
