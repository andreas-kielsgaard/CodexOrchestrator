"""Local utterance transcription. JSON lines in and out; audio is not retained."""
import base64
import io
import json
import sys
import wave
import numpy as np
from faster_whisper import WhisperModel

model = WhisperModel(sys.argv[1], device="cpu", compute_type="int8", cpu_threads=4, local_files_only=True)
for line in sys.stdin:
    try:
        audio = base64.b64decode(json.loads(line)["audio"], validate=True)
        if not audio.startswith(b"RIFF") or len(audio) > 4_000_000:
            raise ValueError("Expected a WAV utterance under 4 MB")
        with wave.open(io.BytesIO(audio)) as wav:
            if (wav.getnchannels(), wav.getsampwidth(), wav.getframerate()) != (1, 2, 16000):
                raise ValueError("Expected mono 16-bit PCM at 16 kHz")
            samples = np.frombuffer(wav.readframes(wav.getnframes()), dtype=np.int16).astype(np.float32) / 32768.0
        segments, _ = model.transcribe(samples, beam_size=1, vad_filter=True)
        print(json.dumps({"text": "".join(segment.text for segment in segments).strip()}), flush=True)
    except Exception as error:
        print(json.dumps({"error": str(error)}), flush=True)
