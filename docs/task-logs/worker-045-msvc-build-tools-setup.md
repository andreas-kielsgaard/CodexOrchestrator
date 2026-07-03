# Worker 045 - MSVC Build Tools / link.exe Setup

Date: 2026-07-03

Thread: `019f28ba-be64-7b11-92c1-b7f4bf3d564a`

## Summary

Worker 045 confirmed that Visual Studio Build Tools 2022 were already installed at
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`.

`link.exe` is available through the Visual Studio developer environment at:

`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`

The default shell still does not resolve `link.exe`; Tauri/Rust commands that need native linking
should run after loading `vcvars64.bat`.

## Verification

- Plain shell `Get-Command link.exe`: not found.
- After `vcvars64.bat`, `where link` resolves to the MSVC linker.
- `rustup`, `rustc`, and `cargo` work with `C:\Users\user\.cargo\bin` on `PATH`.
- `cargo metadata --format-version 1 --no-deps`: passed.
- `cargo fmt --check`: passed.
- `cargo test`: progressed past linker discovery and initially exposed a missing Tauri icon asset.

## Orchestrator Follow-Up

The orchestration thread added `src-tauri/icons/icon.ico`, configured it in
`src-tauri/tauri.conf.json`, and fixed Rust compile/test issues exposed by the now-working native
toolchain.

Final verification from the orchestration thread:

- `cargo fmt --check`: passed.
- `cargo test`: passed, 4 Rust tests.
- `npm run build:tauri`: passed through the Visual Studio developer environment and produced MSI and
  NSIS bundles under `src-tauri/target/release/bundle/`.

## Remaining Notes

No reboot or elevation was indicated. Future ad hoc Rust/Tauri native-build commands should be run
through:

`cmd /c "call ""C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"" && <command>"`
