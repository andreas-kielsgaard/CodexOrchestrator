package dev.orchid.voice

import android.app.Instrumentation
import android.os.Bundle
import android.os.SystemClock
import okhttp3.WebSocketListener
import okhttp3.Request
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import java.util.concurrent.atomic.AtomicReference

/** Exercises retry limits against real HTTP and WebSocket handshakes, without a microphone. */
object ConnectionRetryProbe {
    fun run(instrumentation: Instrumentation, result: Bundle) {
        val settings = ConnectionSettings(instrumentation.targetContext)
        fun await(timeout: Long = 15000, condition: () -> Boolean) {
            val deadline = SystemClock.elapsedRealtime() + timeout
            while (!condition() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(50)
            check(condition()) { "Connection did not finish within $timeout ms" }
        }
            MockWebServer().use { server ->
                repeat(8) { server.enqueue(MockResponse().setResponseCode(503)) }
                server.start()
                val testRequest = Request.Builder().url(server.url("/phone")).build()
                val last = AtomicReference("")
                val connection = OrchidConnection(settings, { last.set(it) }, {}, { testRequest })
                try {
                    connection.start()
                    await { last.get().contains("stopped after 3 retries") }
                    SystemClock.sleep(1500)
                    check(server.requestCount == 4) { "Expected initial attempt + 3 retries, got ${server.requestCount}" }
                    connection.start()
                    await { server.requestCount == 8 && last.get().contains("stopped after 3 retries") }
                    SystemClock.sleep(1500)
                    check(server.requestCount == 8) { "Manual restart exceeded its retry budget" }
                } finally { connection.close() }
                result.putInt("failedAttempts", server.requestCount)
            }
            MockWebServer().use { server ->
                server.enqueue(MockResponse().setResponseCode(401))
                server.start()
                val testRequest = Request.Builder().url(server.url("/phone")).build()
                val last = AtomicReference("")
                val connection = OrchidConnection(settings, { last.set(it) }, {}, { testRequest })
                try {
                    connection.start(); await { last.get().contains("check the access token") }
                    SystemClock.sleep(1500)
                    check(server.requestCount == 1) { "Rejected credentials were retried" }
                } finally { connection.close() }
                result.putInt("rejectedAttempts", server.requestCount)
            }
            instrumentation.sendStatus(1, Bundle().apply { putString("stage", "Retry caps passed; testing missing hello timeout") })
            MockWebServer().use { server ->
                repeat(4) { server.enqueue(MockResponse().withWebSocketUpgrade(object : WebSocketListener() {})) }
                server.start()
                val testRequest = Request.Builder().url(server.url("/phone")).build()
                val last = AtomicReference("")
                val connection = OrchidConnection(settings, { last.set(it) }, {}, { testRequest })
                try {
                    connection.start(); await(48000) { last.get().contains("stopped after 3 retries") }
                    SystemClock.sleep(1500)
                    check(server.requestCount == 4 && !connection.connected) { "Missing hello did not stop after four bounded attempts" }
                } finally { connection.close() }
                result.putInt("missingHelloAttempts", server.requestCount)
            }
            result.putString("retries", "Initial attempt + 3 retries; manual restart gets a fresh budget; 401 stops immediately; missing hello times out")
    }
}
