package dev.orchid.voice

import android.app.Instrumentation
import android.app.Activity
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.speech.tts.TextToSpeech
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

/** Tests the installed app's network route and credentials without opening its microphone. */
class ConnectionProbeInstrumentation : Instrumentation() {
    private var reconnect = false
    private var probe = "connection"
    override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); reconnect = arguments?.getString("reconnect") == "true"; probe = arguments?.getString("probe") ?: "connection"; start() }
    override fun onStart() {
        val result = Bundle()
        val client = OkHttpClient.Builder().connectTimeout(10, TimeUnit.SECONDS).build()
        try {
            if (probe == "audio") { probeMutedSpeech(result); finish(Activity.RESULT_OK, result); return }
            val settings = ConnectionSettings(targetContext)
            settings.validate()
            if (probe == "history") {
                val base = settings.endpoint.substringBefore("/phone").replaceFirst("ws://", "http://")
                fun get(path: String, authenticated: Boolean = true): Response {
                    val request = Request.Builder().url(base + path)
                    if (authenticated) request.header("Authorization", "Bearer ${settings.token}")
                    return client.newCall(request.build()).execute()
                }
                get("/sessions", false).use { check(it.code == 401) { "Unauthenticated history was accepted" } }
                val list = get("/sessions").use { check(it.isSuccessful); JSONObject(it.body!!.string()).getJSONArray("sessions") }
                check(list.length() >= 1) { "Main interaction session is missing" }
                for (index in 0 until list.length()) {
                    val id = list.getJSONObject(index).getString("id")
                    get("/sessions/$id").use { check(it.isSuccessful); check(JSONObject(it.body!!.string()).getString("sessionId") == id) }
                }
                get("/sessions/not-a-conversation-member").use { check(it.code == 404) { "Unrelated session history was accessible" } }
                result.putInt("sessions", list.length()); result.putString("history", "Authenticated main and worker history loaded; unauthenticated and unrelated access rejected")
                finish(Activity.RESULT_OK, result); return
            }
            if (reconnect) {
                probeReconnect(settings, result)
                finish(Activity.RESULT_OK, result)
                return
            }
            val latch = CountDownLatch(1)
            var failure: String? = null
            var socket: WebSocket? = null
            socket = client.newWebSocket(Request.Builder().url(settings.endpoint).header("Authorization", "Bearer ${settings.token}").build(), object : WebSocketListener() {
                override fun onMessage(ws: WebSocket, text: String) {
                    val value = JSONObject(text)
                    if (value.optString("type") == "hello" && value.optInt("version") == 1) {
                        result.putString("connection", "Authenticated version-1 hello received from Orchid")
                        ws.close(1000, "Probe complete"); latch.countDown()
                    }
                }
                override fun onFailure(ws: WebSocket, error: Throwable, response: Response?) { failure = "${error.javaClass.simpleName}: ${error.message}; HTTP ${response?.code}"; latch.countDown() }
            })
            check(latch.await(15, TimeUnit.SECONDS)) { "Phone connection timed out" }
            socket?.cancel()
            check(failure == null) { failure ?: "Connection failed" }
            val speechReady = CountDownLatch(1)
            val handler = Handler(Looper.getMainLooper())
            var speech: TextToSpeech? = null
            var voiceCount = 0
            handler.post { speech = TextToSpeech(targetContext) { status -> handler.post {
                if (status == TextToSpeech.SUCCESS) voiceCount = speech?.voices?.count { !it.isNetworkConnectionRequired && it.locale.language == "en" && !it.features.contains(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED) } ?: 0
                speechReady.countDown()
            } } }
            val initialized = speechReady.await(15, TimeUnit.SECONDS)
            handler.post { speech?.shutdown() }
            check(initialized && voiceCount > 0) { "No installed offline English speech voice available" }
            result.putInt("offlineEnglishVoices", voiceCount)
            finish(Activity.RESULT_OK, result)
        } catch (error: Exception) {
            result.putString("failure", error.message); finish(Activity.RESULT_CANCELED, result)
        } finally { client.dispatcher.executorService.shutdown(); client.connectionPool.evictAll() }
    }
    private fun probeMutedSpeech(result: Bundle) {
        val audio = targetContext.getSystemService(AudioManager::class.java)
        val initial = audio.getStreamVolume(AudioManager.STREAM_MUSIC)
        check(initial == 0 || audio.isStreamMute(AudioManager.STREAM_MUSIC)) { "Mute media volume before this probe; the probe does not change volume" }
        check(SpeechPlayer.playbackAttributes().volumeControlStream == AudioManager.STREAM_MUSIC) { "Speech does not use media volume" }
        val latch = CountDownLatch(1)
        var player: SpeechPlayer? = null
        var playback: JSONObject? = null
        val handler = Handler(Looper.getMainLooper())
        handler.post { player = SpeechPlayer(targetContext, { value -> playback = value.getJSONObject("playback"); latch.countDown() }, {}, {}) }
        val deadline = SystemClock.elapsedRealtime() + 15000
        while (player?.isReady != true && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100)
        try {
            check(player?.isReady == true) { "Speech player did not initialize" }
            player?.speak(JSONObject().put("id", "muted-volume-probe").put("text", "Orchid respects media volume."))
            check(latch.await(5, TimeUnit.SECONDS)) { "Muted playback receipt was not sent" }
            check(playback?.getString("state") == "muted" && playback?.getLong("playedMs") == 0L) { "Muted speech was played" }
            check(audio.getStreamVolume(AudioManager.STREAM_MUSIC) == initial) { "Media volume changed" }
            result.putString("audio", "Speech uses media volume; muted utterance suppressed with zero playback; volume unchanged")
        } finally { handler.post { player?.close() } }
    }
    private fun probeReconnect(settings: ConnectionSettings, result: Bundle) {
        val first = CountDownLatch(1)
        val lost = CountDownLatch(1)
        val restored = CountDownLatch(1)
        val hellos = AtomicInteger()
        val lostAt = AtomicLong()
        val restoredAt = AtomicLong()
        val connection = OrchidConnection(settings, { status ->
            if (status.startsWith("Disconnected")) { lostAt.compareAndSet(0, SystemClock.elapsedRealtime()); lost.countDown() }
        }, { message ->
            if (message.optString("type") == "hello" && message.optInt("version") == 1) {
                if (hellos.incrementAndGet() == 1) first.countDown()
                else { restoredAt.set(SystemClock.elapsedRealtime()); restored.countDown() }
            }
        })
        try {
            connection.start()
            check(first.await(15, TimeUnit.SECONDS)) { "Initial app connection timed out" }
            sendStatus(1, Bundle().apply { putString("stage", "App connected; interrupt and restore the desktop gateway now") })
            check(lost.await(40, TimeUnit.SECONDS)) { "App did not observe connection loss" }
            check(restored.await(40, TimeUnit.SECONDS) && connection.connected) { "App did not reconnect" }
            result.putString("reconnect", "OrchidConnection detected loss and received a new authenticated hello")
            result.putLong("recoveryMs", restoredAt.get() - lostAt.get())
            result.putInt("hellos", hellos.get())
        } finally { connection.close() }
    }
}
