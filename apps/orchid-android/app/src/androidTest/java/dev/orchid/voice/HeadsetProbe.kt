package dev.orchid.voice

import android.content.Context
import android.media.AudioManager
import android.media.AudioDeviceInfo
import android.media.AudioRecord
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import org.json.JSONObject
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Exercises production playback/capture; captured audio is discarded locally. */
object HeadsetProbe {
    fun run(context: Context, result: Bundle) {
        val audio = context.getSystemService(AudioManager::class.java)
        val volume = audio.getStreamVolume(AudioManager.STREAM_MUSIC)
        check(volume > 0) { "Media is muted; no playback route can be observed" }
        val handler = Handler(Looper.getMainLooper())
        val originalMode = audio.mode
        var captureFailure: String? = null
        val capture = AudioCapture(context, {}, {}, { captureFailure = it })
        val created = CountDownLatch(1)
        val finished = CountDownLatch(1)
        var player: SpeechPlayer? = null
        var playback = ""
        try {
            capture.start()
            val field = AudioCapture::class.java.getDeclaredField("recorder").apply { isAccessible = true }
            val recorder = field.get(capture) as AudioRecord
            val routeDeadline = SystemClock.elapsedRealtime() + 30000
            while (recorder.routedDevice?.type !in listOf(AudioDeviceInfo.TYPE_BLUETOOTH_SCO, AudioDeviceInfo.TYPE_BLE_HEADSET) && SystemClock.elapsedRealtime() < routeDeadline) SystemClock.sleep(100)
            val device = recorder.routedDevice ?: error("No active microphone route")
            check(device.type in listOf(AudioDeviceInfo.TYPE_BLUETOOTH_SCO, AudioDeviceInfo.TYPE_BLE_HEADSET)) { "Headset microphone was not selected: ${device.type}" }
            result.putInt("microphoneType", device.type)
            result.putString("microphoneName", device.productName.toString())
            result.putInt("microphoneDeviceId", device.id)
            val configuration = audio.activeRecordingConfigurations.find { it.clientAudioSessionId == recorder.audioSessionId }
            check(configuration != null && !configuration.isClientSilenced) { "Microphone capture is silenced" }
            result.putBoolean("microphoneSilenced", false)
            handler.post {
                player = SpeechPlayer(context, { receipt ->
                    playback = receipt.getJSONObject("playback").getString("state")
                    if (playback in listOf("completed", "failed", "muted", "unavailable")) finished.countDown()
                }, {}, {})
                created.countDown()
            }
            check(created.await(5, TimeUnit.SECONDS))
            try {
                val deadline = SystemClock.elapsedRealtime() + 15000
                while (player?.isReady != true && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100)
                check(player?.isReady == true) { "Speech engine did not initialize" }
                player!!.speak(JSONObject().put("id", "headset-route-probe").put("text",
                    "This is the Orchid headset routing check. This short message should be heard in your connected headphones. We are checking the playback route, and then the microphone route. No recorded audio will be sent to your laptop."))
                check(finished.await(30, TimeUnit.SECONDS)) { "Playback did not finish" }
                check(playback == "completed") { "Playback ended with $playback" }
                result.putString("playback", playback)
            } finally { handler.post { player?.close() } }
            check(recorder.routedDevice?.id == device.id) { "Microphone route changed during playback" }
            check(captureFailure == null) { captureFailure!! }
        } finally { capture.close() }
        val restoreDeadline = SystemClock.elapsedRealtime() + 5000
        while ((audio.mode != originalMode || audio.getStreamVolume(AudioManager.STREAM_MUSIC) != volume) && SystemClock.elapsedRealtime() < restoreDeadline) SystemClock.sleep(100)
        result.putInt("mediaVolumeBefore", volume)
        result.putInt("mediaVolumeAfter", audio.getStreamVolume(AudioManager.STREAM_MUSIC))
        check(audio.mode == originalMode) { "Audio mode was not restored" }
        check(audio.getStreamVolume(AudioManager.STREAM_MUSIC) == volume) { "Media volume changed during probe" }
        result.putString("headset", "Production playback and microphone routes exercised; audio discarded locally; media volume unchanged")
    }
}
