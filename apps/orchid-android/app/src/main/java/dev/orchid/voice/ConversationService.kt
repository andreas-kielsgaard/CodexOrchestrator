package dev.orchid.voice

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.*
import android.util.Base64
import org.json.JSONObject
import java.util.UUID

class ConversationService : Service() {
    private lateinit var settings: ConnectionSettings
    private var connection: OrchidConnection? = null
    private var capture: AudioCapture? = null
    private var player: SpeechPlayer? = null
    private var wake: PowerManager.WakeLock? = null
    private val handler = Handler(Looper.getMainLooper())
    override fun onCreate() { super.onCreate(); settings = ConnectionSettings(this); getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel("conversation", "Voice conversation", NotificationManager.IMPORTANCE_LOW)) }
    override fun onBind(intent: Intent?) = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            "STOP" -> stopSelf()
            "TEXT" -> { val text = intent.getStringExtra("text") ?: ""; if (connection?.send(JSONObject().put("type", "text").put("id", UUID.randomUUID().toString()).put("text", text)) != true) settings.status = "Start the connection before sending" }
            "START" -> if (connection == null) try { begin() } catch (error: Exception) { settings.status = error.message ?: "Unable to start"; stopSelf() }
        }
        return START_NOT_STICKY
    }
    private fun begin() {
        settings.validate()
        val types = if (Build.VERSION.SDK_INT >= 30) ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE or ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK else ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK
        startForeground(1, notification("Connecting…"), types)
        wake = getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Orchid:conversation").apply { acquire(8 * 60 * 60 * 1000L) }
        player = SpeechPlayer(this, { connection?.send(it) }, { status(it) }, { if (connection?.connected == true) status("Listening · connected") })
        connection = OrchidConnection(settings, { status(it) }, { value ->
            when (value.optString("type")) {
                "speak" -> { val utterance = value.getJSONObject("utterance"); settings.lastExchange = "Orchid: ${utterance.getString("text")}"; if (capture?.speaking != true) player?.speak(utterance) else connection?.send(JSONObject().put("type", "playback").put("playback", JSONObject().put("utteranceId", utterance.getString("id")).put("playedMs", 0).put("state", "interrupted"))) }
                "transcript" -> settings.lastExchange = "You: ${value.getString("text")}"
                "error" -> status(value.optString("message"))
                "ended" -> handler.post { stopSelf() }
            }
        }).also { it.start() }
        capture = AudioCapture({ player?.interrupt() }, { bytes ->
            if (connection?.connected == true) connection?.send(JSONObject().put("type", "audio").put("id", UUID.randomUUID().toString()).put("audio", Base64.encodeToString(bytes, Base64.NO_WRAP)))
        }, { status(it) }).also { it.start() }
    }
    private fun notification(text: String): Notification {
        val open = PendingIntent.getActivity(this, 1, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        val stop = PendingIntent.getService(this, 2, Intent(this, ConversationService::class.java).setAction("STOP"), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Builder(this, "conversation").setSmallIcon(android.R.drawable.ic_btn_speak_now).setContentTitle("Orchid Voice").setContentText(text).setContentIntent(open).setOngoing(true).addAction(Notification.Action.Builder(null, "Stop", stop).build()).build()
    }
    private fun status(text: String) { settings.status = text; getSystemService(NotificationManager::class.java).notify(1, notification(text)) }
    override fun onDestroy() {
        capture?.close(); player?.close(); connection?.close(); if (wake?.isHeld == true) wake?.release()
        settings.status = "Stopped"; stopForeground(STOP_FOREGROUND_REMOVE); super.onDestroy()
    }
}
