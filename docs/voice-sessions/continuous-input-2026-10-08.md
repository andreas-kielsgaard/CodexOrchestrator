# Continuous voice input

Implemented on `feature/reasoned-voice-workflow` and merged into main. Android sends consecutive PCM packets including silence. The laptop revisits a continuous rolling audio window, reconciles sentence identities and revisions, and evaluates every sentence in the recent text window with JEV. Accepted relevance is sticky. Completion dispatches during capture; separate continuation decisions attach corrections to the delivered request. Phone Settings > Input processing exposes sentence cadence, pause, context and follow-up duration. Desktop Voice prompts includes continuation instructions.

Published source: `55431a91`. The release executable was launched from the main checkout and verified through Orchid's native commands and Voice Sessions screen. SHA-256: `A02B6568C6EC4B74541C262B451DFBA101FDED9BDA51C9A5BFCC5CEF4672487C`. Final live completion/continuation verification passed; conversation and reasoning-session identities were retained. Phone defaults remain 2 sentences, 2 seconds of pause, 60 seconds of context and 30 seconds of follow-up, with the phone microphone stopped until activation.

## Verification

- Sixteen focused Rust tests passed, including packet continuity, window eviction, sentence revisions, sticky relevance, query scheduling and delivered-revision eligibility and retaining long unfinished requests beyond the query window.
- Frontend typecheck/build and the Voice Sessions UI test passed. Android debug build, lint and installed transcript/inactivity probes passed.
- A real authenticated protocol-2 audio fixture displayed its first text after 1.1–2.9 seconds, before capture ended. Later audio revised the same sentence from “pass the saw” to “pass the salt.” All three ambient sentences were classified unaddressed and no input reached reasoning.
- Real JEV completion dispatched while capture reported speaking. A second completed input was classified as a continuation and carried the preceding request identity. Summarization published “received”; no new delegated worker was created and reasoning completed without errors.
- Real phone capture stopped 30 seconds after delivery in a controlled trial, while the authenticated connection remained available for replies. The test temporarily increased query thresholds to prevent ambient speech from renewing the clock.
- The phone saved all four numeric controls. A Tasker task named Orchid was created with an explicit ASSIST activity intent; running it opened the listening screen and started unsilenced phone-microphone capture.
- Restart retained the conversation and all reasoning-stage identities. Raw audio remains in memory.
- A later ambient fixture exposed a false task-context classification for “This conversation is with Alex.” Relevance instructions now explicitly exclude speech addressed elsewhere and explanations of that conversation. The repeated fixture passed with all sentences unaddressed. Test-fixture blocks were removed from pending input; history and conversation identities were retained.

## Limits

Full-window CPU decoding has variable latency. A minute of continuous fixture speech took 18–31 seconds including model startup, depending on concurrent transcription and CPU load; a window containing brief speech and silence took about 1.3 seconds after startup. This uses repeated decoding of overlapping continuous audio, not a streaming decoder with constant latency.

An earlier uncontrolled follow-up timer trial was renewed by background speech and revised sentences. Delivered revisions now renew listening only when JEV confirms continuation; unchanged buffered continuation does not renew it. The phone clock probe verifies the 30-second default, configurable duration, continuation renewal and no automatic reactivation.

Tasker's documented Google command is “Hey Google, run Orchid in Tasker,” with US-English App Actions. Google-to-Tasker support on this Gemini setup and authenticated locked-screen invocation remain unverified. Orchid remains the default assistant. The phone's Hey Google switch is disabled until Google is selected as default. Its observed path is Gemini > profile > Settings > Talk to Gemini hands-free > Hey Google and Voice Match. Enabling Google changes watch/headset assistant routing too.

Local development evidence is under `.dev/continuous-voice/`; credentials are withheld from test output.
