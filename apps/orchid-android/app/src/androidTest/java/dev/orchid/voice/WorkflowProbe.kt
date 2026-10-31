package dev.orchid.voice

import android.app.Instrumentation
import android.content.Intent
import android.media.AudioManager
import android.os.Bundle
import android.os.SystemClock
import org.json.JSONObject

/** Real phone capture and playback around a text request sent by the production service. */
object WorkflowProbe {
    fun run(instrumentation: Instrumentation, result: Bundle, endpoint: String?) {
        val context = instrumentation.targetContext
        val settings = ConnectionSettings(context)
        val originalEndpoint = settings.endpoint
        val originalTimeout = settings.listeningTimeoutSeconds
        val audio = context.getSystemService(AudioManager::class.java)
        val originalVolume = audio.getStreamVolume(AudioManager.STREAM_MUSIC)
        fun await(seconds: Int, condition: () -> Boolean) {
            val end = SystemClock.elapsedRealtime() + seconds * 1000L
            while (!condition() && SystemClock.elapsedRealtime() < end) SystemClock.sleep(100)
            check(condition()) { "Voice workflow probe timed out: ${settings.status}" }
        }
        try {
            context.stopService(Intent(context, ConversationService::class.java))
            SystemClock.sleep(500)
            if (endpoint != null) settings.endpoint = endpoint
            settings.listeningTimeoutSeconds = 180
            context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            SystemClock.sleep(1000)
            context.startForegroundService(Intent(context, ConversationService::class.java).setAction("START"))
            await(20) { settings.status == "Listening · connected" && audio.activeRecordingConfigurations.any { !it.isClientSilenced } }
            result.putString("capture", "Production service connected with an unsilenced microphone")
            val before = settings.playbackStatus
            context.startService(Intent(context, ConversationService::class.java).setAction("TEXT").putExtra("text", "Orchid, this is a voice workflow test. Please say: Voice workflow is connected. No other work is needed."))
            await(150) { settings.playbackStatus != before && JSONObject(settings.playbackStatus).let { it.optString("state") == "completed" && it.optString("utteranceId").isNotBlank() && it.optString("text").contains("Voice workflow is connected", ignoreCase = true) } }
            result.putString("playback", settings.playbackStatus)
            result.putString("exchange", settings.lastExchange)
            check(audio.getStreamVolume(AudioManager.STREAM_MUSIC) == originalVolume) { "Media volume changed" }
        } finally {
            context.stopService(Intent(context, ConversationService::class.java))
            SystemClock.sleep(500)
            settings.endpoint = originalEndpoint
            settings.listeningTimeoutSeconds = originalTimeout
        }
    }
}
