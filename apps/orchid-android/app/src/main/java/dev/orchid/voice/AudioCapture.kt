package dev.orchid.voice

import android.annotation.SuppressLint
import android.content.Context
import android.media.*
import android.media.audiofx.AcousticEchoCanceler
import android.media.audiofx.NoiseSuppressor
import com.konovalov.vad.webrtc.VadWebRTC
import com.konovalov.vad.webrtc.config.*
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import android.os.SystemClock
import org.json.JSONObject
import kotlin.math.sqrt
import kotlin.concurrent.thread

class AudioCapture(context: Context, private val speechStarted: () -> Unit, private val utterance: (ByteArray) -> Unit, private val failed: (String) -> Unit, private val speechEnded: () -> Unit = {}, private val progress: (JSONObject) -> Unit = {}) {
    private val settings = ConnectionSettings(context)
    private val routing = MicrophoneRouting(context, failed)
    @Volatile private var active = false
    @Volatile var speaking = false; private set
    private var recorder: AudioRecord? = null
    private var worker: Thread? = null
    private var echo: AcousticEchoCanceler? = null
    private var noise: NoiseSuppressor? = null
    private var framesRead = 0L
    private var segmentsCaptured = 0L
    private var lastReport = 0L
    private var captureError: String? = null
    private fun report(level: Double = 0.0) {
        val device = recorder?.routedDevice
        val external = device != null && device.type != AudioDeviceInfo.TYPE_BUILTIN_MIC
        progress(JSONObject().put("recording", active).put("speechDetected", speaking)
            .put("requestedMicrophone", settings.microphone).put("routedMicrophone", device?.let { "${if (external) "External" else "Phone"}: ${it.productName}" } ?: "Waiting for microphone route")
            .put("external", external).put("silenced", recorder?.activeRecordingConfiguration?.isClientSilenced ?: false)
            .put("level", level).put("framesRead", framesRead).put("segmentsCaptured", segmentsCaptured)
            .put("error", captureError ?: JSONObject.NULL).put("updatedAt", java.time.Instant.now().toString()))
    }
    @SuppressLint("MissingPermission")
    fun start() {
        try {
            routing.start()
            val size = AudioRecord.getMinBufferSize(16000, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
            val audio = AudioRecord(MediaRecorder.AudioSource.VOICE_COMMUNICATION, 16000, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT, maxOf(size, 9600))
            require(audio.state == AudioRecord.STATE_INITIALIZED) { "Microphone unavailable" }
            recorder = audio
            routing.preferredInput?.let { require(audio.setPreferredDevice(it)) { "Unable to select phone microphone" } }
            if (AcousticEchoCanceler.isAvailable()) echo = AcousticEchoCanceler.create(audio.audioSessionId)?.apply { enabled = true }
            if (NoiseSuppressor.isAvailable()) noise = NoiseSuppressor.create(audio.audioSessionId)?.apply { enabled = true }
            active = true; audio.startRecording(); report()
            worker = thread(name = "orchid-microphone") {
                try {
                    VadWebRTC(sampleRate = SampleRate.SAMPLE_RATE_16K, frameSize = FrameSize.FRAME_SIZE_480, mode = Mode.VERY_AGGRESSIVE, silenceDurationMs = 650, speechDurationMs = 90).use { vad ->
                        val samples = ShortArray(480); val preroll = ArrayDeque<ByteArray>(); var output = ByteArrayOutputStream()
                        while (active) {
                            var offset = 0
                            while (active && offset < samples.size) { val count = audio.read(samples, offset, samples.size - offset); if (count < 0) throw IllegalStateException("Audio read failed: $count"); offset += count }
                            if (!active) break
                            framesRead += samples.size
                            val bytes = ByteBuffer.allocate(960).order(ByteOrder.LITTLE_ENDIAN).apply { samples.forEach { putShort(it) } }.array()
                            val voice = vad.isSpeech(samples)
                            if (!speaking) {
                                preroll.addLast(bytes); while (preroll.size > 12) preroll.removeFirst()
                                if (voice) { speaking = true; speechStarted(); output = ByteArrayOutputStream(); preroll.forEach { output.write(it) }; preroll.clear() }
                            } else {
                                output.write(bytes)
                                if (!voice || output.size() >= 160_000) {
                                    utterance(wav(output.toByteArray())); segmentsCaptured++; output.reset()
                                    if (!voice) { speaking = false; speechEnded() }
                                }
                            }
                            val now = SystemClock.elapsedRealtime()
                            if (now - lastReport >= 1000) {
                                lastReport = now
                                report(sqrt(samples.sumOf { it.toDouble() * it.toDouble() } / samples.size) / 32768.0)
                            }
                        }
                    }
                } catch (error: Exception) { if (active) { captureError = error.message ?: "Audio capture failed"; failed(captureError!!) } }
                finally { active = false; speaking = false; report() }
            }
        } catch (error: Exception) { captureError = error.message ?: "Microphone unavailable"; close(); throw error }
    }
    fun close() {
        try { active = false; try { recorder?.stop() } catch (_: Exception) {}; worker?.join(1500); echo?.release(); noise?.release(); recorder?.release(); recorder = null }
        finally { speaking = false; routing.close(); report() }
    }
    private fun wav(pcm: ByteArray): ByteArray = ByteBuffer.allocate(44 + pcm.size).order(ByteOrder.LITTLE_ENDIAN).apply {
        put("RIFF".toByteArray()); putInt(36 + pcm.size); put("WAVEfmt ".toByteArray()); putInt(16); putShort(1); putShort(1); putInt(16000); putInt(32000); putShort(2); putShort(16); put("data".toByteArray()); putInt(pcm.size); put(pcm)
    }.array()
}
