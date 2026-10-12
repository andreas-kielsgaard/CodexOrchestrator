package dev.orchid.voice

import android.app.Instrumentation
import android.os.Bundle
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/** Reconnects to the live desktop while the prior authenticated phone socket is still open. */
object ConnectionTakeoverProbe {
    fun run(instrumentation: Instrumentation, result: Bundle) {
        val settings = ConnectionSettings(instrumentation.targetContext)
        val client = OkHttpClient.Builder().connectTimeout(5, TimeUnit.SECONDS).build()
        val firstHello = CountDownLatch(1)
        val firstClosed = CountDownLatch(1)
        val secondHello = CountDownLatch(1)
        val pongs = AtomicInteger()
        var failure: String? = null
        fun listener(hello: CountDownLatch, closed: CountDownLatch? = null) = object : WebSocketListener() {
            override fun onMessage(socket: WebSocket, text: String) {
                when (JSONObject(text).optString("type")) {
                    "hello" -> hello.countDown()
                    "pong" -> pongs.incrementAndGet()
                }
            }
            override fun onClosing(socket: WebSocket, code: Int, reason: String) { socket.close(code, reason); closed?.countDown() }
            override fun onClosed(socket: WebSocket, code: Int, reason: String) { closed?.countDown() }
            override fun onFailure(socket: WebSocket, error: Throwable, response: Response?) { failure = "${error.message}; HTTP ${response?.code}"; hello.countDown() }
        }
        fun request(authenticated: Boolean = true) = Request.Builder().url(settings.endpoint).apply {
            if (authenticated) header("Authorization", "Bearer ${settings.token}")
        }.build()
        var first: WebSocket? = null
        var second: WebSocket? = null
        var rejected: WebSocket? = null
        try {
            first = client.newWebSocket(request(), listener(firstHello, firstClosed))
            check(firstHello.await(10, TimeUnit.SECONDS) && failure == null) { failure ?: "Initial hello missing" }
            second = client.newWebSocket(request(), listener(secondHello))
            check(secondHello.await(10, TimeUnit.SECONDS) && failure == null) { failure ?: "Replacement hello missing" }
            check(firstClosed.await(5, TimeUnit.SECONDS)) { "Previous socket stayed open" }
            val unauthorized = CountDownLatch(1)
            rejected = client.newWebSocket(request(false), object : WebSocketListener() {
                override fun onFailure(socket: WebSocket, error: Throwable, response: Response?) {
                    if (response?.code != 401) failure = "Unauthenticated response ${response?.code}"
                    unauthorized.countDown()
                }
            })
            check(unauthorized.await(5, TimeUnit.SECONDS) && failure == null)
            second.send("{\"type\":\"ping\"}")
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
            while (pongs.get() == 0 && System.nanoTime() < deadline) Thread.sleep(50)
            check(pongs.get() > 0) { "Replacement connection did not stay alive" }
            val statusUrl = settings.endpoint.replaceFirst("ws://", "http://").substringBefore("/phone") + "/status"
            client.newCall(Request.Builder().url(statusUrl).header("Authorization", "Bearer ${settings.token}").build()).execute().use {
                check(it.isSuccessful && JSONObject(it.body!!.string()).getBoolean("phoneConnected")) { "Old socket cleanup cleared the replacement's connected state" }
            }
            result.putString("takeover", "New authenticated socket replaced old socket; old cleanup and unauthenticated requests preserved replacement state; ping passed")
        } finally {
            first?.cancel(); second?.cancel(); rejected?.cancel()
            client.dispatcher.executorService.shutdown()
        }
    }
}
