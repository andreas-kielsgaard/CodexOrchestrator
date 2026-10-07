package dev.orchid.voice

import android.os.Handler
import android.os.Looper
import okhttp3.*
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit

class PhoneHistoryClient(private val settings: ConnectionSettings) {
    private val client = OkHttpClient.Builder().callTimeout(15, TimeUnit.SECONDS).build()
    private val main = Handler(Looper.getMainLooper())
    fun get(path: String, complete: (JSONObject?, String?) -> Unit): Call? {
        try {
            settings.validate()
            val base = settings.endpoint.substringBefore("/phone").replaceFirst("ws://", "http://")
            val call = client.newCall(Request.Builder().url(base + path)
                .header("Authorization", "Bearer ${settings.token}").build())
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, error: IOException) {
                    if (!call.isCanceled()) main.post { complete(null, "Laptop unavailable. Keep Tailscale connected and enable Voice Sessions on the laptop.") }
                }
                override fun onResponse(call: Call, response: Response) {
                    response.use {
                        val result = runCatching {
                            check(response.isSuccessful) { if (response.code == 401) "Access token was not accepted. Check Settings." else "${if (path == "/status") "Live status" else "Session history"} is unavailable (${response.code})." }
                            JSONObject(response.body?.string() ?: "{}")
                        }
                        if (!call.isCanceled()) main.post { complete(result.getOrNull(), result.exceptionOrNull()?.message) }
                    }
                }
            })
            return call
        } catch (error: Exception) { main.post { complete(null, error.message) }; return null }
    }
    fun close() { client.dispatcher.cancelAll(); client.dispatcher.executorService.shutdown(); client.connectionPool.evictAll() }
}
