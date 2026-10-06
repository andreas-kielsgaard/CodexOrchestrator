package dev.orchid.voice

import android.app.Instrumentation
import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.tts.TextToSpeech
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Tests the installed app's network route and credentials without opening its microphone. */
class ConnectionProbeInstrumentation : Instrumentation() {
    override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); start() }
    override fun onStart() {
        val result = Bundle()
        val client = OkHttpClient.Builder().connectTimeout(10, TimeUnit.SECONDS).build()
        try {
            val settings = ConnectionSettings(targetContext)
            settings.validate()
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
}
