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
    private lateinit var detectedInput: DetectedInput
    private var connection: OrchidConnection? = null
    private var capture: AudioCapture? = null
    private var player: SpeechPlayer? = null
    private var cues: CuePlayer? = null
    private var wake: PowerManager.WakeLock? = null
    private val handler = Handler(Looper.getMainLooper())
    private val inactivity = ListeningInactivity()
    @Volatile private var listening = false
    private val checkInactivity = object : Runnable {
        override fun run() {
            if (!listening) return
            if (inactivity.remainingSeconds(SystemClock.elapsedRealtime(), settings.listeningTimeoutSeconds) == 0L) {
                listening = false
                val followingUp = inactivity.followingUp()
                inactivity.stop()
                settings.listeningStopReason = if (followingUp) "Stopped · follow-up window ended" else "Stopped · no relevant input for ${settings.listeningTimeoutSeconds} seconds"
                capture?.close(); capture = null
                connection?.send(JSONObject().put("type", "speech_state").put("speaking", false))
                startForeground(1, notification(settings.listeningStopReason), ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
                status(settings.listeningStopReason)
            } else handler.postDelayed(this, 1000)
        }
    }
    override fun onCreate() { super.onCreate(); settings = ConnectionSettings(this); detectedInput = DetectedInput(settings); getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel("conversation", "Voice conversation", NotificationManager.IMPORTANCE_LOW)) }
    override fun onBind(intent: Intent?) = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            "STOP" -> stopSelf()
            "TEXT" -> { val text = intent.getStringExtra("text") ?: ""; if (connection?.send(JSONObject().put("type", "text").put("id", UUID.randomUUID().toString()).put("text", text)) != true) settings.status = "Start the connection before sending" }
            "START" -> try {
                settings.listeningStopReason = ""
                inactivity.activate(SystemClock.elapsedRealtime())
                if (connection == null) begin() else if (!listening) {
                    startForeground(1, notification("Starting microphone…"), foregroundTypes())
                    listening = true; startCapture()
                    status(if (connection?.connected == true) "Listening · connected" else "Listening · connecting")
                }
                handler.removeCallbacks(checkInactivity); handler.postDelayed(checkInactivity, 1000)
            } catch (error: Exception) { settings.status = error.message ?: "Unable to start"; stopSelf() }
            "INPUT_SETTINGS" -> { connection?.send(settings.inputSettings()); if (connection == null) stopSelf() }
            "MICROPHONE" -> {
                settings.microphone = intent.getStringExtra("microphone") ?: settings.microphone
                if (listening) try { startCapture() } catch (error: Exception) { status(error.message ?: "Microphone unavailable") }
            }
        }
        return START_NOT_STICKY
    }
    private fun begin() {
        settings.validate()
        listening = true
        startForeground(1, notification("Connecting…"), foregroundTypes())
        wake = getSystemService(PowerManager::class.java).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Orchid:conversation").apply { acquire(8 * 60 * 60 * 1000L) }
        player = SpeechPlayer(this, { connection?.send(it) }, { status(it) }, { if (connection?.connected == true) status("Listening · connected") })
        cues = CuePlayer(this)
        connection = OrchidConnection(settings, { status(it) }, { value ->
            handler.post {
                inactivity.receive(value.optString("type"), SystemClock.elapsedRealtime())
                if (value.optString("type") == "input_released") inactivity.delivered(SystemClock.elapsedRealtime(), value.optInt("followupSeconds", settings.followupSeconds))
            }
            when (value.optString("type")) {
                "hello" -> {
                    connection?.send(settings.inputSettings())
                    handler.post {
                        val inputs = value.optJSONArray("inputClassifications")
                        if (inputs != null) for (index in 0 until inputs.length()) {
                            val input = inputs.getJSONObject(index)
                            detectedInput.classify(input.getString("id"), input.getString("relevance"))
                        }
                    }
                    connection?.send(JSONObject().put("type", "capture_status").put("capture", JSONObject(settings.captureStatus)))
                    connection?.send(JSONObject().put("type", "speech_state").put("speaking", capture?.speaking == true))
                }
                "cue" -> cues?.play(value.optString("cue"))
                "speak" -> { val utterance = value.getJSONObject("utterance"); settings.lastExchange = "Orchid: ${utterance.getString("text")}"; if (capture?.speaking != true) player?.speak(utterance) else connection?.send(JSONObject().put("type", "playback").put("playback", JSONObject().put("utteranceId", utterance.getString("id")).put("playedMs", 0).put("state", "interrupted"))) }
                "transcript" -> handler.post {
                    settings.lastExchange = "You: ${value.getString("text")}"
                    detectedInput.append(value.getString("id"), value.getString("text"))
                }
                "transcript_window" -> handler.post {
                    detectedInput.window(value.getString("streamId"), value.getLong("windowStartMs"), value.getJSONArray("blocks"))
                    value.getJSONArray("blocks").let { blocks -> if (blocks.length() > 0) settings.lastExchange = "You: ${blocks.getJSONObject(blocks.length() - 1).getString("text")}" }
                }
                "input_classification" -> handler.post { detectedInput.classify(value.getString("id"), value.getString("relevance")) }
                "error" -> status(value.optString("message"))
                "ended" -> handler.post { stopSelf() }
            }
        }).also { it.start() }
        startCapture()
    }
    private fun startCapture() {
        capture?.close()
        connection?.send(JSONObject().put("type", "speech_state").put("speaking", false))
        var segmentsSent = 0L
        capture = AudioCapture(this, { player?.interrupt(); connection?.send(JSONObject().put("type", "speech_state").put("speaking", true)) }, { streamId, epochMs, startSample, bytes, hasSpeech ->
            if (connection?.send(JSONObject().put("type", "audio_stream").put("streamId", streamId).put("epochMs", epochMs).put("startSample", startSample).put("hasSpeech", hasSpeech).put("pcm", Base64.encodeToString(bytes, Base64.NO_WRAP))) == true) segmentsSent++
        }, { status(it) }, { connection?.send(JSONObject().put("type", "speech_state").put("speaking", false)) }, { report ->
            report.put("segmentsSent", segmentsSent)
            inactivity.remainingSeconds(SystemClock.elapsedRealtime(), settings.listeningTimeoutSeconds)?.let { report.put("idleRemainingSeconds", it) }
            settings.captureStatus = report.toString()
            connection?.send(JSONObject().put("type", "capture_status").put("capture", report))
        }).also { it.start() }
    }
    private fun notification(text: String): Notification {
        val open = PendingIntent.getActivity(this, 1, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        val stop = PendingIntent.getService(this, 2, Intent(this, ConversationService::class.java).setAction("STOP"), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Builder(this, "conversation").setSmallIcon(android.R.drawable.ic_btn_speak_now).setContentTitle("Orchid Voice").setContentText(text).setContentIntent(open).setOngoing(true).addAction(Notification.Action.Builder(null, "Stop", stop).build()).build()
    }
    private fun foregroundTypes() = if (Build.VERSION.SDK_INT >= 30) ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE or ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK else ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK
    private fun status(text: String) {
        val message = if (!listening && text.startsWith("Listening") && settings.listeningStopReason.isNotBlank()) settings.listeningStopReason else text
        settings.status = message; getSystemService(NotificationManager::class.java).notify(1, notification(message))
    }
    override fun onDestroy() {
        listening = false; inactivity.stop(); handler.removeCallbacksAndMessages(null)
        capture?.close(); cues?.close(); player?.close(); connection?.close(); if (wake?.isHeld == true) wake?.release()
        settings.listeningStopReason = ""; settings.status = "Stopped"; stopForeground(STOP_FOREGROUND_REMOVE); super.onDestroy()
    }
}
