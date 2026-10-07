package dev.orchid.voice

import android.content.Context
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Handler
import android.os.Looper

class SpeechAudioFocus(context: Context, private val interrupted: () -> Unit) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val settings = ConnectionSettings(context)
    private val handler = Handler(Looper.getMainLooper())
    private var request: AudioFocusRequest? = null
    private var gain: Int? = null

    @Synchronized fun acquire(): Boolean {
        val desired = when (settings.backgroundAudio) {
            "pause" -> AudioManager.AUDIOFOCUS_GAIN_TRANSIENT
            "off" -> { release(); return true }
            else -> AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK
        }
        if (request != null && gain == desired) return true
        release()
        val next = AudioFocusRequest.Builder(desired)
            .setAudioAttributes(SpeechPlayer.playbackAttributes())
            .setWillPauseWhenDucked(true)
            .setOnAudioFocusChangeListener({ change ->
                if (change == AudioManager.AUDIOFOCUS_LOSS || change == AudioManager.AUDIOFOCUS_LOSS_TRANSIENT || change == AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK) interrupted()
            }, handler).build()
        if (audio.requestAudioFocus(next) != AudioManager.AUDIOFOCUS_REQUEST_GRANTED) return false
        request = next; gain = desired
        return true
    }

    @Synchronized fun release() {
        val previous = request
        request = null; gain = null
        if (previous != null) audio.abandonAudioFocusRequest(previous)
    }
}
