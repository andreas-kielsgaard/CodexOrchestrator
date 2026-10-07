package dev.orchid.voice

import android.content.Context
import java.net.URI

class ConnectionSettings(context: Context) {
    private val preferences = context.getSharedPreferences("connection", Context.MODE_PRIVATE)
    var endpoint: String
        get() = preferences.getString("endpoint", "") ?: ""
        set(value) { preferences.edit().putString("endpoint", value).apply() }
    var token: String
        get() = preferences.getString("token", "") ?: ""
        set(value) { preferences.edit().putString("token", value).apply() }
    var status: String
        get() = preferences.getString("status", "Stopped") ?: "Stopped"
        set(value) { preferences.edit().putString("status", value).apply() }
    var lastExchange: String
        get() = preferences.getString("exchange", "") ?: ""
        set(value) { preferences.edit().putString("exchange", value).apply() }
    var backgroundAudio: String
        get() = preferences.getString("background-audio", "lower") ?: "lower"
        set(value) { require(value in listOf("lower", "pause", "off")); preferences.edit().putString("background-audio", value).apply() }
    fun validate() {
        val uri = URI(endpoint)
        val octets = uri.host?.split('.')?.mapNotNull { it.toIntOrNull() } ?: emptyList()
        require(uri.scheme == "ws" && uri.path == "/phone" && octets.size == 4 && octets.all { it in 0..255 } && octets[0] == 100 && octets[1] in 64..127) { "Use the laptop Tailscale endpoint: ws://100.x.x.x:4818/phone" }
        require(token.length >= 24) { "Copy the access token from Orchid" }
    }
}
