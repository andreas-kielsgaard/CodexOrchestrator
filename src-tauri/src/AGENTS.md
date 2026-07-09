# Tauri Backend Organization Guidance

When adding Rust/Tauri backend code in this folder, keep conceptual ownership clear.

If the code you need to add does not clearly belong in an existing concept, folder, or file, create a new file or folder that fits the new concept instead of forcing it into a nearby place.

If the code almost fits an existing concept that is narrowly defined and still small line-wise, it is acceptable to refactor that file, folder, or concept so it cleanly includes the new responsibility. Keep the resulting concept explicit and easy to name.

Prefer files and folders named after the responsibility they own. Avoid adding logic to vague catch-all files.
