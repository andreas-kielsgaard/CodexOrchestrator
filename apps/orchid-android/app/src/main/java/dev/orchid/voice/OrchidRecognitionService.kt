package dev.orchid.voice

import android.content.Intent
import android.speech.RecognitionService
import android.speech.SpeechRecognizer

/** Android requires this component for assistant registration. Orchid transcribes on its laptop. */
class OrchidRecognitionService : RecognitionService() {
    override fun onStartListening(intent: Intent, listener: Callback) { listener.error(SpeechRecognizer.ERROR_CLIENT) }
    override fun onStopListening(listener: Callback) {}
    override fun onCancel(listener: Callback) {}
}
