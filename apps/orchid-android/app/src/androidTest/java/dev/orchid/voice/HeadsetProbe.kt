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
    fun run(context: Context, result: Bundle, microphone: String = "headset", output: String = "automatic", interrupt: Boolean = false) {
        val settings = ConnectionSettings(context)
        val preferences = context.getSharedPreferences("connection", Context.MODE_PRIVATE)
        val previous = preferences.getString("microphone", null)
        val previousOutput = settings.audioOutput
        val devices = AudioDevices(context)
        val input = if (microphone == "usb") devices.inputs().firstOrNull { it.type in listOf(AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET) } ?: error("No USB microphone connected") else null
        val sink = when (output) {
            "bluetooth" -> devices.outputs().firstOrNull { it.type in listOf(AudioDeviceInfo.TYPE_BLUETOOTH_A2DP, AudioDeviceInfo.TYPE_BLE_HEADSET) } ?: error("No Bluetooth media output connected")
            "speaker" -> devices.outputs().first { it.type == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER }
            else -> null
        }
        try {
            settings.microphone = if (microphone == "missing") "11||Disconnected USB microphone" else input?.let { AudioDevices.key(it) } ?: microphone
            settings.audioOutput = sink?.let { AudioDevices.key(it) } ?: output
            exercise(context, result, microphone, sink, interrupt)
        } finally {
            if (previous == null) preferences.edit().remove("microphone").apply() else settings.microphone = previous
            settings.audioOutput = previousOutput
        }
    }
    private fun exercise(context: Context, result: Bundle, microphone: String, sink: AudioDeviceInfo?, interrupt: Boolean) {
        val audio = context.getSystemService(AudioManager::class.java)
        val volume = audio.getStreamVolume(AudioManager.STREAM_MUSIC)
        check(volume > 0) { "Media is muted; no playback route can be observed" }
        val handler = Handler(Looper.getMainLooper())
        val originalMode = audio.mode
        var captureFailure: String? = null
        val capture = AudioCapture(context, {}, { _, _, _, _, _ -> }, { captureFailure = it })
        val created = CountDownLatch(1)
        val finished = CountDownLatch(1)
        var player: SpeechPlayer? = null
        var playback = ""
        var outputKey: String? = null
        val queuedAt = SystemClock.elapsedRealtime()
        try {
            capture.start()
            val field = AudioCapture::class.java.getDeclaredField("recorder").apply { isAccessible = true }
            val recorder = field.get(capture) as AudioRecord
            val routeDeadline = SystemClock.elapsedRealtime() + 30000
            val expectedTypes = when (microphone) {
                "phone", "missing" -> listOf(AudioDeviceInfo.TYPE_BUILTIN_MIC)
                "usb" -> listOf(AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET)
                else -> listOf(AudioDeviceInfo.TYPE_BLUETOOTH_SCO, AudioDeviceInfo.TYPE_BLE_HEADSET)
            }
            while (recorder.routedDevice?.type !in expectedTypes && SystemClock.elapsedRealtime() < routeDeadline) SystemClock.sleep(100)
            val device = recorder.routedDevice ?: error("No active microphone route")
            check(device.type in expectedTypes) { "Requested microphone was not selected: ${device.type}" }
            result.putInt("microphoneType", device.type)
            result.putString("microphoneName", device.productName.toString())
            result.putInt("microphoneDeviceId", device.id)
            var configuration = audio.activeRecordingConfigurations.find { it.clientAudioSessionId == recorder.audioSessionId }
            val configurationDeadline = SystemClock.elapsedRealtime() + 3000
            while ((configuration == null || configuration.isClientSilenced) && SystemClock.elapsedRealtime() < configurationDeadline) {
                SystemClock.sleep(50)
                configuration = audio.activeRecordingConfigurations.find { it.clientAudioSessionId == recorder.audioSessionId }
            }
            check(configuration != null && !configuration.isClientSilenced) { "Microphone capture is silenced" }
            result.putBoolean("microphoneSilenced", false)
            handler.post {
                player = SpeechPlayer(context, { receipt ->
                    playback = receipt.getJSONObject("playback").getString("state")
                    if (playback == "playing") {
                        result.putLong("speechStartMs", SystemClock.elapsedRealtime() - queuedAt)
                        if (interrupt) handler.postDelayed({ player?.interrupt() }, 500)
                    }
                    if (playback in listOf("completed", "interrupted", "failed", "muted", "unavailable")) finished.countDown()
                }, {}, {})
                created.countDown()
            }
            check(created.await(5, TimeUnit.SECONDS))
            try {
                val deadline = SystemClock.elapsedRealtime() + 15000
                while (player?.isReady != true && SystemClock.elapsedRealtime() < deadline) SystemClock.sleep(100)
                check(player?.isReady == true) { "Speech engine did not initialize" }
                player!!.speak(JSONObject().put("id", "headset-route-probe").put("text",
                    "Orchid audio check. Recording is active while this message plays through your selected output."))
                val playbackDeadline = SystemClock.elapsedRealtime() + 15000
                while (outputKey == null && SystemClock.elapsedRealtime() < playbackDeadline && finished.count > 0) {
                    val status = JSONObject(ConnectionSettings(context).playbackStatus)
                    if (status.optBoolean("routeActive") && !status.isNull("routedDeviceKey")) {
                        outputKey = status.getString("routedDeviceKey")
                        result.putString("playbackRoute", status.getString("routedOutput"))
                    }
                    SystemClock.sleep(20)
                }
                check(finished.await(30, TimeUnit.SECONDS)) { "Playback did not finish" }
                result.putString("requestedOutput", sink?.let { AudioDevices.key(it) } ?: "automatic")
                result.putString("finalPlaybackStatus", ConnectionSettings(context).playbackStatus)
                check(playback == if (interrupt) "interrupted" else "completed") { "Playback ended with $playback" }
                if (sink != null) check(outputKey == AudioDevices.key(sink)) { "Wrong output route: $outputKey" }
                result.putString("playback", playback)
            } finally { handler.post { player?.close() } }
            check(recorder.routedDevice?.id == device.id) { "Microphone route changed during playback" }
            check(captureFailure == null) { captureFailure!! }
            if (microphone != "headset") check(audio.mode == originalMode) { "Non-Bluetooth microphone changed audio mode" }
            result.putInt("audioModeDuringCapture", audio.mode)
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
