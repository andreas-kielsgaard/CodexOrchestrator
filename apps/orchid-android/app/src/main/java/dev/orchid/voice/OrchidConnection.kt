package dev.orchid.voice

import android.os.Handler
import android.os.Looper
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class OrchidConnection(settings: ConnectionSettings, private val status: (String) -> Unit, private val receive: (JSONObject) -> Unit,
    private val request: () -> Request = { Request.Builder().url(settings.endpoint).header("Authorization", "Bearer ${settings.token}").build() }) {
    private val client = OkHttpClient.Builder().connectTimeout(5, TimeUnit.SECONDS).pingInterval(10, TimeUnit.SECONDS).build()
    private val handler = Handler(Looper.getMainLooper())
    @Volatile private var active = false
    private var socket: WebSocket? = null
    private var retries = 0
    @Volatile var connected = false; private set
    fun start() { handler.post {
        if (!active) { retries = 0; active = true }
        if (socket == null) { handler.removeCallbacksAndMessages(null); connect() }
    } }
    private fun connect() {
        if (!active) return
        status("Connecting…")
        val attempt = client.newWebSocket(request(), object : WebSocketListener() {
            override fun onMessage(webSocket: WebSocket, text: String) {
                handler.post {
                    if (!active || socket !== webSocket) return@post
                    try { val value = JSONObject(text)
                        if (value.optString("type") == "hello") { if (value.optInt("version") != 2) { close(); status("Protocol version mismatch"); return@post }; connected = true; retries = 0; status("Listening · connected") }
                        receive(value)
                    } catch (error: Exception) { status("Connection message error: ${error.message}") }
                }
            }
            override fun onFailure(webSocket: WebSocket, error: Throwable, response: Response?) {
                val rejected = response?.code in listOf(401, 403)
                val message = when {
                    rejected -> "Connection rejected · check the access token"
                    response?.code == 409 -> "Laptop reports an existing phone connection"
                    response != null -> "Laptop rejected connection (${response.code})"
                    else -> "Laptop unavailable · ${error.message ?: "connection failed"}"
                }
                lost(webSocket, message, rejected)
            }
            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) { webSocket.close(if (code == 1005) 1000 else code, reason); lost(webSocket, "Connection closed") }
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { lost(webSocket, "Connection closed") }
        })
        socket = attempt
        handler.postDelayed({ if (active && socket === attempt && !connected) lost(attempt, "Laptop did not respond within 10 seconds") }, 10000)
    }
    private fun lost(webSocket: WebSocket, message: String, rejected: Boolean = false) {
        handler.post {
            if (socket !== webSocket) return@post
            connected = false; socket = null
            webSocket.cancel()
            handler.removeCallbacksAndMessages(null)
            if (!active) { status("Stopped"); return@post }
            if (rejected || retries == 3) {
                active = false
                status(if (rejected) message else "$message · stopped after 3 retries · tap Start listening")
            } else {
                retries++
                status("$message · retry $retries of 3")
                handler.postDelayed({ connect() }, 1000)
            }
        }
    }
    fun send(value: JSONObject): Boolean = connected && socket?.send(value.toString()) == true
    fun close() { active = false; connected = false; handler.removeCallbacksAndMessages(null); socket?.close(1000, "Conversation ended"); socket = null; client.dispatcher.executorService.shutdown() }
}
