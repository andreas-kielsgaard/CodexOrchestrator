package dev.orchid.voice

import android.app.ActivityManager
import android.app.Instrumentation
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.json.JSONObject
import java.net.InetAddress
import java.net.NetworkInterface
import java.util.concurrent.atomic.AtomicInteger

/** Uses a phone-local gateway, so microphone capture never reaches the task engine. */
object IdleLifecycleProbe {
    fun run(instrumentation: Instrumentation, result: Bundle) {
        val context = instrumentation.targetContext
        val settings = ConnectionSettings(context)
        val preferences = context.getSharedPreferences("connection", Context.MODE_PRIVATE)
        val endpoint = settings.endpoint
        val timeout = settings.listeningTimeoutSeconds
        val status = settings.status
        val stopReason = settings.listeningStopReason
        val intent = Intent(context, ConversationService::class.java)
        val phoneHost = NetworkInterface.getNetworkInterfaces().toList().flatMap { it.inetAddresses.toList() }
            .mapNotNull { it.hostAddress }.first { address ->
                val octets = address.split('.').mapNotNull { it.toIntOrNull() }
                octets.size == 4 && octets[0] == 100 && octets[1] in 64..127
            }
        fun running() = context.getSystemService(ActivityManager::class.java).getRunningServices(100)
            .any { it.service.className == ConversationService::class.java.name }
        fun power() = ParcelFileDescriptor.AutoCloseInputStream(instrumentation.uiAutomation.executeShellCommand("dumpsys power"))
            .bufferedReader().use { it.readText() }
        fun await(message: String, timeoutMs: Long = 15_000, condition: () -> Boolean) {
            val deadline = SystemClock.elapsedRealtime() + timeoutMs
            while (!condition() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100)
            check(condition()) { "$message: ${settings.status}" }
        }
        fun assertIdle(reason: String, timeoutMs: Long = 15_000) {
            await("Session resources retained after $reason", timeoutMs) {
                !running() && settings.status.contains(reason) && !JSONObject(settings.captureStatus).optBoolean("recording") &&
                    context.getSystemService(NotificationManager::class.java).activeNotifications.none { it.id == 1 }
            }
            check(!power().contains("'Orchid:conversation'")) { "CPU wake lock retained" }
            val stoppedStatus = settings.status
            SystemClock.sleep(1200)
            check(!running() && settings.status == stoppedStatus) { "Late callback restarted service or changed idle status" }
        }
        fun start(server: MockWebServer) {
            settings.endpoint = "ws://$phoneHost:${server.port}/phone"
            context.startForegroundService(Intent(intent).setAction("START"))
        }
        fun gateway(followup: Boolean = false, version: Int = 2, closed: AtomicInteger = AtomicInteger()) = MockResponse()
            .withWebSocketUpgrade(object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: okhttp3.Response) {
                    webSocket.send("{\"type\":\"hello\",\"version\":$version}")
                    if (followup) webSocket.send("{\"type\":\"input_released\",\"followupSeconds\":3}")
                }
                override fun onFailure(webSocket: WebSocket, t: Throwable, response: okhttp3.Response?) { closed.incrementAndGet() }
                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { closed.incrementAndGet() }
            })
        try {
            context.stopService(intent)
            await("Existing service did not stop") { !running() }
            settings.listeningTimeoutSeconds = 5
            MockWebServer().use { server ->
                val closed = AtomicInteger()
                repeat(3) { server.enqueue(gateway(closed = closed)) }
                server.start(InetAddress.getByName("0.0.0.0"), 0)
                start(server)
                await("Activation did not record") { JSONObject(settings.captureStatus).optBoolean("recording") }
                check(power().contains("'Orchid:conversation'")) { "Active wake lock was not acquired" }
                assertIdle("no relevant input for 5 seconds")
                await("Idle WebSocket still connected") { closed.get() >= 1 }
                check(server.requestCount == 1) { "Idle connection retried" }
                // Automatic timeout must permit a fresh activation without retaining the old connection.
                start(server)
                await("Reactivation did not record") { JSONObject(settings.captureStatus).optBoolean("recording") && settings.listeningStopReason.isBlank() }
                context.startService(Intent(intent).setAction("STOP"))
                assertIdle("Stopped")
                check(server.requestCount == 2) { "Reactivation did not establish a fresh socket" }
                context.startService(Intent(intent).setAction("MICROPHONE").putExtra("microphone", settings.microphone))
                SystemClock.sleep(500)
                check(!running()) { "Idle microphone setting retained a service" }
                result.putString("timeout", "Recording, WebSocket, wake lock and notification stopped; manual activation reconnects; idle settings stay idle")
            }
            MockWebServer().use { server ->
                server.enqueue(gateway(followup = true)); server.start(InetAddress.getByName("0.0.0.0"), 0)
                start(server)
                await("Follow-up activation did not record") { JSONObject(settings.captureStatus).optBoolean("recording") }
                assertIdle("follow-up window ended")
                result.putString("followup", "Follow-up expiry releases all session resources")
            }
            MockWebServer().use { server ->
                repeat(5) { server.enqueue(MockResponse().setResponseCode(503)) }
                server.start(InetAddress.getByName("0.0.0.0"), 0)
                start(server); assertIdle("stopped after 3 retries")
                check(server.requestCount == 4) { "Retry budget was not initial attempt plus three retries" }
                result.putInt("failedAttempts", server.requestCount)
            }
            MockWebServer().use { server ->
                server.enqueue(MockResponse().setResponseCode(401)); server.start(InetAddress.getByName("0.0.0.0"), 0)
                start(server); assertIdle("check the access token")
                check(server.requestCount == 1) { "Rejected credentials retried" }
                result.putInt("rejectedAttempts", server.requestCount)
            }
            MockWebServer().use { server ->
                server.enqueue(gateway(version = 99)); server.start(InetAddress.getByName("0.0.0.0"), 0)
                start(server); assertIdle("Protocol version mismatch")
                check(server.requestCount == 1) { "Incompatible protocol retried" }
                result.putString("protocol", "Protocol rejection releases all session resources")
            }
            MockWebServer().use { server ->
                repeat(4) { server.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {})) }
                server.start(InetAddress.getByName("0.0.0.0"), 0)
                start(server); assertIdle("stopped after 3 retries", 50_000)
                check(server.requestCount == 4) { "Missing hello exceeded its retry budget" }
                result.putInt("missingHelloAttempts", server.requestCount)
            }
        } finally {
            context.stopService(intent)
            await("Cleanup did not stop service") { !running() }
            check(preferences.edit().putString("endpoint", endpoint).putInt("listening-timeout-seconds", timeout)
                .putString("status", status).putString("listening-stop-reason", stopReason).commit())
        }
    }
}
