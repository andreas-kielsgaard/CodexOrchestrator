package dev.orchid.voice

import android.app.Instrumentation
import android.content.Intent
import android.content.Context
import android.media.AudioManager
import android.os.Bundle
import android.os.SystemClock
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import org.json.JSONObject

object InactivityProbe {
    fun run(instrumentation: Instrumentation, result: Bundle) {
        val clock = ListeningInactivity()
        clock.activate(0)
        check(clock.remainingSeconds(59_000, 60) == 1L)
        for (type in listOf("transcript", "input_classification", "cue", "speak", "hello", "speech_state")) clock.receive(type, 59_000)
        check(clock.remainingSeconds(60_000, 60) == 0L) { "Unrelated activity renewed listening" }
        clock.receive("relevant_input", 59_000)
        check(clock.remainingSeconds(60_000, 60) == 59L)
        check(clock.remainingSeconds(119_000, 60) == 0L)
        check(clock.remainingSeconds(64_000, 5) == 0L) { "Changed timeout did not apply to the last relevant input" }
        clock.stop(); clock.receive("relevant_input", 120_000)
        check(clock.remainingSeconds(120_000, 60) == null) { "Late input reactivated a stopped microphone" }
        clock.activate(120_000)
        check(clock.remainingSeconds(120_000, 60) == 60L)

        val context = instrumentation.targetContext
        val settings = ConnectionSettings(context)
        val previousTimeout = settings.listeningTimeoutSeconds
        val audio = context.getSystemService(AudioManager::class.java)
        fun recording() = JSONObject(settings.captureStatus).optBoolean("recording") && audio.activeRecordingConfigurations.any { it.clientAudioSessionId != 0 }
        fun stopped() = !JSONObject(settings.captureStatus).optBoolean("recording") && audio.activeRecordingConfigurations.none { it.clientAudioSessionId != 0 }
        fun await(message: String, condition: () -> Boolean) {
            val deadline = SystemClock.elapsedRealtime() + 15_000
            while (!condition() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100)
            check(condition()) { "$message: ${settings.status}" }
        }
        fun button(view: View, title: String): Button? {
            if (view is Button && view.text.toString() == title) return view
            if (view is ViewGroup) for (index in 0 until view.childCount) button(view.getChildAt(index), title)?.let { return it }
            return null
        }
        try {
            context.stopService(Intent(context, ConversationService::class.java))
            settings.listeningTimeoutSeconds = 5
            val activity = instrumentation.startActivitySync(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            instrumentation.runOnMainSync { check(button(activity.window.decorView, "Start listening")?.performClick() == true) }
            await("App activation did not record") { recording() }
            await("Inactivity did not stop recording") { stopped() && settings.listeningStopReason.contains("5 seconds") }
            SystemClock.sleep(2000)
            check(stopped()) { "Microphone restarted itself" }
            context.startService(Intent(context, ConversationService::class.java).setAction("MICROPHONE").putExtra("microphone", settings.microphone))
            SystemClock.sleep(300)
            check(stopped()) { "Microphone selection restarted inactive listening" }
            context.startActivity(Intent(Intent.ACTION_ASSIST).setClass(context, AssistantActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            await("Assistant activation did not resume recording") { recording() && settings.listeningStopReason.isBlank() }
            await("Second activation did not expire") { stopped() && settings.listeningStopReason.contains("5 seconds") }
            instrumentation.runOnMainSync { button(activity.window.decorView, "Start listening")!!.performClick() }
            await("App activation did not resume recording") { recording() && settings.listeningStopReason.isBlank() }
            result.putString("inactivity", "Only confirmed relevance resets the clock; expiry stops recording; microphone selection does not resume; app and assistant activation resume")
        } finally {
            context.stopService(Intent(context, ConversationService::class.java))
            await("Probe cleanup did not stop capture") { stopped() }
            // Instrumentation.finish terminates the process; flush restoration before returning.
            check(context.getSharedPreferences("connection", Context.MODE_PRIVATE).edit().putInt("listening-timeout-seconds", previousTimeout).commit())
        }
    }
}
