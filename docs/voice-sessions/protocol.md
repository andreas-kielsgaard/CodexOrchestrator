# Phone protocol v1

Connect to `ws://<laptop Tailscale IPv4>:4818/phone` with `Authorization: Bearer <token>`. The server binds to that exact Tailscale address. Loopback is permitted for desktop tests; wildcard, LAN, and public bindings are rejected. Tailscale supplies encrypted device transport. One phone connection is accepted at a time.

All WebSocket data messages are JSON text. Native WebSocket ping/pong maintains transport liveness.

| Direction | Type | Fields |
| --- | --- | --- |
| Laptop to phone | `hello` | `version: 1`, `conversationId`, `mainSessionId` |
| Phone to laptop | `audio` | unique `id`, `audio` containing base64 WAV: mono signed 16-bit PCM, little endian, 16 kHz |
| Phone to laptop | `text` | unique `id`, `text` |
| Laptop to phone | `transcript` | `id`, completed `text` |
| Laptop to phone | `speak` | `utterance: {id,kind,text,topicId,createdAt}` |
| Phone to laptop | `playback` or `interrupt` | `playback: {utteranceId,playedMs,state}` |
| Laptop to phone | `error` | `message` |
| Laptop to phone | `ended` | no additional fields |

Publication kinds are `acknowledgement`, `announcement`, and `explanation`. The agent selects the kind and material. The phone synthesizes only deliberately published text. Ordinary agent output is not played automatically.

Playback reports distinguish playing, completed, interrupted, failed, and unavailable output. `playedMs` is elapsed playback time reported by the phone; it is approximate. Publication and network send are not proof of listening.

The phone keeps a bounded audio utterance and a short pre-roll for voice activity detection. Speech start interrupts current playback and clears queued speech locally. Completed utterances reach a serial transcription queue; playback receipts remain responsive while transcription runs. Speech captured while disconnected is discarded, and reconnect does not replay output.

Conversation state, handoff relationships, transcripts, published text, pending topics, and playback context persist on the laptop. Raw audio is held in memory during transcription and is not retained by the feature.
