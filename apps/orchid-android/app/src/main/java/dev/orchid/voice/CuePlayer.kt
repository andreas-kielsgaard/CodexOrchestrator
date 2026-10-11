package dev.orchid.voice

import android.content.Context
import android.media.*
import android.os.Handler
import android.os.Looper
import kotlin.math.sin

class CuePlayer(context: Context) {
    private val audio = context.getSystemService(AudioManager::class.java)
    private val settings = ConnectionSettings(context)
    private val inventory = AudioDevices(context)
    private val handler = Handler(Looper.getMainLooper())
    private var track: AudioTrack? = null
    fun play(kind: String) {
        handler.post {
            if (audio.getStreamVolume(AudioManager.STREAM_MUSIC) == 0 || audio.isStreamMute(AudioManager.STREAM_MUSIC)) return@post
            val frequency = when (kind) { "listening" -> 660.0; "processing" -> 880.0; else -> return@post }
            track?.release()
            val samples = ShortArray(1920) { index ->
                val envelope = minOf(index / 160.0, (1919 - index) / 160.0, 1.0).coerceAtLeast(0.0)
                (sin(2 * Math.PI * frequency * index / 16000) * 8000 * envelope).toInt().toShort()
            }
            val signal = AudioTrack.Builder().setAudioAttributes(SpeechPlayer.playbackAttributes())
                .setAudioFormat(AudioFormat.Builder().setSampleRate(16000).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
                .setTransferMode(AudioTrack.MODE_STATIC).setBufferSizeInBytes(samples.size * 2).build()
            track = signal
            signal.setPreferredDevice(inventory.output(settings.audioOutput))
            signal.write(samples, 0, samples.size); signal.play()
            handler.postDelayed({ if (track === signal) { signal.release(); track = null } }, 500)
        }
    }
    fun close() { handler.post { handler.removeCallbacksAndMessages(null); track?.release(); track = null } }
}
