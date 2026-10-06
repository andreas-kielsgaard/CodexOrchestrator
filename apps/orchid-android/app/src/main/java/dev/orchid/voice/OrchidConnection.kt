package dev.orchid.voice

import android.os.Handler
import android.os.Looper
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class OrchidConnection(private val settings: ConnectionSettings, private val status: (String) -> Unit, private val receive: (JSONObject) -> Unit) {
    private val client = OkHttpClient.Builder().pingInterval(20, TimeUnit.SECONDS).build()
    private val handler = Handler(Looper.getMainLooper())
    private var active = false
    private var socket: WebSocket? = null
    @Volatile var connected = false; private set
    fun start() { active = true; connect() }
    private fun connect() {
        if (!active) return
        status("Connecting…")
        socket = client.newWebSocket(Request.Builder().url(settings.endpoint).header("Authorization", "Bearer ${settings.token}").build(), object : WebSocketListener() {
            override fun onMessage(webSocket: WebSocket, text: String) {
                try { val value = JSONObject(text)
                    if (value.optString("type") == "hello") { if (value.optInt("version") != 1) { close(); status("Protocol version mismatch"); return }; connected = true; status("Listening · connected") }
                    receive(value)
                } catch (error: Exception) { status("Connection message error: ${error.message}") }
            }
            override fun onFailure(webSocket: WebSocket, error: Throwable, response: Response?) { lost(webSocket) }
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { lost(webSocket) }
            private fun lost(webSocket: WebSocket) { if (socket !== webSocket) return; connected = false; socket = null; status(if (active) "Disconnected · retrying; speech discarded" else "Stopped"); if (active) handler.postDelayed({ connect() }, 3000) }
        })
    }
    fun send(value: JSONObject): Boolean = connected && socket?.send(value.toString()) == true
    fun close() { active = false; connected = false; handler.removeCallbacksAndMessages(null); socket?.close(1000, "Conversation ended"); socket = null; client.dispatcher.executorService.shutdown() }
}
