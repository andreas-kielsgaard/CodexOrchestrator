# Orchid Concepts

Offline Android viewer for desktop layout sketches and prototype screenshots in `docs/voice-sessions/visual-exploration`. This is a separate application (`dev.orchid.concepts`); it has no voice, assistant, network or runtime functionality. Screens 1–3 are concept mockups; 4–6 show the working desktop prototype.

`MainActivity` owns image navigation, native zoom/pan, Fit and the Wide/Portrait switch. Gradle stages the original images as generated assets; edit the source images rather than maintaining another copy here. Captions identify known errors in the sketches.

From the repository root:

```powershell
./scripts/android/build.ps1 -ConceptViewer
./scripts/android/build.ps1 -ConceptViewer -ConnectionProbe
./scripts/android/build.ps1 -ConceptViewer -Lint
```

Install `build/outputs/apk/debug/concept-viewer-debug.apk`. Install the Android test APK to run `adb -s <transport> shell am instrument -w dev.orchid.concepts.test/dev.orchid.concepts.ViewerProbe`. The probe checks navigation, rendered content and zoom/reset on the device; screenshots verify that the bundled images are visible.
