# Phone connection repair, 2026-10-12

The desktop retained yesterday's phone-connected flag while the phone retried. A real S21 request failed with HTTP 409 despite a reachable Tailscale gateway. Google remained the default assistant for Hey Google; Orchid's enabled handoff service was unbound.

Changes:
- OrchidConnection keeps the existing five-second connect and ten-second hello deadlines, caps automatic retries at three, and exposes the failure reason. Start listening or assistant activation starts a fresh budget after exhaustion. Credential rejection stops immediately.
- Gateway socket ownership is held by PhoneConnections. The newest authenticated, upgraded socket replaces the old socket; only the current owner clears phone state. An unsuccessful upgrade never reserves the phone connection.
- Empty WebSocket close frames receive a normal close reply.
- Retry tests inject local requests without altering saved endpoint/token/device preferences.

Verification:
- Rust socket-ownership test passed; desktop release build passed and was launched as main PID 6836.
- Installed Android build passed actual handshake tests: initial attempt plus three retries, no subsequent attempt, manual restart, immediate 401 rejection, and four missing-hello timeouts.
- S21 live takeover probe passed against the rebuilt desktop: replacement hello, previous socket closed, unauthenticated request rejected, replacement ping and connected state preserved.
- Android build and lint passed (zero errors).
- Restored the previously enabled Google handoff after phone instrumentation, which can leave accessibility services crashed/unbound/disabled. The service is bound with no crashed service; Android system assistant activation reached Orchid's AssistantActivity and connected to the rebuilt laptop. Physical watch/headset buttons were not pressed by these tests.
- Google remains the default; automatic Google handoff remains enabled. Saved microphone and playback preferences are preserved.

Evidence: ignored .dev/connection-retry-{phone-results.txt,rust-tests.log,android-lint.log,desktop-build.log}, .dev/connection-takeover-phone-results.txt, .dev/orchid-connection-fixed.xml.

Desktop SHA256: 20256780D01AC68DF6A857586D5C1C313360C59440E7B53D8FE7FAD83D576784.
Android APK SHA256: BCAF837AC1A828536E6B0D392DA02113180A450FE2431F198E22250B8A15CC76.
