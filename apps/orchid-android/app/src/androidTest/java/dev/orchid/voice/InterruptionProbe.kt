package dev.orchid.voice

import android.os.Bundle

object InterruptionProbe {
    fun run(result: Bundle) {
        val gate = AddressedSpeechInterruption()
        gate.remember("history", 4)
        check(!gate.accept("history", 4, true, true))
        check(!gate.accept("ambient", 1, false, true))
        check(!gate.accept("partial", 1, true, false))
        check(gate.accept("request", 1, true, true))
        check(!gate.accept("request", 1, true, true)) // Old rolling-window classification.
        check(!gate.accept("request", 2, false, true)) // Sticky relevance alone is insufficient.
        check(gate.accept("request", 2, true, true)) // A revised addressed sentence.
        check(!gate.accept("request", 1, true, true))
        result.putString("interruption", "Ambient and partial input ignored; fresh addressed sentences accepted once per revision")
    }
}
