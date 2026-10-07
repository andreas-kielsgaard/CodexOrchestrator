package dev.orchid.voice

import android.content.Context
import android.media.AudioAttributes
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import org.json.JSONObject
import java.util.Locale

class SpeechPlayer(context: Context, private val report: (JSONObject) -> Unit, private val status: (String) -> Unit, private val idle: () -> Unit) {
    private val handler = Handler(Looper.getMainLooper())
    private var ready = false
    @Volatile private var current: String? = null
    @Volatile private var started = 0L
    private val queue = ArrayDeque<JSONObject>()
    @Volatile private var epoch = 0L
    private var tts: TextToSpeech? = null
    init {
        tts = TextToSpeech(context) { result -> handler.post {
            val engine = tts ?: return@post
            if (result != TextToSpeech.SUCCESS) { status("Phone speech engine unavailable"); return@post }
            val voice = engine.voices?.filter { !it.isNetworkConnectionRequired && it.locale.language == "en" && !it.features.contains(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED) }?.sortedByDescending { it.quality }?.firstOrNull()
            if (voice == null) { status("Install an offline English voice in Android text-to-speech settings"); return@post }
            engine.language = Locale.US; engine.voice = voice
            engine.setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build())
            engine.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                override fun onStart(id: String) { if (current == id) { started = SystemClock.elapsedRealtime(); progress(id, "playing") } }
                override fun onDone(id: String) { if (current == id) { progress(id, "completed"); current = null; handler.post { next() } } }
                @Deprecated("Android callback") override fun onError(id: String) { if (current == id) { progress(id, "failed"); current = null; status("Phone speech failed"); handler.post { next() } } }
            }); ready = true
        } }
    }
    fun speak(utterance: JSONObject) { val generation = epoch; handler.post {
        if (generation != epoch) return@post
        val id = utterance.getString("id")
        if (!ready) { report(JSONObject().put("type", "playback").put("playback", JSONObject().put("utteranceId", id).put("playedMs", 0).put("state", "unavailable"))); return@post }
        queue.addLast(utterance); next()
    } }
    private fun next() {
        if (current != null) return
        if (queue.isEmpty()) { idle(); return }
        val utterance = queue.removeFirst(); val id = utterance.getString("id")
        current = id; started = 0; status("Speaking · you can interrupt")
        if (tts?.speak(utterance.getString("text"), TextToSpeech.QUEUE_FLUSH, null, id) == TextToSpeech.ERROR) { progress(id, "failed"); current = null; next() }
    }
    private fun progress(id: String, state: String) { report(JSONObject().put("type", if (state == "interrupted") "interrupt" else "playback").put("playback", JSONObject().put("utteranceId", id).put("playedMs", if (started == 0L) 0 else SystemClock.elapsedRealtime() - started).put("state", state))) }
    fun interrupt() { epoch++; val id = current; current = null; tts?.stop(); if (id != null) progress(id, "interrupted"); handler.post { queue.clear(); if (id != null) idle() } }
    fun close() { interrupt(); handler.removeCallbacksAndMessages(null); tts?.shutdown(); tts = null }
}
