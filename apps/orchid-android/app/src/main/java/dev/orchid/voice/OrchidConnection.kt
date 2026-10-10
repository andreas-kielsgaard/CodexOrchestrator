package dev.orchid.voice

import android.os.Handler
import android.os.Looper
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class OrchidConnection(private val settings: ConnectionSettings, private val status: (String) -> Unit, private val receive: (JSONObject) -> Unit) {
    private val client = OkHttpClient.Builder().connectTimeout(5, TimeUnit.SECONDS).pingInterval(10, TimeUnit.SECONDS).build()
    private val handler = Handler(Looper.getMainLooper())
    @Volatile private var active = false
    private var socket: WebSocket? = null
    @Volatile var connected = false; private set
    fun start() { handler.post { active = true; if (socket == null) { handler.removeCallbacksAndMessages(null); connect() } } }
    private fun connect() {
        if (!active) return
        status("Connecting…")
        val attempt = client.newWebSocket(Request.Builder().url(settings.endpoint).header("Authorization", "Bearer ${settings.token}").build(), object : WebSocketListener() {
            override fun onMessage(webSocket: WebSocket, text: String) {
                handler.post {
                    if (!active || socket !== webSocket) return@post
                    try { val value = JSONObject(text)
                        if (value.optString("type") == "hello") { if (value.optInt("version") != 2) { close(); status("Protocol version mismatch"); return@post }; connected = true; status("Listening · connected") }
                        receive(value)
                    } catch (error: Exception) { status("Connection message error: ${error.message}") }
                }
            }
            override fun onFailure(webSocket: WebSocket, error: Throwable, response: Response?) {
                lost(webSocket, if (response?.code in listOf(401, 403)) "Connection rejected · check the access token" else "Disconnected · retrying")
            }
            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) { webSocket.close(code, reason); lost(webSocket, "Disconnected · retrying") }
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { lost(webSocket, "Disconnected · retrying") }
        })
        socket = attempt
        handler.postDelayed({ if (active && socket === attempt && !connected) { attempt.cancel(); lost(attempt, "Laptop not responding · retrying") } }, 10000)
    }
    private fun lost(webSocket: WebSocket, message: String) {
        handler.post {
            if (socket !== webSocket) return@post
            connected = false; socket = null
            handler.removeCallbacksAndMessages(null)
            status(if (active) message else "Stopped")
            if (active) handler.postDelayed({ connect() }, 1000)
        }
    }
    fun send(value: JSONObject): Boolean = connected && socket?.send(value.toString()) == true
    fun close() { active = false; connected = false; handler.removeCallbacksAndMessages(null); socket?.close(1000, "Conversation ended"); socket = null; client.dispatcher.executorService.shutdown() }
}
