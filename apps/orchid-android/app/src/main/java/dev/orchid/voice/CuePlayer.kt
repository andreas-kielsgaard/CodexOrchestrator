package dev.orchid.voice

import android.content.Context
import android.media.AudioManager
import android.media.ToneGenerator
import android.os.Handler
import android.os.Looper

class CuePlayer(context: Context) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val handler = Handler(Looper.getMainLooper())
    private var tone: ToneGenerator? = null
    fun play(kind: String) {
        handler.post {
            if (audio.getStreamVolume(AudioManager.STREAM_MUSIC) == 0 || audio.isStreamMute(AudioManager.STREAM_MUSIC)) return@post
            val signal = when (kind) { "listening" -> ToneGenerator.TONE_PROP_BEEP; "processing" -> ToneGenerator.TONE_PROP_ACK; else -> return@post }
            if (tone == null) tone = ToneGenerator(AudioManager.STREAM_MUSIC, 40)
            tone?.startTone(signal, 120)
        }
    }
    fun close() { handler.post { tone?.release(); tone = null } }
}
