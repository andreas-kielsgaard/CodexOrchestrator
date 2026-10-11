package dev.orchid.voice

import android.content.Context
import android.media.*
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import org.json.JSONObject
import java.io.File
import java.io.ByteArrayOutputStream
import java.util.Locale

/** TTS supplies PCM; AudioTrack owns playback and its independent device route. */
class SpeechPlayer(private val context: Context, private val report: (JSONObject) -> Unit, private val status: (String) -> Unit, private val idle: () -> Unit) {
    private val handler = Handler(Looper.getMainLooper())
    @Volatile private var ready = false
    val isReady get() = ready
    private val audio = context.getSystemService(AudioManager::class.java)
    private val inventory = AudioDevices(context)
    private val settings = ConnectionSettings(context)
    private val focus = SpeechAudioFocus(context) { interrupt() }
    private var current: String? = null
    private var started = 0L
    private var playbackError: String? = null
    private var media: AudioTrack? = null
    private val pcm = ByteArrayOutputStream()
    private var sampleRate = 16000
    private var encoding = AudioFormat.ENCODING_PCM_16BIT
    private var channels = 1
    private var file: File? = null
    private val queue = ArrayDeque<JSONObject>()
    @Volatile private var epoch = 0L
    private var tts: TextToSpeech? = null
    private val devices = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(added: Array<AudioDeviceInfo>) { applyOutput() }
        override fun onAudioDevicesRemoved(removed: Array<AudioDeviceInfo>) { applyOutput() }
    }
    init {
        audio.registerAudioDeviceCallback(devices, handler)
        tts = TextToSpeech(context) { result -> handler.post {
            val engine = tts ?: return@post
            if (result != TextToSpeech.SUCCESS) { status("Phone speech engine unavailable"); return@post }
            val voice = engine.voices?.filter { !it.isNetworkConnectionRequired && it.locale.language == "en" && !it.features.contains(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED) }?.sortedByDescending { it.quality }?.firstOrNull()
            if (voice == null) { status("Install an offline English voice in Android text-to-speech settings"); return@post }
            engine.language = Locale.US; engine.voice = voice
            engine.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                override fun onStart(id: String) {}
                override fun onBeginSynthesis(id: String, rate: Int, format: Int, channelCount: Int) { handler.post {
                    if (current == id) { sampleRate = rate; encoding = format; channels = channelCount }
                } }
                override fun onAudioAvailable(id: String, audio: ByteArray) { val bytes = audio.copyOf(); handler.post {
                    if (current == id) pcm.write(bytes)
                } }
                override fun onDone(id: String) { handler.post { if (current == id) play(id) } }
                @Deprecated("Android callback") override fun onError(id: String) { handler.post { if (current == id) finish(id, "failed") } }
            }); ready = true
        } }
    }
    fun speak(utterance: JSONObject) { val generation = epoch; handler.post {
        if (generation != epoch) return@post
        val id = utterance.getString("id")
        if (!ready) { progress(id, "unavailable"); return@post }
        queue.addLast(utterance); next()
    } }
    private fun muted() = audio.isStreamMute(AudioManager.STREAM_MUSIC) || audio.getStreamVolume(AudioManager.STREAM_MUSIC) == 0
    private fun next() {
        if (current != null) return
        if (queue.isEmpty()) { focus.release(); idle(); return }
        val utterance = queue.removeFirst(); val id = utterance.getString("id")
        if (muted()) { progress(id, "muted"); status("Media volume is muted"); next(); return }
        current = id; started = 0; playbackError = null; status("Preparing speech")
        pcm.reset()
        try {
            file = File.createTempFile("orchid-speech-", ".wav", context.cacheDir)
            if (tts?.synthesizeToFile(utterance.getString("text"), null, file!!, id) == TextToSpeech.ERROR) finish(id, "failed")
        } catch (error: Exception) { playbackError = error.message; finish(id, "failed") }
    }
    private fun play(id: String) {
        try {
            val bytes = pcm.toByteArray()
            check(bytes.isNotEmpty() && channels in 1..2) { "Speech engine returned no supported audio" }
            val bytesPerSample = when (encoding) { AudioFormat.ENCODING_PCM_8BIT -> 1; AudioFormat.ENCODING_PCM_16BIT -> 2; AudioFormat.ENCODING_PCM_FLOAT -> 4; else -> error("Unsupported speech format") }
            val player = AudioTrack.Builder().setAudioAttributes(playbackAttributes())
                .setAudioFormat(AudioFormat.Builder().setSampleRate(sampleRate).setEncoding(encoding)
                    .setChannelMask(if (channels == 1) AudioFormat.CHANNEL_OUT_MONO else AudioFormat.CHANNEL_OUT_STEREO).build())
                .setTransferMode(AudioTrack.MODE_STATIC).setBufferSizeInBytes(bytes.size)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY).build()
            media = player
            applyOutput()
            player.addOnRoutingChangedListener({ if (current == id && started != 0L) outputStatus("playing") }, handler)
            player.setPlaybackPositionUpdateListener(object : AudioTrack.OnPlaybackPositionUpdateListener {
                override fun onPeriodicNotification(track: AudioTrack) {}
                override fun onMarkerReached(track: AudioTrack) { if (current == id) finish(id, "completed") }
            }, handler)
            player.notificationMarkerPosition = bytes.size / (bytesPerSample * channels)
            check(player.write(bytes, 0, bytes.size) == bytes.size) { "Unable to load speech audio" }
            check(player.state == AudioTrack.STATE_INITIALIZED) { "Speech playback unavailable" }
            if (muted()) { finish(id, "muted"); return }
            if (!focus.acquire()) { status("Speech unavailable while another app holds audio focus"); finish(id, "unavailable"); return }
            player.play(); started = SystemClock.elapsedRealtime(); status("Speaking · you can interrupt"); progress(id, "playing")
        } catch (error: Exception) { playbackError = error.message; finish(id, "failed") }
    }
    fun refreshRouting() { handler.post { applyOutput() } }
    private fun applyOutput() {
        if (media?.setPreferredDevice(inventory.output(settings.audioOutput)) == false) status("Unable to select audio output")
        outputStatus(if (started == 0L) "idle" else "playing")
    }
    private fun outputStatus(state: String) {
        val requested = settings.audioOutput
        val routed = media?.routedDevice
        val previous = runCatching { JSONObject(settings.playbackStatus) }.getOrDefault(JSONObject())
        settings.playbackStatus = JSONObject().put("state", state).put("requestedOutput", AudioDevices.label(requested))
            .put("error", playbackError ?: JSONObject.NULL)
            .put("routeActive", routed != null && started != 0L && state == "playing")
            .put("preferredDeviceKey", media?.preferredDevice?.let { AudioDevices.key(it) } ?: JSONObject.NULL)
            .put("routedOutput", routed?.let { AudioDevices.label(it) } ?: previous.optString("routedOutput", "No active playback"))
            .put("outputUnavailable", requested != "automatic" && inventory.output(requested) == null)
            .put("routedDeviceKey", routed?.let { AudioDevices.key(it) } ?: previous.opt("routedDeviceKey") ?: JSONObject.NULL)
            .put("updatedAt", java.time.Instant.now().toString()).toString()
    }
    private fun progress(id: String, state: String) {
        outputStatus(state)
        report(JSONObject().put("type", if (state == "interrupted") "interrupt" else "playback").put("playback", JSONObject()
            .put("utteranceId", id).put("playedMs", if (started == 0L) 0 else SystemClock.elapsedRealtime() - started).put("state", state)))
    }
    private fun cleanup() { media?.release(); media = null; file?.delete(); file = null; pcm.reset(); current = null; started = 0 }
    private fun finish(id: String, state: String) {
        progress(id, state); cleanup()
        if (state == "failed") status("Phone speech failed")
        next()
    }
    fun interrupt() { epoch++; handler.post {
        val id = current
        tts?.stop(); queue.clear()
        if (id != null) progress(id, "interrupted")
        cleanup(); focus.release(); if (id != null) idle()
    } }
    fun close() { epoch++; ready = false; handler.post {
        audio.unregisterAudioDeviceCallback(devices)
        val id = current; if (id != null) progress(id, "interrupted")
        tts?.stop(); tts?.shutdown(); tts = null; queue.clear(); cleanup(); focus.release()
    } }
    companion object {
        fun playbackAttributes(): AudioAttributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build()
    }
}
