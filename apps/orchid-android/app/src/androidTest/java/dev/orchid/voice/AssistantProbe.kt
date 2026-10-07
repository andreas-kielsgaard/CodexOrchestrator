package dev.orchid.voice

import android.app.Instrumentation
import android.app.role.RoleManager
import android.content.Intent
import android.media.AudioManager
import android.os.Bundle
import android.os.SystemClock

/** Exercises installed assistant activation, microphone access, repeat activation, and cleanup. */
object AssistantProbe {
    fun run(instrumentation: Instrumentation, result: Bundle) {
        val context = instrumentation.targetContext
        val settings = ConnectionSettings(context)
        val audio = context.getSystemService(AudioManager::class.java)
        check(context.getSystemService(RoleManager::class.java).isRoleHeld(RoleManager.ROLE_ASSISTANT)) { "Select Orchid as phone assistant first" }
        val microphone = settings.microphone
        val initialVolume = audio.getStreamVolume(AudioManager.STREAM_MUSIC)
        fun await(condition: () -> Boolean) {
            val deadline = SystemClock.elapsedRealtime() + 15000
            while (!condition() && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100)
            check(condition()) { "Assistant did not reach expected state; ${settings.status}" }
        }
        fun recordings() = audio.activeRecordingConfigurations.filter { it.clientAudioSessionId != 0 }
        try {
            context.stopService(Intent(context, ConversationService::class.java))
            await { recordings().isEmpty() }
            val launch = Intent(Intent.ACTION_ASSIST).setClass(context, AssistantActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(launch)
            await { settings.status == "Listening · connected" && recordings().size == 1 }
            check(!recordings().single().isClientSilenced) { "Assistant microphone is silenced" }
            val session = recordings().single().clientAudioSessionId
            context.startActivity(Intent(Intent.ACTION_VOICE_COMMAND).setClass(context, AssistantActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            SystemClock.sleep(1000)
            check(recordings().size == 1 && recordings().single().clientAudioSessionId == session) { "Repeat activation duplicated or restarted recording" }
            check(settings.microphone == microphone && audio.getStreamVolume(AudioManager.STREAM_MUSIC) == initialVolume) { "Assistant changed audio preferences" }
            result.putString("assistant", "Default assistant activated and connected; one unsilenced recording; repeat activation reused capture; microphone and media volume unchanged")
        } finally {
            context.stopService(Intent(context, ConversationService::class.java))
            await { settings.status == "Stopped" && recordings().isEmpty() }
        }
    }
}
