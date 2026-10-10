package dev.orchid.voice

import android.content.Context
import java.net.URI
import org.json.JSONObject

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
    var microphone: String
        get() = preferences.getString("microphone", "phone") ?: "phone"
        set(value) { require(value in listOf("phone", "headset")); preferences.edit().putString("microphone", value).apply() }
    var googleHandoff: Boolean
        get() = preferences.getBoolean("google-handoff", false)
        set(value) { preferences.edit().putBoolean("google-handoff", value).apply() }
    var captureStatus: String
        get() = preferences.getString("capture-status", "{}") ?: "{}"
        set(value) { preferences.edit().putString("capture-status", value).apply() }
    var listeningTimeoutSeconds: Int
        get() = preferences.getInt("listening-timeout-seconds", 60)
        set(value) { require(value > 0); preferences.edit().putInt("listening-timeout-seconds", value).apply() }
    var listeningStopReason: String
        get() = preferences.getString("listening-stop-reason", "") ?: ""
        set(value) { preferences.edit().putString("listening-stop-reason", value).apply() }
    var detectedInput: String
        get() = preferences.getString("detected-input", "[]") ?: "[]"
        set(value) { preferences.edit().putString("detected-input", value).apply() }
    var sentencesPerQuery: Int
        get() = preferences.getInt("sentences-per-query", 2)
        set(value) { require(value in 1..20); preferences.edit().putInt("sentences-per-query", value).apply() }
    var pauseSeconds: Int
        get() = preferences.getInt("pause-seconds", 2)
        set(value) { require(value in 1..60); preferences.edit().putInt("pause-seconds", value).apply() }
    var contextSeconds: Int
        get() = preferences.getInt("context-seconds", 60)
        set(value) { require(value in 10..120); preferences.edit().putInt("context-seconds", value).apply() }
    var followupSeconds: Int
        get() = preferences.getInt("followup-seconds", 30)
        set(value) { require(value in 1..600); preferences.edit().putInt("followup-seconds", value).apply() }
    fun inputSettings() = JSONObject().put("type", "input_settings").put("settings", JSONObject()
        .put("sentencesPerQuery", sentencesPerQuery).put("pauseSeconds", pauseSeconds)
        .put("contextSeconds", contextSeconds).put("followupSeconds", followupSeconds))
    fun validate() {
        val uri = URI(endpoint)
        val octets = uri.host?.split('.')?.mapNotNull { it.toIntOrNull() } ?: emptyList()
        require(uri.scheme == "ws" && uri.path == "/phone" && octets.size == 4 && octets.all { it in 0..255 } && octets[0] == 100 && octets[1] in 64..127) { "Use the laptop Tailscale endpoint: ws://100.x.x.x:4818/phone" }
        require(token.length >= 24) { "Copy the access token from Orchid" }
    }
}
